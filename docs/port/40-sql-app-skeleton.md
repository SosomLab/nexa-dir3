# 40. 기준 구조 가이드 — nexa-sql 앱 골격: "nexa-ui 위에 앱을 세우는 방법"

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 2026-10-03.
> 대상: nexa-sql `0.1.3` 작업 트리(D:/Projects/kiros33/nexa-sql · winit 0.30 + softbuffer 0.4 + nexa-ui).
> 목적: 파일 탐색기 앱 nexa-dir3가 **그대로 본뜰** 창·이벤트 루프·입력·그리기·OS 분기 골격을 항목별로 고정한다. ID 접두사 `SKEL-`.
> 표기: 근거는 `저장소/경로:줄`. 표 안의 경로 접두는 줄여 쓴다 —
> **`S/` = `nexa-sql/crates/nexa-sql/src/`** · **`D/` = `nexa-sql/docs/`** · **`U/` = `nexa-ui/crates/`** · **`R/` = `nexa-sql/`**(저장소 루트).
> 확인하지 못한 것은 **추정**으로 표시한다. "dir3 적용" 열의 분류: **그대로** = 코드를 거의 복사 / **개명** = 식별자·설정 키만 바꿈 / **치환** = 같은 틀에 dir3 내용을 채움 / **불요** = SQL 전용이라 옮기지 않음 / **승격 후보** = nexa-ui 공용 부품으로 올릴 것.

---

## 0. 범위 — 읽은 파일과 줄 수

| 파일 | 줄 수 | 읽은 범위 |
| --- | ---: | --- |
| `S/main.rs` | 2,872 | 1~700(모듈 선언·`App` 구조체 전부) · 880~1276(`layout`·보조 함수) · 1276~2075(`main()` 전부·`FrameTrace`) · 2655~2765(인자·인스턴스 잠금) + 함수 목록 Grep 전부. 700~880·2075~2655·2765~끝은 SQL 도메인 보조 함수·시험이라 목록만 확인 |
| `S/app/mod.rs` | 34 | 전부 |
| `S/app/event_loop.rs` | 1,976 | 전부(1~1976) |
| `S/app/input.rs` | 1,864 | 1~1040 · 1140~1290 · 1620~1864(라우팅 본체·고리 대표·포인터 캡처 전부). 1040~1140·1290~1620은 패널별 `route_*` 고리(같은 꼴 반복 · 시그니처 Grep으로 확인) |
| `S/app/paint.rs` | 679 | 전부 |
| `S/app/windows.rs` | 290 | 전부 |
| `S/app/toolbar.rs` | 319 | 전부 |
| `S/app/menus.rs` | 1,829 | 구조 위주: 300~430 · 1020~1140 · 1300~1475 · 1705~1829 + 함수 목록 Grep |
| `S/app/events.rs` | 1,471 | 180~240 · 800~920(워커 응답 수거 틀) + 함수 목록 Grep |
| `S/app/settings.rs` | 1,013 | 1~260 · 940~1013(`apply_setting`·`apply_theme`·`persist_settings`·`relabel`) |
| `S/app/session.rs` · `files.rs` · `connwin.rs` · `license.rs` · `startup_cmd.rs` | — | 골격과 닿는 구간만(`drain_all` 400~460 · `finish_exit`/`request_exit` 170~250 · `open_file_window` 330~401 · `open_conn_window` 20~45 · `open_license_window` 1~60 · `ui.click` 24~48) |
| `S/winhost.rs` | 72 | 전부 |
| `S/present.rs` | 169 | 전부 |
| `S/wingeom.rs` | 206 | 전부 |
| `S/winfocus.rs` | 233 | 전부 |
| `S/theme.rs` | 180 | 전부 |
| `S/icon.rs` | 299 | 전부 |
| `S/toast.rs` | 345 | 1~200(모델·틱·그리기 틀) |
| `S/clipboard.rs` | 299 | 전부 |
| `S/clipboard_x11.rs` | 388 | 전부 |
| `S/imestate.rs` | 411 | 전부 |
| `S/imehint.rs` | 193 | 전부 |
| `S/imewatch.rs` | 254 | 전부 |
| `S/input.rs` | 511 | 전부 |
| `S/worker.rs` | 1,896 | 1~140 · 395~525 · 735~775 · 1575~1617(명령·핸들·spawn·패닉 격리·테스트 스레드) + 구조 Grep |
| `S/about_win.rs` | 326 | 전부(보조 창 본보기) |
| `S/toolfloat.rs` · `S/log_win.rs` | — | `toolfloat.rs` 1~110 · `log_win.rs` 695~760(공통 호스트 사용 본보기) |
| `R/crates/nexa-sql/build.rs` · `Cargo.toml` | 6 · 67 | 전부 |
| `R/Cargo.toml` · `R/.cargo/config.toml` | 101 · 13 | 전부 |
| `R/packaging/windows/winres.rs` · `nexa-sql.rc` | — | `winres.rs` 1~90 · `.rc` 전부 |
| `D/01-architecture.md` | 77 | 전부 |
| `D/30-architecture-patterns.md` | 126 | 전부 |
| `D/61-core-design-and-working-rules.md` | 305 | 전부 |
| `D/62-macos-input-and-present.md` | 116 | 전부 |
| `D/26-performance-architecture.md` | 786 | 1~175(원칙·단계 모델·경량 구조·예산·메모리 기준선) + 목차 Grep(§7-3 이후는 실측 기록) |
| 대조: `U/nexa-sys/src/*` · `U/nexa-fs/src/*` · `U/nexa-dlg/src/lib.rs` · `U/nexa-conf/src/lib.rs` · `U/nexa-ctl/src/controls/` | — | 공개 API Grep · 파일 목록(컨트롤 유무 확인용) |

**범위 밖**: nexa-ctl 컨트롤 낱낱의 API(다른 인벤토리 문서 몫) · 설정 레지스트리 `nsql-settings` 상세 · 라이선스 정책(`nsql-license`) — 여기서는 "골격이 그것들을 어떻게 잇는가"만 적는다.

---

## 1. 골격 요소 표

### 1-1. 프로세스 진입 · 기동 순서

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-001 | 콘솔 없는 GUI exe | `#![cfg_attr(windows, windows_subsystem = "windows")]` | `S/main.rs:11` | 그대로 |
| SKEL-002 | `main()` 기동 순서 | 설정 열기 → i18n 언어 → nexa-ctl 내장 라벨 주입 → 글꼴(UI·고정폭) → 인자 → 이벤트 루프 생성 → 프록시·워커·프로브 → 부품 생성(`[startup]` 구간 표시) → `App { … }` → 설정을 전역 토큰·컨트롤에 일괄 반영 → `el.run_app(&mut app)`. **창은 여기서 만들지 않는다**(`resumed`에서) | `S/main.rs:1276-2005` | 치환(같은 순서 · SQL 부품 자리에 패널·파일 목록·도크) |
| SKEL-003 | 설정 열기 실패 폴백 | `Settings::open_default()` 실패 = 임시 폴더 경로의 기본값(저장은 실패해도 앱은 뜬다) | `S/main.rs:1284-1290` | 개명(`ndir-settings` 등 · 설정 문서 참조) |
| SKEL-004 | i18n + 컨트롤 라벨 주입 | `nsql_i18n::set_lang(settings.lang())` · `nexa_ctl::controls::set_ctl_labels(|m| …)`로 nexa-ctl 내장 우클릭 편집 메뉴 라벨(`CtlMsg::CtxSelectAll/Copy/Cut/Paste`)을 앱 i18n에 잇는다(미주입 = 영어) | `S/main.rs:1291-1301` | 치환(dir2 문자열 자원 → 같은 훅) |
| SKEL-005 | 글꼴 적재 | `nexa_font::ui_font(pref)`(없으면 OS 사슬) · `nexa_font::mono_font(pref)` · 둘 중 하나라도 없으면 오류 출력 후 `exit(1)` | `S/main.rs:1302-1313` | 그대로(터미널·미리보기용 고정폭 포함) |
| SKEL-006 | `--smoke` · `--help` | 창 없이 끝나는 자가 점검/도움말(글꼴 사슬·드라이버 목록 출력) | `S/main.rs:1314-1332` | 치환(CI 스모크용 — 회귀 하네스의 첫 줄) |
| SKEL-007 | 실행 인자 갈라내기 | `split_file_args`(프로젝트 파일 · 존재하는 파일 · **폴더** · 나머지) + `parse_gui_args` — 순수 함수 + 시험 | `S/main.rs:2662-2701` · `2739-2764` · 시험 `2766-` | 치환(dir3 = 시작 폴더/파일 선택 인자) |
| SKEL-008 | 인스턴스 잠금 | 설정 폴더 `instance.lock`에 `File::try_lock`(표준 라이브러리 · 3-OS) — "첫 인스턴스인가"만 판단, 다중 인스턴스 허용 | `S/main.rs:2723-2737` | 그대로(dir2의 단일/다중 인스턴스 정책에 맞춰 사용 — 정책은 dir2 문서 확인) |
| SKEL-009 | 이벤트 루프 생성 | `EventLoop::<Wake>::with_user_event()` · Linux만 설정 `gfx.linux_backend`로 `with_x11()`/`with_wayland()` 선택 → 실패하면 winit 기본으로 한 번 더 | `S/main.rs:1344-1366` | 그대로(설정 키 개명) |
| SKEL-010 | 기동 구간 계측 | `marks` 벡터에 누적 ms → `NSQL_TRACE_FRAMES`가 있으면 `[startup] … (ms, cumulative)` · `resumed` 안 2부 `[startup:resumed]` | `S/main.rs:1278-1281` · `1992-2000` · `S/app/event_loop.rs:35-43` · `105-108` | 개명(`NDIR_TRACE_FRAMES` — 제안) |
| SKEL-011 | 상태는 `App` 한 곳 · 동작은 `app/*.rs` | `struct App`(필드 전부 main.rs) + `mod app;` 아래 기능별 `impl App` 조각 30개(`use crate::*;`) · 새 동작은 해당 기능 파일에 | `S/main.rs:205-605` · `645` · `S/app/mod.rs:1-34` · `D/30-architecture-patterns.md:56` | 그대로(틀) — §5-1 구성안 |
| SKEL-012 | 설정 → 전역 토큰 일괄 반영(부팅 1회) | 글리프 캐시 상한 · 아이콘 캐시 · hover 의도 지연 · 클릭 가드 · 페이드 ms · hover/눌림 색 · 메뉴 아이콘 · 자연 스크롤 · 클립보드 네이티브 · present 방식 · 텍스트 렌더(대비·스냅·힌트·GDI·굵기) | `S/main.rs:1783-1951` · `S/app/settings.rs:140-149` | 치환(같은 호출 묶음 · SQL 전용 줄 제거) |
| SKEL-013 | 아이콘 마스크 선굽기 스레드 | 첫 표시 때 UI 스레드가 래스터화로 멎지 않게 별 스레드에서 `prewarm()` | `S/main.rs:1426-1431` | 치환(dir2 아이콘 자원 디코드 선굽기) |
| SKEL-014 | 루프 진입 | `el.run_app(&mut app)` · 실패 = 메시지 + `exit(1)` | `S/main.rs:2001-2004` | 그대로 |
| SKEL-015 | 종료 정리 훅 | `exiting()` — 메모리에 든 비밀 폐기 | `S/app/event_loop.rs:192-195` | 치환(터미널 자식 프로세스 종료 등) |

### 1-2. 메인 창 생성

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-016 | 창은 `resumed`에서 1회 | `if self.window.is_some() { return; }` | `S/app/event_loop.rs:8-11` | 그대로 |
| SKEL-017 | 창 속성 | 제목 · **`with_visible(false)`** · `with_theme(window_theme(mode))` · 마지막 닫힌 논리 크기(`window.main_size` · 기본 1375×945) · `icon::with_icon` 통과 | `S/app/event_loop.rs:20-34` | 개명 |
| SKEL-018 | 숨긴 채 만들고 → 위치 → 보이기 | 모니터 목록은 **창 핸들**로(`win.available_monitors()`) · `window.monitor`(1-기준) 또는 `window.main_pos`(논리 좌표 · 살아 있는 모니터 안일 때만) → `set_outer_position` → `keep_on_screen` → `set_visible(true)` | `S/app/event_loop.rs:49-85` | 그대로 |
| SKEL-019 | 배율·테마 확정 | `self.scale = win.scale_factor()` · 창이 생긴 뒤 `theme::resolve(mode, win.theme())`로 System 모드 확정 | `S/app/event_loop.rs:86-88` | 그대로 |
| SKEL-020 | 표면 붙이기 | `present::Presenter::new(win.clone())` — 실패는 stderr만(앱은 계속) | `S/app/event_loop.rs:91-99` | 그대로 |
| SKEL-021 | 창 생성 뒤 초기화 묶음 | `apply_on_top` → `layout` → `set_focus` → 메뉴 장식 → 입력 소스 감시 `nexa_sys::input_source::watch()` → `sync_hangul_mode` → 툴바 배치 복원 → 메뉴 재구성 | `S/app/event_loop.rs:109-157` | 치환 |
| SKEL-022 | 기동 파일·기동 명령 | 인자 파일 열기 → `NSQL_STARTUP_CMD`(쉼표 구분 · `@after:<ms>:` · `@connected:`) 해석 | `S/app/event_loop.rs:165-184` | 치환(§1-14 하네스) |

### 1-3. 보조 창 · 모달 창

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-023 | 보조 창 골격(본보기 = About) | 창마다 구조체 `{ window: Option<Rc<Window>>, surface: Option<Presenter>, scale, cursor, 컨트롤들 }` + 고정 메서드 `new` · `open(el, theme, owner)` · `close` · `window()` · `is_open()` · `is(id)` · `redraw()` · `handle(&WindowEvent) -> Action` · `paint(…)`. `close` = `surface = None; window = None` + 컨트롤 `clear_transient()` | `S/about_win.rs:27-55` · `57-87` · `117-145` · `147-224` · `227-325` | 그대로(틀) — dir2 대화상자마다 한 벌 |
| SKEL-024 | 창은 "의도"만 돌려준다 | `handle`이 `enum XxxAction { None, Paint, Close, … }`을 돌려주고 **실행은 App**(설정·클립보드·다른 창 열기)이 한다 — 창은 앱 상태를 모른다 | `S/about_win.rs:17-25` · `S/app/event_loop.rs:1477-1503` | 그대로 |
| SKEL-025 | 그리기 요청도 Action | `RedrawRequested` → `Action::Paint` → App이 글꼴·테마·데이터를 인자로 `paint` 호출(창이 `App` 필드를 빌리지 않는다) | `S/about_win.rs:151` · `S/app/event_loop.rs:1479-1484` | 그대로 |
| SKEL-026 | 보조 창 공통 호스트 `winhost` | `OpenSpec{title, theme, near, dy, owner, memo, default_size, ime}` → `open_window` = 속성 → 생성 → 기억 자리 재배치(`place_outer`) → 화면 안(`keep_on_screen`) → `Presenter` → IME 허용 → `Opened{window, surface, scale}`. 사용처 5창(로그·메모리·세션·트랜잭션 로그·변수) | `S/winhost.rs:12-67` · `S/log_win.rs:700-732` | 그대로 · **승격 후보**(§5-8) |
| SKEL-027 | 닫기 꼬리(기하 기억) | `last_geom(w)` → 창의 `take_last()` → App `persist_window_sizes`가 `window.<name>_pos/_size`에 저장 → `apply_window_sizes`가 `Memo`로 되돌려 준다 | `S/winhost.rs:70-72` · `S/log_win.rs:734-750` · `S/app/windows.rs:71-135` | 개명 |
| SKEL-028 | 소유 창 `owned_by` | 창 속성 단계에서 건다(OS별 — SKEL-202) · 작업표시줄 항목 1개 · 항상 메인 위 | `S/winfocus.rs:16-38` | 그대로 |
| SKEL-029 | 자식 붙이기 `attach_child` | 창이 생긴 **뒤** 건다(mac `addChildWindow` · Linux X11 `WM_TRANSIENT_FOR`+`_NET_WM_STATE_MODAL`) — `if !was_open { attach_child(owner, child) }` 꼴 | `S/winfocus.rs:97-121` · `S/app/event_loop.rs:1671-1683` · `S/app/connwin.rs:29-45` | 그대로 |
| SKEL-030 | 모달 판정 한 곳 | `modal_open()`(모달 창 목록의 OR) · `modal_window()`(앞으로 올릴 창) · `sync_modal()`(열림/닫힘 전환 때 메인+보조 창 `set_enabled(!open)` · 닫히면 `focus_window`) | `S/app/windows.rs:9-68` | 치환(dir3 모달 목록) |
| SKEL-031 | 모달 사건 가드 | `window_event` 초입: 모달이 열려 있고 사건이 모달 창 것이 아니면 키·마우스·휠·IME를 **버리고** 모달 창을 `focus_window()` | `S/app/event_loop.rs:604-627` | 그대로 |
| SKEL-032 | OS 수준 입력 차단 | `winfocus::set_enabled(w, on)`(OS별 — SKEL-204) | `S/winfocus.rs:53-79` | 그대로 |
| SKEL-033 | 보조 창 사건 분배 | `aux_window_event(el, id, &event) -> bool` — `if self.xxx_win.is(id) { match self.xxx_win.handle(event) { … } return true; }` 를 창마다 나열 · 처리했으면 메인 처리로 가지 않는다 | `S/app/event_loop.rs:628-633` · `911-1648` | 치환 |
| SKEL-034 | 창 열기는 깃발로 미룬다 | 메뉴·기동 명령·워커 응답은 `ActiveEventLoop`가 없으므로 `open_xxx: bool`/`xxx_pending: Option<…>` 깃발만 세우고, `el`이 있는 두 자리에서 소비: ① `window_event` 끝 `open_requested_windows(el)` ② `about_to_wait` 앞 `pump_window_requests(el)` | `S/app/event_loop.rs:902-903` · `1651-1763` · `1770-1975` · `S/main.rs:278-282` | 그대로 |
| SKEL-035 | 보조 창의 소유자 선택 | 설정 창에서 연 색·단축키 창 = **설정 창이 소유자**(메인 소유면 설정 창 뒤로 숨는다) | `S/app/event_loop.rs:1688-1705` · `1733-1751` | 그대로(규칙) |
| SKEL-036 | 모달 닫힘 뒤 포커스 복귀 창 | `picker_return: Option<WindowId>` — 파일 창을 띄운 보조 창이 살아 있으면 그 창으로, 아니면 메인으로 | `S/main.rs:345-346` · `S/app/files.rs:401` · `S/app/windows.rs:61-67` | 그대로 |
| SKEL-037 | 창 목록 단일 원천 | `all_windows()`(열린 것만) · `aux_window(id)` | `S/app/windows.rs:149-177` | 치환 |
| SKEL-038 | z-order · 그룹 올리기 | `z_order: Vec<WindowId>` · `Focused(true)`마다 `on_window_focused` → 설정 `window.focus = group`이면 `winfocus::raise_group`(활성화 없이) · 모달이 닫히면 죽은 id 걷기 | `S/app/windows.rs:42-46` · `180-193` · `S/winfocus.rs:124-173` | 그대로 |
| SKEL-039 | 항상 위 | `set_window_level(AlwaysOnTop/Normal)` — 설정 `window.always_on_top` | `S/app/windows.rs:138-146` | 개명 |
| SKEL-040 | 토글 창 | 열려 있으면 닫고(`persist_window_sizes`) 아니면 메인 오른쪽(`near`)에 연다 | `S/app/windows.rs:195-237` | 그대로 |
| SKEL-041 | 가운데 배치 | `wingeom::centered_over(attrs, owner, 논리폭, y_div)` — 주인 창 위 가로 가운데·세로 1/k | `S/wingeom.rs:148-165` · `S/about_win.rs:74-75` | 그대로 |
| SKEL-042 | 화면 밖 금지 | 생성 직후 `wingeom::keep_on_screen(&win, owner)`(순수 `clamp_into` + 시험) | `S/wingeom.rs:105-139` · 시험 `170-196` | 그대로 |
| SKEL-043 | 플로팅 도구 창 | 도크에서 떼어 낸 그룹 = 작은 OS 창(`with_resizable(false)` · `with_enabled_buttons(CLOSE)` · 소유 창) — 컨트롤은 도크가 계속 소유, 창은 표면·사건 변환만(`FloatAction::Input(ev)`) | `S/toolfloat.rs:1-80` · `S/app/toolbar.rs:111-158` · `S/app/event_loop.rs:1202-1242` | 그대로(dir2 툴바/도크 떼기 여부에 따라) |
| SKEL-044 | 파일 대화상자 = 자체 그리기 | OS 대화상자 0 — `nexa_dlg::FilePicker`를 자체 창(`file_win`)에 얹는다 · 용도는 `FilePurpose` 열거로 구분, 결과는 `FileWinAction::Confirm(mode, path, enc)` | `S/main.rs:2157` · `S/app/files.rs:338-401` · `S/app/event_loop.rs:917-1031` · `U/nexa-dlg/src/lib.rs:29` · `154` | 치환(dir3는 폴더 선택·대상 폴더 고르기 등) |
| SKEL-045 | 포커스 가져오기 통제 | `winfocus::focus(w)` — `NSQL_NO_ACTIVATE`가 있으면 하지 않는다 · 새 창은 `with_active(false)` | `S/winfocus.rs:177-181` · `S/icon.rs:175-181` | 개명 |
| SKEL-046 | 닫힌 창의 키가 새지 않게 | ① `is_synthetic` 누름은 어느 창이든 버린다 ② 입력 창을 키로 닫은 뒤 그 키가 떼어질 때까지 자동 반복을 버린다(`key_guard_step` 순수 함수 + 시험) | `S/app/event_loop.rs:517-539` · `S/main.rs:1206-1216` · `776-812` | 그대로 |

### 1-4. 이벤트 루프 · 재그리기 정책

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-047 | 처리기 | `impl ApplicationHandler<Wake> for App` — `resumed` · `user_event` · `exiting` · `about_to_wait` · `window_event` | `S/app/event_loop.rs:7-905` | 그대로 |
| SKEL-048 | 그리기는 `RedrawRequested`에서만 | `redraw()` = `window.request_redraw()`뿐 · 실제 `paint()`는 `WindowEvent::RedrawRequested`에서 | `S/main.rs:1126-1131` · `S/app/event_loop.rs:892-896` | 그대로 |
| SKEL-049 | 유휴 = 잠든다 | `about_to_wait` 끝에서 `el.set_control_flow(ControlFlow::WaitUntil(next))` — `next`는 "가장 이른 다음 일"의 시각 | `S/app/event_loop.rs:499` | 그대로 |
| SKEL-050 | 캐럿 깜빡임 게이트 | 0.5초 타이머가 **실제로 만료됐을 때만** 다시 그림 · 포커스가 글 입력이고 메인 창·앱이 활성일 때만 · 입력이 오면 위상을 "켜짐"으로 초기화 · 설정으로 끔 = 3600초 | `S/app/event_loop.rs:208-227` · `540-563` · `S/app/paint.rs:52-55` | 그대로(경로 입력란·이름 바꾸기·터미널 커서) |
| SKEL-051 | 틱 모으기 | 컨트롤·패널·창마다 `tick(now_ms) -> bool`(다시 그릴 것 있음) → OR 해서 `redraw()` · 보조 창은 자기 `redraw()` | `S/app/event_loop.rs:229-326` | 치환 |
| SKEL-052 | 애니메이션 중에만 프레임 타이머 | `bars_live`(스크롤바 표시·hover 페이드·툴팁 대기·토스트·배경 작업 진행 등의 OR)일 때만 `1000 / ui.max_fps` ms 뒤에 깬다 | `S/app/event_loop.rs:327-363` | 치환 |
| SKEL-053 | 깨움 시각 합성 | 하위 기능은 `xxx_tick(now) -> Option<Instant>`(다음에 깨워 달라)를 돌려주고 루프는 `next = next.min(t)` — 자체 타이머·스레드 0 | `S/app/event_loop.rs:364-498` | 그대로(패턴) |
| SKEL-054 | 크기·배율·이동 | `Resized` → `layout()+redraw()` · `ScaleFactorChanged` → `scale` 갱신 후 같은 것 · `Moved`에서도 배율을 다시 읽는다 | `S/app/event_loop.rs:651-673` | 그대로 |
| SKEL-055 | 활성/비활성 | `Focused(on)`(메인) → `main_active` · 비활성 = `pointer_gone()` · 활성 = 외부 변경 확인 · 깜빡임 위상 초기화 | `S/app/event_loop.rs:564-595` | 치환(활성화 때 폴더 새로 고침 판단) |
| SKEL-056 | 닫기(X) = 종료 흐름 | `CloseRequested` → `request_exit()`(저장·확인 물음) → 통과하면 `flush_on_exit` · `persist_window_sizes(true)` · 워커 `Quit` · `el.exit()` · 유휴 틱에서도 `exit_requested`를 본다(`finish_exit`) | `S/app/event_loop.rs:198-201` · `635-650` · `S/app/files.rs:181-233` | 치환(진행 중 파일 작업 확인) |
| SKEL-057 | OS 테마 변경 | `ThemeChanged` → System 모드일 때만 `apply_theme()` | `S/app/event_loop.rs:674-680` · `S/app/settings.rs:951-960` | 그대로 |
| SKEL-058 | 부분 무효화는 "다시 그릴까"의 신호 | 컨트롤은 `Invalidations`에 사각형을 쌓고, 호스트는 비어 있지 않으면 **창 전체**를 다시 그린다(부분 present 없음) | `S/app/input.rs:385` · `958-960` | 그대로 |
| SKEL-059 | 워커 깨움 | `user_event(Wake)` → `drain_all()` | `S/app/event_loop.rs:187-189` | 그대로 |
| SKEL-060 | 창 기하 저장 시점 | 유휴 틱마다 `persist_window_sizes(false)`(닫힌 보조 창의 1회성 값만) · 종료 때 메인 포함 | `S/app/event_loop.rs:206` · `644` | 그대로 |

### 1-5. 입력 변환 · 라우팅 · 포커스

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-061 | 수식키 상태 | `ModifiersChanged` → `shift` · `primary`(mac ⌘ / 그 밖 Ctrl) · `alt` · `ctrl_raw` · `ctrl_mac`(mac Control) 필드 | `S/app/event_loop.rs:681-706` · `S/main.rs:584-595` | 그대로 |
| SKEL-062 | 커서 위치·모양 | `CursorMoved` → `cursor` 저장 + `set_cursor`(스플리터 = Col/RowResize · 헤더 경계 = ColResize · 글 영역 = Text · 팝업이 덮으면 화살표) | `S/app/event_loop.rs:707-747` | 치환 |
| SKEL-063 | winit → `nexa_ctl::InputEvent` 변환(메인) | `ctl_event(&WindowEvent)`: `MouseMove` · `MouseDown{x,y,shift,primary}` · `MouseUp` · `RightDown` · 휠 · `Key{key,shift,primary}` · `Char{c}`(Backspace=`\u{8}` · Tab=`\t` · Space) | `S/app/input.rs:196-311` | 그대로(더블클릭·가운데 버튼·뒤로/앞으로 버튼이 필요하면 추가 — nexa-ctl `InputEvent` 확장 여부는 컨트롤 문서에서 확인) |
| SKEL-064 | 휠 변환 한 곳 | `input::wheel_event(delta, shift)`: LineDelta ×120 · PixelDelta ×3 · 3의 배수 양자화 + 잔여 이월 · 큰 축만(축 잠금) · mac 가로 부호 반전 · 자연 스크롤 설정 · Shift+세로 = 가로 | `S/input.rs:12-39` · `191-229` | 그대로 · 승격 후보 |
| SKEL-065 | 단축키는 라우팅보다 먼저 | 키 누름 → `Chord::from_winit(logical, physical, primary, shift, alt, ctrl_mac)` → 키맵 조회(2단 코드 `pending_chord` · 접두 판정 · 자동 반복은 `repeatable` 명령만) → `key_command(id, el)` | `S/app/event_loop.rs:830-890` · `S/keymap.rs:1161-1225` | 치환(dir2 단축키 표) |
| SKEL-066 | 명령 어휘 한 길 | 메뉴·툴바·팔레트·단축키·기동 명령이 전부 **명령 id 문자열**로 `menu_action(id)` 한 곳에 온다 · `el`이 필요한 것만 `key_command`가 먼저 처리 | `S/app/menus.rs:324-363` · `366-430` | 그대로(dir2 명령 id 표 필요) |
| SKEL-067 | 라우팅 진입 | `route(ev)` = 메뉴 배타 규칙 감싸기 → `route_dispatch`(포인터 기록·상태줄 hover) → `route_inner`(책임 연쇄) | `S/app/input.rs:315-352` · `385-961` | 그대로(틀) |
| SKEL-068 | 책임 연쇄 순서 | ① 토스트·카드 클릭 ② 포인터 캡처 ③ 창 위에 뜬 패널 메뉴 ④ 드래그 중인 컨트롤(Esc 취소 포함) ⑤ 팔레트(모달) ⑥ 완성 팝업 ⑦ 상태줄 팝업 ⑧ 스플리터 ⑨ 상태줄 클릭 영역 ⑩ 열린 메뉴바(모달) ⑪ 우클릭 진입점(툴바·거터) ⑫ 메뉴바·툴바(마우스) ⑬ 찾기 막대 ⑭ 그리드 메뉴 ⑮ 활동 막대 ⑯ 패널들(`route_<패널>`: 포인터 = 안일 때 · 키 = 포커스일 때) ⑰ 탭 바 ⑱ 팝업 사각형 안 사건 ⑲ 누름 = 포커스 이동 ⑳ 휠 = 커서 아래 · 나머지 = 포커스 대상 | `S/app/input.rs:385-961` · 고리 `965-1667` | 치환(dir3 영역으로 재구성 — §5-4) |
| SKEL-069 | 포커스 = 열거 하나 | `enum Focus { … }` · `set_focus(f)`가 모든 영역에 `set_focused(f == X)`를 뿌리고 `ime_refresh()` — 포커스 링 ≤ 1 | `S/main.rs:167-185` · `S/app/input.rs:88-100` | 치환 |
| SKEL-070 | 마우스 규칙 | 마우스·휠 = **커서 아래** 영역 · 키·글자 = **포커스** 영역 · 좌/우 누름 모두 그 영역으로 포커스 이동 | `S/app/input.rs:874-957` · `D/61-core-design-and-working-rules.md:94` | 그대로 |
| SKEL-071 | 포인터 캡처 | `press_capture: Option<Focus>` — MouseDown 때 `area_at(p)`를 기억, 영역 밖 Move/Up은 `dispatch_captured`로 누른 영역에(그 영역이 포커스일 때만) · Up으로 해제. 새 영역 = `area_at`/`area_bounds`/`dispatch_captured` 표에 한 줄 | `S/app/input.rs:476-515` · `1698-1826` | 그대로(파일 목록 러버밴드·열 폭 드래그·터미널 선택에 필수) |
| SKEL-072 | hover 이탈 통지 | 영역이 바뀌면 이전 영역에 `MouseMove{-1,-1}` 한 번 · `CursorLeft`/비활성 = `pointer_gone()`(hover·툴팁·캡처 전부 걷기) | `S/app/input.rs:479-495` · `1671-1695` · `S/app/event_loop.rs:575-586` | 그대로 |
| SKEL-073 | MouseUp은 드래그 가능한 컨트롤에도 | 커서가 어디에서 놓이든 편집기·그리드에 MouseUp 전달(멱등) | `S/app/input.rs:546-555` | 치환(캡처로 일반화되지만 안전망 유지) |
| SKEL-074 | 팝업 배타 | `open_menus()` 비트 집합을 사건 전후로 비교 → 새로 열린 메뉴가 있으면 나머지를 `close_menu_bits` · 풀다운이 열리면 `close_context_menus()` | `S/app/input.rs:315-324` · `704-709` · `S/app/menus.rs:307-313` · `1753-1828` | 그대로(틀) |
| SKEL-075 | 바깥 클릭 = 닫고 통과 | 열린 팝업은 사건을 먼저 받는다 · 안/키/휠 = 소비 · **바깥 클릭만** 닫은 뒤 아래로 흘린다 · 바깥 **이동**은 흘리지 않는다 | `S/app/input.rs:516-545` · `1166-1180` · `1216-1235` · `D/30-architecture-patterns.md:74` | 그대로 |
| SKEL-076 | 팝업 사각형 기준 라우팅 | 컨트롤 밖으로 펼쳐진 팝업 안의 사건은 커서 아래 영역이 아니라 **팝업의 소유 컨트롤**이 받는다(`popup_open()`·`popup_bounds()`) | `S/app/input.rs:843-873` | 그대로 |
| SKEL-077 | 우클릭 = 포인터 사건 | `is_ptr = is_mouse || RightDown` — 우클릭 메뉴가 있는 영역은 `is_ptr`로 판정 | `S/app/input.rs:466-475` | 그대로 |
| SKEL-078 | 스플리터 | `Splitter::on_event` → `SplitEvent::{Hover, Start, Drag(pos), End}` → 설정 값 갱신 → `layout()` · End에서 저장 | `S/app/input.rs:9-86` · `S/main.rs:607-608` · `1078-1105` | 치환(dir2 패널 분할·도크 분할) |
| SKEL-079 | 보조 창용 키 변환 | `input::text_key_event(kev, shift, primary, TextKeys::{Line, Multi, MultiTab})` — 순수 판정 `text_key_of` + 시험 · ⌘/Ctrl+글자는 `None`(호스트가 단축키로) | `S/input.rs:122-189` · 시험 `442-511` | 그대로 · 승격 후보 |
| SKEL-080 | 단축키 글자 = 물리 키 폴백 | `shortcut_letter_of(logical, physical)` — 논리 키가 ASCII가 아니면(한글 자모) 물리 `KeyA..Z`로 | `S/input.rs:45-93` · 시험 `358-440` | 그대로 · 승격 후보 |
| SKEL-081 | 파일 끌어다 놓기(받기) | `WindowEvent::DroppedFile(p)` → 열기(3-OS 공통 winit 경로). `HoveredFile`·끌어 **내기**는 없음(Grep 0건) | `S/app/event_loop.rs:598-603` | 치환 + **신규 필요**(dir3는 DnD 내보내기·대상 강조 필요 — §5-7) |
| SKEL-082 | 상태줄 클릭 영역 | 그릴 때 세그먼트 Rect를 필드에 기록 → 라우팅이 그 Rect로 판정(그리기가 히트 영역의 원천) | `S/app/paint.rs:263-337` · `S/app/input.rs:623-667` · `1151-1164` | 치환 |
| SKEL-083 | 우클릭 전 클립보드 탐침 | 메뉴가 열리기 전 "붙여넣기 가능" 여부를 넣어 준다(설정 `ui.clipboard_probe`로 끔) | `S/app/input.rs:454-465` | 치환(파일 붙여넣기 가능 여부) |
| SKEL-084 | 수식키 붙은 Space | ⌘/Ctrl/Control+Space는 글자가 아니다(키맵에 없으면 버림) | `S/app/input.rs:287-290` | 그대로 |
| SKEL-085 | Alt = 전체 경로 보기 | `nexa_ctl::draw::set_show_full(alt)` — 가운데 `…` 축약 해제 전역 스위치 | `S/app/event_loop.rs:694-697` | 그대로(경로 표시가 많은 dir3에 유용) |

### 1-6. 배치 · 그리기 · 팝업 층

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-086 | 배치 한 함수 | `layout()` — `inner_size()`(물리 px)와 `scale`로 모든 영역의 `Rect`를 계산해 `set_scale(s)` + `set_bounds(rect)` · 논리→물리 = `px(v, s)` | `S/main.rs:886-888` · `895-1117` | 치환(dir2 컨트롤 배치 그대로 재현) |
| SKEL-087 | 프레임 합성 | `paint()`: `surface.resize` → `buffer_mut` → `nexa_gfx::Surface::new(&mut buf, w, h)` → 층마다 `RasterCtx::new(&mut gfx, &font, scale).with_fonts(FontPrefs::with_base(px))` → `buf.present()` | `S/app/paint.rs:37-68` · `641` | 그대로(틀) |
| SKEL-088 | 층 순서(아래 → 위) | ① 배경·탭·툴바·상태줄(UI 글꼴) ② 본문(고정폭) ③ 그리드 ④ 찾기·도구줄 글·탭 툴팁 ⑤ 옆 패널들 ⑥ 스플리터 ⑦ 툴팁·드래그 고스트·패널 팝업 → 상태줄 메뉴 → 토스트·카드 → 그리드/편집기 팝업 → 팔레트 → 진행 막 ⑧ **메뉴바 + 드롭다운(맨 마지막)** | `S/app/paint.rs:62-638` | 치환 — **팝업은 항상 마지막 패스** 규칙 유지 |
| SKEL-089 | 글꼴 층 분리 | 층마다 다른 `Font`·크기의 `RasterCtx`를 새로 만든다(UI `ui.font_size` · 편집기 `editor.font_size` · 그리드 `grid.font_size` · 메뉴 `ui.menu_font_size` · 탐색기 `explorer.font_size`) · 고정폭 슬롯은 `FontSet{ mono, ..FontSet::single(f) }` | `S/app/paint.rs:64-67` · `378` · `429-436` · `512-517` · `628-636` | 치환 |
| SKEL-090 | 팝업 영역 = 창 전체 | 우클릭 메뉴의 호스트 Rect는 패널이 아니라 창 전체(`set_menu_area(Rect::new(0,0,w,h))`) | `S/main.rs:1011-1014` · `S/app/toolbar.rs:30-40` | 그대로 |
| SKEL-091 | 그리기 전 값 계산 | `surface`를 가변으로 빌리기 전에 상태 글·툴팁 자리를 미리 계산(빌림 충돌 회피) · 필드만 빌리는 연관 함수(`license_badge_of(&self.licensing)`) | `S/app/paint.rs:9-36` · `S/app/license.rs:55-60` | 그대로(규칙) |
| SKEL-092 | 보조 창 프레임 상용구 | `let Some(mut surface) = self.surface.take()` → `surface.frame(size)`(크기 0·실패 = `None` = 이번 프레임 건너뜀) → 그림 → `buf.present()` → `self.surface = Some(surface)` | `S/about_win.rs:234-244` · `322-323` · `S/present.rs:96-102` | 그대로 |
| SKEL-093 | 창 크기를 내용에 맞추기 | 첫 페인트에서 필요한 높이를 재어 `request_inner_size` | `S/about_win.rs:285-297` | 그대로 |
| SKEL-094 | 툴바 재배치 안전망 | 글자 항목 폭이 바뀌면 paint 뒤 `tool_dock.relayout_if_stale` → 다시 그림 | `S/app/paint.rs:76-80` · `D/61-core-design-and-working-rules.md:130` | 그대로 |
| SKEL-095 | 토스트 | 앱 쪽 부품 `toast::Toasts`(push · tick · click · paint · 최대 5장 · 수명·불투명도 설정) — 팝업보다 **아래** 층 | `S/toast.rs:18-200` · `S/app/paint.rs:594-609` | 그대로 · 승격 후보(nexa-ctl에는 `flash.rs`만 있다 — `U/nexa-ctl/src/controls/` 목록) |
| SKEL-096 | 프레임 계측 | `FrameTrace`(60프레임마다 구간 평균/최대) · `tmark`(입력→present 지연) — 환경 변수가 있을 때만 | `S/main.rs:1119-1124` · `2007-2066` · `S/app/paint.rs:17-28` · `640-659` | 개명 |
| SKEL-097 | 그리기 원칙(26번 문서 요지) | 가시 영역만 그린다 · 페인트 경로 할당 0을 향한다 · 중간 산출물은 단계가 끝나면 버린다 · 프레임 예산 8 ms · 창을 닫으면 프레임버퍼를 즉시 놓는다 · 숨김 창을 만들지 않는다 · 워킹셋 트림은 하지 않는다 | `D/26-performance-architecture.md:8-14` · `89-101` · `108-118` · `168-174` | 그대로(파일 목록 가상화·썸네일 캐시 상한에 적용) |

### 1-7. IME

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-098 | IME 사건 처리 | `WindowEvent::Ime`: `Preedit(t)` → `tb.set_preedit` · `Commit(t)` → preedit 비우고 글자마다 `InputEvent::Char` — 대상은 `focused_textbox()` | `S/app/event_loop.rs:748-794` · `S/app/input.rs:177-194` | 그대로 |
| SKEL-099 | IME 허용 토글 | `ime_refresh()` — 글 입력 포커스(또는 팔레트 열림)일 때만 `set_ime_allowed(true)` · 값이 바뀔 때만 OS 호출(`ime_last`) | `S/app/input.rs:115-136` | 치환(경로 입력·이름 바꾸기·필터·터미널) |
| SKEL-100 | 한글 조합 방식 맞춤 | 설정 `input.hangul_compose`(auto/system/app) × OS × 입력 소스 → 순수 판정 `hangul_app_mode` → `nexa_ctl::controls::set_hangul_app_compose` + 전역 `set_system_ime` + **모든 창** `set_ime_allowed(!app)` · 부르는 때 = 기동·창 활성화·입력 소스 바뀜·설정 변경 | `S/app/input.rs:138-175` · `S/input.rs:95-120` · `S/app/event_loop.rs:128-130` · `202-205` · `570-571` | 그대로 |
| SKEL-101 | 목록 포커스 = 타입어헤드 | `typeahead_target()`이면 IME를 끊고, Windows는 한/영 키를 앱이 토글해 `nexa_ctl::hangul::jamo_from_qwerty`로 자모 변환 | `S/app/input.rs:102-113` · `296-303` · `S/app/event_loop.rs:796-813` | 그대로(파일 목록 앞글자 점프의 핵심) |
| SKEL-102 | 명령 앞 조합 확정 | `menu_action` 초입에서 `tb.commit_composition()` | `S/app/menus.rs:367-374` | 그대로 |
| SKEL-103 | 조합 중 글자도 즉시 필터 | IME 사건 뒤 포커스 패널의 `query_changed()` 호출(자모 완성과 무관하게 검색) | `S/app/event_loop.rs:781-791` | 치환(파일 필터) |
| SKEL-104 | 팝업 입력란의 IME | `Focus` 변형이 아닌 입력란(팔레트)은 IME 사건을 먼저 받게 분기 | `S/app/event_loop.rs:750-769` | 그대로(규칙) |
| SKEL-105 | IME 상태 읽기 | `imestate::current(hwnd) -> Option<ImeState{glyph, lang, latin}>`(OS별 — SKEL-218) · 순수 `classify`·`engine_state` + 시험 | `S/imestate.rs:71-80` · `146-162` · `164-265` · `295-315` | 그대로(가린 입력란이 있을 때만 필요) |
| SKEL-106 | 가린 입력란 언어 안내 | `ImeHint`(150 ms 폴링 · 창의 `tick`이 다음 깨움 시각 반환 · 그림 = `draw_tooltip_in`) | `S/imehint.rs:15-163` | dir3에 비밀번호 칸이 있으면 그대로 · 없으면 불요(추정: 네트워크 드라이브 인증·라이선스 입력 여부에 따름) |
| SKEL-107 | Linux IME 감시 | `dbus-monitor`로 ibus 패널 메시지 엿듣기(자식 프로세스 1 + 스레드 1 · 소유자 토큰) | `S/imewatch.rs:17-181` | 위와 같음 |
| SKEL-108 | IME 진단 | `NSQL_TRACE_IME=1` → 창·키·IME 사건을 온 순서대로 stderr | `S/app/event_loop.rs:503-516` · `S/input.rs:108-112` | 개명 |

### 1-8. 클립보드

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-109 | API | `clipboard::read_text() -> Option<String>` · `write_text(&str) -> bool` · `write_rich(text, html) -> bool` — 컨트롤은 OS를 모르고 호스트가 잇는다 | `S/clipboard.rs:1-29` | 그대로 + **신규 필요**: 파일 목록 형식(CF_HDROP / NSPasteboard file URL / `text/uri-list`·`x-special/gnome-copied-files`) — nexa-sql에는 없다 |
| SKEL-110 | Windows 구현 | user32/kernel32 직접 FFI · 5회 재시도 열기 · `CF_UNICODETEXT` · `HTML Format`(CF_HTML 컨테이너 `cf_html`) | `S/clipboard.rs:31-50` · `52-203` | 그대로 |
| SKEL-111 | macOS 구현 | `pbpaste`/`pbcopy` 프로세스 · 서식 = `osascript`(«class HTML») | `S/clipboard.rs:205-265` | 그대로(파일 복사는 별도 구현 필요) |
| SKEL-112 | Linux 구현 | X11 `CLIPBOARD` selection 직접(전용 스레드가 소유권 유지 · INCR · 1.2초 읽기 상한) → 실패 시 `wl-paste`/`xclip`/`xsel` 폴백 · 설정 `clipboard.x11_native` | `S/clipboard_x11.rs:1-334` · `S/clipboard.rs:214-298` · `S/main.rs:21-30` | 그대로(대상 형식 추가) |
| SKEL-113 | 호스트 접착 | 컨트롤이 `take_edit_ctx() -> Option<EditCtxAction>`으로 요청 → `clip_action(act)` · **잘라내기는 클립보드에 올라간 것을 확인한 뒤에만 지운다** · 실패 = 상태줄 | `S/app/menus.rs:1330-1442` · `S/app/input.rs:952-956` | 그대로 |
| SKEL-114 | 클립보드 진단 | `NSQL_TRACE_CLIP` | `S/clipboard.rs:271-273` · `S/clipboard_x11.rs:92-101` | 개명 |

### 1-9. DPI · 창 기하

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-115 | 좌표계 규약 | 컨트롤 bounds·마우스 = **물리 px** · 설계 치수 = 논리 px × `scale`(`px()`) · 저장하는 창 위치·크기 = **논리 좌표**(모니터마다 배율이 달라 물리 px는 섞이면 틀린다) | `S/main.rs:886-888` · `S/wingeom.rs:14-36` | 그대로 |
| SKEL-116 | 모니터 사각형 | `monitor_rect(m)` = 물리 위치·크기 ÷ 그 모니터 배율 · `on_any_monitor`(없어진 모니터 좌표는 버림) | `S/wingeom.rs:58-81` | 그대로 |
| SKEL-117 | 기억 기하 `Memo` | `{pos, size}` · `on_same_monitor(owner)` — 주인 창과 같은 모니터일 때만 기억 값 사용 | `S/wingeom.rs:38-56` | 그대로 |
| SKEL-118 | 프레임 기준 재배치 | `place_outer` — 생성 인자 `with_position`(내용 영역 기준)과 `outer_position`(프레임 기준)의 차이를 생성 뒤 `set_outer_position`으로 보정 | `S/wingeom.rs:94-101` | 그대로 |
| SKEL-119 | 파싱·형식 | `parse_size`(< 200×150 무시) · `parse_pos` · `format_*` + 시험 | `S/wingeom.rs:7-36` · `198-205` | 그대로 |
| SKEL-120 | 컨트롤 배율 배선 | 컨트롤을 직접 놓을 때 `set_bounds`와 함께 **`set_scale(scale)`** 필수 | `S/main.rs:908-916` · `S/about_win.rs:304` · `D/61-core-design-and-working-rules.md:175` | 그대로(규칙) |

### 1-10. present(화면 내보내기)

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-121 | `Presenter` 한 곳 | 모든 창이 `present::Presenter`로 픽셀을 낸다(창 코드에 `softbuffer::Context/Surface` 직접 금지) · 모양 = `resize` → `buffer_mut` → 그림 → `Buffer::present` · `Buffer`는 `Deref<[u32]>`(`0x00RRGGBB`) | `S/present.rs:30-143` · `D/61-core-design-and-working-rules.md:172` | 그대로 · 승격 후보 |
| SKEL-122 | 뒷단 선택 | 기본 softbuffer · macOS + 설정 `gfx.mac_present = iosurface`면 `nexa_sys::layer_present::LayerPresenter`(실패 = 조용히 softbuffer) · 순수 판정 `use_layer` + MC/DC · 방식은 **창을 만들 때** 고정 | `S/present.rs:17-28` · `55-72` · `145-157` · `159-169` · `S/main.rs:1923-1924` | 그대로 |
| SKEL-123 | 앱 활성 판정 | `nexa_sys::layer_present::app_active()`(mac `NSApplication.isActive` · 그 밖 `None`) — 뒤에 있으면 깜빡임 그리기 생략 | `S/app/event_loop.rs:220-226` · `S/app/paint.rs:52-55` · `U/nexa-sys/src/layer_present.rs:49` | 그대로 |

### 1-11. 워커 스레드 · 통지

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-124 | 깨움 사건 | `struct Wake;`(내용 없는 사용자 사건) · 스레드는 `EventLoopProxy<Wake>`를 `Box<dyn Fn() + Send>` 클로저로만 받는다(winit을 모른다) | `S/main.rs:163-165` · `1368-1387` · `S/worker.rs:477-482` | 그대로 |
| SKEL-125 | 채널 + 깨움 = 통지 규약 | 워커: `mpsc::Sender<Cmd>`(UI→워커) · `mpsc::Receiver<Event>`(워커→UI) · 결과를 보낸 뒤 `wake()` → UI `user_event` → `try_recv` 루프로 전부 수거(`drain_*`) | `S/worker.rs:419-498` · `S/app/event_loop.rs:187-189` · `S/app/session.rs:408-417` · `S/app/events.rs:813-816` | 그대로(파일 열거·복사/이동·검색·썸네일·터미널 출력) |
| SKEL-126 | 명령 열거 | `enum Cmd { …, Quit }` — 워커는 `while let Ok(cmd) = rx.recv()` 순차 처리 | `S/worker.rs:30-113` · `745` | 치환 |
| SKEL-127 | 패닉 격리 | 명령마다 `catch_unwind(AssertUnwindSafe(…))` — 패닉해도 워커는 살아남고 오류 사건을 낸다 | `S/worker.rs:8-9` · `745-747` | 그대로(단 `panic = "abort"` 릴리스 프로필에서는 catch가 듣지 않는다 — SKEL-343) |
| SKEL-128 | 요청당 짧은 스레드 | 오래 막힐 수 있는 독립 요청(접속 테스트)은 요청마다 스레드 + 결과 채널 + `wake()` · 동시 상한 + FIFO 큐는 호스트가 | `S/worker.rs:1585-1617` · `S/app/connwin.rs:49-80` · `S/main.rs:578-582` | 치환(폴더 크기 계산·네트워크 경로 탐침) |
| SKEL-129 | 취소 | `Arc<AtomicBool>` 깃발(배치 경계에서 확인) · 막힐 수 있는 취소 호출은 별 스레드에서 | `S/worker.rs:425-474` | 그대로(파일 작업 취소) |
| SKEL-130 | 워커가 UI에 묻는다 | 워커가 "물음" 사건을 내고 **전용 답 채널**을 기다린다(`InputReply`·`PwReply`) → UI는 깃발 → `pump_window_requests`가 모달 창을 연다 → 답을 채널로 | `S/worker.rs:404-448` · `S/app/event_loop.rs:1909-1955` | 그대로(덮어쓰기 확인·권한 상승 물음) |
| SKEL-131 | 스레드 없이 폴링하는 일 | 일회성 스레드의 `Receiver`를 필드에 두고 유휴 틱에서 `try_recv` | `S/app/event_loop.rs:1959-1964` · `S/main.rs:394-395` | 그대로 |
| SKEL-132 | UI 스레드는 기다리지 않는다 | 무거운 준비는 스레드에서, UI는 옮겨 받기만 · 창 전체 입력을 막지 않는다(막는 범위 = 그 일의 대상 하나) | `D/61-core-design-and-working-rules.md:33-35` · `D/01-architecture.md:51` | 그대로(원칙) |
| SKEL-133 | 스레드 이름 | `thread::Builder::new().name("nsql-…")` | `S/worker.rs:465-466` · `496-497` · `S/main.rs:1426-1427` | 개명(`ndir-…`) |

### 1-12. 아이콘 · 테마 · 메뉴 · 툴바 · 설정 반영

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-134 | 앱 아이콘 = 코드 래스터 | `icon_rgba(side)`(4×4 슈퍼샘플 · 256 좌표계 도형) → `winit::window::Icon::from_rgba` · 모든 창이 `with_icon(attrs)` 한 곳을 지난다 | `S/icon.rs:73-183` | 치환 — dir3는 **dir2 아이콘 자원을 그대로** 써야 하므로 `include_bytes!` 디코드 경로로(추정: `U/nexa-gfx/src/image.rs`·`inflate.rs`·`jpeg.rs`가 있으나 디코더 범위는 미확인) |
| SKEL-135 | 테마 해석 | `theme::resolve(mode, window_theme)` = winit 창 판정 → OS 조회 → `Theme::dark()/light()` · `window_theme(mode)`로 제목줄도 맞춤 | `S/theme.rs:18-44` · 시험 `160-180` | 그대로(dir2 테마가 2종보다 많으면 팔레트 확장 — 설정 문서 참조) |
| SKEL-136 | 테마 적용 | `apply_theme()` — 팔레트 교체 · `set_theme` · 전 창 다시 그림 | `S/app/settings.rs:951-969` | 그대로 |
| SKEL-137 | 메뉴바 = 자체 그리기(3-OS 동일) | `nexa_ctl::MenuBar::new(build_menus())` — `MenuDef` + `MenuEntry::{Item, Disabled, Separator, sub}` + `ComboItem::new(id, label)` · **macOS 네이티브 메뉴(NSMenu) 없음**(Grep 0건) · 창 안 맨 위에 그린다 | `S/main.rs:1502` · `S/app/menus.rs:1026-1140` · `S/app/paint.rs:626-638` | 그대로(dir2 메뉴 구조를 `MenuDef`로 옮김) |
| SKEL-138 | 메뉴 재구성·언어 전환 | 상태가 바뀌면 `rebuild_menus()` · 언어가 바뀌면 `relabel()`(메뉴·툴바·창 라벨 재생성 후 `layout`) | `S/app/menus.rs:1737-1749` · `S/app/settings.rs:971-1007` | 그대로 |
| SKEL-139 | 메뉴 장식 | `set_edit_menu_decor`(아이콘·단축키 표시) — 부팅·키맵 변경 때 | `S/app/menus.rs:1713-1735` | 그대로 |
| SKEL-140 | 우클릭 메뉴 | `nexa_ctl::controls::ctxmenu::ContextMenu` 인스턴스 · `open_at(x, y, items, host, min_w)` · `take_picked()` → id 문자열 · 여러 용도가 한 인스턴스(`status_menu`)를 빌려 쓴다 | `S/main.rs:312` · `S/app/toolbar.rs:9-41` · `S/app/input.rs:1166-1180` | 그대로(**셸 컨텍스트 메뉴의 OS 항목은 별도** — §5-7) |
| SKEL-141 | 툴바 | `ToolDock::new(vec![ToolGroup::new(id, title, vec![ToolItem::new(id, 아이콘).tip(…), ToolItem::separator(), ToolItem::text(…).with_dropdown()])])` · 표시 여부 `toolbar.hidden` · 배치 `toolbar.layout` · `take_clicked()` → `menu_action` · `take_actions()` → `DockAction::{Float, LayoutChanged, Resized}` | `S/app/toolbar.rs:43-108` · `248-318` · `S/app/input.rs:694-710` | 치환(dir2 툴바 항목) |
| SKEL-142 | 명령 팔레트 | 명령 목록 `(id, "메뉴: 항목")`을 모아 `palette.open(prefill)` · 프롬프트 응답 = `PaletteAction::Prompt{id, text}` | `S/app/menus.rs:1464-1710` · `S/app/input.rs:966-1014` | dir2에 팔레트/퀵 런처가 있으면 치환(18번 문서 참조) |
| SKEL-143 | 상태줄 | 호스트가 직접 그린다(오른쪽에서 왼쪽으로 세그먼트 · 클릭 가능 항목 hover·눌림 · 왼쪽 문구는 세그먼트 앞에서 자름) | `S/app/paint.rs:88-373` | 치환 |
| SKEL-144 | 설정 즉시 반영 | `apply_setting(key) -> bool`(도메인 조각별 분배 · 모르는 키 = `false` = "재시작 필요") → `layout()+redraw()` | `S/app/settings.rs:230-260` | 치환 |
| SKEL-145 | 설정 저장·외부 편집 감시 | `persist_settings()` · settings.json 내보내기 후 1초 폴링으로 바뀐 키만 반영 | `S/app/settings.rs:8-93` · `982-986` | 치환(설정 문서 참조) |
| SKEL-146 | OS 연결 프로그램 열기 | `open_external(path)`: `cmd /C start` · `open` · `xdg-open` | `S/main.rs:1191-1204` | 그대로(+ `nexa_fs::shell::reveal_in_file_manager` `U/nexa-fs/src/shell.rs:856`) |
| SKEL-147 | 상세 로그 매크로 | `dlog!(self, layer, level, make)` — 게이트가 꺼져 있으면 인자를 평가하지 않는다 | `S/main.rs:641-677`(10-05 §75 정정 · 종전 628-663) | 치환 ✅ dir3 f2f2f1d(`dlog!` + `detail_push` · 10-05 §77) |
| SKEL-148 | 라이선스 접착(참고) | `licensing: nsql_license::Licensing` 필드(단일 원천) · 상태줄 배지 클릭 → `open_license` 깃발 → 모달 창 · `Focused(true)`마다 `licensing.refresh()` · 기능 게이트 `lic_gate(Feature)`는 UI 진입점 1곳 | `S/main.rs:227-230` · `S/app/event_loop.rs:566-569` · `S/app/license.rs:8-23` · `209` · `S/main.rs:1183-1184` | 치환(정책은 dir2 계승 — 라이선스 문서 몫) |

### 1-13. 빌드 · 단일 실행 파일 리소스

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-149 | GUI 크레이트 의존 | nexa-ui: `nexa-gfx` · `nexa-ctl` · `nexa-font` · `nexa-fs` · `nexa-dlg` · `nexa-sys{features=["gui"]}` · 창 = `winit 0.30{wayland-dlopen}` · 표면 = `softbuffer 0.4{wayland-dlopen}` | `R/crates/nexa-sql/Cargo.toml:32-45` | 그대로(+ `nexa-conf`는 설정 크레이트 경유) |
| SKEL-150 | OS 한정 의존 | Linux/BSD: `x11rb 0.13`(기본 기능 끔) · macOS: `objc2 0.5` · `objc2-app-kit 0.2` · `objc2-foundation 0.2`(winit과 같은 판 = 중복 0) · Windows: 외부 crate 없이 `#[link(name = "user32")]` 직접 FFI | `R/crates/nexa-sql/Cargo.toml:47-67` · `S/winfocus.rs:56-59` | 그대로(dir3는 Windows 셸 연동 때문에 FFI 범위가 훨씬 넓다 — 19번 문서) |
| SKEL-151 | 형제 저장소 경로 의존 | `nexa-xxx = { path = "../nexa-ui/crates/nexa-xxx" }` · `nexa-license = { path = "../nexa-license/crates/nexa-license" }` | `R/Cargo.toml:69-88` | 그대로(형제 폴더 배치 전제 — CI·문서에 명시) |
| SKEL-152 | Windows exe 리소스 | `build.rs`가 `include!("../../packaging/windows/winres.rs")` → `embed_windows_resources("<앱>.rc", "<stem>")` — 버전 define 래퍼 `.rc` 생성 → `rc.exe`/`llvm-rc`/`windres` → `cargo:rustc-link-arg-bins` · 도구가 없으면 조용히 건너뜀 · `.rc` = `1 ICON` + `VERSIONINFO` | `R/crates/nexa-sql/build.rs:1-6` · `R/packaging/windows/winres.rs:19-89` · `R/packaging/windows/nexa-sql.rc` | 개명(dir2의 `.ico`·버전 정보 문구 유지) |
| SKEL-153 | Windows 정적 CRT | `.cargo/config.toml` `rustflags = ["-C", "target-feature=+crt-static"]`(x86_64·aarch64 msvc) | `R/.cargo/config.toml:9-13` | 그대로 |
| SKEL-154 | 릴리스 프로필 | `opt-level 3` · `lto = "fat"` · `codegen-units = 1` · `panic = "abort"` · `strip = "symbols"` | `R/Cargo.toml:90-95` | 그대로 |
| SKEL-155 | 개발 빌드의 의존 최적화 | `[profile.dev.package."*"] opt-level = 2`(래스터·컨트롤만 빠르게) | `R/Cargo.toml:97-101` | 그대로 |
| SKEL-156 | 정적 자원 임베드 | 텍스트 자원 = `include_str!`(SVG·SQL 예제) · 아이콘 = 코드 래스터 → 실행 파일 하나로 배포 | `S/dbms_icons.rs:13-30` · `S/main.rs:2619` | 치환(dir2 아이콘·문자열·샘플 = `include_bytes!`/`include_str!`) |
| SKEL-157 | 작업공간 린트 | `missing_debug_implementations` · `unreachable_pub` · clippy `unwrap_used` 등 = warn(CI는 `-D warnings`) · 크레이트 `[lints] workspace = true` | `R/Cargo.toml:41-48` · `R/crates/nexa-sql/Cargo.toml:13-14` | 그대로 |
| SKEL-158 | OS 패키징 자리 | `packaging/{branding,windows,macos,linux,homebrew}`(ico·icns·png · wxs · Info.plist · .desktop·deb·rpm) | `R/packaging/` 목록 | 치환(릴리스 단계) |

### 1-14. 자체 검증 · 계측 훅

| ID | 요소 | nexa-sql 구현 방식 | 위치 | dir3 적용 방법 |
| --- | --- | --- | --- | --- |
| SKEL-159 | 기동 명령 | `NSQL_STARTUP_CMD=<명령 id>,…` — 키 주입 없이 특정 화면을 띄운다 · `@after:<ms>:<명령>` 시차 · 덤프 명령(`*.dump:<파일>`)으로 상태를 파일에 | `S/app/event_loop.rs:162-184` · `395-425` · `S/app/startup_cmd.rs:9-` · `D/61-core-design-and-working-rules.md:236-244` | 개명 — **회귀 하네스의 뼈대** |
| SKEL-160 | 앱 안 마우스 사건 | `ui.move:x/y` · `ui.click` · `ui.dclick` · `ui.rclick` · `ui.wheel` — OS 주입이 아니라 `InputEvent`를 만들어 실제 `route` 경로에 넣는다 | `S/app/startup_cmd.rs:24-60` | 그대로 |
| SKEL-161 | 설정 폴더 격리 | `NSQL_HOME`이 있으면 그것이 설정 폴더 | `R/crates/nsql-settings/src/lib.rs:154-161` | 개명(`NDIR_HOME` — 제안) |
| SKEL-162 | 비활성 기동 | `NSQL_NO_ACTIVATE=1` — 창을 활성화하지 않고 띄운다 | `S/icon.rs:175-181` · `S/winfocus.rs:175-181` | 개명 |
| SKEL-163 | 추적 변수 | `NSQL_TRACE_FRAMES` · `_IME` · `_CLIP` · `_WINDOW`(모니터 목록) · `_MEM` | `S/main.rs:1680-1681` · `S/app/event_loop.rs:53-64` | 개명 |
| SKEL-164 | 분기 = 순수 함수 + MC/DC | 조건 2개 이상인 판정은 순수 함수로 떼어 시험(`use_layer` · `hangul_app_mode` · `key_guard_step` · `text_key_of` · `clamp_into`) | `S/present.rs:159-169` · `S/input.rs:335-356` · `S/main.rs:776-812` · `D/61-core-design-and-working-rules.md:94` | 그대로(규칙) |
| SKEL-165 | 전역 상태 시험 직렬화 | 전역을 만지는 시험은 정적 뮤텍스 가드로 | `S/input.rs:235-246` · `S/imestate.rs:381-382` | 그대로 |
| SKEL-166 | 실제 장치가 필요한 시험 | `#[ignore]` + 환경 변수로 입력(X11 클립보드 왕복 · 실 IME 조회) | `S/clipboard_x11.rs:336-388` · `S/imestate.rs:384-397` | 그대로 |
| SKEL-167 | 메모리 회수·사용량 | `memtrim::trim()` · `usage()`(OS별 — SKEL-225) | `S/memtrim.rs:12-185` | 그대로(썸네일·큰 폴더를 닫은 뒤) |

---

## 2. 모듈 구성(nexa-sql GUI 크레이트의 층)

```text
crates/nexa-sql/
├─ build.rs                      Windows exe 리소스(본체 = packaging/windows/winres.rs)          SKEL-152
├─ Cargo.toml                    nexa-ui 6크레이트 + winit + softbuffer + OS 한정(x11rb·objc2)   SKEL-149·150
└─ src/
   ├─ main.rs                    ① 모듈 선언 ② struct App(상태 전부) ③ Focus 열거 ④ layout() ⑤ main()
   ├─ app/                       ★ impl App 조각(동작) — 상태는 main.rs 한 곳                    SKEL-011
   │  ├─ event_loop.rs           ApplicationHandler · aux_window_event · open_requested_windows · pump_window_requests
   │  ├─ input.rs                ctl_event · route* 책임 연쇄 · set_focus · ime_refresh · 포인터 캡처
   │  ├─ paint.rs                paint()(층 합성)
   │  ├─ windows.rs              모달 판정 · 창 기하 저장 · z-order · 보조 창 토글
   │  ├─ menus.rs                build_menus · menu_action(명령 한 길) · key_command · clip_action · 팔레트 · 메뉴 배타 비트
   │  ├─ toolbar.rs              ToolDock 구성 · 플로팅 · 배치 저장
   │  ├─ settings.rs             apply_setting · apply_theme · relabel · persist_settings
   │  ├─ events.rs               워커 응답 수거(drain_events)
   │  ├─ startup_cmd.rs          기동 명령(자체 시험)
   │  └─ (도메인) session·run·tx·files·find·grid_results·project·… 
   ├─ [호스트 껍질 — OS와 닿는 얇은 층]
   │  present.rs   Presenter/Buffer(softbuffer | IOSurface)        SKEL-121~123
   │  winhost.rs   보조 창 열기/닫기 꼬리                            SKEL-026~027
   │  wingeom.rs   창 기하(논리 좌표·모니터·기억·화면 안)             SKEL-115~119
   │  winfocus.rs  소유 창·자식·모달 차단·그룹 올리기·HWND            SKEL-028~032·038
   │  theme.rs     OS 테마 조회 + 모드 해석                          SKEL-135
   │  icon.rs      창/작업표시줄/Dock 아이콘 · app_id                SKEL-134
   │  input.rs     휠 변환 · 보조 창 키 변환 · 단축키 글자 · IME 전역 스위치   SKEL-064·079·080·100
   │  clipboard.rs · clipboard_x11.rs                               SKEL-109~112
   │  imestate.rs · imehint.rs · imewatch.rs                        SKEL-105~107
   │  toast.rs     토스트                                           SKEL-095
   │  memtrim.rs · memstat.rs                                       SKEL-167
   ├─ [보조 창 — 창 하나 = 파일 하나]  about_win · license_win · prefs_win · colors_win · keys_win · file_win ·
   │                                   conn_win · log_win · txlog_win · sessions_win · vars_win · input_win · import_win ·
   │                                   sqlprev_win · drop_win · mem_win · toolfloat
   ├─ [메인 창 패널 — 영역 하나 = 파일 하나]  activity · explorer(s) · project_panel · search_panel · bookmarks_panel ·
   │                                          outline_panel · ext_panel · objdetail · findbar · filterbar · palette · results · grid · editors
   └─ [작업 스레드]  worker.rs(명령 큐 워커) · probe.rs · parwalk.rs · fileload.rs · gitstat.rs
```

**nexa-ui 크레이트가 맡는 것(앱이 직접 부르는 것만 · 확인된 범위)**

| 크레이트 | 앱이 쓰는 것 | 근거 |
| --- | --- | --- |
| `nexa-gfx` | `Surface::new(&mut [u32], w, h)` · `Font` · 전역 텍스트 설정(`text::set_glyph_cache_max` · `set_text_contrast/snap/hint/gdi/weight` · `set_tab_cols`) | `S/main.rs:133` · `1825` · `S/app/paint.rs:57` · `S/app/settings.rs:140-149` |
| `nexa-ctl` | `Control`/`Widget` 트레이트 · `InputEvent` · `Key` · `Invalidations` · `RasterCtx`/`FontSet` · `DrawCtx`/`FontSlot` · `Theme`/`FontPrefs`/`SlotFont` · `geom::{Point, Rect}` · 컨트롤(`MenuBar` · `ToolDock` · `TextBox` · `Button` · `Splitter` · `ContextMenu` …) · 전역 토큰(`tokens::set_*`) · `hangul` · `typeahead` | `S/main.rs:122-131` · `1932-1951` · 컨트롤 파일 목록 `U/nexa-ctl/src/controls/` |
| `nexa-font` | `ui_font(pref)` · `mono_font(pref)` → `{font, chain}` | `S/main.rs:1304-1321` |
| `nexa-fs` | `list`/`list_opts` · `Entry` · `sort_by` · `natural_cmp` · `places` · `drives` · `shell::{IconService, icon_for_path, set_os_icons, set_icon_cache_max, reveal_in_file_manager}` · `watch::StatWatch` · `path::display` | `U/nexa-fs/src/lib.rs:26-870` · `U/nexa-fs/src/shell.rs:12-883` · `S/main.rs:486` · `1826` · `1937` |
| `nexa-dlg` | `FilePicker` · `PickerMode{Open, Save, Folder}` · `PickerAction` · `FileFilter` | `U/nexa-dlg/src/lib.rs:29-154` |
| `nexa-sys` | `input_source::{is_korean, watch, take_changed}` · `layer_present::{LayerPresenter, Frame, app_active}` · `Signals`/`on_battery`/`remote_session`/`reduce_motion`/`cpu_count` | `U/nexa-sys/src/lib.rs:19-69` · `input_source.rs:17-28` · `layer_present.rs:18-49` |
| `nexa-conf` | `user_config_dir(app)`(설정 크레이트 경유) | `U/nexa-conf/src/lib.rs:314-334` |

**nexa-ui에 없고 nexa-sql 앱에만 있는 호스트 부품**(dir3도 똑같이 필요): `present` · `winhost` · `wingeom` · `winfocus` · `theme`(OS 조회) · `icon` · `input`(휠·키 변환) · `clipboard`(+x11) · `imestate`/`imehint`/`imewatch` · `toast`. nexa-sql 문서도 "창마다 복사한 softbuffer/RasterCtx/이벤트 변환/틱을 **WindowHost 부품**으로"를 미완 과제(T-65)로 적고 있다(`D/30-architecture-patterns.md:113`).

---

## 3. OS별 분기 목록

| ID | 분기점 | Windows | macOS | Linux | 위치 | dir3 적용 |
| --- | --- | --- | --- | --- | --- | --- |
| SKEL-201 | 창 백엔드 | winit 기본 | winit 기본 | 설정 `gfx.linux_backend`(기본 **x11** = XWayland · `DISPLAY` 있을 때 / `wayland`) · 실패 = winit 기본 | `S/main.rs:1344-1362` | 그대로 |
| SKEL-202 | 소유 창(생성 속성) | `with_owner_window(HWND)` | 없음(Dock 아이콘이 앱당 하나) | `with_x11_window_type([Dialog])` | `S/winfocus.rs:16-38` | 그대로 |
| SKEL-203 | 자식/모달 붙이기(생성 뒤) | no-op(소유 창이 같은 일) | `NSWindow.addChildWindow_ordered(Above)` | x11rb로 `WM_TRANSIENT_FOR` + `_NET_WM_STATE_MODAL`(속성 + 클라이언트 메시지) · Wayland 네이티브 = no-op | `S/winfocus.rs:97-121` · `188-233` | 그대로 |
| SKEL-204 | 모달 중 다른 창 차단 | `EnableWindow(FALSE)` | 사건 가드 + `setMovable(false)`(이동 잠금) | 사건 가드만 | `S/winfocus.rs:53-79` · `S/app/event_loop.rs:604-627` | 그대로 |
| SKEL-205 | 그룹 올리기 | `SetWindowPos(HWND_TOP, NOSIZE|NOMOVE|NOACTIVATE)` | `orderFront:` | no-op(WM 정책) | `S/winfocus.rs:124-173` | 그대로 |
| SKEL-206 | 창·작업표시줄 아이콘 | 창 아이콘 32 + `with_taskbar_icon(64)` + exe 리소스 `.ico` | 창 아이콘 무시 → `NSApplication.setApplicationIconImage`(첫 `resumed`에서 · 256px) | X11 = 창 아이콘 · Wayland = `app_id`와 같은 이름의 `.desktop`의 `Icon=` · `with_name("nexa-sql","nexa-sql")` | `S/icon.rs:8-12` · `157-253` · `S/app/event_loop.rs:12-14` | 개명(`nexa-dir` app_id — dir2 값 확인) |
| SKEL-207 | present 뒷단 | softbuffer | softbuffer 또는 IOSurface(`gfx.mac_present`) | softbuffer | `S/present.rs:57-72` | 그대로 |
| SKEL-208 | 앱 활성 판정 | `None`(창 포커스로만) | `NSApplication.isActive` | `None` | `S/app/event_loop.rs:223` | 그대로 |
| SKEL-209 | OS 다크/라이트 조회 | 레지스트리 `HKCU\…\Themes\Personalize\AppsUseLightTheme`(advapi32 FFI) | `defaults read -g AppleInterfaceStyle` | `gsettings get org.gnome.desktop.interface color-scheme` | `S/theme.rs:46-158` | 그대로 |
| SKEL-210 | 클립보드 텍스트 | user32/kernel32 FFI | `pbcopy`/`pbpaste` | X11 selection 직접 → CLI 폴백 | `S/clipboard.rs:52-299` · `S/clipboard_x11.rs` | 그대로 + 파일 형식 추가 |
| SKEL-211 | 클립보드 서식 | `HTML Format` | `osascript` | 텍스트만 | `S/clipboard.rs:24-50` · `143-169` · `245-265` | dir3는 서식 복사 불요(추정) |
| SKEL-212 | 주 수식키 | Ctrl | ⌘(`super_key`) · Control은 `ctrl_mac`으로 따로 | Ctrl | `S/app/event_loop.rs:681-691` | 그대로 |
| SKEL-213 | 단어/서브워드 이동 | Ctrl = 단어 · Alt = 서브워드 | ⌥ = 단어 · ⌃ = 서브워드 · ⌘ = 줄 처음/끝 | Windows와 같음 | `S/app/input.rs:254-274` | 그대로(입력란) |
| SKEL-214 | 열 선택 마우스 규칙 | Alt+Shift | Option | Shift+우클릭 드래그 | `S/app/paint.rs:662-678` · `S/app/input.rs:218-242` | 불요(편집기 전용) |
| SKEL-215 | 휠 | LineDelta | PixelDelta 1:1 · **가로 부호 반전** | PixelDelta(libinput) | `S/input.rs:191-229` | 그대로 |
| SKEL-216 | 한글 조합 | 시스템 IME | `auto` = 입력 소스가 한글일 때 **앱 조합**(시스템 IME 끊음) | 시스템 IME(X11 = XIM 경로) | `S/input.rs:95-103` · `S/app/input.rs:138-175` · `D/62-macos-input-and-present.md:23-31` | 그대로 |
| SKEL-217 | 목록 타입어헤드의 한/영 | 앱이 `HangulMode`/`Lang1` 키를 받아 토글 + QWERTY→자모 | 레이아웃이 자모를 직접 줌(불요) | 미확인(추정: 시스템 IME 차단 시 라틴만) | `S/app/event_loop.rs:796-813` · `S/app/input.rs:296-303` | 그대로 |
| SKEL-218 | IME 상태 읽기 | `GetKeyboardLayout` + `ImmGetDefaultIMEWnd` + `WM_IME_CONTROL` | `nexa_sys::input_source::is_korean()` | `ibus engine`/`fcitx5-remote -n` 프로세스(500 ms 캐시) + 감시 | `S/imestate.rs:164-293` | 필요 시 그대로 |
| SKEL-219 | 입력 소스 변경 알림 | 없음 | 분산 알림(`watch`/`take_changed`) | `dbus-monitor` 감시(가린 칸 창이 열린 동안만) | `S/app/event_loop.rs:128-130` · `202-205` · `S/imewatch.rs` | 그대로 |
| SKEL-220 | 단축키 글자 판정 | `logical_key` = 라틴 | ⌘+키의 `logical_key` = 자모 → 물리 키 | `logical_key` = 라틴(xkb) | `S/input.rs:43-93` · `D/61-core-design-and-working-rules.md:188` | 그대로(규칙: ⌘/Ctrl+글자는 논리 글자를 비교하지 않는다) |
| SKEL-221 | 포커스 한정 단축키의 통과 | 포커스 밖이면 버림 | 포커스 밖이면 통과(Alt+글자 입력 보존) | Windows와 같음 | `S/app/event_loop.rs:860-872` | 그대로(규칙) |
| SKEL-222 | 연결 프로그램 열기 | `cmd /C start "" <p>` | `open <p>` | `xdg-open <p>` | `S/main.rs:1191-1204` | 그대로 |
| SKEL-223 | 설정 폴더 | `%APPDATA%\<app>` | `~/Library/Application Support/<app>` | `$XDG_CONFIG_HOME/<app>` 또는 `~/.config/<app>` | `U/nexa-conf/src/lib.rs:314-334` · `D/61-core-design-and-working-rules.md:191` | 개명 |
| SKEL-224 | 실행 파일 리소스·런타임 | `.rc`(아이콘·버전) + 정적 CRT + `windows_subsystem` | `.app` 번들 `Info.plist`·`.icns`(패키징) | `.desktop` + hicolor PNG(패키징) | `R/packaging/` · `R/.cargo/config.toml` · `S/main.rs:11` | 개명 |
| SKEL-225 | 힙 회수·사용량 | `HeapSetInformation` + `HeapCompact` · `K32GetProcessMemoryInfo` | `malloc_zone_pressure_relief` · `task_info` | glibc `malloc_trim(0)`(gnu만) | `S/memtrim.rs:23-188` | 그대로 |
| SKEL-226 | 창 위치 기억 보정 | 불요 | 제목 표시줄 높이 드리프트 → `place_outer` | 불요(추정) | `S/wingeom.rs:94-101` | 그대로 |
| SKEL-227 | 숨김 생성 이유 | — | 보이는 창을 다른 배율 모니터로 옮기면 생성 시점 배율 유지 | — | `S/app/event_loop.rs:15-19` | 그대로 |
| SKEL-228 | 모니터 열거 시점 | — | `el.available_monitors()`가 `resumed`에서 빈다 → 창 핸들로 | — | `S/app/event_loop.rs:15-17` · `51` | 그대로 |
| SKEL-229 | 창 핸들 얻기 | `hwnd(w)`(`RawWindowHandle::Win32`) | `ns_window(w)`(`AppKit` → NSView → window) | `x11_window(w)`(`Xlib`/`Xcb`) | `S/winfocus.rs:40-48` · `81-93` · `188-197` | 그대로(dir3 셸 연동의 입구 — 컨텍스트 메뉴·DnD가 HWND/NSView를 요구) |
| SKEL-230 | 글자 래스터 | GDI ClearType(전진폭 정수) | CoreText(전진폭 소수) | 고정 경로 후보 + `/usr/share/fonts` 걷기 1회 캐시 | `D/61-core-design-and-working-rules.md:186` · `U/nexa-gfx/src/{gdi,coretext}.rs` | nexa-ui 몫(그대로) |
| SKEL-231 | 미사용 경고 관리 | — | — | — (OS 한쪽에서만 쓰는 함수는 `#[cfg_attr(…, allow(dead_code))]`) | `S/main.rs:23-27` · `S/imestate.rs:22-61` · `S/clipboard.rs:31-33` | 그대로(CI `-D warnings` 3-OS) |
| SKEL-232 | 비활성 앱의 타이머 | — | App Nap으로 `@after` 타이머가 수 초 늦음 | — | `D/61-core-design-and-working-rules.md:250` | 하네스 여유(마지막 `@after` + 5 s) |
| SKEL-233 | Wayland 제약 | — | — | 부모 창·활성화 API 없음 · 창 아이콘 직접 지정 불가 · `xdotool`/캡처 불가 · 클립보드 다리는 포커스 창만 | `S/winfocus.rs:107-109` · `S/icon.rs:166-174` · `D/61-core-design-and-working-rules.md:182-187` | X11 기본 유지 |
| SKEL-234 | 자체 캡처 수단 | `PrintWindow`(스크립트) | `screencapture -l` | stderr 추적 + `/proc` | `D/61-core-design-and-working-rules.md:182` | 하네스 설계에 반영 |

**nexa-sql에 없어서 dir3가 새로 정해야 하는 OS 분기**(이 범위에서 구현을 찾지 못함 — Grep 0건): 끌어 **내기** DnD · 끌기 중 대상 강조(`HoveredFile`) · 파일 클립보드 형식 · 휴지통 · 셸 컨텍스트 메뉴 · 터미널(PTY) · 트레이. 상세는 18·19번 문서와 §5-7.

---

## 4. 함정 · 교훈

| ID | 함정(증상) | 교훈(규칙) | 근거 |
| --- | --- | --- | --- |
| SKEL-301 | `about_to_wait`마다 `request_redraw` → 그리기↔대기 무한 루프 · 유휴 CPU 한 코어 100% · 키가 프레임당 하나씩만 처리 | 타이머가 **실제로 만료됐을 때만** 다시 그린다 · 유휴는 `WaitUntil` | `S/app/event_loop.rs:208-211` |
| SKEL-302 | 보조 창이 그린 직후 다시 그리기를 요청 → 창을 열어 두면 유휴 CPU 90% | `Paint` 액션 뒤에는 `redraw()`를 부르지 않는다 | `S/app/event_loop.rs:1539-1566` |
| SKEL-303 | 창 열기 깃발을 `window_event` 끝에서만 보면 사건이 없을 때(기동 명령·워커 물음) 창이 안 열린다 | 깃발은 `about_to_wait`에서도 본다(`pump_window_requests`) | `S/app/event_loop.rs:1767-1775` · `D/61-core-design-and-working-rules.md:171` |
| SKEL-304 | 타이머·기동 명령이 요청한 종료가 창 사건이 없으면 영영 처리 안 됨 | `exit_requested`를 유휴 틱 첫 줄에서 확인 | `S/app/event_loop.rs:198-201` · `S/app/files.rs:178-188` |
| SKEL-305 | 뒤에 있는(한 번도 활성화 안 된) 창이 계속 깜빡이며 그림 → 맥 유휴 CPU 19% | `main_active` 초기값 `false` · 깜빡임은 메인 창·앱이 활성일 때만 | `S/main.rs:1712-1714` · `S/app/event_loop.rs:220-226` |
| SKEL-306 | 보이는 창을 다른 배율 모니터로 옮기면 반 크기로 그려짐(mac) | 숨긴 채 만들고 위치를 정한 뒤 보인다 | `S/app/event_loop.rs:18-19` |
| SKEL-307 | 프로그램 이동 뒤 `ScaleFactorChanged`가 안 오는 경우 | `Moved`에서 배율을 다시 읽는다 | `S/app/event_loop.rs:651-662` |
| SKEL-308 | 토글할수록 보조 창이 위로 올라감(mac) | 기억 위치는 생성 뒤 `set_outer_position`으로 다시 놓는다 | `S/wingeom.rs:94-101` |
| SKEL-309 | "메인 창 오른쪽에 연다"가 화면 밖으로 수백 px 나감 | 새 창은 생성 직후 `keep_on_screen` 한 줄 | `S/wingeom.rs:117-139` · `D/61-core-design-and-working-rules.md:285` |
| SKEL-310 | `preferred_height()`(논리 px)를 그대로 쓰면 HiDPI에서 툴바가 1/배율로 납작 | 컨트롤이 주는 권장 치수는 논리 px — `px(v, s)`로 변환 | `S/main.rs:912-916` |
| SKEL-311 | `set_scale` 누락 → 레티나에서 글자 겹침 | `set_bounds`와 `set_scale`은 한 쌍 | `D/61-core-design-and-working-rules.md:175` |
| SKEL-312 | 닫힌 창에서 누른 Enter가 메인 창 편집기에 줄바꿈 | 합성 누름 버리기 + 키 문지기(SKEL-046) | `S/app/event_loop.rs:517-539` |
| SKEL-313 | Ctrl+T를 누르고 있자 탭 80개 · 밀린 사건으로 UI 정지 | 자동 반복은 `repeatable` 명령만 실행 | `S/app/event_loop.rs:843-887` |
| SKEL-314 | 우클릭이 `is_mouse`에 없어 우클릭 분기가 처음부터 닿지 않는 코드(컴파일러가 못 잡음) | 포인터 판정은 `is_ptr` · 실제 라우팅 경로를 밟는 시험(`ui.rclick`)으로 확인 | `S/app/input.rs:472-475` · `D/61-core-design-and-working-rules.md:237` |
| SKEL-315 | 밖에서 놓으면 드래그가 안 끝남 → 다음 이동이 선택을 바꿈 | 호스트의 포인터 캡처가 기본 처리 — 컨트롤마다 고치지 않는다 | `S/app/input.rs:476-515` · `D/61-core-design-and-working-rules.md:116-122` |
| SKEL-316 | 커서가 빠르게 나가면 툴팁이 남음 | `CursorLeft`·비활성 = `pointer_gone()` · 영역 전환 때 이전 영역에 "밖" 이동 | `S/app/event_loop.rs:574-586` · `S/app/input.rs:479-495` |
| SKEL-317 | 메뉴가 열린 채 바깥 **이동**을 아래로 흘리면 다른 영역이 포커스를 가져가 메뉴가 닫힘 | 바깥 클릭만 통과 · 이동은 메뉴에서 끝 · 포커스는 **누를 때만** 옮김 | `S/app/input.rs:534-545` · `783-786` |
| SKEL-318 | 팝업이 뒤 층에 가려짐(풀다운·툴팁·우클릭 메뉴) · 구분선이 팝업 위로 지나감 | 팝업·툴팁은 창의 **맨 마지막 패스** · 토스트는 팝업 아래 | `S/app/paint.rs:81-87` · `542-560` · `594` |
| SKEL-319 | 토스트가 바꾼 글꼴 슬롯 때문에 뒤이은 메뉴 글자가 커짐 | 부품이 바꾼 `select_font`는 팝업 그리기 전에 되돌린다 | `S/app/paint.rs:608-609` |
| SKEL-320 | 팝업이 그리기 표면 밖으로 나감 | 위치를 직접 계산하지 않는다(`ContextMenu`·`draw_tooltip_in`·`Combo` + `place_popup`/`nudge_into`) · host는 창 전체 Rect | `D/61-core-design-and-working-rules.md:96-104` |
| SKEL-321 | 메뉴 항목 id의 `starts_with` 분기가 겹쳐 다른 동작 실행 | 새 접두는 기존 접두와 겹치지 않는지 grep · 기동 명령 id도 같음 | `D/61-core-design-and-working-rules.md:132` · `242` |
| SKEL-322 | 두 메뉴가 동시에 떠 있음 | 열린 메뉴 비트 비교로 배타(SKEL-074) · 새 메뉴 = 비트 한 줄 추가(연 쪽·닫는 쪽 둘 다) | `S/app/menus.rs:1751-1828` |
| SKEL-323 | Dock 아이콘을 루프 생성 직후에 넣으면 winit 기동이 덮는다 | 첫 `resumed`에서 넣는다 | `S/app/event_loop.rs:12-14` |
| SKEL-324 | 시험용 창이 전경을 가져가 사용자가 치던 글자를 받음 | 자체 시험 인스턴스는 반드시 `*_NO_ACTIVATE=1` · OS 키 주입 금지 | `S/icon.rs:175-181` · `D/61-core-design-and-working-rules.md:148` · `238` |
| SKEL-325 | mac에서 모달 창을 자식으로 안 붙이면 메인 뒤로 숨음 → 워커가 답을 기다리며 멈춤 | 모달 창 = `owned_by` + `attach_child` + `sync_modal` 세 줄 한 벌 | `S/app/event_loop.rs:1920-1927` |
| SKEL-326 | winit `with_parent_window`는 X11에서 **삽입(embed)** | 부모 관계는 x11rb로 `WM_TRANSIENT_FOR` | `S/winfocus.rs:27-31` |
| SKEL-327 | 설정 창에서 연 창이 메인 소유면 설정 창 뒤로 숨어 "안 열린 것처럼" | 소유자 = 그 창을 띄운 창 | `S/app/event_loop.rs:1689` |
| SKEL-328 | 모달이 닫힌 뒤 메인으로 포커스를 주면 호스트 보조 창이 뒤로 숨음 | `picker_return`으로 띄운 창에 돌려준다 | `S/app/files.rs:388-401` · `S/app/windows.rs:61-67` |
| SKEL-329 | 열 때마다 새 `WindowId`가 z-order 목록에 쌓임 | 모달이 닫힐 때 죽은 id를 걷는다 | `S/app/windows.rs:42-46` |
| SKEL-330 | mac softbuffer present 37 ms/프레임(레티나) | IOSurface 경로(2.9 ms) — 함정: 알파 0 = 투명 → 0xFF 채움 · 행 바이트를 강제하면 합성기가 안 그림 | `D/62-macos-input-and-present.md:38-62` |
| SKEL-331 | mac IME: 첫 키가 조합 없는 자모로 유출 · 한글 뒤 첫 1바이트 글자 삼킴 | 한글 입력 소스일 때는 IME를 거치지 않고 앱이 조합 | `D/62-macos-input-and-present.md:17-31` |
| SKEL-332 | ⌘C의 논리 키가 "ㅊ"으로 와서 단축키 불발 | ⌘/Ctrl+글자는 물리 키로 판정 | `S/input.rs:43-44` |
| SKEL-333 | `Focus` 변형이 아닌 입력란(팔레트)의 IME 글자가 편집기로 샘 · 포커스가 목록인 채 팔레트를 열면 IME가 꺼져 있음 | 팝업 입력란은 IME 분기·`ime_refresh` 조건에 따로 넣는다 | `S/app/event_loop.rs:750-753` · `S/app/input.rs:115-117` |
| SKEL-334 | 새 창을 만들고 IME 등록을 빠뜨림 | 새 창 = `set_ime_allowed(input::system_ime())` + `sync_hangul_mode`의 창 목록에 등록 | `S/app/input.rs:160-174` · `D/61-core-design-and-working-rules.md:173` |
| SKEL-335 | Linux에서 외부 클립보드 프로그램이 없으면 복사 불가 · 잘라내기가 글자만 지우고 클립보드는 빔 | 프로토콜 직접 구현 · **쓰기 성공 확인 뒤 삭제** | `S/clipboard_x11.rs:1-14` · `S/app/menus.rs:1404-1416` |
| SKEL-336 | X11 요청 오류를 흘려보내면 소유권도 응답도 조용히 실패 | `.check()`로 받는다 · 소유권 실패 시 보관분 비움 | `S/clipboard_x11.rs:78` · `138-146` |
| SKEL-337 | 트랙패드 느린 스크롤(1~2 px)이 통째로 버려짐 · 사선 드리프트로 세로 스크롤 끊김 · mac 가로 방향 반대 | 잔여 누적 양자화 · 축 잠금 · mac 가로 부호 반전(SKEL-064) | `S/input.rs:14-29` · `199-217` |
| SKEL-338 | 전역 상태를 공유하는 시험이 병렬로 돌아 CI에서 간헐 실패 | 정적 뮤텍스 가드 · "되돌리니 통과"는 원인의 증거가 아니다(로그를 본 뒤 고침) | `S/input.rs:235-246` · `D/61-core-design-and-working-rules.md:147` |
| SKEL-339 | `RUSTFLAGS` 환경 변수가 있으면 `.cargo/config.toml`의 rustflags가 통째로 무시 · 동적 CRT exe는 깨끗한 Windows에서 실행 불가 | 워크플로에 `RUSTFLAGS`를 두지 않는다 · 임포트 화이트리스트로 실측 | `R/.cargo/config.toml:3-8` |
| SKEL-340 | `rc.exe`가 없으면 아이콘·버전 없는 exe가 조용히 나옴 · build.rs 없는 bin은 버전 정보 공란 | 릴리스 CI에서 리소스 유무를 확인 · bin마다 build.rs | `R/packaging/windows/winres.rs:4-12` |
| SKEL-341 | 창마다 softbuffer/RasterCtx/사건 변환(`to_input`)/틱을 복사(≈300줄×N) | 공통 호스트(`winhost`·`Presenter::frame`·`text_key_event`·`centered_over`)를 먼저 쓰고, 둘째 사용처에서 부품으로 올린다 | `D/30-architecture-patterns.md:107` · `113` · `S/conn_win.rs:1995` · `S/file_win.rs:290` · `S/prefs_win.rs:962` · `S/colors_win.rs:274` |
| SKEL-342 | main.rs 거대 객체 · `window_event` 1,151줄 | 상태 = `App` 한 곳 · 동작 = 기능 파일 · 긴 함수는 흐름만 남기고 단계 함수로 | `S/app/event_loop.rs:907` · `D/30-architecture-patterns.md:56` |
| SKEL-343 | 릴리스 프로필 `panic = "abort"`에서는 `catch_unwind`가 패닉을 잡지 못한다(추정 — Rust 규칙상 abort는 unwind 불가 · nexa-sql 문서에 별도 언급 없음) | 워커 패닉 격리를 dir3에서 실제로 원하면 프로필/스레드 경계 설계를 먼저 결정 | `R/Cargo.toml:94` · `S/worker.rs:745-747` |
| SKEL-344 | paint 중 `surface`가 가변 빌림이라 `&self` 메서드를 못 부름 | 그리기 전에 값 계산 · 필드만 받는 연관 함수 · 보조 창은 `surface.take()` | `S/app/paint.rs:9-36` · `215` · `S/about_win.rs:237-244` |
| SKEL-345 | 프레임버퍼가 사적 메모리의 60% 이상(창마다 4 B/px) | 창을 닫으면 표면을 즉시 놓는다 · 숨김 창을 만들지 않는다 | `D/26-performance-architecture.md:153-174` |
| SKEL-346 | Debug 빌드의 "멈춤"은 최적화 없는 래스터의 느림일 수 있다 | 의존 크레이트 `opt-level = 2` · 측정은 Release · 빌드 직후 첫 실행은 버린다 | `R/Cargo.toml:97-101` · `D/61-core-design-and-working-rules.md:140` · `183` |
| SKEL-347 | ⌃Space가 표에 없어 공백 입력 | 수식키 붙은 Space는 글자가 아니다 | `S/app/input.rs:287-289` |
| SKEL-348 | 글자 항목 폭이 바뀐 뒤 클릭이 닿지 않음 | paint 뒤 `relayout_if_stale` 한 번 | `S/app/paint.rs:76-80` |
| SKEL-349 | 격리 폴더가 엉뚱하게 잡혀 실제 설정에 씀 | 격리가 **정말 적용됐는지** 확인(설정 파일 수정 시각) · 단위 시험은 폴더를 인자로 받는다 | `D/61-core-design-and-working-rules.md:145-146` |
| SKEL-350 | "잠금"이 사건 전달만 막고 포커스·IME·붙여넣기 경로로 샘 | 포커스 지정 · `focused_textbox` · 값 수거 · 재계산 시점 넷을 다 본다 | `D/61-core-design-and-working-rules.md:284` |

---

## 5. dir3 골격 제안

> 아래는 nexa-sql 골격을 dir3에 옮길 때의 **제안**이다. 영역·창·명령의 실제 목록은 dir2 인벤토리(10~19번 문서)가 원천이며, 여기 이름은 자리표시(추정)다.

### 5-1. 모듈 구성안

| ID | 제안 | 내용 |
| --- | --- | --- |
| SKEL-401 | GUI 크레이트 구조 = nexa-sql과 같은 3층 | `src/main.rs`(모듈 선언 · `struct App` · `enum Focus` · `layout()` · `main()`) / `src/app/*.rs`(`impl App` 조각) / 호스트 껍질·보조 창·패널·작업 스레드는 파일 하나씩(§2 그림) |
| SKEL-402 | `app/` 최소 조각 | `event_loop.rs` · `input.rs` · `paint.rs` · `windows.rs` · `menus.rs` · `toolbar.rs` · `settings.rs` · `events.rs`(워커 응답 수거) · `startup_cmd.rs` · `license.rs` + 도메인 조각(추정: `panels.rs` · `fileops.rs` · `dock.rs` · `terminal.rs` · `preview.rs` · `clipboard.rs` · `dnd.rs`) |
| SKEL-403 | 호스트 껍질은 **그대로 복사해 시작** | `present.rs` · `winhost.rs` · `wingeom.rs` · `winfocus.rs` · `theme.rs` · `icon.rs`(그림만 교체) · `input.rs` · `clipboard.rs`(+`clipboard_x11.rs`) · `toast.rs` — 환경 변수 접두·app_id·제목만 개명 |
| SKEL-404 | OS 분기 전용 모듈 `platform/` | nexa-sql은 OS 분기가 얇아 파일마다 `#[cfg]` `mod imp`로 흩어 두었지만(예: `theme.rs` · `clipboard.rs` · `memtrim.rs`), dir3는 분기가 넓다(터미널·셸 메뉴·휴지통·DnD·파일 클립보드) → `platform/{mod.rs, windows.rs, macos.rs, linux.rs}`에 **같은 시그니처 함수**를 두고 호출부는 OS를 모르게 한다. 각 함수는 `Option`/`Result`로 "미지원"을 돌려줄 수 있어야 한다(nexa-sys 관례: 다른 OS = `None`) |
| SKEL-405 | 작업공간 설정 복사 | `Cargo.toml`의 `[workspace.lints]` · `[profile.release]` · `[profile.dev.package."*"]` · nexa-ui/nexa-license 경로 의존 · `.cargo/config.toml` 정적 CRT · `build.rs` + `packaging/windows/{winres.rs, <앱>.rc}` |

### 5-2. `App` 골격 필드(최소)

| ID | 묶음 | 필드(nexa-sql 대응) |
| --- | --- | --- |
| SKEL-406 | 창·표면 | `window: Option<Rc<Window>>` · `surface: Option<Presenter>` · `scale: f32` · `z_order: Vec<WindowId>` · `conn_modal`에 해당하는 `modal_on: bool` · `picker_return` |
| SKEL-407 | 자원 | `ui_font` · `mono_font` · `theme: Theme` · `settings` · `keymap` · `licensing` |
| SKEL-408 | 입력 상태 | `cursor` · `pointer` · `shift` · `primary` · `alt` · `ctrl_raw` · `ctrl_mac` · `focus: Focus` · `press_capture` · `hover_area` · `pending_chord` · `hangul_mode` · `hangul_app` · `ime_last` · `key_guard` |
| SKEL-409 | 시간 | `started` · `blink_origin` · `next_blink` · `main_active` + 하위 기능의 `*_next: Instant` |
| SKEL-410 | 깨움·작업 | `wake_proxy: EventLoopProxy<Wake>` · 워커 핸들·수신자들 · 일회성 `Receiver` |
| SKEL-411 | 창 열기 깃발 | `open_<창>: bool` · `<물음>_pending: Option<…>` · `exit_requested` |
| SKEL-412 | 진단 | `frame_trace` · `trace_ime` · `trace_marks` · `startup_timed` |

### 5-3. 기동·루프 규약

| ID | 제안 | 내용 |
| --- | --- | --- |
| SKEL-413 | 기동 순서 고정 | SKEL-002 순서 그대로 · `--smoke`는 창 없이 "설정·글꼴·자원 디코드·플러그인 런타임 초기화"까지 확인하고 0으로 종료(CI 3-OS 공통 게이트) |
| SKEL-414 | 재그리기 3원칙 | ① 그리기는 `RedrawRequested`에서만 ② 유휴는 `WaitUntil(next)` ③ 하위 기능은 `tick(now) -> bool`과 `next_wake() -> Option<Instant>` 두 모양만 내놓는다(자체 타이머·스레드 금지) |
| SKEL-415 | 통지 규약 | 모든 배경 작업 = `mpsc` 채널 + `Box<dyn Fn() + Send>` 깨움 클로저 · UI는 `user_event`에서 `try_recv` 루프 · 진행률은 원자값(그릴 때 읽음) · 취소는 `Arc<AtomicBool>` · 물음은 전용 답 채널 + 깃발 + `pump_window_requests` |
| SKEL-416 | 터미널 출력도 같은 길 | PTY 읽기 스레드 → 채널 → `wake()`(추정: 출력이 잦으므로 깨움은 "보낸 뒤 1회" · 수거는 틱당 시간 예산으로 — nexa-sql `parwalk` 수신 방식 `D/30-architecture-patterns.md:80`) |

### 5-4. 포커스·영역(추정 — dir2 화면 구성 문서로 확정)

| ID | 제안 | 내용 |
| --- | --- | --- |
| SKEL-417 | `enum Focus` 초안 | 추정: `PanelLeft` · `PanelRight`(파일 목록) · `Tree`(폴더 트리) · `PathBar`(경로 입력) · `Filter` · `Dock`(하단 도크: 터미널·미리보기 등) · `Preview` — 실제 영역은 13·14·16번 문서의 컨트롤 배치로 확정 |
| SKEL-418 | 영역 추가 체크리스트 | 새 영역 하나 = ① `Focus` 변형 ② `set_focus`의 `set_focused` 줄 ③ `layout()`의 `set_scale`+`set_bounds` ④ `paint()`의 본문 층 + 팝업 층 ⑤ `route_<영역>` 고리(포인터 = 안 · 키 = 포커스) ⑥ `area_at`/`area_bounds`/`dispatch_captured` 한 줄씩 ⑦ `pointer_gone`·hover 이탈 ⑧ 메뉴가 있으면 `open_menus`/`close_menu_bits` 비트 ⑨ 글 입력이 있으면 `focused_textbox`·`ime_refresh` ⑩ `tick` + `bars_live` 조건 |
| SKEL-419 | 보조 창 추가 체크리스트 | 새 창 하나 = ① `<이름>_win.rs`(SKEL-023 메서드 한 벌 + `Action` 열거) ② `App` 필드 + `open_<이름>` 깃발 ③ `open_requested_windows`·`pump_window_requests` 둘 다에 소비 줄 ④ `aux_window_event`에 분배 줄 ⑤ `all_windows` ⑥ 모달이면 `modal_open`·`modal_window`·가드의 `is_modal_win` + `attach_child` + `sync_modal` ⑦ 글 입력이 있으면 `sync_hangul_mode` 창 목록 ⑧ 기하 기억이 필요하면 `persist_window_sizes`/`apply_window_sizes` ⑨ 틱이 있으면 `about_to_wait` ⑩ `reset_layout`류의 "전부 닫기" |

### 5-5. 메뉴·명령

| ID | 제안 | 내용 |
| --- | --- | --- |
| SKEL-420 | 메뉴바 = nexa-ctl `MenuBar` 3-OS 공통 | nexa-sql은 macOS에서도 창 안 자체 메뉴바를 쓴다(네이티브 NSMenu 없음) → "OS 기본 컨트롤 미사용" 기조와 일치하므로 dir3도 같게. macOS 전역 메뉴(⌘Q·⌘H 등 기본 항목)를 따로 둘지는 **미결**(nexa-sql에 구현 없음 — winit 기본 동작에 맡김, 추정) |
| SKEL-421 | 명령 id 표를 먼저 만든다 | dir2의 메뉴·툴바·단축키·컨텍스트 메뉴 동작을 **명령 id 문자열** 하나의 어휘로 정리 → `menu_action(id)` 한 길 · 기동 명령으로도 그대로 실행(하네스가 모든 명령을 키 주입 없이 호출). nexa-sql은 이 표가 문자열로 흩어져 있어 "Command 레지스트리"를 과제(T-67)로 남겼다(`D/30-architecture-patterns.md:117`) → dir3는 처음부터 표(id · 라벨 · 기본 키 · 반복 가능 여부)로 |
| SKEL-422 | 컨텍스트 메뉴 2층 | 앱 항목 = nexa-ctl `ContextMenu`(자체 그리기) · OS 셸 항목(Windows `IContextMenu` 등)은 `platform` 층이 **항목 목록(id·라벨·아이콘)** 으로 번역해 같은 `ContextMenu`에 하위 메뉴로 넣거나, 번역이 불가능한 OS/항목은 "OS 메뉴 열기" 한 항목으로(19번 문서와 맞춰 결정) |

### 5-6. 자원

| ID | 제안 | 내용 |
| --- | --- | --- |
| SKEL-423 | dir2 자원 임베드 | 아이콘·문자열·샘플을 `include_bytes!`/`include_str!`로 exe에 넣고(SKEL-156) 첫 사용 시 디코드 · 무거운 디코드는 선굽기 스레드(SKEL-013) · 캐시 상한은 설정(SKEL-012) |
| SKEL-424 | 창 아이콘 | `icon::with_icon`의 그림 원천만 dir2 아이콘으로 교체(32·64·256 RGBA) · Windows `.rc`의 `.ico` · mac Dock · Linux `app_id` 세 자리를 같은 자원에서 |

### 5-7. nexa-sql에 없는 것 — dir3가 새로 세울 OS 분기(platform 층 시그니처 제안)

| ID | 기능 | 제안 시그니처(추정) | 비고 |
| --- | --- | --- | --- |
| SKEL-425 | 기본 셸 | `default_shell() -> (프로그램, 인자)` — Windows `pwsh`(없으면 `powershell`/`cmd`) · mac/Linux `$SHELL`(없으면 `/bin/sh`) | 18번 문서 |
| SKEL-426 | PTY | `Pty::spawn(shell, cwd, cols, rows)` · `read`/`write`/`resize`/`kill` — Windows ConPTY · Unix `openpty`/`forkpty` | 18번 문서 |
| SKEL-427 | 휴지통 | `trash(paths) -> Result` — Windows `IFileOperation`/`SHFileOperation` · mac `NSFileManager trashItem` · Linux XDG Trash 규격 | 19번 문서 |
| SKEL-428 | 파일 클립보드 | `read_files() -> Option<(Vec<PathBuf>, 잘라내기?)>` · `write_files(paths, cut)` | SKEL-109 확장 |
| SKEL-429 | DnD 내보내기 | `begin_drag(window_handle, paths, 효과)` — 창 핸들은 `winfocus::{hwnd, ns_window, x11_window}`(SKEL-229) | winit은 받기(`DroppedFile`)만 제공 |
| SKEL-430 | 셸 컨텍스트 메뉴 | `shell_menu_items(paths) -> Vec<Item>` · `invoke(id)` | SKEL-422 |
| SKEL-431 | 연결 프로그램·탐색기에서 보기 | 이미 있음: `open_external`(SKEL-146) · `nexa_fs::shell::reveal_in_file_manager` | 그대로 |
| SKEL-432 | OS 파일 아이콘 | 이미 있음: `nexa_fs::shell::IconService`(`U/nexa-fs/src/shell.rs:108`) · 폴백 `fallback_file_icon`(`D/30-architecture-patterns.md:76`) | 그대로 |

### 5-8. nexa-ui로 올릴 후보(결정 필요)

| ID | 후보 | 근거 · 선택지 |
| --- | --- | --- |
| SKEL-433 | 창 호스트 부품(가칭 `nexa-host`): `Presenter` · `winhost` · `wingeom` · `winfocus` · `theme`(OS 조회) · 휠/키 변환 · 텍스트 클립보드 | nexa-sql과 dir3가 **같은 코드를 두 벌** 갖게 된다. "같은 문제를 두 번째 만나면 부품으로 올린다"(`D/30-architecture-patterns.md:107`) · nexa-sql 자체 과제 T-65(`D/30-architecture-patterns.md:113`). 선택지: ① 먼저 dir3에 복사해 동작시킨 뒤 승격(위험 낮음 · 권장) ② 처음부터 nexa-ui에 새 크레이트(nexa-sql도 함께 이관해야 3-OS 검증 범위가 커짐). 주의: 이 부품은 winit·softbuffer·x11rb·objc2에 의존하므로 nexa-ui의 "의존 0" 크레이트들과 분리된 크레이트여야 한다(nexa-sys가 `gui` 기능으로 가른 선례 — `R/Cargo.toml:87-88`) |
| SKEL-434 | 토스트 | nexa-ctl에는 `flash.rs`만 있다(`U/nexa-ctl/src/controls/` 목록) — 앱 2곳이 쓰게 되면 컨트롤로 승격 |
| SKEL-435 | IME 상태/안내 | 가린 입력란이 dir3에도 있을 때만 |

### 5-9. 회귀 하네스가 기대는 골격 훅

| ID | 훅 | 내용 |
| --- | --- | --- |
| SKEL-436 | 환경 변수 한 벌(제안 이름) | `NDIR_HOME`(격리) · `NDIR_STARTUP_CMD` · `NDIR_NO_ACTIVATE` · `NDIR_TRACE_FRAMES`/`_IME`/`_CLIP`/`_WINDOW` — dir2가 이미 쓰는 이름이 있으면 그것을 따른다(15번 문서 확인) |
| SKEL-437 | 덤프 명령 | 영역마다 `<영역>.dump:<파일>`(목록 행·선택·경로·정렬·필터 상태를 글로) — 화면 캡처 없이 3-OS에서 같은 판정 |
| SKEL-438 | 앱 안 입력 | `ui.move/click/dclick/rclick/wheel:x/y`를 실제 `route`에 넣는다(SKEL-160) · 키는 명령 id 직접 호출 |
| SKEL-439 | 생존 판정 | 시나리오 = 격리 홈 + 기동 명령 + `@after` 종료 명령 → 종료 코드·덤프 파일·stderr 패닉 유무로 판정(`D/30-architecture-patterns.md:72` win-func-check 틀) |
| SKEL-440 | 순수 함수 우선 | OS·창 없이 시험되는 판정을 최대화(인자 해석 · 기하 · 키 변환 · 라우팅 판정 · 명령 문지기) — 3-OS CI에서 `cargo test`만으로 핵심 회귀를 잡는다 |

---

### 부록 A. 항목 수

§1 골격 요소 167(SKEL-001~167) · §3 OS 분기 34(SKEL-201~234) · §4 함정·교훈 50(SKEL-301~350) · §5 제안 40(SKEL-401~440) = **291**.

### 부록 B. 이 문서가 확정하지 않은 것(다음 단계에서 닫을 것)

1. macOS 전역 메뉴 막대를 둘지(SKEL-420) — nexa-sql에는 구현이 없다.
2. 호스트 부품의 nexa-ui 승격 시점(SKEL-433).
3. 릴리스 프로필 `panic = "abort"`와 워커 패닉 격리의 관계(SKEL-343 — 추정, 실측 필요).
4. `InputEvent`에 더블클릭·가운데/뒤로·앞으로 버튼이 있는지(SKEL-063 — nexa-ctl 문서에서 확인).
5. nexa-gfx 이미지 디코더가 dir2 아이콘 형식을 덮는지(SKEL-134).
6. `Focus`·창·명령의 실제 목록(SKEL-417·419·421 — dir2 인벤토리 10~19번이 원천).
