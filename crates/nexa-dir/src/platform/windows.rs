//! Windows 구현(T-50 몫: 셸 탐지 · 열기/보기 · 드라이브 용량). 셸 메뉴·휴지통·CF_HDROP·OLE DnD·ConPTY·감시는 T-51.
//! 외부 crate 0 — kernel32 수동 extern(nexa-sys 관례).

use super::*;

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

pub(super) struct NativeDisk;

#[link(name = "kernel32")]
extern "system" {
    fn GetDiskFreeSpaceExW(
        dir: *const u16,
        free_to_caller: *mut u64,
        total: *mut u64,
        total_free: *mut u64,
    ) -> i32;
}

impl Disk for NativeDisk {
    fn space(&self, root: &Path) -> Option<(u64, u64)> {
        use std::os::windows::ffi::OsStrExt as _;
        let mut w: Vec<u16> = root.as_os_str().encode_wide().collect();
        w.push(0);
        let (mut total, mut free) = (0u64, 0u64);
        // SAFETY: NUL 종단 UTF-16 경로 · 출력 포인터는 살아 있는 지역 변수 · 실패 = 0 반환(에러 코드는 안 본다).
        let ok =
            unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
        (ok != 0).then_some((total, free))
    }
}
