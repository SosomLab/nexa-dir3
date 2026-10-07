//! App — 압축 풀기(T-169 1차 · NEW-040 · dir3 신규 · 사용자 10-05 "대상 목록"): 우클릭 "압축 풀기 ▸ 여기에 / "<이름>" 폴더에".
//! 엔진 = `ndir_vfs::archive::extract`(zip Store/Deflate · tar · gz/tgz · zip slip 차단 · 기존 파일 건너뜀). 작업 스레드 1개 ·
//! 전송과 같은 슬롯(전송 중이거나 다른 풀기가 진행 중이면 거부) · 진행 = 전송 진행 창 재사용(`transfer.close_ms > 0`일 때) ·
//! 취소 = 진행 창 [취소]/X · 끝 = 토스트 + 상태줄 + 재열람.

use crate::*;
use ndir_vfs::archive::extract::{self as xt, Report};
use ndir_vfs::archive::ArchiveError;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type ExtractOutcome = Result<Report, ArchiveError>;

#[derive(Debug, Default)]
pub(crate) struct ExtractShared {
    cancel: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
    finished: AtomicBool,
    result: Mutex<Option<ExtractOutcome>>,
}

#[derive(Debug)]
pub(crate) struct ExtractJob {
    pub(crate) dest: PathBuf,
    shared: Arc<ExtractShared>,
}

/// 1차 풀기가 다루는 확장자(메뉴 노출 판정 — 파일을 열지 않는다 · 시그니처 판정은 엔진이 한다).
pub(crate) fn extract_ext_ok(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "zip" | "tar" | "gz" | "tgz"))
}

/// 대상 폴더(순수): `here` = 압축 파일이 있는 폴더 · 아니면 그 안의 `<이름>` 폴더(`.tar.gz`는 `.tar`까지 벗긴다 ·
/// `a.zip` → `a` · `a.tgz` → `a`).
pub(crate) fn extract_dest(archive: &Path, here: bool) -> PathBuf {
    let parent = archive
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    if here {
        return parent;
    }
    let stem = archive
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "extracted".into());
    let stem = stem
        .strip_suffix(".tar")
        .or_else(|| stem.strip_suffix(".TAR"))
        .unwrap_or(&stem)
        .to_string();
    parent.join(if stem.is_empty() {
        "extracted".to_string()
    } else {
        stem
    })
}

/// 결과 안내 글(순수).
pub(crate) fn extract_summary(rep: &Report) -> String {
    let mut s = trf(
        "extract.done",
        &[&rep.files.to_string(), &rep.dirs.to_string()],
    );
    let skipped = rep.skipped_existing + rep.unsupported + rep.suspicious + rep.errors.len() as u64;
    if skipped > 0 {
        s.push_str(" · ");
        s.push_str(&trf(
            "extract.skipped",
            &[
                &rep.skipped_existing.to_string(),
                &rep.unsupported.to_string(),
                &rep.suspicious.to_string(),
                &rep.errors.len().to_string(),
            ],
        ));
    }
    if rep.canceled {
        s.push_str(" · ");
        s.push_str(&tr("ops.canceled"));
    }
    s
}

impl App {
    /// `ctx.extract_here` / `ctx.extract_to` — 풀기 시작(전송 · 다른 풀기가 진행 중이면 거부).
    pub(crate) fn start_extract(&mut self, archive: &Path, here: bool) {
        if self.transfer.is_some() || self.extract_job.is_some() || self.sync_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("ops.busy"), &mut inv);
            self.redraw();
            return;
        }
        let dest = extract_dest(archive, here);
        let total = xt::total_bytes(archive).unwrap_or(0);
        let shared = Arc::new(ExtractShared::default());
        shared.total.store(total, Ordering::Relaxed);
        let sh = Arc::clone(&shared);
        let (src, dst) = (archive.to_path_buf(), dest.clone());
        std::thread::spawn(move || {
            let r = xt::extract(
                &src,
                &dst,
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
        let label = trf("extract.label", &[&ndir_ops::leaf_name(archive)]);
        if self.settings.int("transfer.close_ms") > 0 {
            self.progress_win.reset(&label);
            self.progress_win.update(0, total.max(1), Vec::new(), 1, 1);
            self.open_progress = true;
        }
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&label, &mut inv);
        self.extract_job = Some(ExtractJob { dest, shared });
        self.redraw();
    }

    /// 폴링 — 진행 창 · 취소 · 완료 수거. 돌려주는 값 = 아직 진행 중.
    pub(crate) fn extract_tick(&mut self) -> bool {
        let Some(job) = &self.extract_job else {
            return false;
        };
        let (done, total) = (
            job.shared.done.load(Ordering::Relaxed),
            job.shared.total.load(Ordering::Relaxed).max(1),
        );
        if self.progress_win.is_active() {
            self.progress_win
                .update(done.min(total), total, Vec::new(), 1, 1);
            if self.progress_win.take_cancelled() {
                job.shared.cancel.store(true, Ordering::Relaxed);
            }
        }
        if !job.shared.finished.load(Ordering::Relaxed) {
            return true;
        }
        let job = self.extract_job.take().expect("checked above");
        self.mem_after_job();
        let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
        let (text, warn) = match result {
            Some(Ok(rep)) => {
                let warn = rep.canceled
                    || rep.unsupported + rep.suspicious + rep.skipped_existing > 0
                    || !rep.errors.is_empty();
                (extract_summary(&rep), warn)
            }
            Some(Err(e)) => (trf("extract.failed", &[&format!("{e:?}")]), true),
            None => (tr("extract.failed"), true),
        };
        if self.progress_win.is_active() {
            let ms = self.settings.int("transfer.close_ms").max(0) as u64;
            let now = self.started.elapsed().as_millis() as u64;
            self.progress_win.update(total, total, Vec::new(), 1, 1);
            self.progress_win.set_done(&text, ms, now);
        }
        self.dirsizes.clear();
        self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        // 풀어 넣은 폴더(또는 여기에 풀었으면 압축 파일이 있던 폴더의 선택 그대로)를 가리킨다.
        if job.dest.is_dir() && self.panels[self.active].root_path() != job.dest {
            self.panels[self.active].select_path(&job.dest, &mut inv);
        }
        self.toasts.push(
            if warn {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("ctx.extract"),
            text.clone(),
        );
        self.statusbar.set_left(&text, &mut inv);
        self.update_status();
        self.redraw();
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dest_folder_rules() {
        let a = Path::new("D:/x/a.zip");
        assert_eq!(extract_dest(a, true), PathBuf::from("D:/x"));
        assert_eq!(extract_dest(a, false), PathBuf::from("D:/x").join("a"));
        assert_eq!(
            extract_dest(Path::new("D:/x/b.tar.gz"), false),
            PathBuf::from("D:/x").join("b")
        );
        assert_eq!(
            extract_dest(Path::new("D:/x/c.tgz"), false),
            PathBuf::from("D:/x").join("c")
        );
        assert!(extract_ext_ok(Path::new("q.ZIP")) && !extract_ext_ok(Path::new("q.7z")));
    }
}
