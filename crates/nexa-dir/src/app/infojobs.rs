//! 정보 도크 **메타데이터 워커**(T-189 ③ · 10-18 구조 점검 "정보 도크 행 이동마다 `metadata` + `size_on_disk`가 UI 스레드"):
//! 선택 1개의 기본 정보(`dockinfo::basic` = 크기 · 세 시각)와 디스크 할당 크기를 이 워커가 잰다. 느린 드라이브(네트워크 · 잠든 USB)에서
//! 행을 옮길 때마다 UI가 멈추던 것을 없앤다.
//!
//! 구조(docs/01 §3 · [`super::imgjobs`]와 같은 꼴): 워커 스레드 1(첫 요청 때 생성) · 요청은 도크마다 **마지막 것만** · 세대 번호로 낡은 결과
//! 폐기 · 결과는 `mpsc` 큐를 유휴 틱이 폴링. 요청 직후 `dock.info_wait_ms`(기본 30)까지는 그 자리에서 기다려 빠른 디스크는 종전처럼
//! 바로 보인다(빈 줄 깜빡임 없음) — 넘으면 이름 · 경로 줄만 먼저 보이고 결과가 오면 틱이 채운다. 결과는 잰 시각과 함께 보관해
//! [`FRESH_MS`] 안의 재계산(폴더 크기 틱 · 상태줄 갱신)은 다시 묻지 않는다.

use crate::dockinfo::{self, Basic};
use crate::*;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};

/// 보관 결과를 다시 묻지 않고 쓰는 시간 — 그 뒤 같은 대상의 갱신은 새로 잰다(바뀐 크기 · 시각 반영).
pub(crate) const FRESH_MS: u64 = 1000;

/// 할당 크기 조회 함수(`Disk::size_on_disk_fn` — 다른 스레드에서 불러도 되는 OS 구현).
type OnDiskFn = fn(&Path) -> Option<u64>;

struct Req {
    gen: u64,
    dock: usize,
    path: PathBuf,
    on_disk: Option<OnDiskFn>,
}

struct Res {
    gen: u64,
    dock: usize,
    probe: Probe,
}

/// 한 경로의 잰 값 — `on_disk` = 워커가 잰 할당 크기(`None` = 워커가 묻지 않았다 → 호출부가 동기로).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Probe {
    pub path: PathBuf,
    pub basic: Option<Basic>,
    pub on_disk: Option<Option<u64>>,
}

/// 요청 보내기 · 결과 받기 · 도크별 세대 · 완료 결과(잰 시각 포함) 보관.
pub(crate) struct InfoProber {
    tx: Option<Sender<Req>>,
    rx: Option<Receiver<Res>>,
    gen: [u64; 2],
    pending: [Option<PathBuf>; 2],
    ready: [Option<(Instant, Probe)>; 2],
}

impl InfoProber {
    pub(crate) fn new() -> Self {
        InfoProber {
            tx: None,
            rx: None,
            gen: [0; 2],
            pending: [None, None],
            ready: [None, None],
        }
    }

    fn ensure_worker(&mut self) {
        if self.tx.is_some() {
            return;
        }
        let (tx, rx_req) = channel::<Req>();
        let (res_tx, rx) = channel::<Res>();
        std::thread::Builder::new()
            .name("ndir-info".into())
            .spawn(move || {
                while let Ok(first) = rx_req.recv() {
                    // 밀린 요청은 도크마다 마지막 것만(빠르게 지나간 행은 건너뜀).
                    let mut latest: [Option<Req>; 2] = [None, None];
                    let d = first.dock.min(1);
                    latest[d] = Some(first);
                    while let Ok(r) = rx_req.try_recv() {
                        let d = r.dock.min(1);
                        latest[d] = Some(r);
                    }
                    for req in latest.into_iter().flatten() {
                        let basic = dockinfo::basic(&req.path);
                        // 할당 크기는 파일일 때만(폴더 = 줄 없음 · `info_lines`와 같은 조건).
                        let on_disk = req.on_disk.map(|f| match &basic {
                            Some(b) if !b.is_dir => f(&req.path),
                            _ => None,
                        });
                        let probe = Probe {
                            path: req.path,
                            basic,
                            on_disk,
                        };
                        let res = Res {
                            gen: req.gen,
                            dock: req.dock,
                            probe,
                        };
                        if res_tx.send(res).is_err() {
                            return;
                        }
                    }
                }
            })
            .expect("spawn info worker");
        self.tx = Some(tx);
        self.rx = Some(rx);
    }

    /// 도크 `dock`의 보관 결과 — 같은 경로이고 `now`가 잰 뒤 [`FRESH_MS`] 안일 때만.
    pub(crate) fn fresh(&self, dock: usize, path: &Path, now: Instant) -> Option<&Probe> {
        self.ready[dock.min(1)]
            .as_ref()
            .filter(|(at, p)| {
                p.path == path && now.duration_since(*at) < Duration::from_millis(FRESH_MS)
            })
            .map(|(_, p)| p)
    }

    /// 도크 `dock`의 마지막 보관 결과(신선도 무관 · 같은 경로일 때) — 새로 재는 동안 옛 값을 보여 줄 때.
    pub(crate) fn last(&self, dock: usize, path: &Path) -> Option<&Probe> {
        self.ready[dock.min(1)]
            .as_ref()
            .filter(|(_, p)| p.path == path)
            .map(|(_, p)| p)
    }

    /// 같은 경로가 이미 워커에 가 있는가.
    pub(crate) fn is_pending(&self, dock: usize, path: &Path) -> bool {
        self.pending[dock.min(1)].as_deref() == Some(path)
    }

    /// 도크 `dock`에 `path`를 재게 맡긴다(세대 올림 — 앞선 결과는 버려진다).
    pub(crate) fn request(&mut self, dock: usize, path: PathBuf, on_disk: Option<OnDiskFn>) {
        self.ensure_worker();
        let dock = dock.min(1);
        self.gen[dock] += 1;
        self.pending[dock] = Some(path.clone());
        if let Some(tx) = &self.tx {
            let _ = tx.send(Req {
                gen: self.gen[dock],
                dock,
                path,
                on_disk,
            });
        }
    }

    fn accept(&mut self, res: Res, done: &mut Vec<usize>) {
        let dock = res.dock.min(1);
        if res.gen != self.gen[dock] {
            return; // 낡은 결과(그 사이 다른 행으로 옮겼다)
        }
        self.ready[dock] = Some((Instant::now(), res.probe));
        self.pending[dock] = None;
        done.push(dock);
    }

    /// 결과 수거(UI 스레드) — 현재 세대 결과만 보관하고 그 도크 번호를 돌려준다.
    pub(crate) fn poll(&mut self) -> Vec<usize> {
        let mut done = Vec::new();
        let Some(rx) = self.rx.take() else {
            return done;
        };
        while let Ok(res) = rx.try_recv() {
            self.accept(res, &mut done);
        }
        self.rx = Some(rx);
        done
    }

    /// 기다리는 요청이 다 올 때까지 최대 `limit` 그 자리에서 받는다(요청 직후 · 빠른 디스크 = 깜빡임 없음).
    pub(crate) fn wait(&mut self, limit: Duration) {
        let Some(rx) = self.rx.take() else {
            return;
        };
        let until = Instant::now() + limit;
        let mut done = Vec::new();
        while self.busy() {
            let left = until.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(res) => self.accept(res, &mut done),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        self.rx = Some(rx);
    }

    /// 결과를 기다리는 도크가 있는가(이벤트 루프가 짧게 깬다).
    pub(crate) fn busy(&self) -> bool {
        self.pending.iter().any(Option::is_some)
    }
}

impl App {
    /// 정보 도크 `i`가 보여 줄 선택 1개의 잰 값 — 창 없는 시험은 `None`(종전 동기 경로) · 신선한 보관 결과 → 그대로 ·
    /// 아니면 워커에 맡기고 `dock.info_wait_ms`까지 기다린다(그래도 없으면 같은 경로의 옛 값 · 없으면 빈 값 = 이름 · 경로 줄만).
    pub(crate) fn info_probe_for(&mut self, i: usize, path: &Path) -> Option<Probe> {
        self.window.as_ref()?;
        let now = Instant::now();
        if let Some(p) = self.info_probe.fresh(i, path, now) {
            return Some(p.clone());
        }
        if !self.info_probe.is_pending(i, path) {
            let f = self.platform.disk.size_on_disk_fn();
            self.info_probe.request(i, path.to_path_buf(), f);
        }
        let wait = self.settings.int("dock.info_wait_ms").max(0) as u64;
        self.info_probe.wait(Duration::from_millis(wait));
        if let Some(p) = self.info_probe.fresh(i, path, Instant::now()) {
            return Some(p.clone());
        }
        Some(self.info_probe.last(i, path).cloned().unwrap_or(Probe {
            path: path.to_path_buf(),
            basic: None,
            on_disk: None,
        }))
    }

    /// 유휴 틱: 끝난 측정이 있으면 정보 도크만 다시 계산한다(`true` = 다시 그릴 것).
    pub(crate) fn info_probe_tick(&mut self) -> bool {
        if self.info_probe.poll().is_empty() {
            return false;
        }
        self.update_docks_filtered(true);
        true
    }

    /// 측정 결과를 기다린다(덤프·단언 · T4 결정성 · 상한 안에서).
    pub(crate) fn info_probe_wait(&mut self, limit: Duration) {
        if self.info_probe.busy() {
            self.info_probe.wait(limit);
            self.update_docks_filtered(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_on_disk(_p: &Path) -> Option<u64> {
        Some(4096)
    }

    /// 요청 → 워커 → 결과 보관(파일 = 할당 크기 · 폴더 = 없음) · 연타 = 마지막 것만 · 신선도 창 · 낡은 결과 버림 · busy 전이.
    #[test]
    fn worker_probes_latest_path_and_keeps_fresh_result() {
        let dir = std::env::temp_dir().join(format!("ndir-infojobs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        std::fs::write(&a, b"aa").unwrap();
        std::fs::write(&b, b"bbbbb").unwrap();
        let mut w = InfoProber::new();
        assert!(!w.busy());
        w.request(0, a.clone(), Some(fake_on_disk));
        w.request(0, b.clone(), Some(fake_on_disk)); // 연타 = a는 낡은 세대
        assert!(w.busy() && w.is_pending(0, &b) && !w.is_pending(0, &a));
        w.wait(Duration::from_secs(10));
        assert!(!w.busy());
        assert!(w.fresh(0, &a, Instant::now()).is_none(), "낡은 세대는 버림");
        let p = w.fresh(0, &b, Instant::now()).expect("b");
        assert_eq!(p.basic.as_ref().and_then(|x| x.size), Some(5));
        assert_eq!(p.on_disk, Some(Some(4096)));
        let later = Instant::now() + Duration::from_millis(FRESH_MS + 1);
        assert!(w.fresh(0, &b, later).is_none(), "신선도 지남");
        assert!(w.last(0, &b).is_some(), "옛 값은 남는다");
        // 폴더 = 할당 크기 묻지 않음 · 조회 함수 없음 = 워커가 묻지 않았다(None).
        w.request(1, dir.join("sub"), Some(fake_on_disk));
        w.wait(Duration::from_secs(10));
        let s = w.fresh(1, &dir.join("sub"), Instant::now()).expect("sub");
        assert!(s.basic.as_ref().is_some_and(|x| x.is_dir));
        assert_eq!(s.on_disk, Some(None));
        w.request(1, a.clone(), None);
        let at = Instant::now();
        let mut done = Vec::new();
        while done.is_empty() && at.elapsed() < Duration::from_secs(10) {
            done = w.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(done, vec![1]);
        assert_eq!(w.fresh(1, &a, Instant::now()).expect("a").on_disk, None);
        // 없는 경로 = basic None.
        w.request(0, dir.join("nope"), None);
        w.wait(Duration::from_secs(10));
        assert!(w
            .fresh(0, &dir.join("nope"), Instant::now())
            .expect("nope")
            .basic
            .is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
