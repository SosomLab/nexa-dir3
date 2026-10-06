//! App — 라이선스(T-80 · docs/port/42 LIC-158~164): About 대화상자(라이선스 줄 + [라이선스…]) · 라이선스 창 보기/설치/제거/요청 코드 ·
//! 파일 창 용도 · 덤프(`license.dump`). 상태의 단일 원천 = `App.licensing`(`ndir_license::Licensing`) — 게이트는 0(dir2 정책 · Feature 변형 없음).
//!
//! 진입점 결정(LIC-158): ⓐ Help ▸ 라이선스… ⓑ About [라이선스…] — 상태줄 배지(ⓒ)는 dir2 상태줄 배치를 바꾸므로 보류.
//! CLI 결정(LIC-162): GUI만(테스트 훅 = 기동 명령 `license.install:<파일>`).

use crate::app::dialogs::DlgReply;
use crate::dlg_win::DlgSpec;
use crate::license_win::LicView;
use crate::*;
use ndir_license::{InstallError, LicenseState, Licensing, RequestMeta, Tier};
use nexa_dlg::PickerMode;
use std::path::Path;

/// 파일 창의 용도(확정 경로를 어디로 보내나).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FilePurpose {
    /// 라이선스 파일 → `license_install`.
    License,
    /// 설정 창의 폴더 찾아보기 → 설정 키에 저장.
    Setting(String),
    /// 플러그인 설치(`*.wasm` → 사용자 플러그인 폴더 · T-63).
    Plugin,
    /// 로그를 파일로 저장(로그 창 메뉴 · T-92 · 전 기능 무료라 게이트 없음 · DR-4).
    LogExport,
}

impl App {
    /// 시험은 픽스처 폴더 아래(실제 사용자 폴더를 건드리지 않는다 · LIC-164 ④) · 운영은 기본 자리(사용자 → 기기 공용).
    pub(crate) fn licensing_for(start: Option<&Path>) -> Licensing {
        #[cfg(test)]
        {
            if let Some(d) = start {
                return Licensing::open(vec![d.join(ndir_license::LICENSE_SUBDIR)]);
            }
        }
        let _ = start;
        Licensing::open_default()
    }

    /// 배지 글(LIC-103): `Free · non-commercial use only` / `Pro · ACME` / `⚠ …`.
    pub(crate) fn license_badge(&self) -> String {
        Self::license_badge_of(&self.licensing)
    }

    pub(crate) fn license_badge_of(lic: &Licensing) -> String {
        match lic.state() {
            LicenseState::Free => tr("license.st.free"),
            LicenseState::Licensed(l) => {
                let tier = match Tier::parse(&l.tier) {
                    Tier::Trial => tr("license.tier.trial"),
                    Tier::Org => tr("license.tier.org"),
                    Tier::Pro | Tier::Free => tr("license.tier.pro"),
                };
                trf("license.st.licensed", &[&tier, &l.licensee])
            }
            LicenseState::Invalid(i) => trf(
                "license.st.invalid",
                &[&format!("{i:?}").to_ascii_lowercase()],
            ),
            LicenseState::Outdated(_) => tr("license.st.outdated"),
            LicenseState::Expired(l) => trf("license.st.expired", &[&l.expires]),
        }
    }

    /// 라이선스 창 보기(표 · 요청 코드) — 그릴 때마다 만든다(값 몇 줄).
    pub(crate) fn license_view(&self) -> LicView {
        let st = self.licensing.state();
        let warn = !matches!(st, LicenseState::Licensed(_));
        let dash = || "-".to_string();
        let mut rows: Vec<(String, String)> = vec![
            (tr("license.lbl.state"), st.name().to_string()),
            (
                tr("license.lbl.file"),
                self.licensing
                    .path()
                    .map_or_else(dash, |p| p.display().to_string()),
            ),
        ];
        if let Some(l) = st.license() {
            let feats = if l.features.contains("*") {
                "*".to_string()
            } else {
                l.features.iter().cloned().collect::<Vec<_>>().join(", ")
            };
            rows.push((tr("license.lbl.id"), l.id.clone()));
            rows.push((tr("license.lbl.licensee"), l.licensee.clone()));
            rows.push((tr("license.lbl.kind"), format!("{} / {}", l.kind, l.tier)));
            rows.push((tr("license.lbl.features"), feats));
            rows.push((tr("license.lbl.issued"), l.issued.clone()));
            rows.push((tr("license.lbl.until"), l.updates_until.clone()));
            rows.push((
                tr("license.lbl.expires"),
                if l.expires.is_empty() {
                    dash()
                } else {
                    l.expires.clone()
                },
            ));
            rows.push((
                tr("license.lbl.maxMajor"),
                l.max_major.map_or_else(dash, |m| format!("{m}.x")),
            ));
            if !l.max_version.is_empty() {
                rows.push((tr("license.lbl.maxVersion"), l.max_version.clone()));
            }
        }
        rows.push((
            tr("license.lbl.version"),
            ndir_license::PRODUCT.version.to_string(),
        ));
        rows.push((
            tr("license.lbl.build"),
            ndir_license::PRODUCT.build_date.to_string(),
        ));
        rows.push((
            tr("license.lbl.machine"),
            Licensing::machine_code().unwrap_or_else(dash),
        ));
        rows.push((
            tr("license.lbl.installTo"),
            self.licensing
                .primary_path()
                .map_or_else(dash, |p| p.display().to_string()),
        ));
        LicView {
            state: self.license_badge(),
            warn,
            rows,
            request: Licensing::request_code(&RequestMeta::default()),
            contact: ndir_license::LICENSE_CONTACT.to_string(),
        }
    }

    /// 결과 한 줄 = 상태줄 + 창 안내 + 토스트(창이 없을 때도 보인다).
    fn license_note(&mut self, note: String, warn: bool) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&note, &mut inv);
        self.toasts.push(
            if warn {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("license.title"),
            note.clone(),
        );
        self.license_win.set_note(note, warn);
        self.redraw();
    }

    /// 파일 창/기동 명령 → 설치(검증 통과만 씀 · 원본 보존) → 안내 + 배지 갱신.
    pub(crate) fn license_install(&mut self, path: &Path) {
        let (note, warn) = match self.licensing.install(path) {
            Ok(l) => (trf("license.note.installed", &[&l.id]), false),
            Err(InstallError::Rejected(s)) => (trf("license.note.rejected", &[s.name()]), true),
            Err(InstallError::Io(e)) => (trf("license.note.io", &[&e.to_string()]), true),
        };
        self.license_note(note, warn);
    }

    pub(crate) fn license_remove(&mut self) {
        let (note, warn) = match self.licensing.remove() {
            Ok(true) => (tr("license.note.removed"), false),
            Ok(false) => (tr("license.note.nothing"), true),
            Err(e) => (trf("license.note.io", &[&e.to_string()]), true),
        };
        self.license_note(note, warn);
    }

    pub(crate) fn license_copy_request(&mut self, name: &str, email: &str) {
        let meta = RequestMeta {
            name: name.to_string(),
            email: email.to_string(),
        };
        let (note, warn) = match Licensing::request_code(&meta) {
            Some(code) if clipboard::write_text(&code) => (tr("license.note.copied"), false),
            Some(_) => (tr("license.note.copyFailed"), true),
            None => (tr("license.noMachine"), true),
        };
        self.license_win.set_note(note, warn);
    }

    /// `license.dump` — 배지 · 상태 · 창 열림 · 표 · 안내(LIC-115 · 창 없이도 같은 내용).
    pub(crate) fn license_dump(&self) -> String {
        let v = self.license_view();
        let mut s = format!(
            "badge={}\nstate={}\nopen={}\nrequest={}\n",
            v.state,
            self.licensing.state().name(),
            self.license_win.is_open(),
            v.request.as_deref().map_or(0, str::len)
        );
        for (k, val) in &v.rows {
            s.push_str(&format!("{k}={val}\n"));
        }
        if let Some((n, w)) = self.license_win.note() {
            s.push_str(&format!("note={}:{n}\n", if *w { "warn" } else { "ok" }));
        }
        s
    }

    /// About 줄들(dir2 X-26 ③ + LIC-116: 제품 · 버전 · 빌드일 · 시스템 · 라이선스 상태 · 조건 · 저작권).
    pub(crate) fn about_text(&self) -> String {
        [
            format!("Nexa Dir {}", env!("CARGO_PKG_VERSION")),
            tr("about.desc"),
            format!("{}: {}", tr("about.version"), env!("CARGO_PKG_VERSION")),
            format!(
                "{}: {}",
                tr("about.build"),
                ndir_license::PRODUCT.build_date
            ),
            format!(
                "{}: {} {}",
                tr("about.system"),
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
            format!("{}: {}", tr("about.state"), self.license_badge()),
            tr("about.license"),
            tr("about.terms"),
            tr("about.copyright"),
        ]
        .join("\n")
    }

    /// Help ▸ About = 대화상자(제목 · 줄들 · [라이선스…] [확인]) — 2 = 라이선스 창.
    pub(crate) fn about_ask(&mut self) {
        self.licensing.refresh();
        let spec = DlgSpec {
            title: tr("about.title"),
            text: self.about_text(),
            buttons: vec![(2, tr("about.btn.license")), (1, tr("about.ok"))],
            default: 1,
            cancel: 1,
            input: None,
        };
        self.ask(spec, DlgReply::About);
    }

    /// 파일 창 열기 요청(펌프 `open_requested_windows`가 연다).
    pub(crate) fn open_file_window(&mut self, purpose: FilePurpose) {
        self.file_purpose = Some(purpose);
        self.open_file = true;
    }

    /// 파일 창 사양(용도 → 모드 · 시작 폴더 · 필터).
    pub(crate) fn file_window_spec(
        &self,
    ) -> (PickerMode, Option<PathBuf>, Vec<nexa_dlg::FileFilter>) {
        match &self.file_purpose {
            Some(FilePurpose::Setting(key)) => (
                PickerMode::Folder,
                self.settings
                    .get(key)
                    .map(PathBuf::from)
                    .filter(|p| p.is_dir()),
                Vec::new(),
            ),
            Some(FilePurpose::Plugin) => (
                PickerMode::Open,
                Some(self.term_cwd(0)),
                file_win::plugin_filters(),
            ),
            Some(FilePurpose::LogExport) => (PickerMode::Save, Some(self.term_cwd(0)), Vec::new()),
            _ => (
                PickerMode::Open,
                Some(self.term_cwd(0)),
                file_win::license_filters(),
            ),
        }
    }

    /// 파일 창 확정 → 용도별 처리.
    pub(crate) fn file_confirmed(&mut self, path: PathBuf) {
        match self.file_purpose.take() {
            Some(FilePurpose::License) => self.license_install(&path),
            Some(FilePurpose::Plugin) => self.plugin_install(&path),
            Some(FilePurpose::LogExport) => {
                let text = self.log_win.export_text();
                let shown = path.display().to_string();
                match std::fs::write(&path, text) {
                    Ok(()) => self.log(ndir_log::LogKind::Info, trf("log.msg.saved", &[&shown])),
                    Err(e) => self.log(
                        ndir_log::LogKind::Error,
                        trf("log.msg.saveFailed", &[&shown, &e.to_string()]),
                    ),
                }
            }
            Some(FilePurpose::Setting(key)) => {
                let v = path.display().to_string();
                if self.settings.set(&key, &v).is_ok() {
                    let _ = self.settings.save();
                    self.after_setting_changed(&key);
                }
            }
            None => {}
        }
    }
}
