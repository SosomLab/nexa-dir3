# 27 · winit 0.30.13 `TaskbarCreated` 교착(GAP-042) — 원인 · 우회 · 다른 프로젝트 이식 안내

> 이 문서는 **자족 문서**다. nexa-sql · nexa-beep · nexa-clip 같은 형제 프로젝트에서 "nexa-dir3의 winit 결함 해결 내용을 반영해 줘"라는
> 요청을 받은 세션이 이 문서만 읽고 적용할 수 있게 쓴다. 원 기록 = [journal 11-04 §1](journal/2026-11-04.md) · 원장 = [port/99 GAP-042](port/99-coverage-gaps.md) · 결정 = [DR-38](10-decision-record.md).

## 1. 증상

- Windows에서 앱 창이 **영구 "응답 없음"**(제목에 `(응답 없음)` · 저절로 풀리지 않음 · CPU 0). 다른 창도 같은 프로세스면 함께 멈춘다.
- 계기 = **작업 표시줄이 다시 만들어지는 순간**: RDP 재접속 · explorer.exe 재시작 · 셸 크래시 뒤 복구. 확률적(아래 메시지 순서가 맞아야 한다).
- 확인된 사례: nexa-dir 설치본 0.29.0(2026-11-04 · RDP-Tcp 세션 · 이벤트 로그에 DWM 세션 포트 재등록 직후).

## 2. 원인(덤프로 확정 · 상류 결함)

winit `0.30.13`(crates.io · 0.30 계열 마지막 판) `src/platform_impl/windows/event_loop.rs`:

1. 셸이 등록 메시지 **`TaskbarCreated`**(`RegisterWindowMessageW("TaskbarCreated")`)를 모든 최상위 창에 브로드캐스트한다.
2. winit `:2455-2456`이 **창 상태 `Mutex` 가드를 쥔 채** `set_skip_taskbar` → `ITaskbarList::AddTab` → `SendMessageW`(작업 표시줄 창)로 **동기 대기**한다.
3. 대기 중 들어온 테마 변경 후처리(uxtheme `ThemePostWndProc` → `SetFrameTheme` → `SetWindowPos`)가 같은 창에 **`WM_WINDOWPOSCHANGING`** 을 동기 전달한다.
4. winit `:1300`이 같은 스레드가 이미 쥔 `Mutex`를 다시 잠그려다 `WaitOnAddress`에서 **영구 대기**.

- 상류 수정 = [rust-windowing/winit PR #4563](https://github.com/rust-windowing/winit/pull/4563) "win32: fix WM_TASKBARCREATED deadlock"(2026-07-27 master 병합 · 가드 대신 값만 읽는 2줄). **0.31.0-beta에만 들어 있고 0.30 계열에는 없다** → crates.io 판 갱신으로는 못 고친다.
- 판독 방법(재현 가능): `Get-Process`로 `Responding=False` 확인 → PowerShell `Add-Type`으로 `dbghelp!MiniDumpWriteDump`(FullMemory) → WinDbg `cdbX64 -z <dmp> -y "srv*<cache>*https://msdl.microsoft.com/download/symbols" -logo <log> -c "~*k 25; q"` → GUI 스레드 스택에 `explorerframe!TaskbarList::AddTab` → `user32!SendMessageW` 위에 `uxtheme!CThemeWnd::SetFrameTheme` → `win32u!NtUserSetWindowPos` → 앱 WndProc → `KERNELBASE!WaitOnAddress`가 보이면 이 결함. `kb`의 WndProc 인자 = 바깥 msg `0xC06E`(TaskbarCreated · 세션마다 값 다름) · 안쪽 `0x46`(WM_WINDOWPOSCHANGING).

## 3. 우회(nexa-dir3 구현 · 사용자 결정 11-04 ⓑ "우리 코드만 · 포크 없음")

**원리**: 창을 만든 직후 `SetWindowSubclass`(comctl32)로 서브클래스 프로시저를 단다. 서브클래스는 클래스 WndProc(winit)보다 **먼저** 메시지를 받으므로, `TaskbarCreated`만 `DefWindowProcW`로 돌려보내 winit의 처리(AddTab)를 건너뛴다. 앱이 `skip_taskbar`를 쓰지 않으면 AddTab은 불필요하다(탐색기가 다시 뜨면 최상위 창은 스스로 작업 표시줄에 오른다). 나머지 메시지는 `DefSubclassProc`로 그대로 흘린다. `WM_NCDESTROY`에서 서브클래스를 뗀다.

**구성(3조각 · 전부 nexa-dir3 저장소)**:

| 조각 | 위치 | 역할 |
| --- | --- | --- |
| 모듈 | `crates/nexa-dir/src/platform/wintaskbar.rs`(`#[cfg(windows)]`) | `install(hwnd: isize) -> bool` · 서브클래스 프로시저 · 메시지 id `OnceLock` · Win32 단위 시험 1(메시지 전용 창 · 가드 전 닿음 / 뒤 걸러짐 / 다른 메시지 통과 / 두 번 달아도 1건) |
| 공개 함수 | `platform/mod.rs` `guard_taskbar_created(&winit::window::Window) -> Result<(), PlatformError>` | HWND 추출 → `install` · 다른 OS = `Err(Unsupported)` |
| 호출 | winit 창을 만드는 **모든** 곳(`el.create_window(attrs)` 직후 · 19곳) | `let _ = crate::platform::guard_taskbar_created(&win);` |
| 정적 가드 | `platform::tests::taskbar_guard_follows_every_window_creation`(3-OS) | 파일별 `el.create_window(` 건수 = `guard_taskbar_created(&win)` 건수 · 새 창을 추가하며 호출을 빠뜨리면 실패 |

**한계**: 자동 재현은 불가(Win32 메시지 순서) → 실기 = `PostMessage(HWND_BROADCAST, TaskbarCreated)` 직후 테마 변경(고대비 전환) 또는 RDP 재접속 반복 · 매트릭스 "실기 필요". 서브클래스는 그 창의 **UI 스레드에서** 달아야 한다(생성 직후 호출이면 자동 충족).

**제거 조건**: winit을 #4563이 든 판(0.31 이후)으로 올릴 때 모듈 · 호출 19곳 · 정적 가드 시험 · 이 문서의 "현행" 표기를 함께 지운다.

## 4. 다른 프로젝트 이식 절차(nexa-sql · nexa-beep · nexa-clip — 모두 winit 0.30.13)

> 요청 예: "nexa-dir3의 winit 결함(GAP-042 · TaskbarCreated 교착) 해결 내용 반영해 줘". 적용 조건 = `Cargo.lock`의 `winit` 판이 `0.30.x`이고 Windows 창을 만든다.

1. **판 확인**: `grep -A1 '^name = "winit"' Cargo.lock` → `0.30.13`이면 대상(0.31 이후면 불필요 — 이 문서 §3 제거 조건).
2. **`windows` 크레이트 기능**: `Win32_UI_Shell`(`SetWindowSubclass` · `DefSubclassProc` · `RemoveWindowSubclass`) · `Win32_UI_WindowsAndMessaging`(`DefWindowProcW` · `RegisterWindowMessageW` · `WM_NCDESTROY`) · `Win32_Foundation`. nexa-dir3는 `windows 0.62`. 없으면 `[target.'cfg(windows)'.dependencies]`의 features에 추가(판은 그 프로젝트 것 유지 · API 이름 동일).
3. **모듈 복사**: `../nexa-dir3/crates/nexa-dir/src/platform/wintaskbar.rs`를 그 프로젝트의 Windows 전용 자리(nexa-sql = `platform/` 또는 창 호스트 옆)에 그대로 복사. 머리 주석의 dir3 경로 링크만 그 프로젝트 문서로 바꾼다. 시험 모듈도 함께(Windows에서만 돈다).
4. **공개 함수**: HWND를 꺼내는 그 프로젝트의 헬퍼(nexa-dir3 = `winfocus::hwnd(&Window) -> Option<isize>` · `raw_window_handle` `RawWindowHandle::Win32(h) => h.hwnd.get()`)로 `guard_taskbar_created(&Window)`를 `platform/mod.rs`(또는 호스트 모듈)에 추가 — nexa-dir3 `platform/mod.rs`의 같은 이름 함수를 복사.
5. **호출 지점**: `grep -rn "create_window(" src/`로 winit 창 생성 지점을 전부 찾아(X11 `conn.create_window`는 제외) **바로 뒤**에 `let _ = platform::guard_taskbar_created(&win);` 1줄. 창을 한 헬퍼에서만 만드는 프로젝트면 1곳.
6. **정적 가드 시험 복사**: nexa-dir3 `platform/mod.rs` `taskbar_guard_follows_every_window_creation`을 복사 · 창 생성 식별자(`el.create_window(`)와 최소 건수(`total >= N`)를 그 프로젝트에 맞춘다.
7. **검증**: `cargo test` Windows(단위 시험 + 정적 가드) · 3-OS clippy `-D warnings`(다른 OS는 `Err(Unsupported)` 경로만 컴파일) · 실기 = §3 한계.
8. **기록**: 그 프로젝트 결정 기록에 "GAP-042 우회 · 출처 nexa-dir3 docs/27 · 제거 조건 winit 0.31" 1행 · 릴리스 노트(영어) "Fix: window could hang permanently after RDP reconnect / Explorer restart (winit 0.30.13 TaskbarCreated deadlock, worked around in-app)".

## 5. 대안(검토 · 기각 경위)

| 안 | 내용 | 기각 이유 |
| --- | --- | --- |
| ⓐ winit 포크 백포트 | SosomLab/winit `v0.30.13` + #4563 2줄 · 각 저장소 `[patch.crates-io]` | 개발 권고였으나 **사용자 11-04: 외부 저장소·라이브러리 수정 없이 우리 코드만 · dir3만 수정 · 형제는 요청 때 이 문서로** |
| ⓒ 0.31 대기 | crates.io `0.31.0-beta.4` | 베타 · API 변경 큼 |
