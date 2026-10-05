//! App — 체크섬 계산(T-167 · NEW-038 · 사용자 10-05 "큰 파일은 메인 프로세스에 영향 → 별도 스레드 · 진행 확인 · 그동안 다른 해시
//! 요청은 제한"): 작업 스레드 1개 · **작업 슬롯 1개**(진행 중 두 번째 요청 = 상태줄 안내 · 거부) · 진행 = 창 안 막대 ·
//! 취소 = 창 닫기. 폴링 = 기존 `OPS_POLL_MS` 틱.

use crate::*;
use ndir_ops::hash::{self, Algo};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 계산 결과(알고리즘 · 16진 값) 또는 읽기 오류.
type HashOutcome = std::io::Result<Vec<(Algo, String)>>;

/// 작업 스레드 ↔ UI 공유 상태.
#[derive(Debug, Default)]
pub(crate) struct HashShared {
    cancel: AtomicBool,
    done: AtomicU64,
    finished: AtomicBool,
    result: Mutex<Option<HashOutcome>>,
}

/// 진행 중인 계산.
#[derive(Debug)]
pub(crate) struct HashJob {
    /// 대상(시험 · 안내용).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) path: PathBuf,
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

impl App {
    /// `ctx.checksum` — 파일 1개의 체크섬 창을 열고 계산을 시작한다. 진행 중인 계산이 있으면 거부(슬롯 1개).
    pub(crate) fn open_checksum(&mut self, path: &Path) {
        if self.hash_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("hash.busy"), &mut inv);
            self.toasts
                .push(toast::ToastKind::Warn, tr("hash.title"), tr("hash.busy"));
            self.redraw();
            return;
        }
        let Ok(md) = std::fs::metadata(path) else {
            return;
        };
        if !md.is_file() {
            return;
        }
        let algos = parse_algos(self.settings.get("hash.algos").unwrap_or(""));
        let shared = Arc::new(HashShared::default());
        let sh = Arc::clone(&shared);
        let p = path.to_path_buf();
        let list = algos.clone();
        std::thread::spawn(move || {
            let r = hash::hash_file(
                &p,
                &list,
                &mut |n| {
                    sh.done.fetch_add(n, Ordering::Relaxed);
                },
                &sh.cancel,
            );
            if let Ok(mut slot) = sh.result.lock() {
                *slot = Some(r);
            }
            sh.finished.store(true, Ordering::Relaxed);
        });
        self.hash_job = Some(HashJob {
            path: path.to_path_buf(),
            shared,
        });
        self.hash_win.start(path, md.len(), &algos);
        self.open_hash = true;
    }

    /// 폴링 — 진행값을 창에 넣고, 끝났으면 결과/오류를 넘긴다. 돌려주는 값 = 아직 계산 중.
    pub(crate) fn hash_tick(&mut self) -> bool {
        let Some(job) = &self.hash_job else {
            return false;
        };
        self.hash_win
            .update(job.shared.done.load(Ordering::Relaxed));
        if !job.shared.finished.load(Ordering::Relaxed) {
            return true;
        }
        let job = self.hash_job.take().expect("checked above");
        let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
        match result {
            Some(Ok(list)) => self.hash_win.set_results(&list),
            Some(Err(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                self.hash_win.set_error(&tr("ops.canceled"));
            }
            Some(Err(e)) => self
                .hash_win
                .set_error(&trf("hash.failed", &[&e.to_string()])),
            None => self.hash_win.set_error(&tr("hash.failed")),
        }
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
