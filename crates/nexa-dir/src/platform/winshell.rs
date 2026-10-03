//! Windows 셸 컨텍스트 메뉴(T-51 B · dir2 `shellmenu.rs` 축약 이식 — docs/port/19 SHELL-001~002·005·006·012·013):
//! 경로 → PIDL(`SHParseDisplayName`) → 공통 부모 `IShellFolder`(`SHBindToParent`) → `IContextMenu`(`GetUIObjectOf`) →
//! `QueryContextMenu`로 받은 HMENU를 **열거해 앱 메뉴 항목으로 번역**(dir2는 네이티브 HMENU를 띄웠다 — dir3는 nexa-ctl `ContextMenu`가 그린다).
//! 서브메뉴는 `IContextMenu2::HandleMenuMsg(WM_INITMENUPOPUP)`로 채운 뒤 2단까지 열거(보내기 · 연결 프로그램). verb(`GetCommandString`)는
//! 호출부가 가로채기(cut/copy/paste/delete/rename/copyaspath → 앱 경로)에 쓴다. 선택 = `InvokeCommand`(UI 스레드 · 포그라운드 양도는 후속).
//! `windows` crate(DR-8 허용 목록 OS 바인딩). 마지막으로 만든 메뉴(COM 객체 · PIDL · HMENU)는 다음 `items`/`invoke`까지 보유.
//! 배경 메뉴(SHELL-009 · T-51 B-2): `SHGetDesktopFolder` → `BindToObject` → `CreateViewObject::<IContextMenu>` — 실행 뒤 폴더 이름 diff로
//! **정확히 1개** 신규면 생성 경로를 보고(dir2 `detect_created` · 20ms×10 재시도) → 호스트가 선택 + 인라인 이름 바꾸기.

use super::*;
use ::windows::core::{Interface, PCWSTR, PSTR, PWSTR};
use ::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use ::windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_APARTMENTTHREADED};
use ::windows::Win32::UI::Shell::Common::ITEMIDLIST;
use ::windows::Win32::UI::Shell::{
    IContextMenu, IContextMenu2, IShellFolder, SHBindToParent, SHGetDesktopFolder,
    SHParseDisplayName, CMF_NORMAL, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, GCS_VERBW,
};
use ::windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetMenuItemCount, GetMenuItemInfoW, HMENU, MENUITEMINFOW,
    MFS_DISABLED, MFS_GRAYED, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING,
    MIIM_SUBMENU, SW_SHOWNORMAL, WM_INITMENUPOPUP,
};
use std::cell::RefCell;
use std::os::windows::ffi::OsStrExt as _;

const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x6FFF;
const CMIC_MASK_UNICODE: u32 = 0x4000;
/// 서브메뉴 열거 깊이(보내기 ▸ · 연결 프로그램 ▸).
const MAX_DEPTH: u32 = 2;

/// 메뉴 종류(행 선택 · 폴더 배경).
#[derive(Clone, PartialEq, Eq)]
enum Key {
    Rows(Vec<PathBuf>),
    Bg(PathBuf),
}

/// 만든 메뉴 1벌(COM 수명 = 이 구조체).
struct Built {
    key: Key,
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

pub(super) struct NativeShellMenu {
    owner: std::cell::Cell<isize>,
    built: RefCell<Option<Built>>,
}

impl NativeShellMenu {
    pub(super) fn new() -> Self {
        NativeShellMenu {
            owner: std::cell::Cell::new(0),
            built: RefCell::new(None),
        }
    }

    fn hwnd(&self) -> HWND {
        HWND(self.owner.get() as *mut core::ffi::c_void)
    }

    /// 경로들 → IContextMenu + HMENU(QueryContextMenu). 접근 불가 항목은 제외 · 부모가 다른 항목은 첫 부모 기준으로 축소(SHELL-002).
    unsafe fn build(&self, paths: &[PathBuf]) -> Result<Built, PlatformError> {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED); // 이미 초기화돼 있으면 S_FALSE/RPC_E_CHANGED_MODE — 무시.
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
        let icm = match folder.GetUIObjectOf::<IContextMenu>(self.hwnd(), &children, None) {
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
        let hr = icm.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, CMF_NORMAL);
        if hr.is_err() {
            let _ = DestroyMenu(hmenu);
            free(&pidls);
            return Err(PlatformError::Failed(format!("QueryContextMenu: {hr}")));
        }
        Ok(Built {
            key: Key::Rows(paths.to_vec()),
            icm,
            hmenu,
            pidls,
            _folder: folder,
            hwnd_owner: self.owner.get() as *mut core::ffi::c_void,
        })
    }

    /// 폴더 배경 메뉴(SHELL-009): 폴더 PIDL → `IShellFolder` → `CreateViewObject::<IContextMenu>` → QueryContextMenu.
    unsafe fn build_bg(&self, dir: &Path) -> Result<Built, PlatformError> {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
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
        let icm = match folder.CreateViewObject::<IContextMenu>(self.hwnd()) {
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
        let hr = icm.QueryContextMenu(hmenu, 0, ID_FIRST, ID_LAST, CMF_NORMAL);
        if hr.is_err() {
            let _ = DestroyMenu(hmenu);
            free();
            return Err(PlatformError::Failed(format!("QueryContextMenu: {hr}")));
        }
        Ok(Built {
            key: Key::Bg(dir.to_path_buf()),
            icm,
            hmenu,
            pidls: vec![pidl],
            _folder: folder,
            hwnd_owner: self.owner.get() as *mut core::ffi::c_void,
        })
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

    /// HMENU 열거 → 항목 트리(서브메뉴는 `HandleMenuMsg(WM_INITMENUPOPUP)` 뒤).
    unsafe fn enumerate(icm: &IContextMenu, hmenu: HMENU, depth: u32) -> Vec<ShellMenuItem> {
        let mut out = Vec::new();
        let n = GetMenuItemCount(Some(hmenu)).max(0);
        let icm2: Option<IContextMenu2> = icm.cast().ok();
        for pos in 0..n {
            let mut buf = [0u16; 256];
            let mut mii = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_ID | MIIM_STRING | MIIM_SUBMENU | MIIM_FTYPE | MIIM_STATE,
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
                children = Self::enumerate(icm, mii.hSubMenu, depth + 1);
            }
            let id = mii.wID;
            let verb = if (ID_FIRST..=ID_LAST).contains(&id) && children.is_empty() {
                Self::verb(icm, id - ID_FIRST).unwrap_or_default()
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

    unsafe fn verb(icm: &IContextMenu, offset: u32) -> Option<String> {
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
}

impl ContextMenuProvider for NativeShellMenu {
    fn set_owner(&self, hwnd: isize) {
        self.owner.set(hwnd);
    }

    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        // SAFETY: COM/셸 API — 입력 포인터는 이 함수 안의 유효 버퍼 · 산출 핸들은 `Built`가 수명 관리.
        let built = unsafe { self.build(paths)? };
        let items = unsafe { Self::enumerate(&built.icm, built.hmenu, 0) };
        *self.built.borrow_mut() = Some(built);
        Ok(items)
    }

    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        let offset = Self::offset_of(id)?;
        let same = self
            .built
            .borrow()
            .as_ref()
            .is_some_and(|b| b.key == Key::Rows(paths.to_vec()));
        if !same {
            // SAFETY: 위와 같음.
            let built = unsafe { self.build(paths)? };
            *self.built.borrow_mut() = Some(built);
        }
        let b = self.built.borrow();
        let Some(b) = b.as_ref() else {
            return Err(PlatformError::Failed("menu not built".into()));
        };
        // SAFETY: icm은 살아 있는 COM 객체 · 오프셋은 범위 검사됨.
        unsafe { Self::invoke_offset(b, offset) }
    }

    fn bg_items(&self, dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        // SAFETY: COM/셸 API — 산출 핸들은 `Built`가 수명 관리.
        let built = unsafe { self.build_bg(dir)? };
        let items = unsafe { Self::enumerate(&built.icm, built.hmenu, 0) };
        *self.built.borrow_mut() = Some(built);
        Ok(items)
    }

    fn invoke_bg(&self, id: &str, dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        let offset = Self::offset_of(id)?;
        let same = self
            .built
            .borrow()
            .as_ref()
            .is_some_and(|b| b.key == Key::Bg(dir.to_path_buf()));
        if !same {
            // SAFETY: 위와 같음.
            let built = unsafe { self.build_bg(dir)? };
            *self.built.borrow_mut() = Some(built);
        }
        let before = dir_names(dir);
        {
            let b = self.built.borrow();
            let Some(b) = b.as_ref() else {
                return Err(PlatformError::Failed("menu not built".into()));
            };
            // SAFETY: 위와 같음.
            unsafe { Self::invoke_offset(b, offset)? };
        }
        Ok(detect_created(dir, &before, 10))
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
        // 모르는 id · 범위 밖 = 오류(실행 없음).
        assert!(m.invoke("edit.copy", std::slice::from_ref(&f)).is_err());
        assert!(m.invoke("shell:70000", std::slice::from_ref(&f)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 실제 셸: 임시 폴더의 **배경** 메뉴에 항목이 있고(보기·새로 만들기·붙여넣기·속성 …) 서브메뉴가 하나는 채워진다 ·
    /// 생성 감지는 정확히 1개일 때만 · 범위 밖 id = 오류.
    #[test]
    fn background_menu_lists_items_and_detects_single_creation() {
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
}
