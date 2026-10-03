# 23 · 값 분류와 설정 추출 원장(상수 · 고급 설정 · 일반 설정)

> 작성 2026-10-03(초안). 결정 = [10](10-decision-record.md) **DR-19**(값 분류 원칙 · 사용자 10-03 지시). 이 표는 [22](22-dir3-features.md) NEW-008(성능 향상 모드)의 **선행 작업 목록**이기도 하다 — 덮어쓸 대상이 먼저 설정 키여야 한다.
> 범위 = `crates/nexa-dir/src/**` · `crates/ndir-*/src/**`(ndir-check · `#[cfg(test)]` 제외). 줄 번호는 **10-03 조사 시점** 기준이라 이후 편집으로 몇 줄 밀릴 수 있다(값·이름으로 찾는다).
> 분류: **상수** = 바뀌지 않는 고정값(포맷 · OS · 프로토콜 한계 · 정합성 불변식) / **고급** = 바꾸면 위치·속도가 달라지지만 자주 안 바꾸거나 다른 곳에 영향(폴링 · 캐시 상한 · 임계 · 타임아웃 · 내부 레이아웃 지표 · 샌드박스 한도) → REGISTRY `ADVANCED`(드물면 `HIDDEN`) / **설정** = 자주 바꾸거나 취향(보이는 크기 · 체감 시간 · 켜고 끄기).
> 상태 칸: ☐ 미추출 · ✅ 추출됨(키 존재). 추출하면 값을 키 기본값으로 옮기고 코드는 키를 읽는다(i18n 라벨 · 설명 = DR-14).

## 0-1. 내부 전용 설정(INTERNAL · §81)

- 사용자에게 **절대 보이면 안 되는 키**는 `ndir_settings::INTERNAL`(+ `is_internal`)에 둔다 — 설정 창(고급 포함) · 검색 · JSON 내보내기에서 빠지고, 가져오기에서는 알 수 없는 키로 보고된다. `HIDDEN`(설정 창에는 안 보이지만 고급/JSON으로는 다룰 수 있음)과 다르다.
- 현재: `license.gates`(라이선스 기능 게이트 자리 · DR-4 · 사용자 10-03 "라이선스 게이트는 보이면 안 됨"). 시험 `internal_keys_never_surface`.

## 0. 먼저 고칠 불일치(설정 범위 ↔ 코드 클램프)

| 키 | REGISTRY 범위 | 코드 클램프 | 조치 제안 |
| --- | --- | --- | --- |
| `layout.dock_height_pct` | 15..50 | `main.rs` `clamp(5, 50)` | 코드를 키 범위(15..50)로 맞춘다 |
| `layout.dock_split_pct` | 15..85 | `main.rs` `clamp(10, 90)` | 코드를 키 범위(15..85)로 맞춘다 |

- 키가 없는데 주석·문서에 이름만 있는 것: 토스트 `ui.toast_*`(nexa-ctl `configure_progress` 문서 주석) · 복사 버튼 `copy_feedback_ms`(`prefs_win.rs`가 `DEFAULT_FEEDBACK_MS` 고정).
- 예시로 거론됐으나 코드에 없는 것: `WATCH_CAP 64`(감시 대상 = 두 패널의 활성 폴더뿐) · 별도 디바운스 250 ms 상수(통지 감시자의 `poll_interval_ms()` 반환 250이 그 역할).

## 1. 원장

| 값 | 위치 | 뜻 | 기존 키 | 분류 | 제안 키 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| **감시** | | | | | | |
| 1000 ms | `platform/mod.rs:237` | 폴링 감시 기본 간격(폴백) | — | 고급 | `watch.poll_ms` | ☐ |
| 250 / 1000 ms | `platform/winwatch.rs:278,280` · `linuxwatch.rs:218,220` · `macwatch.rs:250,252` | 통지 감시 틱(디바운스) / 폴백이 섞이면 1000 | — | 고급 | `watch.notify_ms`(+ `watch.poll_ms`) | ☐ |
| 1000 ms | `platform/winwatch.rs:114` | 감시 스레드 무장 대기 상한 | — | 상수 | — | — |
| 8 KiB · 64 KiB · 64 | `winwatch.rs:142` · `linuxwatch.rs:113` · `macwatch.rs:144` | OS 감시 읽기 버퍼 · kevent 배열 | — | 상수 | — | — |
| **아이콘** | | | | | | |
| 150 ms | `app/row_icons.rs:23` | 행 아이콘 결과 폴링 | — | 고급 | `list.icon_poll_ms` | ☐ |
| 512 | `app/row_icons.rs:25` | 행 아이콘 캐시 상한 | — | 고급 | `list.icon_cache_max` | ☐ |
| 150 ms | `app/launcher_icons.rs:9` | 런처 아이콘 폴링 | — | 고급 | `launcher.icon_poll_ms` | ☐ |
| 14 / 여백 4 | `panel.rs:112-113` | 네비 버튼 글리프 크기·여백 | — | 고급 | `list.nav_icon_size` | ☐ |
| 8..256 · 32/64 | `icons.rs:78` · `icon.rs:85,89` | SVG 래스터 클램프 · 창 아이콘 크기(OS) | — | 상수 | — | — |
| **셸 메뉴** | | | | | | |
| 300 ms | `app/ctxmenu.rs:57` | 선행 구축 머무름(`CTX_PREBUILD_MS` · dir2 값) | — | 고급 | `ctxmenu.prebuild_ms` | ☐ |
| 256 | `app/ctxmenu.rs:59` | 선행 구축 대상 선택 수 상한 | — | 고급 | `ctxmenu.prebuild_max` | ☐ |
| 30 ms | `app/ctxmenu.rs:61` | 구축/실행 대기 틱 | — | 고급 | `ctxmenu.poll_ms` | ☐ |
| 20 | `panel.rs` `PATH_SUGGEST_MAX`(§85 · dir2 win.rs:7199-7206) | 경로 자동완성 제안 개수 상한 | — | 상수(dir2 고정값) | — | — |
| 3000 ms | `app/ctxmenu.rs` `CTX_WAIT_MAX_MS`(§81) | 우클릭 메뉴를 셸 항목과 함께 열기 위해 기다리는 상한(넘으면 자체 항목만) | — | 상수(DR-20 1초 규칙과 짝 · 상태줄 `ctx.loading` 표시) | — | — |
| 240 px · 220 px | `app/ctxmenu.rs:83` · `app/input.rs:451` | 컨텍스트 메뉴 · 탭 메뉴 텍스트 폭 | — | 고급 | `ctxmenu.text_w` · `tabs.menu_text_w` | ☐ |
| 2 | `platform/winshell.rs:52` | 셸 서브메뉴 열거 깊이 | — | 고급 | `ctxmenu.shell_depth` | ☐ |
| 64 px | `platform/winshell.rs:54` | 셸 항목 아이콘 최대 변 | — | 상수 | — | — |
| 30 s · 25 ms | `platform/winshell.rs:56,58` | 메뉴 스레드 동기 대기 상한 · 펌프 주기 | — | 고급 | `ctxmenu.sync_timeout_ms` · `ctxmenu.pump_ms` | ☐ |
| 10회 × 20 ms | `platform/winshell.rs:282,779` | 새로 만들기 뒤 신규 항목 감지 재시도 | — | 고급 | `ctxmenu.detect_retries` | ☐ |
| **파일 작업** | | | | | | |
| 100 ms | `app/ops.rs:53` | 전송 진행 폴링 | — | 고급 | `transfer.poll_ms` | ☐ |
| 4 MiB | `ndir-ops/src/lib.rs:42` | 복사 버퍼(`COPY_BUF`) | — | 고급 | `transfer.copy_buf_kb`(M9 전략 계층과 함께 · NEW-007) | ☐ |
| 100 | `ndir-ops/src/history.rs:58` | 실행 취소 기록 상한 | — | 고급 | `history.max` | ☐ |
| 64 | `app/bulk.rs:51` | 일괄 이름 바꾸기 프리셋 상한 | — | 고급 | `bulk.preset_max` | ☐ |
| 10×10 · 3×15 · 5×10 ms | `platform/windows.rs:185,191,293,298` · `clipboard.rs:82,87` | 클립보드 열기·HDROP 재시도 | — | 상수 | — | — |
| **배치·레이아웃** | | | | | | |
| 3.0 · 20.0 · 200.0 | `main.rs` `SPLIT_TH` · `SNAP_PX` · `MIN_PANEL` | 스플리터 두께(세 스플리터 공통 · §103) · 50 % 자석 스냅(+ 서로의 분할선) · 패널 최소 폭 | — | 고급 | `layout.splitter_px` · `layout.snap_px` · `layout.min_panel_w` | ☐ |
| 20 | `main.rs:111` | 툴바 아이콘 기본 크기 | `toolbar.icon_size` | 설정 | (기존) | ✅ |
| 4 | `app/settings.rs:193` | 툴바 그룹 간격 기본 | `toolbar.group_gap` | 고급 | (기존) | ✅ |
| 1 | `app/settings.rs` `make_tool_dock`(§76) | 툴바 아이콘 둘레 여백(칸 = 아이콘 + 2) | `toolbar.icon_pad`(0~8) | 고급 | (기존) | ✅ |
| 8 · 26 · 12 · 20 % · 4 px(§80 재조정 · §78은 18/55/12) | `app/settings.rs` `make_tool_dock`(nexa-ctl `SoftStates`) | 툴바 상태 표시 농도(hover 채움 · 켜짐 채움 · 켜짐 테두리 · 단계 · 모서리) | `toolbar.hover_fill_pct` · `toolbar.on_fill_pct` · `toolbar.on_line_pct` · `toolbar.state_step_pct` · `toolbar.state_radius` | 고급 | (기존) | ✅ |
| off | ndir-vfs `is_protected_os_item` · ndir-tree `Filter.show_protected`(§83) | 보호된 운영 체제 파일(숨김 + 시스템 속성 · macOS UF_HIDDEN + SF_RESTRICTED) 표시 — 꺼져 있으면 `list.show_hidden`이 켜져 있어도 숨김(탐색기 규칙) | `list.show_protected` | 설정(일반 · 탐색기 권장값 off · **기본 off = 사용자 확정 10-03** · dir2와 다른 의도된 차이) | (기존) | ✅ |
| 3.0 · 0.7 | `main.rs` `SPLIT_HALF` · `SPLIT_HOVER_ALPHA`(§103) | 스플리터 잡는 띠가 틈 양쪽으로 넓어지는 폭(배율 적용 — dir2는 일부 비배율) · hover 강조 알파 상한(서서히 · NEW-017) | — | 상수 | — | — |
| off(dir2 on) | `ndir-settings/src/registry.rs` `term.wrap`(§107) | 터미널 줄 바꿈 기본값 — 끄면 고정 열 `term.cols`(240) + 가로 스크롤(사용자 10-03 "줄바꿈은 기본으로 꺼지도록" · **의도된 차이** · dir2 설정 가져오기는 on 유지) | `term.wrap` | 설정 | (기존) | ✅ |
| off · 새 탭 기본값 | `app/menus.rs`(§105 · 523857f) | 대소문자 구분 정렬 — 탭 보기 옵션 4번째(툴바 토글 · 보기 관리 방법만큼 적용) · 설정은 **새 탭의 기본값**(§104의 전역 → 정정) | `list.sort_case_sensitive` | 설정 | (기존) | ✅ |
| system(dir2 dark) | `ndir-settings/src/registry.rs` `ui.theme`(§104) | 테마 기본값 — 시스템 따름(사용자 10-03 "테마는 시스템을 기본값으로" · **의도된 차이** · dir2 설정 가져오기는 dark 유지) | `ui.theme` | 설정 | (기존) | ✅ |
| dir(§104 · §102 tab · dir2 panel) | `ndir-settings/src/registry.rs` · `migrate.rs` · `app/menus.rs::toggle_view_option`(§102 · §104) | 보기 토글(숨김 · Dot · 폴더 우선) 전파 폭 — dir = 그 폴더를 보는 탭 전부 + 폴더별 기억(NEW-018) · tab = 활성 탭 · panel = 활성 패널 전 탭 · global = 두 패널 전 탭 | `list.view_scope` | 설정(**기본 dir = 의도된 차이** · 사용자 10-03 "기본은 디렉토리 단위" · 사용자 10-03 "이후는 탭별" · dir2 `settings.cfg` 가져오기는 panel 유지) | (기존) | ✅ |
| off | `app/settings.rs`(§80) | 켜진 툴바 아이콘을 강조색으로 칠할지(기본 = 본문색 · dir2 규약) | `toolbar.on_icon_accent` | 고급 | (기존) | ✅ |
| em 13 · em 9 | `app/fonts.rs` `NAV_GLYPH_EM` · `CHEVRON_EM`(§78 · dir2 dw.rs:331-350) | 네비 글리프 · 쉐브론 크기(dir2 DIP 그대로) | — | 상수(dir2 규약) | — | — |
| +3 px(종전 −4) | `app/fonts.rs` `FALLBACK_CHEVRON_DELTA` · `fallback_chevrons`(§94) | 글자 대체 쉐브론 크기 증분 · 후보 = › ⌄ → › ˅ → ▸ ▾ → > v 중 둘 다 그릴 수 있는 첫 쌍 — **§98부터 선 쉐브론을 끈 경우의 예비**(아이콘 글꼴 없는 OS는 선 쉐브론이 기본) | — | 상수(예비) | — | — |
| 칸 16 → 닫힘 4×8 · 열림 8×4 · 굵기 1 | nexa-ui `nexa-grid/src/rows.rs` `marker_chevron_points` · dir3 `app/fonts.rs`(`set_marker_vector` · §98) | 선 쉐브론 크기(긴 변 = 칸 폭 절반 짝수 · 짧은 변 = 그 절반 · Segoe MDL2 em 9 잉크 추정값) · 아이콘 글꼴(MDL2) 없는 OS만 | — | 상수(사용자 "해결" 10-03) | — | — |
| 3줄 고정 | `app/input.rs`(터미널) · `preview_win.rs`(F3) | 휠 1노치 줄 수 → §76부터 OS 값(`SPI_GETWHEELSCROLLLINES`) | — | 상수(OS 값 따름) | — | ✅ |
| 20 / 글꼴+6 / ≥14 | `main.rs:308` | 목록 행 높이 | — | 설정 | `list.row_h`(0 = 글꼴 기준 자동) | ☐ |
| 6 · 16 | `main.rs:309-310` | 행 좌우 여백 · 트리 들여쓰기 | — | 고급 · 설정 | `list.pad_x` · `list.indent_w` | ☐ |
| 22 · 24 | `main.rs:311-312` | 탭 바 높이 · 경로/네비 바 높이 | — | 고급 | `tabs.height` · `list.bar_h` | ☐ |
| 340/64/96/140/110 · 120/8 | `main.rs:321-325` · `panel.rs:473-489` | 기본 열 폭(+ 내 PC 열) · 이름 열 최소/여유 | (`list.col_layout`이 사용자 폭 기억) | 고급 | `list.col_default_w` · `list.col_name_min` | ☐ |
| 글꼴+11 · 24 · 22 | `main.rs:630,640,648` | 메뉴 바 · 퀵 런처 바 · 상태 바 높이 | — | 고급 | `ui.menubar_pad` · `launcher.bar_h` · `statusbar.height` | ☐(런처는 개발 세션 진행 중) |
| row_h × 3 | `main.rs:660` | 도크 최소 높이 | — | 고급 | `dock.min_rows` | ☐ |
| 3000 ms · 85 % | `main.rs:359` | 토스트 표시 시간 · 불투명도 | — | 설정 | `ui.toast_ms` · `ui.toast_alpha` | ☐ |
| 클램프 10..90 · 100..2000 · 80..1000 | `main.rs:616,722` · `app/input.rs:20` · `termview.rs:162` | 키 범위와 같은 클램프 | `layout.panel_split_pct` · `ui.dblclick_ms` · `term.cols` | 상수 | — | — |
| **이벤트 루프·세션** | | | | | | |
| 16 ms | `app/event_loop.rs:137` | 애니메이션 프레임 간격 | — | 고급 | `ui.frame_ms` | ☐ |
| 250 ms · 1000/5000 ms | `app/sessions.rs:61` · `main.rs:430` | 세션 dirty 틱 · 저장 디바운스(조용/최대) | — | 고급 | `session.tick_ms` · `session.save_quiet_ms` · `session.save_max_ms` | ☐ |
| 200 | `session.rs:66` | 탭당 펼친 노드 저장 상한 | — | 고급 | `session.expanded_max` | ☐ |
| 3600 s · 64 · 200×150~10000 | `app/event_loop.rs:139` · `session.rs:161` · `wingeom.rs:13` | 유휴 최대 대기 · 손상 방어 · 창 크기 유효 범위 | — | 상수 | — | — |
| **터미널** | | | | | | |
| 530 ms | `termview.rs:18` | 캐럿 깜빡임 | — | 설정 | `term.caret_blink_ms` | ☐ |
| 30 ms | `termview.rs:20` | 출력 폴링 | — | 고급 | `term.poll_ms` | ☐ |
| 6 ms | `termview.rs` `PUMP_BUDGET_MS`(§75) | 펌프 1회 시간 예산(넘으면 backlog → 1 ms 뒤 재개) | — | 고급(HIDDEN) | `term.pump_budget_ms` | ☐ |
| 256 KiB | `platform/winpty.rs` `BACKLOG_CAP`(§75) | ConPTY 읽기 버퍼 상한(넘으면 읽기 쉼 = 셸 역압) | — | 고급(HIDDEN) | `term.backlog_kb` | ☐ |
| DejaVu Sans Mono · (Noto Sans CJK, Noto Sans Mono CJK KR) | `app/fonts.rs` `MONO_FALLBACK_FAMILIES` · `MONO_CJK_FACE`(§98) | 터미널 고정폭 폴백(➜ ✗ 한 칸 폭 · TTC 안 고정폭 한글 얼굴) — 한글 UI 글꼴 앞 · 폴백 em 맞춤(`term_fallback_em_match` = Windows 밖) | — | 상수(후보 목록) | — | — |
| 16자 평균 반올림 | `termview.rs::grid_dims`(§98) | 터미널 칸 폭(종전 "M" 1자 올림 → 8.21 px가 9 px) — 전 OS 공통 · ⚠ Windows 실기 필요 | — | 상수 | — | — |
| 12종 · U+E0A0/F07B/E0B0 | `app/fonts.rs` `NERD_FAMILIES` · `NERD_PROBE`(§75) | 자동 폴백 Nerd Font 후보 · 판정 글리프 | `term.fallback_fonts`(사용자 지정 폴백 · 고급 · §75) | 상수(후보 목록) | — | ✅(사용자 지정은 키로) |
| 3줄 · 4열 /노치 | `app/input.rs:319,326` | 세로·가로 휠 이동량 | — | 설정 | `term.wheel_lines` · `term.hwheel_cols` | ☐ |
| 800 | `ndir-term/src/lib.rs:48` | 스크롤백 줄 상한(`MAX_SCROLLBACK`) | — | 설정 | `term.scrollback` | ☐ |
| 80×24 · 8192 · 4096 · 65535 · 5 ms | `termview.rs:119,223` · `winpty.rs:290` · `ndir-term/src/lib.rs:807` · `unixpty.rs:151` | 초기 PTY 크기 · 읽기 버퍼 · CSI 상한 · 재시도 | — | 상수 | — | — |
| **클립보드(X11)** | | | | | | |
| 1200 ms | `clipboard_x11.rs:39` | 붙여넣기 응답 대기 | — | 고급 | `clipboard.x11_timeout_ms` | ☐ |
| 128 KiB · 15 ms · 2 s · 5 ms | `clipboard_x11.rs:37,311,315,513,539` | INCR 경계 · 스레드 sleep/준비 · 폴링 | — | 상수 | — | — |
| **기동·자가 점검** | | | | | | |
| 20 s | `app/startup_cmd.rs:532` | `ctx.wait` 보류 상한 | — | 고급(HIDDEN) | `startup.ctx_wait_ms` | ☐ |
| 120 · 3 | `app/startup_cmd.rs:15,243,251` | 휠 1노치 delta · assert 실패 종료 코드 | — | 상수 | — | — |
| 30 · 20 · 5 · 50 ms · 30 s · ×4 | `selfcheck.rs:554-632` | 점검 대기·폴링·"즉시" 임계 | — | 상수(시험 하네스) | — | — |
| **미리보기·플러그인·압축** | | | | | | |
| 16 KiB · 200줄 | `preview/mod.rs:45-46` | 텍스트 미리보기 읽기 · 줄 상한 | — | 고급 · 설정 | `preview.text_read_kb` · `preview.text_lines` | ☐ |
| 256 KiB · 2000 px | `preview/mod.rs:107,135` | SVG 입력 · 한 변 상한 | — | 고급 | `preview.svg_max_kb` · `preview.svg_max_px` | ☐ |
| 60 | `dockinfo.rs:120` | 도크 압축 요약 항목 상한 | — | 설정 | `dock.archive_rows` | ☐ |
| 2억 · 64 MiB · 256 KiB · 1 MiB · 4/64 MiB · 1500 ms · 3 · 8 MiB · 1000줄/4096자 | `preview/wasm.rs:21-43,393-394` | WASM 연료 · 메모리 · 입출력 · read_at · 호출 타임아웃 · 격리 횟수 · 모듈 크기 · 출력 자르기 | — | 고급(HIDDEN · DR-7 격리 수치) | `plugins.fuel` · `plugins.mem_mb` · `plugins.read_kb` · `plugins.out_kb` · `plugins.read_at_mb` · `plugins.call_timeout_ms` · `plugins.breaker_limit` · `plugins.module_mb` · `plugins.out_lines` | ☐ |
| 50,000 | `ndir-vfs/src/archive/mod.rs:39` · `preview/wasm.rs:37` | 압축 목록 항목 상한 | — | 고급 | `archive.max_entries` | ☐ |
| 1000/64 · 20만/1만 · 4096 · 압축 포맷 상한 | `preview/wasm.rs:33-35,110-262` · `ndir-vfs/src/archive/*` | 연료 비용 · 문자열 버퍼 · 포맷 고정값 | — | 상수 | — | — |
| **보조 창·위젯** | | | | | | |
| 2000 ms(300..10000) | `copybtn.rs:15,50` | 복사 완료 표시 복귀 | —(`prefs_win.rs:281` 고정) | 설정 | `ui.copy_feedback_ms` | ☐ |
| 1200 / 700 ms | `license_win.rs:49-50` | 플래시 유지 · 페이드 | — | 고급 | `ui.flash_hold_ms` · `ui.flash_fade_ms` | ☐ |
| 50 ms · 12행 · 5 px | `preview_win.rs:86` · `order_win.rs:59-60` | 드래그 자동 스크롤 틱 · 순서 목록 행 · 드래그 시작 임계 | — | 고급 | `preview.autoscroll_ms` · `ui.order_rows` · `ui.drag_threshold` | ☐ |
| 32 | `launcher.rs:13` | 런처 항목 상한 | — | 고급 | `launcher.max_items` | ☐ |
| 900×640 · 960×640(소유자 0.75 비례) | `preview_win.rs:406-409` · `archive_win.rs:422-424` | 미리보기·압축 창 기본 크기 | — | 고급 | `window.preview_size` · `window.archive_size` | ☐ |
| 대화상자 PAD/BTN/ROW/열 폭 묶음 | `dlg_win.rs:89-93` · `bulk_win.rs:542-548` · `check_win.rs:31-38` · `keys_win.rs:37-42` · `order_win.rs:52-58` · `prefs_win.rs:61-68` · `progress_win.rs:26-28` · `archive_win.rs:34` · `preview_win.rs:28-29` · `license_win.rs:423-532` | 보조 창 레이아웃 지표 | — | 고급(묶음 · 1차 = 상수 유지 권장) | `ui.dialog_metrics`(후속) | ☐ |
| 120 ms · 2 · row_h≥20/8/16 | `copybtn.rs:13` · `launcher.rs:11` · `archive_win.rs:315` | 눌림 효과 · 런처 시드 버전 · VirtualRows 인자(8·16 뜻 확인 필요) | — | 상수(마지막은 확인 필요) | — | — |

## 2. 집계 · 추출 순서 제안

- 분류(행 기준 · 원조사): 상수 52 · 고급 70 · 설정 13. 이미 키가 있는 것 = `toolbar.icon_size` · `toolbar.group_gap` 2건.
- **1차(NEW-008 성능 향상 모드의 덮을 대상)**: `watch.poll_ms`/`watch.notify_ms` · `list.icon_poll_ms`/`list.icon_cache_max` · `term.scrollback` · `ui.frame_ms` · `ui.toast_ms` · `preview.text_lines` · (신설 on/off) 행 셸 아이콘 · 메뉴 아이콘.
- **2차(사용자 체감 설정)**: `list.row_h` · `list.indent_w` · `term.caret_blink_ms` · `term.wheel_lines`/`hwheel_cols` · `ui.copy_feedback_ms` · `dock.archive_rows`.
- **3차(고급 · HIDDEN 다수)**: 셸 메뉴 · 전송 · 세션 · 플러그인 샌드박스 · 레이아웃 지표 · 보조 창 지표(묶음).
- 상수로 남기는 것은 이유(포맷 · OS · 불변식 · 시험 하네스)를 코드 주석에 한 줄 남긴다.
