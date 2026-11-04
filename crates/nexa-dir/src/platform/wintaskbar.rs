//! GAP-042 — winit 0.30.13 `TaskbarCreated` 교착 우회(Windows 전용 · 앱 쪽 보완 · 사용자 결정 11-04 ⓑ).
//!
//! **결함**(상류 winit `platform_impl/windows/event_loop.rs:2455-2456` · PR #4563이 master(0.31)에만 병합):
//! 작업 표시줄이 다시 만들어질 때(RDP 재접속 · explorer 재시작) 셸이 등록 메시지 `TaskbarCreated`를 모든 최상위 창에
//! 브로드캐스트한다. winit은 이 메시지를 받으면 **창 상태 `Mutex` 가드를 쥔 채** `ITaskbarList::AddTab`을 부르고, 그 안의
//! `SendMessageW`(작업 표시줄 창)가 동기 대기한다. 대기 중 들어온 테마 변경 후처리(uxtheme `SetFrameTheme` → `SetWindowPos`)가
//! 같은 창에 `WM_WINDOWPOSCHANGING`을 동기 전달하고, winit이 같은 가드를 다시 잠그려다 영구 대기 → 창이 "응답 없음".
//!
//! **우회**: 창을 만든 직후 `SetWindowSubclass`(comctl32)로 서브클래스 프로시저를 단다. 서브클래스는 클래스 WndProc(winit)보다
//! **먼저** 메시지를 받으므로 `TaskbarCreated`만 `DefWindowProcW`로 돌려보내 winit의 처리(= AddTab)를 건너뛴다. 앱은
//! `skip_taskbar`를 쓰지 않으므로 AddTab은 필요 없고(탐색기가 다시 뜨면 최상위 창은 스스로 작업 표시줄에 오른다) 동작 차이가
//! 없다. 나머지 메시지는 `DefSubclassProc`로 그대로 흘려보낸다. `WM_NCDESTROY`에서 서브클래스를 뗀다(Raymond Chen 패턴).
//!
//! **적용 규칙**: winit 창을 만드는 모든 곳(`el.create_window(...)`)은 바로 뒤에 `platform::guard_taskbar_created(&win)`을 부른다 —
//! 정적 가드 시험 `taskbar_guard_follows_every_window_creation`(platform/mod.rs)이 파일별 건수를 대조한다.
//! **제거 조건**: winit을 #4563이 든 판(0.31 이후)으로 올리면 이 모듈과 호출 19곳 · 가드 시험을 지운다.
//! 다른 프로젝트 이식 = [docs/27](../../../../docs/27-winit-taskbar-deadlock.md).

use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, RegisterWindowMessageW, WM_NCDESTROY,
};

/// 서브클래스 식별자("NDTB") — 같은 창에 두 번 달아도 comctl32가 같은 (proc, id) 쌍은 갱신만 한다.
const SUBCLASS_ID: usize = 0x4e44_5442;

/// `RegisterWindowMessageW("TaskbarCreated")` — 세션 안에서 고정(셸과 같은 값 · 0xC000~0xFFFF).
fn taskbar_created_msg() -> u32 {
    static MSG: OnceLock<u32> = OnceLock::new();
    // SAFETY: 널 종료 상수 문자열 · 실패 = 0(어떤 메시지와도 같지 않음).
    *MSG.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

/// 창에 서브클래스를 단다 — 창을 만든 스레드(UI 스레드)에서 바로 부른다. 반환 = 성공 여부.
pub(crate) fn install(hwnd: isize) -> bool {
    // 등록 실패(0)면 걸러 낼 메시지가 없으니 달지 않는다(종전 그대로).
    if taskbar_created_msg() == 0 {
        return false;
    }
    let hwnd = HWND(hwnd as *mut core::ffi::c_void);
    // SAFETY: 살아 있는 창의 HWND · 프로시저는 'static · 참조 자료 없음(0).
    unsafe { SetWindowSubclass(hwnd, Some(guard_proc), SUBCLASS_ID, 0).as_bool() }
}

unsafe extern "system" fn guard_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    _data: usize,
) -> LRESULT {
    if msg == taskbar_created_msg() {
        // winit의 처리(가드 쥔 채 AddTab 동기 대기)를 건너뛴다 — winit도 결국 DefWindowProc로 끝낸다.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    if msg == WM_NCDESTROY {
        // SAFETY: 창 소멸 마지막 메시지 · 같은 (proc, id)로 뗀다 · 이후 DefSubclassProc 호출은 허용된 패턴.
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(guard_proc), id);
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, RegisterClassW, SendMessageW, UnregisterClassW,
        HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WNDCLASSW,
    };

    /// 클래스 WndProc(= winit 자리)에 닿은 메시지 수 — TaskbarCreated · WM_APP+1 각각.
    static SEEN_TASKBAR: AtomicU32 = AtomicU32::new(0);
    static SEEN_APP: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn class_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == taskbar_created_msg() {
            SEEN_TASKBAR.fetch_add(1, Ordering::SeqCst);
        } else if msg == WM_APP + 1 {
            SEEN_APP.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    /// 메시지 전용 창(화면 없음)에 서브클래스를 달면 `TaskbarCreated`는 클래스 WndProc에 닿지 않고, 다른 메시지는 그대로 닿는다.
    /// 소멸(`WM_NCDESTROY`) 뒤에도 패닉 없이 끝난다.
    #[test]
    fn taskbar_created_is_swallowed_and_other_messages_pass_through() {
        let class = w!("ndir-taskbar-guard-test");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(class_proc),
            lpszClassName: class,
            ..Default::default()
        };
        // SAFETY: 이 시험 전용 클래스 · 메시지 전용 창 · 같은 스레드에서 만들고 지운다.
        unsafe {
            assert_ne!(RegisterClassW(&wc), 0, "RegisterClassW");
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
            .expect("CreateWindowExW");
            let msg = taskbar_created_msg();
            assert!(msg >= 0xC000, "등록 메시지 범위");

            // 가드 전: 클래스 WndProc에 닿는다(= winit이 받던 상태).
            SendMessageW(hwnd, msg, None, None);
            assert_eq!(SEEN_TASKBAR.load(Ordering::SeqCst), 1);

            assert!(install(hwnd.0 as isize), "SetWindowSubclass");
            // 가드 뒤: TaskbarCreated는 걸러지고 다른 메시지는 통과.
            SendMessageW(hwnd, msg, None, None);
            SendMessageW(hwnd, WM_APP + 1, None, None);
            assert_eq!(SEEN_TASKBAR.load(Ordering::SeqCst), 1, "걸러져야 한다");
            assert_eq!(SEEN_APP.load(Ordering::SeqCst), 1, "통과해야 한다");
            // 두 번 달아도 갱신만(같은 proc · id) — 여전히 1건.
            assert!(install(hwnd.0 as isize));
            SendMessageW(hwnd, msg, None, None);
            assert_eq!(SEEN_TASKBAR.load(Ordering::SeqCst), 1);

            DestroyWindow(hwnd).expect("DestroyWindow");
            UnregisterClassW(class, None).expect("UnregisterClassW");
        }
    }
}
