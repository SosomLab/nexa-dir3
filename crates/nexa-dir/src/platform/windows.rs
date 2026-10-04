//! Windows 구현(T-50·T-51 A): 셸 탐지 · 열기/보기 · 드라이브 용량 · **휴지통(SHFileOperationW)** · **파일 클립보드(CF_HDROP + Preferred DropEffect)**.
//! 셸 메뉴(IContextMenu) · OLE DnD · ConPTY · ReadDirectoryChangesW는 T-51 B(COM vtable 수동 · M5).
//! 외부 crate 0 — kernel32/user32/shell32 수동 extern(nexa-sys 관례). 모든 실패 = `Err(Failed)`(패닉 없음).

use super::*;
use std::ffi::c_void;
use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};

pub(super) struct NativeShell;

impl Shell for NativeShell {
    /// dir2 `term.shell` 규약: `pwsh`(PowerShell 7) → `powershell`(5) → `cmd`(SKEL-425).
    fn candidates(&self) -> Vec<ShellSpec> {
        let mut v = shell_from_names(&[
            ("pwsh.exe", &["-NoLogo"], "PowerShell 7"),
            ("powershell.exe", &["-NoLogo"], "Windows PowerShell"),
        ]);
        // cmd는 PATH에 없어도 시스템 폴더에 있다.
        let sysroot = std::env::var_os("SystemRoot")
            .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        let cmd = sysroot.join("System32").join("cmd.exe");
        if cmd.is_file() {
            v.push(ShellSpec {
                program: cmd,
                args: vec![],
                label: "Command Prompt".into(),
            });
        }
        v
    }
}

/// 열기 = **`ShellExecuteW("open")`**(dir2와 같음 · GAP-010) — 종전 `cmd.exe /C start "" {path}`는 경로가 명령줄로 다시 해석돼
/// `&` `^` `%` 같은 메타문자가 든 이름에서 엉뚱한 명령이 실행될 수 있었다. 셸 API는 경로를 그대로 받는다(.lnk · 연결 프로그램 ·
/// 폴더 모두 탐색기와 같은 동작 · 작업 폴더 = 부모 폴더). 보기 = `explorer /select,`(cmd를 거치지 않는 직접 실행).
pub(super) struct NativeOpener {
    reveal: CommandOpener,
}

pub(super) fn opener() -> NativeOpener {
    NativeOpener {
        reveal: CommandOpener {
            open: Vec::new(),
            reveal: ["explorer.exe", "/select,{path}"]
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        },
    }
}

impl Opener for NativeOpener {
    fn resolve_alias(&self, input: &str) -> Option<PathBuf> {
        super::winshell::resolve_shell_alias(input)
    }

    fn open(&self, path: &Path) -> Result<(), PlatformError> {
        let file = wide(path);
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(wide);
        // SAFETY: 모든 포인터는 이 호출 동안 살아 있는 NUL 종료 UTF-16 버퍼(또는 null) · 창 핸들 0 = 소유 창 없음.
        let rc = unsafe {
            ShellExecuteW(
                0,
                verb.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
                SW_SHOWNORMAL,
            )
        };
        // 반환값 > 32 = 성공(그 이하는 SE_ERR_* 코드).
        if rc > 32 {
            Ok(())
        } else {
            Err(PlatformError::Failed(format!("ShellExecuteW: {rc}")))
        }
    }
    fn reveal(&self, path: &Path) -> Result<(), PlatformError> {
        self.reveal.reveal(path)
    }
    fn link_target(&self, path: &Path) -> Option<PathBuf> {
        shell_link_target(path)
    }
}

/// `.lnk`의 대상 경로(IShellLinkW::GetPath — 대상을 찾아 헤매는 `Resolve`는 부르지 않는다: UI 스레드를 막지 않고 저장된 경로만).
/// 대상이 파일 시스템 경로가 아니면(제어판 항목 등) 빈 문자열 → `None`.
fn shell_link_target(path: &Path) -> Option<PathBuf> {
    use ::windows::core::{Interface, PCWSTR};
    use ::windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use ::windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    let file = wide(path);
    // SAFETY: COM 호출 — UI 스레드(STA · 이미 초기화돼 있으면 S_FALSE) · 버퍼는 호출 동안 살아 있다 · 인터페이스는 Drop에서 Release.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        persist.Load(PCWSTR(file.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        (len > 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
    }
}

fn wide(p: &Path) -> Vec<u16> {
    let mut w: Vec<u16> = p.as_os_str().encode_wide().collect();
    w.push(0);
    w
}

// ── kernel32 / user32 / shell32 ──────────────────────────────────────────

#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(
        dir: *const u16,
        free_to_caller: *mut u64,
        total: *mut u64,
        total_free: *mut u64,
    ) -> i32;
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalLock(h: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(h: *mut c_void) -> i32;
    fn GlobalSize(h: *mut c_void) -> usize;
    fn GlobalFree(h: *mut c_void) -> *mut c_void;
    fn GetLastError() -> u32;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const c_void,
        disposition: u32,
        flags: u32,
        template: *const c_void,
    ) -> isize;
    fn CloseHandle(h: isize) -> i32;
    fn GetLogicalDrives() -> u32;
    fn GetCompressedFileSizeW(name: *const u16, high: *mut u32) -> u32;
    fn GetDiskFreeSpaceW(
        root: *const u16,
        sectors_per_cluster: *mut u32,
        bytes_per_sector: *mut u32,
        free_clusters: *mut u32,
        total_clusters: *mut u32,
    ) -> i32;
    fn GetFileAttributesW(name: *const u16) -> u32;
    fn SetLastError(code: u32);
}

/// 파일 속성: 폴더 · 오프라인(온라인 전용) · 접근하면 내려받는 플레이스홀더(dir2 SHELL-085).
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

/// 디스크 할당 크기를 물어도 되는 대상인가(순수): 속성을 읽지 못함 · 폴더 · 클라우드 온라인 전용(오프라인/플레이스홀더)은 아니다.
pub(super) fn wants_size_on_disk(attrs: u32) -> bool {
    attrs != u32::MAX
        && attrs
            & (FILE_ATTRIBUTE_DIRECTORY
                | FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            == 0
}

#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(hwnd: isize) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(format: u32) -> *mut c_void;
    fn SetClipboardData(format: u32, h: *mut c_void) -> *mut c_void;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
}

#[repr(C)]
struct ShFileOpStructW {
    hwnd: *mut c_void,
    func: u32,
    from: *const u16,
    to: *const u16,
    flags: u16,
    any_aborted: i32,
    name_mappings: *mut c_void,
    progress_title: *const u16,
}

#[link(name = "shell32")]
extern "system" {
    fn SHFileOperationW(op: *mut ShFileOpStructW) -> i32;
    fn DragQueryFileW(hdrop: *mut c_void, index: u32, out: *mut u16, cap: u32) -> u32;
    fn ShellExecuteW(
        hwnd: isize,
        verb: *const u16,
        file: *const u16,
        params: *const u16,
        dir: *const u16,
        show: i32,
    ) -> isize;
}

const SW_SHOWNORMAL: i32 = 1;

const FO_DELETE: u32 = 3;
const FOF_SILENT: u16 = 0x4;
const FOF_NOCONFIRMATION: u16 = 0x10;
const FOF_ALLOWUNDO: u16 = 0x40;
const FOF_NOERRORUI: u16 = 0x400;
const GMEM_MOVEABLE: u32 = 0x2;
const CF_HDROP: u32 = 15;
const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;

pub(super) struct NativeDisk;

impl Disk for NativeDisk {
    fn size_on_disk(&self, path: &Path) -> Option<u64> {
        // 네트워크 경로(`\\server\share`)는 묻지 않는다 — 볼륨 조회가 네트워크 왕복이라 UI 스레드를 멈춘다(dir2 fileinfo.rs:83).
        let text = path.as_os_str().to_string_lossy();
        if text.starts_with("\\\\") {
            return None;
        }
        let w = wide(path);
        // SAFETY: NUL 종단 UTF-16 경로 · 출력 포인터는 살아 있는 지역 변수. GetCompressedFileSizeW는 실패를 0xFFFF_FFFF +
        // GetLastError ≠ 0으로 알린다(정상 값일 수도 있어 호출 전에 오류 코드를 비운다).
        let used = unsafe {
            if !wants_size_on_disk(GetFileAttributesW(w.as_ptr())) {
                return None;
            }
            let mut high = 0u32;
            SetLastError(0);
            let low = GetCompressedFileSizeW(w.as_ptr(), &mut high);
            if low == u32::MAX && GetLastError() != 0 {
                return None;
            }
            (u64::from(high) << 32) | u64::from(low)
        };
        // 클러스터 크기는 볼륨 루트("C:\")에 묻는다.
        let root = path
            .components()
            .next()?
            .as_os_str()
            .to_string_lossy()
            .into_owned();
        let root: Vec<u16> = format!("{root}\\")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let (mut spc, mut bps, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
        // SAFETY: NUL 종단 UTF-16 루트 · 출력 포인터는 살아 있는 지역 변수 · 실패 = 0 반환.
        let ok =
            unsafe { GetDiskFreeSpaceW(root.as_ptr(), &mut spc, &mut bps, &mut free, &mut total) };
        let cluster = if ok != 0 {
            u64::from(spc) * u64::from(bps)
        } else {
            0
        };
        Some(super::round_up_cluster(used, cluster))
    }

    fn volumes_stamp(&self) -> u64 {
        // SAFETY: 인자 없는 조회(드라이브 문자 비트맵 · A = 비트 0).
        u64::from(unsafe { GetLogicalDrives() }).max(1)
    }

    fn space(&self, root: &Path) -> Option<(u64, u64)> {
        let w = wide(root);
        let (mut total, mut free) = (0u64, 0u64);
        // SAFETY: NUL 종단 UTF-16 경로 · 출력 포인터는 살아 있는 지역 변수 · 실패 = 0 반환.
        let ok =
            unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
        (ok != 0).then_some((total, free))
    }
}

/// 휴지통 = `SHFileOperationW(FO_DELETE | FOF_ALLOWUNDO)`(dir2 `delete_to_recycle_bin`과 같은 호출 · 확인창·진행창 없음).
pub(super) struct NativeTrash;

/// 이중 NUL 종단 경로 목록(SHFileOperation 규약).
fn double_null_list(paths: &[PathBuf]) -> Vec<u16> {
    let mut list: Vec<u16> = Vec::new();
    for p in paths {
        list.extend(p.as_os_str().encode_wide());
        list.push(0);
    }
    list.push(0);
    list
}

impl Trash for NativeTrash {
    fn restore(&self, original: &[PathBuf]) -> Result<usize, PlatformError> {
        super::winrecycle::restore_by_original_paths(original)
    }

    /// dir2 `win.rs:3849-3880`: 삭제 권한으로 열어 본다(공유 = 읽기·쓰기·삭제 전부 허용) — **공유 위반**만 "잠김"으로 본다
    /// (없는 파일 · 권한 없음 등 다른 오류는 잠김이 아니다 — 삭제 단계가 따로 알린다). 폴더도 열리게 BACKUP_SEMANTICS.
    fn probe_locked(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        const DELETE: u32 = 0x0001_0000;
        const SHARE_ALL: u32 = 0x1 | 0x2 | 0x4;
        const OPEN_EXISTING: u32 = 3;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const ERROR_SHARING_VIOLATION: u32 = 32;
        const INVALID_HANDLE: isize = -1;
        paths
            .iter()
            .filter(|p| {
                let w = wide(p);
                // SAFETY: NUL로 끝나는 경로 버퍼 · 나머지 인자는 값 · 받은 핸들은 바로 닫는다.
                unsafe {
                    let h = CreateFileW(
                        w.as_ptr(),
                        DELETE,
                        SHARE_ALL,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        FILE_FLAG_BACKUP_SEMANTICS,
                        std::ptr::null(),
                    );
                    if h == INVALID_HANDLE {
                        GetLastError() == ERROR_SHARING_VIOLATION
                    } else {
                        CloseHandle(h);
                        false
                    }
                }
            })
            .cloned()
            .collect()
    }

    fn trash(&self, paths: &[PathBuf]) -> Result<usize, PlatformError> {
        if paths.is_empty() {
            return Ok(0);
        }
        let list = double_null_list(paths);
        let mut op = ShFileOpStructW {
            hwnd: std::ptr::null_mut(),
            func: FO_DELETE,
            from: list.as_ptr(),
            to: std::ptr::null(),
            flags: FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI,
            any_aborted: 0,
            name_mappings: std::ptr::null_mut(),
            progress_title: std::ptr::null(),
        };
        // SAFETY: 구조체·목록은 호출 동안 살아 있다 · 이중 NUL 종단.
        let rc = unsafe { SHFileOperationW(&mut op) };
        if rc == 0 && op.any_aborted == 0 {
            Ok(paths.len())
        } else {
            Err(PlatformError::Failed(format!("SHFileOperationW: {rc}")))
        }
    }
}

/// 파일 클립보드 = CF_HDROP(DROPFILES 헤더 + wide 경로 이중 NUL) + "Preferred DropEffect"(DWORD · MOVE = 잘라내기).
pub(super) struct NativeFileClipboard;

/// `DROPFILES` = pFiles(u32) · pt(i32,i32) · fNC(i32) · fWide(i32) = 20 바이트.
const DROPFILES_LEN: usize = 20;

fn drop_effect_format() -> u32 {
    let name = wide(Path::new("Preferred DropEffect"));
    // SAFETY: NUL 종단 이름.
    unsafe { RegisterClipboardFormatW(name.as_ptr()) }
}

struct ClipGuard;

impl ClipGuard {
    fn open() -> Result<ClipGuard, PlatformError> {
        // ★ 변경 직후엔 클립보드 기록/클라우드 서비스가 잠깐 잡고 있다(10-03 실기: 바로 열면 실패·null) → 5회 · 10 ms 재시도
        //   (nexa-sql `clipboard.rs`와 같은 규약).
        for attempt in 0..10 {
            // SAFETY: 소유 창 없음(0) — 읽기/쓰기 모두 허용.
            if unsafe { OpenClipboard(0) } != 0 {
                return Ok(ClipGuard);
            }
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        // SAFETY: 마지막 오류 코드 조회.
        Err(PlatformError::Failed(format!(
            "OpenClipboard: {}",
            unsafe { GetLastError() }
        )))
    }
}

impl Drop for ClipGuard {
    fn drop(&mut self) {
        // SAFETY: open과 짝.
        unsafe {
            CloseClipboard();
        }
    }
}

/// DROPFILES HGLOBAL → 경로 목록(클립보드 읽기 · 드래그 데이터 객체 시험 공용).
///
/// # Safety
/// `h`는 유효한 CF_HDROP 블록(DROPFILES 헤더 + 이중 NUL 목록)이어야 한다.
pub(super) unsafe fn paths_from_hdrop_global(h: *mut c_void) -> Vec<PathBuf> {
    let n = DragQueryFileW(h, u32::MAX, std::ptr::null_mut(), 0);
    let mut out = Vec::with_capacity(n as usize);
    for i in 0..n {
        let len = DragQueryFileW(h, i, std::ptr::null_mut(), 0);
        let mut buf = vec![0u16; len as usize + 1];
        let got = DragQueryFileW(h, i, buf.as_mut_ptr(), buf.len() as u32);
        buf.truncate(got as usize);
        out.push(PathBuf::from(std::ffi::OsString::from_wide(&buf)));
    }
    out
}

/// DROPFILES HGLOBAL 만들기(소유권은 SetClipboardData로 넘긴다 · 실패 시 GlobalFree).
pub(super) fn hdrop_global(paths: &[PathBuf]) -> Option<*mut c_void> {
    let list = double_null_list(paths);
    let total = DROPFILES_LEN + list.len() * 2;
    // SAFETY: GMEM_MOVEABLE 블록 할당 → 잠금 → 헤더·목록 복사 → 해제.
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, total);
        if h.is_null() {
            return None;
        }
        let base = GlobalLock(h) as *mut u8;
        if base.is_null() {
            GlobalFree(h);
            return None;
        }
        std::ptr::write_bytes(base, 0, DROPFILES_LEN);
        std::ptr::write_unaligned(base as *mut u32, DROPFILES_LEN as u32); // pFiles
        std::ptr::write_unaligned(base.add(16) as *mut i32, 1); // fWide
        std::ptr::copy_nonoverlapping(
            list.as_ptr() as *const u8,
            base.add(DROPFILES_LEN),
            list.len() * 2,
        );
        GlobalUnlock(h);
        Some(h)
    }
}

fn dword_global(v: u32) -> Option<*mut c_void> {
    // SAFETY: 4바이트 블록.
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, 4);
        if h.is_null() {
            return None;
        }
        let p = GlobalLock(h) as *mut u32;
        if p.is_null() {
            GlobalFree(h);
            return None;
        }
        std::ptr::write_unaligned(p, v);
        GlobalUnlock(h);
        Some(h)
    }
}

impl NativeFileClipboard {
    fn read_once(&self) -> Option<(Vec<PathBuf>, bool)> {
        // SAFETY: 형식 가용 확인 → 열기 → HDROP 질의(개수·각 경로) → 효과 DWORD 읽기 → 닫기(가드).
        unsafe {
            let _g = ClipGuard::open().ok()?;
            // ★ 효과 DWORD를 HDROP보다 **먼저** 읽는다(10-03 실기: HDROP를 읽은 뒤에는 등록 형식 GetClipboardData가 null · ERROR_CLIPBOARD_NOT_OPEN 1418).
            let mut cut = false;
            let eff = GetClipboardData(drop_effect_format());
            if !eff.is_null() && GlobalSize(eff) >= 4 {
                let p = GlobalLock(eff) as *const u32;
                if !p.is_null() {
                    cut = std::ptr::read_unaligned(p) & DROPEFFECT_MOVE != 0;
                    GlobalUnlock(eff);
                }
            }
            let h = GetClipboardData(CF_HDROP);
            if h.is_null() {
                return None;
            }
            Some((paths_from_hdrop_global(h), cut))
        }
    }
}

impl FileClipboard for NativeFileClipboard {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        // ★ 쓰기 직후 ~25 ms 동안은 열려도 HDROP가 null(클립보드 기록 서비스가 다시 렌더 · 10-03 실기 23 ms) → 짧게 재시도.
        for attempt in 0..3 {
            if let Some(r) = self.read_once() {
                return Some(r);
            }
            if attempt < 2 {
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
        None
    }

    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError> {
        let hdrop =
            hdrop_global(paths).ok_or_else(|| PlatformError::Failed("GlobalAlloc".into()))?;
        let eff = dword_global(if cut {
            DROPEFFECT_MOVE
        } else {
            DROPEFFECT_COPY
        })
        .ok_or_else(|| PlatformError::Failed("GlobalAlloc".into()))?;
        // SAFETY: 열기 → 비우기 → 두 형식 넣기(성공 시 소유권 이전) → 닫기.
        unsafe {
            let _g = ClipGuard::open()?;
            EmptyClipboard();
            if SetClipboardData(CF_HDROP, hdrop).is_null() {
                GlobalFree(hdrop);
                GlobalFree(eff);
                return Err(PlatformError::Failed("SetClipboardData(CF_HDROP)".into()));
            }
            if SetClipboardData(drop_effect_format(), eff).is_null() {
                GlobalFree(eff);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 잠금 확인(WINB-024): 공유 없이 열어 둔 파일 = 잠김 · 닫으면 풀림 · 없는 파일 · 그냥 있는 파일 = 잠김 아님.
    #[test]
    fn probe_locked_reports_only_sharing_violations() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = std::env::temp_dir().join(format!("ndir-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let (held, free, gone) = (dir.join("held.txt"), dir.join("free.txt"), dir.join("gone"));
        std::fs::write(&held, b"x").expect("write");
        std::fs::write(&free, b"x").expect("write");
        let all = [held.clone(), free, gone, dir.clone()];
        let guard = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0) // 아무와도 공유하지 않는다 = 다른 프로그램이 쓰는 중
            .open(&held)
            .expect("exclusive open");
        assert_eq!(NativeTrash.probe_locked(&all), vec![held]);
        drop(guard);
        assert!(NativeTrash.probe_locked(&all).is_empty(), "닫으면 풀린다");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn double_null_list_layout() {
        let l = double_null_list(&[PathBuf::from("C:\\a"), PathBuf::from("D:\\b")]);
        let s: Vec<char> = l
            .iter()
            .map(|&u| char::from_u32(u32::from(u)).unwrap())
            .collect();
        assert_eq!(s.iter().collect::<String>(), "C:\\a\0D:\\b\0\0");
        assert_eq!(double_null_list(&[]), vec![0]);
    }

    /// 실제 클립보드·휴지통을 건드린다 → `cargo test -- --ignored`(자가 점검 `--with-clipboard`와 같은 opt-in).
    #[test]
    #[ignore]
    fn clipboard_roundtrip_and_trash_real() {
        let dir = std::env::temp_dir().join(format!("ndir-win-clip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("x.txt");
        std::fs::write(&f, b"1").unwrap();
        let c = NativeFileClipboard;
        c.write_files(std::slice::from_ref(&f), true).unwrap();
        // 쓰기 직후엔 클립보드 기록 서비스가 잠깐 잡는다 → 1 s까지 폴링(걸린 시간을 출력).
        let t0 = std::time::Instant::now();
        let mut got = None;
        while t0.elapsed() < std::time::Duration::from_secs(1) {
            got = c.read_files();
            if got.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        eprintln!("[clip] readable after {:?}", t0.elapsed());
        assert_eq!(got, Some((vec![f.clone()], true)));
        assert_eq!(NativeTrash.trash(std::slice::from_ref(&f)), Ok(1));
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-007: 임시 폴더에 실제 `.lnk`를 만들어(IShellLinkW::SetPath + IPersistFile::Save) 대상 경로를 되읽는다 ·
    /// `.lnk`가 아닌 파일 = `None`. 사용자 파일은 건드리지 않는다.
    #[test]
    fn shell_link_target_reads_a_real_lnk() {
        use ::windows::core::{Interface, PCWSTR};
        use ::windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        };
        use ::windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
        let dir = std::env::temp_dir().join(format!("ndir-lnk-{}", std::process::id()));
        let target = dir.join("대상 폴더");
        std::fs::create_dir_all(&target).unwrap();
        let lnk = dir.join("to.lnk");
        // SAFETY: 시험 스레드에서 COM 초기화 뒤 표준 호출 · 버퍼는 호출 동안 살아 있다.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).expect("ShellLink");
            link.SetPath(PCWSTR(wide(&target).as_ptr()))
                .expect("SetPath");
            let persist: IPersistFile = link.cast().expect("IPersistFile");
            persist
                .Save(PCWSTR(wide(&lnk).as_ptr()), true)
                .expect("Save");
        }
        // 경로 문자열은 표기가 다를 수 있다(CI 러너의 TEMP = 8.3 짧은 이름 `RUNNER~1` · 바로 가기는 긴 이름을 저장) → 양쪽 다
        // 정규화해 같은 폴더인지로 비교(지우기 전에 계산).
        let got = shell_link_target(&lnk).map(|p| std::fs::canonicalize(p).expect("대상 존재"));
        let want = std::fs::canonicalize(&target).expect("canonicalize");
        let plain = dir.join("plain.txt");
        std::fs::write(&plain, b"not a link").unwrap();
        let none = shell_link_target(&plain);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(got, Some(want));
        assert_eq!(none, None);
    }
}
