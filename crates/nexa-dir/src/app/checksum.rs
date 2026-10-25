//! App — 체크섬 계산(T-167 · NEW-038 · 사용자 10-05 · 10-06 개편): 파일 **여러 개**를 작업 스레드 1개가 차례로 계산(파일마다
//! 알고리즘들을 한 번 읽기) · **작업 슬롯 1개**(진행 중 두 번째 요청 = 안내 · 거부) · 스레드 우선순위 낮춤(UI · 다른 작업에 양보) ·
//! 합계가 설정 `hash.auto_limit_mb`를 넘으면 자동 시작하지 않고 창의 [계산]을 기다린다(0 = 언제나 수동). 폴링 = `OPS_POLL_MS` 틱.

use crate::*;
use ndir_ops::hash::{self, Algo};
use std::path::Path;
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
        // 검증 모드면 창이 가진 알고리즘 하나로(검증 파일의 것) · 아니면 설정.
        let algos = match self.hash_win.verify_counts() {
            Some(_) => self
                .hash_win
                .strongest()
                .map(|a| vec![a])
                .unwrap_or_default(),
            None => parse_algos(self.settings.get("hash.algos").unwrap_or("")),
        };
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
            // 결과를 다 넣은 **뒤** 끝 표시(Release) — 수거 쪽은 끝 표시를 먼저 읽고(Acquire) 결과를 비운다 → 마지막 결과를 잃지 않는다.
            sh.finished.store(true, Ordering::Release);
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
        // 끝 표시를 **먼저** 읽고(Acquire) 결과를 비운다 — 종전(비운 뒤 읽기)은 그 사이 워커가 마지막 결과를 넣고 끝 표시를 세우면
        // 그 결과를 영영 못 받았다(10-18 전수 흔들림 `checksum_window_computes_…` = 첫 파일 값 None).
        let finished = job.shared.finished.load(Ordering::Acquire);
        let ready: Vec<(usize, FileOutcome)> = job
            .shared
            .results
            .lock()
            .map(|mut r| r.drain(..).collect())
            .unwrap_or_default();
        for (i, out) in ready {
            self.hash_win.set_file_result(i, out);
        }
        if !finished {
            return true;
        }
        let job = self.hash_job.take().expect("checked above");
        self.mem_after_job();
        let canceled = job.shared.cancel.load(Ordering::Relaxed);
        self.hash_win.finish(canceled.then(|| tr("ops.canceled")));
        false
    }

    /// [저장 .sha256](T-167 · DR-32 ⑥): 가장 강한 알고리즘 값을 GNU 형식(`<16진>  <이름>`)으로 같은 폴더에 — 파일 하나 = `<이름>.<키>` ·
    /// 여럿 = `checksums.<키>` · 이미 있으면 덮어쓰기 확인.
    pub(crate) fn checksum_save(&mut self) {
        let Some(algo) = self.hash_win.strongest() else {
            return;
        };
        let rows = self.hash_win.save_rows();
        if rows.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = self
            .hash_win
            .files()
            .iter()
            .map(|f| f.path.clone())
            .collect();
        let Some(dest) = hash::sums_file_name(&paths, algo) else {
            return;
        };
        let text = hash::sums_text(&rows);
        if dest.exists() {
            let spec = crate::dlg_win::DlgSpec::confirm(
                tr("hash.title"),
                trf("hash.overwrite", &[&ndir_ops::leaf_name(&dest)]),
                tr("bulk.preset.overwrite"),
            );
            self.ask(spec, app::dialogs::DlgReply::HashSave { dest, text });
            return;
        }
        self.checksum_write(&dest, &text);
    }

    /// 검증 파일을 실제로 쓴다(저장 · 덮어쓰기 확인 뒤).
    pub(crate) fn checksum_write(&mut self, dest: &Path, text: &str) {
        let a = self.active;
        match std::fs::write(dest, text) {
            Ok(()) => {
                self.toast_panel(
                    a,
                    toast::ToastKind::Info,
                    tr("hash.title"),
                    trf("hash.saved", &[&ndir_ops::leaf_name(dest)]),
                );
                let mut inv = Invalidations::default();
                for p in &mut self.panels {
                    p.reopen(&mut inv);
                }
                self.redraw();
            }
            Err(e) => self.toast_panel(
                a,
                toast::ToastKind::Warn,
                tr("hash.title"),
                trf("hash.saveFailed", &[&e.to_string()]),
            ),
        }
    }

    /// `ctx.verify_checksum`(T-167): 검증 파일(GNU `<16진>  <이름>`)을 읽어 그 폴더의 파일들을 계산하고 줄별로 대조 — 결과 열(✓/✗) · 요약(일치 · 불일치 · 없음).
    pub(crate) fn checksum_verify(&mut self, sums: &Path) {
        if self.hash_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("hash.busy"), &mut inv);
            self.redraw();
            return;
        }
        let a = self.active;
        let text = match std::fs::read_to_string(sums) {
            Ok(t) => t,
            Err(e) => {
                self.toast_panel(
                    a,
                    toast::ToastKind::Warn,
                    tr("hash.title"),
                    trf("hash.failed", &[&e.to_string()]),
                );
                return;
            }
        };
        let entries = hash::parse_sums(&text);
        let Some(algo) = hash::sums_algo(sums, &entries) else {
            self.toast_panel(
                a,
                toast::ToastKind::Warn,
                tr("hash.title"),
                tr("hash.noEntries"),
            );
            return;
        };
        if entries.is_empty() {
            self.toast_panel(
                a,
                toast::ToastKind::Warn,
                tr("hash.title"),
                tr("hash.noEntries"),
            );
            return;
        }
        let base = sums.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        let mut files: Vec<(PathBuf, u64)> = Vec::new();
        let mut expected: Vec<String> = Vec::new();
        let mut missing = 0usize;
        for (hex, name) in &entries {
            let p = base.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
            match std::fs::metadata(&p) {
                Ok(md) if md.is_file() => {
                    files.push((p, md.len()));
                    expected.push(hex.clone());
                }
                _ => missing += 1,
            }
        }
        let total: u64 = files.iter().map(|(_, s)| *s).sum();
        let auto = auto_start(total, self.settings.int("hash.auto_limit_mb"));
        self.hash_win.start(&files, &[algo], auto);
        self.hash_win.set_verify(algo, expected, missing);
        if auto && !files.is_empty() {
            self.hash_begin(files.into_iter().map(|(p, _)| p).collect(), vec![algo]);
        } else if files.is_empty() {
            self.hash_win.finish(None);
        }
        self.open_hash = true;
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
