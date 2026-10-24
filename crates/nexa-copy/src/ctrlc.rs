//! Ctrl+C(콘솔 중단) → 전송 엔진의 `cancel` 원자값(외부 crate 0 · OS 호출 2종만).
//!
//! - Windows: `SetConsoleCtrlHandler` — Ctrl+C · Ctrl+Break · 콘솔 닫기 모두 깃발을 세운다(`TRUE` 반환 = 처리함 · 프로세스는 엔진이
//!   항목/청크 경계에서 `Interrupted`로 멈춘 뒤 요약을 찍고 130으로 끝난다). 닫기 사건은 OS가 곧 프로세스를 죽이므로 부분 파일 정리는
//!   최선 노력.
//! - Unix: `signal(SIGINT, handler)` — 핸들러는 원자값 저장만(async-signal-safe).
//!
//! 두 번째 Ctrl+C는 기본 동작(강제 종료)으로 돌려주지 않는다 — 엔진이 늦어도 청크(32 MiB) 경계에서 멈춘다.

use std::sync::atomic::{AtomicBool, Ordering};

static CANCEL: AtomicBool = AtomicBool::new(false);

/// 취소 깃발(한 프로세스에 하나 · 핸들러가 세운다).
pub(crate) fn flag() -> &'static AtomicBool {
    &CANCEL
}

/// 핸들러 설치 — 실패해도(권한 · 콘솔 없음) 복사는 진행한다(Ctrl+C가 기본 동작으로 떨어질 뿐).
pub(crate) fn install() {
    os::install();
}

#[cfg(windows)]
mod os {
    use super::{Ordering, CANCEL};
    use std::ffi::c_int;

    type HandlerRoutine = unsafe extern "system" fn(ctrl_type: u32) -> c_int;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetConsoleCtrlHandler(routine: Option<HandlerRoutine>, add: c_int) -> c_int;
    }

    unsafe extern "system" fn handler(_ctrl_type: u32) -> c_int {
        CANCEL.store(true, Ordering::Relaxed);
        1 // TRUE = 처리함
    }

    pub(super) fn install() {
        // SAFETY: 핸들러는 정적 함수 · 깃발 저장만 한다 · 프로세스 수명 동안 유효.
        let _ = unsafe { SetConsoleCtrlHandler(Some(handler), 1) };
    }
}

#[cfg(unix)]
mod os {
    use super::{Ordering, CANCEL};
    use std::ffi::c_int;

    const SIGINT: c_int = 2;

    extern "C" {
        fn signal(sig: c_int, handler: usize) -> usize;
    }

    extern "C" fn handler(_sig: c_int) {
        CANCEL.store(true, Ordering::Relaxed); // async-signal-safe: 원자값 저장만
    }

    pub(super) fn install() {
        // SAFETY: `signal`에 정적 핸들러 주소를 넘긴다 · 핸들러는 신호 안전 동작만 한다.
        let _ = unsafe { signal(SIGINT, handler as *const () as usize) };
    }
}

#[cfg(not(any(windows, unix)))]
mod os {
    pub(super) fn install() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 깃발은 프로세스 전역 하나 · 설치는 실패해도 패닉 없음(시험 러너에 콘솔이 없어도).
    #[test]
    fn flag_is_global_and_install_is_safe() {
        install();
        install(); // 두 번 설치해도 무해
        assert!(std::ptr::eq(flag(), flag()));
    }
}
