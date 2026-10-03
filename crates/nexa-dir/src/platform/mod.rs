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
#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unixpty;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod winpty;
#[cfg(windows)]
mod winshell;
#[cfg(windows)]
mod winwatch;

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
}

pub(crate) trait Trash {
    /// 휴지통으로 — 옮긴 개수.
    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError>;
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
            Rc::new(macos::HomeTrash::new()),
            Box::new(Unsupported),
        );
        #[cfg(all(unix, not(target_os = "macos")))]
        let (shell, opener, disk, trash, clipboard): OsPorts = (
            Box::new(linux::NativeShell),
            Box::new(linux::opener()),
            Box::new(linux::NativeDisk),
            Rc::new(linux::FreedesktopTrash::new()),
            Box::new(Unsupported),
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
        #[cfg(not(windows))]
        let watcher: Box<dyn Watcher> = Box::new(PollWatcher::default());
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
            log: None,
        }
    }

    /// 가짜 플랫폼(시험 · T3) — 모든 호출을 기록하고 주입한 값을 돌려준다.
    pub(crate) fn fake() -> Platform {
        fake::platform()
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
