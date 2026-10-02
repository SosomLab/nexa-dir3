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

/// `start`로 연결 프로그램 · `explorer /select,`로 보기.
pub(super) fn opener() -> CommandOpener {
    CommandOpener {
        open: ["cmd.exe", "/C", "start", "", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        reveal: ["explorer.exe", "/select,{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
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
}

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

/// DROPFILES HGLOBAL 만들기(소유권은 SetClipboardData로 넘긴다 · 실패 시 GlobalFree).
fn hdrop_global(paths: &[PathBuf]) -> Option<*mut c_void> {
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
            let n = DragQueryFileW(h, u32::MAX, std::ptr::null_mut(), 0);
            let mut out = Vec::with_capacity(n as usize);
            for i in 0..n {
                let len = DragQueryFileW(h, i, std::ptr::null_mut(), 0);
                let mut buf = vec![0u16; len as usize + 1];
                let got = DragQueryFileW(h, i, buf.as_mut_ptr(), buf.len() as u32);
                buf.truncate(got as usize);
                out.push(PathBuf::from(std::ffi::OsString::from_wide(&buf)));
            }
            Some((out, cut))
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
}
