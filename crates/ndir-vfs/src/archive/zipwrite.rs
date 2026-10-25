//! zip **쓰기**(DR-31 ⑳ⓑ · 10-25 · "Linux 압축하기 없음" — 외부 아카이버가 없어도 폴더/파일을 zip으로): 저장(store) 방식만(외부 crate 0 · DR-8) ·
//! CRC-32 · UTF-8 이름 깃발(0x800) · 항목마다 **데이터 서술자**(깃발 비트 3 — 파일을 한 번만 읽는다) · 폴더는 재귀(`dir/` 항목 포함) ·
//! ZIP64 없음(파일 하나 또는 전체가 4 GiB를 넘으면 그 항목만 `too_big`에 적고 건너뛴다) · 취소 = 항목 경계.
//! 읽기 쪽(`zip.rs` · `extract.rs`)이 그대로 열 수 있다(왕복 시험).

use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::extract::crc32_update;

/// 쓰기 결과.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WriteReport {
    pub files: u64,
    pub dirs: u64,
    /// 원본 바이트(= 저장 바이트 · 압축 없음).
    pub bytes: u64,
    /// 4 GiB 이상이라 건너뛴 항목.
    pub too_big: Vec<PathBuf>,
    /// 항목별 실패(경로 · 사유).
    pub errors: Vec<(PathBuf, String)>,
    pub canceled: bool,
}

const SIG_LOCAL: u32 = 0x0403_4b50;
const SIG_CENTRAL: u32 = 0x0201_4b50;
const SIG_END: u32 = 0x0605_4b50;
const SIG_DESC: u32 = 0x0807_4b50;
const FLAG_DESC: u16 = 1 << 3;
const FLAG_UTF8: u16 = 1 << 11;
const VERSION: u16 = 20;
const LIMIT: u64 = u32::MAX as u64;

struct Central {
    name: Vec<u8>,
    crc: u32,
    size: u32,
    is_dir: bool,
    local_off: u32,
    dos_time: u16,
    dos_date: u16,
}

fn w16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn w32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// 수정 시각 → DOS (time, date). 모르면 1980-01-01 00:00.
fn dos_datetime(meta: &std::fs::Metadata) -> (u16, u16) {
    let secs = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    // 현지 시간대 없이 UTC(zip은 현지 시각이지만 시간대 정보가 없다 — 읽는 쪽도 그대로 보여 준다).
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (
        (rem / 3600) as u16,
        ((rem % 3600) / 60) as u16,
        (rem % 60) as u16,
    );
    // 1970-01-01 기준 일수 → 연월일(그레고리력).
    let (mut y, mut mo, mut d) = (1970i64, 1i64, 1i64);
    {
        let mut n = days;
        loop {
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            let yl = if leap { 366 } else { 365 };
            if n < yl {
                break;
            }
            n -= yl;
            y += 1;
        }
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let ml = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        for len in ml {
            if n < len {
                break;
            }
            n -= len;
            mo += 1;
        }
        d += n;
    }
    if y < 1980 {
        return (0, (1 << 5) | 1);
    }
    let time = (h << 11) | (m << 5) | (s / 2);
    let date = (((y - 1980) as u16) << 9) | ((mo as u16) << 5) | d as u16;
    (time, date)
}

/// `name` = zip 안 경로(`/` 구분 · 폴더는 끝에 `/`).
fn local_header(out: &mut Vec<u8>, name: &[u8], is_dir: bool, dt: (u16, u16)) {
    w32(out, SIG_LOCAL);
    w16(out, VERSION);
    w16(out, FLAG_UTF8 | if is_dir { 0 } else { FLAG_DESC });
    w16(out, 0); // store
    w16(out, dt.0);
    w16(out, dt.1);
    w32(out, 0); // crc(서술자)
    w32(out, 0); // csize
    w32(out, 0); // usize
    w16(out, name.len() as u16);
    w16(out, 0); // extra
    out.extend_from_slice(name);
}

/// 항목 목록을 모은다(파일 · 폴더 재귀 · 이름은 `base` 기준 상대 경로 · `/`).
fn collect(src: &Path, base: &Path, out: &mut Vec<(PathBuf, String, bool)>, rep: &mut WriteReport) {
    let rel = src
        .strip_prefix(base)
        .unwrap_or_else(|_| Path::new(src.file_name().unwrap_or_default()));
    let mut name = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let meta = match std::fs::symlink_metadata(src) {
        Ok(m) => m,
        Err(e) => {
            rep.errors.push((src.to_path_buf(), e.to_string()));
            return;
        }
    };
    if meta.file_type().is_symlink() {
        return; // 링크는 건너뜀(풀기 쪽도 지원하지 않는다)
    }
    if meta.is_dir() {
        if !name.is_empty() {
            name.push('/');
            out.push((src.to_path_buf(), name, true));
        }
        let mut kids: Vec<PathBuf> = match std::fs::read_dir(src) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(e) => {
                rep.errors.push((src.to_path_buf(), e.to_string()));
                return;
            }
        };
        kids.sort();
        for k in kids {
            collect(&k, base, out, rep);
        }
    } else {
        out.push((src.to_path_buf(), name, false));
    }
}

/// 선택한 경로들(`sources` · 같은 부모 폴더 아래라고 가정 — 이름은 **그 부모 기준**)을 `dest`(zip 파일)로. `on_bytes` = 진행(누적 원본 바이트) ·
/// `cancel` = 항목 경계에서 멈추고 부분 파일을 지운다.
pub fn write_zip(
    sources: &[PathBuf],
    dest: &Path,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> io::Result<WriteReport> {
    let mut rep = WriteReport::default();
    let base = sources
        .first()
        .and_then(|p| p.parent())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut entries = Vec::new();
    for s in sources {
        collect(s, &base, &mut entries, &mut rep);
    }
    let mut file = std::fs::File::create(dest)?;
    let mut central: Vec<Central> = Vec::new();
    let mut offset: u64 = 0;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    for (path, name, is_dir) in entries {
        if cancel.load(Ordering::Relaxed) {
            rep.canceled = true;
            break;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                rep.errors.push((path.clone(), e.to_string()));
                continue;
            }
        };
        if !is_dir && meta.len() > LIMIT {
            rep.too_big.push(path.clone());
            continue;
        }
        if offset > LIMIT {
            rep.too_big.push(path.clone());
            continue;
        }
        let dt = dos_datetime(&meta);
        let nb = name.as_bytes().to_vec();
        let mut head = Vec::with_capacity(30 + nb.len());
        local_header(&mut head, &nb, is_dir, dt);
        file.write_all(&head)?;
        let local_off = offset as u32;
        offset += head.len() as u64;
        // CRC-32 = 시작 0xFFFF_FFFF · 끝에 반전(`crc32_update` 규약 · 폴더 항목은 0).
        let (mut crc, mut size) = (0u32, 0u64);
        if is_dir {
            rep.dirs += 1;
        } else {
            crc = 0xFFFF_FFFF;
            let mut f = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    rep.errors.push((path.clone(), e.to_string()));
                    // 머리만 쓰인 항목 = 크기 0으로 마감(읽는 쪽이 빈 파일로 본다).
                    let mut desc = Vec::with_capacity(16);
                    w32(&mut desc, SIG_DESC);
                    w32(&mut desc, 0);
                    w32(&mut desc, 0);
                    w32(&mut desc, 0);
                    file.write_all(&desc)?;
                    offset += desc.len() as u64;
                    central.push(Central {
                        name: nb,
                        crc: 0,
                        size: 0,
                        is_dir: false,
                        local_off,
                        dos_time: dt.0,
                        dos_date: dt.1,
                    });
                    continue;
                }
            };
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                crc = crc32_update(crc, &buf[..n]);
                size += n as u64;
                file.write_all(&buf[..n])?;
                done += n as u64;
                on_bytes(done);
            }
            offset += size;
            crc = !crc;
            let mut desc = Vec::with_capacity(16);
            w32(&mut desc, SIG_DESC);
            w32(&mut desc, crc);
            w32(&mut desc, size as u32);
            w32(&mut desc, size as u32);
            file.write_all(&desc)?;
            offset += desc.len() as u64;
            rep.files += 1;
            rep.bytes += size;
        }
        central.push(Central {
            name: nb,
            crc,
            size: size as u32,
            is_dir,
            local_off,
            dos_time: dt.0,
            dos_date: dt.1,
        });
    }
    if rep.canceled {
        drop(file);
        let _ = std::fs::remove_file(dest);
        return Ok(rep);
    }
    // 중앙 디렉터리.
    let cd_off = offset;
    let mut cd = Vec::new();
    for c in &central {
        w32(&mut cd, SIG_CENTRAL);
        w16(&mut cd, VERSION); // made by
        w16(&mut cd, VERSION); // needed
        w16(&mut cd, FLAG_UTF8 | if c.is_dir { 0 } else { FLAG_DESC });
        w16(&mut cd, 0);
        w16(&mut cd, c.dos_time);
        w16(&mut cd, c.dos_date);
        w32(&mut cd, c.crc);
        w32(&mut cd, c.size);
        w32(&mut cd, c.size);
        w16(&mut cd, c.name.len() as u16);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w16(&mut cd, 0);
        w32(&mut cd, if c.is_dir { 0x10 } else { 0 }); // external attrs(DOS 폴더 비트)
        w32(&mut cd, c.local_off);
        cd.extend_from_slice(&c.name);
    }
    file.write_all(&cd)?;
    let mut end = Vec::with_capacity(22);
    w32(&mut end, SIG_END);
    w16(&mut end, 0);
    w16(&mut end, 0);
    w16(&mut end, central.len() as u16);
    w16(&mut end, central.len() as u16);
    w32(&mut end, cd.len() as u32);
    w32(&mut end, cd_off as u32);
    w16(&mut end, 0);
    file.write_all(&end)?;
    file.flush()?;
    let _ = file.seek(io::SeekFrom::End(0));
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 왕복: 폴더(하위 폴더 · 파일 2 · 빈 폴더) + 파일 1 → zip → 기존 풀기(`extract`)로 풀어 내용 · 수가 같다 · UTF-8 이름 · 취소 = 파일 없음.
    #[test]
    fn write_then_extract_round_trip() {
        let base = std::env::temp_dir().join(format!("ndir-zipw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(src.join("보고서/깊은")).unwrap();
        std::fs::create_dir_all(src.join("empty")).unwrap();
        std::fs::write(src.join("보고서/a.txt"), b"hello zip").unwrap();
        std::fs::write(src.join("보고서/깊은/b.bin"), vec![7u8; 70_000]).unwrap();
        std::fs::write(src.join("top.md"), b"# top").unwrap();
        let dest = base.join("out.zip");
        let mut last = 0;
        let rep = write_zip(
            &[src.join("보고서"), src.join("empty"), src.join("top.md")],
            &dest,
            &mut |b| last = b,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!((rep.files, rep.dirs), (3, 3), "{rep:?}");
        assert_eq!(rep.bytes, 9 + 70_000 + 5);
        assert_eq!(last, rep.bytes, "진행 = 누적 바이트");
        assert!(rep.errors.is_empty() && rep.too_big.is_empty() && !rep.canceled);
        let out = base.join("unz");
        let r = super::super::extract::extract(&dest, &out, &mut |_| {}, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(r.files, 3, "{r:?}");
        assert_eq!(
            std::fs::read(out.join("보고서/a.txt")).unwrap(),
            b"hello zip"
        );
        assert_eq!(
            std::fs::read(out.join("보고서/깊은/b.bin")).unwrap().len(),
            70_000
        );
        assert_eq!(std::fs::read(out.join("top.md")).unwrap(), b"# top");
        assert!(out.join("empty").is_dir(), "빈 폴더도 항목으로");
        // 취소 = 부분 파일 제거.
        let dest2 = base.join("canceled.zip");
        let rep2 = write_zip(
            &[src.join("보고서")],
            &dest2,
            &mut |_| {},
            &AtomicBool::new(true),
        )
        .unwrap();
        assert!(rep2.canceled && !dest2.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
