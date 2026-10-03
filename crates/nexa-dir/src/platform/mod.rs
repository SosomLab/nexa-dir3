//! 플랫폼 층 — **OS 분기는 이 폴더 안에만**(DR-5 · docs/port/40 SKEL-404 · port/44 CI-103 · ADR-0001).
//!
//! 포트(trait) 9종: [`Shell`](기본 셸 탐지) · [`Pty`] · [`ContextMenuProvider`] · [`Trash`] · [`FileClipboard`] · [`DragSource`] · [`Watcher`] ·
//! [`Opener`] · [`Disk`](드라이브 용량 — PANEL-044 때문에 DR-5 8종에 더함). 기기 ID는 nexa-license 내장.
//! 운영 = [`Platform::native`](`windows.rs`/`macos.rs`/`linux.rs` + 공용 폴백) · 시험 = [`Platform::fake`](`fake.rs` · 기록·주입).
//! 규칙: 미지원·다른 OS = `Err(PlatformError::Unsupported)`/`None` → 호출부가 사유 문구를 한 번 안내(docs/18 §7) · 호출부는 OS를 모른다 ·
//! OS 분기 **판정**은 순수 함수([`pick_shell`] 등 · MC/DC 시험).

// 포트 계약은 T-51~53·M5(터미널)·M6(파일 작업)이 차례로 소비한다 — 그때까지 미사용 경고를 끈다(계약 전체를 한 번에 세우는 것이 DR-5).
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

pub(crate) mod fake;
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(target_os = "linux")]
mod linuxwatch;
#[cfg(target_os = "macos")]
mod macclip;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macwatch;
#[cfg(unix)]
mod unixpty;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod winpty;
#[cfg(windows)]
mod winrecycle;
#[cfg(windows)]
mod winshell;
#[cfg(windows)]
mod wintemplates;
#[cfg(windows)]
mod winwatch;

/// OS 자원(셸 메뉴 COM · 폴더 감시 스레드 · PTY · 자가 점검 전체)을 쓰는 **시험 직렬화 뮤텍스** — `cargo test` 병렬 실행에서 두 개가 겹치면
/// 교착했다(10-03 로컬 실증 · 운영은 Platform 1개라 무관). 해당 시험은 첫 줄에서 `let _g = os_test_guard();`.
#[cfg(test)]
pub(crate) static OS_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn os_test_guard() -> std::sync::MutexGuard<'static, ()> {
    OS_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// 포트 호출 실패 — `Unsupported`(이 OS/빌드에 구현 없음 · 안내만) · `Failed`(구현이 있으나 실패 · 사유).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlatformError {
    Unsupported(&'static str),
    Failed(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::Unsupported(what) => write!(f, "unsupported: {what}"),
            PlatformError::Failed(why) => f.write_str(why),
        }
    }
}

/// 셸 하나(실행 파일 · 인자 · 표시 이름).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub label: String,
}

/// 셸 컨텍스트 메뉴 항목(OS 셸이 주는 것 — id · 라벨 · 활성 · verb(가로채기 판정) · 서브메뉴 · 구분자).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ShellMenuItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    /// canonical verb(`GetCommandString(GCS_VERBW)` · 없으면 빈 문자열) — cut/copy/paste/delete/rename/copyaspath는 앱이 가로챈다.
    pub verb: String,
    pub children: Vec<ShellMenuItem>,
    pub separator: bool,
    /// 항목 아이콘(셸 확장이 준 비트맵 · dir2 SHELL-011) — 없으면 `None`(아이콘 칸은 메뉴가 하나라도 있으면 전 행에 예약).
    pub icon: Option<ShellIcon>,
}

/// 셸 메뉴 항목 아이콘 — straight RGBA(`w*h*4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellIcon {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// 셸 메뉴의 대상(선행 구축 · 재사용 판정의 열쇠 — dir2 `MenuReq::same_menu`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuTarget {
    /// 선택 항목들.
    Rows(Vec<PathBuf>),
    /// 폴더 배경.
    Bg(PathBuf),
}

/// 셸 메뉴 비동기 통지(`ContextMenuProvider::poll`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuEvent {
    /// 구축 끝 — 항목 목록(실패 = 빈 목록).
    Items {
        target: MenuTarget,
        items: Vec<ShellMenuItem>,
    },
    /// 실행 끝 — 배경 메뉴가 항목을 정확히 1개 만들었으면 그 경로.
    Invoked {
        target: MenuTarget,
        result: Result<Option<PathBuf>, String>,
    },
}

/// PTY 세션(M5 터미널이 쓴다) — 읽기/쓰기/크기/종료.
pub(crate) trait PtySession {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    /// 읽을 것이 없으면 `Ok(0)`(비차단).
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()>;
    fn kill(&mut self);
    /// 프로세스가 아직 살아 있는가(종료 감지 → 안내 · 재시작). 기본 = 항상 참(가짜).
    fn alive(&self) -> bool {
        true
    }
}

/// 기본 셸 탐지(Windows `pwsh` → `powershell` → `cmd` · Unix `$SHELL` → `/bin/sh`).
pub(crate) trait Shell {
    fn candidates(&self) -> Vec<ShellSpec>;
    fn default_shell(&self) -> Option<ShellSpec> {
        self.candidates().into_iter().next()
    }
}

pub(crate) trait Pty {
    fn spawn(
        &self,
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn PtySession>, PlatformError>;
}

pub(crate) trait ContextMenuProvider {
    /// 메뉴 소유 창(셸 확장의 대화상자 부모) — 창이 생기면 호스트가 한 번 알린다. 기본 = 무시.
    fn set_owner(&self, _hwnd: isize) {}
    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError>;
    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError>;
    /// 폴더 **배경** 메뉴(dir2 SHELL-009 · 보기·새로 만들기·붙여넣기·속성). 기본 = 없음(자체 항목만).
    fn bg_items(&self, _dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        Ok(Vec::new())
    }
    /// 배경 항목 실행 — 셸이 그 폴더에 항목을 **정확히 1개** 만들었으면(새로 만들기) 그 경로(호스트가 선택 + 이름 바꾸기).
    fn invoke_bg(&self, id: &str, _dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        Err(PlatformError::Failed(format!(
            "not a background item: {id}"
        )))
    }

    // ── 비차단 경로(dir2 X-61 "우클릭 가속" · SHELL-014/015) — 기본 구현 = 동기 경로 그대로(가짜 · 다른 OS) ──

    /// **선행 구축**: 선택이 머물면 호스트가 미리 시킨다(비차단 · 결과는 구현 쪽 캐시). 기본 = 아무것도 안 함.
    fn prepare(&self, _target: &MenuTarget) {}
    /// **비차단 조회**: `Some` = 지금 줄 수 있음 · `None` = 구축 중(끝나면 `poll`이 `Items`를 준다 — 호스트는 자체 항목만 먼저 띄운다).
    /// 기본 = 동기 `items`/`bg_items`.
    fn try_items(&self, target: &MenuTarget) -> Option<Vec<ShellMenuItem>> {
        Some(
            match target {
                MenuTarget::Rows(paths) => self.items(paths),
                MenuTarget::Bg(dir) => self.bg_items(dir),
            }
            .unwrap_or_default(),
        )
    }
    /// **비차단 실행**: true = 접수(결과는 `poll`의 `Invoked`) · false = 미지원(호출자가 동기 `invoke`/`invoke_bg`). 기본 = false.
    fn invoke_async(&self, _id: &str, _target: &MenuTarget) -> bool {
        false
    }
    /// 비동기 통지 1건(없으면 `None`). 호스트가 틱에서 비울 때까지 부른다.
    fn poll(&self) -> Option<MenuEvent> {
        None
    }
    /// 준비분 폐기(폴더 내용이 바뀜). 기본 = 무시.
    fn invalidate(&self) {}
    /// 진행 중인 구축/실행이 있는가(호스트가 틱을 유지). 기본 = 없음.
    fn busy(&self) -> bool {
        false
    }
}

pub(crate) trait Trash {
    /// 휴지통으로 — 옮긴 개수.
    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError>;
    /// 휴지통에서 **원래 경로**로 복원(삭제 undo · dir2 SHELL-049) — 복원한 개수(없으면 0). 기본 = 미지원.
    fn restore(&self, _original: &[PathBuf]) -> Result<usize, PlatformError> {
        Err(PlatformError::Unsupported("trash.restore"))
    }
}

pub(crate) trait FileClipboard {
    /// (경로 목록, 잘라내기 여부) · 없으면 `None`.
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)>;
    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError>;
}

pub(crate) trait DragSource {
    /// OS 드래그 시작(끝날 때까지 막힘) — 결과 = 옮김/복사/취소.
    fn begin_drag(&self, paths: &[PathBuf]) -> Result<DragOutcome, PlatformError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragOutcome {
    Cancelled,
    Copied,
    Moved,
}

/// 폴더 변경 감시 — `watch`로 대상 집합을 바꾸고 `poll`로 바뀐 폴더를 거둔다(자체 스레드·타이머 없음 · 호스트 틱이 부른다).
pub(crate) trait Watcher {
    fn watch(&mut self, dirs: &[PathBuf]);
    fn poll(&mut self) -> Vec<PathBuf>;
    /// 호스트가 `poll`을 부르는 간격(ms) — 폴링 프로브 1000 · OS 통지 250(디바운스 역할).
    fn poll_interval_ms(&self) -> u64 {
        1000
    }
}

pub(crate) trait Opener {
    /// 연결 프로그램으로 열기.
    fn open(&self, path: &Path) -> Result<(), PlatformError>;
    /// 파일 관리자에서 보기(선택 상태).
    fn reveal(&self, path: &Path) -> Result<(), PlatformError>;
}

pub(crate) trait Disk {
    /// (전체, 여유) 바이트 · 모르면 `None`.
    fn space(&self, root: &Path) -> Option<(u64, u64)>;
}

/// "새로 만들기 ▸" 템플릿 하나(SHELL-008 · dir2 ShellNew 전체 목록의 3-OS 대응): 라벨(OS 종류 이름 또는 파일 이름) · 확장자(점 없음) · 원천.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewTemplate {
    pub label: String,
    pub ext: String,
    pub source: TemplateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TemplateSource {
    /// 빈 파일.
    Empty,
    /// 이 파일을 복사.
    Copy(PathBuf),
    /// 이 바이트를 쓴다.
    Data(Vec<u8>),
}

/// 템플릿 목록 포트 — Windows = 레지스트리 ShellNew + 사용자 폴더 · macOS/Linux = 사용자 폴더(`<설정>/templates` · Linux XDG `TEMPLATES`).
pub(crate) trait Templates {
    fn list(&self) -> Vec<NewTemplate>;
}

/// 사용자 템플릿 폴더들의 파일 = 템플릿(라벨 = 파일 이름 줄기 · 숨김 파일 제외 · 이름순).
pub(crate) struct UserTemplates {
    dirs: Vec<PathBuf>,
}

impl UserTemplates {
    pub(crate) fn new(dirs: Vec<PathBuf>) -> Self {
        UserTemplates { dirs }
    }
}

impl Templates for UserTemplates {
    fn list(&self) -> Vec<NewTemplate> {
        let mut out: Vec<NewTemplate> = Vec::new();
        for d in &self.dirs {
            let Ok(rd) = std::fs::read_dir(d) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                if !p.is_file() || name.starts_with('.') {
                    continue;
                }
                let label = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| name.clone());
                let ext = p
                    .extension()
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                if out.iter().any(|t| t.label == label && t.ext == ext) {
                    continue;
                }
                out.push(NewTemplate {
                    label,
                    ext,
                    source: TemplateSource::Copy(p),
                });
            }
        }
        out.sort_by_key(|t| t.label.to_lowercase());
        out
    }
}

/// `~/.config/user-dirs.dirs`의 `XDG_TEMPLATES_DIR="$HOME/Templates"` 해석(순수 함수 · 없으면 None).
pub(crate) fn xdg_templates_dir(user_dirs: &str, home: &Path) -> Option<PathBuf> {
    let line = user_dirs
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_TEMPLATES_DIR="))?;
    let v = line["XDG_TEMPLATES_DIR=".len()..].trim().trim_matches('"');
    let v = v
        .strip_prefix("$HOME/")
        .map_or_else(|| PathBuf::from(v), |rest| home.join(rest));
    (!v.as_os_str().is_empty()).then_some(v)
}

/// 이 OS의 템플릿 폴더들(존재 여부는 묻지 않는다 — 목록을 읽을 때 없으면 건너뜀).
pub(crate) fn os_template_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Some(c) = ndir_settings::config_dir() {
        v.push(c.join("templates"));
    }
    if cfg!(all(unix, not(target_os = "macos"))) {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let cfg = std::env::var_os("XDG_CONFIG_HOME")
                .map_or_else(|| home.join(".config"), PathBuf::from)
                .join("user-dirs.dirs");
            let dir = std::fs::read_to_string(&cfg)
                .ok()
                .and_then(|t| xdg_templates_dir(&t, &home))
                .unwrap_or_else(|| home.join("Templates"));
            v.push(dir);
        }
    }
    v
}

/// OS 모듈이 주는 포트 다섯(셸 · 열기 · 용량 · 휴지통 · 파일 클립보드).
type OsPorts = (
    Box<dyn Shell>,
    Box<dyn Opener>,
    Box<dyn Disk>,
    Rc<dyn Trash>,
    Box<dyn FileClipboard>,
);

/// 포트 묶음 — 호출부는 이 안의 trait 객체만 본다.
pub(crate) struct Platform {
    pub shell: Box<dyn Shell>,
    pub pty: Box<dyn Pty>,
    pub ctxmenu: Box<dyn ContextMenuProvider>,
    /// 휴지통 — `Rc`(히스토리 CopyBatchOp의 삭제 주입이 공유 · M6).
    pub trash: Rc<dyn Trash>,
    pub clipboard: Box<dyn FileClipboard>,
    pub drag: Box<dyn DragSource>,
    pub watcher: Box<dyn Watcher>,
    pub opener: Box<dyn Opener>,
    pub disk: Box<dyn Disk>,
    /// "새로 만들기" 템플릿(SHELL-008).
    pub templates: Box<dyn Templates>,
    /// 가짜 플랫폼의 기록(시험) · 운영은 `None`.
    pub log: Option<Rc<RefCell<fake::FakeLog>>>,
}

impl std::fmt::Debug for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Platform")
            .field("fake", &self.log.is_some())
            .finish()
    }
}

/// 아직 구현이 없는 포트(T-51~53까지의 자리) — 전부 `Unsupported`.
struct Unsupported;

impl Pty for Unsupported {
    fn spawn(
        &self,
        _shell: &ShellSpec,
        _cwd: &Path,
        _cols: u16,
        _rows: u16,
    ) -> Result<Box<dyn PtySession>, PlatformError> {
        Err(PlatformError::Unsupported("pty"))
    }
}

impl ContextMenuProvider for Unsupported {
    fn items(&self, _paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        Err(PlatformError::Unsupported("shell context menu"))
    }
    fn invoke(&self, _id: &str, _paths: &[PathBuf]) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("shell context menu"))
    }
}

impl Trash for Unsupported {
    fn trash(&self, _paths: &[PathBuf]) -> Result<usize, PlatformError> {
        Err(PlatformError::Unsupported("trash"))
    }
}

impl FileClipboard for Unsupported {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        None
    }
    fn write_files(&self, _paths: &[PathBuf], _cut: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("file clipboard"))
    }
}

impl DragSource for Unsupported {
    fn begin_drag(&self, _paths: &[PathBuf]) -> Result<DragOutcome, PlatformError> {
        Err(PlatformError::Unsupported("drag source"))
    }
}

impl Templates for Unsupported {
    fn list(&self) -> Vec<NewTemplate> {
        Vec::new()
    }
}

impl Disk for Unsupported {
    fn space(&self, _root: &Path) -> Option<(u64, u64)> {
        None
    }
}

/// 후보 가운데 **존재하는 첫 것**(순수 판정 — `exists`를 주입해 시험). 하나도 없으면 `None`.
pub(crate) fn pick_shell(
    candidates: &[ShellSpec],
    exists: &dyn Fn(&Path) -> bool,
) -> Option<ShellSpec> {
    candidates.iter().find(|c| exists(&c.program)).cloned()
}

/// PATH에서 실행 파일 찾기(Windows는 `PATHEXT` 없이 주어진 이름 그대로 + `.exe`).
pub(crate) fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
        if cfg!(windows) && !name.to_ascii_lowercase().ends_with(".exe") {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// 폴링 감시자(공용 폴백 — 3-OS 동일 · 외부 crate 0): 폴더의 수정 시각 + 항목 수 스냅숏을 비교한다. OS 통지(T-51~53)는 이 위에 덧댄다.
#[derive(Debug, Default)]
pub(crate) struct PollWatcher {
    dirs: Vec<PathBuf>,
    snap: HashMap<PathBuf, (Option<SystemTime>, usize)>,
}

impl PollWatcher {
    fn snapshot(dir: &Path) -> (Option<SystemTime>, usize) {
        let mtime = std::fs::metadata(dir).and_then(|m| m.modified()).ok();
        let count = std::fs::read_dir(dir).map_or(0, |rd| rd.count());
        (mtime, count)
    }
}

impl Watcher for PollWatcher {
    fn watch(&mut self, dirs: &[PathBuf]) {
        if self.dirs == dirs {
            return;
        }
        self.dirs = dirs.to_vec();
        self.snap = dirs
            .iter()
            .map(|d| (d.clone(), Self::snapshot(d)))
            .collect();
    }

    fn poll(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for d in &self.dirs {
            let now = Self::snapshot(d);
            if self.snap.get(d) != Some(&now) {
                self.snap.insert(d.clone(), now);
                changed.push(d.clone());
            }
        }
        changed
    }
}

/// 공용 셸 후보(OS 모듈이 이름을 주면 PATH에서 찾는다).
fn shell_from_names(names: &[(&str, &[&str], &str)]) -> Vec<ShellSpec> {
    names
        .iter()
        .filter_map(|(name, args, label)| {
            let program = if Path::new(name).is_absolute() {
                Path::new(name).is_file().then(|| PathBuf::from(name))
            } else {
                find_in_path(name)
            }?;
            Some(ShellSpec {
                program,
                args: args.iter().map(|a| (*a).to_string()).collect(),
                label: (*label).to_string(),
            })
        })
        .collect()
}

/// 프로세스 실행으로 "열기"/"보기"를 하는 공용 열기(각 OS가 명령 이름만 준다).
struct CommandOpener {
    open: Vec<String>,
    reveal: Vec<String>,
}

impl CommandOpener {
    fn run(argv: &[String], path: &Path) -> Result<(), PlatformError> {
        let Some((prog, rest)) = argv.split_first() else {
            return Err(PlatformError::Unsupported("opener"));
        };
        let mut cmd = std::process::Command::new(prog);
        let mut has_path = false;
        for a in rest {
            if a == "{path}" {
                cmd.arg(path);
                has_path = true;
            } else {
                cmd.arg(a);
            }
        }
        if !has_path {
            cmd.arg(path);
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| PlatformError::Failed(format!("{prog}: {e}")))
    }
}

impl Opener for CommandOpener {
    fn open(&self, path: &Path) -> Result<(), PlatformError> {
        Self::run(&self.open, path)
    }
    fn reveal(&self, path: &Path) -> Result<(), PlatformError> {
        Self::run(&self.reveal, path)
    }
}

impl Platform {
    /// 운영 플랫폼(이 OS 모듈 + 공용 폴백).
    pub(crate) fn native() -> Platform {
        #[cfg(windows)]
        let (shell, opener, disk, trash, clipboard): OsPorts = (
            Box::new(windows::NativeShell),
            Box::new(windows::opener()),
            Box::new(windows::NativeDisk),
            Rc::new(windows::NativeTrash),
            Box::new(windows::NativeFileClipboard),
        );
        #[cfg(target_os = "macos")]
        let (shell, opener, disk, trash, clipboard): OsPorts = (
            Box::new(macos::NativeShell),
            Box::new(macos::opener()),
            Box::new(macos::NativeDisk),
            Rc::new(macos::SystemTrash::new()),
            Box::new(macclip::PasteboardFiles::new()),
        );
        #[cfg(all(unix, not(target_os = "macos")))]
        let (shell, opener, disk, trash, clipboard): OsPorts = (
            Box::new(linux::NativeShell),
            Box::new(linux::opener()),
            Box::new(linux::NativeDisk),
            Rc::new(linux::FreedesktopTrash::new()),
            Box::new(linux::X11Files),
        );
        #[cfg(windows)]
        let pty: Box<dyn Pty> = Box::new(winpty::ConPty);
        #[cfg(unix)]
        let pty: Box<dyn Pty> = Box::new(unixpty::ForkPty);
        #[cfg(windows)]
        let ctxmenu: Box<dyn ContextMenuProvider> = Box::new(winshell::NativeShellMenu::new());
        #[cfg(not(windows))]
        let ctxmenu: Box<dyn ContextMenuProvider> = Box::new(Unsupported);
        #[cfg(windows)]
        let watcher: Box<dyn Watcher> = Box::new(winwatch::NativeWatcher::new());
        #[cfg(target_os = "linux")]
        let watcher: Box<dyn Watcher> = Box::new(linuxwatch::InotifyWatcher::new());
        #[cfg(target_os = "macos")]
        let watcher: Box<dyn Watcher> = Box::new(macwatch::KqueueWatcher::new());
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        let watcher: Box<dyn Watcher> = Box::new(PollWatcher::default());
        #[cfg(windows)]
        let templates: Box<dyn Templates> = Box::new(wintemplates::ShellNewTemplates::new());
        #[cfg(not(windows))]
        let templates: Box<dyn Templates> = Box::new(UserTemplates::new(os_template_dirs()));
        Platform {
            shell,
            pty,
            ctxmenu,
            trash,
            clipboard,
            drag: Box::new(Unsupported),
            watcher,
            opener,
            disk,
            templates,
            log: None,
        }
    }

    /// 가짜 플랫폼(시험 · T3) — 모든 호출을 기록하고 주입한 값을 돌려준다.
    pub(crate) fn fake() -> Platform {
        fake::platform()
    }
}

/// 시스템 "휠 한 번에 스크롤할 줄 수"(Windows `SPI_GETWHEELSCROLLLINES` · 페이지 단위 = -1 → 호출자가 상한으로 해석) —
/// 다른 OS는 `None`(nexa-ctl 기본 3줄 유지 · dir2 `sync_wheel_lines` win.rs:6325).
pub(crate) fn wheel_lines() -> Option<i32> {
    #[cfg(windows)]
    {
        use ::windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        };
        let mut n: u32 = 3;
        // SAFETY: 출력 버퍼는 u32 하나(SPI 규약) · 플래그 0.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWHEELSCROLLLINES,
                0,
                Some(std::ptr::from_mut(&mut n).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        ok.is_ok().then(|| i32::try_from(n).unwrap_or(-1))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(p: &str) -> ShellSpec {
        ShellSpec {
            program: PathBuf::from(p),
            args: vec![],
            label: p.to_string(),
        }
    }

    /// XDG 템플릿 폴더 해석 · 사용자 템플릿 폴더 목록(숨김 제외 · 이름순 · 라벨/확장자).
    #[test]
    fn xdg_templates_and_user_templates() {
        let home = Path::new("/home/u");
        assert_eq!(
            xdg_templates_dir(
                "XDG_DESKTOP_DIR=\"$HOME/Desktop\"\nXDG_TEMPLATES_DIR=\"$HOME/Templates\"\n",
                home
            ),
            Some(PathBuf::from("/home/u/Templates"))
        );
        assert_eq!(
            xdg_templates_dir("XDG_TEMPLATES_DIR=\"/srv/tpl\"", home),
            Some(PathBuf::from("/srv/tpl"))
        );
        assert_eq!(xdg_templates_dir("# nothing", home), None);
        let d = std::env::temp_dir().join(format!("ndir-tpl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Letter.docx"), b"x").unwrap();
        std::fs::write(d.join(".hidden.txt"), b"x").unwrap();
        std::fs::write(d.join("a-note.md"), b"x").unwrap();
        let t = UserTemplates::new(vec![d.clone(), d.join("missing")]).list();
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].label.as_str(), t[0].ext.as_str()), ("a-note", "md"));
        assert_eq!(t[1].label, "Letter");
        assert!(matches!(&t[1].source, TemplateSource::Copy(p) if p == &d.join("Letter.docx")));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// 순수 판정: 존재하는 첫 후보 · 전부 없으면 None · 순서 보존.
    #[test]
    fn pick_shell_prefers_first_existing() {
        let c = [spec("pwsh"), spec("powershell"), spec("cmd")];
        let only_cmd = |p: &Path| p == Path::new("cmd");
        assert_eq!(
            pick_shell(&c, &only_cmd).map(|s| s.label),
            Some("cmd".into())
        );
        let all = |_: &Path| true;
        assert_eq!(pick_shell(&c, &all).map(|s| s.label), Some("pwsh".into()));
        let none = |_: &Path| false;
        assert!(pick_shell(&c, &none).is_none());
    }

    /// 폴링 감시자: 파일이 생기면 그 폴더만 보고 · 같은 집합 재지정은 스냅숏을 보존 · 집합이 바뀌면 다시.
    #[test]
    fn poll_watcher_reports_changed_dirs_only() {
        let base = std::env::temp_dir().join(format!("ndir-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let mut w = PollWatcher::default();
        w.watch(&[a.clone(), b.clone()]);
        assert!(w.poll().is_empty(), "변화 없음");
        std::fs::write(a.join("x.txt"), b"1").unwrap();
        assert_eq!(w.poll(), vec![a.clone()]);
        assert!(w.poll().is_empty(), "한 번 보고하면 끝");
        w.watch(&[a, b.clone()]);
        std::fs::write(b.join("y.txt"), b"1").unwrap();
        assert_eq!(w.poll(), vec![b]);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 운영 플랫폼: 이 OS의 기본 셸이 **있고 존재하는 파일**이다 · 미구현 포트는 Unsupported(패닉 아님).
    #[test]
    fn native_has_a_shell_and_unsupported_ports_say_so() {
        let p = Platform::native();
        let sh = p.shell.default_shell().expect("default shell");
        assert!(sh.program.is_file(), "{:?}", sh.program);
        assert_eq!(
            p.trash.trash(&[]),
            Ok(0),
            "빈 목록 = 0(3-OS 휴지통 구현 존재)"
        );
        if cfg!(windows) {
            assert_eq!(
                p.ctxmenu.items(&[]).map(|v| v.len()),
                Ok(0),
                "Windows = 셸 메뉴 포트(빈 입력 = 빈 목록)"
            );
        } else {
            assert!(matches!(
                p.ctxmenu.items(&[]),
                Err(PlatformError::Unsupported(_))
            ));
        }
        assert!(p.log.is_none());
        let cwd = std::env::current_dir().unwrap();
        let (total, free) = p.disk.space(&cwd).expect("drive space (3-OS)");
        assert!(total >= free && total > 0);
    }
}
