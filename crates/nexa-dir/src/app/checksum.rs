//! App — 체크섬 계산(T-167 · NEW-038 · 사용자 10-05 · 10-06 개편): 파일 **여러 개**를 작업 스레드 1개가 차례로 계산(파일마다
//! 알고리즘들을 한 번 읽기) · **작업 슬롯 1개**(진행 중 두 번째 요청 = 안내 · 거부) · 스레드 우선순위 낮춤(UI · 다른 작업에 양보) ·
//! 합계가 설정 `hash.auto_limit_mb`를 넘으면 자동 시작하지 않고 창의 [계산]을 기다린다(0 = 언제나 수동). 폴링 = `OPS_POLL_MS` 틱.

use crate::*;
use ndir_ops::hash::{self, Algo};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 파일 1건의 결과(알고리즘 · 16진) 또는 읽기 오류 글.
type FileOutcome = Result<Vec<(Algo, String)>, String>;

/// 작업 스레드 ↔ UI 공유 상태.
#[derive(Debug, Default)]
pub(crate) struct HashShared {
    cancel: AtomicBool,
    /// 읽은 바이트 누계(모든 파일).
    done: AtomicU64,
    /// 지금 계산 중인 파일 번호.
    current: AtomicUsize,
    finished: AtomicBool,
    /// 끝난 파일의 결과(번호 · 결과) — UI가 꺼내 간다.
    results: Mutex<Vec<(usize, FileOutcome)>>,
}

/// 진행 중인 계산.
#[derive(Debug)]
pub(crate) struct HashJob {
    /// 대상(시험 · 안내용).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) paths: Vec<PathBuf>,
    shared: Arc<HashShared>,
}

/// 설정 `hash.algos`(쉼표 목록) → 알고리즘(순수 · 모르는 키 무시 · 비면 기본 4종).
pub(crate) fn parse_algos(text: &str) -> Vec<Algo> {
    let mut v: Vec<Algo> = text
        .split([',', ' ', ';'])
        .filter_map(|k| Algo::from_key(k.trim().to_ascii_lowercase().as_str()))
        .collect();
    v.dedup();
    if v.is_empty() {
        v = vec![Algo::Crc32, Algo::Md5, Algo::Sha1, Algo::Sha256];
    }
    // 표시 순서는 고정.
    v.sort_by_key(|a| Algo::ALL.iter().position(|x| x == a));
    v.dedup();
    v
}

/// 자동 시작 판정(순수): 상한 0 = 늘 수동 · 합계가 상한(MB) 이하일 때만 자동.
pub(crate) fn auto_start(total_bytes: u64, limit_mb: i64) -> bool {
    limit_mb > 0 && total_bytes <= (limit_mb as u64).saturating_mul(1024 * 1024)
}

impl App {
    /// `ctx.checksum` — 선택한 파일들의 체크섬 창을 연다(폴더는 뺀다). 진행 중인 계산이 있으면 거부(슬롯 1개).
    pub(crate) fn open_checksum(&mut self, paths: &[PathBuf]) {
        if self.hash_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("hash.busy"), &mut inv);
            self.toasts
                .push(toast::ToastKind::Warn, tr("hash.title"), tr("hash.busy"));
            self.redraw();
            return;
        }
        let files: Vec<(PathBuf, u64)> = paths
            .iter()
            .filter_map(|p| {
                let md = std::fs::metadata(p).ok()?;
                md.is_file().then(|| (p.clone(), md.len()))
            })
            .collect();
        if files.is_empty() {
            return;
        }
        let algos = parse_algos(self.settings.get("hash.algos").unwrap_or(""));
        let total: u64 = files.iter().map(|(_, s)| *s).sum();
        let auto = auto_start(total, self.settings.int("hash.auto_limit_mb"));
        self.hash_win.start(&files, &algos, auto);
        if auto {
            self.hash_begin(files.into_iter().map(|(p, _)| p).collect(), algos);
        }
        self.open_hash = true;
    }

    /// 창의 [계산] — 상한을 넘어 대기하던 계산을 시작한다.
    pub(crate) fn checksum_start(&mut self) {
        if self.hash_job.is_some() {
            return;
        }
        let paths: Vec<PathBuf> = self
            .hash_win
            .files()
            .iter()
            .map(|f| f.path.clone())
            .collect();
        if paths.is_empty() {
            return;
        }
        let algos = parse_algos(self.settings.get("hash.algos").unwrap_or(""));
        self.hash_win.begin();
        self.hash_begin(paths, algos);
    }

    fn hash_begin(&mut self, paths: Vec<PathBuf>, algos: Vec<Algo>) {
        let shared = Arc::new(HashShared::default());
        let sh = Arc::clone(&shared);
        let list = paths.clone();
        std::thread::spawn(move || {
            // UI와 다른 작업에 양보(사용자 10-06 "해시가 프로세스를 잡아먹어 멈추거나 느려진다").
            platform::lower_thread_priority();
            for (i, p) in list.iter().enumerate() {
                if sh.cancel.load(Ordering::Relaxed) {
                    break;
                }
                sh.current.store(i, Ordering::Relaxed);
                let r = hash::hash_file(
                    p,
                    &algos,
                    &mut |n| {
                        sh.done.fetch_add(n, Ordering::Relaxed);
                    },
                    &sh.cancel,
                );
                let out: FileOutcome = match r {
                    Ok(v) => Ok(v),
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => break,
                    Err(e) => Err(e.to_string()),
                };
                if let Ok(mut slot) = sh.results.lock() {
                    slot.push((i, out));
                }
            }
            sh.finished.store(true, Ordering::Relaxed);
        });
        self.hash_job = Some(HashJob { paths, shared });
    }

    /// 폴링 — 진행값 · 끝난 파일 결과를 창에 넣고, 전부 끝났으면 마감. 돌려주는 값 = 아직 계산 중.
    pub(crate) fn hash_tick(&mut self) -> bool {
        let Some(job) = &self.hash_job else {
            return false;
        };
        self.hash_win.update(
            job.shared.done.load(Ordering::Relaxed),
            job.shared.current.load(Ordering::Relaxed),
        );
        let ready: Vec<(usize, FileOutcome)> = job
            .shared
            .results
            .lock()
            .map(|mut r| r.drain(..).collect())
            .unwrap_or_default();
        for (i, out) in ready {
            self.hash_win.set_file_result(i, out);
        }
        if !job.shared.finished.load(Ordering::Relaxed) {
            return true;
        }
        let job = self.hash_job.take().expect("checked above");
        self.mem_after_job();
        let canceled = job.shared.cancel.load(Ordering::Relaxed);
        self.hash_win.finish(canceled.then(|| tr("ops.canceled")));
        false
    }

    /// 창을 닫는다 — 진행 중이면 계산도 취소.
    pub(crate) fn close_checksum(&mut self) {
        if let Some(job) = self.hash_job.take() {
            job.shared.cancel.store(true, Ordering::Relaxed);
        }
        self.hash_win.close();
        if let Some(w) = &self.window {
            w.focus_window();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_start_limit() {
        assert!(auto_start(10 << 20, 512));
        assert!(auto_start(512 << 20, 512));
        assert!(!auto_start((512 << 20) + 1, 512));
        assert!(!auto_start(1, 0), "0 = 늘 수동");
    }

    #[test]
    fn parse_algos_rules() {
        assert_eq!(
            parse_algos(""),
            vec![Algo::Crc32, Algo::Md5, Algo::Sha1, Algo::Sha256]
        );
        assert_eq!(
            parse_algos("sha512, crc32,bogus,SHA256"),
            vec![Algo::Crc32, Algo::Sha256, Algo::Sha512]
        );
    }
}
