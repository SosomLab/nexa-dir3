//! Unix PTY(T-61 · macOS/Linux 공용): `forkpty` + `execvp` · 비차단 읽기(`O_NONBLOCK`) · `TIOCSWINSZ` · `SIGHUP`/`waitpid`.
//! libc 수동 extern(외부 crate 0 · Linux는 `libutil`). 자식은 `TERM=xterm-256color` · cwd 변경 뒤 exec.

use super::*;
use std::cell::Cell;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_ulong, c_void};

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg_attr(target_os = "linux", link(name = "util"))]
extern "C" {
    fn forkpty(
        amaster: *mut c_int,
        name: *mut c_char,
        termp: *const c_void,
        winp: *const Winsize,
    ) -> c_int;
}

extern "C" {
    fn execvp(file: *const c_char, argv: *const *const c_char) -> c_int;
    fn chdir(path: *const c_char) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn ioctl(fd: c_int, req: c_ulong, ...) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

const F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;
#[cfg(target_os = "linux")]
const O_NONBLOCK: c_int = 0o4000;
#[cfg(not(target_os = "linux"))]
const O_NONBLOCK: c_int = 0x4;
#[cfg(target_os = "linux")]
const TIOCSWINSZ: c_ulong = 0x5414;
#[cfg(not(target_os = "linux"))]
const TIOCSWINSZ: c_ulong = 0x8008_7467;
const WNOHANG: c_int = 1;
const SIGHUP: c_int = 1;
const SIGKILL: c_int = 9;

pub(super) struct ForkPty;

impl Pty for ForkPty {
    fn spawn(
        &self,
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn PtySession>, PlatformError> {
        let c = |s: &std::ffi::OsStr| {
            CString::new(s.as_encoded_bytes())
                .map_err(|_| PlatformError::Failed("NUL in argument".into()))
        };
        let mut argv: Vec<CString> = vec![c(shell.program.as_os_str())?];
        for a in &shell.args {
            argv.push(c(std::ffi::OsStr::new(a))?);
        }
        let argv_ptrs: Vec<*const c_char> = argv
            .iter()
            .map(|a| a.as_ptr())
            .chain(std::iter::once(std::ptr::null()))
            .collect();
        let cwd_c = c(cwd.as_os_str())?;
        let ws = Winsize {
            ws_row: rows.max(2),
            ws_col: cols.max(2),
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let mut master: c_int = -1;
        // SAFETY: 모든 포인터는 이 스택 프레임의 유효한 NUL 종단 버퍼. 자식은 fork 뒤 exec/_exit만 한다.
        let pid = unsafe { forkpty(&mut master, std::ptr::null_mut(), std::ptr::null(), &ws) };
        if pid < 0 {
            return Err(PlatformError::Failed(format!(
                "forkpty: {}",
                std::io::Error::last_os_error()
            )));
        }
        if pid == 0 {
            // SAFETY: 자식 — async-signal-safe 범위의 호출만(chdir·setenv·execvp·_exit).
            unsafe {
                chdir(cwd_c.as_ptr());
                setenv(c"TERM".as_ptr(), c"xterm-256color".as_ptr(), 1);
                execvp(argv_ptrs[0], argv_ptrs.as_ptr());
                _exit(127);
            }
        }
        // SAFETY: master는 방금 받은 열린 fd.
        unsafe {
            let fl = fcntl(master, F_GETFL);
            if fl >= 0 {
                fcntl(master, F_SETFL, fl | O_NONBLOCK);
            }
        }
        Ok(Box::new(ForkPtySession {
            master,
            pid,
            dead: Cell::new(false),
        }))
    }
}

struct ForkPtySession {
    master: c_int,
    pid: c_int,
    dead: Cell<bool>,
}

impl ForkPtySession {
    fn reap(&self) -> bool {
        if self.dead.get() {
            return true;
        }
        let mut st: c_int = 0;
        // SAFETY: pid는 우리가 fork한 자식.
        let r = unsafe { waitpid(self.pid, &mut st, WNOHANG) };
        if r == self.pid || r < 0 {
            self.dead.set(true);
        }
        self.dead.get()
    }
}

impl PtySession for ForkPtySession {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let mut off = 0;
        while off < bytes.len() {
            // SAFETY: 유효 버퍼 범위.
            let n = unsafe { write(self.master, bytes[off..].as_ptr().cast(), bytes.len() - off) };
            if n < 0 {
                let e = std::io::Error::last_os_error();
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                return Err(e);
            }
            off += n as usize;
        }
        Ok(())
    }
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        // SAFETY: 유효 버퍼 · 비차단 fd.
        let n = unsafe { read(self.master, buf.as_mut_ptr().cast(), buf.len()) };
        if n > 0 {
            return Ok(n as usize);
        }
        if n == 0 {
            self.dead.set(true); // EOF
            return Ok(0);
        }
        let e = std::io::Error::last_os_error();
        match e.kind() {
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => Ok(0),
            _ => {
                // Linux: 자식 종료 뒤 EIO — 세션 끝.
                self.dead.set(true);
                Ok(0)
            }
        }
    }
    fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        let ws = Winsize {
            ws_row: rows.max(2),
            ws_col: cols.max(2),
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: 열린 pty master · 구조체는 커널 winsize 레이아웃.
        let r = unsafe { ioctl(self.master, TIOCSWINSZ, &ws as *const Winsize) };
        if r < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    fn alive(&self) -> bool {
        !self.reap()
    }
    fn kill(&mut self) {
        if !self.reap() {
            // SAFETY: 우리 자식 pid.
            unsafe { kill(self.pid, SIGKILL) };
        }
    }
}

impl Drop for ForkPtySession {
    fn drop(&mut self) {
        // SAFETY: 열린 fd 닫기 → 세션 SIGHUP · 남아 있으면 SIGHUP 명시 · 가능한 만큼 수거(블록하지 않는다).
        unsafe {
            close(self.master);
            if !self.reap() {
                kill(self.pid, SIGHUP);
                let mut st: c_int = 0;
                waitpid(self.pid, &mut st, WNOHANG);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// 실제 pty: `/bin/sh -c "echo hello-pty"` 출력이 비차단 read로 온다(CI Linux/macOS · 10 s 상한).
    #[test]
    fn forkpty_runs_sh_echo() {
        let spec = ShellSpec {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".into(), "echo hello-pty".into()],
            label: "sh".into(),
        };
        let mut s = ForkPty
            .spawn(&spec, &std::env::temp_dir(), 80, 24)
            .expect("forkpty");
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut got = Vec::new();
        loop {
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap();
            got.extend_from_slice(&buf[..n]);
            if String::from_utf8_lossy(&got).contains("hello-pty") {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "출력 없음: {:?}",
                String::from_utf8_lossy(&got)
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        // 종료 수거는 수 ms 안에.
        let deadline = Instant::now() + Duration::from_secs(5);
        while s.alive() && Instant::now() < deadline {
            let mut buf = [0u8; 256];
            let _ = s.read(&mut buf);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!s.alive(), "echo 뒤 셸은 끝난다");
    }
}
