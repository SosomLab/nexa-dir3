//! Windows 셸 컨텍스트 메뉴(T-51 B · dir2 `shellmenu.rs` + `menuthread.rs` 이식 — docs/port/19 SHELL-001~002·005·006·011~015):
//! 경로 → PIDL(`SHParseDisplayName`) → 공통 부모 `IShellFolder`(`SHBindToParent`) → `IContextMenu`(`GetUIObjectOf`) →
//! `QueryContextMenu`로 받은 HMENU를 **열거해 앱 메뉴 항목으로 번역**(dir2는 네이티브 HMENU를 띄웠다 — dir3는 nexa-ctl `ContextMenu`가 그린다).
//! 서브메뉴는 `IContextMenu2::HandleMenuMsg(WM_INITMENUPOPUP)`로 채운 뒤 2단까지 열거(보내기 · 연결 프로그램). verb(`GetCommandString`)는
//! 호출부가 가로채기(cut/copy/paste/delete/rename/copyaspath → 앱 경로)에 쓴다. 항목 아이콘 = `MIIM_BITMAP`의 `hbmpItem`(32bpp
//! 프리멀티플라이드 ARGB · Vista 이후 셸 확장 표준)을 `GetDIBits`로 읽어 straight RGBA로([`ShellIcon`] · SHELL-011).
//!
//! ★ **전용 메뉴 스레드**(dir2 10-02 X-61 "우클릭 가속 설계 A" · `nexa-dir2/docs/audit/20261002-ultracode/ctxmenu-latency.md`):
//! 우클릭 지연의 95 %는 등록된 셸 확장들이 `QueryContextMenu`에서 도는 시간(0.6~1.4 s)이고 `IContextMenu`는 프록시가 없어 만든 아파트
//! 밖으로 못 넘긴다 → **STA 스레드 하나(`ndir-ctxmenu`)가 메뉴의 전 생애**(구축 → 열거 → 명령 실행)를 맡는다.
//! - [`ContextMenuProvider::prepare`]: 선택이 머물면 호스트가 **미리 구축**을 시킨다(결과 = 항목 목록 · UI 쪽 캐시 1벌).
//! - [`ContextMenuProvider::try_items`]: 우클릭 때 캐시가 같은 대상이면 즉시 항목 · 아니면 구축을 걸고 `None`(호스트는 자체 항목만 먼저
//!   띄우고 [`ContextMenuProvider::poll`]의 `Items`가 오면 채운다). 어느 쪽이든 **UI 스레드는 막히지 않는다**.
//! - [`ContextMenuProvider::invoke_async`]: `InvokeCommand`도 메뉴 스레드에서(속성 창 같은 모달이 UI를 붙잡지 않는다) · 결과 = `Invoked`.
//! - dir2와의 차이: dir2는 그 스레드가 `TrackPopupMenuEx`까지 했지만(숨은 소유자 창 · `AttachThreadInput`) dir3는 메뉴를 직접 그리므로
//!   표시가 필요 없다 — 스레드는 메시지 펌프만 돈다(STA 규약 · 확장이 창 메시지를 쓰는 경우).
//! - 동기 `items`/`invoke`/`bg_items`/`invoke_bg`는 같은 스레드에 시키고 기다린다(자가 점검 · 시험 · 폴백).
//!
//! 배경 메뉴(SHELL-009): `SHGetDesktopFolder` → `BindToObject` → `CreateViewObject::<IContextMenu>` — 실행 뒤 폴더 이름 diff로
//! **정확히 1개** 신규면 생성 경로를 보고(dir2 `detect_created` · 20ms×10 재시도) → 호스트가 선택 + 인라인 이름 바꾸기.
//! `windows` crate(DR-8 허용 목록 OS 바인딩).

use super::*;
use ::windows::core::{Interface, PCWSTR, PSTR, PWSTR};
use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use ::windows::Win32::Graphics::Gdi::{
    GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP,
};
use ::windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use ::windows::Win32::UI::Shell::Common::ITEMIDLIST;
use ::windows::Win32::UI::Shell::{
    IContextMenu, IContextMenu2, IShellFolder, SHBindToParent, SHGetDesktopFolder,
    SHParseDisplayName, CMF_EXTENDEDVERBS, CMF_NORMAL, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX,
    GCS_VERBW,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, DispatchMessageW, GetMenuItemCount, GetMenuItemInfoW,
    PeekMessageW, TranslateMessage, HMENU, MENUITEMINFOW, MFS_DISABLED, MFS_GRAYED, MFT_SEPARATOR,
    MIIM_BITMAP, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, MSG, PM_REMOVE,
    SW_SHOWNORMAL, WM_INITMENUPOPUP,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::os::windows::ffi::OsStrExt as _;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x6FFF;
const CMIC_MASK_UNICODE: u32 = 0x4000;
/// 서브메뉴 열거 깊이(보내기 ▸ · 연결 프로그램 ▸).
const MAX_DEPTH: u32 = 2;
/// 항목 아이콘 한 변 상한(px) — 그보다 큰 비트맵은 아이콘이 아니다.
const ICON_MAX: i32 = 64;
/// 동기 호출이 메뉴 스레드를 기다리는 상한(느린 셸 확장 · 네트워크 경로).
const SYNC_WAIT: Duration = Duration::from_secs(30);
/// 메뉴 스레드의 메시지 펌프 주기.
const PUMP_MS: u64 = 25;

/// 만든 메뉴 1벌(COM 수명 = 이 구조체 · **메뉴 스레드 밖으로 나가지 않는다**).
struct Built {
    key: MenuTarget,
    icm: IContextMenu,
    hmenu: HMENU,
    pidls: Vec<*mut ITEMIDLIST>,
    _folder: IShellFolder,
    /// 실행 때 쓸 소유 창.
    hwnd_owner: *mut core::ffi::c_void,
}

impl Drop for Built {
    fn drop(&mut self) {
        // SAFETY: 우리가 만든 HMENU · CoTaskMem PIDL.
        unsafe {
            let _ = DestroyMenu(self.hmenu);
            for &p in &self.pidls {
                CoTaskMemFree(Some(p as *const core::ffi::c_void));
            }
        }
    }
}

/// UI → 메뉴 스레드.
enum Cmd {
    Prepare {
        target: MenuTarget,
        owner: isize,
    },
    Invoke {
        id: String,
        target: MenuTarget,
        owner: isize,
    },
    /// 구축해 둔 메뉴 객체 해제(유휴 트림 · 통지 없음).
    Release,
    Quit,
}

/// 메뉴 스레드 → UI.
enum Evt {
    Items {
        target: MenuTarget,
        result: Result<Vec<ShellMenuItem>, String>,
    },
    Invoked {
        target: MenuTarget,
        result: Result<Option<PathBuf>, String>,
    },
}

/// 메뉴 스레드 쪽 상태(마지막으로 만든 메뉴 1벌 — 사용자가 본 항목 id와 실행 id가 같은 HMENU에서 나온다).
struct Worker {
    built: Option<Built>,
}

impl Worker {
    /// 경로들 → IContextMenu + HMENU(QueryContextMenu). 접근 불가 항목은 제외 · 부모가 다른 항목은 첫 부모 기준으로 축소(SHELL-002).
    unsafe fn build(
        owner: isize,
        paths: &[PathBuf],
        extended: bool,
    ) -> Result<Built, PlatformError> {
        let hwnd = HWND(owner as *mut core::ffi::c_void);
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::new();
        let mut children: Vec<*const ITEMIDLIST> = Vec::new();
        let mut folder: Option<IShellFolder> = None;
        for p in paths {
            let wide: Vec<u16> = p
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            if SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None).is_err() {
                continue;
            }
            pidls.push(pidl);
            let mut child: *mut ITEMIDLIST = std::ptr::null_mut();
            let Ok(f) = SHBindToParent::<IShellFolder>(pidl, Some(&mut child)) else {
                continue;
            };
            folder.get_or_insert(f);
            children.push(child as *const ITEMIDLIST);
        }
        let free = |pidls: &[*mut ITEMIDLIST]| {
            for &p in pidls {
                CoTaskMemFree(Some(p as *const core::ffi::c_void));
            }
        };
        let (Some(folder), false) = (folder, children.is_empty()) else {
            free(&pidls);
            return Err(PlatformError::Failed("no shell items".into()));
        };
        let icm = match folder.GetUIObjectOf::<IContextMenu>(hwnd, &children, None) {
            Ok(i) => i,
            Err(e) => {
                free(&pidls);
                return Err(PlatformError::Failed(format!("GetUIObjectOf: {e}")));
            }
        };
        let hmenu = match CreatePopupMenu() {
            Ok(h) => h,
            Err(e) => {
                free(&pidls);
                return Err(PlatformError::Failed(format!("CreatePopupMenu: {e}")));
            }
        };
        // Shift+우클릭 = 확장 동사까지(dir2 SHELL-004).
        let flags = if extended {
            CMF_NORMAL | CMF_EXTENDEDVERBS
        } else {
            CMF_NORMAL
        };
        let hr = icm.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, flags);
        if hr.is_err() {
            let _ = DestroyMenu(hmenu);
            free(&pidls);
            return Err(PlatformError::Failed(format!("QueryContextMenu: {hr}")));
        }
        Ok(Built {
            key: if extended {
                MenuTarget::RowsExtended(paths.to_vec())
            } else {
                MenuTarget::Rows(paths.to_vec())
            },
            icm,
            hmenu,
            pidls,
            _folder: folder,
            hwnd_owner: owner as *mut core::ffi::c_void,
        })
    }

    /// 폴더 배경 메뉴(SHELL-009): 폴더 PIDL → `IShellFolder` → `CreateViewObject::<IContextMenu>` → QueryContextMenu.
    unsafe fn build_bg(owner: isize, dir: &Path, extended: bool) -> Result<Built, PlatformError> {
        let hwnd = HWND(owner as *mut core::ffi::c_void);
        let wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        if SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None).is_err() {
            return Err(PlatformError::Failed(format!(
                "not a shell folder: {}",
                dir.display()
            )));
        }
        let free = || CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
        let folder = match SHGetDesktopFolder()
            .and_then(|d| d.BindToObject::<_, IShellFolder>(pidl, None))
        {
            Ok(f) => f,
            Err(e) => {
                free();
                return Err(PlatformError::Failed(format!("BindToObject: {e}")));
            }
        };
        let icm = match folder.CreateViewObject::<IContextMenu>(hwnd) {
            Ok(i) => i,
            Err(e) => {
                free();
                return Err(PlatformError::Failed(format!("CreateViewObject: {e}")));
            }
        };
        let hmenu = match CreatePopupMenu() {
            Ok(h) => h,
            Err(e) => {
                free();
                return Err(PlatformError::Failed(format!("CreatePopupMenu: {e}")));
            }
        };
        // Shift+우클릭 = 확장 동사까지(dir2 win.rs:2875).
        let flags = if extended {
            CMF_NORMAL | CMF_EXTENDEDVERBS
        } else {
            CMF_NORMAL
        };
        let hr = icm.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, flags);
        if hr.is_err() {
            let _ = DestroyMenu(hmenu);
            free();
            return Err(PlatformError::Failed(format!("QueryContextMenu: {hr}")));
        }
        Ok(Built {
            key: if extended {
                MenuTarget::BgExtended(dir.to_path_buf())
            } else {
                MenuTarget::Bg(dir.to_path_buf())
            },
            icm,
            hmenu,
            pidls: vec![pidl],
            _folder: folder,
            hwnd_owner: owner as *mut core::ffi::c_void,
        })
    }

    unsafe fn build_for(owner: isize, target: &MenuTarget) -> Result<Built, PlatformError> {
        match target {
            MenuTarget::Rows(paths) => Self::build(owner, paths, false),
            MenuTarget::RowsExtended(paths) => Self::build(owner, paths, true),
            MenuTarget::Bg(dir) => Self::build_bg(owner, dir, false),
            MenuTarget::BgExtended(dir) => Self::build_bg(owner, dir, true),
        }
    }

    /// 새로 구축 + 열거(선행 구축 · 동기 조회 공통). 앞의 메뉴는 버린다.
    fn prepare(
        &mut self,
        target: &MenuTarget,
        owner: isize,
    ) -> Result<Vec<ShellMenuItem>, PlatformError> {
        self.built = None;
        // SAFETY: COM/셸 API — 입력 포인터는 호출 안의 유효 버퍼 · 산출 핸들은 `Built`가 수명 관리(이 스레드 전용).
        let built = unsafe { Self::build_for(owner, target)? };
        // SAFETY: 살아 있는 COM 객체 · 우리가 만든 HMENU.
        let items = unsafe { enumerate(&built.icm, built.hmenu, 0) };
        self.built = Some(built);
        Ok(items)
    }

    /// 명령 실행 — 사용자가 본 메뉴(같은 대상)가 남아 있으면 그 HMENU의 id 그대로 · 아니면 다시 구축.
    fn invoke(
        &mut self,
        id: &str,
        target: &MenuTarget,
        owner: isize,
    ) -> Result<Option<PathBuf>, PlatformError> {
        let offset = offset_of(id)?;
        if !self.built.as_ref().is_some_and(|b| b.key == *target) {
            self.built = None;
            // SAFETY: 위와 같음.
            self.built = Some(unsafe { Self::build_for(owner, target)? });
        }
        let Some(b) = self.built.as_mut() else {
            return Err(PlatformError::Failed("menu not built".into()));
        };
        b.hwnd_owner = owner as *mut core::ffi::c_void;
        match target {
            MenuTarget::Rows(_) | MenuTarget::RowsExtended(_) => {
                // SAFETY: icm은 살아 있는 COM 객체 · 오프셋은 범위 검사됨.
                unsafe { invoke_offset(b, offset)? };
                Ok(None)
            }
            MenuTarget::Bg(dir) | MenuTarget::BgExtended(dir) => {
                let before = dir_names(dir);
                // SAFETY: 위와 같음.
                unsafe { invoke_offset(b, offset)? };
                Ok(detect_created(dir, &before, 10))
            }
        }
    }
}

/// `shell:<id>` → 명령 오프셋(범위 검사).
fn offset_of(id: &str) -> Result<u32, PlatformError> {
    let Some(n) = id
        .strip_prefix("shell:")
        .and_then(|s| s.parse::<u32>().ok())
    else {
        return Err(PlatformError::Failed(format!("not a shell item: {id}")));
    };
    if !(ID_FIRST..=ID_LAST).contains(&n) {
        return Err(PlatformError::Failed(format!("id out of range: {id}")));
    }
    Ok(n - ID_FIRST)
}

/// InvokeCommand(오프셋 · 유니코드 · SW_SHOWNORMAL).
unsafe fn invoke_offset(b: &Built, offset: u32) -> Result<(), PlatformError> {
    let inv = CMINVOKECOMMANDINFOEX {
        cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>() as u32,
        fMask: CMIC_MASK_UNICODE,
        hwnd: HWND(b.hwnd_owner),
        lpVerb: ::windows::core::PCSTR(offset as usize as *const u8),
        lpVerbW: PCWSTR(offset as usize as *const u16),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    b.icm
        .InvokeCommand(&inv as *const _ as *const CMINVOKECOMMANDINFO)
        .map_err(|e| PlatformError::Failed(format!("InvokeCommand: {e}")))
}

/// HMENU 열거 → 항목 트리(서브메뉴는 `HandleMenuMsg(WM_INITMENUPOPUP)` 뒤 · 아이콘 = `hbmpItem`).
unsafe fn enumerate(icm: &IContextMenu, hmenu: HMENU, depth: u32) -> Vec<ShellMenuItem> {
    let mut out = Vec::new();
    let icm2: Option<IContextMenu2> = icm.cast().ok();
    // 최상위 메뉴도 "열리기 직전" 통지를 준다 — 셸 기본 메뉴(DefCM)와 확장은 이때 항목 비트맵·상태를 채운다(dir2는 표시 중
    // WM_INITMENUPOPUP을 포워딩 · SHELL-011). 서브메뉴는 아래에서 항목별로.
    if depth == 0 {
        if let Some(i2) = &icm2 {
            let _ = i2.HandleMenuMsg(WM_INITMENUPOPUP, WPARAM(hmenu.0 as usize), LPARAM(0));
        }
    }
    let n = GetMenuItemCount(Some(hmenu)).max(0);
    for pos in 0..n {
        let mut buf = [0u16; 256];
        let mut mii = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_ID | MIIM_STRING | MIIM_SUBMENU | MIIM_FTYPE | MIIM_STATE | MIIM_BITMAP,
            dwTypeData: PWSTR(buf.as_mut_ptr()),
            cch: buf.len() as u32,
            ..Default::default()
        };
        if GetMenuItemInfoW(hmenu, pos as u32, true, &mut mii).is_err() {
            continue;
        }
        if mii.fType.contains(MFT_SEPARATOR) {
            out.push(ShellMenuItem {
                separator: true,
                ..Default::default()
            });
            continue;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let label = String::from_utf16_lossy(&buf[..len]).replace('&', "");
        if label.trim().is_empty() {
            continue;
        }
        let enabled = !mii.fState.contains(MFS_DISABLED) && !mii.fState.contains(MFS_GRAYED);
        let mut children = Vec::new();
        if !mii.hSubMenu.is_invalid() && depth < MAX_DEPTH {
            if let Some(i2) = &icm2 {
                let _ = i2.HandleMenuMsg(
                    WM_INITMENUPOPUP,
                    WPARAM(mii.hSubMenu.0 as usize),
                    LPARAM(pos as isize),
                );
            }
            children = enumerate(icm, mii.hSubMenu, depth + 1);
        }
        let id = mii.wID;
        let verb = if (ID_FIRST..=ID_LAST).contains(&id) && children.is_empty() {
            verb_of(icm, id - ID_FIRST).unwrap_or_default()
        } else {
            String::new()
        };
        out.push(ShellMenuItem {
            id: format!("shell:{id}"),
            label,
            enabled,
            verb,
            children,
            separator: false,
            icon: bitmap_icon(mii.hbmpItem),
        });
    }
    // 앞뒤·연속 구분자 정리.
    let mut cleaned: Vec<ShellMenuItem> = Vec::new();
    for it in out {
        if it.separator && cleaned.last().is_none_or(|l| l.separator) {
            continue;
        }
        cleaned.push(it);
    }
    while cleaned.last().is_some_and(|l| l.separator) {
        cleaned.pop();
    }
    cleaned
}

unsafe fn verb_of(icm: &IContextMenu, offset: u32) -> Option<String> {
    let mut buf = [0u16; 256];
    icm.GetCommandString(
        offset as usize,
        GCS_VERBW,
        None,
        PSTR(buf.as_mut_ptr() as *mut u8),
        buf.len() as u32,
    )
    .ok()?;
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// 메뉴 항목 비트맵 → RGBA 아이콘. `hbmpItem`의 예약 값(없음 0 · `HBMMENU_CALLBACK` -1 · 시스템 1~11)은 아이콘이 아니다.
unsafe fn bitmap_icon(hbmp: HBITMAP) -> Option<ShellIcon> {
    // 핸들 값은 부호 확장돼 음수일 수 있다(10-03 실측 — `Code(으)로 열기` = -1845163910) → 예약 값 구간(-1..=11)만 거른다.
    if (-1..=11).contains(&(hbmp.0 as isize)) {
        return None;
    }
    let mut bm = BITMAP::default();
    if GetObjectW(
        hbmp.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut BITMAP as *mut core::ffi::c_void),
    ) == 0
    {
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
    if w <= 0 || h <= 0 || w > ICON_MAX || h > ICON_MAX {
        return None;
    }
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // 위에서 아래로
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut px = vec![0u8; (w * h * 4) as usize];
    let hdc = GetDC(None);
    let lines = GetDIBits(
        hdc,
        hbmp,
        0,
        h as u32,
        Some(px.as_mut_ptr() as *mut core::ffi::c_void),
        &mut bi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, hdc);
    if lines == 0 {
        return None;
    }
    Some(ShellIcon {
        w: w as u32,
        h: h as u32,
        rgba: bgra_to_rgba(&px),
    })
}

/// GDI DIB(BGRA · **프리멀티플라이드 알파**) → straight RGBA. 알파가 전부 0이면(알파 채널 없는 옛 비트맵) 불투명으로 본다.
fn bgra_to_rgba(px: &[u8]) -> Vec<u8> {
    let (quads, _) = px.as_chunks::<4>();
    let opaque = quads.iter().all(|p| p[3] == 0);
    let mut out = Vec::with_capacity(px.len());
    for p in quads {
        let (b, g, r, a) = (p[0], p[1], p[2], p[3]);
        if opaque {
            out.extend_from_slice(&[r, g, b, 255]);
        } else if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let un = |c: u8| ((u32::from(c) * 255) / u32::from(a)).min(255) as u8;
            out.extend_from_slice(&[un(r), un(g), un(b), a]);
        }
    }
    out
}

/// 메뉴 스레드 본체: 명령 수신(밀린 선행 구축은 마지막 것만) + STA 메시지 펌프.
fn run(rx: Receiver<Cmd>, tx: Sender<Evt>) {
    // SAFETY: 이 스레드의 COM 초기화(STA) — 스레드 수명 동안 유지(프로세스 수명 데몬).
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let mut w = Worker { built: None };
    loop {
        match rx.recv_timeout(Duration::from_millis(PUMP_MS)) {
            Ok(first) => {
                let mut cmds = vec![first];
                while let Ok(c) = rx.try_recv() {
                    cmds.push(c);
                }
                let last_prepare = cmds.iter().rposition(|c| matches!(c, Cmd::Prepare { .. }));
                for (i, c) in cmds.into_iter().enumerate() {
                    let evt = match c {
                        Cmd::Quit => return,
                        Cmd::Release => {
                            w.built = None;
                            continue;
                        }
                        Cmd::Prepare { target, owner } => {
                            // 밀린 선행 구축은 건너뛴다(확장 실행 비용 0.6~1.4 s) — 기다리는 쪽이 있을 수 있어 통지는 한다.
                            let result = if Some(i) == last_prepare {
                                w.prepare(&target, owner).map_err(|e| e.to_string())
                            } else {
                                Err("superseded".to_string())
                            };
                            Evt::Items { target, result }
                        }
                        Cmd::Invoke { id, target, owner } => {
                            let result = w.invoke(&id, &target, owner).map_err(|e| e.to_string());
                            Evt::Invoked { target, result }
                        }
                    };
                    if tx.send(evt).is_err() {
                        return;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // SAFETY: 이 스레드의 메시지 큐만 비운다(STA 규약 · 확장이 숨은 창을 쓰는 경우).
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

pub(super) struct NativeShellMenu {
    owner: Cell<isize>,
    tx: Sender<Cmd>,
    rx: Receiver<Evt>,
    /// 마지막으로 받은 항목 목록(대상 일치 = 즉시 표시 · FS 변경 시 `invalidate`).
    cache: RefCell<Option<(MenuTarget, Vec<ShellMenuItem>)>>,
    /// 구축 중인 대상(중복 요청 방지).
    inflight: RefCell<Option<MenuTarget>>,
    /// 진행 중인 비동기 실행 수.
    invoking: Cell<u32>,
    /// 호스트가 `poll`로 거둘 사건.
    queue: RefCell<VecDeque<MenuEvent>>,
}

impl NativeShellMenu {
    pub(super) fn new() -> Self {
        let (tx, crx) = mpsc::channel::<Cmd>();
        let (etx, rx) = mpsc::channel::<Evt>();
        // 스폰 실패(자원 고갈)면 채널이 끊겨 동기 호출이 오류를 돌려준다 — 앱은 자체 항목만 보인다.
        let _ = std::thread::Builder::new()
            .name("ndir-ctxmenu".into())
            .spawn(move || run(crx, etx));
        NativeShellMenu {
            owner: Cell::new(0),
            tx,
            rx,
            cache: RefCell::new(None),
            inflight: RefCell::new(None),
            invoking: Cell::new(0),
            queue: RefCell::new(VecDeque::new()),
        }
    }

    /// 메뉴 스레드 사건 1건 반영(캐시 · 진행 표시 · 호스트 큐).
    fn absorb(&self, evt: Evt) {
        match evt {
            Evt::Items { target, result } => {
                if self.inflight.borrow().as_ref() == Some(&target) {
                    *self.inflight.borrow_mut() = None;
                }
                let items = match result {
                    Ok(items) => {
                        *self.cache.borrow_mut() = Some((target.clone(), items.clone()));
                        items
                    }
                    Err(_) => Vec::new(),
                };
                self.queue
                    .borrow_mut()
                    .push_back(MenuEvent::Items { target, items });
            }
            Evt::Invoked { target, result } => {
                self.invoking.set(self.invoking.get().saturating_sub(1));
                self.queue
                    .borrow_mut()
                    .push_back(MenuEvent::Invoked { target, result });
            }
        }
    }

    fn drain(&self) {
        while let Ok(evt) = self.rx.try_recv() {
            self.absorb(evt);
        }
    }

    fn send_prepare(&self, target: &MenuTarget) {
        if self
            .tx
            .send(Cmd::Prepare {
                target: target.clone(),
                owner: self.owner.get(),
            })
            .is_ok()
        {
            *self.inflight.borrow_mut() = Some(target.clone());
        }
    }

    fn cached(&self, target: &MenuTarget) -> Option<Vec<ShellMenuItem>> {
        self.cache
            .borrow()
            .as_ref()
            .filter(|(t, _)| t == target)
            .map(|(_, items)| items.clone())
    }

    /// 동기 조회 — 항상 새로 구축해 그 결과를 기다린다(자가 점검 · 시험 · 폴백).
    fn wait_items(&self, target: &MenuTarget) -> Result<Vec<ShellMenuItem>, PlatformError> {
        self.send_prepare(target);
        // 같은 대상의 통지가 여럿일 수 있다(앞선 선행 구축) — 마지막으로 보낸 것의 답 = 채널 순서상 뒤에 온다. 밀린 답("superseded")은 건너뛴다.
        loop {
            let evt = self
                .rx
                .recv_timeout(SYNC_WAIT)
                .map_err(|e| PlatformError::Failed(format!("menu thread: {e}")))?;
            let hit = match &evt {
                Evt::Items { target: t, result } if t == target => match result {
                    Err(e) if e == "superseded" => None,
                    Ok(items) => Some(Ok(items.clone())),
                    Err(e) => Some(Err(PlatformError::Failed(e.clone()))),
                },
                _ => None,
            };
            self.absorb(evt);
            if let Some(r) = hit {
                // 동기 호출의 답은 호스트 큐에 남기지 않는다(같은 대상의 Items).
                self.queue
                    .borrow_mut()
                    .retain(|e| !matches!(e, MenuEvent::Items { target: t, .. } if t == target));
                return r;
            }
        }
    }

    /// 동기 실행 — 메뉴 스레드에 시키고 결과를 기다린다.
    fn wait_invoke(&self, id: &str, target: &MenuTarget) -> Result<Option<PathBuf>, PlatformError> {
        offset_of(id)?;
        self.tx
            .send(Cmd::Invoke {
                id: id.to_string(),
                target: target.clone(),
                owner: self.owner.get(),
            })
            .map_err(|e| PlatformError::Failed(format!("menu thread: {e}")))?;
        loop {
            let evt = self
                .rx
                .recv_timeout(SYNC_WAIT)
                .map_err(|e| PlatformError::Failed(format!("menu thread: {e}")))?;
            match evt {
                Evt::Invoked { target: t, result } if t == *target => {
                    return result.map_err(PlatformError::Failed);
                }
                other => self.absorb(other),
            }
        }
    }
}

impl Drop for NativeShellMenu {
    fn drop(&mut self) {
        // 스레드는 기다리지 않는다(확장 안에서 멈춰 있을 수 있다) — 채널이 끊기면 스스로 끝난다.
        let _ = self.tx.send(Cmd::Quit);
    }
}

impl ContextMenuProvider for NativeShellMenu {
    fn set_owner(&self, hwnd: isize) {
        self.owner.set(hwnd);
    }

    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        self.wait_items(&MenuTarget::Rows(paths.to_vec()))
    }

    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        self.wait_invoke(id, &MenuTarget::Rows(paths.to_vec()))
            .map(|_| ())
    }

    fn bg_items(&self, dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        self.wait_items(&MenuTarget::Bg(dir.to_path_buf()))
    }

    fn invoke_bg(&self, id: &str, dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        self.wait_invoke(id, &MenuTarget::Bg(dir.to_path_buf()))
    }

    fn prepare(&self, target: &MenuTarget) {
        self.drain();
        if self.cached(target).is_some() || self.inflight.borrow().as_ref() == Some(target) {
            return;
        }
        self.send_prepare(target);
    }

    fn try_items(&self, target: &MenuTarget) -> Option<Vec<ShellMenuItem>> {
        self.drain();
        if let Some(items) = self.cached(target) {
            return Some(items);
        }
        if self.inflight.borrow().as_ref() != Some(target) {
            self.send_prepare(target);
        }
        // 스레드가 없으면(스폰 실패) 기다릴 것이 없다 — 자체 항목만.
        if self.inflight.borrow().is_none() {
            return Some(Vec::new());
        }
        None
    }

    fn invoke_async(&self, id: &str, target: &MenuTarget) -> bool {
        let sent = offset_of(id).map_err(|e| e.to_string()).and_then(|_| {
            self.tx
                .send(Cmd::Invoke {
                    id: id.to_string(),
                    target: target.clone(),
                    owner: self.owner.get(),
                })
                .map_err(|e| format!("menu thread: {e}"))
        });
        match sent {
            Ok(()) => self.invoking.set(self.invoking.get() + 1),
            Err(e) => self.queue.borrow_mut().push_back(MenuEvent::Invoked {
                target: target.clone(),
                result: Err(e),
            }),
        }
        true
    }

    fn poll(&self) -> Option<MenuEvent> {
        self.drain();
        self.queue.borrow_mut().pop_front()
    }

    fn invalidate(&self) {
        *self.cache.borrow_mut() = None;
    }

    fn release(&self) {
        let _ = self.tx.send(Cmd::Release);
    }

    fn busy(&self) -> bool {
        self.inflight.borrow().is_some()
            || self.invoking.get() > 0
            || !self.queue.borrow().is_empty()
    }
}

/// 폴더의 항목 이름 스냅샷(생성 감지용 · dir2 `dir_names`).
fn dir_names(dir: &Path) -> std::collections::HashSet<std::ffi::OsString> {
    std::fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.file_name()).collect())
        .unwrap_or_default()
}

/// invoke 뒤 신규 항목 감지 — **정확히 1개**일 때만(압축 해제 등 다건 오탐 방지) · `retries`×20ms(생성이 늦게 보이는 경우).
fn detect_created(
    dir: &Path,
    before: &std::collections::HashSet<std::ffi::OsString>,
    retries: u32,
) -> Option<PathBuf> {
    for i in 0..=retries {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let now = dir_names(dir);
        let mut fresh = now.difference(before);
        if let Some(first) = fresh.next() {
            if fresh.next().is_none() {
                return Some(dir.join(first));
            }
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제 셸: 임시 .txt의 메뉴에 항목이 있고 verb `open`/`delete`/`copy` 중 하나는 보인다(CI Windows 러너 포함).
    #[test]
    fn shell_menu_lists_items_for_temp_file() {
        let _g = crate::platform::os_test_guard();
        let dir = std::env::temp_dir().join(format!("ndir-shellmenu-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("a.txt");
        std::fs::write(&f, b"x").unwrap();
        let m = NativeShellMenu::new();
        let items = m.items(std::slice::from_ref(&f)).expect("shell menu");
        assert!(!items.is_empty());
        let verbs: Vec<String> = items.iter().map(|i| i.verb.to_ascii_lowercase()).collect();
        assert!(
            verbs
                .iter()
                .any(|v| matches!(v.as_str(), "open" | "delete" | "copy" | "cut" | "rename")),
            "{verbs:?}"
        );
        assert!(items
            .iter()
            .all(|i| i.separator || i.id.starts_with("shell:")));
        // 아이콘이 있으면 크기·버퍼가 맞는다(있고 없고는 그 PC의 셸 확장에 달렸다).
        for ic in items.iter().filter_map(|i| i.icon.as_ref()) {
            assert!(ic.w > 0 && ic.h > 0 && ic.rgba.len() == (ic.w * ic.h * 4) as usize);
        }
        // 모르는 id · 범위 밖 = 오류(실행 없음).
        assert!(m.invoke("edit.copy", std::slice::from_ref(&f)).is_err());
        assert!(m.invoke("shell:70000", std::slice::from_ref(&f)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 실제 셸: 임시 폴더의 **배경** 메뉴에 항목이 있고(보기·새로 만들기·붙여넣기·속성 …) 서브메뉴가 하나는 채워진다 ·
    /// 생성 감지는 정확히 1개일 때만 · 범위 밖 id = 오류.
    #[test]
    fn background_menu_lists_items_and_detects_single_creation() {
        let _g = crate::platform::os_test_guard();
        let dir = std::env::temp_dir().join(format!("ndir-shellbg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let m = NativeShellMenu::new();
        let items = m.bg_items(&dir).expect("background menu");
        assert!(items.len() >= 3, "{items:?}");
        assert!(
            items.iter().any(|i| !i.children.is_empty()),
            "서브메뉴(보기/정렬/새로 만들기) 하나는 채워진다"
        );
        assert!(m.invoke_bg("shell:70000", &dir).is_err());
        assert!(m.invoke_bg("edit.paste", &dir).is_err());
        let before = dir_names(&dir);
        assert_eq!(detect_created(&dir, &before, 0), None);
        std::fs::write(dir.join("one.txt"), b"").unwrap();
        assert_eq!(detect_created(&dir, &before, 0), Some(dir.join("one.txt")));
        std::fs::write(dir.join("two.txt"), b"").unwrap();
        assert_eq!(
            detect_created(&dir, &before, 0),
            None,
            "2개 이상 = 새로 만들기 아님"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 비차단 경로(dir2 X-61): 첫 `try_items` = None(구축 시작 · UI 안 막힘) → `poll`이 `Items`를 준다 → 같은 대상 재조회 = 즉시(캐시) ·
    /// `prepare`로 미리 구축해 두면 첫 조회부터 즉시 · `invalidate` 뒤에는 다시 구축.
    #[test]
    fn try_items_is_non_blocking_and_prepare_makes_it_instant() {
        let _g = crate::platform::os_test_guard();
        let dir = std::env::temp_dir().join(format!("ndir-shellasync-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let (a, b) = (dir.join("a.txt"), dir.join("b.txt"));
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let m = NativeShellMenu::new();
        let wait = |m: &NativeShellMenu, target: &MenuTarget| {
            let t0 = std::time::Instant::now();
            loop {
                match m.poll() {
                    Some(MenuEvent::Items { target: t, items }) if t == *target => return items,
                    Some(_) => {}
                    None => std::thread::sleep(Duration::from_millis(10)),
                }
                assert!(t0.elapsed() < SYNC_WAIT, "menu thread did not answer");
            }
        };
        let ta = MenuTarget::Rows(vec![a]);
        let t0 = std::time::Instant::now();
        assert!(m.try_items(&ta).is_none(), "첫 조회 = 구축 중");
        assert!(
            t0.elapsed() < Duration::from_millis(200),
            "UI 스레드는 기다리지 않는다"
        );
        assert!(m.busy());
        let items = wait(&m, &ta);
        assert!(!items.is_empty());
        assert_eq!(m.try_items(&ta), Some(items), "같은 대상 = 캐시에서 즉시");
        // 선행 구축.
        let tb = MenuTarget::Rows(vec![b]);
        m.prepare(&tb);
        let items_b = wait(&m, &tb);
        assert_eq!(m.try_items(&tb), Some(items_b));
        assert!(!m.busy());
        m.invalidate();
        assert!(m.try_items(&tb).is_none(), "무효화 뒤에는 다시 구축");
        let _ = wait(&m, &tb);
        // 잘못된 id의 비동기 실행 = 실행 없이 오류 통지.
        assert!(m.invoke_async("shell:70000", &tb));
        assert!(matches!(
            m.poll(),
            Some(MenuEvent::Invoked { result: Err(_), .. })
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GDI DIB → RGBA: 프리멀티플라이드 풀기 · 완전 투명 · 알파 채널 없는 비트맵(전부 0) = 불투명.
    #[test]
    fn bgra_premultiplied_to_straight_rgba() {
        // (B,G,R,A) = (64,32,16,128) → 풀면 (R,G,B) = (31,63,127).
        assert_eq!(
            bgra_to_rgba(&[64, 32, 16, 128, 0, 0, 0, 0, 10, 20, 30, 255]),
            vec![31, 63, 127, 128, 0, 0, 0, 0, 30, 20, 10, 255]
        );
        assert_eq!(
            bgra_to_rgba(&[10, 20, 30, 0, 1, 2, 3, 0]),
            vec![30, 20, 10, 255, 3, 2, 1, 255],
            "알파가 전부 0 = 알파 없는 비트맵 → 불투명"
        );
        assert!(bgra_to_rgba(&[]).is_empty());
    }
}

/// `shell:` 별칭 → 폴더 경로(dir2 shellpath.rs:16-45): 셸의 정식 해석기 `SHParseDisplayName`에 맡긴다 — KnownFolders에 등록된
/// 이름 전부(앞으로 OS가 추가하는 것까지)를 표 없이 받는다. 모르는 이름 · 파일 시스템 경로가 없는 가상 폴더 = `None`.
pub(crate) fn resolve_shell_alias(input: &str) -> Option<PathBuf> {
    use ::windows::Win32::UI::Shell::{SHGetPathFromIDListEx, GPFIDL_DEFAULT};
    let wide: Vec<u16> = input
        .trim()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: 셸 COM — 이미 초기화된 STA면 S_FALSE(무시) · `wide`는 NUL로 끝나고 호출 동안 살아 있다 · PIDL은 CoTaskMemFree.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        if SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None).is_err() {
            return None;
        }
        let mut buf = [0u16; 1024];
        let ok = SHGetPathFromIDListEx(pidl, &mut buf, GPFIDL_DEFAULT).as_bool();
        CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
        if !ok {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(PathBuf::from(String::from_utf16_lossy(&buf[..end])))
    }
}

#[cfg(test)]
mod alias_tests {
    use super::resolve_shell_alias;

    /// dir2 shellpath.rs 시험 이식: 대표 이름 · 대소문자 무시 · 모르는 이름 = None.
    #[test]
    fn resolves_known_shell_names() {
        let startup = resolve_shell_alias("shell:startup").expect("startup");
        assert!(
            startup
                .to_string_lossy()
                .to_lowercase()
                .ends_with("startup"),
            "{startup:?}"
        );
        let dl = resolve_shell_alias("Shell:Downloads").expect("downloads");
        assert!(
            dl.to_string_lossy().to_lowercase().contains("downloads"),
            "{dl:?}"
        );
        assert_eq!(resolve_shell_alias("shell:no-such-folder-xyz"), None);
    }
}
