//! App — 내장 zip 압축(DR-31 ⑳ⓑ · 10-25 · T-187 "Linux 압축하기 없음"): 우클릭 "압축"(Linux `xdg.compress`)이 외부 아카이버
//! (file-roller · ark · xarchiver …)가 없는 기기에서도 동작하도록 `ndir_vfs::archive::zipwrite`(store · 외부 crate 0)로 직접 쓴다.
//! 구조는 압축 풀기(`extract.rs`)와 같다: 작업 스레드 1개 · 전송과 같은 슬롯 · 진행 = 전송 진행 창 재사용 · 취소 = 진행 창 ·
//! 끝 = 토스트 + 상태줄 + 재열람 + 새 zip 선택. **store(무압축)** — 메뉴 이름이 "압축"이므로 라벨·토스트에 명시(협업 검토 ②) · DEFLATE = 후속 후보. 대상 이름 = 하나면 그 이름(확장자 제외) · 여럿이면 부모 폴더 이름 · 충돌 = " (2)".

use crate::*;
use ndir_vfs::archive::zipwrite::{self as zw, WriteReport};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type ZipOutcome = std::io::Result<WriteReport>;

#[derive(Debug, Default)]
pub(crate) struct ZipShared {
    cancel: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
    finished: AtomicBool,
    result: Mutex<Option<ZipOutcome>>,
}

#[derive(Debug)]
pub(crate) struct ZipJob {
    pub(crate) dest: PathBuf,
    shared: Arc<ZipShared>,
}

/// 새 zip 경로: 선택이 하나면 `<이름(확장자 제외)>.zip` · 여럿이면 `<부모 폴더 이름>.zip`(루트면 `archive.zip`) · 같은 폴더에 · 충돌 = " (2)"….
pub(crate) fn zip_dest(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?;
    let parent = first.parent()?;
    let stem = if paths.len() == 1 {
        if first.is_dir() {
            first.file_name().map(|s| s.to_string_lossy().into_owned())
        } else {
            first.file_stem().map(|s| s.to_string_lossy().into_owned())
        }
    } else {
        parent.file_name().map(|s| s.to_string_lossy().into_owned())
    }
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "archive".to_string());
    Some(ndir_ops::unique_dest(parent, &format!("{stem}.zip"), false))
}

/// 원본 바이트 합(진행 분모 · 링크 제외 · 읽기 실패는 0).
fn total_bytes(paths: &[PathBuf]) -> u64 {
    fn walk(p: &Path, acc: &mut u64) {
        let Ok(m) = std::fs::symlink_metadata(p) else {
            return;
        };
        if m.file_type().is_symlink() {
            return;
        }
        if m.is_dir() {
            if let Ok(rd) = std::fs::read_dir(p) {
                for e in rd.flatten() {
                    walk(&e.path(), acc);
                }
            }
        } else {
            *acc += m.len();
        }
    }
    let mut n = 0;
    for p in paths {
        walk(p, &mut n);
    }
    n
}

/// 결과 요약(토스트 · 상태줄). 두 번째 = 경고 여부.
fn zip_summary(rep: &WriteReport, dest: &Path) -> (String, bool) {
    let warn = rep.canceled || !rep.too_big.is_empty() || !rep.errors.is_empty();
    let text = if rep.canceled {
        tr("zip.canceled")
    } else if !rep.too_big.is_empty() {
        // 4 GiB 한도(ZIP64 없음) = 쓰지 않았다(엔진이 파일을 만들지 않는다 · 협업 검토 ③).
        trf("zip.tooBig", &[&rep.too_big.len().to_string()])
    } else {
        trf(
            "zip.done",
            &[
                &rep.files.to_string(),
                &rep.dirs.to_string(),
                &ndir_ops::leaf_name(dest),
            ],
        )
    };
    (text, warn)
}

impl App {
    /// 내장 zip 시작(우클릭 "압축" · 외부 아카이버 없음). 전송/풀기/다른 압축이 진행 중이면 거부(상태줄).
    pub(crate) fn start_zip(&mut self, panel: usize, paths: Vec<PathBuf>) {
        let Some(dest) = zip_dest(&paths) else {
            return;
        };
        if self.transfer.is_some()
            || self.extract_job.is_some()
            || self.sync_job.is_some()
            || self.zip_job.is_some()
        {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("ops.busy"), &mut inv);
            self.redraw();
            return;
        }
        let shared = Arc::new(ZipShared::default());
        let sh = Arc::clone(&shared);
        let (src, dst) = (paths, dest.clone());
        std::thread::spawn(move || {
            sh.total.store(total_bytes(&src), Ordering::Relaxed);
            let r = zw::write_zip(
                &src,
                &dst,
                &mut |done| {
                    sh.done.store(done, Ordering::Relaxed);
                },
                &sh.cancel,
            );
            if let Ok(mut slot) = sh.result.lock() {
                *slot = Some(r);
            }
            sh.finished.store(true, Ordering::Release); // 결과를 넣은 뒤(Release) — 수거는 Acquire
        });
        let label = trf("zip.label", &[&ndir_ops::leaf_name(&dest)]);
        if self.settings.int("transfer.close_ms") > 0 {
            self.progress_win.reset(&label);
            self.progress_win.update(0, 1, Vec::new(), 1, 1);
            self.open_progress = true;
        }
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&label, &mut inv);
        if panel < self.panels.len() && panel != self.active {
            self.set_active(panel);
        }
        self.zip_job = Some(ZipJob { dest, shared });
        self.redraw();
    }

    /// 폴링 — 진행 창 · 취소 · 완료 수거. 돌려주는 값 = 아직 진행 중.
    pub(crate) fn zip_tick(&mut self) -> bool {
        let Some(job) = &self.zip_job else {
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
        if !job.shared.finished.load(Ordering::Acquire) {
            return true;
        }
        let job = self.zip_job.take().expect("checked above");
        self.mem_after_job();
        let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
        let (text, warn) = match result {
            Some(Ok(rep)) => {
                for (p, why) in &rep.errors {
                    self.log_with(
                        ndir_log::LogKind::Error,
                        format!("{}: {why}", p.display()),
                        None,
                        None,
                    );
                }
                for p in &rep.too_big {
                    self.log_with(
                        ndir_log::LogKind::Error,
                        trf("zip.tooBigItem", &[&p.display().to_string()]),
                        None,
                        None,
                    );
                }
                zip_summary(&rep, &job.dest)
            }
            Some(Err(e)) => (trf("zip.failed", &[&e.to_string()]), true),
            None => (tr("zip.failed"), true),
        };
        if self.progress_win.is_active() {
            let ms = self.settings.int("transfer.close_ms").max(0) as u64;
            let now = self.started.elapsed().as_millis() as u64;
            self.progress_win.update(total, total, Vec::new(), 1, 1);
            self.progress_win.set_done(&text, ms, now);
        }
        self.dirsizes.clear();
        self.git_detail.clear();
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        if job.dest.is_file() {
            self.panels[self.active].select_path(&job.dest, &mut inv);
        }
        let a = self.active;
        self.toast_panel(
            a,
            if warn {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("ctx.compress"),
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
    fn dest_name_rules() {
        let base = std::env::temp_dir().join(format!("ndir-zipdest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("docs")).unwrap();
        std::fs::write(base.join("a.txt"), b"x").unwrap();
        assert_eq!(
            zip_dest(&[base.join("a.txt")]),
            Some(base.join("a.zip")),
            "파일 하나 = 확장자 제외 이름"
        );
        assert_eq!(
            zip_dest(&[base.join("docs")]),
            Some(base.join("docs.zip")),
            "폴더 하나 = 폴더 이름 그대로(점 포함 이름 보존)"
        );
        let many = zip_dest(&[base.join("a.txt"), base.join("docs")]).unwrap();
        assert_eq!(
            many.file_name().unwrap().to_string_lossy(),
            format!("{}.zip", base.file_name().unwrap().to_string_lossy()),
            "여럿 = 부모 폴더 이름"
        );
        std::fs::write(base.join("a.zip"), b"").unwrap();
        assert_eq!(
            zip_dest(&[base.join("a.txt")]),
            Some(base.join("a (2).zip")),
            "충돌 = 순번"
        );
        assert!(zip_dest(&[]).is_none());
        let _ = std::fs::remove_dir_all(&base);
    }
}
