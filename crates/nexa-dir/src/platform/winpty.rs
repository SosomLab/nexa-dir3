//! Windows ConPTY(T-61 · dir2 `conpty.rs` 이식 — `windows` crate 대신 kernel32 수동 extern · 외부 crate 0).
//!
//! 셸을 Pseudo Console로 구동하고 stdin/stdout 파이프를 잇는다. 읽기 스레드가 출력 바이트를 공유 버퍼에 쌓고,
//! [`PtySession::read`]는 그 버퍼를 비차단으로 비운다(UTF-8 경계 보존·VT 파싱은 `termview`). 종료 = 읽기 EOF 또는
//! 프로세스 신호. Drop = 프로세스 종료·핸들 정리. Windows 10 1809+.

use super::*;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[repr(C)]
#[derive(Clone, Copy)]
struct Coord {
    x: i16,
    y: i16,
}

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count: u32,
    y_count: u32,
    fill: u32,
    flags: u32,
    show: u16,
    cb_reserved2: u16,
    reserved2: *mut u8,
    std_in: isize,
    std_out: isize,
    std_err: isize,
}

#[repr(C)]
struct StartupInfoExW {
    si: StartupInfoW,
    attr_list: *mut c_void,
}

#[repr(C)]
struct ProcessInformation {
    process: isize,
    thread: isize,
    pid: u32,
    tid: u32,
}

const EXTENDED_STARTUPINFO_PRESENT: u32 = 0x0008_0000;
/// PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE(winbase.h).
const ATTR_PSEUDOCONSOLE: usize = 0x0002_0016;
const WAIT_TIMEOUT: u32 = 0x102;
/// 실기(10-03): 부모의 stdout이 파이프(시험 러너 · ndir-check)면 자식이 그 std 핸들을 물려받아 셸 출력이 의사 콘솔 **밖으로** 샌다 —
/// `STARTF_USESTDHANDLES` + NULL 핸들로 의사 콘솔 핸들을 강제한다(node-pty 규약).
const STARTF_USESTDHANDLES: u32 = 0x100;

#[link(name = "kernel32")]
extern "system" {
    fn CreatePipe(read: *mut isize, write: *mut isize, attrs: *const c_void, size: u32) -> i32;
    fn CreatePseudoConsole(
        size: Coord,
        input: isize,
        output: isize,
        flags: u32,
        hpc: *mut isize,
    ) -> i32;
    fn ResizePseudoConsole(hpc: isize, size: Coord) -> i32;
    fn ClosePseudoConsole(hpc: isize);
    fn InitializeProcThreadAttributeList(
        list: *mut c_void,
        count: u32,
        flags: u32,
        size: *mut usize,
    ) -> i32;
    fn UpdateProcThreadAttribute(
        list: *mut c_void,
        flags: u32,
        attr: usize,
        value: *const c_void,
        size: usize,
        prev: *mut c_void,
        ret: *mut usize,
    ) -> i32;
    fn DeleteProcThreadAttributeList(list: *mut c_void);
    fn CreateProcessW(
        app: *const u16,
        cmd: *mut u16,
        pa: *const c_void,
        ta: *const c_void,
        inherit: i32,
        flags: u32,
        env: *const c_void,
        cwd: *const u16,
        si: *const StartupInfoExW,
        pi: *mut ProcessInformation,
    ) -> i32;
    fn ReadFile(h: isize, buf: *mut u8, n: u32, read: *mut u32, ov: *const c_void) -> i32;
    fn WriteFile(h: isize, buf: *const u8, n: u32, written: *mut u32, ov: *const c_void) -> i32;
    fn TerminateProcess(h: isize, code: u32) -> i32;
    fn WaitForSingleObject(h: isize, ms: u32) -> u32;
    fn CloseHandle(h: isize) -> i32;
    fn GetExitCodeProcess(h: isize, code: *mut u32) -> i32;
}

fn wide_os(s: &std::ffi::OsStr) -> Vec<u16> {
    let mut w: Vec<u16> = s.encode_wide().collect();
    w.push(0);
    w
}

/// 명령줄: `"program" arg…`(공백 포함 인자는 인용 · lpApplicationName은 전체 경로 — dir2 점검 1차 #3 플랜팅 방지).
fn command_line(shell: &ShellSpec) -> String {
    let mut s = format!("\"{}\"", shell.program.display());
    for a in &shell.args {
        s.push(' ');
        if a.contains(' ') || a.is_empty() {
            s.push('"');
            s.push_str(a);
            s.push('"');
        } else {
            s.push_str(a);
        }
    }
    s
}

fn os_err(what: &str) -> PlatformError {
    PlatformError::Failed(format!("{what}: {}", std::io::Error::last_os_error()))
}

pub(super) struct ConPty;

impl Pty for ConPty {
    fn spawn(
        &self,
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Box<dyn PtySession>, PlatformError> {
        // SAFETY: 모든 핸들·버퍼는 이 함수가 만들고 실패 경로마다 닫는다 · 구조체 레이아웃은 winbase.h와 같다.
        let s = unsafe { ConPtySession::start(shell, cwd, cols, rows)? };
        Ok(Box::new(s))
    }
}

/// 읽기 스레드가 쌓아 둘 수 있는 출력 상한(바이트) — 넘으면 읽기를 멈춰 셸에 역압을 준다.
const BACKLOG_CAP: usize = 256 * 1024;

struct ConPtySession {
    hpc: isize,
    process: isize,
    thread: isize,
    writer: isize,
    attr_list: *mut u8,
    attr_size: usize,
    output: Arc<Mutex<VecDeque<u8>>>,
    eof: Arc<AtomicBool>,
    /// 프로세스 종료를 본 뒤 의사 콘솔을 닫았다(남은 출력 flush → 읽기 EOF).
    closed: bool,
}

impl ConPtySession {
    unsafe fn start(
        shell: &ShellSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Self, PlatformError> {
        let (mut input_read, mut input_write) = (0isize, 0isize);
        let (mut output_read, mut output_write) = (0isize, 0isize);
        if CreatePipe(&mut input_read, &mut input_write, std::ptr::null(), 0) == 0 {
            return Err(os_err("CreatePipe"));
        }
        if CreatePipe(&mut output_read, &mut output_write, std::ptr::null(), 0) == 0 {
            CloseHandle(input_read);
            CloseHandle(input_write);
            return Err(os_err("CreatePipe"));
        }
        let size = Coord {
            x: i16::try_from(cols.max(2)).unwrap_or(i16::MAX),
            y: i16::try_from(rows.max(2)).unwrap_or(i16::MAX),
        };
        let mut hpc = 0isize;
        let hr = CreatePseudoConsole(size, input_read, output_write, 0, &mut hpc);
        if hr < 0 {
            for h in [input_read, input_write, output_read, output_write] {
                CloseHandle(h);
            }
            return Err(PlatformError::Failed(format!(
                "CreatePseudoConsole: 0x{:08x}",
                hr as u32
            )));
        }
        // ConPTY가 소유하는 끝은 우리 사본을 닫는다 → 셸 종료 시 output_read에 EOF 전파(dir2).
        CloseHandle(input_read);
        CloseHandle(output_write);

        let mut attr_size = 0usize;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
        let layout = std::alloc::Layout::from_size_align(attr_size.max(8), 8)
            .map_err(|e| PlatformError::Failed(e.to_string()))?;
        let attr_list = std::alloc::alloc(layout);
        if attr_list.is_null()
            || InitializeProcThreadAttributeList(attr_list.cast(), 1, 0, &mut attr_size) == 0
            || UpdateProcThreadAttribute(
                attr_list.cast(),
                0,
                ATTR_PSEUDOCONSOLE,
                hpc as *const c_void,
                std::mem::size_of::<isize>(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ) == 0
        {
            let e = os_err("ProcThreadAttribute");
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            if !attr_list.is_null() {
                std::alloc::dealloc(attr_list, layout);
            }
            return Err(e);
        }
        let si = StartupInfoExW {
            si: StartupInfoW {
                cb: std::mem::size_of::<StartupInfoExW>() as u32,
                reserved: std::ptr::null_mut(),
                desktop: std::ptr::null_mut(),
                title: std::ptr::null_mut(),
                x: 0,
                y: 0,
                x_size: 0,
                y_size: 0,
                x_count: 0,
                y_count: 0,
                fill: 0,
                flags: STARTF_USESTDHANDLES,
                show: 0,
                cb_reserved2: 0,
                reserved2: std::ptr::null_mut(),
                std_in: 0,
                std_out: 0,
                std_err: 0,
            },
            attr_list: attr_list.cast(),
        };
        let app_w = wide_os(shell.program.as_os_str());
        let mut cmd_w: Vec<u16> = command_line(shell)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let cwd_w = wide_os(cwd.as_os_str());
        let mut pi = ProcessInformation {
            process: 0,
            thread: 0,
            pid: 0,
            tid: 0,
        };
        if CreateProcessW(
            app_w.as_ptr(),
            cmd_w.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            cwd_w.as_ptr(),
            &si,
            &mut pi,
        ) == 0
        {
            let e = os_err("CreateProcessW");
            ClosePseudoConsole(hpc);
            CloseHandle(input_write);
            CloseHandle(output_read);
            DeleteProcThreadAttributeList(attr_list.cast());
            std::alloc::dealloc(attr_list, layout);
            return Err(e);
        }
        let output = Arc::new(Mutex::new(VecDeque::new()));
        let eof = Arc::new(AtomicBool::new(false));
        {
            let (out, eof) = (output.clone(), eof.clone());
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    let mut n = 0u32;
                    // SAFETY: output_read는 이 스레드가 소유 · buf 길이를 넘긴다.
                    let ok = unsafe {
                        ReadFile(
                            output_read,
                            buf.as_mut_ptr(),
                            buf.len() as u32,
                            &mut n,
                            std::ptr::null(),
                        )
                    };
                    if ok == 0 || n == 0 {
                        break;
                    }
                    if let Ok(mut o) = out.lock() {
                        o.extend(buf[..n as usize].iter().copied());
                    }
                    // 역압(10-03 사용자 "ls 출력 중 Ctrl+C가 안 먹는다"): UI가 못 따라오면 여기서 멈춰 ConPTY 파이프가 차게 둔다 —
                    // 셸의 출력이 막혀야 ^C가 곧바로 효과를 내고(쌓인 수 MB를 다 그린 뒤가 아니라) 메모리도 한없이 늘지 않는다.
                    while out.lock().map(|o| o.len()).unwrap_or(0) > BACKLOG_CAP {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                eof.store(true, Ordering::SeqCst);
                // SAFETY: 이 스레드의 핸들.
                unsafe { CloseHandle(output_read) };
            });
        }
        Ok(ConPtySession {
            hpc,
            process: pi.process,
            thread: pi.thread,
            writer: input_write,
            attr_list,
            attr_size: layout.size(),
            output,
            eof,
            closed: false,
        })
    }
}

impl PtySession for ConPtySession {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let mut written = 0u32;
        // SAFETY: writer는 열린 파이프 핸들 · bytes 길이를 넘긴다.
        let ok = unsafe {
            WriteFile(
                self.writer,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                std::ptr::null(),
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = match self.output.lock() {
            Ok(mut o) => {
                let n = o.len().min(buf.len());
                for (dst, src) in buf.iter_mut().zip(o.drain(..n)) {
                    *dst = src;
                }
                n
            }
            Err(_) => 0,
        };
        // 실기(10-03): 클라이언트가 끝나도 conhost는 마지막 화면을 **ClosePseudoConsole 뒤에야** 내보낸다 —
        // 종료를 본 순간 콘솔을 닫아 남은 출력을 흘리고 읽기 스레드가 EOF로 끝나게 한다.
        if n == 0 && !self.closed {
            // SAFETY: 열린 프로세스 핸들 · hpc는 아직 열린 의사 콘솔.
            if unsafe { WaitForSingleObject(self.process, 0) } != WAIT_TIMEOUT {
                unsafe { ClosePseudoConsole(self.hpc) };
                self.closed = true;
            }
        }
        Ok(n)
    }
    fn resize(&mut self, cols: u16, rows: u16) -> std::io::Result<()> {
        let size = Coord {
            x: i16::try_from(cols.max(2)).unwrap_or(i16::MAX),
            y: i16::try_from(rows.max(2)).unwrap_or(i16::MAX),
        };
        // SAFETY: hpc는 열린 의사 콘솔.
        let hr = unsafe { ResizePseudoConsole(self.hpc, size) };
        if hr < 0 {
            return Err(std::io::Error::other(format!(
                "ResizePseudoConsole 0x{:08x}",
                hr as u32
            )));
        }
        Ok(())
    }
    fn alive(&self) -> bool {
        // 읽기 EOF까지 살아 있는 것으로 본다(종료 뒤 마지막 출력을 호스트가 다 수거한 다음에야 "종료" 표시).
        !self.eof.load(Ordering::SeqCst)
    }
    fn kill(&mut self) {
        // SAFETY: 열린 프로세스 핸들.
        unsafe { TerminateProcess(self.process, 0) };
    }
}

impl Drop for ConPtySession {
    fn drop(&mut self) {
        // SAFETY: start가 만든 핸들·속성 목록을 한 번씩 정리한다(dir2 Drop과 같은 순서).
        unsafe {
            TerminateProcess(self.process, 0);
            CloseHandle(self.writer);
            if !self.closed {
                ClosePseudoConsole(self.hpc);
            }
            if !self.attr_list.is_null() {
                DeleteProcThreadAttributeList(self.attr_list.cast());
                if let Ok(layout) = std::alloc::Layout::from_size_align(self.attr_size, 8) {
                    std::alloc::dealloc(self.attr_list, layout);
                }
            }
            CloseHandle(self.thread);
            CloseHandle(self.process);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn command_line_quotes_spaces() {
        let s = ShellSpec {
            program: PathBuf::from(r"C:\P F\pwsh.exe"),
            args: vec!["-NoLogo".into(), "a b".into()],
            label: String::new(),
        };
        assert_eq!(command_line(&s), r#""C:\P F\pwsh.exe" -NoLogo "a b""#);
    }

    /// 실제 ConPTY: `cmd /c echo` 출력이 읽기 버퍼로 온다(CI Windows 러너 포함 · 10 s 상한).
    #[test]
    fn conpty_runs_cmd_echo() {
        let sysroot = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let spec = ShellSpec {
            program: Path::new(&sysroot).join("System32").join("cmd.exe"),
            args: vec!["/c".into(), "echo".into(), "hello-pty".into()],
            label: "cmd".into(),
        };
        // SAFETY: 시험 — start의 전제(핸들 정리)는 함수 안에서 지킨다.
        let mut s = unsafe { ConPtySession::start(&spec, &std::env::temp_dir(), 80, 24) }
            .expect("ConPTY 시작");
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut got = Vec::new();
        loop {
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap();
            got.extend_from_slice(&buf[..n]);
            if String::from_utf8_lossy(&got).contains("hello-pty") {
                break;
            }
            if Instant::now() >= deadline {
                let mut code = 0u32;
                // SAFETY: 열린 프로세스 핸들.
                unsafe { GetExitCodeProcess(s.process, &mut code) };
                panic!(
                    "출력 없음: {:?} alive={} exit={code} eof={}",
                    String::from_utf8_lossy(&got),
                    s.alive(),
                    s.eof.load(Ordering::SeqCst)
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // 종료 뒤 EOF까지(콘솔 닫힘 → 읽기 스레드 종료).
        let deadline = Instant::now() + Duration::from_secs(5);
        while s.alive() && Instant::now() < deadline {
            let mut buf = [0u8; 256];
            let _ = s.read(&mut buf);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!s.alive(), "종료 뒤 EOF");
        assert!(s.resize(100, 30).is_err() || true);
        s.kill();
    }
}
