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
pub(crate) mod procmem;
pub(crate) mod sysload;
#[cfg(unix)]
mod unixpty;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
mod windrag;
#[cfg(windows)]
mod windrop;
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
// 순수 해석이라 어느 OS에서나 컴파일·시험한다 — 쓰는 곳은 Linux 메뉴 공급자뿐.
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
pub(crate) mod xdgapps;

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

/// 창을 띄우지 않는 자식 프로세스(Windows = 콘솔 창 없음 `CREATE_NO_WINDOW` · 그 밖 = 그대로) — `git` 같은 명령줄 도구를
/// 조용히 돌릴 때(탭 상태바 Git 상태 · NEW-005).
pub(crate) fn quiet_command(program: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// 시스템 휴지통 호출 한 건의 판정(macOS `trashItemAtURL` — 순수 · T-135).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrashOutcome {
    /// 휴지통으로 갔다 — 휴지통 안 경로(모르면 `None` = 복원 기록 없음).
    Trashed(Option<PathBuf>),
    /// 옮겨지지 않았다(원본이 그대로 있다) — 폴백(직접 옮기기)을 시도해도 된다.
    Fallback,
}

/// `api` = 호출 결과(`Ok(결과 경로)` · `Err` = 오류를 돌려줌) · `still_exists` = 호출 뒤 원본이 아직 있는가.
/// 성공인데 결과 경로가 없을 수 있고(Apple 문서) 오류를 돌려주고도 실제로는 옮겨졌을 수 있다 — 어느 쪽이든 **원본이 없으면
/// 옮겨진 것**이다(종전 = 결과 경로가 없으면 실패로 보고 폴백 → 이미 없는 원본을 rename 해 ENOENT · CI macOS 2회).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn trash_outcome(api: Result<Option<PathBuf>, ()>, still_exists: bool) -> TrashOutcome {
    match (api, still_exists) {
        (Ok(Some(t)), _) => TrashOutcome::Trashed(Some(t)),
        (Ok(None) | Err(()), false) => TrashOutcome::Trashed(None),
        (Ok(None) | Err(()), true) => TrashOutcome::Fallback,
    }
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
    /// 선택 항목들 + **확장 동사**(Shift+우클릭 · dir2 SHELL-004 `CMF_EXTENDEDVERBS` — "경로로 복사" · "PowerShell 창 열기" 등
    /// 평소 숨는 항목까지). 캐시 · 구축 · 실행이 평소 메뉴와 따로 간다(대상이 다르다).
    RowsExtended(Vec<PathBuf>),
    /// 폴더 배경.
    Bg(PathBuf),
    /// 폴더 배경 + **확장 동사**(Shift+우클릭 · dir2 win.rs:2875 — "여기에 PowerShell 창 열기" 등 평소 숨는 항목).
    BgExtended(PathBuf),
}

/// 터미널에서 수식키 없는 Ctrl+V가 **붙여넣기**인가 — Windows(dir2 · Windows Terminal 관례)만. 다른 OS의 터미널에서 Ctrl+V는
/// "다음 글자를 그대로"(0x16)라 셸로 보낸다(붙여넣기 = Ctrl+Shift+V · ⌘V).
pub(crate) fn term_ctrl_v_pastes() -> bool {
    cfg!(windows)
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
    /// **유휴 해제**(T-179 J-b): 들고 있는 네이티브 메뉴 객체(COM · HMENU)를 놓는다 — 항목 캐시는 남겨 다음 우클릭은 즉시 뜨고,
    /// 실행 때만 다시 구축한다. 기본 = 아무것도 안 함.
    fn release(&self) {}
    /// **비차단 조회**: `Some` = 지금 줄 수 있음 · `None` = 구축 중(끝나면 `poll`이 `Items`를 준다 — 호스트는 자체 항목만 먼저 띄운다).
    /// 기본 = 동기 `items`/`bg_items`.
    fn try_items(&self, target: &MenuTarget) -> Option<Vec<ShellMenuItem>> {
        Some(
            match target {
                MenuTarget::Rows(paths) | MenuTarget::RowsExtended(paths) => self.items(paths),
                MenuTarget::Bg(dir) | MenuTarget::BgExtended(dir) => self.bg_items(dir),
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
    /// **삭제 전 잠금 확인**(dir2 WINB-024 `probe_locked`): 다른 프로그램이 쓰고 있어 지금 지울 수 없는 항목들. 폴더는 자신만 본다
    /// (하위 항목은 보지 않는다). 기본 = 없음(검사하지 않는 OS · 파일을 열어 둔 채 지울 수 있는 Unix).
    fn probe_locked(&self, _paths: &[PathBuf]) -> Vec<PathBuf> {
        Vec::new()
    }
    /// **삭제 뒤에도 남아 있는 항목**(dir2 X-35 `on_delete_message`: 셸 일괄 삭제의 반환값으로는 항목별 성패를 알 수 없어
    /// "아직 있는가"가 판정 원천이다) — 호출부가 이것으로 성공/실패를 가른다. 기본 = 파일 시스템 조회.
    fn remaining(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        paths
            .iter()
            .filter(|p| p.symlink_metadata().is_ok())
            .cloned()
            .collect()
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
    /// 바로 가기 파일(Windows `.lnk`)이 가리키는 대상 경로 — 해석할 수 없거나 이 OS에 그런 파일이 없으면 `None`(GAP-007 ·
    /// 호출부는 대상이 폴더일 때 앱 안에서 이동하고, 아니면 [`Self::open`]으로 연다).
    fn link_target(&self, _path: &Path) -> Option<PathBuf> {
        None
    }
    /// **`shell:` 특수 폴더 별칭** → 실제 폴더 경로(dir2 shellpath.rs — 탐색기와 같은 이름: `shell:startup` · `shell:downloads` ·
    /// `shell:sendto` · `shell:::{GUID}`). 모르는 이름 · 파일 시스템 경로가 없는 가상 폴더 · 이 스킴이 없는 OS = `None`
    /// (호출부는 원문 그대로 열어 보고 "열기 실패 = 자리 유지"로 처리한다).
    fn resolve_alias(&self, _input: &str) -> Option<PathBuf> {
        None
    }
}

pub(crate) trait Disk {
    /// (전체, 여유) 바이트 · 모르면 `None`.
    fn space(&self, root: &Path) -> Option<(u64, u64)>;
    /// **볼륨 구성 지문** — 드라이브/볼륨이 붙거나 떨어지면 값이 바뀐다(dir2 WINC-061 `WM_DEVICECHANGE` 대응: "내 PC"를 보는 패널이
    /// 이 값이 바뀌면 다시 읽는다). 디스크 I/O 없이 싸게(Windows = `GetLogicalDrives` 비트맵 · Unix = 마운트 목록).
    /// 0 = 모름(다시 읽지 않는다).
    fn volumes_stamp(&self) -> u64 {
        0
    }
    /// **디스크 할당 크기**(dir2 SHELL-081 — 탐색기 속성의 "디스크 할당 크기"): 파일이 실제로 차지하는 바이트(압축 · 스파스 반영)를
    /// 클러스터 단위로 올린 값. 모르면 · 물으면 안 되는 경우(네트워크 경로 = 왕복이 UI를 멈춘다 · 클라우드 온라인 전용 파일 ·
    /// 폴더)는 `None` → 호출부가 그 줄을 생략한다.
    fn size_on_disk(&self, _path: &Path) -> Option<u64> {
        None
    }
}

// ───────────────────────── 드롭 수신(자체 수신부 — Windows `windrop` · 다른 OS는 winit 기본 수신 + 포인터 조회)

/// 끌려온 것이 창 위에서 겪는 일 — 좌표는 **창 안**(내용 영역) 기준.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DropEvent {
    Enter {
        paths: Vec<PathBuf>,
        at: (i32, i32),
        ctrl: bool,
        shift: bool,
    },
    Over {
        at: (i32, i32),
        ctrl: bool,
        shift: bool,
    },
    Leave,
    Drop {
        paths: Vec<PathBuf>,
        at: (i32, i32),
        ctrl: bool,
        shift: bool,
    },
}

/// 놓았을 때 일어날 일(= 커서 모양): 불가 · 복사 · 이동.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DropChoice {
    #[default]
    None,
    Copy,
    Move,
}

/// 복사/이동 규칙(순수 · 탐색기 관례 · dir2 `op_for`): **Ctrl = 복사 · Shift = 이동 · 그 밖 = 같은 볼륨이면 이동 / 다르면 복사**.
/// Ctrl이 Shift보다 먼저다(둘 다 = 복사 — dir2 계승 · 탐색기의 "바로 가기 만들기"는 없다).
pub(crate) fn drop_choice(ctrl: bool, shift: bool, same_volume: bool) -> DropChoice {
    if ctrl {
        DropChoice::Copy
    } else if shift || same_volume {
        DropChoice::Move
    } else {
        DropChoice::Copy
    }
}

/// 패널 한 칸의 요약(창 안 사각형 + 그 패널의 현재 폴더) — 다른 프로그램의 드래그에 **즉시** 효과를 답하려고 수신부가 본다
/// (정확한 대상 · 금지 판정은 앱이 사건을 거둘 때 다시 한다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DropZone {
    pub rect: (i32, i32, i32, i32),
    pub root: PathBuf,
}

/// 수신부와 앱이 나눠 쓰는 상태: 쌓인 사건 · 패널 요약 · 지금 끌려오는 경로.
#[derive(Debug, Default)]
pub(crate) struct DropShared {
    pub events: Vec<DropEvent>,
    pub zones: Vec<DropZone>,
    pub dragging: Vec<PathBuf>,
}

/// 요약만으로 정하는 효과(순수): 포인터가 어느 패널 위에도 없으면 불가 · 끌려오는 것이 그 폴더 자신이거나 그 폴더를 품으면 불가 ·
/// 아니면 [`drop_choice`].
pub(crate) fn zone_choice(
    zones: &[DropZone],
    dragging: &[PathBuf],
    at: (i32, i32),
    ctrl: bool,
    shift: bool,
) -> DropChoice {
    let Some(zone) = zones.iter().find(|z| {
        let (x, y, w, h) = z.rect;
        at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h
    }) else {
        return DropChoice::None;
    };
    let Some(first) = dragging.first() else {
        return DropChoice::None;
    };
    if dragging
        .iter()
        .any(|s| s == &zone.root || ndir_ops::is_same_or_sub(s, &zone.root))
    {
        return DropChoice::None;
    }
    drop_choice(ctrl, shift, ndir_ops::same_volume(first, &zone.root))
}

/// 실시간 수신기 — 우리 창에서 시작한 드래그가 도는 동안 앱이 걸어 둔다(사건을 그 자리에서 처리하고 효과를 답한다).
pub(crate) type DropSink = Box<dyn FnMut(&DropEvent) -> DropChoice>;

thread_local! {
    static LIVE_SINK: RefCell<Option<DropSink>> = const { RefCell::new(None) };
}

/// `f`가 도는 동안 실시간 수신기를 걸어 둔다(끝나면 걷는다) — `f` = OS 드래그 호출.
pub(crate) fn with_live_drop_sink<R>(sink: DropSink, f: impl FnOnce() -> R) -> R {
    LIVE_SINK.with(|s| *s.borrow_mut() = Some(sink));
    let out = f();
    LIVE_SINK.with(|s| *s.borrow_mut() = None);
    out
}

/// 수신부가 만든 사건의 갈 곳: 실시간 수신기가 걸려 있으면 그리로(앱이 바로 처리) · 아니면 큐에 쌓고 요약으로 효과를 정한다.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn drop_dispatch(shared: &Rc<RefCell<DropShared>>, ev: DropEvent) -> DropChoice {
    // 수신기를 잠시 꺼내 부른다(부르는 동안 thread_local을 빌려 두지 않는다 — 수신기 안에서 다시 들어와도 안전).
    let sink = LIVE_SINK.with(|s| s.borrow_mut().take());
    if let Some(mut sink) = sink {
        let choice = sink(&ev);
        LIVE_SINK.with(|s| {
            let mut slot = s.borrow_mut();
            if slot.is_none() {
                *slot = Some(sink);
            }
        });
        return choice;
    }
    let mut sh = shared.borrow_mut();
    let choice = match &ev {
        DropEvent::Enter {
            paths,
            at,
            ctrl,
            shift,
        } => {
            sh.dragging.clone_from(paths);
            zone_choice(&sh.zones, &sh.dragging, *at, *ctrl, *shift)
        }
        DropEvent::Over { at, ctrl, shift } => {
            zone_choice(&sh.zones, &sh.dragging, *at, *ctrl, *shift)
        }
        DropEvent::Leave => {
            sh.dragging.clear();
            DropChoice::None
        }
        DropEvent::Drop {
            paths,
            at,
            ctrl,
            shift,
        } => {
            let c = zone_choice(&sh.zones, paths, *at, *ctrl, *shift);
            sh.dragging.clear();
            c
        }
    };
    sh.events.push(ev);
    choice
}

/// 메인 창에 자체 드롭 수신부를 단다 — Windows만(창은 `with_drag_and_drop(false)`로 만들어져 있어야 한다). 다른 OS · 실패 =
/// `None`(winit 기본 수신 경로를 쓴다).
pub(crate) fn register_drop_target(
    window: &winit::window::Window,
) -> Option<Rc<RefCell<DropShared>>> {
    #[cfg(windows)]
    {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(h) => windrop::register(h.hwnd.get()),
            _ => None,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}

/// 부모 프로세스의 콘솔에 붙는다 — Windows의 **창 프로그램**(콘솔 없음)이 터미널에서 실행됐을 때 `--version` · `--selfcheck` 같은
/// 출력이 그 터미널에 보이게(nexa-clip `console::attach_parent`와 같다). 부모에 콘솔이 없으면(탐색기 · 바로 가기) 조용히 실패한다 —
/// 그게 "콘솔 창이 뜨지 않는" 정상 경로다. 부모가 표준 출력/오류 핸들(파이프 · 파일)을 넘겼으면 **붙지 않고 그 핸들을 그대로 쓴다**
/// (붙으면 핸들이 콘솔로 바뀌어 `$v = & exe --version` 같은 캡처가 비게 된다 — T-160 2차 release 스모크). 다른 OS = 아무것도 안 함.
pub(crate) fn attach_parent_console() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn AttachConsole(pid: u32) -> i32;
            fn GetStdHandle(which: u32) -> *mut std::ffi::c_void;
        }
        const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
        const STD_ERROR_HANDLE: u32 = -12i32 as u32;
        let given = |h: *mut std::ffi::c_void| !h.is_null() && h as isize != -1;
        // SAFETY: 인자 하나짜리 단순 호출들 — 실패는 무시한다.
        unsafe {
            if given(GetStdHandle(STD_OUTPUT_HANDLE)) || given(GetStdHandle(STD_ERROR_HANDLE)) {
                return;
            }
            AttachConsole(u32::MAX);
        }
    }
}

/// 터미널 셸에 **물려주지 않을** 환경 변수(순수 · T-107 · 사용자 10-06 "터미널 색이 사라졌다"): 앱을 띄운 쪽(AI 에이전트 ·
/// CI)의 `NO_COLOR` · `CLAUDECODE` · `CLAUDE_*`가 PTY 셸(PowerShell · PSReadLine · ls)까지 내려가 색을 끈다. 사용자 셸은 사용자
/// 환경 그대로여야 한다.
pub(crate) fn pty_env_blocked(name: &str) -> bool {
    name == "NO_COLOR" || name == "CLAUDECODE" || name.starts_with("CLAUDE_")
}

/// 지금 프로세스 환경에서 차단 변수를 뺀 목록(이름 · 값) — PTY 생성 때 환경 블록/unsetenv의 원천.
pub(crate) fn pty_env() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    std::env::vars_os()
        .filter(|(k, _)| !k.to_str().is_some_and(pty_env_blocked))
        .collect()
}

/// 작업 집합을 운영체제에 돌려준다(유휴 트림 · dir2 M2-8 `trim_resident` 계승 · T-179 A). Windows =
/// `SetProcessWorkingSetSize(-1, -1)`(미사용 페이지를 대기 목록으로 — 작업 관리자의 "메모리"가 곧바로 준다 · 되돌아올 때
/// 소프트 페이지 폴트 비용) · 다른 OS = 아무것도 안 함(힙 반납은 `procmem::trim`이 한다).
pub(crate) fn trim_working_set() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn SetProcessWorkingSetSize(
                process: *mut std::ffi::c_void,
                min: usize,
                max: usize,
            ) -> i32;
        }
        // SAFETY: 의사 핸들에 문서화된 호출 — 실패는 반환값으로만 알려지고 부작용이 없다.
        unsafe {
            SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        }
    }
}

/// 지금 스레드의 우선순위를 낮춘다(긴 계산 워커 — 체크섬 · 사용자 10-06 "해시가 프로세스를 잡아먹는다"). Windows =
/// `THREAD_PRIORITY_BELOW_NORMAL` · 다른 OS = 아무것도 안 함(nice는 프로세스 단위라 건드리지 않는다).
pub(crate) fn lower_thread_priority() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> *mut std::ffi::c_void;
            fn SetThreadPriority(thread: *mut std::ffi::c_void, priority: i32) -> i32;
        }
        const THREAD_PRIORITY_BELOW_NORMAL: i32 = -1;
        // SAFETY: 의사 핸들에 단순 호출 — 실패는 무시한다.
        unsafe {
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
        }
    }
}

/// 지금의 포인터 자리(화면 좌표)와 수식키 — **다른 프로그램에서 끌어오는 동안**의 상태를 묻는다(T-147 수신 보강).
/// winit은 OS 드래그 중에는 포인터 이동 · 수식키 사건을 주지 않고 `HoveredFile`/`DroppedFile`에도 자리가 없다 →
/// 놓는 자리 · 복사/이동 판정을 위해 직접 읽는다. 읽을 수 없는 OS = `None`(호출부는 마지막으로 본 값으로 간다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointerState {
    pub x: i32,
    pub y: i32,
    pub ctrl: bool,
    pub shift: bool,
}

/// [`PointerState`] 조회 — Windows = `GetCursorPos` + `GetAsyncKeyState` · macOS · Linux = 후속(`None`).
pub(crate) fn pointer_state() -> Option<PointerState> {
    #[cfg(windows)]
    {
        windows::pointer_state()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// 클러스터 올림(순수 · dir2 fileinfo.rs `round_up_cluster`): 클러스터 0(모름)이면 점유 바이트 그대로 · 0바이트 = 0.
/// 압축 파일은 점유가 논리 크기보다 작을 수 있다(그대로 둔다).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn round_up_cluster(used: u64, cluster: u64) -> u64 {
    if cluster == 0 || used == 0 {
        return used;
    }
    used.div_ceil(cluster) * cluster
}

/// Unix의 디스크 할당 크기 = `st_blocks` × 512(POSIX 단위 · 이미 블록 단위로 올려진 값) — 파일만.
#[cfg(unix)]
pub(crate) fn unix_size_on_disk(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    let md = std::fs::symlink_metadata(path).ok()?;
    md.is_file().then(|| md.blocks() * 512)
}

/// Unix의 볼륨 구성 지문 = "내 PC" 항목 이름들의 해시(`ndir_vfs::drive_entries` — `/proc/self/mounts` · `/Volumes` 읽기).
#[cfg(unix)]
pub(crate) fn unix_volumes_stamp() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for e in ndir_vfs::drive_entries() {
        e.name.hash(&mut h);
    }
    h.finish().max(1)
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
    /// 드래그 발신 — `Rc`(호출 동안 앱을 빌리지 않으려고 복제해 부른다: 드래그 중 드롭 수신부가 앱을 다시 부른다).
    pub drag: Rc<dyn DragSource>,
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

/// 환경 변수 `NDIR_FAKE_CLIPBOARD`(값이 있고 `0`이 아님) — 이 프로세스는 **OS 클립보드를 읽지도 쓰지도 않는다**(텍스트 + 파일 ·
/// 프로세스 안 메모리로 대신). 시나리오 시험(T4 `ndir-check`)과 캡처용 실행이 사용자의 클립보드를 덮어쓰지 않게 하는 스위치
/// (CLAUDE.md §5 · 10-03 발견: copy-paste.scn · ctx-menu.scn이 실제 CF_HDROP을 게시했다).
pub(crate) fn fake_clipboard() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| fake_clipboard_value(std::env::var_os("NDIR_FAKE_CLIPBOARD").as_deref()))
}

/// 스위치 값 해석(순수): 없음 · 빈 값 · `0` = 꺼짐 / 그 밖 = 켜짐.
fn fake_clipboard_value(v: Option<&std::ffi::OsStr>) -> bool {
    v.is_some_and(|v| !v.is_empty() && v != "0")
}

/// 프로세스 안 파일 클립보드([`fake_clipboard`]) — 같은 프로세스의 복사 → 붙여넣기는 그대로 이어진다.
#[derive(Default)]
struct MemoryFiles(std::cell::RefCell<Option<(Vec<PathBuf>, bool)>>);

impl FileClipboard for MemoryFiles {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        self.0.borrow().clone()
    }
    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError> {
        *self.0.borrow_mut() = Some((paths.to_vec(), cut));
        Ok(())
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
        // Linux = 앱 연결(MimeApps)로 만든 항목(1차) · macOS = 아직 없음(앱 메뉴만).
        #[cfg(all(unix, not(target_os = "macos")))]
        let ctxmenu: Box<dyn ContextMenuProvider> = Box::new(linux::XdgMenu::default());
        #[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
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
        let clipboard: Box<dyn FileClipboard> = if fake_clipboard() {
            Box::new(MemoryFiles::default())
        } else {
            clipboard
        };
        Platform {
            shell,
            pty,
            ctxmenu,
            trash,
            clipboard,
            // 드래그 발신: Windows = OLE(`windrag`) · macOS(NSDraggingSource) · Linux(XDND 발신)는 후속 — 그때까지 미지원.
            #[cfg(windows)]
            drag: Rc::new(windrag::NativeDrag),
            #[cfg(not(windows))]
            drag: Rc::new(Unsupported),
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

/// "점 파일 표시" 토글이 따로 있는 OS인가 — Windows만(숨김 속성과 점 파일이 별개). Linux · macOS는 점으로 시작하면 숨김 파일이라
/// "숨김 파일 표시" 하나로 다룬다(`ndir_vfs::DOT_IS_HIDDEN` · 사용자 10-03) → 메뉴 · 툴바 · 순서 편집기에서 뺀다.
pub(crate) fn has_dotfile_toggle() -> bool {
    !ndir_vfs::DOT_IS_HIDDEN
}

/// 바로 가기 파일의 확장자를 이름에서 숨기는 OS인가(GAP-006) — Windows 탐색기는 `.lnk` · `.url` · `.appref-ms`를
/// "확장명 표시" 설정과 무관하게 늘 숨긴다. 다른 OS에서는 평범한 파일이라 그대로 보인다.
pub(crate) fn hides_shortcut_ext() -> bool {
    cfg!(windows)
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

/// Windows Terminal 기본 프로필에서 터미널 도크가 따라갈 값(DR-21 · 사용자 10-03 "기본 터미널과 동일하게").
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WtProfile {
    /// 글꼴 이름 목록(`font.face`의 쉼표 구분 순서 = 대체 글꼴 순서).
    pub faces: Vec<String>,
    /// 글꼴 크기(pt · Windows Terminal 기본 12). `None` = 크기는 따라가지 않는다(설정 `term.font_size` — Linux: 사용자 10-03
    /// "터미널 글꼴이 너무 크다 · 정보/미리보기와 맞춰" — 데스크톱 고정폭 글꼴 11 pt는 본문(12 em = 9 pt)보다 크다).
    pub size_pt: Option<f32>,
    /// 색 구성표 이름(없으면 Windows Terminal 기본 = Campbell).
    pub scheme: Option<String>,
    /// 시작 명령(`commandline` · 없으면 프로필 원천의 기본).
    pub commandline: Option<String>,
}

/// JSON 주석 제거(`// …` · `/* … */` — 문자열 안은 그대로). Windows Terminal의 settings.json은 주석을 허용한다.
fn strip_json_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut it = text.chars().peekable();
    let (mut in_str, mut esc) = (false, false);
    while let Some(c) = it.next() {
        if in_str {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match (c, it.peek()) {
            ('"', _) => {
                in_str = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for d in it.by_ref() {
                    if d == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                it.next();
                let mut prev = ' ';
                for d in it.by_ref() {
                    if prev == '*' && d == '/' {
                        break;
                    }
                    prev = d;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// settings.json 본문 → 기본 프로필 값(순수 · 3-OS 시험 가능). 값 우선순위 = 기본 프로필 > `profiles.defaults` > 내장 기본.
/// 옛 형식(`fontFace`/`fontSize`)도 읽는다.
pub(crate) fn parse_wt_settings(text: &str) -> Option<WtProfile> {
    use ndir_settings::json::Json;
    fn get<'a>(v: &'a Json, key: &str) -> Option<&'a Json> {
        match v {
            Json::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn text_of(v: Option<&Json>) -> Option<String> {
        match v {
            Some(Json::Str(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            _ => None,
        }
    }
    fn num_of(v: Option<&Json>) -> Option<f32> {
        match v {
            Some(Json::Num(n)) => Some(*n as f32),
            _ => None,
        }
    }
    let root = ndir_settings::json::parse(&strip_json_comments(text)).ok()?;
    let profiles = get(&root, "profiles")?;
    let default_guid = text_of(get(&root, "defaultProfile"));
    let (defaults, list): (Option<&Json>, Option<&Json>) = match profiles {
        Json::Arr(_) => (None, Some(profiles)),
        _ => (get(profiles, "defaults"), get(profiles, "list")),
    };
    let profile = match list {
        Some(Json::Arr(items)) => default_guid
            .as_deref()
            .and_then(|g| {
                items.iter().find(|p| {
                    text_of(get(p, "guid")).is_some_and(|x| x.eq_ignore_ascii_case(g))
                        || text_of(get(p, "name")).is_some_and(|x| x == g)
                })
            })
            .or_else(|| items.first()),
        _ => None,
    };
    let pick = |f: &dyn Fn(&Json) -> Option<String>| -> Option<String> {
        profile.and_then(f).or_else(|| defaults.and_then(f))
    };
    let face = pick(&|p| {
        text_of(get(p, "font").and_then(|f| get(f, "face"))).or_else(|| text_of(get(p, "fontFace")))
    });
    let size = profile
        .and_then(|p| {
            num_of(get(p, "font").and_then(|f| get(f, "size")))
                .or_else(|| num_of(get(p, "fontSize")))
        })
        .or_else(|| {
            defaults.and_then(|p| {
                num_of(get(p, "font").and_then(|f| get(f, "size")))
                    .or_else(|| num_of(get(p, "fontSize")))
            })
        });
    Some(WtProfile {
        faces: face
            .map(|f| {
                f.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        size_pt: Some(size.filter(|s| (4.0..=96.0).contains(s)).unwrap_or(12.0)),
        scheme: pick(&|p| text_of(get(p, "colorScheme"))),
        commandline: pick(&|p| text_of(get(p, "commandline"))),
    })
}

/// 이 PC의 Windows Terminal 설정에서 기본 프로필을 읽는다(정식 → 프리뷰 → 비패키지 순 · 없으면 `None`). 다른 OS = `None`.
pub(crate) fn windows_terminal_profile() -> Option<WtProfile> {
    #[cfg(windows)]
    {
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
        [
            "Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json",
            "Packages/Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe/LocalState/settings.json",
            "Microsoft/Windows Terminal/settings.json",
        ]
        .iter()
        .find_map(|rel| {
            let text = std::fs::read_to_string(local.join(rel)).ok()?;
            parse_wt_settings(text.trim_start_matches('\u{feff}'))
        })
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// 터미널 글꼴의 폴백(한글 등)을 주 글꼴과 **같은 em**으로 그릴까(nexa-gfx `set_fallback_em_match`). Linux·macOS = 켬
/// (터미널 관례 — 전각 글자가 두 칸을 채운다 · Linux 실기 10-03 "한글이 작고 벌어진다"). Windows = 종전 유지
/// (Windows Terminal 대조 캡처로 맞춘 화면을 실기 확인 없이 바꾸지 않는다 — 확인되면 켠다).
pub(crate) fn term_fallback_em_match() -> bool {
    cfg!(not(windows))
}

/// 글꼴 지정 문자열(GNOME `monospace-font-name` · Pango 표기 `"Ubuntu Sans Mono 11"` · `"DejaVu Sans Mono Bold 10.5"`) →
/// (글꼴 이름, 크기 pt)(순수). 끝의 숫자 = 크기 · 그 앞의 굵기/기울기 낱말은 떼어 낸다. 크기가 없거나 범위(4~96) 밖이면 `None`.
pub(crate) fn parse_font_spec(spec: &str) -> Option<(String, f32)> {
    const STYLES: [&str; 12] = [
        "regular",
        "bold",
        "italic",
        "oblique",
        "light",
        "medium",
        "semi-bold",
        "semibold",
        "thin",
        "book",
        "condensed",
        "heavy",
    ];
    let spec = spec.trim().trim_matches(['\'', '"']).trim();
    let (name, size) = spec.rsplit_once(' ')?;
    let size: f32 = size.trim().parse().ok()?;
    if !(4.0..=96.0).contains(&size) {
        return None;
    }
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1
        && words
            .last()
            .is_some_and(|w| STYLES.contains(&w.to_ascii_lowercase().as_str()))
    {
        words.pop();
    }
    (!words.is_empty()).then(|| (words.join(" "), size))
}

/// 이 OS의 **기본 터미널이 쓰는 글꼴**(터미널 도크가 따라갈 값 · DR-21 확장 — 사용자 10-03 Linux 실기 "시스템 기본(터미널과 동일한)
/// 폰트로"): Windows = Windows Terminal 기본 프로필(글꼴 + 크기) · Linux = 데스크톱의 고정폭 글꼴 **이름**(`gsettings
/// org.gnome.desktop.interface monospace-font-name` — GNOME 터미널 기본값이 이것을 쓴다) · 그 밖/없음 = `None`(설정 값 사용).
/// 시험 빌드에서는 Linux도 `None`(프로세스를 띄우지 않고 · 화면이 PC 설정에 따라 달라지지 않게).
pub(crate) fn system_terminal_profile() -> Option<WtProfile> {
    #[cfg(windows)]
    {
        windows_terminal_profile()
    }
    #[cfg(all(target_os = "linux", not(test)))]
    {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "monospace-font-name"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        // 글꼴 이름만 따라간다 — 크기는 설정 `term.font_size`(기본 12 = 정보/미리보기 본문과 같은 em).
        let (face, _) = parse_font_spec(&String::from_utf8_lossy(&out.stdout))?;
        Some(WtProfile {
            faces: vec![face],
            size_pt: None,
            scheme: None,
            commandline: None,
        })
    }
    #[cfg(not(any(windows, all(target_os = "linux", not(test)))))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    /// 디스크 할당 크기의 클러스터 올림(dir2 fileinfo.rs:431 시험 이식): 올림 · 딱 맞음 · 클러스터 모름 · 0바이트.
    #[test]
    fn cluster_round_up() {
        use super::round_up_cluster;
        assert_eq!(round_up_cluster(1, 4096), 4096);
        assert_eq!(round_up_cluster(4096, 4096), 4096);
        assert_eq!(round_up_cluster(4097, 4096), 8192);
        assert_eq!(round_up_cluster(123, 0), 123, "클러스터 모름 = 그대로");
        assert_eq!(round_up_cluster(0, 4096), 0, "0바이트 = 0");
    }

    /// 실제 파일의 디스크 할당 크기(이 OS 구현): 내용이 있는 파일 = 논리 크기 이상의 블록 배수 · 폴더 = 없음.
    #[test]
    fn native_size_on_disk_for_file_not_folder() {
        let dir = std::env::temp_dir().join(format!("ndir-ondisk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let f = dir.join("data.bin");
        std::fs::write(&f, vec![7u8; 5000]).expect("write");
        let p = super::Platform::native();
        assert_eq!(p.disk.size_on_disk(&dir), None, "폴더 = 묻지 않는다");
        // 파일 시스템에 따라(압축 · 지연 할당) 값이 다를 수 있다 — 주면 512의 배수여야 한다.
        if let Some(used) = p.disk.size_on_disk(&f) {
            assert_eq!(used % 512, 0, "{used}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::*;

    /// 글꼴 지정 문자열: 끝 숫자 = 크기 · 굵기 낱말 제거 · 따옴표/공백 · 크기 없음/범위 밖 = None.
    #[test]
    fn font_spec_splits_family_and_size() {
        assert_eq!(
            parse_font_spec("'Ubuntu Sans Mono 11'\n"),
            Some(("Ubuntu Sans Mono".into(), 11.0))
        );
        assert_eq!(
            parse_font_spec("DejaVu Sans Mono Bold 10.5"),
            Some(("DejaVu Sans Mono".into(), 10.5))
        );
        assert_eq!(
            parse_font_spec("Monospace Semi-Bold Italic 12"),
            Some(("Monospace".into(), 12.0))
        );
        assert_eq!(parse_font_spec("Bold 9"), Some(("Bold".into(), 9.0)));
        assert_eq!(parse_font_spec("Ubuntu Mono"), None);
        assert_eq!(parse_font_spec("Mono 200"), None);
        assert_eq!(parse_font_spec("''"), None);
        // 시험 빌드의 Linux는 프로세스를 띄우지 않는다.
        #[cfg(target_os = "linux")]
        assert_eq!(system_terminal_profile(), None);
    }

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
        // Windows = 셸 메뉴 · Linux = 앱 연결 메뉴(빈 입력 = 빈 목록) · macOS = 아직 없음(Unsupported).
        if cfg!(any(windows, all(unix, not(target_os = "macos")))) {
            assert_eq!(
                p.ctxmenu.items(&[]).map(|v| v.len()),
                Ok(0),
                "메뉴 포트 있음(빈 입력 = 빈 목록)"
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

    /// Windows Terminal settings.json 해석: 주석 허용 · 기본 프로필 > defaults > 내장 기본(12pt) · 글꼴 목록은 쉼표 순서 · 옛 키.
    #[test]
    fn wt_settings_parse_default_profile() {
        let text = r#"{
            // 주석 한 줄
            "defaultProfile": "{574e775e-4f2a-5b96-ac1e-a2962a402336}",
            "profiles": {
                "defaults": { "font": { "face": "D2Coding, JetBrainsMono Nerd Font" }, "opacity": 80 },
                "list": [
                    { "guid": "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}", "name": "cmd", "font": { "size": 9 } },
                    /* 기본 프로필 */
                    { "guid": "{574E775E-4F2A-5B96-AC1E-A2962A402336}", "name": "PowerShell", "colorScheme": "One Half Dark" }
                ]
            },
            "url": "https://example.com/a//b"
        }"#;
        let p = parse_wt_settings(text).expect("profile");
        assert_eq!(p.faces, ["D2Coding", "JetBrainsMono Nerd Font"]);
        assert_eq!(
            p.size_pt,
            Some(12.0),
            "지정 없음 = Windows Terminal 기본 12pt"
        );
        assert_eq!(p.scheme.as_deref(), Some("One Half Dark"));
        assert_eq!(p.commandline, None);
        // 프로필 값이 defaults보다 우선 · 옛 키.
        let old = r#"{"defaultProfile":"x","profiles":[{"name":"x","fontFace":"Consolas","fontSize":10,"commandline":"cmd.exe /k"}]}"#;
        let p = parse_wt_settings(old).expect("old format");
        assert_eq!(
            (p.faces.as_slice(), p.size_pt),
            (&["Consolas".to_string()][..], Some(10.0))
        );
        assert_eq!(p.commandline.as_deref(), Some("cmd.exe /k"));
        assert!(parse_wt_settings("not json").is_none());
        assert!(parse_wt_settings("{}").is_none(), "profiles 없음");
    }

    /// `NDIR_FAKE_CLIPBOARD` 값 해석(없음 · 빈 값 · 0 = 꺼짐) + 프로세스 안 파일 클립보드는 쓴 것을 그대로 돌려준다.
    #[test]
    fn fake_clipboard_switch_and_memory_files() {
        use std::ffi::OsStr;
        assert!(!fake_clipboard_value(None));
        assert!(!fake_clipboard_value(Some(OsStr::new(""))));
        assert!(!fake_clipboard_value(Some(OsStr::new("0"))));
        assert!(fake_clipboard_value(Some(OsStr::new("1"))));
        let m = MemoryFiles::default();
        assert_eq!(m.read_files(), None);
        m.write_files(&[PathBuf::from("a.txt")], true).unwrap();
        assert_eq!(m.read_files(), Some((vec![PathBuf::from("a.txt")], true)));
    }

    /// 휴지통 판정(T-135 MC/DC): 결과 경로가 있으면 그대로 · 없거나 오류여도 원본이 사라졌으면 옮겨진 것 · 원본이 남았을 때만 폴백.
    #[test]
    fn trash_outcome_decides_by_original_presence() {
        let t = PathBuf::from("/x/.Trash/a");
        assert_eq!(
            trash_outcome(Ok(Some(t.clone())), false),
            TrashOutcome::Trashed(Some(t.clone()))
        );
        assert_eq!(
            trash_outcome(Ok(Some(t.clone())), true),
            TrashOutcome::Trashed(Some(t))
        );
        assert_eq!(trash_outcome(Ok(None), false), TrashOutcome::Trashed(None));
        assert_eq!(trash_outcome(Err(()), false), TrashOutcome::Trashed(None));
        assert_eq!(trash_outcome(Ok(None), true), TrashOutcome::Fallback);
        assert_eq!(trash_outcome(Err(()), true), TrashOutcome::Fallback);
    }
}

/// PTY 환경 차단(T-107 · MC/DC): NO_COLOR · CLAUDECODE · CLAUDE_* 는 막고 · PATH · CLAUDE(접두 아님) · NO_COLORS(다른 이름)는 둔다 ·
/// `pty_env`는 지금 환경에서 그것들만 뺀다.
#[cfg(test)]
mod pty_env_tests {
    use super::*;

    #[test]
    fn blocklist_rules_and_env_filter() {
        assert!(pty_env_blocked("NO_COLOR"));
        assert!(pty_env_blocked("CLAUDECODE"));
        assert!(pty_env_blocked("CLAUDE_CODE_SESSION_ID"));
        assert!(!pty_env_blocked("PATH"));
        assert!(!pty_env_blocked("CLAUDE"));
        assert!(!pty_env_blocked("NO_COLORS"));
        assert!(!pty_env_blocked("no_color"));
        std::env::set_var("NDIR_PTY_ENV_PROBE", "1");
        let env = pty_env();
        assert!(env.iter().any(|(k, _)| k == "NDIR_PTY_ENV_PROBE"));
        assert!(!env
            .iter()
            .any(|(k, _)| k.to_str().is_some_and(pty_env_blocked)));
        std::env::remove_var("NDIR_PTY_ENV_PROBE");
    }
}
