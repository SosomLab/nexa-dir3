//! App — 폴더 크기 계산(T-166 · NEW-037 · dir3 신규 — dir2와 탐색기 목록에는 없고 탐색기 속성 창에만 있다 · 사용자 10-05 ·
//! **10-06 작업 큐 개편**).
//!
//! 정보 도크에 **폴더 1개**가 보이면 그 트리를 훑어 바이트 · 파일 수 · 폴더 수를 재고, 재는 동안은 진행값을 보여 준다.
//! - **작업 큐 + 동시 N개**(사용자 10-06 "100개 폴더를 차례로 클릭하면 프로세스·디스크를 과하게 쓴다"): 요청은 큐에 들어가고
//!   작업 스레드는 설정 `dock.folder_size_threads`(기본 2)개까지만 — 하나가 끝나면 다음 하나를 꺼낸다. 지금 보고 있는 폴더는 큐
//!   **맨 앞**으로(다음 빈 자리를 먼저 받는다) · 큐 상한(`dock.folder_size_queue`)을 넘으면 가장 오래된 것부터 버린다 ·
//!   스레드 우선순위는 낮춤.
//! - **빠른 이동 건너뛰기**(`dock.folder_size_settle_ms` · 기본 250): 직전 요청에서 이 시간이 안 지났으면(키보드로 빠르게
//!   지나가는 중 · 빠른 스크롤) 큐에 넣지 않고 **마지막 것만 보류**해 두었다가 멈추면 넣는다 — 지나친 폴더의 일은 아예
//!   생기지 않는다.
//! - 상태 = 측정 대기 중(큐) → 계산 중(진행값) → 끝(캐시). 캐시는 경로별 · 그 트리에 변경이 닿으면(폴더 감시 · 전송 ·
//!   삭제 · 이름 변경) 버린다 · 설정 `dock.folder_size`로 끌 수 있고 성능 향상 모드에서는 끈다. 링크(심볼릭 링크 · 정션)는
//!   따라가지 않는다(대상이 다른 곳에 있다 — 전송 엔진과 같은 규칙).

use crate::*;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// 작업 스레드 ↔ UI 공유 상태(진행값은 훑는 동안 계속 커진다).
#[derive(Debug, Default)]
pub(crate) struct DirSizeShared {
    cancel: AtomicBool,
    bytes: AtomicU64,
    files: AtomicU64,
    dirs: AtomicU64,
    /// 읽지 못한 폴더 수(권한 · 사라짐) — 0이 아니면 결과는 "일부".
    errors: AtomicU64,
    done: AtomicBool,
}

/// 진행 중인 계산 1건.
#[derive(Debug)]
pub(crate) struct DirSizeJob {
    pub(crate) path: PathBuf,
    shared: Arc<DirSizeShared>,
}

/// 끝난 계산.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirSize {
    pub(crate) bytes: u64,
    pub(crate) files: u64,
    pub(crate) dirs: u64,
    /// 읽지 못한 폴더가 있었다.
    pub(crate) partial: bool,
}

/// 경로 하나의 상태.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SizeState {
    /// 끝남(캐시).
    Done(DirSize),
    /// 재는 중(지금까지의 진행값).
    Running(DirSize),
    /// 큐에서 차례를 기다리는 중(보류 포함).
    Queued,
    /// 모름(요청한 적 없음 · 꺼짐).
    Unknown,
}

/// 조절값(설정 `dock.folder_size_*` · 고급).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tuning {
    /// 동시에 재는 폴더 수(1 이상).
    pub(crate) threads: usize,
    /// 큐 상한(넘으면 가장 오래된 것부터 버림 · 1 이상).
    pub(crate) queue_max: usize,
    /// 빠른 이동 판정(ms · 직전 요청에서 이 시간이 안 지났으면 보류 · 0 = 늘 즉시).
    pub(crate) settle_ms: u64,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            threads: 2,
            queue_max: 50,
            settle_ms: 250,
        }
    }
}

/// 폴더 크기 상태(App 필드 하나).
#[derive(Debug, Default)]
pub(crate) struct DirSizes {
    /// 재는 중(≤ `threads`).
    running: Vec<DirSizeJob>,
    /// 차례를 기다리는 경로(앞 = 다음 차례).
    queue: VecDeque<PathBuf>,
    cache: HashMap<PathBuf, DirSize>,
    /// 빠른 이동 중 보류한 마지막 요청(경로 · 시각) — settle이 지나면 큐에 넣는다.
    pending: Option<(PathBuf, Instant)>,
    /// 마지막 요청 시각(빠른 이동 판정).
    last_request: Option<Instant>,
    tuning: Tuning,
    /// 표시 조절(10-08 프레임 계측 적발): 상태 전이(끝남 · 시작 · 큐 변화)가 있었다 → 다음 표시는 즉시.
    changed: bool,
    /// 마지막으로 도크에 반영한 시각 — 재는 중의 진행값("지금까지 …")은 [`DISPLAY_EVERY_MS`]마다만. 종전에는 유휴 틱(사건마다 =
    /// 그리기 뒤에도)마다 다시 그려 **그리기 → 틱 → 그리기** 고리로 재는 내내 120~170 fps가 됐다(docs/25 §8-1).
    last_shown: Option<Instant>,
}

/// 재는 중 진행값 표시 간격(ms).
pub(crate) const DISPLAY_EVERY_MS: u64 = 250;

/// 도크에 반영할 때인가(순수 · MC/DC): 상태 전이가 있으면 즉시 · 재는 중이면 마지막 표시에서 `every_ms` 이상 지났을 때 ·
/// 둘 다 아니면(유휴 · 방금 보여 줌) 아니다.
pub(crate) fn display_due(
    changed: bool,
    busy: bool,
    last_shown: Option<Instant>,
    now: Instant,
    every_ms: u64,
) -> bool {
    changed
        || (busy && last_shown.is_none_or(|t| now.duration_since(t).as_millis() as u64 >= every_ms))
}

/// 캐시 항목 `key`가 `changed` 폴더의 변경에 영향을 받는가(순수): 바뀐 폴더가 항목 자신이거나 그 안(조상 항목의 합계가 변함) ·
/// 또는 항목이 바뀐 폴더 안(항목 자체가 지워지거나 바뀌었을 수 있다).
pub(crate) fn stale(key: &Path, changed: &Path) -> bool {
    changed.starts_with(key) || key.starts_with(changed)
}

/// 빠른 이동 판정(순수 · MC/DC): settle 0 = 늘 즉시 · 첫 요청 = 즉시 · 직전 요청에서 settle 이상 지났으면 즉시 · 아니면 보류.
pub(crate) fn settled(now: Instant, last: Option<Instant>, settle_ms: u64) -> bool {
    settle_ms == 0 || last.is_none_or(|l| now.duration_since(l).as_millis() as u64 >= settle_ms)
}

/// 트리를 훑는다(작업 스레드 · 반복문 — 깊은 트리에서 스택을 쓰지 않는다). 링크는 세지도 따라가지도 않는다 ·
/// 열거 실패는 `errors`에 세고 계속 · `cancel`이면 바로 그친다.
pub(crate) fn walk(root: &Path, sh: &DirSizeShared) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if sh.cancel.load(Ordering::Relaxed) {
            return;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            sh.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        for e in rd {
            if sh.cancel.load(Ordering::Relaxed) {
                return;
            }
            let Ok(e) = e else {
                sh.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let Ok(ft) = e.file_type() else {
                sh.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let p = e.path();
            if ft.is_symlink() || ndir_ops::is_link(&p) {
                continue;
            }
            if ft.is_dir() {
                sh.dirs.fetch_add(1, Ordering::Relaxed);
                stack.push(p);
            } else {
                sh.files.fetch_add(1, Ordering::Relaxed);
                if let Ok(m) = e.metadata() {
                    sh.bytes.fetch_add(m.len(), Ordering::Relaxed);
                }
            }
        }
    }
}

impl DirSizeShared {
    fn snapshot(&self) -> DirSize {
        DirSize {
            bytes: self.bytes.load(Ordering::Relaxed),
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            partial: self.errors.load(Ordering::Relaxed) > 0,
        }
    }
}

impl DirSizes {
    /// 조절값(설정 변경 때 · 재는 중인 작업은 그대로 끝까지).
    pub(crate) fn set_tuning(&mut self, t: Tuning) {
        self.tuning = Tuning {
            threads: t.threads.max(1),
            queue_max: t.queue_max.max(1),
            settle_ms: t.settle_ms,
        };
    }

    /// `path`의 크기를 요청한다 — 캐시에 있거나 재는 중이면 무동작 · 빠른 이동 중이면 보류(마지막 것만) · 아니면 큐 맨 앞에.
    pub(crate) fn request(&mut self, path: &Path, now: Instant) {
        if self.cache.contains_key(path) || self.running.iter().any(|j| j.path == path) {
            return;
        }
        let ok = settled(now, self.last_request, self.tuning.settle_ms);
        self.last_request = Some(now);
        if ok {
            self.pending = None;
            self.enqueue_front(path.to_path_buf());
            self.fill();
        } else {
            self.pending = Some((path.to_path_buf(), now));
        }
    }

    /// 큐 맨 앞에(이미 있으면 앞으로 옮김) · 상한을 넘으면 뒤(가장 오래된 것)부터 버림.
    fn enqueue_front(&mut self, path: PathBuf) {
        self.queue.retain(|p| *p != path);
        self.queue.push_front(path);
        while self.queue.len() > self.tuning.queue_max {
            self.queue.pop_back();
        }
    }

    /// 빈 자리만큼 큐에서 꺼내 작업 스레드를 띄운다.
    fn fill(&mut self) {
        while self.running.len() < self.tuning.threads {
            let Some(path) = self.queue.pop_front() else {
                break;
            };
            let shared = Arc::new(DirSizeShared::default());
            let sh = Arc::clone(&shared);
            let root = path.clone();
            std::thread::spawn(move || {
                platform::lower_thread_priority(); // UI · 다른 작업에 양보
                walk(&root, &sh);
                sh.done.store(true, Ordering::Relaxed);
            });
            self.running.push(DirSizeJob { path, shared });
        }
    }

    /// 보고 있는 폴더가 없다 — 보류만 버린다(큐와 진행 중인 작업은 계속 간다 · 결과는 캐시로).
    pub(crate) fn idle(&mut self) {
        self.pending = None;
    }

    /// 폴링: 끝난 작업을 캐시로 옮기고 · 보류가 멈춤 기준을 넘었으면 큐에 넣고 · 빈 자리를 채운다.
    /// 돌려주는 값 = 아직 할 일이 있다(깨움이 필요하다).
    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        let before = (self.running.len(), self.queue.len(), self.cache.len());
        let (done, still): (Vec<DirSizeJob>, Vec<DirSizeJob>) = self
            .running
            .drain(..)
            .partition(|j| j.shared.done.load(Ordering::Relaxed));
        self.running = still;
        for j in done {
            if !j.shared.cancel.load(Ordering::Relaxed) {
                self.cache.insert(j.path, j.shared.snapshot());
            }
        }
        if let Some((p, at)) = &self.pending {
            if now.duration_since(*at).as_millis() as u64 >= self.tuning.settle_ms {
                let p = p.clone();
                self.pending = None;
                if !self.cache.contains_key(&p) {
                    self.enqueue_front(p);
                }
            }
        }
        self.fill();
        if before != (self.running.len(), self.queue.len(), self.cache.len()) {
            self.changed = true;
        }
        self.busy()
    }

    /// 지금 도크에 반영해야 하는가([`display_due`]) — 참을 돌려주면 "보여 줬다"로 기록한다(호스트는 그때만 `update_docks` + 다시 그리기).
    pub(crate) fn take_display_due(&mut self, now: Instant) -> bool {
        if display_due(
            self.changed,
            self.busy(),
            self.last_shown,
            now,
            DISPLAY_EVERY_MS,
        ) {
            self.changed = false;
            self.last_shown = Some(now);
            true
        } else {
            false
        }
    }

    /// 할 일이 있는가(재는 중 · 큐 · 보류) — 시험 판정용(호스트는 `tick`의 반환값과 `take_display_due`만 쓴다).
    #[cfg(test)]
    pub(crate) fn running(&self) -> bool {
        self.busy()
    }

    fn busy(&self) -> bool {
        !self.running.is_empty() || !self.queue.is_empty() || self.pending.is_some()
    }

    /// `path`의 상태.
    pub(crate) fn get(&self, path: &Path) -> SizeState {
        if let Some(d) = self.cache.get(path) {
            return SizeState::Done(*d);
        }
        if let Some(j) = self.running.iter().find(|j| j.path == path) {
            return SizeState::Running(j.shared.snapshot());
        }
        if self.queue.iter().any(|p| p == path)
            || self.pending.as_ref().is_some_and(|(p, _)| p == path)
        {
            return SizeState::Queued;
        }
        SizeState::Unknown
    }

    /// 바뀐 폴더들에 닿는 캐시 항목 · 큐 항목 · 보류를 버리고, 닿는 작업은 취소한다(다음 도크 갱신이 다시 요청한다).
    pub(crate) fn invalidate(&mut self, changed: &[PathBuf]) {
        let hit = |p: &Path| changed.iter().any(|c| stale(p, c));
        self.cache.retain(|k, _| !hit(k));
        self.queue.retain(|p| !hit(p));
        if self.pending.as_ref().is_some_and(|(p, _)| hit(p)) {
            self.pending = None;
        }
        self.running.retain(|j| {
            if hit(&j.path) {
                j.shared.cancel.store(true, Ordering::Relaxed);
                false
            } else {
                true
            }
        });
        self.fill();
    }

    /// 전부 버린다(전송 · 삭제 · 이름 변경 뒤 · 설정 끔 — 어디가 바뀌었는지 일일이 따지지 않는다).
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.queue.clear();
        self.pending = None;
        for j in self.running.drain(..) {
            j.shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    #[cfg(test)]
    pub(crate) fn cached(&self, path: &Path) -> Option<DirSize> {
        self.cache.get(path).copied()
    }

    #[cfg(test)]
    pub(crate) fn queue_len(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    pub(crate) fn running_len(&self) -> usize {
        self.running.len()
    }
}

/// 정보 도크의 크기 줄(순수): 끝남 = "크기: …" + "포함: 파일 N개, 폴더 M개"(일부면 표시) · 재는 중 = "크기: 계산 중… 지금까지" ·
/// 대기 = "크기: 측정 대기 중…" · 모름 = 없음.
pub(crate) fn size_lines(state: SizeState) -> Vec<String> {
    let long = crate::dockinfo::fmt_size_long;
    match state {
        SizeState::Done(d) => {
            let size = if d.partial {
                trf("info.sizePartial", &[&long(d.bytes)])
            } else {
                trf("info.size", &[&long(d.bytes)])
            };
            vec![
                size,
                trf(
                    "info.contains",
                    &[
                        &crate::dockinfo::group_thousands(d.files),
                        &crate::dockinfo::group_thousands(d.dirs),
                    ],
                ),
            ]
        }
        SizeState::Running(p) => vec![trf(
            "info.sizeCalc",
            &[
                &filelist::format_size(p.bytes),
                &crate::dockinfo::group_thousands(p.files),
            ],
        )],
        SizeState::Queued => vec![tr("info.sizeQueued")],
        SizeState::Unknown => Vec::new(),
    }
}

impl App {
    /// 폴더 크기 계산을 쓰는가 — 설정 `dock.folder_size` · 성능 향상 모드면 끔.
    pub(crate) fn dirsize_enabled(&self) -> bool {
        self.settings.flag("dock.folder_size") && !self.settings.flag("perf.boost")
    }

    /// 설정 `dock.folder_size_threads` · `_queue` · `_settle_ms` → 조절값.
    pub(crate) fn apply_dirsize_tuning(&mut self) {
        let s = &self.settings;
        let t = Tuning {
            threads: s.int("dock.folder_size_threads").clamp(1, 64) as usize,
            queue_max: s.int("dock.folder_size_queue").clamp(1, 10_000) as usize,
            settle_ms: s.int("dock.folder_size_settle_ms").clamp(0, 60_000) as u64,
        };
        self.dirsizes.set_tuning(t);
    }

    /// 도크 갱신 길목: 정보 도크가 보여 줄 폴더 1개가 있으면 그 크기를 요청(빠른 이동 중이면 보류) · 없으면 보류만 버림 ·
    /// 설정이 꺼져 있으면 전부 멈춤.
    pub(crate) fn dirsize_sync(&mut self, folder: Option<&Path>) {
        if !self.dirsize_enabled() {
            self.dirsizes.clear();
            self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
            return;
        }
        match folder {
            Some(p) if !ndir_vfs::is_virtual_root(p) => self.dirsizes.request(p, Instant::now()),
            _ => self.dirsizes.idle(),
        }
    }

    /// 폴링(`OPS_POLL_MS`) — 할 일이 있으면 도크의 상태(대기 · 진행값 · 결과)를 갱신하고 `true`.
    pub(crate) fn dirsize_tick(&mut self) -> bool {
        let now = Instant::now();
        let live = self.dirsizes.tick(now);
        // 상태 전이는 즉시 · 진행값은 250 ms마다(종전 "재는 중이면 틱마다" = 그리기 → 틱 → 그리기 고리 · 10-08 프레임 계측).
        if self.dirsizes.take_display_due(now) {
            self.update_docks();
            self.redraw();
        }
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-dirsize-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 표시 조절(MC/DC · 10-08 프레임 계측): 전이 = 즉시 · 재는 중 + 첫 표시 = 즉시 · 재는 중 + 250 ms 안 = 아니다 · 재는 중 + 250 ms
    /// 지남 = 예 · 유휴 + 전이 없음 = 아니다. `take_display_due`는 참 뒤 바로 다시 부르면 거짓(보여 줬다로 기록).
    #[test]
    fn display_is_throttled_while_measuring_and_immediate_on_transition() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        assert!(display_due(true, false, Some(ms(0)), ms(1), 250));
        assert!(display_due(false, true, None, ms(1), 250));
        assert!(!display_due(false, true, Some(ms(0)), ms(249), 250));
        assert!(display_due(false, true, Some(ms(0)), ms(250), 250));
        assert!(!display_due(false, false, Some(ms(0)), ms(10_000), 250));
        let mut s = DirSizes {
            changed: true,
            ..Default::default()
        };
        assert!(s.take_display_due(ms(5)));
        assert!(!s.take_display_due(ms(6)));
        assert!(!s.take_display_due(ms(1000))); // 유휴 = 시간이 지나도 다시 그리지 않는다
    }

    /// 캐시 무효화 판정(MC/DC): 자신 · 안쪽 변경 · 바깥 조상 변경 · 무관.
    #[test]
    fn stale_rules() {
        let k = Path::new("D:/a/b");
        assert!(stale(k, Path::new("D:/a/b")));
        assert!(stale(k, Path::new("D:/a/b/c/d")));
        assert!(stale(k, Path::new("D:/a")));
        assert!(!stale(k, Path::new("D:/a/bb")));
        assert!(!stale(k, Path::new("E:/x")));
    }

    /// 빠른 이동 판정(MC/DC): settle 0 = 늘 즉시 · 첫 요청 = 즉시 · 지남 = 즉시 · 안 지남 = 보류.
    #[test]
    fn settled_rules() {
        let t0 = Instant::now();
        assert!(settled(t0, None, 250));
        assert!(settled(t0 + Duration::from_millis(10), Some(t0), 0));
        assert!(settled(t0 + Duration::from_millis(250), Some(t0), 250));
        assert!(!settled(t0 + Duration::from_millis(249), Some(t0), 250));
    }

    /// 훑기: 바이트 · 파일 · 폴더 합계 · 빈 폴더 · 없는 폴더 = 오류 1 · 취소 = 즉시 그침.
    #[test]
    fn walk_counts_and_cancels() {
        let d = fixture("walk");
        std::fs::create_dir_all(d.join("x").join("y")).unwrap();
        std::fs::create_dir_all(d.join("empty")).unwrap();
        std::fs::write(d.join("a.txt"), vec![1u8; 100]).unwrap();
        std::fs::write(d.join("x").join("b.txt"), vec![1u8; 250]).unwrap();
        std::fs::write(d.join("x").join("y").join("c.txt"), vec![1u8; 7]).unwrap();
        let sh = DirSizeShared::default();
        walk(&d, &sh);
        assert_eq!(
            sh.snapshot(),
            DirSize {
                bytes: 357,
                files: 3,
                dirs: 3,
                partial: false
            }
        );
        let sh = DirSizeShared::default();
        walk(&d.join("nope"), &sh);
        assert!(sh.snapshot().partial);
        let sh = DirSizeShared::default();
        sh.cancel.store(true, Ordering::Relaxed);
        walk(&d, &sh);
        assert_eq!(sh.snapshot().files, 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    fn drain(s: &mut DirSizes) {
        let at = Instant::now();
        while s.tick(Instant::now()) {
            assert!(at.elapsed().as_secs() < 10, "계산이 끝나지 않음");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// 작업 큐(사용자 10-06): 동시 1개면 둘째 요청은 큐(대기 상태) → 첫째가 끝나면 시작 · 같은 경로 재요청 무동작 · 지금 보는 폴더는
    /// 큐 맨 앞 · 큐 상한 = 오래된 것부터 버림 · 무효화 = 캐시 · 큐 · 진행 중 작업 모두 · clear = 전부.
    #[test]
    fn queue_runs_n_at_a_time_and_prioritizes_latest() {
        let d = fixture("queue");
        let mk = |n: &str| {
            let p = d.join(n);
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("f"), vec![0u8; 10]).unwrap();
            p
        };
        let (a, b, c) = (mk("a"), mk("b"), mk("c"));
        let mut s = DirSizes::default();
        s.set_tuning(Tuning {
            threads: 1,
            queue_max: 2,
            settle_ms: 0,
        });
        let now = Instant::now();
        s.request(&a, now);
        s.request(&b, now);
        s.request(&c, now);
        assert_eq!(s.running_len(), 1, "동시 1개");
        assert_eq!(s.queue_len(), 2);
        assert_eq!(s.get(&c), SizeState::Queued, "마지막 요청 = 대기");
        assert!(matches!(
            s.get(&a),
            SizeState::Running(_) | SizeState::Done(_)
        ));
        // 지금 보는 폴더(b)를 다시 요청하면 큐 맨 앞으로.
        s.request(&b, now);
        assert_eq!(s.queue_len(), 2);
        // 상한 2: 하나 더 넣으면 가장 오래된 것(뒤)이 떨어진다.
        let e = mk("e");
        s.request(&e, now);
        assert_eq!(s.queue_len(), 2);
        assert_eq!(s.get(&c), SizeState::Unknown, "뒤(오래된 것)부터 버림");
        drain(&mut s);
        assert_eq!(s.get(&a).map_done(), Some(10));
        assert_eq!(s.get(&b).map_done(), Some(10));
        assert_eq!(s.get(&e).map_done(), Some(10));
        s.request(&a, now);
        assert!(!s.running(), "캐시 = 다시 재지 않음");
        // 무효화: 캐시 제거.
        s.invalidate(&[a.join("sub")]);
        assert_eq!(s.get(&a), SizeState::Unknown);
        s.request(&a, now);
        s.clear();
        assert!(!s.running());
        let _ = std::fs::remove_dir_all(&d);
    }

    impl SizeState {
        fn map_done(self) -> Option<u64> {
            match self {
                SizeState::Done(d) => Some(d.bytes),
                _ => None,
            }
        }
    }

    /// 빠른 이동 건너뛰기: 250 ms 안에 잇단 요청은 큐에 안 들어가고 마지막 것만 보류 → 틱에서 멈춤 기준을 넘으면 큐로 ·
    /// 보고 있는 폴더가 없어지면(idle) 보류는 버린다.
    #[test]
    fn rapid_requests_keep_only_the_last_one() {
        let d = fixture("rapid");
        let mut s = DirSizes::default();
        s.set_tuning(Tuning {
            threads: 2,
            queue_max: 50,
            settle_ms: 250,
        });
        let t0 = Instant::now();
        s.request(&d.join("p1"), t0);
        assert_eq!(s.running_len() + s.queue_len(), 1, "첫 요청은 즉시");
        s.request(&d.join("p2"), t0 + Duration::from_millis(50));
        s.request(&d.join("p3"), t0 + Duration::from_millis(100));
        assert_eq!(
            s.running_len() + s.queue_len(),
            1,
            "빠른 이동 = 큐에 안 넣는다"
        );
        assert_eq!(
            s.get(&d.join("p2")),
            SizeState::Unknown,
            "지나친 것은 일이 안 생긴다"
        );
        assert_eq!(s.get(&d.join("p3")), SizeState::Queued, "마지막 것만 보류");
        assert!(
            !s.tick(t0 + Duration::from_millis(200)) || s.get(&d.join("p3")) == SizeState::Queued
        );
        s.tick(t0 + Duration::from_millis(400));
        assert!(
            matches!(
                s.get(&d.join("p3")),
                SizeState::Running(_) | SizeState::Done(_)
            ),
            "멈추면 큐로"
        );
        s.request(&d.join("p4"), t0 + Duration::from_millis(700));
        assert_ne!(
            s.get(&d.join("p4")),
            SizeState::Unknown,
            "멈춘 뒤 요청 = 즉시"
        );
        s.request(&d.join("p5"), t0 + Duration::from_millis(720));
        assert_eq!(
            s.get(&d.join("p5")),
            SizeState::Queued,
            "다시 빠른 이동 = 보류"
        );
        s.idle();
        assert_eq!(
            s.get(&d.join("p5")),
            SizeState::Unknown,
            "보고 있는 폴더가 없으면 보류를 버린다"
        );
        drain(&mut s);
        let _ = std::fs::remove_dir_all(&d);
    }
}
