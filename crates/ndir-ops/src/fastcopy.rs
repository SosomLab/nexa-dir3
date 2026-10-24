//! 고속 복사 **전략 계층**(NEW-007 1차 · DR-17 · 사용자 10-05 "고속 복사 — 윈도우 용 먼저").
//!
//! dir2 전송 엔진(`lib.rs` — 충돌 · 스테이징 교체 · 교차 볼륨 이동 · 취소 · 개별 격리)은 그대로 두고, **파일 하나를 옮기는 방법**과
//! **폴더 안 파일들을 도는 방법**만 이 모듈이 고른다. 엔진의 보장(취소 · 부분 파일 정리 · `copied` 후위 순서)은 바뀌지 않는다.
//!
//! - **파일 1개** — Windows = `CopyFileExW`(커널 복사 경로: ReFS/Dev Drive 블록 복제 · SMB 서버 쪽 복사 · 수정 시각 · 속성 · 대체
//!   스트림 보존 · 큰 파일은 `COPY_FILE_NO_BUFFERING`으로 파일 캐시를 거치지 않는다) · **Linux** = `ioctl(FICLONE)` 블록 복제
//!   (Btrfs · XFS reflink · bcachefs · 같은 파일 시스템) → 안 되면 `copy_file_range`(커널 안 복사 · NFS 4.2/SMB 서버 쪽 복사 · 32 MiB
//!   청크마다 진행 · 중단) · **macOS** = `copyfile(COPYFILE_ALL | COPYFILE_CLONE)`(APFS 같은 볼륨 = `clonefile` 즉시 · 아니면 커널 복사 ·
//!   확장 속성 · ACL · 수정 시각 보존 · 진행 콜백 · 중단) — 2차 T-164(10-24) · 그 밖(과 끈 경우 · 커널 경로가 지원하지 않는 파일 시스템 ·
//!   심볼릭 링크 원본) = 읽고 쓰는 루프(dir2 원형) + 수정 시각 · 권한 보존.
//! - **폴더** — 먼저 한 번 훑어 폴더를 만들고 파일 작업 목록을 모은 뒤, 작업 스레드 여러 개가 나눠 복사한다(작은 파일이 많은 폴더에서
//!   효과가 크다 — 파일당 비용은 디스크 탐색보다 메타데이터 · 실시간 검사가 지배한다). 스레드 수 자동 = 양쪽이 모두 탐색 지연
//!   없는 저장 장치(SSD · 네트워크)면 8까지, 아니면 4까지 · **큰 파일은 한 번에 하나만**(회전 디스크에서 헤드가 오가지 않게).
//!   진행 통지(`on_bytes`)는 **부른 스레드에서만** 불린다.
//!
//! 실측(10-05 · 이 PC · 릴리스 · 16 KiB × 4,000개): 종전 루프 15.0 ~ 18.8초 → `CopyFileExW` 8스레드 4.1 ~ 5.5초(3.4 ~ 4.6배) ·
//! 512 MiB 1개: 루프 5.8 ~ 10.8초 → `CopyFileExW` 5.3 ~ 6.1초 · 비버퍼드는 12.6 ~ 15.3초로 **느렸다** → 비버퍼드 기본 = 끔.
//!
//! OS 의존은 이 파일의 `os` 모듈에만 둔다(NEW-007 "엔진 크레이트의 os 포트" — nexa-dir `platform/`과 독립 · 외부 crate 0).

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;

/// 읽고 쓰는 루프의 버퍼 상한(dir2 `CopyBufferSize` — 4 MiB).
const COPY_BUF: usize = 4 * 1024 * 1024;
/// 자동 스레드 수의 상한 — 탐색 지연 없는 장치끼리(실측: 4 → 8에서 아직 이득).
const AUTO_THREADS_FREE: usize = 8;
/// 자동 스레드 수의 상한 — 탐색 지연이 있거나 알 수 없는 장치가 낀 경우.
const AUTO_THREADS_COSTLY: usize = 4;
/// 이 크기 이상인 파일은 병렬 복사 중에도 **한 번에 하나만** 복사한다(순차 처리량이 한계인 구간 — 여럿이 겹치면 손해).
const BIG_FILE: u64 = 32 * 1024 * 1024;
/// 스레드 수 설정의 상한.
pub const THREADS_MAX: usize = 16;

/// 복사 전략 조절값(설정 `transfer.native` · `transfer.unbuffered_mb` · `transfer.threads`가 넣는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tuning {
    /// OS의 파일 복사 경로를 쓴다(Windows `CopyFileExW`) — 끄면 어디서나 읽고 쓰는 루프.
    pub native: bool,
    /// 이 크기(바이트) 이상인 파일은 파일 캐시를 거치지 않고 복사한다(Windows · 0 = 쓰지 않음 = 기본 — 메모리보다 훨씬 큰
    /// 파일에서 다른 프로그램의 캐시를 밀어내지 않으려는 용도이고, 보통은 버퍼드가 빠르다).
    pub unbuffered_min: u64,
    /// 폴더 복사의 작업 스레드 수 — 0 = 자동(저장 장치를 보고 정한다) · 1 = 차례로.
    pub threads: usize,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            native: true,
            unbuffered_min: 0,
            threads: 0,
        }
    }
}

static NATIVE: AtomicBool = AtomicBool::new(true);
static UNBUFFERED_MIN: AtomicU64 = AtomicU64::new(0);
static THREADS: AtomicUsize = AtomicUsize::new(0);

/// 전송 엔진이 쓸 조절값을 바꾼다(프로세스 전역 — 다음 파일부터 적용).
pub fn set_tuning(t: Tuning) {
    NATIVE.store(t.native, Ordering::Relaxed);
    UNBUFFERED_MIN.store(t.unbuffered_min, Ordering::Relaxed);
    THREADS.store(t.threads.min(THREADS_MAX), Ordering::Relaxed);
}

/// 지금의 조절값.
#[must_use]
pub fn tuning() -> Tuning {
    Tuning {
        native: NATIVE.load(Ordering::Relaxed),
        unbuffered_min: UNBUFFERED_MIN.load(Ordering::Relaxed),
        threads: THREADS.load(Ordering::Relaxed),
    }
}

/// 중단 판정(취소 · 다른 작업 스레드의 실패) — 여러 스레드가 함께 본다.
pub(crate) type Stop<'a> = &'a (dyn Fn() -> bool + Sync);

fn canceled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "canceled")
}

/// 저장 장치의 탐색 지연(회전 디스크) 판정 결과.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Seek {
    /// 탐색 지연 없음(SSD · 네트워크 공유).
    Free,
    /// 탐색 지연 있음(HDD) 또는 알 수 없음 — 병렬 복사가 손해일 수 있다.
    Costly,
}

/// 폴더 복사의 작업 스레드 수(순수): 설정이 1 이상이면 그 값 · 0(자동)이면 양쪽이 모두 [`Seek::Free`]일 때 `min(코어, 8)`,
/// 아니면 `min(코어, 4)`.
#[must_use]
pub fn thread_count(setting: usize, src: Seek, dest: Seek, cores: usize) -> usize {
    if setting >= 1 {
        return setting.min(THREADS_MAX);
    }
    if src == Seek::Free && dest == Seek::Free {
        cores.clamp(1, AUTO_THREADS_FREE)
    } else {
        cores.clamp(1, AUTO_THREADS_COSTLY)
    }
}

/// 파일 하나를 복사한다 — 증분 바이트를 `on_bytes`로 보고 · `stop`이 참이 되면 `Interrupted`로 끝낸다.
/// 실패 · 중단 시 **이번에 만든** 대상 파일은 남기지 않는다(이미 있던 대상을 `overwrite = false`로 만난 경우는 건드리지 않는다).
pub(crate) fn copy_file(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    stop: Stop<'_>,
    t: &Tuning,
) -> io::Result<()> {
    if stop() {
        return Err(canceled());
    }
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    if t.native {
        let mut sent = 0u64;
        let r = os::copy_file(
            src,
            dest,
            overwrite,
            &mut |n| {
                sent += n;
                on_bytes(n);
            },
            stop,
            t.unbuffered_min,
        );
        return match r {
            // 커널 복사 경로가 이 조합에서 안 된다 → 읽고 쓰는 루프로 다시 — **아직 한 바이트도 보고하지 않았을 때만**(중간 실패를
            // 처음부터 다시 하면 진행 합계가 이중으로 보고된다 · 오류 종류 · errno는 그대로 상위로 · 협업 리뷰 10-24 ②).
            Err(e) if sent == 0 && os::unsupported(&e) => {
                copy_file_portable(src, dest, overwrite, on_bytes, stop)
            }
            r => r,
        };
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    let _ = t;
    copy_file_portable(src, dest, overwrite, on_bytes, stop)
}

/// 루프로 복사한 뒤의 보존(수정 시각 · 권한 — 최선 노력 · 못 해도 복사는 성공이다 · FAT · 네트워크 공유 등).
#[cfg(target_os = "linux")]
fn preserve_after_copy(dest: &Path, output: fs::File, meta: &fs::Metadata) {
    if let Ok(t) = meta.modified() {
        let _ = output.set_modified(t);
    }
    drop(output);
    let _ = fs::set_permissions(dest, meta.permissions());
}

/// 읽고 쓰는 루프(dir2 `CopyFileWithProgress` 원형) + 수정 시각 · 권한 보존(dir3 보강 — 종전에는 사본의 수정 시각이 복사한 시각이
/// 되고 Unix에서 실행 권한이 사라졌다).
pub(crate) fn copy_file_portable(
    src: &Path,
    dest: &Path,
    overwrite: bool,
    on_bytes: &mut dyn FnMut(u64),
    stop: Stop<'_>,
) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let meta = input.metadata().ok();
    let mut output = if overwrite {
        fs::File::create(dest)?
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)?
    };
    let len = meta.as_ref().map_or(u64::MAX, fs::Metadata::len);
    let cap = usize::try_from(len)
        .unwrap_or(COPY_BUF)
        .clamp(64 * 1024, COPY_BUF);
    let mut buf = vec![0u8; cap];
    let run = (|| -> io::Result<()> {
        loop {
            if stop() {
                return Err(canceled());
            }
            let n = input.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            output.write_all(&buf[..n])?;
            on_bytes(n as u64);
        }
    })();
    if run.is_err() {
        drop(output);
        let _ = fs::remove_file(dest); // 부분 파일 정리
        return run;
    }
    // 보존은 최선 노력 — 못 해도 복사는 성공이다(FAT · 네트워크 공유 등).
    if let Some(m) = meta {
        if let Ok(t) = m.modified() {
            let _ = output.set_modified(t);
        }
        drop(output);
        let _ = fs::set_permissions(dest, m.permissions());
    }
    Ok(())
}

/// 폴더 복사의 파일 작업 하나.
struct Job {
    src: PathBuf,
    dest: PathBuf,
    /// 원본 크기(열거 때 읽은 값 — 큰 파일 판정용 · 못 읽으면 0).
    len: u64,
    /// 끝나면 원본 경로를 `copied`에 적는다(링크 안쪽은 적지 않는다 — `lib.rs` `remove_copied` 주석).
    record: bool,
}

/// 폴더를 훑어 대상 폴더를 만들고 파일 작업과 "복사를 마친 폴더"(후위 순서)를 모은다 — 종전 재귀 복사의 순회 규칙 그대로.
fn plan(
    src: &Path,
    dest: &Path,
    record: bool,
    stop: Stop<'_>,
    jobs: &mut Vec<Job>,
    dirs: &mut Vec<PathBuf>,
) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for e in fs::read_dir(src)? {
        if stop() {
            return Err(canceled());
        }
        // 열거 중 `Err` 엔트리는 오류로 전파한다(점검 2차 A14 — 빠진 항목을 조용히 버리면 교차 볼륨 이동에서 유실된다).
        let e = e?;
        let p = e.path();
        let d = dest.join(e.file_name());
        let ft = e.file_type()?;
        let link = crate::is_link(&p);
        if ft.is_dir() && !link {
            plan(&p, &d, record, stop, jobs, dirs)?;
            if record {
                dirs.push(p);
            }
        } else if link && fs::metadata(&p).map(|m| m.is_dir()).unwrap_or(false) {
            // 폴더 링크(심볼릭 링크 · 정션): 탐색기처럼 **대상 내용**을 복사한다. 단 ① 링크 안쪽 경로는 적지 않는다 — 교차 볼륨
            // 이동의 원본 정리가 링크를 통해 **대상의 실제 파일을 지우지 않게**(10-03 데이터 손실 결함) 링크 경로 하나만 적는다
            // ② 상위 폴더를 가리키는 링크(순환)는 오류로 멈춘다.
            if let (Ok(target), Ok(here)) = (fs::canonicalize(&p), fs::canonicalize(src)) {
                if here.starts_with(&target) {
                    return Err(io::Error::other(format!(
                        "링크가 상위 폴더를 가리켜 복사할 수 없음(순환): {}",
                        crate::leaf_name(&p)
                    )));
                }
            }
            plan(&p, &d, false, stop, jobs, dirs)?;
            if record {
                dirs.push(p);
            }
        } else {
            jobs.push(Job {
                len: e.metadata().map_or(0, |m| m.len()),
                src: p,
                dest: d,
                record,
            });
        }
    }
    Ok(())
}

/// 폴더 재귀 복사 — `copied`가 있으면 복사를 마친 원본 경로를 적는다(파일들 → 폴더들의 **후위 순서** · 교차 볼륨 이동의 원본 정리용).
/// 폴더 안 파일은 원본 규약대로 덮어쓰기 복사다. 첫 실패에서 멈추고 그 오류를 돌려준다(취소 = `Interrupted`).
pub(crate) fn copy_tree(
    src: &Path,
    dest: &Path,
    on_bytes: &mut dyn FnMut(u64),
    stop: Stop<'_>,
    copied: Option<&mut Vec<PathBuf>>,
    t: &Tuning,
) -> io::Result<()> {
    let mut jobs = Vec::new();
    let mut dirs = Vec::new();
    plan(src, dest, copied.is_some(), stop, &mut jobs, &mut dirs)?;
    let threads = if jobs.len() < 2 {
        1
    } else {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        let n = if t.threads == 0 {
            thread_count(0, os::seek_of(src), os::seek_of(dest), cores)
        } else {
            thread_count(t.threads, Seek::Free, Seek::Free, cores)
        };
        n.min(jobs.len())
    };
    let mut done: Vec<usize> = Vec::new();
    let result = if threads <= 1 {
        (|| {
            for (i, job) in jobs.iter().enumerate() {
                copy_file(&job.src, &job.dest, true, on_bytes, stop, t)?;
                done.push(i);
            }
            Ok(())
        })()
    } else {
        copy_jobs_parallel(&jobs, threads, on_bytes, stop, t, &mut done)
    };
    result?;
    if let Some(c) = copied {
        c.extend(
            done.into_iter()
                .filter(|&i| jobs[i].record)
                .map(|i| jobs[i].src.clone()),
        );
        c.extend(dirs);
    }
    Ok(())
}

enum Msg {
    Bytes(u64),
    Done(usize),
    Fail(io::Error),
}

/// 작업 스레드 `threads`개가 작업 목록을 나눠 복사한다 — 진행 · 완료는 채널로 모아 **부른 스레드**가 `on_bytes`를 부른다.
/// 하나가 실패하면 나머지도 멈춘다(진행 중이던 파일은 정리된다) · 돌려주는 오류는 진짜 실패가 취소보다 우선이다.
fn copy_jobs_parallel(
    jobs: &[Job],
    threads: usize,
    on_bytes: &mut dyn FnMut(u64),
    stop: Stop<'_>,
    t: &Tuning,
    done: &mut Vec<usize>,
) -> io::Result<()> {
    let next = AtomicUsize::new(0);
    let abort = AtomicBool::new(false);
    // 큰 파일은 한 번에 하나만(작은 파일들은 그 사이에도 계속 돈다).
    let big = std::sync::Mutex::new(());
    let (tx, rx) = mpsc::channel::<Msg>();
    let mut first: Option<io::Error> = None;
    std::thread::scope(|s| {
        for _ in 0..threads {
            let tx = tx.clone();
            let (next, abort, big) = (&next, &abort, &big);
            s.spawn(move || {
                let halt = || abort.load(Ordering::Relaxed) || stop();
                loop {
                    if halt() {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let _one_big = (job.len >= BIG_FILE).then(|| {
                        big.lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                    });
                    let r = copy_file(
                        &job.src,
                        &job.dest,
                        true,
                        &mut |n| {
                            let _ = tx.send(Msg::Bytes(n));
                        },
                        &halt,
                        t,
                    );
                    match r {
                        Ok(()) => {
                            let _ = tx.send(Msg::Done(i));
                        }
                        Err(e) => {
                            abort.store(true, Ordering::Relaxed);
                            let _ = tx.send(Msg::Fail(e));
                            break;
                        }
                    }
                }
            });
        }
        drop(tx);
        for msg in rx {
            match msg {
                Msg::Bytes(n) => on_bytes(n),
                Msg::Done(i) => done.push(i),
                Msg::Fail(e) => {
                    let real = e.kind() != io::ErrorKind::Interrupted;
                    let held_real = first
                        .as_ref()
                        .is_some_and(|f| f.kind() != io::ErrorKind::Interrupted);
                    if first.is_none() || (real && !held_real) {
                        first = Some(e);
                    }
                }
            }
        }
    });
    match first {
        Some(e) => Err(e),
        None if stop() && done.len() < jobs.len() => Err(canceled()),
        None => Ok(()),
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod os {
    use super::Seek;
    use std::path::Path;

    /// 저장 장치 판정 — 그 밖의 Unix(BSD 등)는 보지 않는다 → 보수적으로 [`Seek::Costly`](자동 4까지).
    pub(super) fn seek_of(_path: &Path) -> Seek {
        Seek::Costly
    }
}

/// Linux(T-164 · docs/24 §6): ① `ioctl(FICLONE)` = 블록 복제(Btrfs · XFS reflink · bcachefs · 같은 파일 시스템 — 데이터를 옮기지 않고
/// 즉시) ② `copy_file_range` = 커널 안 복사(사용자 공간 왕복 없음 · NFS 4.2 · SMB 3 서버 쪽 복사 · 5.3+ 교차 파일 시스템) — 32 MiB
/// 청크마다 진행 보고 · 중단 점검 ③ 둘 다 안 되는 조합(오래된 커널 · 특수 파일 시스템)은 상위가 루프로 다시. 장치 판정은
/// `/sys/dev/block/<maj>:<min>/queue/rotational`(파티션이면 상위 장치) · 블록 장치가 아닌 것(NFS · CIFS · tmpfs · FUSE = major 0)은 탐색
/// 지연 없음으로 본다. 외부 crate 0 — libc 심벌만 선언한다.
#[cfg(target_os = "linux")]
mod os {
    use super::{canceled, preserve_after_copy, Seek, Stop};
    use std::ffi::{c_int, c_uint, c_ulong};
    use std::fs;
    use std::io;
    use std::os::fd::AsRawFd as _;
    use std::os::linux::fs::MetadataExt as _;
    use std::path::Path;

    /// `copy_file_range` 한 번에 옮기는 상한 — 이 단위로 진행을 보고하고 중단을 본다.
    const CHUNK: usize = 32 * 1024 * 1024;
    /// `FICLONE` = `_IOW(0x94, 9, int)`.
    const FICLONE: c_ulong = 0x4004_9409;
    const POSIX_FADV_SEQUENTIAL: c_int = 2;
    const EPERM: i32 = 1;
    const EBADF: i32 = 9;
    const EXDEV: i32 = 18;
    const EINVAL: i32 = 22;
    const ENOSYS: i32 = 38;
    const EOPNOTSUPP: i32 = 95;

    extern "C" {
        fn copy_file_range(
            fd_in: c_int,
            off_in: *mut i64,
            fd_out: c_int,
            off_out: *mut i64,
            len: usize,
            flags: c_uint,
        ) -> isize;
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
        fn posix_fadvise(fd: c_int, offset: i64, len: i64, advice: c_int) -> c_int;
    }

    /// 파일 하나 — 블록 복제 → 커널 안 복사. 실패 · 중단 시 이번에 만든 대상은 지운다.
    pub(super) fn copy_file(
        src: &Path,
        dest: &Path,
        overwrite: bool,
        on_bytes: &mut dyn FnMut(u64),
        stop: Stop<'_>,
        _unbuffered_min: u64,
    ) -> io::Result<()> {
        let input = fs::File::open(src)?;
        let meta = input.metadata()?;
        let len = meta.len();
        let output = if overwrite {
            fs::File::create(dest)?
        } else {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dest)?
        };
        let run = (|| -> io::Result<()> {
            // ① 블록 복제 — 같은 파일 시스템 · reflink 지원일 때만 성공(아니면 EXDEV/EOPNOTSUPP/EINVAL — 조용히 ②로).
            // SAFETY: 두 fd는 열린 파일이고 호출 동안 살아 있다 · FICLONE의 인자는 원본 fd(int) 하나다.
            if unsafe { ioctl(output.as_raw_fd(), FICLONE, input.as_raw_fd()) } == 0 {
                on_bytes(len);
                return Ok(());
            }
            // SAFETY: 열린 fd · 범위 0 = 파일 전체 · 조언이라 실패해도 무해.
            let _ = unsafe { posix_fadvise(input.as_raw_fd(), 0, 0, POSIX_FADV_SEQUENTIAL) };
            // ② 커널 안 복사 — 파일 오프셋(둘 다 NULL)을 쓴다. 폴백(상위가 루프로 다시)은 **아직 한 바이트도 안 옮겼을 때만**:
            // 첫 호출이 "지원 안 함"이거나 **0을 돌려주면**(procfs/sysfs 의사 파일 · 일부 FUSE — 그대로 두면 빈 대상이 조용히 성공 ·
            // std `kernel_copy`와 같은 규칙) `Unsupported` · 중간 실패는 원 오류 그대로(상위는 보낸 바이트가 0일 때만 루프로 — 종류 · errno 보존).
            let mut written = 0u64;
            loop {
                if stop() {
                    return Err(canceled());
                }
                // SAFETY: 두 fd는 열린 파일 · 오프셋 NULL = 파일 위치 사용 · flags 0.
                let n = unsafe {
                    copy_file_range(
                        input.as_raw_fd(),
                        std::ptr::null_mut(),
                        output.as_raw_fd(),
                        std::ptr::null_mut(),
                        CHUNK,
                        0,
                    )
                };
                if n < 0 {
                    return Err(io::Error::last_os_error()); // 종류 · errno 보존(상위가 보낸 바이트 0일 때만 폴백)
                }
                if n == 0 {
                    if written == 0 {
                        return Err(io::Error::from(io::ErrorKind::Unsupported));
                    }
                    return Ok(()); // EOF
                }
                written += n as u64;
                on_bytes(n as u64);
            }
        })();
        if let Err(e) = run {
            drop(output);
            let _ = fs::remove_file(dest); // 부분 파일 정리
            return Err(e);
        }
        preserve_after_copy(dest, output, &meta);
        Ok(())
    }

    /// 커널 복사 경로가 이 조합에서 지원되지 않는다는 오류인가(→ 읽고 쓰는 루프로 다시) — `EPERM`(컨테이너/snap seccomp 차단) ·
    /// `EBADF`(일부 파일 종류)도 std 폴백 목록과 같게 포함. 중간 실패의 폴백은 상위가 "보낸 바이트 0"으로 막는다(오류 종류 보존).
    pub(super) fn unsupported(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::Unsupported
            || matches!(
                e.raw_os_error(),
                Some(EPERM | EBADF | EXDEV | EINVAL | ENOSYS | EOPNOTSUPP)
            )
    }

    /// 경로가 놓인 저장 장치의 탐색 지연 여부 — `st_dev` → `/sys/dev/block/<maj>:<min>/queue/rotational`(파티션은 상위 장치의
    /// `../queue`) · 블록 장치가 아니면(major 0: Btrfs/ZFS/overlayfs의 익명 장치 · NFS · CIFS · tmpfs · FUSE) **모르는 것으로**(있음 ·
    /// 자동 4 — Btrfs가 HDD 위일 수 있어 "모르면 있음" 원칙 · 네트워크 판정은 `statfs f_type` 후속) · 못 읽어도 있음.
    pub(super) fn seek_of(path: &Path) -> Seek {
        let meta = fs::metadata(path).or_else(|e| match path.parent() {
            Some(p) if !p.as_os_str().is_empty() => fs::metadata(p),
            _ => Err(e),
        });
        let Ok(m) = meta else {
            return Seek::Costly;
        };
        let dev = m.st_dev();
        let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0xfff);
        let minor = ((dev >> 12) & 0xffff_ff00) | (dev & 0xff);
        if major == 0 {
            return Seek::Costly;
        }
        rotational(major, minor).map_or(
            Seek::Costly,
            |rot| {
                if rot {
                    Seek::Costly
                } else {
                    Seek::Free
                }
            },
        )
    }

    /// `queue/rotational` 읽기: "0" = 탐색 지연 없음 · "1" = 있음 · 없으면 None(상위가 "있음"으로 둔다).
    fn rotational(major: u64, minor: u64) -> Option<bool> {
        let base = format!("/sys/dev/block/{major}:{minor}");
        for rel in ["queue/rotational", "../queue/rotational"] {
            if let Ok(s) = fs::read_to_string(format!("{base}/{rel}")) {
                return match s.trim() {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                };
            }
        }
        None
    }
}

/// macOS(T-164 · docs/24 §6): `copyfile(3)` 한 번으로 — `COPYFILE_CLONE`이면 APFS 같은 볼륨은 `clonefile`(즉시 · 쓰기 시 복사) · 아니면
/// 커널 복사로 떨어진다(`COPYFILE_ALL` = 데이터 + 수정 시각/권한 + 확장 속성 + ACL 보존) · `COPYFILE_STATE_STATUS_CB`로 진행 보고 ·
/// 중단은 콜백이 `COPYFILE_QUIT`. `COPYFILE_CLONE`은 `NOFOLLOW_SRC`를 함의하므로 **심볼릭 링크 원본은 루프**(다른 OS처럼 내용 복사).
/// 장치 판정은 아직 보지 않는다(알 수 없음 = 자동 4). 외부 crate 0 — libSystem 심벌만 선언한다.
#[cfg(target_os = "macos")]
mod os {
    use super::{canceled, Seek, Stop};
    use std::ffi::{c_char, c_int, c_void, CString};
    use std::fs;
    use std::io;
    use std::os::unix::ffi::OsStrExt as _;
    use std::path::Path;

    type CopyfileState = *mut c_void;
    /// `COPYFILE_ACL | COPYFILE_STAT | COPYFILE_XATTR | COPYFILE_DATA`.
    const COPYFILE_ALL: u32 = 0xF;
    const COPYFILE_CLONE: u32 = 1 << 24;
    const COPYFILE_STATE_STATUS_CB: u32 = 6;
    const COPYFILE_STATE_STATUS_CTX: u32 = 7;
    const COPYFILE_STATE_COPIED: u32 = 8;
    const COPYFILE_COPY_DATA: c_int = 4;
    const COPYFILE_PROGRESS: c_int = 4;
    const COPYFILE_CONTINUE: c_int = 0;
    const COPYFILE_QUIT: c_int = 2;
    const EXDEV: i32 = 18;
    const EINVAL: i32 = 22;
    const ENOTSUP: i32 = 45;
    const ECANCELED: i32 = 89;

    type StatusCb = unsafe extern "C" fn(
        what: c_int,
        stage: c_int,
        state: CopyfileState,
        src: *const c_char,
        dst: *const c_char,
        ctx: *mut c_void,
    ) -> c_int;

    extern "C" {
        fn copyfile_state_alloc() -> CopyfileState;
        fn copyfile_state_free(s: CopyfileState) -> c_int;
        fn copyfile_state_set(s: CopyfileState, flag: u32, src: *const c_void) -> c_int;
        fn copyfile_state_get(s: CopyfileState, flag: u32, dst: *mut c_void) -> c_int;
        fn copyfile(
            from: *const c_char,
            to: *const c_char,
            state: CopyfileState,
            flags: u32,
        ) -> c_int;
    }

    struct Ctx<'a> {
        on_bytes: &'a mut dyn FnMut(u64),
        stop: Stop<'a>,
        sent: u64,
    }

    /// 진행 콜백 — 데이터 복사 진행 단계에서 누적 바이트(`COPYFILE_STATE_COPIED`)의 증분을 보고 · 중단이면 `COPYFILE_QUIT`.
    unsafe extern "C" fn status(
        what: c_int,
        stage: c_int,
        state: CopyfileState,
        _src: *const c_char,
        _dst: *const c_char,
        ctx: *mut c_void,
    ) -> c_int {
        // SAFETY: `ctx`는 `copy_file`이 넘긴 `Ctx`이고 `copyfile` 호출 동안 살아 있다.
        let ctx = unsafe { &mut *ctx.cast::<Ctx<'_>>() };
        if what == COPYFILE_COPY_DATA && stage == COPYFILE_PROGRESS {
            let mut copied: i64 = 0;
            // SAFETY: `state`는 라이브러리가 넘긴 유효한 상태 · COPIED는 off_t(i64) 하나를 쓴다.
            if unsafe { copyfile_state_get(state, COPYFILE_STATE_COPIED, (&raw mut copied).cast()) }
                == 0
            {
                let c = u64::try_from(copied).unwrap_or(0);
                if c > ctx.sent {
                    (ctx.on_bytes)(c - ctx.sent);
                    ctx.sent = c;
                }
            }
        }
        if (ctx.stop)() {
            COPYFILE_QUIT
        } else {
            COPYFILE_CONTINUE
        }
    }

    fn cstr(p: &Path) -> io::Result<CString> {
        CString::new(p.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))
    }

    /// 파일 하나 — `copyfile(ALL | CLONE)`. 실패 · 중단 시 이번에 만든 대상은 지운다 · 심볼릭 링크 원본 = `Unsupported`(상위가 루프로).
    pub(super) fn copy_file(
        src: &Path,
        dest: &Path,
        overwrite: bool,
        on_bytes: &mut dyn FnMut(u64),
        stop: Stop<'_>,
        _unbuffered_min: u64,
    ) -> io::Result<()> {
        let meta = fs::symlink_metadata(src)?;
        if meta.file_type().is_symlink() {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let len = meta.len();
        // CLONE = EXCL(있는 대상 거부)을 함의한다 → 덮어쓰기는 먼저 지운다 · 아니면 AlreadyExists.
        if fs::symlink_metadata(dest).is_ok() {
            if !overwrite {
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
            fs::remove_file(dest)?;
        }
        let (from, to) = (cstr(src)?, cstr(dest)?);
        let mut ctx = Ctx {
            on_bytes,
            stop,
            sent: 0,
        };
        // SAFETY: 상태 객체는 이 함수 안에서만 쓰고 끝에 해제한다 · 콜백/컨텍스트 포인터는 호출 동안 유효하다.
        let state = unsafe { copyfile_state_alloc() };
        if state.is_null() {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let cb: StatusCb = status;
        let rc = unsafe {
            copyfile_state_set(state, COPYFILE_STATE_STATUS_CB, cb as *const c_void);
            copyfile_state_set(state, COPYFILE_STATE_STATUS_CTX, (&raw mut ctx).cast());
            copyfile(
                from.as_ptr(),
                to.as_ptr(),
                state,
                COPYFILE_ALL | COPYFILE_CLONE,
            )
        };
        let err = (rc != 0).then(io::Error::last_os_error);
        // SAFETY: 위에서 받은 상태 객체를 한 번만 해제한다.
        unsafe {
            copyfile_state_free(state);
        }
        if let Some(e) = err {
            let _ = fs::remove_file(dest); // 부분 파일 정리
            return Err(if e.raw_os_error() == Some(ECANCELED) || stop() {
                canceled()
            } else {
                e
            });
        }
        // 복제로 끝났으면 콜백이 불리지 않는다 → 합계를 채운다.
        if ctx.sent < len {
            (ctx.on_bytes)(len - ctx.sent);
        }
        Ok(())
    }

    /// 커널 복사 경로가 지원되지 않는다는 오류인가(→ 읽고 쓰는 루프로 다시).
    pub(super) fn unsupported(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::Unsupported
            || matches!(e.raw_os_error(), Some(EXDEV | EINVAL | ENOTSUP))
    }

    /// 장치 판정은 아직 보지 않는다(IOKit 특성 조회는 후속) → 알 수 없음 = [`Seek::Costly`](자동 4까지).
    pub(super) fn seek_of(_path: &Path) -> Seek {
        Seek::Costly
    }
}

#[cfg(windows)]
mod os {
    use super::{canceled, Seek, Stop};
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::ffi::OsStrExt as _;
    use std::path::{Component, Path, Prefix};

    const COPY_FILE_FAIL_IF_EXISTS: u32 = 0x1;
    const COPY_FILE_NO_BUFFERING: u32 = 0x1000;
    const PROGRESS_CONTINUE: u32 = 0;
    const PROGRESS_CANCEL: u32 = 1;
    const ERROR_INVALID_FUNCTION: i32 = 1;
    const ERROR_NOT_SUPPORTED: i32 = 50;
    const ERROR_INVALID_PARAMETER: i32 = 87;
    const ERROR_REQUEST_ABORTED: i32 = 1235;
    const FILE_SHARE_READ_WRITE: u32 = 0x3;
    const OPEN_EXISTING: u32 = 3;
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
    const STORAGE_DEVICE_SEEK_PENALTY_PROPERTY: u32 = 7;

    type ProgressRoutine = unsafe extern "system" fn(
        total: i64,
        total_done: i64,
        stream: i64,
        stream_done: i64,
        stream_no: u32,
        reason: u32,
        src: *mut c_void,
        dest: *mut c_void,
        data: *mut c_void,
    ) -> u32;

    #[link(name = "kernel32")]
    extern "system" {
        fn CopyFileExW(
            existing: *const u16,
            new: *const u16,
            routine: Option<ProgressRoutine>,
            data: *mut c_void,
            cancel: *mut i32,
            flags: u32,
        ) -> i32;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *mut c_void,
            disposition: u32,
            flags: u32,
            template: *mut c_void,
        ) -> *mut c_void;
        fn DeviceIoControl(
            device: *mut c_void,
            code: u32,
            input: *const c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    /// 경로 → NUL로 끝나는 UTF-16. 긴 경로(MAX_PATH 근처 이상)는 `\\?\` 꼴로 바꿔 길이 제한을 벗어난다(std가 안에서 하는 일과 같다).
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        let abs = std::path::absolute(path)?;
        let raw: Vec<u16> = abs.as_os_str().encode_wide().collect();
        let verbatim: [u16; 4] = [92, 92, 63, 92]; // \\?\
        let mut out: Vec<u16> = if raw.len() < 248 || raw.starts_with(&verbatim) {
            raw
        } else if raw.starts_with(&[92, 92]) {
            let mut v: Vec<u16> = "\\\\?\\UNC\\".encode_utf16().collect();
            v.extend_from_slice(&raw[2..]);
            v
        } else {
            let mut v = verbatim.to_vec();
            v.extend_from_slice(&raw);
            v
        };
        if out.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "경로에 NUL 문자가 있음",
            ));
        }
        out.push(0);
        Ok(out)
    }

    struct Ctx<'a> {
        on_bytes: &'a mut dyn FnMut(u64),
        stop: Stop<'a>,
        /// 지금까지 보고한 기본 스트림 바이트.
        sent: u64,
    }

    /// `CopyFileExW` 진행 콜백 — 기본 데이터 스트림(1번)의 증분만 보고한다(대체 스트림은 계획한 총량에 없다).
    unsafe extern "system" fn progress(
        _total: i64,
        _total_done: i64,
        _stream: i64,
        stream_done: i64,
        stream_no: u32,
        _reason: u32,
        _src: *mut c_void,
        _dest: *mut c_void,
        data: *mut c_void,
    ) -> u32 {
        // SAFETY: `data`는 `copy_file`이 넘긴 `Ctx`를 가리키고, 그 호출이 끝날 때까지 살아 있다(콜백은 그 호출 안에서만 불린다).
        let ctx = unsafe { &mut *data.cast::<Ctx<'_>>() };
        if stream_no == 1 {
            let now = u64::try_from(stream_done).unwrap_or(0);
            if now > ctx.sent {
                (ctx.on_bytes)(now - ctx.sent);
                ctx.sent = now;
            }
        }
        if (ctx.stop)() {
            PROGRESS_CANCEL
        } else {
            PROGRESS_CONTINUE
        }
    }

    /// `CopyFileExW`로 파일 하나를 복사한다. 중단 · 실패 시 대상 파일은 Windows가 지운다(이미 있던 대상은 `FAIL_IF_EXISTS`라 그대로).
    pub(super) fn copy_file(
        src: &Path,
        dest: &Path,
        overwrite: bool,
        on_bytes: &mut dyn FnMut(u64),
        stop: Stop<'_>,
        unbuffered_min: u64,
    ) -> io::Result<()> {
        let len = std::fs::metadata(src)?.len();
        let (from, to) = (wide(src)?, wide(dest)?);
        let mut flags = 0;
        if !overwrite {
            flags |= COPY_FILE_FAIL_IF_EXISTS;
        }
        if unbuffered_min > 0 && len >= unbuffered_min {
            flags |= COPY_FILE_NO_BUFFERING;
        }
        let mut ctx = Ctx {
            on_bytes,
            stop,
            sent: 0,
        };
        let mut cancel = 0i32;
        // SAFETY: 두 경로는 NUL로 끝나는 UTF-16 버퍼이고 호출 동안 살아 있다 · `ctx` · `cancel`도 호출 동안 유효하다.
        let ok = unsafe {
            CopyFileExW(
                from.as_ptr(),
                to.as_ptr(),
                Some(progress),
                (&raw mut ctx).cast(),
                &raw mut cancel,
                flags,
            )
        };
        if ok == 0 {
            let e = io::Error::last_os_error();
            return Err(if e.raw_os_error() == Some(ERROR_REQUEST_ABORTED) {
                canceled()
            } else {
                e
            });
        }
        // 콜백이 마지막 증분을 건너뛴 경우(블록 복제 등)를 채운다 — 보고 합계 = 파일 크기.
        if ctx.sent < len {
            (ctx.on_bytes)(len - ctx.sent);
        }
        Ok(())
    }

    /// 커널 복사 경로가 이 파일 시스템 · 장치에서 지원되지 않는다는 오류인가(→ 읽고 쓰는 루프로 다시).
    pub(super) fn unsupported(e: &io::Error) -> bool {
        matches!(
            e.raw_os_error(),
            Some(ERROR_INVALID_FUNCTION | ERROR_NOT_SUPPORTED | ERROR_INVALID_PARAMETER)
        )
    }

    /// 경로가 놓인 저장 장치의 탐색 지연 여부 — 네트워크 공유(UNC) = 없음 · 드라이브 = `IOCTL_STORAGE_QUERY_PROPERTY`
    /// (`StorageDeviceSeekPenaltyProperty`) · 물어볼 수 없으면(이동식 등) 보수적으로 있음.
    pub(super) fn seek_of(path: &Path) -> Seek {
        let Ok(abs) = std::path::absolute(path) else {
            return Seek::Costly;
        };
        let letter = match abs.components().next() {
            Some(Component::Prefix(p)) => match p.kind() {
                Prefix::Disk(l) | Prefix::VerbatimDisk(l) => l,
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) => return Seek::Free,
                _ => return Seek::Costly,
            },
            _ => return Seek::Costly,
        };
        match seek_penalty(letter) {
            Some(false) => Seek::Free,
            _ => Seek::Costly,
        }
    }

    #[repr(C)]
    struct PropertyQuery {
        property_id: u32,
        query_type: u32,
        additional: [u8; 4],
    }

    #[repr(C)]
    #[derive(Default)]
    struct SeekPenaltyDescriptor {
        version: u32,
        size: u32,
        incurs_seek_penalty: u8,
    }

    fn seek_penalty(letter: u8) -> Option<bool> {
        let name: Vec<u16> = format!("\\\\.\\{}:\0", char::from(letter))
            .encode_utf16()
            .collect();
        // SAFETY: 이름은 NUL로 끝나는 UTF-16 · 접근 권한 0(질의 전용)이라 관리자 권한이 필요 없다.
        let h = unsafe {
            CreateFileW(
                name.as_ptr(),
                0,
                FILE_SHARE_READ_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if h.is_null() || h as isize == -1 {
            return None;
        }
        let query = PropertyQuery {
            property_id: STORAGE_DEVICE_SEEK_PENALTY_PROPERTY,
            query_type: 0,
            additional: [0; 4],
        };
        let mut desc = SeekPenaltyDescriptor::default();
        let mut returned = 0u32;
        // SAFETY: 입력 · 출력 버퍼는 선언한 크기 그대로이고 호출 동안 살아 있다 · 핸들은 위에서 연 것이다.
        let ok = unsafe {
            let ok = DeviceIoControl(
                h,
                IOCTL_STORAGE_QUERY_PROPERTY,
                (&raw const query).cast(),
                size_of::<PropertyQuery>() as u32,
                (&raw mut desc).cast(),
                size_of::<SeekPenaltyDescriptor>() as u32,
                &raw mut returned,
                std::ptr::null_mut(),
            );
            CloseHandle(h);
            ok
        };
        (ok != 0 && returned as usize >= size_of::<SeekPenaltyDescriptor>())
            .then_some(desc.incurs_seek_penalty != 0)
    }

    #[cfg(test)]
    pub(super) fn wide_for_test(path: &Path) -> Vec<u16> {
        wide(path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nexa_fast_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const GO: Stop<'static> = &|| false;

    fn tunings() -> Vec<Tuning> {
        vec![
            Tuning {
                native: true,
                unbuffered_min: 0,
                threads: 1,
            },
            // 비버퍼드 경로(1바이트 이상 전부).
            Tuning {
                native: true,
                unbuffered_min: 1,
                threads: 4,
            },
            Tuning {
                native: false,
                unbuffered_min: 0,
                threads: 3,
            },
        ]
    }

    /// 자동 스레드 수(MC/DC): 설정 우선 · 자동은 양쪽이 모두 탐색 지연 없음일 때만 여럿 · 코어 수 · 상한.
    #[test]
    fn thread_count_rules() {
        use Seek::{Costly, Free};
        assert_eq!(thread_count(0, Free, Free, 16), 8);
        assert_eq!(thread_count(0, Free, Free, 2), 2);
        assert_eq!(thread_count(0, Free, Free, 0), 1);
        assert_eq!(thread_count(0, Costly, Free, 16), 4);
        assert_eq!(thread_count(0, Free, Costly, 16), 4);
        assert_eq!(thread_count(0, Costly, Costly, 2), 2);
        assert_eq!(thread_count(1, Free, Free, 8), 1);
        assert_eq!(
            thread_count(6, Costly, Costly, 2),
            6,
            "설정이 판정을 이긴다"
        );
        assert_eq!(thread_count(99, Free, Free, 8), THREADS_MAX);
    }

    #[test]
    fn tuning_round_trip_clamps_threads() {
        let before = tuning();
        set_tuning(Tuning {
            native: false,
            unbuffered_min: 7,
            threads: 999,
        });
        let now = tuning();
        set_tuning(before);
        assert_eq!(
            now,
            Tuning {
                native: false,
                unbuffered_min: 7,
                threads: THREADS_MAX
            }
        );
    }

    /// 파일 1개: 내용 · 보고 합계 = 크기 · **수정 시각 보존** · `overwrite = false`가 있던 대상을 건드리지 않음 — 모든 전략에서 같다.
    #[test]
    fn copy_file_keeps_content_mtime_and_respects_existing() {
        for (k, t) in tunings().iter().enumerate() {
            let d = fixture(&format!("one{k}"));
            let src = d.join("a.bin");
            let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
            fs::write(&src, &data).unwrap();
            let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000);
            fs::File::options()
                .write(true)
                .open(&src)
                .unwrap()
                .set_modified(old)
                .unwrap();
            let dest = d.join("b.bin");
            let mut sum = 0u64;
            copy_file(&src, &dest, false, &mut |n| sum += n, GO, t).unwrap();
            assert_eq!(fs::read(&dest).unwrap(), data, "{t:?}");
            assert_eq!(sum, data.len() as u64, "{t:?}");
            let got = fs::metadata(&dest).unwrap().modified().unwrap();
            let diff = got
                .duration_since(old)
                .unwrap_or_else(|e| e.duration())
                .as_secs();
            assert!(diff <= 2, "수정 시각 보존 {t:?} — 차이 {diff}s");
            // 있는 대상 + overwrite = false → AlreadyExists · 대상 그대로.
            fs::write(&dest, b"keep").unwrap();
            let e = copy_file(&src, &dest, false, &mut |_| {}, GO, t).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::AlreadyExists, "{t:?}");
            assert_eq!(fs::read(&dest).unwrap(), b"keep");
            // overwrite = true → 바뀐다 · 빈 파일도 된다.
            copy_file(&src, &dest, true, &mut |_| {}, GO, t).unwrap();
            assert_eq!(fs::read(&dest).unwrap(), data);
            let empty = d.join("empty");
            fs::write(&empty, b"").unwrap();
            copy_file(&empty, &d.join("empty2"), false, &mut |_| {}, GO, t).unwrap();
            assert_eq!(fs::metadata(d.join("empty2")).unwrap().len(), 0);
            let _ = fs::remove_dir_all(&d);
        }
    }

    /// 중단: 첫 바이트 보고 뒤 멈추면 `Interrupted` · 만들던 대상 파일은 남지 않는다 · 시작 전 중단 = 대상 안 만듦.
    #[test]
    fn copy_file_stop_removes_partial() {
        for (k, t) in tunings().iter().enumerate() {
            let d = fixture(&format!("stop{k}"));
            let src = d.join("big.bin");
            fs::write(&src, vec![7u8; 24 * 1024 * 1024]).unwrap();
            let dest = d.join("out.bin");
            let hit = AtomicBool::new(false);
            let stop = || hit.load(Ordering::Relaxed);
            match copy_file(
                &src,
                &dest,
                false,
                &mut |_| hit.store(true, Ordering::Relaxed),
                &stop,
                t,
            ) {
                Err(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::Interrupted, "{t:?}");
                    assert!(!dest.exists(), "부분 파일 정리 {t:?}");
                }
                // 블록 복제(APFS clonefile · Linux reflink)는 **즉시** 끝나 중단할 틈이 없다 — 그때는 완전한 사본이어야 한다
                // (macOS CI 10-24 · 합계 보고는 성공 뒤라 첫 보고 때 이미 끝난 상태).
                Ok(()) => {
                    assert!(t.native, "루프 경로는 중단된다 {t:?}");
                    assert_eq!(fs::metadata(&dest).unwrap().len(), 24 * 1024 * 1024);
                    fs::remove_file(&dest).unwrap();
                }
            }
            assert!(hit.load(Ordering::Relaxed));
            let e = copy_file(&src, &dest, false, &mut |_| {}, &stop, t).unwrap_err();
            assert_eq!(e.kind(), io::ErrorKind::Interrupted);
            assert!(!dest.exists());
            let _ = fs::remove_dir_all(&d);
        }
    }

    /// Linux/macOS 네이티브 경로(T-164): 큰 파일(40 MiB · Linux는 32 MiB 청크 2회 이상) 내용 · 보고 합계 = 크기 · 심볼릭 링크 원본은
    /// **내용**을 복사(링크가 아님 · 다른 OS와 같게) · `overwrite = false`에 있는 대상 = AlreadyExists · `seek_of`는 어떤 경로에도 패닉 없음.
    #[cfg(unix)]
    #[test]
    fn unix_native_copy_large_file_symlink_and_seek() {
        let d = fixture("unixnative");
        let src = d.join("big.bin");
        let data: Vec<u8> = (0..40u32 * 1024 * 1024).map(|i| (i % 253) as u8).collect();
        fs::write(&src, &data).unwrap();
        let t = Tuning {
            native: true,
            unbuffered_min: 0,
            threads: 1,
        };
        let dest = d.join("copy.bin");
        let mut sum = 0u64;
        copy_file(&src, &dest, false, &mut |n| sum += n, GO, &t).unwrap();
        assert_eq!(sum, data.len() as u64);
        assert_eq!(fs::read(&dest).unwrap(), data);
        let e = copy_file(&src, &dest, false, &mut |_| {}, GO, &t).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::AlreadyExists);
        let link = d.join("link.bin");
        std::os::unix::fs::symlink(&src, &link).unwrap();
        let dest2 = d.join("via-link.bin");
        let mut sum2 = 0u64;
        copy_file(&link, &dest2, false, &mut |n| sum2 += n, GO, &t).unwrap();
        assert!(!fs::symlink_metadata(&dest2)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::metadata(&dest2).unwrap().len(), data.len() as u64);
        assert_eq!(sum2, data.len() as u64);
        let _ = os::seek_of(&d);
        let _ = os::seek_of(Path::new("/nonexistent/x/y"));
        let _ = fs::remove_dir_all(&d);
    }

    /// Linux: `copy_file_range`가 처음부터 0을 돌려주는 의사 파일(procfs)은 빈 대상으로 "성공"하지 않고 루프로 떨어져 내용이 복사된다
    /// (협업 리뷰 ① · std `kernel_copy` 규칙) · 네이티브 경로의 `Unsupported`가 상위 폴백으로 이어진다.
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_pseudo_file_falls_back_to_loop() {
        let d = fixture("procfs");
        let t = Tuning {
            native: true,
            unbuffered_min: 0,
            threads: 1,
        };
        let dest = d.join("status.txt");
        let mut sum = 0u64;
        copy_file(
            Path::new("/proc/self/status"),
            &dest,
            false,
            &mut |n| sum += n,
            GO,
            &t,
        )
        .unwrap();
        let got = fs::read_to_string(&dest).unwrap();
        assert!(got.contains("Pid:"), "{got}");
        assert_eq!(sum, got.len() as u64);
        let _ = fs::remove_dir_all(&d);
    }

    fn make_tree(root: &Path, dirs: usize, files: usize) -> u64 {
        let mut total = 0u64;
        for a in 0..dirs {
            let dir = root.join(format!("d{a}")).join("sub");
            fs::create_dir_all(&dir).unwrap();
            for f in 0..files {
                let body = format!("{a}-{f}-").repeat(1 + (a * 31 + f) % 40);
                total += body.len() as u64;
                fs::write(dir.join(format!("f{f}.txt")), body).unwrap();
            }
        }
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::write(root.join("top.txt"), b"top").unwrap();
        total + 3
    }

    fn assert_same_tree(a: &Path, b: &Path) {
        for e in fs::read_dir(a).unwrap() {
            let e = e.unwrap();
            let other = b.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                assert!(other.is_dir(), "{}", other.display());
                assert_same_tree(&e.path(), &other);
            } else {
                assert_eq!(fs::read(e.path()).unwrap(), fs::read(&other).unwrap());
            }
        }
    }

    /// 폴더: 차례로 · 병렬 모두 같은 결과 — 내용 · 빈 폴더 · 보고 합계 · `copied` = 모든 파일과 폴더(폴더는 자기 안의 것 **뒤**).
    #[test]
    fn copy_tree_sequential_and_parallel_agree() {
        for (k, t) in tunings().iter().enumerate() {
            let d = fixture(&format!("tree{k}"));
            let src = d.join("src");
            let total = make_tree(&src, 6, 25);
            let dest = d.join("dest");
            let mut sum = 0u64;
            let mut copied = Vec::new();
            copy_tree(&src, &dest, &mut |n| sum += n, GO, Some(&mut copied), t).unwrap();
            assert_eq!(sum, total, "{t:?}");
            assert_same_tree(&src, &dest);
            assert!(dest.join("empty").is_dir());
            // 6 × 25 파일 + top.txt + 폴더(d0..d5 · 각 sub · empty) 13.
            assert_eq!(copied.len(), 6 * 25 + 1 + 13, "{t:?}");
            for (i, p) in copied.iter().enumerate() {
                if p.is_dir() {
                    assert!(
                        copied[i + 1..].iter().all(|q| !q.starts_with(p) || q == p),
                        "후위 순서: {} 뒤에 그 안의 항목이 온다",
                        p.display()
                    );
                }
            }
            let _ = fs::remove_dir_all(&d);
        }
    }

    /// 병렬 복사 중 중단 → `Interrupted` · 병렬 복사 중 한 파일 실패 → 그 오류(취소보다 우선).
    #[test]
    fn copy_tree_parallel_stop_and_failure() {
        let t = Tuning {
            native: true,
            unbuffered_min: 0,
            threads: 4,
        };
        let d = fixture("treestop");
        let src = d.join("src");
        make_tree(&src, 8, 40);
        // 중단 신호는 **판정 횟수**로 낸다(진행 통지 횟수로 내면 작업 스레드가 통지보다 먼저 다 끝낼 수 있다 — macOS CI에서
        // 실제로 흔들렸다). 훑기가 항목당 1번(338번) 묻고, 그 뒤 작업 스레드가 파일마다 2번 이상 묻는다 → 400번째 = 복사 도중.
        let polls = AtomicUsize::new(0);
        let stop = || polls.fetch_add(1, Ordering::Relaxed) >= 400;
        let e = copy_tree(&src, &d.join("dest"), &mut |_| {}, &stop, None, &t).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::Interrupted);
        assert!(
            d.join("dest").join("d0").is_dir(),
            "훑기는 끝났다(폴더는 만들어졌다) = 복사 단계에서 멈춘 것"
        );
        // 실패: 대상 자리에 같은 이름의 **폴더**가 있으면 그 파일은 쓸 수 없다.
        let dest = d.join("dest2");
        fs::create_dir_all(dest.join("d3").join("sub").join("f7.txt")).unwrap();
        let e = copy_tree(&src, &dest, &mut |_| {}, GO, None, &t).unwrap_err();
        assert_ne!(e.kind(), io::ErrorKind::Interrupted, "{e}");
        let _ = fs::remove_dir_all(&d);
    }

    /// Windows: 긴 경로(260자 초과)는 `\\?\` 꼴로 바뀌어 `CopyFileExW`가 받는다 · 짧은 경로는 그대로.
    #[cfg(windows)]
    #[test]
    fn long_paths_copy_natively() {
        let d = fixture("long");
        let mut deep = d.clone();
        for _ in 0..7 {
            deep.push("가나다라마바사아자차카타파하-0123456789-abcdefghijklmnop");
        }
        fs::create_dir_all(&deep).unwrap();
        assert!(deep.as_os_str().len() > 260);
        let src = d.join("s.txt");
        fs::write(&src, b"long path").unwrap();
        let dest = deep.join("copy.txt");
        let w = os::wide_for_test(&dest);
        assert!(String::from_utf16_lossy(&w).starts_with("\\\\?\\"));
        assert!(!String::from_utf16_lossy(&os::wide_for_test(&src)).starts_with("\\\\?\\"));
        let t = Tuning {
            native: true,
            unbuffered_min: 0,
            threads: 1,
        };
        copy_file(&src, &dest, false, &mut |_| {}, GO, &t).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"long path");
        copy_file(&dest, &d.join("back.txt"), false, &mut |_| {}, GO, &t).unwrap();
        assert_eq!(fs::read(d.join("back.txt")).unwrap(), b"long path");
        let _ = fs::remove_dir_all(&d);
    }

    /// 성능 비교(수동 · `cargo test -p ndir-ops --release bench_copy -- --ignored --nocapture`).
    /// `NDIR_BENCH_DIR`(없으면 임시 폴더) 아래에 작은 파일 4,000개 + 큰 파일 1개(512 MiB)를 만들어 전략별 시간을 잰다.
    #[test]
    #[ignore = "수동 성능 측정"]
    fn bench_copy() {
        let root = std::env::var_os("NDIR_BENCH_DIR")
            .map_or_else(std::env::temp_dir, PathBuf::from)
            .join(format!("nexa_fast_bench_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let small = root.join("small");
        for a in 0..40 {
            let dir = small.join(format!("d{a}"));
            fs::create_dir_all(&dir).unwrap();
            for f in 0..100 {
                fs::write(
                    dir.join(format!("f{f}.dat")),
                    vec![(a + f) as u8; 16 * 1024],
                )
                .unwrap();
            }
        }
        let big = root.join("big.bin");
        {
            let mut f = fs::File::create(&big).unwrap();
            let chunk = vec![0x5Au8; 8 * 1024 * 1024];
            for _ in 0..64 {
                f.write_all(&chunk).unwrap();
            }
        }
        let cases = [
            ("루프 · 1스레드(종전)", false, 0u64, 1usize),
            ("CopyFileEx · 1스레드", true, 0, 1),
            ("CopyFileEx · 4스레드", true, 0, 4),
            ("CopyFileEx · 8스레드", true, 0, 8),
            ("루프 · 4스레드", false, 0, 4),
        ];
        for (i, (name, native, unbuf, threads)) in cases.iter().enumerate() {
            let t = Tuning {
                native: *native,
                unbuffered_min: *unbuf,
                threads: *threads,
            };
            let dest = root.join(format!("out{i}"));
            let at = std::time::Instant::now();
            copy_tree(&small, &dest, &mut |_| {}, GO, None, &t).unwrap();
            println!(
                "작은 파일 4000개  {name:<28} {:>7} ms",
                at.elapsed().as_millis()
            );
        }
        for (i, (name, native, unbuf)) in [
            ("루프(종전)", false, 0u64),
            ("CopyFileEx 버퍼드", true, 0),
            ("CopyFileEx 비버퍼드", true, 1),
        ]
        .iter()
        .enumerate()
        {
            let t = Tuning {
                native: *native,
                unbuffered_min: *unbuf,
                threads: 1,
            };
            let dest = root.join(format!("big{i}.bin"));
            let at = std::time::Instant::now();
            copy_file(&big, &dest, false, &mut |_| {}, GO, &t).unwrap();
            println!(
                "큰 파일 512 MiB   {name:<28} {:>7} ms",
                at.elapsed().as_millis()
            );
        }
        println!("seek: {:?}", os::seek_of(&root));
        let _ = fs::remove_dir_all(&root);
    }
}
