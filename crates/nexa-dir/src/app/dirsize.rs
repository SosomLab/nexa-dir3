//! App — 폴더 크기 계산(T-166 · NEW-037 · dir3 신규 — dir2와 탐색기 목록에는 없고 탐색기 속성 창에만 있다 · 사용자 10-05 ·
//! **10-06 작업 큐 개편** · **11-09 우선순위·유효기간 정비**).
//!
//! 정보 도크에 **폴더 1개**가 보이면 그 트리를 훑어 바이트 · 파일 수 · 폴더 수를 재고, 재는 동안은 진행값을 보여 준다.
//! - **작업 큐 + 동시 N개**(사용자 10-06 "100개 폴더를 차례로 클릭하면 프로세스·디스크를 과하게 쓴다"): 요청은 큐에 들어가고
//!   작업 스레드는 설정 `dock.folder_size_threads`(기본 2)개까지만 — 하나가 끝나면 다음 하나를 꺼낸다. 지금 보고 있는 폴더는 큐
//!   **맨 앞**으로(다음 빈 자리를 먼저 받는다) · 큐 상한(`dock.folder_size_queue`)을 넘으면 가장 오래된 것부터 버린다 ·
//!   스레드 우선순위는 낮춤.
//! - **빠른 이동 건너뛰기**(`dock.folder_size_settle_ms` · 기본 250): 직전 요청에서 이 시간이 안 지났으면(키보드로 빠르게
//!   지나가는 중 · 빠른 스크롤) 큐에 넣지 않고 **마지막 것만 보류**해 두었다가 멈추면 넣는다 — 지나친 폴더의 일은 아예
//!   생기지 않는다.
//! - **보이는 것 우선**(사용자 11-09 "A의 하위를 재는 중에 형제 B로 가면 B1·B2가 늦어진다"): 크기 열 요청([`Origin::Column`])은
//!   **지금 화면에 보이는 폴더 행**([`DirSizes::set_visible`])에만 뜻이 있다 — 탭 이동·스크롤로 안 보이게 된 요청은 큐에서
//!   빼고(다시 보이면 다시 요청된다), 보이는 요청이 기다리는데 슬롯이 안 보이는 열 작업으로 차 있으면 그것을 **선점 취소**한다.
//!   취소된 작업의 진행값은 "≥ n"(일부)로 캐시에 남겨 돌아왔을 때 바로 보이고 뒤에서 마저 잰다. 도크 요청([`Origin::Dock`])과
//!   수동 명령([`Origin::Manual`])은 보임 여부와 무관하게 끝까지 간다.
//! - **유효기간**(`dock.folder_size_ttl_s` · 기본 30분 · 0 = 없음): 지난 값은 **보여 주면서 뒤에서 다시 잰다**(빈칸으로 깜빡이지
//!   않는다). 만료는 외부 프로그램이 감시 밖 깊은 하위를 바꾼 경우의 안전망일 뿐이고, 아래 무효화가 주 경로다.
//! - **무효화 규칙**(11-09 통일): ① 바뀐 경로에 닿는 항목(자신 · 안쪽 · **상위 폴더 전부**)은 즉시 버린다([`stale`] ·
//!   [`DirSizes::invalidate`]) — 폴더 감시 통지 · 앱 자신의 작업(전송 · 휴지통/완전 삭제 · 이름 변경 · 새로 만들기)이 영향 경로를
//!   넘긴다. ② **mtime 가드**: 요청 때 본 폴더 행의 수정 시각을 항목에 적어 두고, 목록 행의 수정 시각이 달라져 있으면(직접 자식
//!   추가 · 삭제 · 이름 변경) 감시 밖이어도 즉시 버린다(stat 비용 0 — 목록이 이미 읽은 값). ③ 영향 범위를 모르는 작업(실행
//!   취소/다시 실행 · 일괄 이름 변경 · 압축/풀기 · 비교/중복 · 설정 변경)은 전부 버린다([`DirSizes::clear`]). ④ 새로 고침
//!   (`view.refresh`)은 보는 폴더의 트리 전체를 버린다(사용자의 강제 재계산).
//! - 상태 = 측정 대기 중(큐) → 계산 중(진행값) → 끝(캐시). 설정 `dock.folder_size`로 끌 수 있고 성능 향상 모드에서는 끈다.
//!   링크(심볼릭 링크 · 정션)는 따라가지 않는다(대상이 다른 곳에 있다 — 전송 엔진과 같은 규칙).

use crate::*;
use std::collections::{HashMap, HashSet, VecDeque};
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
    /// 훑기가 `cancel`을 보고 중간에 그쳤다(진행값은 일부) — 끝까지 돈 뒤에 취소된 것과 구분(11-09).
    aborted: AtomicBool,
    done: AtomicBool,
}

/// 요청의 출처 — 보임 여부로 버리거나 선점할 수 있는가를 가른다(11-09).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    /// 정보 도크가 보여 줄 폴더(큐 맨 앞 · 끝까지 간다).
    Dock,
    /// 크기 열의 보이는 폴더 행(큐 뒤 · 안 보이면 버리거나 선점당한다).
    Column,
    /// 수동 "폴더 크기 계산"(큐 뒤 · 끝까지 간다 — 토스트가 기다린다).
    Manual,
}

/// 큐 항목.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Req {
    path: PathBuf,
    origin: Origin,
    /// 요청 때 본 폴더 행의 수정 시각(mtime 가드용 · 도크/수동 = 모름).
    mtime_ms: Option<i64>,
}

/// 진행 중인 계산 1건.
#[derive(Debug)]
pub(crate) struct DirSizeJob {
    pub(crate) path: PathBuf,
    shared: Arc<DirSizeShared>,
    /// 시작 시각 — 오래 도는 작업(큰 트리)이 슬롯을 막고 있으면 새 요청이 밀어낸다(10-18).
    started: Instant,
    origin: Origin,
    mtime_ms: Option<i64>,
}

/// 끝난 계산.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirSize {
    pub(crate) bytes: u64,
    pub(crate) files: u64,
    pub(crate) dirs: u64,
    /// 읽지 못한 폴더가 있었다 · 또는 중간에 그쳤다(선점 취소) — 표시는 "≥".
    pub(crate) partial: bool,
}

/// 캐시 항목(11-09): 값 + 계산 시각(유효기간) + 끝까지 쟀는가 + 요청 때의 폴더 수정 시각(mtime 가드).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    size: DirSize,
    at: Instant,
    /// 끝까지 쟀다(거짓 = 선점 취소된 진행값 — 보이면 다시 잰다).
    complete: bool,
    mtime_ms: Option<i64>,
}

/// 경로 하나의 상태.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SizeState {
    /// 끝남(캐시 — 만료됐거나 일부여도 값은 돌려준다 · 다시 재야 하는가는 [`DirSizes::wants_refresh`]).
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
    /// 캐시 유효기간(`None` = 만료 없음).
    pub(crate) ttl: Option<Duration>,
}

/// 기본 유효기간(초 · 사용자 11-09 "mtime 가드가 있으면 30분") — 설정 `dock.folder_size_ttl_s`의 기본값과 같다.
pub(crate) const DEFAULT_TTL_S: u64 = 1800;

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            threads: 2,
            queue_max: 50,
            settle_ms: 250,
            ttl: Some(Duration::from_secs(DEFAULT_TTL_S)),
        }
    }
}

/// 폴더 크기 캐시 상한(T-189 ⑦ · 항목 수) — 아이콘 캐시(512 · 36d7057)처럼 상수.
pub(crate) const CACHE_MAX: usize = 1024;

/// 폴더 크기 상태(App 필드 하나).
#[derive(Debug, Default)]
pub(crate) struct DirSizes {
    /// 재는 중(≤ `threads`).
    running: Vec<DirSizeJob>,
    /// 선점 취소돼 슬롯을 내준 작업 — 그치면 진행값을 "일부"로 캐시에 남긴다(11-09).
    retired: Vec<DirSizeJob>,
    /// 차례를 기다리는 요청(앞 = 다음 차례).
    queue: VecDeque<Req>,
    cache: HashMap<PathBuf, Entry>,
    /// 캐시에 넣은 순서(T-189 ⑦ · [`CACHE_MAX`]를 넘으면 오래된 것부터 버린다).
    order: VecDeque<PathBuf>,
    /// 빠른 이동 중 보류한 마지막 요청(경로 · 시각) — settle이 지나면 큐에 넣는다.
    pending: Option<(PathBuf, Instant)>,
    /// 마지막 요청 시각(빠른 이동 판정).
    last_request: Option<Instant>,
    tuning: Tuning,
    /// 지금 화면에 보이는 폴더 행(두 패널 · 크기 열이 켜졌을 때) — 열 요청의 생존 조건(11-09).
    visible: HashSet<PathBuf>,
    /// 표시 조절(10-08 프레임 계측 적발): 상태 전이(끝남 · 시작 · 큐 변화)가 있었다 → 다음 표시는 즉시.
    changed: bool,
    /// 마지막으로 도크에 반영한 시각 — 재는 중의 진행값("지금까지 …")은 [`DISPLAY_EVERY_MS`]마다만. 종전에는 유휴 틱(사건마다 =
    /// 그리기 뒤에도)마다 다시 그려 **그리기 → 틱 → 그리기** 고리로 재는 내내 120~170 fps가 됐다(docs/25 §8-1).
    last_shown: Option<Instant>,
}

/// 재는 중 진행값 표시 간격(ms).
pub(crate) const DISPLAY_EVERY_MS: u64 = 250;
/// 이보다 오래 도는 작업이 슬롯을 전부 막고 있을 때 새 요청(지금 보는 폴더)이 가장 오래된 것을 취소한다(ms · 10-18 체감 점검 —
/// 종전엔 `C:\Windows` 같은 큰 트리 2개가 끝날 때까지 새로 고른 폴더가 "측정 대기 중"에 머물렀다). 짧은 작업은 그대로 끝낸다.
const LONG_RUN_MS: u64 = 2_000;

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

/// 캐시 항목 `key`가 `changed` 폴더의 변경에 영향을 받는가(순수): 바뀐 폴더가 항목 자신이거나 그 안(조상 항목의 합계가 변함 —
/// 상위 폴더 전부가 여기에 걸린다) · 또는 항목이 바뀐 폴더 안(항목 자체가 지워지거나 바뀌었을 수 있다).
pub(crate) fn stale(key: &Path, changed: &Path) -> bool {
    changed.starts_with(key) || key.starts_with(changed)
}

/// 유효기간이 지났는가(순수): `ttl` 없음 = 영원히 유효.
pub(crate) fn expired(at: Instant, now: Instant, ttl: Option<Duration>) -> bool {
    ttl.is_some_and(|ttl| now.saturating_duration_since(at) >= ttl)
}

/// 빠른 이동 판정(순수 · MC/DC): settle 0 = 늘 즉시 · 첫 요청 = 즉시 · 직전 요청에서 settle 이상 지났으면 즉시 · 아니면 보류.
pub(crate) fn settled(now: Instant, last: Option<Instant>, settle_ms: u64) -> bool {
    settle_ms == 0 || last.is_none_or(|l| now.duration_since(l).as_millis() as u64 >= settle_ms)
}

/// 트리를 훑는다(작업 스레드 · 반복문 — 깊은 트리에서 스택을 쓰지 않는다). 링크는 세지도 따라가지도 않는다 ·
/// 열거 실패는 `errors`에 세고 계속 · `cancel`이면 바로 그친다(`aborted` 표시).
pub(crate) fn walk(root: &Path, sh: &DirSizeShared) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if sh.cancel.load(Ordering::Relaxed) {
            sh.aborted.store(true, Ordering::Relaxed);
            return;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            sh.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        for e in rd {
            if sh.cancel.load(Ordering::Relaxed) {
                sh.aborted.store(true, Ordering::Relaxed);
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

    fn has_progress(&self) -> bool {
        self.bytes.load(Ordering::Relaxed) > 0
            || self.files.load(Ordering::Relaxed) > 0
            || self.dirs.load(Ordering::Relaxed) > 0
    }
}

impl DirSizeJob {
    fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Relaxed);
    }

    fn is_done(&self) -> bool {
        self.shared.done.load(Ordering::Relaxed)
    }
}

impl DirSizes {
    /// 조절값(설정 변경 때 · 재는 중인 작업은 그대로 끝까지).
    pub(crate) fn set_tuning(&mut self, t: Tuning) {
        self.tuning = Tuning {
            threads: t.threads.max(1),
            queue_max: t.queue_max.max(1),
            settle_ms: t.settle_ms,
            ttl: t.ttl,
        };
    }

    /// 캐시 값이 지금도 유효한가 — 끝까지 쟀고 유효기간 안.
    fn fresh(&self, path: &Path, now: Instant) -> bool {
        self.cache
            .get(path)
            .is_some_and(|e| e.complete && !expired(e.at, now, self.tuning.ttl))
    }

    fn is_running(&self, path: &Path) -> bool {
        self.running.iter().any(|j| j.path == path)
    }

    fn is_queued(&self, path: &Path) -> bool {
        self.queue.iter().any(|r| r.path == path)
            || self.pending.as_ref().is_some_and(|(p, _)| p == path)
    }

    /// `path`의 크기를 요청한다(정보 도크) — 유효한 캐시가 있거나 재는 중이면 무동작 · 빠른 이동 중이면 보류(마지막 것만) ·
    /// 아니면 큐 맨 앞에.
    pub(crate) fn request(&mut self, path: &Path, now: Instant) {
        if self.fresh(path, now) || self.is_running(path) {
            return;
        }
        let ok = settled(now, self.last_request, self.tuning.settle_ms);
        self.last_request = Some(now);
        if ok {
            self.pending = None;
            self.enqueue_front(Req {
                path: path.to_path_buf(),
                origin: Origin::Dock,
                mtime_ms: None,
            });
            // 슬롯이 전부 차 있으면 자리를 비운다: 먼저 안 보이는 열 작업(11-09 · 보는 폴더가 우선) · 없으면 오래 도는 작업
            // (10-18 체감 점검 — 종전엔 `C:\\Windows` 같은 큰 트리 2개가 끝날 때까지 새로 고른 폴더가 "측정 대기 중"에 머물렀다).
            // 밀려난 작업의 진행값은 "일부"로 남는다(다시 보이면 마저 잰다).
            if self.running.len() >= self.tuning.threads && !self.preempt_hidden(1) {
                if let Some(i) = self
                    .running
                    .iter()
                    .enumerate()
                    .filter(|(_, j)| {
                        now.saturating_duration_since(j.started).as_millis() as u64 >= LONG_RUN_MS
                    })
                    .min_by_key(|(_, j)| j.started)
                    .map(|(i, _)| i)
                {
                    self.retire(i);
                }
            }
            self.fill();
        } else {
            self.pending = Some((path.to_path_buf(), now));
        }
    }

    /// 뒤쪽 큐 요청(크기 열 [`Origin::Column`] · 수동 명령 [`Origin::Manual`] · T-166 3차 · 10-25): 도크의 폴더가 앞 · 멈춤 판정 없음 ·
    /// 유효한 캐시가 있거나 재는 중·대기 중인 경로는 무시 · 큐가 상한이면 넣지 않는다(보이는 폴더가 많아도 큐를 밀어내지 않는다 —
    /// 스크롤하면 다시 요청된다). `mtime_ms` = 요청 때 본 폴더 행의 수정 시각(mtime 가드 · 11-09).
    pub(crate) fn request_back(
        &mut self,
        path: &Path,
        now: Instant,
        origin: Origin,
        mtime_ms: Option<i64>,
    ) {
        if self.fresh(path, now)
            || self.is_running(path)
            || self.is_queued(path)
            || self.queue.len() >= self.tuning.queue_max
        {
            return;
        }
        self.queue.push_back(Req {
            path: path.to_path_buf(),
            origin,
            mtime_ms,
        });
        // 슬롯이 다 찼으면 안 보이는 열 작업 하나를 밀어낸다 — 열 요청은 보이는 행에서만 오고(지금 필요한 값), 수동 명령은 사용자가
        // 지금 시킨 일이다(11-09). 보이는 열 작업 · 도크 · 수동 작업은 밀어내지 않는다.
        if self.running.len() >= self.tuning.threads {
            self.preempt_hidden(1);
        }
        self.fill();
    }

    /// 지금 화면에 보이는 폴더 행(두 패널 합 · 11-09): 안 보이게 된 열 요청은 큐에서 빼고, 보이는 요청이 기다리는데 슬롯이
    /// 안 보이는 열 작업으로 차 있으면 선점 취소한다. 같은 집합이면 무동작(그리기마다 불린다).
    pub(crate) fn set_visible(&mut self, visible: HashSet<PathBuf>) {
        if visible == self.visible {
            return;
        }
        self.visible = visible;
        let before = self.queue.len();
        let vis = &self.visible;
        self.queue
            .retain(|r| r.origin != Origin::Column || vis.contains(&r.path));
        if self.queue.len() != before {
            self.changed = true;
        }
        let waiting = self.queue.len();
        if waiting > 0 && self.running.len() >= self.tuning.threads {
            self.preempt_hidden(waiting);
        }
        self.fill();
    }

    /// 안 보이는 열 작업을 오래된 것부터 최대 `n`개 선점 취소한다(슬롯을 바로 내준다) — 하나라도 취소했으면 참.
    fn preempt_hidden(&mut self, n: usize) -> bool {
        let mut idx: Vec<usize> = self
            .running
            .iter()
            .enumerate()
            .filter(|(_, j)| j.origin == Origin::Column && !self.visible.contains(&j.path))
            .map(|(i, _)| i)
            .collect();
        if idx.is_empty() {
            return false;
        }
        idx.sort_by_key(|&i| self.running[i].started);
        idx.truncate(n);
        // 뒤 인덱스부터 빼야 앞 인덱스가 밀리지 않는다.
        idx.sort_unstable_by(|a, b| b.cmp(a));
        for i in idx {
            self.retire(i);
        }
        true
    }

    /// 재는 중 `i`번째를 취소하고 슬롯에서 빼 둔다(결과 수거는 `tick`).
    fn retire(&mut self, i: usize) {
        let j = self.running.remove(i);
        j.cancel();
        self.retired.push(j);
        self.changed = true;
    }

    /// 큐 맨 앞에(이미 있으면 앞으로 옮김) · 상한을 넘으면 뒤(가장 오래된 것)부터 버림.
    fn enqueue_front(&mut self, req: Req) {
        self.queue.retain(|r| r.path != req.path);
        self.queue.push_front(req);
        while self.queue.len() > self.tuning.queue_max {
            self.queue.pop_back();
        }
    }

    /// 빈 자리만큼 큐에서 꺼내 작업 스레드를 띄운다.
    fn fill(&mut self) {
        while self.running.len() < self.tuning.threads {
            let Some(req) = self.queue.pop_front() else {
                break;
            };
            let shared = Arc::new(DirSizeShared::default());
            let sh = Arc::clone(&shared);
            let root = req.path.clone();
            let spawned = std::thread::Builder::new()
                .name("ndir-dirsize".into())
                .spawn(move || {
                    platform::lower_thread_priority(); // UI · 다른 작업에 양보
                    walk(&root, &sh);
                    sh.done.store(true, Ordering::Relaxed);
                });
            if spawned.is_err() {
                // 스레드를 못 띄웠다(자원 고갈) — 읽지 못한 것으로 끝낸다(다음 요청이 다시 시도).
                shared.errors.fetch_add(1, Ordering::Relaxed);
                shared.done.store(true, Ordering::Relaxed);
            }
            self.running.push(DirSizeJob {
                path: req.path,
                shared,
                started: Instant::now(),
                origin: req.origin,
                mtime_ms: req.mtime_ms,
            });
        }
    }

    /// 보고 있는 폴더가 없다 — 보류만 버린다(큐와 진행 중인 작업은 계속 간다 · 결과는 캐시로).
    pub(crate) fn idle(&mut self) {
        self.pending = None;
    }

    /// 폴링: 끝난 작업을 캐시로 옮기고 · 선점당한 작업의 진행값을 "일부"로 남기고 · 보류가 멈춤 기준을 넘었으면 큐에 넣고 ·
    /// 빈 자리를 채운다. 돌려주는 값 = 아직 할 일이 있다(깨움이 필요하다).
    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        let before = (
            self.running.len(),
            self.retired.len(),
            self.queue.len(),
            self.cache.len(),
        );
        let (done, still): (Vec<DirSizeJob>, Vec<DirSizeJob>) =
            self.running.drain(..).partition(DirSizeJob::is_done);
        self.running = still;
        for j in done {
            let sh = &j.shared;
            // 끝까지 돈 결과(취소 표시가 있어도 중간에 그치지 않았으면 온전하다).
            let complete = !sh.aborted.load(Ordering::Relaxed);
            self.remember(j.path, sh.snapshot(), now, complete, j.mtime_ms);
        }
        let (done, still): (Vec<DirSizeJob>, Vec<DirSizeJob>) =
            self.retired.drain(..).partition(DirSizeJob::is_done);
        self.retired = still;
        for j in done {
            let sh = &j.shared;
            if !sh.aborted.load(Ordering::Relaxed) {
                self.remember(j.path, sh.snapshot(), now, true, j.mtime_ms);
            } else if sh.has_progress() && !self.cache.contains_key(&j.path) {
                let mut d = sh.snapshot();
                d.partial = true;
                self.remember(j.path, d, now, false, j.mtime_ms);
            }
        }
        if let Some((p, at)) = &self.pending {
            if now.duration_since(*at).as_millis() as u64 >= self.tuning.settle_ms {
                let p = p.clone();
                self.pending = None;
                if !self.fresh(&p, now) {
                    self.enqueue_front(Req {
                        path: p,
                        origin: Origin::Dock,
                        mtime_ms: None,
                    });
                }
            }
        }
        self.fill();
        if before
            != (
                self.running.len(),
                self.retired.len(),
                self.queue.len(),
                self.cache.len(),
            )
        {
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

    /// 할 일이 있는가(재는 중 · 큐 · 보류) — 시험 판정 · 기동 명령 `dirsize.wait`(호스트는 `tick`의 반환값과 `take_display_due`만 쓴다).
    pub(crate) fn running(&self) -> bool {
        self.busy()
    }

    /// 결과가 바뀌었는데 아직 도크에 보여 주지 않았는가(`dirsize.wait`가 측정 끝 뒤 표시까지 기다리게).
    pub(crate) fn display_pending(&self) -> bool {
        self.changed
    }

    fn busy(&self) -> bool {
        !self.running.is_empty()
            || !self.retired.is_empty()
            || !self.queue.is_empty()
            || self.pending.is_some()
    }

    /// `path`의 상태. 캐시 값은 만료됐거나 일부여도 `Done`으로 돌려준다(보여 주면서 뒤에서 다시 잰다 — [`Self::wants_refresh`]).
    pub(crate) fn get(&self, path: &Path) -> SizeState {
        if let Some(e) = self.cache.get(path) {
            return SizeState::Done(e.size);
        }
        if let Some(j) = self.running.iter().find(|j| j.path == path) {
            return SizeState::Running(j.shared.snapshot());
        }
        if self.is_queued(path) {
            return SizeState::Queued;
        }
        SizeState::Unknown
    }

    /// 캐시 값이 있지만 다시 재야 하는가(11-09): 일부(선점 취소)거나 유효기간이 지났고 · 아직 재거나 기다리고 있지 않다.
    pub(crate) fn wants_refresh(&self, path: &Path, now: Instant) -> bool {
        self.cache.contains_key(path)
            && !self.fresh(path, now)
            && !self.is_running(path)
            && !self.is_queued(path)
    }

    /// mtime 가드(11-09): 캐시 항목이 요청 때 본 폴더 수정 시각을 알고 있고 지금 목록 행의 값과 다르다 — 직접 자식이 바뀌었다
    /// (감시 밖이어도 안다 · 호출부는 [`Self::invalidate`]로 자신 + 상위를 버린다).
    pub(crate) fn mtime_mismatch(&self, path: &Path, row_mtime_ms: i64) -> bool {
        self.cache
            .get(path)
            .and_then(|e| e.mtime_ms)
            .is_some_and(|m| m != row_mtime_ms)
    }

    /// 캐시에 넣는다 — [`CACHE_MAX`]를 넘으면 가장 먼저 넣은 것부터 버린다(T-189 ⑦ · 종전 = 작업 전까지 상한 없음).
    fn remember(
        &mut self,
        path: PathBuf,
        size: DirSize,
        now: Instant,
        complete: bool,
        mtime_ms: Option<i64>,
    ) {
        let e = Entry {
            size,
            at: now,
            complete,
            mtime_ms,
        };
        if self.cache.insert(path.clone(), e).is_none() {
            self.order.push_back(path);
        }
        while self.cache.len() > CACHE_MAX {
            match self.order.pop_front() {
                Some(old) => {
                    self.cache.remove(&old);
                }
                None => break,
            }
        }
    }

    /// 바뀐 폴더들에 닿는(자신 · 안쪽 · **상위 전부** — [`stale`]) 캐시 항목 · 큐 항목 · 보류를 버리고, 닿는 작업은 취소한다
    /// (다음 동기가 보이는 것부터 다시 요청한다). 하나라도 버렸으면 참(호출부가 즉시 다시 동기 · 11-09).
    pub(crate) fn invalidate(&mut self, changed: &[PathBuf]) -> bool {
        let hit = |p: &Path| changed.iter().any(|c| stale(p, c));
        let before = (
            self.cache.len(),
            self.queue.len(),
            self.running.len(),
            self.retired.len(),
            self.pending.is_some(),
        );
        self.cache.retain(|k, _| !hit(k));
        let cache = &self.cache;
        self.order.retain(|p| cache.contains_key(p));
        self.queue.retain(|r| !hit(&r.path));
        if self.pending.as_ref().is_some_and(|(p, _)| hit(p)) {
            self.pending = None;
        }
        for jobs in [&mut self.running, &mut self.retired] {
            jobs.retain(|j| {
                if hit(&j.path) {
                    j.cancel();
                    false
                } else {
                    true
                }
            });
        }
        self.fill();
        let after = (
            self.cache.len(),
            self.queue.len(),
            self.running.len(),
            self.retired.len(),
            self.pending.is_some(),
        );
        if before != after {
            self.changed = true;
            true
        } else {
            false
        }
    }

    /// 전부 버린다(영향 범위를 모르는 작업 · 설정 끔 — 어디가 바뀌었는지 일일이 따지지 않는다).
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
        self.queue.clear();
        self.pending = None;
        for j in self.running.drain(..).chain(self.retired.drain(..)) {
            j.cancel();
        }
    }

    #[cfg(test)]
    pub(crate) fn cached(&self, path: &Path) -> Option<DirSize> {
        self.cache.get(path).map(|e| e.size)
    }

    #[cfg(test)]
    pub(crate) fn queue_len(&self) -> usize {
        self.queue.len()
    }

    #[cfg(test)]
    pub(crate) fn running_len(&self) -> usize {
        self.running.len()
    }

    #[cfg(test)]
    pub(crate) fn queued_paths(&self) -> Vec<PathBuf> {
        self.queue.iter().map(|r| r.path.clone()).collect()
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
    /// 폴더 크기 계산을 쓰는가 — 설정 `dock.folder_size`의 유효값(성능 향상 모드면 `FORCES`가 off로 · T-130).
    pub(crate) fn dirsize_enabled(&self) -> bool {
        self.settings.flag("dock.folder_size")
    }

    /// 설정 `dock.folder_size_threads` · `_queue` · `_settle_ms` · `_ttl_s` → 조절값.
    pub(crate) fn apply_dirsize_tuning(&mut self) {
        let s = &self.settings;
        let ttl_s = s.int("dock.folder_size_ttl_s").clamp(0, 7 * 86_400) as u64;
        let t = Tuning {
            threads: s.int("dock.folder_size_threads").clamp(1, 64) as usize,
            queue_max: s.int("dock.folder_size_queue").clamp(1, 10_000) as usize,
            settle_ms: s.int("dock.folder_size_settle_ms").clamp(0, 60_000) as u64,
            ttl: (ttl_s > 0).then(|| Duration::from_secs(ttl_s)),
        };
        self.dirsizes.set_tuning(t);
    }

    /// 크기 열에 폴더 크기를 보이는가 — `list.folder_size_col`(기본 off) × 폴더 크기 계산 자체(`dock.folder_size` · 성능 향상 모드면 off).
    pub(crate) fn dirsize_col_enabled(&self) -> bool {
        self.dirsize_enabled() && self.settings.flag("list.folder_size_col")
    }

    /// 크기 열 동기(T-166 3차 · 10-25 · 11-09 보이는 것 우선): 각 패널의 **보이는 폴더 행**을 큐의 생존 집합으로 알리고
    /// ([`DirSizes::set_visible`] — 안 보이는 열 요청은 버리고 필요하면 선점) · mtime 가드(행의 수정 시각이 요청 때와 다르면 자신 + 상위
    /// 무효화) · 모르는 행은 뒤쪽 큐로 요청 · 만료/일부 값은 보여 주면서 다시 요청 · 캐시된 값의 지도를 목록 원천에 넘긴다(바뀐 패널의
    /// 목록 사각형 = `inv`). 꺼져 있으면 지도를 비운다(열은 빈칸). 그리기 직전(스크롤로 드러난 행)과 도크 갱신 길목(결과 도착 · 이동)에서
    /// 부른다 — 파일 시스템은 건드리지 않는다(측정은 작업 스레드).
    pub(crate) fn dirsize_col_sync(&mut self, inv: &mut Invalidations) {
        let on = self.dirsize_col_enabled();
        let now = Instant::now();
        let rows: [Vec<(PathBuf, i64)>; 2] = std::array::from_fn(|i| {
            if on && (i == 0 || self.dual) {
                self.panels[i].visible_dir_rows()
            } else {
                Vec::new()
            }
        });
        self.dirsizes.set_visible(
            rows.iter()
                .flatten()
                .map(|(p, _)| p.clone())
                .collect::<HashSet<PathBuf>>(),
        );
        for (i, rows) in rows.iter().enumerate() {
            let mut map = std::collections::HashMap::new();
            for (p, mtime) in rows {
                if self.dirsizes.mtime_mismatch(p, *mtime) {
                    self.dirsizes.invalidate(std::slice::from_ref(p));
                }
                match self.dirsizes.get(p) {
                    SizeState::Done(d) => {
                        map.insert(p.clone(), (d.bytes, d.partial));
                        if self.dirsizes.wants_refresh(p, now) {
                            self.dirsizes
                                .request_back(p, now, Origin::Column, Some(*mtime));
                        }
                    }
                    SizeState::Unknown => {
                        self.dirsizes
                            .request_back(p, now, Origin::Column, Some(*mtime))
                    }
                    SizeState::Running(_) | SizeState::Queued => {}
                }
            }
            self.panels[i].set_dir_sizes(map, inv);
        }
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

    /// 새로 고침(`view.refresh` · 11-09 규칙 ④): 각 패널이 보는 폴더의 **트리 전체**(안쪽 + 상위)를 버린다 — 사용자의 강제 재계산.
    /// 보이는 행은 다음 동기가 바로 다시 요청한다.
    pub(crate) fn dirsize_refresh_view(&mut self) {
        let roots: Vec<PathBuf> = self
            .panels
            .iter()
            .map(|p| p.root_path())
            .filter(|p| !ndir_vfs::is_virtual_root(p))
            .collect();
        if !roots.is_empty() {
            self.dirsizes.invalidate(&roots);
        }
    }

    /// 앱 자신의 작업이 바꾼 경로들(11-09 규칙 ①): 자신 · 안쪽 · 상위 폴더의 캐시를 버린다. 빈 목록이면 무동작.
    pub(crate) fn dirsize_touched(&mut self, paths: &[PathBuf]) {
        if !paths.is_empty() {
            self.dirsizes.invalidate(paths);
        }
    }

    /// 수동 "폴더 크기 계산"(T-166 · 우클릭 `ctx.folder_size` · 10-25): 고른 폴더들을 큐 뒤쪽으로 요청하고 전부 끝나면 토스트 1장
    /// (폴더별 크기 최대 5줄 + 합계) · 이미 캐시된 것은 바로 셈 · 크기 열 설정이 켜져 있으면 열에도 나타난다.
    pub(crate) fn dirsize_manual_request(&mut self, dirs: Vec<PathBuf>) {
        if dirs.is_empty() || !self.dirsize_enabled() {
            return;
        }
        let now = Instant::now();
        for d in &dirs {
            if ndir_vfs::is_virtual_root(d) {
                continue;
            }
            self.dirsizes.request_back(d, now, Origin::Manual, None);
        }
        self.dirsize_manual = dirs;
        let mut inv = Invalidations::default();
        self.statusbar
            .set_left(&tr("dirsize.manualStarted"), &mut inv);
        self.dirsize_manual_check();
        self.redraw();
    }

    /// 수동 요청이 전부 끝났으면 토스트 — 끝났으면 `true`.
    fn dirsize_manual_check(&mut self) -> bool {
        if self.dirsize_manual.is_empty() {
            return false;
        }
        let states: Vec<(PathBuf, Option<DirSize>)> = self
            .dirsize_manual
            .iter()
            .map(|p| {
                let done = match self.dirsizes.get(p) {
                    SizeState::Done(d) => Some(d),
                    _ => None,
                };
                (p.clone(), done)
            })
            .collect();
        if states.iter().any(|(_, s)| s.is_none()) {
            return false;
        }
        self.dirsize_manual.clear();
        let mut lines: Vec<String> = Vec::new();
        let mut total = 0u64;
        for (p, s) in &states {
            let s = s.as_ref().expect("checked");
            total += s.bytes;
            if lines.len() < 5 {
                lines.push(format!(
                    "{} — {}{}",
                    ndir_ops::leaf_name(p),
                    filelist::format_size(s.bytes),
                    if s.partial { " \u{2265}" } else { "" }
                ));
            }
        }
        if states.len() > 5 {
            lines.push(trf("del.listMore", &[&(states.len() - 5).to_string()]));
        }
        let title = trf(
            "dirsize.manualDone",
            &[&states.len().to_string(), &filelist::format_size(total)],
        );
        let a = self.active;
        self.toast_panel(a, toast::ToastKind::Info, title, lines.join(" · "));
        true
    }

    /// 폴링(`OPS_POLL_MS`) — 할 일이 있으면 도크의 상태(대기 · 진행값 · 결과)를 갱신하고 `true`.
    pub(crate) fn dirsize_tick(&mut self) -> bool {
        let now = Instant::now();
        let live = self.dirsizes.tick(now);
        if self.dirsize_manual_check() {
            self.redraw();
        }
        // 상태 전이는 즉시 · 진행값은 250 ms마다(종전 "재는 중이면 틱마다" = 그리기 → 틱 → 그리기 고리 · 10-08 프레임 계측).
        if self.dirsizes.take_display_due(now) {
            self.update_docks_filtered(true); // 정보 도크만(미리보기 도크는 건드리지 않는다 · 10-18)
            self.redraw();
        }
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-dirsize-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const V: DirSize = DirSize {
        bytes: 1,
        files: 1,
        dirs: 0,
        partial: false,
    };

    fn col(d: &mut DirSizes, p: &Path, now: Instant) {
        d.request_back(p, now, Origin::Column, Some(1));
    }

    /// `request_back`(T-166 3차 · 11-09): 뒤쪽에 · 중복 없음 · 캐시/진행 중 무시 · 큐가 상한이면 넣지 않음 · 도크 요청은 앞이고
    /// 슬롯이 안 보이는 열 작업으로 차 있으면 그것을 선점해 바로 시작한다.
    #[test]
    fn request_back_appends_without_evicting() {
        let mut d = DirSizes::default();
        d.set_tuning(Tuning {
            threads: 1,
            queue_max: 3,
            settle_ms: 0,
            ttl: None,
        });
        let now = Instant::now();
        let p = |n: usize| PathBuf::from(format!("/q/{n}"));
        d.set_visible((0..6).map(p).collect()); // 전부 보이는 행(선점 대상 아님)
        col(&mut d, &p(0), now); // 슬롯 1개를 차지(틱 전까지 진행 중) → 이후 요청은 큐에만
        assert_eq!(d.running.len(), 1);
        col(&mut d, &p(1), now);
        col(&mut d, &p(2), now);
        col(&mut d, &p(1), now);
        assert_eq!(d.queued_paths(), vec![p(1), p(2)], "뒤에 · 중복 없음");
        d.request(&p(9), now); // 도크 요청 = 앞(보이는 열 작업 p(0)은 밀어내지 않는다)
        assert_eq!(d.queued_paths(), vec![p(9), p(1), p(2)]);
        assert_eq!(d.running[0].path, p(0));
        col(&mut d, &p(3), now); // 상한 3 = 넣지 않음
        assert_eq!(d.queue.len(), 3);
        assert!(!d.queued_paths().contains(&p(3)));
        assert_eq!(d.get(&p(2)), SizeState::Queued);
        col(&mut d, &p(0), now); // 진행 중 = 무시
        assert_eq!(d.queue.len(), 3);
        d.remember(p(5), V, now, true, None);
        col(&mut d, &p(5), now); // 캐시 = 무시
        assert!(!d.queued_paths().contains(&p(5)));
        // 안 보이는 열 작업이 슬롯을 쥐고 있으면 도크 요청이 밀어내고 바로 시작한다(11-09).
        d.clear();
        d.set_visible(HashSet::new());
        col(&mut d, &p(0), now);
        assert_eq!(d.running[0].path, p(0));
        d.request(&p(9), now);
        assert_eq!(d.running[0].path, p(9), "도크 요청이 슬롯을 받는다");
        assert_eq!(d.retired.len(), 1, "밀려난 작업은 결과 수거 대기");
        d.clear();
    }

    #[test]
    fn cache_is_bounded_oldest_first() {
        let mut d = DirSizes::default();
        let now = Instant::now();
        let p = |n: usize| PathBuf::from(format!("/x/{n}"));
        for n in 0..CACHE_MAX + 3 {
            d.remember(p(n), V, now, true, None);
        }
        assert_eq!(d.cache.len(), CACHE_MAX);
        assert!(!d.cache.contains_key(&p(0)) && !d.cache.contains_key(&p(2)));
        assert!(d.cache.contains_key(&p(3)) && d.cache.contains_key(&p(CACHE_MAX + 2)));
        d.remember(p(5), V, now, true, None); // 다시 넣기 = 순서 항목 늘지 않음
        assert_eq!(d.order.len(), CACHE_MAX);
        assert!(d.invalidate(&[p(5)]));
        assert!(!d.cache.contains_key(&p(5)));
        assert_eq!(d.order.len(), d.cache.len());
        assert!(
            !d.invalidate(&[PathBuf::from("/nowhere")]),
            "닿는 것 없음 = 거짓"
        );
        d.clear();
        assert!(d.order.is_empty() && d.cache.is_empty());
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

    /// 유효기간(11-09 · MC/DC): ttl 없음 = 영원 · 안 지남 · 딱 지남 · 지남. 만료된 값은 `get`이 여전히 `Done`(보여 준다)이고
    /// `wants_refresh`가 참 → 다시 요청되면(재는 중·대기) 거짓.
    #[test]
    fn ttl_expired_value_is_kept_and_revalidated() {
        let t0 = Instant::now();
        let ttl = Some(Duration::from_secs(60));
        assert!(!expired(t0, t0 + Duration::from_secs(10_000), None));
        assert!(!expired(t0, t0 + Duration::from_secs(59), ttl));
        assert!(expired(t0, t0 + Duration::from_secs(60), ttl));
        assert!(expired(t0, t0 + Duration::from_secs(61), ttl));
        let mut d = DirSizes::default();
        d.set_tuning(Tuning {
            ttl,
            threads: 1,
            ..Tuning::default()
        });
        let p = PathBuf::from("/ttl/a");
        d.remember(p.clone(), V, t0, true, Some(7));
        let soon = t0 + Duration::from_secs(30);
        assert!(!d.wants_refresh(&p, soon));
        col(&mut d, &p, soon);
        assert_eq!(
            d.running_len() + d.queue_len(),
            0,
            "유효한 캐시 = 요청 없음"
        );
        let late = t0 + Duration::from_secs(61);
        assert_eq!(d.get(&p), SizeState::Done(V), "만료돼도 값은 보여 준다");
        assert!(d.wants_refresh(&p, late));
        col(&mut d, &p, late);
        assert_eq!(d.running_len(), 1, "만료 = 뒤에서 다시 잰다");
        assert_eq!(d.get(&p), SizeState::Done(V), "재는 동안에도 옛 값");
        assert!(!d.wants_refresh(&p, late), "이미 재는 중 = 다시 요청 안 함");
        // 일부(선점 취소) 값도 같은 길: 보여 주되 다시 잰다.
        let q = PathBuf::from("/ttl/b");
        d.remember(q.clone(), V, late, false, None);
        assert!(d.wants_refresh(&q, late));
        d.clear();
    }

    /// 보이는 것 우선(사용자 11-09): A1·A2를 재는 중/대기 중에 형제 B로 가면(보이는 집합 = B1·B2) A2는 큐에서 빠지고, B1이
    /// 기다리는데 슬롯이 A1(안 보임)로 차 있으면 A1을 선점 취소해 B1이 바로 시작한다. 도크·수동 요청은 보임과 무관하게 남는다.
    /// 같은 집합을 다시 알리면 무동작.
    #[test]
    fn hidden_column_requests_are_dropped_and_running_ones_preempted() {
        let mut d = DirSizes::default();
        d.set_tuning(Tuning {
            threads: 1,
            queue_max: 10,
            settle_ms: 0,
            ttl: None,
        });
        let now = Instant::now();
        let p = |s: &str| PathBuf::from(format!("/p/{s}"));
        let (a1, a2, b1, b2, m) = (p("a/1"), p("a/2"), p("b/1"), p("b/2"), p("m"));
        d.set_visible([a1.clone(), a2.clone()].into_iter().collect());
        col(&mut d, &a1, now);
        col(&mut d, &a2, now);
        d.request_back(&m, now, Origin::Manual, None);
        assert_eq!(d.running[0].path, a1);
        assert_eq!(d.queued_paths(), vec![a2.clone(), m.clone()]);
        // 형제 B로 이동: 보이는 집합이 B1·B2 — A2 제거 · 수동 m은 유지 · m이 기다리므로 A1 선점.
        d.set_visible([b1.clone(), b2.clone()].into_iter().collect());
        assert_eq!(
            d.get(&a2),
            SizeState::Unknown,
            "안 보이는 열 요청은 큐에서 뺀다"
        );
        assert_eq!(
            d.running[0].path, m,
            "기다리던 수동 요청이 선점한 슬롯을 받는다"
        );
        assert!(d.retired.iter().any(|j| j.path == a1), "A1 = 선점 취소");
        assert!(d.retired[0].shared.cancel.load(Ordering::Relaxed));
        // B1·B2 요청: 슬롯은 수동 m이 쓰는 중(선점 대상 아님) → 큐에서 기다린다.
        col(&mut d, &b1, now);
        col(&mut d, &b2, now);
        assert_eq!(d.queued_paths(), vec![b1.clone(), b2.clone()]);
        assert_eq!(d.running_len(), 1);
        // 같은 집합 = 무동작.
        d.changed = false;
        d.set_visible([b1.clone(), b2.clone()].into_iter().collect());
        assert!(!d.changed);
        // 보이는 열 작업은 선점하지 않는다: m이 끝나 b1이 돌고 있을 때 b2가 기다려도 b1은 그대로.
        d.clear();
        d.set_visible([b1.clone(), b2.clone()].into_iter().collect());
        col(&mut d, &b1, now);
        col(&mut d, &b2, now);
        d.request(&p("dock"), now); // 도크 요청: 슬롯의 b1은 보이는 열 작업 · 오래 돌지도 않음 → 선점 없음
        assert_eq!(d.running[0].path, b1);
        assert_eq!(d.queued_paths(), vec![p("dock"), b2.clone()]);
        d.clear();
    }

    /// 선점 취소된 작업의 진행값은 "일부"로 캐시에 남고(돌아오면 바로 보이고 뒤에서 마저 잰다) · 진행값이 없으면 남기지 않는다 ·
    /// 취소 표시가 있어도 끝까지 돌았으면 온전한 결과다.
    #[test]
    fn preempted_job_keeps_partial_progress() {
        let mut d = DirSizes::default();
        let now = Instant::now();
        let job = |path: &str, bytes: u64, aborted: bool| {
            let sh = DirSizeShared::default();
            sh.bytes.store(bytes, Ordering::Relaxed);
            sh.files.store(u64::from(bytes > 0), Ordering::Relaxed);
            sh.aborted.store(aborted, Ordering::Relaxed);
            sh.cancel.store(true, Ordering::Relaxed);
            sh.done.store(true, Ordering::Relaxed);
            DirSizeJob {
                path: PathBuf::from(path),
                shared: Arc::new(sh),
                started: now,
                origin: Origin::Column,
                mtime_ms: Some(3),
            }
        };
        d.retired.push(job("/r/partial", 500, true));
        d.retired.push(job("/r/empty", 0, true));
        d.retired.push(job("/r/whole", 900, false));
        d.tick(now);
        assert!(d.retired.is_empty());
        assert_eq!(
            d.get(Path::new("/r/partial")),
            SizeState::Done(DirSize {
                bytes: 500,
                files: 1,
                dirs: 0,
                partial: true
            })
        );
        assert!(
            d.wants_refresh(Path::new("/r/partial"), now),
            "일부 = 보이면 다시 잰다"
        );
        assert_eq!(d.get(Path::new("/r/empty")), SizeState::Unknown);
        assert_eq!(
            d.get(Path::new("/r/whole")),
            SizeState::Done(DirSize {
                bytes: 900,
                files: 1,
                dirs: 0,
                partial: false
            })
        );
        assert!(
            !d.wants_refresh(Path::new("/r/whole"), now),
            "끝까지 돌았으면 온전"
        );
    }

    /// mtime 가드(11-09): 요청 때 본 수정 시각과 행의 값이 다르면 불일치 · 같으면 아니다 · 모르는 항목(도크 요청)은 비교하지 않는다 ·
    /// 없는 항목도 아니다. 불일치 항목을 `invalidate`하면 상위 폴더 캐시도 함께 버려진다.
    #[test]
    fn mtime_mismatch_invalidates_entry_and_ancestors() {
        let mut d = DirSizes::default();
        let now = Instant::now();
        let (root, a, a1) = (
            PathBuf::from("/m"),
            PathBuf::from("/m/a"),
            PathBuf::from("/m/a/1"),
        );
        d.remember(root.clone(), V, now, true, None);
        d.remember(a.clone(), V, now, true, Some(10));
        d.remember(a1.clone(), V, now, true, Some(20));
        assert!(!d.mtime_mismatch(&a, 10));
        assert!(d.mtime_mismatch(&a, 11));
        assert!(!d.mtime_mismatch(&root, 99), "모르는 시각 = 비교 안 함");
        assert!(!d.mtime_mismatch(Path::new("/m/zzz"), 1));
        assert!(d.invalidate(std::slice::from_ref(&a)));
        assert!(
            d.cached(&a).is_none() && d.cached(&root).is_none(),
            "자신 + 상위"
        );
        assert!(d.cached(&a1).is_none(), "안쪽도");
    }

    /// 훑기: 바이트 · 파일 · 폴더 합계 · 빈 폴더 · 없는 폴더 = 오류 1 · 취소 = 즉시 그침(`aborted`).
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
        assert!(!sh.aborted.load(Ordering::Relaxed));
        let sh = DirSizeShared::default();
        walk(&d.join("nope"), &sh);
        assert!(sh.snapshot().partial);
        let sh = DirSizeShared::default();
        sh.cancel.store(true, Ordering::Relaxed);
        walk(&d, &sh);
        assert_eq!(sh.snapshot().files, 0);
        assert!(sh.aborted.load(Ordering::Relaxed));
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
            ttl: None,
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
            ttl: None,
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
