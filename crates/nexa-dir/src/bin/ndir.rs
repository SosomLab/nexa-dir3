//! `ndir` — **콘솔 서브시스템 보조 실행 파일**(T-178 · 10-05 §70 · v0.23.0 2차 실패에서 드러남): 앱 `nexa-dir.exe`는 창 서브시스템이라
//! PowerShell에서 `nexa-dir --version`을 치면 셸이 기다리지 않아 프롬프트가 먼저 돌아오고 출력이 뒤에 찍히며(캡처 = 빈 값),
//! `attach_parent_console`로도 순서를 못 고친다. 이 실행 파일은 콘솔 프로그램이라 셸이 끝까지 기다린다 — **같은 폴더의 `nexa-dir`에
//! 인자를 그대로 넘기고**(표준 입출력 상속 → 자식의 `GetStdHandle`이 유효해 그 핸들에 바로 쓴다) 끝나길 기다린 뒤 같은 종료 코드로 끝난다.
//!
//! nexa-sql `nsql`(콘솔 전용 CLI 크레이트)과 같은 자리이되, dir3의 CLI는 `--version` · `--help` · `--smoke` · `--selfcheck`뿐이라 기능을
//! 두 번 구현하지 않고 **전달만** 한다(의존 0 · 외부 crate 0). 패키징 = Windows MSI · 포터블 zip에만(다른 OS는 터미널에서 본체가 그대로 동작).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// 보조 실행 파일 경로 → 본체 경로(순수): 같은 폴더의 `nexa-dir`(Windows `.exe`).
fn gui_path(this_exe: &Path) -> PathBuf {
    let name = if cfg!(windows) {
        "nexa-dir.exe"
    } else {
        "nexa-dir"
    };
    this_exe.with_file_name(name)
}

fn main() -> ExitCode {
    let this_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ndir: cannot locate own executable: {e}");
            return ExitCode::from(127);
        }
    };
    let gui = gui_path(&this_exe);
    let status = std::process::Command::new(&gui)
        .args(std::env::args_os().skip(1))
        .status();
    match status {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("ndir: cannot run {}: {e}", gui.display());
            ExitCode::from(127)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 본체 경로 = 보조 실행 파일과 같은 폴더의 `nexa-dir(.exe)` — 폴더 · 이름 둘 다 확인.
    #[test]
    fn gui_path_is_sibling_nexa_dir() {
        let p = gui_path(Path::new("/opt/nexa/bin/ndir.exe"));
        assert_eq!(p.parent(), Some(Path::new("/opt/nexa/bin")));
        let name = p.file_name().unwrap().to_string_lossy();
        if cfg!(windows) {
            assert_eq!(name, "nexa-dir.exe");
        } else {
            assert_eq!(name, "nexa-dir");
        }
    }
}
