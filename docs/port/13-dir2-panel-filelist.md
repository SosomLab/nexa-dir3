# 13 · nexa-dir2 인벤토리 — 파일 목록 · 패널 (PANEL)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 이 문서의 `PANEL-NNN` ID가 이후 구현 · 교차 검증의 체크리스트다.
> 근거 표기: `저장소/경로:줄`. 확인하지 못한 것은 **추정**이라고 적었다.
> 원본: nexa-dir2 0.22.0 (Windows 전용) · 대상: nexa-dir3 (winit + softbuffer + nexa-ui).

---

## 0. 범위 — 읽은 파일과 줄 수

| 구분 | 파일 | 줄 수 | 읽은 범위 |
|---|---|---|---|
| 담당 | `nexa-dir2/crates/nexa-app/src/panel.rs` | 2215 | 전부(본문 1~1580 · 테스트 1582~2215) |
| 담당 | `nexa-dir2/crates/nexa-app/src/nav.rs` | 105 | 전부 |
| 담당 | `nexa-dir2/crates/nexa-app/src/source.rs` | 726 | 전부 |
| 담당 | `nexa-dir2/crates/nexa-gui/src/widgets/rows.rs` | 3548 | 전부(본문 1~2368 · 테스트 2370~3548) |
| 담당 | `nexa-dir2/crates/nexa-gui/src/columns.rs` | 58 | 전부 |
| 담당 | `nexa-dir2/crates/nexa-gui/src/typeahead.rs` | 165 | 전부 |
| 담당 | `nexa-dir2/crates/nexa-tree/src/lib.rs` | 1488 | 전부 |
| 담당 | `nexa-ui/docs/21-grid-family.md` | 129 | 전부 |
| 접점(부분) | `nexa-dir2/crates/nexa-gui/src/{draw.rs(127), event.rs(134), fastscroll.rs(495), widget.rs(93), lib.rs(24)}` | — | draw · event · widget · lib 전부, fastscroll 1~400 |
| 접점(부분) | `nexa-dir2/crates/nexa-app/src/win.rs` | 9951 | 패널 · 목록 호출부만(1340~1366, 3340~3410, 3990~4060, 6765~6940, 7855~7958, 8030~8150, 8355~8375, 8566~8700, 8760~8935, 9380~9412, 9488~9606) |
| 접점(부분) | `nexa-dir2/crates/nexa-app/src/config.rs` | 1689 | 설정 필드 · 기본값 · 세션 직렬화(326~349, 836~960, 1075) |
| 접점(부분) | `nexa-dir2/crates/nexa-vfs/src/lib.rs` | 593 | 40~135 + 공개 API 목록 |
| 접점(부분) | `nexa-dir2/crates/nexa-app/src/icons.rs` | 597 | 1~42 + 로더 위치 Grep |
| nexa-ui 확인 | `nexa-ui/crates/nexa-ctl/src/controls/tree.rs` | 1244 | 1~1049 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-ctl/src/{draw.rs(416), event.rs(164), view_mode.rs(97), typeahead.rs(471), lib.rs(80)}` | — | draw · view_mode · lib 전부, event 1~140, typeahead 1~330 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-ctl/src/controls/scroll.rs` | 1175 | 1~470 + 공개 함수 목록 |
| nexa-ui 확인 | `nexa-ui/crates/nexa-fs/src/lib.rs` | 957 | 18~135, 180~335, 445~512, 700~884 |

범위 밖(다른 인벤토리 담당으로 가정): `tabbar.rs` · `pathbar.rs` · `dock.rs` · `chrome.rs`(Toolbar) · `edit.rs` 내부 · `icons.rs` 로더 · `win.rs` 전체 메시지 루프. 이 문서는 이들과의 **접점**만 적는다.

---

## 1. 기능 목록

이식 분류: **N** = 플랫폼 중립(거의 그대로) · **A** = nexa-ui 컨트롤 · 그리기로 교체 · **P** = OS별 구현 분기 필요 · **W** = Windows 전용 유지.
기존 테스트는 `파일::테스트명`으로 적는다(없으면 —).

### 1-A. 패널 구성 · 레이아웃 (`panel.rs`)

| ID | 기능(사용자 관점) | 동작 상세(조건 · 예외 · 기본값) | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-001 | 패널 = 탭 바 + 네비 버튼 + 경로 바 + 파일 목록(+ 도크) 한 묶음 | `Panel` 구조체가 `TabBar` · `Toolbar`(네비) · `PathBar` · `InfoDock` · `Vec<Tab>`을 소유. 창은 패널 2개를 스플리터로 배치하고 활성 패널에 키보드를 라우팅 | `nexa-dir2/crates/nexa-app/src/panel.rs:106-140` | 없음 | A | `panel.rs::layout_stacks_tabbar_navbar_rows` |
| PANEL-002 | 수직 배치: 탭 바 → 네비 행 → 목록 | `tab_h × 탭 줄 수`(멀티라인 탭, `bounds.h`로 클램프) → `bar_h` 행에 네비 버튼 4개(폭 합 `nav_btn_w×4`, 틈 없음) + 경로 바(잔여 폭) → 목록(잔여 높이 전부). 도크는 패널 밖 전폭 밴드(호스트가 `dock.set_bounds`). 경로 자동완성 팝업 하한 = 목록 바닥 | `panel.rs:386-416` | 없음 | N | `panel.rs::layout_stacks_tabbar_navbar_rows` |
| PANEL-003 | DPI에 따른 지표 | `PanelMetrics{row_h, pad_x, indent_w, tab_h, bar_h}` = 96dpi 기준 20/6/16/22/24px, `row_h` 하한 14 | `panel.rs:89-97` · `nexa-dir2/crates/nexa-app/src/win.rs:1345-1354` | DPI 값 취득은 호스트(Win32) | P | — |
| PANEL-004 | DPI 변경 시 재배치 | `set_metrics`가 탭 바 · 네비 · 경로 바 · 도크 · 전 탭 목록에 지표 전파, 컬럼을 새 기본 컬럼으로 **교체**(사용자 폭 리셋 — 현행 동작) | `panel.rs:453-467` | 없음 | N | — |
| PANEL-005 | 활성/비활성 패널 구분 | `set_focused`가 탭 바와 전 탭 목록의 포커스 색을 바꾼다. 터미널 포커스 중에는 활성 패널도 비활성 색 | `panel.rs:472-478` | 없음 | N | — |
| PANEL-006 | 페인트 순서 | 목록 → 도크(표시 시) → 네비 버튼 → 경로 바 → 탭 바 → 진행 배지 → 경로 자동완성 팝업(최상단) | `panel.rs:480-493` | 없음 | A | — |
| PANEL-007 | 진행 배지(클라우드 로딩/전송) | 목록 영역 **우하단 플로팅**. 문구 `None`이면 숨김, 변경 시에만 무효화. 목록 폭 < 80 또는 높이 < 24면 생략. 선택 · 키보드에 영향 없음 | `panel.rs:496-526` | 없음 | A | — |
| PANEL-008 | 도크 표시 · 높이 비율 | `set_dock_visible` · `set_dock_ratio`(0.15~0.5 클램프, 기본 0.3). `dock_shown()` = 표시 플래그 **그리고** 도크 높이 > 0(히트 · 키 라우팅 · 터미널 생존 판정은 전부 이것) | `panel.rs:419-451` | 없음 | N | `panel.rs::dock_shown_requires_visible_and_height` |
| PANEL-009 | 마우스 이벤트 y-라우팅 | Down/RightDown: y로 탭 바 / 네비 행(x로 버튼 vs 경로 바) / 도크(`dock_shown`일 때만) / 목록. Move: 전 위젯에 전달. Up: 탭 바 · 경로 바 · 목록 · 도크에 전달(드래그 종료 누락 방지). 그 외(휠 · 키 · 문자) = 활성 탭 목록. 도크 밖 클릭 = 도크 텍스트 선택 해제 | `panel.rs:1458-1505` | 없음 | N | `panel.rs::panel_nav_buttons_drive_this_panels_tab_history` |
| PANEL-010 | 위젯 동작 수거 | `drain_actions`: 탭 동작(Switch/Close/New/Move/Context) · 경로 바 탐색 · 네비 버튼 명령을 활성 탭에 적용 | `panel.rs:1508-1542` | 경로 해석에 `shellpath::resolve`(Windows, `panel.rs:1529-1530`) | P | `panel.rs::panel_nav_buttons_drive_this_panels_tab_history` |
| PANEL-011 | 시작 폴백 루트 | `C:\` → `A:\`~`Z:\` → `%USERPROFILE%` 순 첫 성공 트리. 전부 실패 = `None`(호출부 expect) | `panel.rs:19-24` | 드라이브 문자 · `USERPROFILE` | P | `panel.rs::open_any_root_finds_a_root`(`cfg(windows)`) |

### 1-B. 탭 (`panel.rs`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-012 | 탭 = 독립 뷰 상태 | `Tab{rows, nav, expanded, locked, pinned, show_hidden, show_dotfiles, folders_first, enum_filters, stale}` — 목록 위젯 · 히스토리 · 펼침 집합 · 보기 옵션을 탭이 소유 | `panel.rs:37-59` | 없음 | N | `panel.rs::per_tab_history_is_independent` |
| PANEL-013 | 탭 제목 | 가상 최상위 = `tr("nav.mypc")` · 클라우드 = 폴더명/연결 라벨 · 일반 = 마지막 경로 요소 · 드라이브 루트 = `D:`(후행 `\` 제거) | `panel.rs:68-86` | 드라이브 루트 표기 | P | — |
| PANEL-014 | 새 탭(Ctrl+T · [+]) | 현재 경로 복제. 컬럼 · 포커스 · 보기 모드 · 보기 옵션(숨김/Dot/폴더 우선) 계승, 히스토리는 새로. 열기 실패 시 무동작. 새 탭이 활성 | `panel.rs:560-589` | 없음 | N | `panel.rs::tabs_open_switch_close_keep_at_least_one` · `view_mode_is_per_tab` |
| PANEL-015 | 탭 전환 · 순환(Ctrl+Tab) | 전환된 탭을 `stale = true`로 표시(비활성 탭은 감시 대상이 아니므로 활성화가 갱신 계기). `next_tab`은 탭 2개 이상일 때 순환 | `panel.rs:738-756` | 없음 | N | `panel.rs::tab_activation_marks_stale_and_reopen_clears` |
| PANEL-016 | 탭 닫기(Ctrl+W · ×) | 패널은 항상 탭 1개 이상 · 잠긴 탭은 닫지 않음. 활성 탭이 닫히면 드러난 이웃 탭 `stale = true` | `panel.rs:720-736` | 없음 | N | `panel.rs::tabs_open_switch_close_keep_at_least_one` · `tab_move_lock_duplicate` |
| PANEL-017 | 탭 드래그 재정렬 | `move_tab(from, to)` — 활성 인덱스 추종(잡은 탭 · 사이 탭 밀림 모두) | `panel.rs:592-611` | 없음 | N | `panel.rs::tab_move_lock_duplicate` |
| PANEL-018 | 탭 복제 | 같은 경로 + 펼침 집합 복사, 바로 옆에 삽입 후 활성. 복제본은 잠금 · 고정 해제 | `panel.rs:614-647` | 없음 | N | `panel.rs::tab_move_lock_duplicate` |
| PANEL-019 | 탭 잠금 | 토글 · 조회 · 세션 스냅샷/복원(부족분 무시) | `panel.rs:650-660` · `705-717` | 없음 | N | `panel.rs::tab_move_lock_duplicate` |
| PANEL-020 | 탭 고정(핀) | 토글 시 핀 그룹 경계(자신 제외 고정 탭 수) 위치로 이동. 세션 스냅샷/복원 | `panel.rs:663-697` | 없음 | N | `panel.rs::session_dirty_flag_on_tab_ops` |
| PANEL-021 | 탭 우클릭 메뉴 요청 | `TabAction::Context(i)` → `pending_tab_menu`에 1회성 보관, 호스트가 `take_tab_menu`로 수거해 메뉴 표시 | `panel.rs:700-702` · `1515` | 메뉴 표시는 호스트(네이티브 팝업) | A | — |
| PANEL-022 | 패널 간 탭 이동 | `detach_tab`(마지막 탭 거부) → `attach_tab(at)`(대상에서 활성 · 컬럼은 대상 패널 상속 · 포커스 상속). 범위가 "활성 탭"이 아니면 `adopt_view_values`/`adopt_panel_view`로 대상 패널 값 채택 | `panel.rs:1107-1163` | 없음 | N | `panel.rs::detach_attach_moves_tab_between_panels` |
| PANEL-023 | 세션 복원 | 경로 목록으로 탭 열기(실패 탭 건너뜀 · 전부 실패 시 `fallback` → `open_any_root`). 탭별 펼침 목록을 부모 우선으로 재적용. 활성 인덱스 클램프 | `panel.rs:200-266` | 폴백 루트 | P | `panel.rs::restore_seeds_expanded_from_session` |
| PANEL-024 | 세션 스냅샷 | 탭 경로 · 활성 인덱스 · 탭별 펼침 경로(**탭당 상한 200**) · 잠금 · 고정 · 보기 모드 · 보기 옵션 플래그 | `panel.rs:269-289` · `687-707` · `1095-1104` · `1252-1264` | 없음 | N | `panel.rs::per_tab_view_values_inherit_and_stale` · `view_mode_is_per_tab` |
| PANEL-025 | 세션 저장 요청 플래그 | 탭/경로/구성 변경 시 `session_dirty = true`, 호스트가 `take_session_dirty`로 수거해 디바운스 저장(1000ms) | `panel.rs:1175-1177` · `win.rs:91` · `win.rs:5072` | 타이머는 호스트 | N | `panel.rs::session_dirty_flag_on_tab_ops` |

### 1-C. 네비게이션 · 재열람 (`panel.rs` · `nav.rs`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-026 | 탭별 뒤로/앞으로 히스토리 | `History{entries, pos}`: `push`(현재와 같으면 무시 · 앞으로 기록 절단) · `replace`(위치 이동 없음) · `back`/`forward` · `can_back`/`can_forward` | `nexa-dir2/crates/nexa-app/src/nav.rs:6-61` | 없음 | N | `nav.rs::push_back_forward_roundtrip` · `push_truncates_forward_branch` · `push_same_path_is_noop_and_replace_keeps_position` |
| PANEL-027 | 경로 진입 | `navigate_to`: 열기 성공 시 히스토리 push 후 소스 교체. 실패 시 위치 유지 + 경로 바를 현재 경로로 복귀 | `panel.rs:945-953` | 없음 | N | `panel.rs::navigate_failure_keeps_position` |
| PANEL-028 | 뒤로 · 앞으로(Alt+← · Alt+→ · 마우스 X버튼) | 히스토리 이동 후 열기. 실패 시 히스토리 위치 원복 | `panel.rs:955-979` · `win.rs:8592-8604` · `win.rs:8913-8918` | 없음 | N | `panel.rs::per_tab_history_is_independent` |
| PANEL-029 | 위로(Alt+↑) + 떠난 폴더 자동 선택 | 부모로 이동 후 방금 떠난 폴더를 캐럿 + 단일 선택, 뷰 배치는 설정 `nav_up_align`(top/center/bottom, 기본 center). 드라이브 루트 → 가상 최상위. 클라우드는 전용 부모 계산. 가상 최상위에서는 무동작 | `panel.rs:985-1007` · `1326-1344` | 드라이브 루트 · 가상 최상위 개념 | P | `panel.rs::nav_up_from_drive_root_enters_my_pc`(`cfg(windows)`) |
| PANEL-030 | [홈] — 가상 최상위로 한 번에 | 깊이와 무관하게 `MY_PC`로 이동, 떠난 위치(클라우드면 연결 루트 행)를 선택. 이미 최상위면 무동작 | `panel.rs:1011-1022` | 가상 최상위 | P | `panel.rs::nav_home_jumps_to_my_pc_from_any_depth`(`cfg(windows)`) |
| PANEL-031 | 네비 버튼 [홈][←][→][↑] | 버튼 id 4/1/2/3, 폭 = `row_h + pad_x`. 이전/다음 = 히스토리 가능 시만 활성, 위 · 홈 = 가상 최상위에서 비활성(흐린 글리프). hover 강조 | `panel.rs:100-104` · `549-554` · `1546-1565` | 글리프 = Segoe MDL2 Assets PUA(`U+EA8A` · `U+E72B` · `U+E72A` · `U+E74A`) | A | `panel.rs::nav_home_button_hovers_like_the_others` |
| PANEL-032 | 탭 바 · 경로 바 동기 | `sync_chrome`: 탭 제목 · 잠금 · 고정 목록 설정, 경로 바 문자열(가상 최상위 = 라벨 · 클라우드 = 표시 경로) | `panel.rs:529-555` | 없음 | N | `panel.rs::tabs_open_switch_close_keep_at_least_one` |
| PANEL-033 | 행 활성화(더블클릭 · Enter · Alt+↓) | 폴더 = 진입 · 파일 = 경로 반환(실행은 호스트). 범위 밖 행 = `None` | `panel.rs:1348-1365` · `win.rs:8676-8686` · `8864-8869` · `8922-8929` | 파일 실행(ShellExecute)은 호스트 | P | — |
| PANEL-034 | 경로 바 제출 해석 | 클라우드 표시 경로 → 센티널 복원 후 탐색 · 환경 변수 확장(`%VAR%` · `$env:VAR` · 따옴표) · `shell:` 특수 폴더(Windows) | `panel.rs:1518-1532` | `shellpath::resolve`(Windows) · 환경 변수 문법 | P | — |
| PANEL-035 | 열기 실패 격리 | `open_source`가 `Tree::open_filtered` 실패 시 `None`(stderr 로그) — 호출자가 위치 유지 | `panel.rs:1568-1580` | 없음 | N | `panel.rs::navigate_failure_keeps_position` |
| PANEL-036 | 무간섭 재열람(F5 · watcher · 필터 토글) | `reopen_filtered`: 선택 경로 · 캐럿 경로 · 세로/가로 스크롤 스냅샷 → 재열기(히스토리 `replace`) → 펼침 복원(`apply_source`) → 선택(Toggle로 재삽입) · 캐럿 · 스크롤 복원. 소실 항목은 무시 | `panel.rs:1394-1453` | 없음 | N | `panel.rs::reopen_preserves_expanded_selection_caret_scroll` |
| PANEL-037 | 보고 있던 폴더가 사라졌을 때 | 재열람 실패 시 **최근접 존재 조상**으로 이동(클라우드 · 가상 루트는 비대상) | `panel.rs:1404-1420` | 없음 | N | `panel.rs::reopen_falls_back_to_nearest_ancestor_when_folder_vanishes` |
| PANEL-038 | 클라우드 · 내 PC 탭 전체 재열기 | 비활성 탭 포함 순회 재열람 후 활성 인덱스 · 크롬 원복 | `panel.rs:1376-1392` | 없음 | N | — |
| PANEL-039 | 낡은 탭 수렴 | `active_tab_stale()` = `stale` 또는 마지막 열거 필터 ≠ 현재 값. 호스트 `update_status` 길목이 참이면 재열람. 재열람 **시도**만으로 스탬프(실패 재시도 폭주 방지) | `panel.rs:1074-1077` · `1398-1402` · `win.rs:4970-4973` | 없음 | N | `panel.rs::tab_activation_marks_stale_and_reopen_clears` |
| PANEL-040 | 펼침 상태 영속(탭별 집합) | 키 = 소문자 + 후행 구분자 제거, 값 = 원 표기(BTreeMap = 부모 우선). 소스 교체 직전 `sync_expanded`(가시 폴더: 펼침 = 등재 · 접힘 = 말소 · 비가시 엔트리 보존) → 새 소스에 재적용. **방문 ≠ 확장**(더블클릭 진입은 등재하지 않음) | `panel.rs:62-66` · `763-790` · `792-832` | 경로 대소문자 무시 가정 | P | `panel.rs::navigation_carries_expansion_down_and_up` · `sibling_expansion_survives_enter_and_return` |
| PANEL-041 | 이름변경을 펼침 집합에 반영 | 폴더 자신 + 하위 접두사(`\` · `/`) 치환 | `panel.rs:902-926` | 구분자 2종 처리 | N | — |
| PANEL-042 | 감시 대상 폴더 목록 | `watch_dirs(cap)`: 루트 + 가시 펼침 폴더 + 뷰포트의 접힌 폴더(상한 공유, 호스트 `WATCH_CAP = 64`). `viewport_dirs(expanded_only)`는 트리 보기 전용 | `panel.rs:837-890` · `win.rs:3545` | watcher 자체는 OS별 | N | `panel.rs::viewport_dirs_lists_visible_tree_dirs` |
| PANEL-043 | 삭제 낙관 반영 | `hide_paths`: 활성 탭에서 대상 행을 즉시 표시 제외(FS 무변, 실패 시 재로드가 원복) | `panel.rs:894-899` | 없음 | N | `nexa-tree/src/lib.rs::remove_paths_hides_node_and_descendants` |
| PANEL-044 | 내 PC 전용 컬럼 전환 | 가상 최상위 진입 시 컬럼을 이름 · 종류 · 전체 크기(110) · 여유 공간(130)으로 교체, 이탈 시 기본 컬럼 복귀. **전환 시점에만** 교체(사용자 폭 리셋 방지) | `panel.rs:804-823` · `1287-1303` | 드라이브 용량 | P | — |

### 1-D. 보기 옵션 · 정렬 옵션 전파 (`panel.rs`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-045 | 탭별 보기 옵션(숨김 · Dot · 폴더 우선) | 값의 원천은 항상 탭. 호스트 ctx는 tz · 빈 폴더 글리프만 기여(`tab_ctx`). `set_view_filters(all_tabs, …)` · `set_folders_first`(전 탭) · `set_active_folders_first`(활성 탭). 설정 `view_scope`(tab/panel/global, 기본 panel)는 전파 폭만 결정 | `panel.rs:929-942` · `1031-1070` · `win.rs:5123-5156` | 숨김 속성 의미 | P | `panel.rs::per_tab_view_values_inherit_and_stale` |
| PANEL-046 | 보기 옵션 세션 플래그 | 탭별 `u8`: bit0 숨김 · bit1 Dot · bit2 폴더 우선. 부족분 = 현재 값 유지 | `panel.rs:1081-1104` | 없음 | N | `panel.rs::per_tab_view_values_inherit_and_stale` |
| PANEL-047 | 대소문자 구분 정렬 토글 | 전 탭 소스에 전파 · 즉시 재정렬 · 패널에 보관(기본 false) | `panel.rs:1166-1172` | 없음 | N | `panel.rs::sort_opts_propagate_to_new_sources` |
| PANEL-048 | 새 소스에 옵션 재적용 | 탐색 · 재로드 · 새 탭마다 폴더 우선 · 대소문자 · 타입어헤드 범위/옵션 · 폰트 장식 재적용 | `panel.rs:1189-1206` | 없음 | N | `panel.rs::sort_opts_propagate_to_new_sources` |
| PANEL-049 | 빈 폴더 글리프 억제 전파 | 전 탭 즉시. 새 소스는 `open_source` 관문이 `NavCtx.hide_empty_glyph`로 적용(설정 기본 true) | `panel.rs:1181-1186` · `config.rs:287` | 없음 | N | `source.rs::hide_empty_markers_suppresses_by_current_filter` |
| PANEL-050 | 폰트 장식 전파 | (폴더 굵게, 헤더 굵게, 헤더 이탤릭) 전 탭 + 보관(기본 전부 false) | `panel.rs:1209-1221` | 없음 | A | — |
| PANEL-051 | 보기 모드(트리 · 일반 · 타일) — 탭별 | 활성 탭에만 적용. 트리 이탈 시 `collapse_all`로 평탄화. 세션 문자열 `tree`/`flat`/`tiles`, 복원 시 부족분 = 설정 `view_mode` | `panel.rs:1226-1283` | 없음 | N | `panel.rs::view_mode_propagates_and_flattens` · `view_mode_is_per_tab` |
| PANEL-052 | 타입어헤드 옵션 전파 | (범위, 리셋 ms, 특수문자, 공백, Backspace, HUD 위치) 전 탭 + 보관. 기본 = (VisibleStream, 1000, true, true, true, 6) | `panel.rs:163-170` · `1307-1323` | 없음 | N | — |
| PANEL-053 | 컬럼 구성의 패널 단위 상속 | 폭(`apply_col_widths`) · 순서/표시(`apply_columns`) · 레이아웃(`apply_col_layout`: 현재 폭 보존, 재표시 컬럼 = 기본 폭, 전부 숨김이면 첫 컬럼 강제 표시)을 패널 전 탭에 적용. 리사이즈/재배열 발생은 호스트가 폴링 | `panel.rs:302-363` · `win.rs:6921-6940` | 없음 | N | — |

### 1-E. 데이터 어댑터 `TreeSource` (`source.rs`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-054 | 컬럼 key 상수 | `COL_NAME=0`(트리 컬럼 관례) · `EXT=1` · `SIZE=2` · `MODIFIED=3` · `KIND=4` · `TOTAL=5` · `FREE=6` | `nexa-dir2/crates/nexa-app/src/source.rs:11-18` | 없음 | N | — |
| PANEL-055 | 행 투영 | `RowItem{text, is_dir, depth, marker}`. 마커: 폴더면 Expanded/Collapsed, 빈 폴더 억제가 켜지고 가시 자식이 없으면 None(단 `is_dir`은 유지). 범위 밖 = 빈 행 | `source.rs:249-277` | 없음 | N | `source.rs::rows_project_marker_and_depth` |
| PANEL-056 | 바로가기 확장자 숨김 | 이름이 `.lnk`로 끝나면(대소문자 무시, 길이 > 4) 표시명에서 제거. 확장자 컬럼에는 유지. 실제 리네임 시 호스트가 `.lnk` 복원 | `source.rs:458-465` · `win.rs:4036-4046` | `.lnk`는 Windows 관례 | P | `source.rs::display_name_hides_lnk_only` |
| PANEL-057 | 셀 값 | 확장자(폴더 = 빈 값) · 크기(폴더 = 빈 값) · 수정한 날짜(0 = 빈 값) · 종류(드라이브/폴더/링크/파일/`{EXT} file` — 페인트 시점 `tr()` 조회) · 전체/여유(드라이브만) | `source.rs:279-331` | 드라이브 판정 = 이름이 `:\`로 끝남(`source.rs:317`) | P | `source.rs::cells_project_ext_size_kind_and_dirs_are_blank` |
| PANEL-058 | 크기 표기 | `< 1024` = `N B`, 이후 KB~PB, 100 미만 소수 1자리 · 100 이상 정수 | `source.rs:477-493` | 없음 | N | `source.rs::human_size_units_and_precision` |
| PANEL-059 | 날짜 표기 | Unix ms + tz 오프셋(분) → `yyyy-MM-dd HH:mm`. 음수 = 빈 값. tz 오프셋은 호스트가 주입(소스 생성 시 고정) | `source.rs:496-524` · `win.rs:1368-` | tz 취득 = `GetTimeZoneInformation` | P | `source.rs::fmt_datetime_applies_offset_and_epoch_math` |
| PANEL-060 | 펼침/접힘 토글 | 펼침 = `tree.expand`(빈 폴더도 마커 갱신 위해 true), 접힘 = 제거 행 > 0일 때 true, 접근 불가 폴더 = 조용히 false | `source.rs:333-346` | 없음 | N | `source.rs::toggle_file_is_noop_and_empty_dir_still_repaints` |
| PANEL-061 | 선택 위임 | Single/Toggle/RangeTo → 트리 선택 모델. `select_span`(러버밴드) = Single(lo) + range(hi). `clear_selection`은 비어 있으면 false | `source.rs:350-399` | 없음 | N | `source.rs::cross_folder_selection_via_widget_ops` |
| PANEL-062 | 잘라내기 대기 행 흐림 | 클립보드 잘라내기 표시 집합에 든 경로면 이름을 `text_dim`으로. 집합이 비면 경로 조회 생략(핫패스) | `source.rs:126-145` · `356-364` | `clipboard.rs` 미러가 Windows 전용(비Windows는 항상 false) | P | `rows.rs::ghosted_row_paints_name_dim` |
| PANEL-063 | 타일 보조 줄 | 드라이브 = `tr("drive.freeOf")` + 사용률(0~1), 그 외 = 종류 문자열 | `source.rs:406-424` | 드라이브 용량 | P | — |
| PANEL-064 | 아이콘 키 | `(키, 경로)` 반환. 키: 폴더 = `dir`, 확장자 없음 = `file`, 파일별 아이콘 확장자(`.exe .lnk .ico .cur .msi .scr .appref-ms`) = 소문자 전체 경로, 그 외 = 확장자. 타일은 `L|` 접두(라지) | `source.rs:426-432` · `nexa-dir2/crates/nexa-app/src/icons.rs:8-36` | 로더 = `SHGetFileInfoW`(`icons.rs:145-169`) | P | — |
| PANEL-065 | 정렬 키 매핑 | 컬럼 key → `SortKey`(Name/Ext/Size/Modified/Kind). `TOTAL`/`FREE`는 무시. 빈 목록 = 열거 순서 | `source.rs:434-453` | 없음 | N | `source.rs::set_sort_reorders_visible_rows` |
| PANEL-066 | 드라이브 용량 조회 | 가상 최상위 소스 생성 시 1회, 행 이름 → (전체, 여유) | `source.rs:102-124` · `161-170` | `GetDiskFreeSpaceExW` | P | — |
| PANEL-067 | 빈 폴더 판정 프로브 | 열거 완료 폴더 = 트리 실측, 미열거 = `read_dir` 조기 종료 프로브 1회 후 `NodeId` 캐시. 열 수 없으면 true(글리프 유지). UNC · 원격 · 광학 드라이브는 프로브 생략 | `source.rs:46-100` · `198-215` | `GetDriveTypeW` · `FILE_ATTRIBUTE_HIDDEN` | P | `source.rs::hide_empty_markers_uses_loaded_children_after_expand` |
| PANEL-068 | 타입어헤드 검색 위임 | `tree.find_prefix(caret, prefix, find_scope)` | `source.rs:401-404` | 없음 | N | `source.rs::find_prefix_delegates_visible_stream` |

### 1-F. 목록 위젯 `VirtualRows` (`rows.rs` · `columns.rs` · `typeahead.rs`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-069 | 가상화 페인트 | 가시 행만 그림: `first = scroll_row`, `count = visible_rows()`(부분 행 포함). 프레임 비용은 높이에만 비례. 마지막 행 아래 잔여는 `panel_bg` | `nexa-dir2/crates/nexa-gui/src/widgets/rows.rs:2127-2245` | 없음 | A | `rows.rs::paint_draws_header_cells_and_right_aligned_size` |
| PANEL-070 | 행 배경 규칙 | 선택(포커스 = `sel_bg`, 비포커스 = `sel_bg_inactive`) > 교대 음영(짝수 = `panel_bg`, 홀수 = `panel_bg_alt`) | `rows.rs:2146-2156` | 없음 | A | — |
| PANEL-071 | 트리 셀(마커 + 들여쓰기 + 아이콘 + 이름) | 들여쓰기 = `pad_x + depth × indent_w`, 마커 폭 = `indent_w`, 아이콘 크기 = `indent_w`(세로 중앙), 이름 x = 아이콘 뒤 `pad_x/2`. 텍스트 y = 행 높이의 4/5를 글자 높이로 본 중앙 | `rows.rs:1360-1401` | 아이콘 그리기 = 백엔드 `draw_icon` | A | `rows.rs::paint_draws_header_cells_and_right_aligned_size` |
| PANEL-072 | 펼침 마커 글리프 | 접힘 `U+E76C`(ChevronRight) · 펼침 `U+E70D`(ChevronDown) — 색 `text_dim` | `rows.rs:33-53` · `1376-1379` | Segoe MDL2 Assets 폰트 | A | — |
| PANEL-073 | 일반(Flat) 보기 | 마커 숨김 · 마커 존 없음 · ←/→ 무동작 · DnD 호버 펼침 비대상 | `rows.rs:1285-1287` · `1862-1863` · `2169-2171` · `1228-1232` | 없음 | N | `rows.rs::flat_mode_blocks_expansion` |
| PANEL-074 | 일반 셀 정렬 | Left = `cell.x + pad_x`, Right = `cell.right() - pad_x - 텍스트 폭`(셀 좌측 하한) | `rows.rs:2190-2198` · `nexa-dir2/crates/nexa-gui/src/columns.rs:4-8` | 없음 | A | `rows.rs::paint_draws_header_cells_and_right_aligned_size` |
| PANEL-075 | 컬럼 오른쪽은 빈 본문 | 마지막 컬럼 오른쪽은 행이 아님: 선택 하이라이트 제외, 클릭 = 선택 해제 + 러버밴드 시작 | `rows.rs:1026-1032` · `1188-1207` · `2201-2213` | 없음 | N | `rows.rs::tiles_grid_geometry_and_keys`(타일 판) |
| PANEL-076 | 캐럿 테두리 | 캐럿 행에 1px 사각 테두리(포커스 = `accent`, 비포커스 = `text_dim`), 폭 = 컬럼 총폭. 선택과 독립 | `rows.rs:2216-2230` | 없음 | A | — |
| PANEL-077 | 컬럼 헤더 | 높이 = `row_h`(컬럼 없음 또는 타일 = 0). 라벨 = `▲ `/`▼ ` + 제목 + ` ①`(정렬 순번은 단일 정렬부터 상시, ⑨에서 포화). 셀 오른쪽 1px `border` 구분선. 본문 뒤에 그려 스크롤과 무관하게 고정 | `rows.rs:960-966` · `1542-1553` · `2247-2280` · `columns.rs:43-46` | 없음 | A | `rows.rs::header_label_shows_arrow_before_and_badge_after` · `header_shifts_body_rows_down` · `columns.rs::badge_covers_first_nine` |
| PANEL-078 | 헤더 클릭 정렬 | 단순 클릭 = 단일 정렬로 리셋 + 3상태 순환(없음 → ▲ → ▼ → 없음). Shift+클릭(기존 정렬 ≥ 1) = 키 추가(오름) → 내림 → 제거. 정렬은 **MouseUp 무드래그**에서 확정. `sortable=false` 컬럼 제외 | `rows.rs:1324-1353` · `2081-2095` | 없음 | N | `rows.rs::header_click_cycles_three_states` · `plain_click_resets_to_single_sort` · `shift_click_adds_cycles_and_removes_keys` |
| PANEL-079 | 컬럼 폭 드래그 리사이즈 | 핸들 = 컬럼 오른쪽 경계 `[right-6, right+2)`(핸들 판정이 라벨보다 우선). **단독 조절**: 해당 컬럼만 변하고 총폭 가변(초과분 가로 스크롤), `min_width`(기본 40) 하한. 커서 변경용 `resize_hot` | `rows.rs:16-19` · `175-183` · `410-412` · `1300-1320` · `1931-1938` · `2002-2012` | 커서 변경은 호스트(`win.rs:7840`) | A | `rows.rs::drag_handle_resizes_column_with_min_width` |
| PANEL-080 | 헤더 드래그로 컬럼 재배열 | 5px 임계 초과 시 활성. **라이브 미리보기**(가장 가까운 경계로 즉시 재배열) + 커서 추종 고스트 헤더(헤더 셀 모양 + 1px 테두리, x는 위젯 안으로 클램프). MouseUp = 확정(`col_reordered`), ESC = 시작 순서 복원 | `rows.rs:160-173` · `1404-1444` · `1940-1951` · `2013-2031` · `2284-2308` · `win.rs:8797-8799` | 없음 | A | — (재배열 후 기하: `rows.rs::rename_field_follows_tree_column_after_reorder`) |
| PANEL-081 | 컬럼 경계 더블클릭 = 자동 맞춤 | 대상 = 핸들 위 · `resizable` 컬럼 · 타일 제외. 가시 행 + 헤더 텍스트와 부가 폭(트리 셀은 들여쓰기 · 아이콘 포함)을 위젯이 제공, 실측은 호스트. 상한 = 설정 `col_autofit_max`(기본 400px@96dpi) | `rows.rs:1480-1537` · `win.rs:6846-6866` · `win.rs:8669-8674` | 텍스트 실측은 백엔드 | A | — |
| PANEL-082 | 컬럼 폭 일괄 적용 · 변경 통지 | `set_col_widths`(순서 대응, `min_width` 클램프) · `take_col_resized` · `take_col_reordered`(1회성 플래그) | `rows.rs:1446-1470` | 없음 | N | — |
| PANEL-083 | 컬럼 모델 | `Column{key, title, width, min_width=40, align=Left, sortable=true, resizable=true}` + `right_aligned()`. 위젯은 컬럼 의미를 모르고 key로 소스에 위임 | `columns.rs:10-40` | 없음 | N | — |
| PANEL-084 | 가로 스크롤 | `HWheel`(Shift+휠 포함): 항상 픽셀, 노치당 `wheel_lines × 16px`, 고속 배수 적용. `scroll_x`는 `[0, 총폭 - 위젯폭]` 클램프, 위젯이 넓어지면 재클램프 | `rows.rs:14-16` · `804-812` · `1056-1059` · `1791-1803` | 없음 | N | `rows.rs::hwheel_scrolls_and_clamps_to_total_width` · `widening_bounds_reclamps_scroll_x` |
| PANEL-085 | 세로 휠 스크롤(마우스 노치) | `|delta| ≥ 120`: 행 단위 = `WheelAccum`(시스템 줄 수) × 고속 배수. 상한 = 전체 그리드 행 − 완전 가시 행 | `rows.rs:1041-1069` · `1769-1776` · `nexa-dir2/crates/nexa-gui/src/event.rs:6-21` · `event.rs:86-101` | 줄 수 = `SPI_GETWHEELSCROLLLINES`(`win.rs:6325-6342`) | P | `rows.rs::scroll_clamps_to_total_minus_full_rows` |
| PANEL-086 | 픽셀 스크롤(정밀 터치패드) | `|delta| < 120`: 픽셀 누적(`wheel_lines × 행 높이` / 노치) → `scroll_by_px`. 부분 행 오프셋 `scroll_frac`(0..행 높이), 행 단위 조작(키 · 노치 · 가시화)은 0으로 스냅. 히트 · 페인트는 `row_origin = body_top - scroll_frac` 기준 | `rows.rs:219-223` · `976-994` · `1777-1785` | 터치패드 delta 규격 | P | `rows.rs::fast_scroll_multiplies_rapid_wheel_notches_but_not_trackpad` |
| PANEL-087 | 고속 스크롤 + `×N` 배지 | 같은 방향 사건이 `window_ms`(160) 안에 이어지면 배수 = `1 + 연속/step`(상한 max). 파일 그리드는 한 단계 더 빠름(step−1, max×2). 노치 미만 delta는 배수 1. ↑/↓ 자동 반복에도 적용. 배지 = 캡슐 `×N`, 유지 250ms 후 600ms 페이드 | `rows.rs:230-231` · `301` · `1773` · `1794` · `1811-1819` · `2365-2366` · `nexa-dir2/crates/nexa-gui/src/fastscroll.rs:24-112` · `286-372` | 없음 | A | `rows.rs::fast_scroll_multiplies_rapid_wheel_notches_but_not_trackpad` · `fastscroll.rs::factor_grows_every_step_within_window_and_caps` |
| PANEL-088 | 오버레이 스크롤바(세로 + 가로) | 평소 숨김 → 스크롤 순간 그 축만 반투명 표시 → 유지 → 단계 페이드. 썸 호버/드래그 = 두꺼운 바 + 페이드 보류. 썸 드래그 = 비례 스크롤, 트랙 클릭 = 페이지 이동(표시 중인 축만). 호버는 **보이는 축**에만 반응. 다른 입력 처리보다 우선 | `rows.rs:20-29` · `264-285` · `674-903` · `1714-1728` | 없음 | A | — |
| PANEL-089 | 주기 틱 | 타입어헤드 타임아웃 소거 + 속도 배지 페이드 + 스크롤바 유지/페이드. 필요할 때만 `inv.request_tick()`으로 다음 틱 요청 | `rows.rs:650-672` · `nexa-dir2/crates/nexa-gui/src/widget.rs:19-33` · `win.rs:9493-9535` | 타이머(`SetTimer` 40ms · 250ms) | P | `rows.rs::typeahead_times_out_via_tick_and_space_is_excluded` |
| PANEL-090 | 클릭 선택 | 무수정 = 단일 · Ctrl = 비연속 토글 · Shift = anchor~행 범위. 캐럿은 클릭 행으로 | `rows.rs:67-76` · `1952-1988` | Ctrl 키(mac은 ⌘ 관례) | P | `rows.rs::click_selects_single_ctrl_toggles_shift_ranges` |
| PANEL-091 | 기선택 행 프레스 = 선택 유지 | 다중 선택 드래그(DnD)를 위해 프레스 시점에는 선택을 유지하고 `press_pending`에 보류, MouseUp(무드래그)에서 단일 선택으로 붕괴. 새 프레스(좌/우)는 이전 보류를 폐기. 호스트가 드래그에서 돌아오면 `abort_press()`(선택 불변). 소스 교체 시에도 리셋 | `rows.rs:255-257` · `1156-1163` · `1705-1713` · `1960-1965` · `2096-2103` · `win.rs:8361-8369` | OLE DnD가 MouseUp을 소비 | P | `rows.rs::press_pending_does_not_survive_replace_source` · `new_press_discards_stale_pending_without_replace_source` · `abort_press_clears_pending_and_keeps_selection` |
| PANEL-092 | 마커 클릭 = 펼침/접힘 | 마커 존 = 트리 열 안 `[indent, indent + indent_w)`(열 폭으로 클램프), 마커 있는 행 · 트리 보기만. 선택과 분리. 호스트는 `marker_hit`으로 더블클릭 진입과 구분 | `rows.rs:1277-1297` · `1953-1958` | 없음 | N | `rows.rs::marker_click_toggles_row_without_columns` · `marker_zone_follows_tree_column_after_reorder` |
| PANEL-093 | 러버밴드(드래그 선택) | 시작: 빈 본문 프레스(선택 해제) 또는 **미선택 행** 무수정 프레스. 4px 미만 이동은 클릭 지터로 무시. 밴드 세로 범위와 교차하는 가시 행 범위로 선택 **대체**(`select_span`), 교차 없으면 해제. 외곽선 1px `accent`. MouseUp에서 종료 | `rows.rs:196-211` · `1976-1999` · `2032-2077` · `2312-2321` | 없음 | A | `rows.rs::rubber_band_selects_intersecting_rows_and_ends_on_up` |
| PANEL-094 | 우클릭 선택 규약 | 미선택 행 = 단독 선택, 선택된 행 = 선택 유지, 캐럿 이동. 빈 본문 = 선택 해제. 마커 존은 제외. 메뉴 표시는 호스트 | `rows.rs:2108-2123` | 컨텍스트 메뉴는 OS별 | N | `rows.rs::right_down_selects_unselected_keeps_selection_clears_on_empty` |
| PANEL-095 | 전체 선택(Ctrl+A) | `InputEvent::SelectAll` → 현재 가시 노드 전체 | `rows.rs:1925-1929` | 없음 | N | `rows.rs::ctrl_a_selects_all_visible` |
| PANEL-096 | 키보드 캐럿 이동 | ↑/↓/PageUp/PageDown/Home/End. 평이동 = 단일 선택 · Shift = 범위 · Ctrl = 캐럿만. 캐럿 없으면 `scroll_row` 기준. 캐럿이 보이도록 스크롤 추적 + 세로 바 표시. 빈 목록 = 무동작 | `rows.rs:1071-1108` · `1804-1860` | 없음 | N | `rows.rs::caret_moves_select_and_scroll_follows` · `shift_moves_range_and_ctrl_moves_caret_only` |
| PANEL-097 | ←/→ 트리 조작 | → : 접힘 = 펼침, 펼침 = 첫 자식으로. ← : 펼침 = 접힘, 그 외 = 부모 행으로(더 얕은 깊이의 직전 행) | `rows.rs:1110-1117` · `1864-1893` | 없음 | N | `rows.rs::right_left_keys_toggle_expansion_at_caret` |
| PANEL-098 | Space = 캐럿 행 선택 토글 | 타입어헤드 접두사 입력 중이고 공백 포함 옵션이 켜져 있으면 토글하지 않음(문자 경로로 처리) | `rows.rs:1894-1902` · `win.rs:9388-9395` | 없음 | N | `rows.rs::shift_moves_range_and_ctrl_moves_caret_only` |
| PANEL-099 | 타일 보기 — 기하 · 키 | 타일 = `row_h×12` × `row_h×3`, 열 수 = `위젯폭 / 타일폭`(최소 1). ↑/↓ = ±열 수(고속 배수 적용), ←/→ = ∓1, PageUp/Down = `page × 열 수`, Home/End. 헤더 없음. 마지막 열 오른쪽 잔여 = 빈 본문 | `rows.rs:363-406` · `1192-1201` · `1820-1845` | 없음 | A | `rows.rs::tiles_grid_geometry_and_keys` |
| PANEL-100 | 타일 보기 — 페인트 | 셀(안쪽 2px 여백) + 라지 아이콘(`row_h×2−4`, 키 `L|…`) + 이름 / 용량 바(있으면) / 보조 줄. 용량 바: 트랙 `header_bg`, 사용분 `accent`, 90% 초과 = `0xD33F3F`. 잘라내기 = 이름 흐림. 폴더 굵게 적용. 캐럿 1px 테두리 · 러버밴드 · 타입어헤드 HUD 공유 | `rows.rs:1555-1687` | 라지 아이콘 로더 | A | — |
| PANEL-101 | 타일 러버밴드 | 밴드 사각형과 **교차하는 타일만** 선택(해제 후 Toggle로 재구성) | `rows.rs:2042-2058` | 없음 | N | — |
| PANEL-102 | 타입어헤드(입력으로 찾기) | 버퍼 누적 · 타임아웃(기본 1000ms) 후 새 접두사 · **반복 단일키 = 다음 매치로 cycle** · Backspace = 축소(옵션) · 공백 = 접두사 입력 중일 때만 문자(옵션) · 특수문자 허용(옵션, 끄면 영숫자만). 매치 시 단일 선택 + 캐럿 + 스크롤. 매치 없어도 HUD 갱신 | `nexa-dir2/crates/nexa-gui/src/typeahead.rs:5-99` · `rows.rs:1119-1133` · `1905-1924` | 문자 입력(IME) 경로 | P | `typeahead.rs`의 4건 · `rows.rs::typeahead_jumps_cycles_and_accumulates` · `typeahead_times_out_via_tick_and_space_is_excluded` |
| PANEL-103 | 타입어헤드 HUD | 라벨 `찾기: {버퍼}`(**한국어 하드코딩**), 높이 = `row_h`, 3×3 위치(설정 `typeahead_pos` 0..8, 기본 6 = 좌하), 배경 `header_bg` + 1px `accent` 테두리 | `rows.rs:1657-1686` · `2333-2363` | 없음 | A | — |
| PANEL-104 | 인라인 이름변경 시작(F2 · 느린 재클릭) | 캐럿을 그 행으로 + 가시화. 초기 선택: 파일 = 마지막 `.` 앞(이름부), 폴더 = 전체. 타일 보기 · 트리 열 숨김 · 범위 밖 행은 거부 | `rows.rs:439-465` · `win.rs:4005-4016` | 없음 | A | `rows.rs::inline_rename_flow_and_key_block` |
| PANEL-105 | 이름변경 편집 | 문자 입력(Backspace = `\u{8}`) · 편집 키(←→ Home End Shift 선택 Ctrl+A Delete) · 복사/잘라내기/붙여넣기 · 실행 취소(단일 단계) · 선택 삭제 · 메뉴 상태(실행 취소 가능, 선택 있음, 비어 있음). 편집 중 목록 키 네비 · 타입어헤드 차단 | `rows.rs:467-538` · `1729-1737` · `win.rs:8772-8791` | 클립보드 | P | `rows.rs::inline_rename_flow_and_key_block` |
| PANEL-106 | 이름변경 필드 기하 · IME 배치 | 필드 = 트리 열의 이름 위치에서 3px 왼쪽 확장(이름 x 밀림 없음), 폭 = 열 끝까지(최소 `indent_w×3`). 현재 열 순서 · 가로 스크롤 반영. `rename_edit_info` = (캐럿 앞 텍스트, 필드 rect, pad) | `rows.rs:147-149` · `552-579` · `1011-1022` · `win.rs:4793` | IME 조합 창 위치 | P | `rows.rs::rename_field_follows_tree_column_after_reorder` |
| PANEL-107 | 이름변경 중 마우스 | 필드 안 클릭 = 캐럿 배치 · 드래그 = 텍스트 선택 · 필드 밖 클릭 = 취소 후 정상 처리. 필드 안 더블클릭 = 전체 선택(행 활성화 아님) | `rows.rs:1738-1763` · `win.rs:8628-8637` | 없음 | A | — |
| PANEL-108 | 이름변경 확정 · 취소 | Enter = `submit_rename` → (행, 새 이름) 반환(실제 rename은 호스트) · Esc/외부 클릭 = 취소 | `rows.rs:545-550` · `616-623` · `win.rs:8774-8783` | 없음 | N | `rows.rs::inline_rename_flow_and_key_block` |
| PANEL-109 | 느린 재클릭 리네임 | 같은 패널 · 같은 경로의 기선택 행을 1000ms 이상 간격으로 재클릭(무수정, 리네임 중 아님) → MouseUp 무드래그에서 **더블클릭 시간만큼 지연** 후 진입. 만료 시 활성 패널 · 캐럿 경로가 예약과 일치할 때만. 새 클릭 · 우클릭 · 키 · 명령 · 더블클릭이 예약을 폐기 | `win.rs:51` · `3999-4003` · `8132-8150` · `8571-8581` · `9584-9602` | `GetDoubleClickTime` · `SetTimer` | P | — |
| PANEL-110 | 프로그램적 선택 | `select_program`(캐럿 동반 + 가시화) · `select_program_aligned`(상/중/하 배치). 범위 밖 인덱스 무시 | `rows.rs:151-158` · `581-614` | UIA(접근성) 연동 | N | `rows.rs::select_program_moves_caret_and_ignores_out_of_range` |
| PANEL-111 | 소스 교체(탐색) | 스크롤 · 캐럿 · 밴드 · 프레스 보류 · 타입어헤드 리셋. 컬럼 유지. 정렬은 **명시된 경우에만** 새 소스에 재적용 | `rows.rs:1165-1184` | 없음 | N | `rows.rs::replace_source_resets_view_but_keeps_sort` |
| PANEL-112 | 재로드 후 뷰 복원 | `restore_view(caret, scroll_row, scroll_x)` — 범위 밖 클램프 | `rows.rs:1140-1154` | 없음 | N | `panel.rs::reopen_preserves_expanded_selection_caret_scroll` |
| PANEL-113 | 히트 테스트 API | `row_at`(헤더 · 빈 본문 = None) · `row_anchor`(완전 가시 행의 앵커 — 키보드 컨텍스트 메뉴 위치) · `in_body` · `marker_hit` · `header_area` · `rename_hit`/`rename_field_hit` | `rows.rs:433-437` · `540-543` · `1186-1207` · `1249-1281` · `1472-1478` | 없음 | N | `rows.rs::header_shifts_body_rows_down` |
| PANEL-114 | DnD 보조 | `drag_scroll_edge`(본문 상/하단 한 행 높이 이내에서 1행 스크롤) · `is_collapsed_dir` · `hover_expand`(접힌 폴더만 펼침, 접기 없음). 호스트 폴링 100ms, 호버 대기 = 설정 `dnd_hover_ms`(기본 3000) | `rows.rs:1209-1247` · `win.rs:3343-3408` | DnD 자체는 OS별 | N | `rows.rs::drag_scroll_edge_moves_one_row_within_edge_zone` · `hover_expand_only_collapsed_dirs_in_tree_mode` |
| PANEL-115 | 뷰포트 행 구간 | `viewport()` = `[scroll_row, scroll_row + visible_rows)`(소스 길이로 클램프) | `rows.rs:913-918` | 없음 | N | `panel.rs::viewport_dirs_lists_visible_tree_dirs` |
| PANEL-116 | 보기 모드 전환(위젯) | 밴드 정리 · 타일 진입 시 이름변경 취소 · 스크롤 클램프 · 캐럿 가시화 | `rows.rs:185-194` · `342-361` | 없음 | N | `rows.rs::tiles_grid_geometry_and_keys` |
| PANEL-117 | 폰트 장식 | 폴더 이름 굵게(리스트는 트리 보기 한정 · 타일은 모드 무관) · 헤더 굵게/이탤릭. 목록 폰트 슬롯 = `FontSlot::List` | `rows.rs:327-340` · `1390-1399` · `1584` · `2128` · `2249-2250` | 없음 | A | — |
| PANEL-118 | 지표 갱신 | `set_metrics(row_h, pad_x, indent_w)` — 하한 1, 스크롤 재클램프 | `rows.rs:944-955` | 없음 | N | — |

### 1-G. 트리 코어 `nexa-tree` (API와 UI 접점)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-119 | 트리 열기 | `Tree::open`(모두 표시) · `open_filtered(show_hidden, show_dotfiles)` — 최상위(depth 0)만 열거, 펼침 없음 | `nexa-dir2/crates/nexa-tree/src/lib.rs:181-213` | 없음 | N | `lib.rs::open_lists_top_level_folders_first` · `open_missing_path_errors` |
| PANEL-120 | 열거 3경로 | 가상 최상위 = 드라이브 + 추가 루트(클라우드 연결) · 클라우드 센티널 = 등록 콜백 캐시 · 로컬 = `read_dir_entries`(엔트리 단위 오류 격리). 링크형(`target`)은 실경로 사용 | `lib.rs:227-286` | 드라이브 열거(`nexa-vfs/src/lib.rs:116-133` = `A:\`~`Z:\` 프로브) | P | `lib.rs::open_virtual_root_lists_drives`(`cfg(windows)`) |
| PANEL-121 | 가시성 필터 | Dot 끔 = 이름이 `.`으로 시작하면 제외 · 숨김 끔 = `attrs & 0x2`면 제외. 걸러진 항목은 트리에 생성하지 않음(펼침 시 자식도 동일) | `lib.rs:144-165` | 속성 비트는 Windows 전용(비Windows `attrs = 0`, `nexa-vfs/src/lib.rs:62-72`) | P | `lib.rs::open_filtered_excludes_dotfiles` |
| PANEL-122 | 정렬 | 폴더 우선 그룹핑(옵션) → 키 순차(각 asc/desc) → 이름 · id tie-break. 키: Name · Ext · Size(폴더 = 0) · Modified · Kind(폴더 → 파일 → 심링크, 2차 = 확장자) · None(열거 순서). `set_sort`는 로드된 전 폴더 자식 + 가시 목록 재구성, 펼침 보존. 기본 = 폴더 우선 + 이름 오름 | `lib.rs:93-131` · `288-383` · `798-805` | 없음 | N | `lib.rs`의 `sort_*` 8건 · `set_sort_preserves_expansion` · `set_sort_none_then_name_roundtrip` |
| PANEL-123 | 이름 비교 규칙 | 기본 = 소문자화 코드포인트 비교(**자연 정렬 아님**) · 대소문자 구분 = 원문 코드포인트 순(대문자 그룹 상단) | `lib.rs:771-788` | 없음 | N | `lib.rs::case_sensitive_groups_uppercase_first` |
| PANEL-124 | 타입어헤드 매칭 3범위 | VisibleStream(`caret+1`부터, wrap) · GlobalFirst(0부터, 캐럿 무시) · CurrentLevel(캐럿과 같은 부모 형제만). 대소문자 무시, 이름 starts-with, 할당 0 | `lib.rs:133-142` · `385-429` · `win.rs:6765-6771` | 없음 | N | `lib.rs::find_prefix_visible_stream_wrap_and_cycle` · `find_prefix_global_first_ignores_caret` · `find_prefix_current_level_siblings_only` |
| PANEL-125 | 펼침 · 접힘 | `expand`(최초 지연 열거, 접혔던 하위 펼침 복원, `RangeChange` 반환) · `collapse`(하위 펼침 상태 보존) · `expand_path`(가시 경로만) · `collapse_all` · `is_expanded` | `lib.rs:460-469` · `559-636` | 없음 | N | `lib.rs::expand_and_collapse_roundtrip` · `reexpand_restores_nested_expansion` · `expand_is_noop_on_file_or_twice` · `index_of_path_and_expand_path` |
| PANEL-126 | 가시 스트림 조회 | `visible_len` · `row`(클론) · `row_ref`(이름 빌림 — 셀 핫패스) · `visible_id` · `index_of` · `index_of_path`(후행 구분자 · **ASCII 대소문자 무시**) · `node_path` · `root_path` · `root_count` | `lib.rs:215-223` · `431-458` · `511-546` · `759-762` | 경로 대소문자 무시 가정 | P | `lib.rs::row_ref_matches_row_without_clone` · `index_of_path_and_expand_path` |
| PANEL-127 | 선택 모델(교차 폴더) | 삽입 순서 보존 집합 + anchor. `select(Single/Toggle)` · `select_range`(anchor 없거나 비가시면 단일) · `select_all_visible` · `clear_selection`(anchor 유지) · `selected_paths`/`selected_path(i)`/`selected_ids`/`selection_count`. 범위 밖 id 무시 | `lib.rs:84-91` · `651-757` | 없음 | N | `lib.rs::cross_folder_selection_ordered` · `range_and_select_all` |
| PANEL-128 | 경로 목록 표시 제외 | `remove_paths`: 매치 노드 + 하위를 roots/visible/children/선택에서 제거(arena 유지, anchor 정리). 경로 비교 = 소문자 + 후행 구분자 제거 | `lib.rs:471-509` | 경로 대소문자 무시 가정 | P | `lib.rs::remove_paths_hides_node_and_descendants` |
| PANEL-129 | 빈 폴더 판정 보조 | `loaded_child_count`(열거 완료 폴더만 Some) · `filter_flags` | `lib.rs:638-649` | 없음 | N | `source.rs::hide_empty_markers_uses_loaded_children_after_expand` |
| PANEL-130 | 스케일 보장 | 10만 노드에서 펼침 · 조회 · 선택 · 접힘 정상 완료(이차 폭주 없음) | `lib.rs:807-872` · `948-970` | 없음 | N | `lib.rs::large_tree_scale_ops_complete` · `bench_100k_visible`(`#[ignore]`) |

### 1-H. 호스트 접점(`win.rs` — 목록 동작에 필수인 배선)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| PANEL-131 | 기본 5컬럼 | 이름 340 · 확장자 64 · 크기 96(우측 정렬) · 수정한 날짜 140 · 종류 110(px@96dpi, DPI 스케일). 제목 = `col.*` i18n 키 | `win.rs:1357-1366` | DPI | N | — |
| PANEL-132 | 컬럼 좌우 동기 | 리사이즈/재배열 폴링 후 같은 패널 전 탭에 상속, 설정 `col_width_sync`(기본 true)면 반대 패널에도 적용 | `win.rs:6921-6940` | 없음 | N | — |
| PANEL-133 | 컬럼 레이아웃 문자열 | `cols[name:1,ext:1,…]`(표시 컬럼 = 표시 순, 숨김 = 정의 순 말미 `:0`). key 이름 ↔ id 매핑 | `win.rs:6868-6919` · `config.rs:1075` | 없음 | N | `config.rs`(세션 왕복 테스트, 1580~1622 부근) |
| PANEL-134 | 휠 라우팅 | **마우스 아래 패널**로(활성 패널 아님). Shift+휠 = 가로(`HWheel{-delta}`). 틸트 휠 = `HWheel{delta}` | `win.rs:7855-7958` | `WM_MOUSEWHEEL` · `WM_MOUSEHWHEEL` | P | — |
| PANEL-135 | 문자 입력 라우팅 | Ctrl 눌림 = 제외. 공백은 이름변경 중이거나 타입어헤드 접두사 입력 중일 때만 문자로 전달. 타입어헤드 활성 시 250ms 타이머 무장 | `win.rs:66-67` · `9383-9406` | `WM_CHAR` | P | — |
| PANEL-136 | 위젯 틱 타이머 | 40ms 간격, 재요청 없으면 해제(무효화 flush가 재무장) | `win.rs:123-127` · `9493-9519` | `SetTimer`/`KillTimer` | P | — |
| PANEL-137 | DnD 추적 | 커서 아래 패널의 엣지 스크롤 + 호버 대기(비활성 탭 우선, 다음 접힌 폴더). 발동 시 행-경로 일치 확인 후 펼침 | `win.rs:3343-3408` | OLE DnD | P | — |
| PANEL-138 | 단축키 배선 | §2-7 표 참조(Ctrl+T/W/A/C/X/V/Z/Y, Enter, F2, Delete, Apps, Shift+F10, Alt+방향키, Ctrl+H, Ctrl+., Ctrl+Shift+N/R) | `win.rs:8840-8930` | 가상 키 코드 | P | — |

**기능 수: 138.**

---

## 2. 화면 · 컨트롤 배치

좌표는 창 클라이언트 px. 아래 수치는 96dpi 기준이며 전부 `dpi/96`로 스케일된다(`win.rs:1345-1354`).

### 2-1. 패널 전체(세로 스택)

```
┌──────────────────────────────────────────────┐ y = bounds.y
│ 탭 바            h = tab_h(22) × 탭 줄 수      │
├────┬────┬────┬────┬───────────────────────────┤ y += tab_h
│ 홈 │ ← │ → │ ↑ │ 경로 바(브레드크럼/편집)     │ h = bar_h(24)
├────┴────┴────┴────┴───────────────────────────┤ y += bar_h
│ 컬럼 헤더        h = row_h(20)                 │
│ 본문 행 …        행 높이 row_h(20)             │
│                                 [진행 배지]    │
└──────────────────────────────────────────────┘ y = bounds.bottom()
```

- 네비 버튼: 4개 × 폭 `row_h + pad_x`(= 26) = 104, 틈 없음, 경로 바가 바로 이어 붙음(`panel.rs:392-405` · `1546-1548`).
- 실측 예(패널 400×400): 탭 바 `(0,0,400,22)` · 네비 `(0,22,104,24)` · 경로 바 `(104,22,296,24)` · 목록 `(0,46,400,354)`(`panel.rs:2115-2124`).
- 도크는 패널 bounds 밖의 전폭 밴드로 호스트가 배치한다(`panel.rs:406-409`).

### 2-2. 목록 — 리스트(트리/일반) 보기

| 요소 | 위치 · 크기 | 근거 |
|---|---|---|
| 헤더 행 | `y = bounds.y`, 높이 `row_h`, 배경 `header_bg`, 텍스트 x = 셀 x + `pad_x`, 셀 오른쪽 1px `border` | `rows.rs:2247-2280` |
| 본문 시작 | `body_top = bounds.y + header_h`, 첫 행 y = `body_top − scroll_frac` | `rows.rs:968-979` |
| 행 | 높이 `row_h`, 컬럼 x = `bounds.x − scroll_x + 앞 컬럼 폭 합` | `rows.rs:1005-1009` |
| 트리 셀 | 들여쓰기 x = 셀 x + `pad_x` + `depth × indent_w` → 마커 칸(폭 `indent_w`) → 아이콘(`indent_w` 정사각, 세로 중앙) → `pad_x/2` → 이름 | `rows.rs:1370-1400` |
| 텍스트 세로 위치 | `y + (h − h×4/5) / 2` | `rows.rs:1371` · `2158` |
| 일반 셀 | 좌측 정렬 x = 셀 x + `pad_x` · 우측 정렬 x = 셀 오른쪽 − `pad_x` − 텍스트 폭 | `rows.rs:2190-2198` |
| 캐럿 테두리 | 행 사각 1px, 폭 = 컬럼 총폭(위젯 폭으로 클램프) | `rows.rs:2218-2230` |
| 리사이즈 핸들 | 컬럼 오른쪽 경계 기준 `[−6, +2)` | `rows.rs:18-19` · `1305-1312` |
| 이름변경 필드 | x = 이름 x − 3, 폭 = 트리 열 끝까지 + 3(최소 `indent_w × 3`), 높이 `row_h` | `rows.rs:149` · `569-579` |
| 세로 스크롤 썸 | x = 오른쪽 − 폭 − 2, 폭 6(평소)/10(호버 · 드래그), 최소 길이 24, 본문 영역 안 | `rows.rs:23-25` · `677-694` |
| 가로 스크롤 썸 | y = 아래 − 높이 − 2, 높이 6/10, 최소 길이 24 | `rows.rs:697-710` |
| 스크롤 썸 색 | `theme.text` + 알파 120(평소)/210(호버 · 드래그), 드래그 중 트랙 알파 28, 모서리 = 두께/2 | `rows.rs:26-27` · `864-903` |
| 스크롤바 시간 | 유지 22틱(40ms 틱 ≈ 900ms) 후 틱당 알파 −24 | `rows.rs:28-29` · `651-672` |
| 타입어헤드 HUD | 폭 = 라벨 폭 + `pad_x×2`, 높이 `row_h`. 열: 좌 = x + `pad_x` · 중 = 가운데 · 우 = 오른쪽 − 폭 − `pad_x`. 행: 상 = `body_top + pad_x` · 중 = 세로 가운데 · 하 = 아래 − `row_h` − `pad_x` | `rows.rs:2335-2362` |
| 속도 배지 `×N` | 폭 = 글자 폭 + `pad_x×2`, 높이 `row_h`, 캡슐(모서리 h/2), 여백 `max(pad_x, 4)`, 기본 위치 2(우상), 배경 `accent` 40% × 세기 | `fastscroll.rs:234-277` |
| 진행 배지 | 높이 22, 안쪽 여백 좌우 10 · 상 5, 목록 우하단에서 12 안쪽, 배경 `sel_bg` + 1px `accent` 테두리 | `panel.rs:496-517` |

### 2-3. 목록 — 타일 보기

| 요소 | 위치 · 크기 | 근거 |
|---|---|---|
| 타일 | 폭 `row_h × 12`(240) · 높이 `row_h × 3`(60), 좌→우 · 위→아래 채움 | `rows.rs:365-406` |
| 셀 배경 | 타일 안쪽 2px 여백 | `rows.rs:1580` |
| 아이콘 | 변 = `row_h × 2 − 4`(36), x = 셀 x + `pad_x`, 세로 중앙 | `rows.rs:1563` · `1585-1589` |
| 텍스트 | x = 아이콘 오른쪽 + `pad_x`, 줄 높이 = `row_h × 4/5`, 줄 간격 2, 줄 묶음 세로 중앙 | `rows.rs:1591-1598` · `1612` |
| 용량 바 | 높이 = `max(줄 높이/2, 4)`, 이름 줄 아래 · 보조 줄 위 | `rows.rs:1613-1628` |
| 헤더 | 없음 | `rows.rs:960-963` |

### 2-4. 탭 순서 · 포커스

- 목록에는 OS 탭 순서가 없다. 키 포커스는 호스트가 **활성 패널**에 직접 라우팅한다(`panel.rs:3` · `1457`).
- 패널 전환 · 터미널 포커스 · 경로 바 편집 포커스는 호스트 상태(범위 밖).

### 2-5. 컬럼 기본값

| key | 제목 키 | 폭(px@96) | 정렬 | 비고 |
|---|---|---|---|---|
| 0 name | `col.name` | 340 | 좌 | 트리 컬럼 |
| 1 ext | `col.ext` | 64 | 좌 | |
| 2 size | `col.size` | 96 | 우 | |
| 3 modified | `col.modified` | 140 | 좌 | |
| 4 kind | `col.kind` | 110 | 좌 | |
| 5 total | `col.total` | 110 | 우 | 내 PC 전용 |
| 6 free | `col.free` | 130 | 우 | 내 PC 전용 |

근거: `win.rs:1357-1366` · `panel.rs:1287-1303`.

### 2-6. 마우스 동작 요약

| 위치 | 동작 | 결과 | 근거 |
|---|---|---|---|
| 헤더 라벨 | 클릭(무드래그) | 정렬 3상태 순환 | `rows.rs:2081-2095` |
| 헤더 라벨 | Shift+클릭 | 다중열 정렬 추가/순환/제거 | `rows.rs:1330-1348` |
| 헤더 라벨 | 드래그(> 5px) | 컬럼 재배열(라이브 미리보기) | `rows.rs:2013-2031` |
| 헤더 경계 | 드래그 | 컬럼 폭 조절 | `rows.rs:2002-2012` |
| 헤더 경계 | 더블클릭 | 자동 맞춤 | `win.rs:8669-8674` |
| 헤더 | 우클릭 | 컬럼 설정 팝업(호스트) | `rows.rs:1472-1478` · `win.rs:8172` |
| 마커 | 클릭 | 펼침/접힘 | `rows.rs:1953-1958` |
| 행 | 클릭 / Ctrl / Shift | 단일 / 토글 / 범위 | `rows.rs:1960-1975` |
| 선택된 행 | 드래그 | 파일 DnD(호스트) | `win.rs:8151-8152` · `8361-8369` |
| 미선택 행 · 빈 본문 | 드래그 | 러버밴드 | `rows.rs:1976-1999` |
| 행 | 더블클릭 | 폴더 진입 / 파일 실행 | `win.rs:8676-8686` |
| 선택된 행 | 느린 재클릭 | 이름변경 | `win.rs:8132-8150` |
| 행 / 빈 본문 | 우클릭 | 선택 규약 후 컨텍스트 메뉴(호스트) | `rows.rs:2108-2123` |
| 스크롤 썸 / 트랙 | 드래그 / 클릭 | 비례 스크롤 / 페이지 이동 | `rows.rs:815-851` |

### 2-7. 단축키(목록 · 패널 관련)

| 키 | 동작 | 근거 |
|---|---|---|
| ↑ ↓ PageUp PageDown Home End | 캐럿 이동(Shift = 범위 · Ctrl = 캐럿만) | `rows.rs:1846-1860` |
| → ← | 펼침·첫 자식 / 접힘·부모 | `rows.rs:1864-1893` |
| Space · Ctrl+Space | 캐럿 행 선택 토글 | `rows.rs:1895-1902` |
| 문자 · Backspace | 타입어헤드 | `rows.rs:1905-1924` |
| Ctrl+A | 전체 선택 | `win.rs:8846-8847` |
| Enter | 캐럿 행 활성화 | `win.rs:8864-8869` |
| Alt+↓ | 캐럿 행 활성화 | `win.rs:8922-8929` |
| Alt+← · Alt+→ · Alt+↑ | 뒤로 · 앞으로 · 위로 | `win.rs:8913-8921` |
| 마우스 X1 · X2 | 뒤로 · 앞으로 | `win.rs:8592-8604` |
| F2 | 인라인 이름변경(이름변경 중: Enter 확정 · Esc 취소) | `win.rs:8870-8872` · `8772-8784` |
| Esc | 컬럼 드래그 취소 · 탭 드래그 취소 | `win.rs:8793-8799` |
| Ctrl+T · Ctrl+W | 새 탭 · 탭 닫기 | `win.rs:8841-8845` |
| Ctrl+C · Ctrl+X · Ctrl+V | 복사 · 잘라내기 · 붙여넣기(이름변경 중에는 텍스트 대상) | `win.rs:8848-8857` · `8784-8786` |
| Ctrl+Z · Ctrl+Y · Ctrl+Shift+Z | 실행 취소 · 다시 실행 | `win.rs:8858-8863` |
| Delete · Shift+Delete | 휴지통 · 완전 삭제 | `win.rs:8873-8875` |
| Apps · Shift+F10 | 캐럿 행 컨텍스트 메뉴 | `win.rs:8876-8878` · `8910-8912` |
| Ctrl+H · Ctrl+. | 숨김 · Dot 파일 토글 | `win.rs:8885-8890` |
| Ctrl+Shift+N · Ctrl+Shift+R | 새 폴더 · 일괄 이름변경 | `win.rs:8879-8884` |

---

## 3. nexa-ui 매핑

확인 방법: `nexa-ui/crates` 전체를 `VirtualRows|RowSource|…` 등으로 Grep, `nexa-ctl/src/lib.rs`의 공개 목록 대조. `nexa-ui`의 크레이트는 `nexa-conf · nexa-ctl · nexa-dlg · nexa-font · nexa-fs · nexa-gfx · nexa-sys` 7개이며 **`nexa-grid`는 아직 없다**(`nexa-ui/docs/21-grid-family.md:26` · `111-121`은 계획).

### 3-1. 위젯 · 타입

| dir2 | nexa-ui 대응 | 상태 | 필요한 추가 · 차이 |
|---|---|---|---|
| `VirtualRows<S>` (`rows.rs:214`) | 없음. 유사 = `TreeGrid`(`nexa-ui/crates/nexa-ctl/src/controls/tree.rs:675`) | **추가 필요** | `TreeGrid`는 소유형 `TreeModel` + 행마다 문자열 복제(`tree.rs:146-163`), 단일 선택 + 표시 집합, 헤더 클릭 정렬 · 리사이즈 · 재배열 · 러버밴드 · 이름변경 · 타일 · 픽셀 스크롤이 없다. docs/21 G-1대로 `nexa-grid` 크레이트에 `VirtualRows` 엔진을 이식해야 한다 |
| `RowSource` 트레이트 (`rows.rs:79-145`) | 없음 | **추가 필요** | `len · row · cell · toggle · set_sort · is_selected · is_ghosted · select · select_span · select_all · clear_selection · find_prefix · tile_info · icon`. docs/21 G-2의 `write_cell` 훅은 선택 사항 |
| `RowItem` · `Marker` · `SelectOp` · `ScrollAlign` (`rows.rs:33-76` · `153-158`) | 없음 | **추가 필요** | 엔진과 함께 이식 |
| `ViewMode{Tree, Flat, Tiles}` (`rows.rs:189-194`) | `nexa_ctl::ViewMode{Rich, Compact, Plain}`(`nexa-ui/crates/nexa-ctl/src/view_mode.rs:9-17`) | **의미 불일치** | 이름만 같고 뜻이 다르다. 파일 그리드용 별도 열거형(예: `FileViewMode`)을 `nexa-grid`에 둔다 |
| `Column` · `Align` · `order_badge` (`columns.rs`) | `GridColumn{title, width, badge}`(`tree.rs:645-671`) | **부족** | `key · min_width · align · sortable · resizable`가 없다. dir2 `Column` 그대로 이식 |
| `TypeAhead` (`typeahead.rs`) | `nexa_ctl::TypeAhead`(`nexa-ui/crates/nexa-ctl/src/typeahead.rs:33`) | **동작 불일치** | nexa-ui: 기본 2000ms · 한글 조합기 내장 · **같은 키 반복도 누적**(순환은 ↑/↓ 전용, `typeahead.rs:10-12` · `99-111`). dir2: 1000ms · 반복 단일키 = cycle. dir2 동작 유지를 위해 "반복 단일키 cycle" 모드 옵션 추가 필요. 한글 조합은 얻는 이점 |
| 타입어헤드 필터(`ta_special` · `ta_space`) | `TypeAheadFilter{space, special}`(`typeahead.rs:147-176`) | 있음 | Backspace 허용 옵션은 호출 측에서 처리 |
| 타입어헤드 HUD(`rows.rs:2334-2363`) | `paint_hud` + `HudPos`(`typeahead.rs:180-257` · `292-329`) | 있음(모양 다름) | nexa-ui는 둥근 카드 + `accent` 글자, 라벨 접두 없음. dir2의 `찾기:` 접두 · 사각 테두리와 다름 — 모양 통일 여부 결정 필요. 위치 0..8 ↔ `HudPos` 변환 필요 |
| `FastScroller` · `FastScroll` · `ScrollAccel` · `SpeedHud` (`fastscroll.rs`) | `ScrollAccel` · `SpeedHud` · `FastScroll` · `set_fast_scroll`(`nexa-ui/crates/nexa-ctl/src/controls/scroll.rs:56-316`) | 있음 | nexa-ui가 원본. 차이: 기본값 `enabled=false · step 5 · max 8`(`scroll.rs:75-89`) vs dir2 `true · 3 · 16`. `hud_pos`는 `HudPos`. 그리드 전용 빠른 설정은 `ScrollBars::set_fast_override`(`scroll.rs:358`) |
| 오버레이 스크롤바(`rows.rs:674-903`) | `ScrollBars`(`scroll.rs:320` · `on_event` 458 · `tick` 628 · `paint` 655) | 있음 | 수치 차이: 두께 6/11 · 최소 썸 28 · 알파 0.35/0.6 · 자동 숨김 2000ms(`scroll.rs:20-29`). 오프셋 단위가 픽셀(dir2는 행 + 부분 오프셋) — 엔진의 스크롤 모델을 픽셀로 바꾸거나 변환층 필요 |
| 인라인 이름변경 필드(`edit.rs`의 `EditState::paint_field/hit/click/drag`) | `TextBox`(`nexa-ui/crates/nexa-ctl/src/controls/textbox.rs`) · `gridedit::LiveEditor`(`nexa-ui/crates/nexa-ctl/src/gridedit/live.rs:44-284`) | 있음(조립 필요) | nexa-ui `EditState`는 모델뿐(그리기 · 히트 없음). `LiveEditor`가 "편집 셀 한 곳에 TextBox" 패턴을 제공. 필요: 초기 선택을 이름부까지만 두는 진입(`with_selection_to` 대응), IME 캐럿 위치 조회. `LiveEditor`의 `CellSpec` 검증이 파일명 용도에 맞는지는 **추정**(미확인) |
| `Toolbar` + `ToolButton`(네비 버튼) | `Toolbar` + `ToolItem{id: String, icon, enabled, tip}`(`nexa-ui/crates/nexa-ctl/src/controls/toolbar.rs:93-107` · `230`) | 있음 | id가 문자열. 고정 버튼 폭 · 큰 글리프 지정 가능 여부는 **추정**(미확인) |
| `TabBar` · `TabAction` | `TabBar` · `TabAction{Switch, Close, New, Move, Context, …}`(`nexa-ui/crates/nexa-ctl/src/controls/tabbar.rs:29-45` · `111`) | 있음 | 잠금 · 고정 · 멀티라인은 docs/21 §3-3에 dir2 이식으로 기술. 세부는 탭 담당 인벤토리에서 확인 |
| `PathBar` | 없음(Grep 0건) | **추가 필요** | 브레드크럼 + 편집 + 자동완성 팝업 |
| `InfoDock` | 없음(Grep 0건). 유사 이름 `ToolDock`은 도구 모음 | **추가 필요** | 도크 담당 인벤토리 참조 |
| `History`(`nav.rs`) | `nexa_fs::History`(`nexa-ui/crates/nexa-fs/src/lib.rs:449-510`) | 있음(부족) | `replace`가 없다 — 추가 필요(재열람이 사용, `panel.rs:1439`) |
| `Invalidations` + `request_tick` | `nexa_ctl::Invalidations`(`nexa-ui/crates/nexa-ctl/src/widget.rs:14-38`) | **부족** | `request_tick · tick_requested · take_tick`이 없다(Grep 0건). nexa-ui는 컨트롤 `tick(now_ms) -> bool` 반환 규약 — 엔진을 이 규약으로 바꾸거나 틱 요청을 추가 |
| `Tree` · 선택 모델 · 정렬(`nexa-tree`) | 없음(UI 비종속 코어) | 그대로 이식 | `nexa_fs::sort_by`는 **자연 정렬**(`nexa-fs/src/lib.rs:252-312`)이라 dir2 정렬과 결과가 다르다. dir2 정렬 유지를 위해 `nexa-tree`의 비교기를 그대로 쓴다 |
| 열거(`nexa-vfs`) | `nexa_fs::list_opts` · `ListHandle`(비동기 배치) · `drives` · `VIRTUAL_ROOT`(`nexa-fs/src/lib.rs:84-135` · `lister.rs`) | 부분 대응 | `nexa_fs::Entry`에는 `attrs · target · kind(심링크)`가 없다. 클라우드 센티널 · 추가 루트도 없다 — `nexa-vfs`를 유지하고 드라이브 열거 · 숨김 판정만 `nexa-fs`로 교체하는 편이 안전 |
| 빈 폴더 프로브(`source.rs:46-60`) | `nexa_fs::has_visible_child`(`nexa-fs/src/lib.rs:187-227`) | 있음 | 느린 경로 생략(원격 · 광학) 판정은 없다 — 추가 필요 |
| 크기 · 날짜 표기 | `nexa_fs::fmt_size`(`lib.rs:870-883`) · `local_time` · `LocalTime::short`(`lib.rs:706-764`) | 있음(표기 다름) | `fmt_size`는 100 이상에서도 소수 1자리 · PB 없음 → dir2 `human_size` 유지. 날짜는 `local_time`이 날짜별 DST까지 처리(고정 오프셋보다 정확) — 형식은 동일 `yyyy-MM-dd HH:mm` |
| 셸 아이콘(`icons.rs`) | `nexa_fs::shell::IconService` · `IconKey{Kind, Path}` · `RgbaIcon` · `icon_for_kind/path(large)`(`nexa-ui/crates/nexa-fs/src/shell.rs:12-123`) | 있음 | 워커 + 캐시(상한 512) + `version()` 폴링 방식. dir2의 문자열 키 `(key, hint)` ↔ `IconKey` 변환 필요. 비Windows 구현 범위는 **추정**(`shell.rs:737` `cfg(not(windows))` 블록 미열람) |

### 3-2. 그리기 호출(`DrawCtx`)

| dir2 호출 | nexa-ui 대응 | 상태 |
|---|---|---|
| `select_font(slot, bold, italic)` (`draw.rs:26-28`) | `select_font(slot, bold)`(`nexa-ui/crates/nexa-ctl/src/draw.rs:45-47`) | **italic 없음** — 헤더 이탤릭(PANEL-117)에 필요. docs/21 G-1에 "DrawCtx 세대 어댑터"로 예고됨 |
| `FontSlot::{Base, List, Status}` | `FontSlot::{Base, PeerList, Message, Status, Mono}`(`draw.rs:16-28`) | `List` 없음 — `PeerList` 대용 또는 슬롯 추가 |
| `fill_rect` · `text_opaque` · `text` · `text_width` | 동일 시그니처(`draw.rs:58-68`) | 있음 |
| `glyph_opaque` · `glyph_opaque_lg`(MDL2 글리프) | `draw_chevron_90`(`nexa-ui/crates/nexa-ctl/src/controls/mod.rs:729`) · `polyline` · `glyphs.rs` | 대체 — 폰트 글리프 대신 벡터 셰브론. 네비 4종 아이콘(홈 · 뒤 · 앞 · 위)은 `ToolIcon`으로 준비 필요(보유 여부 **추정**) |
| `draw_icon(x, y, size, key, hint) -> bool` | `image` · `image_scaled(dst, &IconImage, clip)`(`draw.rs:119-127`) | **모델 다름** — nexa-ui는 호출 측이 `IconImage`를 들고 와서 그린다. 엔진의 `RowSource::icon`을 `Option<Rc<IconImage>>` 반환으로 바꾸거나 그리기 훅을 둔다 |
| `fill_round_rect_alpha(rect, r, color, alpha: u8)` | `fill_round_rect_alpha(rect, r, color, alpha: f32)`(`draw.rs:188-191`) | 단위 변환(0..255 → 0..1) |
| `push_clip` · `pop_clip` | 없음(Grep 0건). 각 그리기 호출이 `clip` 인자를 받음 | **차이** — 가로 스크롤 시 왼쪽 번짐 방지는 `clip` 교집합으로 처리(`tree.rs:356-370` 방식) |
| 텍스트 세로 위치 `(h − h×4/5)/2` 근사 | `text_center_y(y, h)` · `text_height` · `text_ascent`(`draw.rs:94-111`) | 대체 권장 — 맥 폰트에서 근사식은 위로 치우친다는 기록이 있다(`draw.rs:104-108`) |
| `theme.header_bg` | 없음. nexa-ui `Theme`에는 `chrome_bg · field_bg` 등(`nexa-ui/crates/nexa-ctl/src/theme.rs:12-56`) | **토큰 부족** — `header_bg`(헤더 · HUD · 용량 바 트랙)에 대응할 토큰 결정 필요(`TreeGrid`는 `chrome_bg` 사용, `tree.rs:854`) |
| `theme.panel_bg_alt · sel_bg · sel_bg_inactive · accent · text · text_dim · border` | 동일 이름 존재 | 있음 |

### 3-3. 입력 이벤트

| dir2 (`event.rs`) | nexa-ui (`nexa-ui/crates/nexa-ctl/src/event.rs`) | 차이 |
|---|---|---|
| `Key{Up…Space}` | `Key`에 `Enter · Escape · Delete · WordLeft/Right · SubwordLeft/Right` 추가(`event.rs:12-45`) | 상위 집합 |
| `InputEvent::Key{key, shift, ctrl}` | `Key{key, shift, primary}`(`event.rs:62-69`) | `ctrl` → `primary`(mac ⌘ / 그 외 Ctrl) |
| `MouseDown{x, y, shift, ctrl}` | `MouseDown{x, y, shift, primary}`(`event.rs:85-94`) | 동일 치환 |
| `Wheel · HWheel · Char{c, now_ms} · SelectAll · RightDown · MouseMove · MouseUp` | 동일 + `Undo · Redo` | 호환 |
| `set_wheel_lines` · `wheel_lines`(전역 줄 수) | 없음(`WheelAccum`만, `event.rs:118-134`) | **추가 필요** 또는 엔진 설정값으로 주입 |

---

## 4. OS 분기점

| # | 항목 | Windows 현 구현 | macOS | Linux |
|---|---|---|---|---|
| OS-1 | 최상위 루트 · 볼륨 열거 | `A:\`~`Z:\` 메타데이터 프로브(`nexa-vfs/src/lib.rs:116-133`), 가상 최상위 `::PC::` | `/` + `/Volumes/*` — `nexa_fs::drives()`가 이 규칙을 문서화(`nexa-fs/src/lib.rs:81-83`) | `/` + `/media/$USER/*` + `/mnt/*`(같은 근거). 추가로 `/proc/mounts` 기반 보강은 **추정** |
| OS-2 | 시작 폴백 루트 | `C:\` → 드라이브 문자 → `%USERPROFILE%`(`panel.rs:19-24`) | `$HOME` → `/` | `$HOME` → `/` (`nexa_fs::home_dir` 존재, `nexa-fs/src/lib.rs:368`) |
| OS-3 | 위로 이동의 끝 | 드라이브 루트(`parent()` 없음) → 가상 최상위(`panel.rs:997-1006`) | `/`의 부모 → 가상 최상위. `/Volumes/X`에서 위로는 `/Volumes`(실폴더)로 갈지 가상 최상위로 갈지 **결정 필요** | `/` → 가상 최상위. 마운트 지점 처리 동일하게 결정 필요 |
| OS-4 | 탭 제목 · 종류 라벨의 드라이브 판정 | 이름이 `:\`로 끝남(`source.rs:317`) · 후행 `\` 제거(`panel.rs:79-83`) | 볼륨 여부 플래그를 엔트리에 실어 전달(문자열 판정 금지) | 동일 |
| OS-5 | 숨김 파일 | `FILE_ATTRIBUTE_HIDDEN(0x2)`과 Dot 파일이 **별개 토글**(`nexa-tree/src/lib.rs:144-165`) | Dot 파일 + `UF_HIDDEN` 플래그(`stat.st_flags & 0x8000`, **추정**) → "숨김" 토글에 매핑, Dot 토글은 그대로 | 숨김 속성 없음 → "숨김" 토글은 효과 없음(비활성 표시 권장). 폴더의 `.hidden` 목록 파일(GNOME 관례) 지원은 **추정 · 선택** |
| OS-6 | 경로 대소문자 | 무시(`expand_key` 소문자화 `panel.rs:62-66` · `index_of_path` ASCII 무시 `lib.rs:540-546` · `remove_paths` 소문자화 `lib.rs:475-479`) | 기본 볼륨(APFS)은 대소문자 무시이나 구분 볼륨도 있음 | **구분** — `a`와 `A`가 다른 폴더. 비교 함수를 OS별 정책 함수 하나로 모아 Linux에서는 정확 일치로 |
| OS-7 | 경로 구분자 | `\`와 `/` 둘 다 후행 제거 · 접두사 비교(`panel.rs:902-926`) | `/`만. 파일명에 `\`가 올 수 있으므로 `\` 제거를 Windows 한정으로 | 동일 |
| OS-8 | 드라이브 용량 | `GetDiskFreeSpaceExW`(`source.rs:102-119`) | `statvfs`(`f_blocks × f_frsize`, `f_bavail × f_frsize`) | `statvfs` 동일 |
| OS-9 | 느린 경로 판정(프로브 생략) | UNC `\\` 접두 · `GetDriveTypeW` = REMOTE/CDROM(`source.rs:67-83`) | `statfs.f_fstypename`이 `smbfs · nfs · afpfs · webdav`면 생략(**추정**) | `/proc/mounts`의 fs 타입이 `nfs · cifs · smb3 · sshfs · fuse.*`면 생략(**추정**) |
| OS-10 | 타임존 | `GetTimeZoneInformation` → 고정 오프셋 주입(`win.rs:1368-`) | `nexa_fs::local_time`(`localtime_r`, `nexa-fs/src/lib.rs:815-859`) | 동일 |
| OS-11 | 휠 줄 수 | `SPI_GETWHEELSCROLLLINES`(`win.rs:6325-6342`) | winit `MouseScrollDelta::LineDelta/PixelDelta` — 시스템 줄 수 개념 없음, 기본 3 고정 + 설정값 | 동일(기본 3) |
| OS-12 | 픽셀 스크롤 구분 | `|delta| < 120`이면 터치패드로 간주(`rows.rs:1770-1785`) | `PixelDelta`를 직접 픽셀로 전달(관성 포함). 120 단위 변환 없이 `scroll_by_px` 경로로 | `PixelDelta`(Wayland) / `LineDelta`(X11 대부분) |
| OS-13 | 가로 휠 | `WM_MOUSEHWHEEL` + Shift+휠(`win.rs:7855-7958`) | 트랙패드 x 델타 그대로. Shift+휠은 OS가 이미 가로로 바꿔 주는 경우가 있어 이중 변환 주의(**추정**) | x 델타 + Shift+휠 변환 |
| OS-14 | 다중 선택 수식키 | Ctrl = 토글(`rows.rs:1967-1973`) | ⌘ = 토글(nexa-ui `primary`), Ctrl+클릭 = 우클릭 관례 | Ctrl |
| OS-15 | 네비 단축키 | Alt+← → ↑ ↓(`win.rs:8913-8929`) | ⌘[ · ⌘] · ⌘↑ · ⌘↓(Finder 관례) — 키맵 레이어에서 분기 | Alt+방향키 |
| OS-16 | 이름변경 · 삭제 키 | F2 · Delete · Shift+Delete | Enter(Finder 관례)와 F2 병행 여부 · ⌘⌫ = 휴지통 — **결정 필요** | F2 · Delete |
| OS-17 | 더블클릭 시간 | `GetDoubleClickTime`(`win.rs:8580`) | `NSEvent.doubleClickInterval`(**추정**) 또는 고정 500ms | GTK 설정 `gtk-double-click-time`(**추정**) 또는 고정 400~500ms |
| OS-18 | 타이머 | `SetTimer`(40 · 100 · 250ms) | winit `ControlFlow::WaitUntil` 기반 틱 스케줄러 | 동일 |
| OS-19 | 파일 아이콘 | `SHGetFileInfoW` 16px/32px + STA 로더 스레드(`icons.rs:145-169` · `381`) | `NSWorkspace.icon(forFile:)` / `icon(forFileType:)` — `nexa_fs::shell::IconService` 경유(비Windows 구현 범위 **추정**) | freedesktop 아이콘 테마 + MIME(`xdg-mime`) 또는 내장 아이콘 세트(`fallback_file_icon`, `nexa-ui/crates/nexa-ctl/src/controls/mod.rs:679`) |
| OS-20 | 파일별 아이콘 확장자 | `.exe .lnk .ico .cur .msi .scr .appref-ms`(`icons.rs:8`) | `.app` 번들 · `.icns` | `.desktop` · `.AppImage`(**추정**) |
| OS-21 | 바로가기 확장자 숨김 | `.lnk`(`source.rs:458-465`) | 해당 없음(별칭은 확장자 없음) — 규칙 비활성 | `.desktop`의 `Name=` 표시 여부는 **결정 필요**. 기본은 규칙 비활성 |
| OS-22 | 잘라내기 고스트 | 클립보드 미러(`CF_HDROP` + 잘라내기 표시, `source.rs:126-135`) | 앱 내부 잘라내기 집합으로 구현(Finder에는 잘라내기 개념이 없음) | 앱 내부 집합 + `x-special/gnome-copied-files`의 `cut` 헤더(**추정**) |
| OS-23 | 환경 변수 · 특수 폴더 | `%VAR%` · `$env:VAR` · `shell:`(`panel.rs:1526-1531`) | `$VAR` · `~` 확장(`nexa_fs::path::expand_env` · `resolve`, `nexa-fs/src/lib.rs:582-610`) | 동일 + XDG 사용자 폴더 |
| OS-24 | 파일 실행 | `ShellExecute`(호스트) | `open <path>` | `xdg-open <path>` |
| OS-25 | 세션 경로 구분자 | `\|` — Windows 경로에 못 쓰는 문자(`config.rs:837`) | **`\|`는 파일명에 쓸 수 있다** — 이스케이프 또는 줄 단위 형식으로 변경 필요 | 동일 |
| OS-26 | 가상 최상위 센티널 | `::PC::`(콜론은 Windows 파일명 금지 문자, `nexa-vfs/src/lib.rs:103-111`) | 콜론이 파일명에 허용된다 — 상대 경로 `::PC::`와의 충돌 가능성은 낮지만 **절대 경로가 아닌 값은 센티널**로 판정하는 규칙을 명시 | 동일 |
| OS-27 | IME 조합 창 위치 | `rename_edit_info`로 캐럿 좌표 제공(`rows.rs:552-565`) | winit `set_ime_cursor_area` | 동일 |
| OS-28 | 리사이즈 커서 | `resize_hot` → 수평 리사이즈 커서(`win.rs:7840`) | winit `CursorIcon::ColResize` | 동일 |

---

## 5. 상태 · 영속 · 스레딩 · 메시지 흐름

### 5-1. 설정 키(목록 · 패널 관련 — `settings` 파일)

| 키 | 기본값 | 의미 | 근거 |
|---|---|---|---|
| `show_hidden` · `show_dotfiles` | true · true | 새 탭 기본 보기 필터 | `config.rs:256-257` |
| `sort_folders_first` | true | 폴더 우선 | `config.rs:285` |
| `sort_case_sensitive` | false | 대소문자 구분 정렬 | `config.rs:289` |
| `view_scope` | `panel` | 보기 토글 전파 폭(`tab`/`panel`/`global`) | `config.rs:286` · `win.rs:5134-5139` |
| `view_mode` | `tree` | 새 세션 · 세션에 없는 탭의 보기 모드 | `config.rs:309` |
| `hide_empty_glyph` | true | 빈 폴더 펼침 글리프 숨김 | `config.rs:287` |
| `nav_up_align` | `center` | 위로 이동 후 선택 행 배치(`top`/`center`/`bottom`) | `config.rs:290` · `win.rs:6795-6802` |
| `typeahead_scope` | `visible` | `visible`/`global`/`level` | `config.rs:294` · `win.rs:6765-6771` |
| `typeahead_reset_ms` | 1000 | 버퍼 리셋 시간 | `config.rs:295` |
| `typeahead_pos` | 6 | HUD 위치 0..8 | `config.rs:296` |
| `typeahead_special` · `typeahead_space` · `typeahead_backspace` | true × 3 | 입력 허용 옵션 | `config.rs:297-299` |
| `fast_scroll` · `_step` · `_max` · `_window_ms` | true · 3 · 16 · 160 | 고속 스크롤 | `config.rs:300-303` |
| `fast_scroll_hud` · `_hud_pos` · `_hud_hold_ms` · `_hud_fade_ms` | true · 2 · 250 · 600 | 속도 배지 | `config.rs:304-307` |
| `fast_scroll_grid_extra` | true | 파일 그리드 한 단계 더 빠르게 | `config.rs:308` |
| `col_width_sync` | true | 컬럼 폭/순서 좌우 동기 | `config.rs:310` |
| `col_autofit_max` | 400 | 자동 맞춤 상한(px@96dpi) | `config.rs:311` |
| `list_folder_bold` · `header_bold` · `header_italic` | false × 3 | 폰트 장식 | `config.rs:282-284` |
| `dnd_hover_ms` | 3000 | DnD 호버 펼침/탭 전환 대기 | `config.rs:271` |
| `tab_dblclick` | `close` | 탭 더블클릭 동작(`close`/`pin`/`lock`) | `config.rs:291` · `win.rs:8649-8656` |

dir3에서는 nexa-sql의 설정 레지스트리(nsql-settings) 구조로 옮기되 **키 의미와 기본값은 위 표를 유지**한다(레지스트리 측 키 이름 규칙은 설정 담당 인벤토리 참조).

### 5-2. 세션 파일(`data\session.cfg` · `# nexa-dir session v1`)

| 키 | 형식 | 근거 |
|---|---|---|
| `active_panel` | 0 또는 1 | `config.rs:840` |
| `panel{i}.tabs` | 경로를 `\|`로 연결 | `config.rs:841-847` |
| `panel{i}.active` | 활성 탭 인덱스 | `config.rs:848` |
| `panel{i}.exp{j}` | 탭 j의 펼침 경로를 `\|`로 연결(빈 목록 생략, 탭당 최대 200) | `config.rs:849-858` · `panel.rs:279-289` |
| `panel{i}.locked` · `.pinned` | `0`/`1`을 `\|`로 연결(하나라도 참일 때만 기록) | `config.rs:859-876` |
| `panel{i}.modes` | `tree`/`flat`/`tiles`를 `\|`로 연결(전부 tree면 생략, 미지 값 = tree) | `config.rs:877-880` |
| `panel{i}.views` | 탭별 플래그 정수(bit0 숨김 · bit1 Dot · bit2 폴더 우선) | `config.rs:881-885` |
| `panel{i}.cols` | `cols[name:1,ext:1,size:0,…]` 레이아웃 문자열 | `config.rs:887-889` · `1075` |
| `panel{i}.colw` | 표시 컬럼의 표시 순 폭을 `,`로 연결(탭별 확장은 `panel{i}.colw{j}` 예약) | `config.rs:890-893` |

저장 계기: 패널의 `session_dirty` → 호스트 1000ms 디바운스(`win.rs:91` · `5072-5076`) + 종료 시.

**영속하지 않는 것**: 정렬 상태(`VirtualRows.sort` — 위젯 메모리에만 있고 탭 수명 동안 유지, `rows.rs:234-235`), 선택 · 캐럿 · 스크롤, 히스토리.

### 5-3. 스레딩

- 패널 · 목록 · 트리는 **전부 UI 스레드 단일 스레드**다. `TreeSource`의 프로브 캐시가 `RefCell`인 것이 그 전제(`source.rs:39-41`).
- 열거는 **동기**: `Tree::open_filtered` · `expand`가 UI 스레드에서 `read_dir`을 돈다(`nexa-tree/src/lib.rs:229-286`). 큰 폴더 · 네트워크 경로에서 UI가 멈출 수 있다.
- 빈 폴더 프로브도 **페인트 경로에서 동기**(폴더당 1회 캐시, `source.rs:198-215`).
- 워커가 있는 곳(범위 밖): 아이콘 로더 스레드(`icons.rs:381`), 클라우드 목록(콜백 + 캐시, `nexa-tree/src/lib.rs:272-277`), watcher, 휴지통 작업.

### 5-4. 메시지 흐름(요약)

```
OS 입력 → 호스트(win.rs) → InputEvent → Panel::on_event(y-라우팅) → VirtualRows::on_event
                                              ↓ 상태 변경
                           Invalidations(더러운 rect + tick 요청) → 호스트 flush → 재그리기
호스트 → Panel::drain_actions(탭 · 경로 바 · 네비 버튼 동작 수거) → navigate_to/…
호스트 → finish_input/update_status → 낡은 탭 재열람 · 세션 디바운스 · watcher 재구독 · 컬럼 동기 폴링
타이머: 위젯 틱 40ms(스크롤바 · 배지) · 타입어헤드 250ms · 리네임 지연(더블클릭 시간) · DnD 폴링 100ms
```

- 위젯 → 호스트 통지는 전부 **폴링형 1회성 플래그**다: `take_col_resized` · `take_col_reordered` · `take_session_dirty` · `take_tab_menu`(`panel.rs:302-322` · `700-702` · `1175-1177`).
- 페인트는 `&self`(불변) — 상태 변경 없는 렌더(`rows.rs:2127`).

---

## 6. 이식 시 주의 — 회귀 방지에 필요한 실측 교훈

| # | 교훈 | 근거 |
|---|---|---|
| L-1 | OS 드래그가 버튼 해제를 소비하면 MouseUp이 오지 않는다. 보류된 클릭 확정이 다음 클릭에서 낡은 행을 단일 선택하던 결함 → 새 프레스에서 보류 폐기 + 드래그 복귀 시 `abort_press` + 소스 교체 시 리셋. **선택 집합은 건드리지 않는다** | `rows.rs:1156-1163` · `1705-1713` · `1173` · `win.rs:8361-8369` |
| L-2 | 러버밴드에서 4px 미만 이동은 클릭 지터 — 높이 0 사각형이 선택 해제로 떨어지던 결함 | `rows.rs:2036-2040` |
| L-3 | 위젯 정렬이 **비어 있을 때** 새 소스에 `set_sort(&[])`를 부르면 기본 정렬(이름 오름)이 열거 순서로 퇴행하고 정렬 옵션이 무효화된다 → 명시된 경우에만 재적용. 소스의 기본 `sort_keys`도 `[(Name, false)]`로 둔다 | `rows.rs:1175-1181` · `source.rs:155-156` |
| L-4 | 도크가 표시 플래그는 켜졌지만 높이 0인 경우 `y >= 0`이 항상 참이라 모든 클릭이 빈 도크로 삼켜졌다 → `dock_shown()`(플래그 그리고 높이 > 0)으로 통일 | `panel.rs:431-437` · `1461-1464` |
| L-5 | MouseUp을 탭 바 · 경로 바 · 목록 · 도크 전부에 전달하지 않으면 드래그 상태가 남는다 | `panel.rs:1493-1502` |
| L-6 | 이름변경 필드 · 마커 존은 "첫 열"이 아니라 **현재 열 순서의 트리 열(key 0)** 위에 놓아야 한다. 트리 열이 숨겨지면 이름변경 진입 거부 | `rows.rs:1011-1022` · `445-447` · `568-579` |
| L-7 | 빈 폴더 글리프가 억제돼도 폴더 서식(굵게 · 이름변경 전체 선택)은 유지 → `marker`와 `is_dir` 분리 | `rows.rs:59-61` · `451` · `1390-1392` |
| L-8 | 이름변경 필드는 텍스트 왼쪽으로 3px 확장해 편집 진입 시 이름 x가 밀리지 않게 한다 | `rows.rs:147-149` |
| L-9 | 느린 재클릭 리네임: 메뉴 모달 루프가 타이머를 디스패치해 메뉴 아래에 필드가 열리던 결함 → 새 클릭 · 우클릭 · 키 · 명령에서 예약 폐기, 만료 시 (패널, 경로) 대조. 리네임을 끝낸 그 클릭은 새 예약을 하지 않는다 | `win.rs:3997-4003` · `8132-8147` · `9584-9602` |
| L-10 | 편집 필드 안 더블클릭은 텍스트 선택이지 행 활성화가 아니다(편집 중인 폴더로 진입하던 결함) | `win.rs:8620-8637` |
| L-11 | 리네임 중 다른 행 클릭은 "취소 후 정상 행 클릭"이다. 히트를 None으로 처리하면 재클릭 리네임에 클릭이 한 번 더 필요해진다 → 가드는 **필드 안 클릭만** | `win.rs:8052-8068` |
| L-12 | 방문 ≠ 확장: 더블클릭 진입은 펼침 집합에 등재하지 않는다. 펼침 집합은 가시 트리 스냅샷이 아니라 탭별 영속 집합이어야 형제 폴더 펼침이 유지된다 | `panel.rs:798-802` · `1960-1963` |
| L-13 | 비활성 탭은 감시 대상이 아니다 → 탭이 드러나는 순간(전환 · 닫기)을 갱신 계기로 삼는다. 재열람 **시도** 자체를 스탬프해야 실패 시 재시도가 폭주하지 않는다 | `panel.rs:55-58` · `742-744` · `1396-1402` |
| L-14 | 보고 있던 폴더가 사라지면 같은 경로 재열람은 영원히 실패한다(F5도 무효) → 최근접 존재 조상으로 이동 | `panel.rs:1405-1418` |
| L-15 | 접힌 폴더 내부의 파일 생성은 부모 통지로 신뢰성 있게 오지 않는다 → 뷰포트에 보이는 접힌 폴더도 감시 목록에 넣는다 | `panel.rs:852-864` |
| L-16 | 내 PC 컬럼 전환은 **전환 시점에만** — 평시 `set_columns` 재호출은 사용자 폭을 리셋한다 | `panel.rs:804-823` |
| L-17 | 셀 · 아이콘 조회는 프레임당 "가시 행 × 열" 번 호출된다 → 이름을 복제하지 않는 `row_ref` 사용, 잘라내기 집합이 비면 경로 조회 생략 | `source.rs:280` · `357-358` · `nexa-tree/src/lib.rs:511-523` |
| L-18 | 페인트 경로의 프로브는 느린 경로(네트워크 · 광학)에서 수 초 걸릴 수 있다 → 생략하고 글리프 유지. 열 수 없으면 "있음"으로 본다 | `source.rs:62-88` · `44-49` |
| L-19 | 가로 스크롤 시 백엔드가 텍스트 **오른쪽만** 자른다 — 왼쪽 번짐은 별도 클립이 필요 | `nexa-dir2/crates/nexa-gui/src/draw.rs:42-50` · `nexa-ui/crates/nexa-ctl/src/controls/tree.rs:356-358` |
| L-20 | 컬럼 리사이즈는 단독 조절(이웃 불변 · 총폭 가변)로 확정 — 한 쌍 동시 조절은 폐지 | `rows.rs:175-177` |
| L-21 | 정밀 터치패드는 OS가 이미 가속 · 관성을 넣는다 → 노치 미만 delta에는 고속 배수를 적용하지 않는다 | `fastscroll.rs:9-10` · `315-328` |
| L-22 | 스크롤바 호버는 **보이는 축**에만 — 숨겨진 바 자리에 커서를 올려도 드러나지 않는다. 축은 독립(세로 스크롤 = 세로만) | `rows.rs:782-789` · `740-747` |
| L-23 | 대소문자 구분 정렬은 코드포인트 순(대문자 그룹 상단)으로 확정 — "알파벳 유지 + 동률 대문자 우선" 규칙은 폐기됨 | `nexa-tree/src/lib.rs:780-788` |
| L-24 | 종류 정렬은 `kind_rank`만으로는 파일이 전부 동률이라 오름/내림이 같아진다 → 확장자를 2차 키로 | `nexa-tree/src/lib.rs:330-334` |
| L-25 | 폴더 심볼릭 링크 · 정션은 종류를 `Dir`로 분류해야 진입 · 펼침이 된다(링크 표식은 속성으로 따로) | `nexa-vfs/src/lib.rs:47-60` |
| L-26 | 빈 폴더 펼침은 가시 행 변화가 0이어도 마커가 바뀌므로 다시 그려야 한다 | `source.rs:339-342` |
| L-27 | 범위 밖 id를 선택에 넣지 않는다(이후 직접 인덱싱 패닉 방지) | `nexa-tree/src/lib.rs:705-714` |

**현행 동작으로 확인된 특이점**(그대로 옮길지 고칠지 결정 필요):

| # | 내용 | 근거 |
|---|---|---|
| Q-1 | 타입어헤드 HUD 라벨 `찾기: `가 한국어로 하드코딩(i18n 미경유) | `rows.rs:1659` · `2335` |
| Q-2 | 타일 보기에서는 속도 배지를 그리지 않는다(타일 분기가 배지 그리기 전에 반환) | `rows.rs:2130-2134` · `2365-2366` |
| Q-3 | 타일 보기 PageUp/PageDown의 `page`가 타일 높이가 아닌 `row_h` 기준으로 계산된다(= 화면보다 3배 멀리 이동) | `rows.rs:1767` · `1829-1830` |
| Q-4 | `set_metrics`가 컬럼을 기본 컬럼으로 교체해 DPI 변경 시 사용자 폭 · 순서가 리셋된다 | `panel.rs:462-465` |
| Q-5 | `duplicate_tab`은 `session_dirty` 설정 · 정렬 옵션 재적용 · 보기 모드 계승을 하지 않는다(`new_tab`과 다름) | `panel.rs:614-647` 대 `560-589` |
| Q-6 | 정렬 상태는 세션에 저장되지 않는다 | `rows.rs:234-235` · `config.rs:836-896` |
| Q-7 | 러버밴드는 드래그 중 자동 스크롤을 하지 않는다(가시 범위만 선택) | `rows.rs:2059-2075` |
| Q-8 | `reopen_filtered`의 선택 복원은 `index_of_path`(가시 행)에 의존 — 부모가 접힌 선택 항목은 복원되지 않는다 | `panel.rs:1444-1450` |

---

## 7. 회귀 테스트 후보

자동화: **U** = 단위(순수 로직 · 기록형 `DrawCtx`) · **H** = 헤드리스 하네스(합성 이벤트 + 임시 폴더) · **M** = 수동/시각 확인.

| # | 시나리오 | 대상 ID | 자동화 | 이식할 기존 테스트 |
|---|---|---|---|---|
| T-01 | 패널 400×400 배치 좌표 일치(탭 바 · 네비 · 경로 바 · 목록) | PANEL-002 | U | `panel.rs::layout_stacks_tabbar_navbar_rows` |
| T-02 | 탭 열기 · 전환 · 닫기 · 최소 1탭 · 탭별 히스토리 독립 | PANEL-014~016 · 026~028 | H | `panel.rs::tabs_open_switch_close_keep_at_least_one` · `per_tab_history_is_independent` |
| T-03 | 탭 이동 · 잠금 · 복제 · 고정 · 패널 간 이동 | PANEL-017~022 | H | `panel.rs::tab_move_lock_duplicate` · `detach_attach_moves_tab_between_panels` |
| T-04 | 탐색 실패 시 위치 · 경로 바 유지 | PANEL-027 · 035 | H | `panel.rs::navigate_failure_keeps_position` |
| T-05 | 위로 이동 후 떠난 폴더 선택(3-OS: 루트 → 가상 최상위 포함) | PANEL-029 · 030 | H | `panel.rs::nav_up_from_drive_root_enters_my_pc` · `nav_home_jumps_to_my_pc_from_any_depth`(OS별 변형 신규) |
| T-06 | 펼침 상태가 하위 진입 · 상위 복귀 · 형제 진입 왕복에서 유지, 접은 폴더는 접힌 채 | PANEL-040 | H | `panel.rs::navigation_carries_expansion_down_and_up` · `sibling_expansion_survives_enter_and_return` |
| T-07 | 재열람이 펼침 · 선택 · 캐럿 · 스크롤 보존 + 새 항목 반영 | PANEL-036 | H | `panel.rs::reopen_preserves_expanded_selection_caret_scroll` |
| T-08 | 폴더 소실 시 최근접 조상으로 이동 | PANEL-037 | H | `panel.rs::reopen_falls_back_to_nearest_ancestor_when_folder_vanishes` |
| T-09 | 탭 활성화 = stale, 재열람이 소거 · 탭별 보기 옵션 계승 · 세션 플래그 왕복 | PANEL-015 · 039 · 045 · 046 | H | `panel.rs::tab_activation_marks_stale_and_reopen_clears` · `per_tab_view_values_inherit_and_stale` |
| T-10 | 세션 복원(탭 · 펼침 · 모드 · 잠금 · 고정) + 경로에 `\|`가 든 폴더(mac/Linux 신규) | PANEL-023 · 024 · OS-25 | H | `panel.rs::restore_seeds_expanded_from_session` + 신규 |
| T-11 | 보기 모드 탭별 · 타일/일반 진입 시 평탄화 | PANEL-051 | H | `panel.rs::view_mode_propagates_and_flattens` · `view_mode_is_per_tab` |
| T-12 | 정렬 옵션(대소문자)이 토글 · 재로드 · 새 탭에서 유지 | PANEL-047 · 048 | H | `panel.rs::sort_opts_propagate_to_new_sources` |
| T-13 | 감시 대상에 뷰포트의 접힌 폴더 포함 · 일반 보기는 비대상 | PANEL-042 | H | `panel.rs::viewport_dirs_lists_visible_tree_dirs` |
| T-14 | 셀 값(확장자 · 크기 · 종류 · 날짜) · 폴더는 빈 값 | PANEL-057~059 | U | `source.rs::cells_project_ext_size_kind_and_dirs_are_blank` · `human_size_units_and_precision` · `fmt_datetime_applies_offset_and_epoch_math` |
| T-15 | 빈 폴더 글리프 억제(필터 반영 · 펼친 뒤 실측 · 토글 즉시) | PANEL-049 · 055 · 067 | H | `source.rs::hide_empty_markers_suppresses_by_current_filter` · `hide_empty_markers_uses_loaded_children_after_expand` |
| T-16 | 헤더 정렬 3상태 · 단일 리셋 · Shift 다중열 · 라벨 화살표/순번 | PANEL-077 · 078 | U | `rows.rs::header_click_cycles_three_states` · `plain_click_resets_to_single_sort` · `shift_click_adds_cycles_and_removes_keys` · `header_label_shows_arrow_before_and_badge_after` |
| T-17 | 컬럼 리사이즈(단독 조절 · 최소 폭 · 업 이후 무효) | PANEL-079 | U | `rows.rs::drag_handle_resizes_column_with_min_width` |
| T-18 | 컬럼 재배열 후 이름변경 필드 · 마커 존이 트리 열을 따라감, 트리 열 숨김 시 진입 거부 | PANEL-080 · 092 · 106 | U | `rows.rs::rename_field_follows_tree_column_after_reorder` · `marker_zone_follows_tree_column_after_reorder` |
| T-19 | 컬럼 드래그 재배열: 임계 · 라이브 미리보기 · MouseUp 확정 · ESC 복원 | PANEL-080 | U | **신규**(기존 없음) |
| T-20 | 컬럼 자동 맞춤(가시 행 + 헤더 · 상한) | PANEL-081 | U | **신규** |
| T-21 | 클릭 선택(단일 · Ctrl/⌘ 토글 · Shift 범위) · 우클릭 규약 · 전체 선택 | PANEL-090 · 094 · 095 | U | `rows.rs::click_selects_single_ctrl_toggles_shift_ranges` · `right_down_selects_unselected_keeps_selection_clears_on_empty` · `ctrl_a_selects_all_visible` |
| T-22 | 드래그 뒤 낡은 프레스 보류가 선택을 되돌리지 않음(3경로) | PANEL-091 | U | `rows.rs::press_pending_does_not_survive_replace_source` · `new_press_discards_stale_pending_without_replace_source` · `abort_press_clears_pending_and_keeps_selection` |
| T-23 | 러버밴드: 교차 행 선택 · 업 후 유지 · 4px 지터 무시 | PANEL-093 | U | `rows.rs::rubber_band_selects_intersecting_rows_and_ends_on_up` + 지터 신규 |
| T-24 | 키보드: 캐럿 이동 · Shift 범위 · Ctrl 캐럿만 · Space 토글 · ←/→ 트리 조작 | PANEL-096~098 | U | `rows.rs::caret_moves_select_and_scroll_follows` · `shift_moves_range_and_ctrl_moves_caret_only` · `right_left_keys_toggle_expansion_at_caret` |
| T-25 | 타입어헤드: 점프 · 반복키 순환 · 누적 · Backspace · 타임아웃 · 공백 규칙 · 3범위 | PANEL-102 · 124 | U | `typeahead.rs` 4건 · `rows.rs::typeahead_jumps_cycles_and_accumulates` · `typeahead_times_out_via_tick_and_space_is_excluded` · `nexa-tree` `find_prefix_*` 3건 |
| T-26 | 인라인 이름변경: 시작(이름부 선택) · 입력 · 키 차단 · 확정 · 취소 | PANEL-104~108 | U | `rows.rs::inline_rename_flow_and_key_block` + 이름부 선택 신규 |
| T-27 | 느린 재클릭 리네임: 간격 · 지연 · 예약 폐기 조건 · 더블클릭과의 구분 | PANEL-109 | H | **신규**(시각 주입 필요) |
| T-28 | 스크롤: 상한 클램프 · 가로 휠 클램프 · 폭 확대 시 재클램프 · 픽셀 스크롤 부분 행 히트 · 고속 배수 | PANEL-084~087 | U | `rows.rs::scroll_clamps_to_total_minus_full_rows` · `hwheel_scrolls_and_clamps_to_total_width` · `widening_bounds_reclamps_scroll_x` · `fast_scroll_multiplies_rapid_wheel_notches_but_not_trackpad` |
| T-29 | 오버레이 스크롤바: 썸 드래그 · 트랙 클릭 · 축 독립 표시 · 페이드 | PANEL-088 · 089 | U | **신규** |
| T-30 | 타일 보기 기하 · 히트 · 키 · 스크롤 상한 · 사각 선택 | PANEL-099 · 101 | U | `rows.rs::tiles_grid_geometry_and_keys` + 사각 선택 신규 |
| T-31 | 일반 보기: 마커 존 없음 · ←/→ 무동작 · 호버 펼침 비대상 | PANEL-073 · 114 | U | `rows.rs::flat_mode_blocks_expansion` · `hover_expand_only_collapsed_dirs_in_tree_mode` |
| T-32 | DnD 엣지 스크롤(한 행 높이 존) | PANEL-114 | U | `rows.rs::drag_scroll_edge_moves_one_row_within_edge_zone` |
| T-33 | 페인트: 헤더 셀 · 우측 정렬 x 좌표 · 잘라내기 행 흐림 | PANEL-069 · 074 · 062 | U | `rows.rs::paint_draws_header_cells_and_right_aligned_size` · `ghosted_row_paints_name_dim` |
| T-34 | 소스 교체 시 뷰 리셋 + 정렬 유지 | PANEL-111 | U | `rows.rs::replace_source_resets_view_but_keeps_sort` |
| T-35 | 트리 코어: 열기 · 펼침/접힘 왕복 · 중첩 펼침 복원 · 경로 조회 · 표시 제외 · 선택(교차 폴더 · 범위 · 전체) | PANEL-119 · 125~128 | U | `nexa-tree/src/lib.rs`의 해당 12건 |
| T-36 | 트리 정렬 전 키 · 대소문자 구분 · 펼침 보존 · 열거 순서 복원 | PANEL-122 · 123 | U | `nexa-tree/src/lib.rs`의 `sort_*` · `set_sort_*` · `case_sensitive_groups_uppercase_first` |
| T-37 | 10만 노드 스케일 가드 | PANEL-130 | U | `nexa-tree/src/lib.rs::large_tree_scale_ops_complete` |
| T-38 | **Linux 대소문자 구분**: `a/`와 `A/`가 공존할 때 펼침 집합 · 경로 조회 · 표시 제외가 서로 섞이지 않음 | PANEL-040 · 126 · 128 · OS-6 | H | **신규** |
| T-39 | **숨김/Dot 필터 OS별 의미**: Windows 숨김 속성 · mac 숨김 플래그 · Linux Dot만 | PANEL-121 · OS-5 | H | `nexa-tree::open_filtered_excludes_dotfiles` + OS별 신규 |
| T-40 | 가상 최상위 목록 · 전용 컬럼 전환 · 용량 표시(3-OS) | PANEL-044 · 066 · 120 | H | `nexa-tree::open_virtual_root_lists_drives`(OS별 변형 신규) |
| T-41 | 시각 확인: 선택/비활성 색 · 캐럿 테두리 · HUD 위치 9곳 · 속도 배지 · 고스트 헤더 · 타일 용량 바 | PANEL-070 · 076 · 080 · 087 · 100 · 103 | M | 스크린샷 비교(골든 이미지)로 반자동화 가능 |

**핵심 기능 점검(스모크) 후보**: T-02 · T-06 · T-07 · T-16 · T-21 · T-22 · T-24 · T-26 · T-28 · T-35 — 임시 폴더 픽스처만으로 수 초 안에 돌 수 있다.
