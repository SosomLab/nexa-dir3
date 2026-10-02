# 14 — nexa-dir2 인벤토리: nexa-gui 위젯·기반 + 경로 입력·툴팁 (GUI-NNN)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 기준일 2026-10-03 · 원본 = nexa-dir2 `0.22.0`(커밋 `89c635c`) · 대조 = nexa-ui `df75f5a`(102차).
> 이 문서의 `GUI-NNN` ID는 이후 구현·교차 검증의 체크리스트다. ID는 고정이며 재번호하지 않는다(결번 허용).

**경로 약어**(표가 길어지지 않게 — 전부 `저장소/경로:줄` 형식의 축약이다)

| 약어 | 실제 경로 |
|---|---|
| `G/` | `nexa-dir2/crates/nexa-gui/src/` |
| `A/` | `nexa-dir2/crates/nexa-app/src/` |
| `U/` | `nexa-ui/crates/nexa-ctl/src/` |
| `UF/` | `nexa-ui/crates/nexa-fs/src/` |
| `UD/` | `nexa-ui/crates/nexa-dlg/src/` |
| `S/` | `nexa-sql/crates/` |

---

## 0. 범위 — 읽은 파일과 줄 수

### 0-1. 담당 범위(전부 끝까지 읽음)

| 파일 | 줄 | 내용 | 내장 테스트 수 |
|---|---:|---|---:|
| `G/lib.rs` | 24 | 모듈 선언·재수출 | 0 |
| `G/widget.rs` | 93 | `Widget` trait · `Invalidations`(rect 병합 + 틱 요청) | 2 |
| `G/geom.rs` | 112 | `Point`/`Size`/`Rect` | 4 |
| `G/event.rs` | 134 | `InputEvent`/`Key`/`WheelAccum`/휠 줄 수 전역 | 3 |
| `G/edit.rs` | 466 | 한 줄 편집 상태 `EditState` + 필드 페인트 | 8 |
| `G/fastscroll.rs` | 495 | 고속 스크롤(`FastScroll`·`ScrollAccel`·`SpeedHud`·`FastScroller`) | 5 |
| `G/widgets/mod.rs` | 16 | 위젯 재수출 | 0 |
| `G/widgets/overlaybar.rs` | 406 | 오버레이 스크롤바 공용 상태·기하 | 5 |
| `G/widgets/tabbar.rs` | 522 | 패널 탭 바 | 6 |
| `G/widgets/menubar.rs` | 637 | 메뉴 바 + 드롭다운 + 1단계 하위 메뉴 | 7 |
| `G/widgets/chrome.rs` | 518 | `Toolbar`·`ToolButton`·`StatusBar` | 2 |
| `G/widgets/pathbar.rs` | 751 | 브레드크럼 경로 바 + 편집 + 자동완성 팝업 | 7 |
| `G/widgets/dock.rs` | 1407 | 하단 도크 `InfoDock`(정보/미리보기/터미널 스트립 + 텍스트 뷰) | 13 |
| `A/pathinput.rs` | 256 | 환경변수 확장 · 폴더 제안(순수 로직) | 4 |
| `A/shellpath.rs` | 97 | `shell:` 특수 폴더 스킴 해석(Win32) | 3 |
| `A/tip.rs` | 161 | 도구 모음 툴팁 팝업 창(Win32) | 0 |
| **합계** | **6,095** | | **69** |

### 0-2. 배치·흐름 확인용으로 부분 열람(범위 밖 — 상세 인벤토리는 다른 문서 몫)

- `G/draw.rs` 1-127(전부) · `G/theme.rs` 1-126(전부) — 위젯이 쓰는 그리기 어휘·토큰 확인용.
- `A/panel.rs` 90-196 · 340-540 · 1380-1560 — 패널 안 위젯 배치·마우스 라우팅·동작 수거.
- `A/win.rs` 40-214 · 376-845 · 1340-1366 · 1808-2025 · 2470-2530 · 4800-4955 · 5836-5908 · 6100-6190 · 6320-6344 · 7108-7345 · 7460-7662 · 7958-8030 · 8150-8265 · 8628-8672 · 8690-8739 · 9486-9521 — 전체 레이아웃·메뉴/도구 모음 정의·타이머·툴팁·경로바 편집 키.
- `A/config.rs` — Grep으로 설정 키만(160-169 · 212-229 · 260-312 · 434-510 · 575-753).
- nexa-ui 대조: `nexa-ui/CLAUDE.md`(전부) · `nexa-ui/docs/TODO.md`(전부) · `nexa-ui/docs/STATUS.md` 1-60 · `U/lib.rs` · `U/draw.rs` · `U/widget.rs` · `U/event.rs` · `U/theme.rs` 1-140 · `U/controls/mod.rs`(전부) · `U/controls/tabbar.rs` 1-1220 · `U/controls/pulldown.rs` 1-800 · `U/controls/scroll.rs` 1-770 · `U/controls/splitter.rs` 1-150 · `U/controls/toolbar.rs` 1-300 · `U/controls/tooldock.rs` 1-175 · `U/controls/editmenu.rs` 1-140 · `U/controls/ctxmenu.rs` 95-270 · 520-600 · `U/edit.rs` 1-140 + 공개 API Grep · `U/controls/textbox.rs` 공개 API Grep · `UF/lib.rs` 556-702 · `UF/shell.rs` 325-470 · 528-560 · 1002-1015.

> ⚠ `G/widgets/rows.rs`(3,548줄) · `G/columns.rs` · `G/typeahead.rs` · `G/draw.rs` · `G/theme.rs`는 이 문서의 **담당 범위가 아니다**(파일 목록/그리드·그리기 백엔드 문서에서 다룬다 — 추정). 여기서는 담당 위젯이 **의존하는 부분만** GUI-008로 요약한다.

---

## 1. 기능 목록

이식 분류: **N** = 플랫폼 중립(거의 그대로) · **A** = nexa-ui 컨트롤·그리기로 교체 · **P** = OS별 구현 분기 필요 · **W** = Windows 전용 유지(타 OS는 대체·비활성).
"Win32/OS 의존" 칸의 `—` = 의존 없음(nexa-gui는 OS 의존 0 — `G/lib.rs:3`).

### 1-1. 기반 — widget · geom · event · lib

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-001 | 논리 위젯 계약 | `Widget` = `bounds` · `set_bounds(bounds, inv)` · `on_event(ev, inv)` · `paint(&self, ctx, theme)`. 좌표는 전부 창 클라이언트 px. 자식 HWND 없음(창 1개 + 논리 위젯). paint는 `&self`(캐시는 `RefCell`/`Cell`) | `G/widget.rs:57-68` | — | N | — |
| GUI-002 | 더러워진 영역만 다시 그리기 | `Invalidations::push`: 빈 rect 무시 · **교차하는 기존 rect가 있으면 union 병합**(변 접촉은 비교차) · `drain`으로 호스트가 OS 무효화로 번역 | `G/widget.rs:35-54` · 호스트 `A/win.rs:1824-1837` | 호스트: `InvalidateRect` | N | `push_merges_intersecting_rects` · `empty_rect_is_ignored` |
| GUI-003 | 위젯의 시간 기반 후속 처리(페이드 등) | `request_tick()`/`tick_requested()`/`take_tick()`. 호스트 flush가 `take_tick()`이면 40ms 타이머 무장 → 틱 핸들러가 각 위젯 `tick` 호출 → 재요청 없으면 타이머 해제 | `G/widget.rs:14-33` · `A/win.rs:1825-1827` · `A/win.rs:9493-9520` · 상수 `A/win.rs:123,127` | 호스트: `SetTimer`/`KillTimer` | A | (overlaybar·dock 테스트가 간접 검증) |
| GUI-004 | 정수 픽셀 기하 | `Rect{x,y,w,h}`: `right`·`bottom`·`size`·`is_empty`(w≤0 또는 h≤0) · `contains`(반개구간) · `intersects`(빈 rect·변 접촉 = false) · `union`(빈 rect = 항등원) | `G/geom.rs:3-77` | — | N | 4건(`contains_is_half_open` 등) |
| GUI-005 | 플랫폼 중립 입력 이벤트 | `InputEvent`: `Wheel{delta}`(양수=위) · `HWheel{delta}`(양수=오른쪽) · `Key{key,shift,ctrl}` · `Char{c,now_ms}`(`'\u{8}'`=Backspace) · `SelectAll` · `MouseDown{x,y,shift,ctrl}` · `RightDown{x,y}` · `MouseMove{x,y}` · `MouseUp{x,y}`. `Key` = Up/Down/PageUp/PageDown/Home/End/Right/Left/Space 9종 | `G/event.rs:22-85` | 호스트: WM_* 번역(`A/win.rs:7649-7662`) | A | — |
| GUI-006 | 트랙패드 분수 휠 누적 | `WHEEL_DELTA=120`. `WheelAccum::add(delta, lines_per_notch)` = 누적 후 `accum*lines/120` 줄 반환, 잔여 이월(손실 없음) | `G/event.rs:4` · `G/event.rs:87-103` | — | N | 3건(`fractional_notches_accumulate` 등) |
| GUI-007 | 시스템 "휠 한 칸 줄 수" 존중 | 프로세스 전역 `WHEEL_LINES_SYS`(기본 3). `set_wheel_lines(n)`: n≤0(페이지 단위 -1 포함) → 10 · 상한 20. 모든 스크롤 영역이 노치→줄/px 환산에 사용 | `G/event.rs:6-20` · 주입 `A/win.rs:6327-6344` · 기동 `A/win.rs:1425` · `WM_SETTINGCHANGE` `A/win.rs:9663-9664` | `SystemParametersInfoW(SPI_GETWHEELSCROLLLINES)` | P | — |
| GUI-008 | (의존 요약) 그리기 어휘·토큰 | 담당 위젯이 호출하는 `DrawCtx`: `select_font(slot,bold,italic)` · `fill_rect` · `text_opaque` · `text` · `text_width` · `push_clip`/`pop_clip` · `draw_image(rect, 경로)` · `draw_icon(x,y,size,key,hint)->bool` · `glyph_opaque`/`glyph_opaque_lg` · `fill_round_rect_alpha(rect,r,color,alpha u8)`. `FontSlot` = Base/List/Status. 쓰는 토큰 = `chrome_bg` `panel_bg` `tab_bar_bg` `header_bg` `field_bg` `status_bar_bg` `border` `accent` `text` `text_dim` `sel_bg` `sel_bg_inactive` `is_dark` | `G/draw.rs:11-127` · `G/theme.rs:25-58` | 백엔드 = GDI+DirectWrite(`A/dw.rs` — 범위 밖) | A | — |

### 1-2. 한 줄 편집 모델 — `edit.rs`(경로바 편집·인라인 리네임 공용)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-010 | 편집 시작 상태 | `EditState::new(text, select_all)`: 캐럿 = 끝 · `select_all && !빈값`이면 전체 선택. `with_selection_to(text, n)`: 앞 n자 선택 + 캐럿 n(리네임 이름부 선택) | `G/edit.rs:42-64` | — | A | `select_all_start_replaces_on_insert` · `delete_forward_and_stem_selection` |
| GUI-011 | 문자 입력·삭제 | `insert(c)` = 선택 있으면 대체 · `backspace()` = 선택 삭제 또는 캐럿 앞 1자 · `EditKey::DeleteForward` = 선택 삭제 또는 캐럿 뒤 1자 · `insert_str(s)`(붙여넣기 — 제어 문자 필터는 호출자). 버퍼는 `Vec<char>`(문자 단위 인덱스) | `G/edit.rs:136-183` · `G/edit.rs:210-215` | — | A | 위 2건 + `caret_moves_home_end_shift_selects` |
| GUI-012 | 캐럿 이동·선택 키 | `EditKey` = Left/Right/Home/End/SelectAll/DeleteForward. Shift = anchor 유지 확장. **비Shift ←/→ 중 선택이 있으면 선택 가장자리로 접기**. SelectAll = anchor 0 + 캐럿 끝(빈 버퍼면 anchor 없음) | `G/edit.rs:15-26` · `G/edit.rs:185-228` | — | A | `caret_moves_home_end_shift_selects` · `nonshift_left_collapses_to_selection_edge` |
| GUI-013 | 클립보드 연동용 API | `selected_text()` · `cut_selection()`(반환 후 삭제) · `delete_selected()`(메뉴 "삭제") · `has_selection()` · `is_empty()` · `text()` | `G/edit.rs:66-165` | 클립보드는 호스트(`A/clipboard.rs`) | A | `undo_restores_last_change_and_delete_selected` |
| GUI-014 | 실행 취소(단일 단계) | 내용이 바뀌는 조작 **직전** `(buf, caret, anchor)` 스냅샷 1개 보관(직전 것은 버림) · `undo()` 1회만 복귀 · `can_undo()` | `G/edit.rs:35-37` · `G/edit.rs:93-113` | — | A | `undo_restores_last_change_and_delete_selected` |
| GUI-015 | 마우스 캐럿 배치·드래그 선택 | paint가 캐시한 문자 경계 오프셋으로 최근접 경계 역참조. `hit(x,y)`(paint 전 = false) · `click(x)` = 캐럿+anchor 기록+드래그 시작 · `drag(x)` = 변화 시 true · `release()` = 이동 없었으면 선택 해제 | `G/edit.rs:230-284` | — | A | `click_places_caret_by_cached_layout` · `drag_selects_range_click_alone_does_not` |
| GUI-016 | 편집 필드 그리기 | `paint_field(ctx, rc, pad_x, theme)`: ① `field_bg` 채움 ② 선택 = `sel_bg` 사각(상하 1px 안쪽·좌우 클램프) ③ 텍스트 **1회 호출**(런 분할 금지) ④ 세로바 캐럿 1px(`text` 색, 상하 2px 안쪽) ⑤ accent 1px 테두리 4변. 오프셋 = **접두사 폭**(문자별 합산 금지). 넘치면 끝 정렬 + 캐럿이 항상 가시(`avail = w - pad*2 - 1`) · 좌측은 문자 경계로 잘라 그림 | `G/edit.rs:286-345` | — | A | `overflow_keeps_caret_visible_end_aligned` |
| GUI-017 | 버퍼 교체·IME 보조 | `set_text_end(text)` = 버퍼 통째 교체·캐럿 끝·선택 해제(**undo 스냅샷 없음**) · `text_before_caret()`(IME 조합 창 x 계산) | `G/edit.rs:70-73` · `G/edit.rs:167-173` | IME는 호스트(GUI-100) | A | (pathbar `suggest_cycle_and_restore_and_click` 간접) |

### 1-3. 고속 스크롤 — `fastscroll.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-020 | 고속 스크롤 설정(전역·핫스왑) | `FastScroll{enabled, step, max, window_ms, hud, hud_pos(0..8 행우선 3×3), hud_hold_ms, hud_fade_ms}`. **기본 = 켬 · step 3 · max 16 · window 160ms · hud 켬 · 위치 2(우상) · hold 250 · fade 600**. `set_fast_scroll`/`fast_scroll`(RwLock 전역 — 영역이 값을 들고 다니지 않음) | `G/fastscroll.rs:22-82` · 설정→값 `A/config.rs:212-224` · 적용 `A/win.rs:1422-1423` · `A/win.rs:6714-6715` | — | A | — |
| GUI-021 | 파일 그리드 "한 단계 더 빠르게" | `set_fast_scroll_grid(Option)` · `fast_scroll_grid()`(없으면 전역) · `grid_extra_of(base)` = `step-1`(최소 1) · `max×2` | `G/fastscroll.rs:84-112` · `A/config.rs:226-229` | — | A | `factor_grows_every_step_within_window_and_caps`(끝부분) |
| GUI-022 | 가속 배수 계산 | `ScrollAccel::factor_at(dir, now, cfg)`: dir=0 또는 꺼짐 → 리셋·1. 이전 사건과 **같은 방향 && 간격 ≤ window_ms**면 streak+1, 아니면 0. 배수 = `min(1 + streak/step, max)`. `reset()` | `G/fastscroll.rs:114-154` | `Instant` | N | `factor_grows_…` · `direction_change_or_gap_resets` |
| GUI-023 | `×N` 속도 배지 | `SpeedHud`: `note(k)` — k>1이면 표시·시각 갱신, k=1이면 그 시점부터 페이드. hud/enabled 꺼짐 = 즉시 소거. `alpha_at` = hold 동안 1.0 → `1 - t²` 감속 → fade 끝 None. `place` = 3×3 자리(여백 `pad`, 영역 좌상 클램프). `paint`: 캡슐 폭 = 글자폭 + `pad_x*2` · 높이 = `row_h` · 반경 h/2 · **accent 알파 `102×a`** · 글자는 a>0.35일 때 `theme.text` · a<0.08 미표시 · `drawn_rect()`로 무효화 영역 제공 | `G/fastscroll.rs:156-284` | `Instant` | A | `hud_holds_then_fades_and_clears` · `hud_placement_follows_3x3_index` |
| GUI-024 | 위젯용 묶음(가속기+배지) | `FastScroller::wheel(delta, units)`: units=0 → 0. **`\|delta\| < 120`(정밀 터치패드)은 배수 1 + 가속 리셋**(OS가 이미 가속·관성). 그 외 `units × k`(saturating). `key(dir)` → 배수(키 자동 반복). `tick(area, inv)` = 배지 rect 무효화 + 보이는 동안 틱 재요청. `paint(…, area, row_h, pad_x)`. `for_grid()` = 그리드 설정 사용. `hud_visible()` · `reset()` | `G/fastscroll.rs:286-372` | — | A | `scroller_ignores_sub_notch_trackpad_deltas` |

### 1-4. 오버레이 스크롤바 — `overlaybar.rs`(도크 Info/Preview 사용. rows.rs는 자체 내장 구현 — `G/widgets/overlaybar.rs:6-8`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-030 | 축 기하 공급 | `AxisGeom{view, content, visible, offset}`(i64 — 세로 = 행 단위·가로 = px 단위 혼용 가능). `scrollable()` = `visible>0 && content>visible && view 비어있지 않음`. `max_offset()` | `G/widgets/overlaybar.rs:23-52` | — | A | `thumb_none_when_content_fits` |
| GUI-031 | 썸 기하 | 두께 `BAR_THIN 6`/hot `BAR_WIDE 10` · `THUMB_MIN 24` · 길이 = `len×visible/content`(최소 24·최대 len) · 위치 = `(len-th)×off/max` · 세로 = 우측에서 2px 안쪽, 가로 = 하단에서 2px 안쪽 · 오프셋 초과값 클램프 | `G/widgets/overlaybar.rs:15-21` · `76-102` | — | A | `thumb_geometry_is_proportional_and_clamped` |
| GUI-032 | 스크롤 직후 표시 → 유지 → 페이드 | `flash(axis)` = 알파 120 + 유지 22틱(≈900ms@40ms) + 틱 요청. `tick` = 유지 소진 후 틱당 알파 −24(5틱) · 호버/드래그 중인 축은 페이드 보류 · **축별 독립** | `G/widgets/overlaybar.rs:126-152` | 틱 = GUI-003 | A | `flash_then_tick_fades_out_and_stops_requesting` |
| GUI-033 | 썸 호버 = 두껍게 | **보이는 축의 썸에만** 호버(숨겨진 바 자리에 올려도 드러나지 않음). 호버 = 두께 10 + 알파 210. 이탈 = `flash`로 유지→페이드 재개. 히트 여유 ±2px | `G/widgets/overlaybar.rs:112-120` · `154-198` | — | A | `hover_only_on_visible_bar_and_drag_maps_offset` |
| GUI-034 | 썸 드래그 스크롤 | `mouse_down` = 썸 위면 `(축, BarHit::Drag)` + 프레스 좌표·오프셋 기록. `mouse_move` 드래그 중 = `off0 + Δ×max_offset/(트랙−썸)` 클램프한 새 오프셋 반환. `mouse_up` = 종료 + flash, 소비 여부 반환 | `G/widgets/overlaybar.rs:164-182` · `200-249` | — | A | 위 테스트 |
| GUI-035 | 트랙 클릭 = 한 페이지 이동 | 썸 앞쪽 트랙 = `PageBack` · 뒤쪽 = `PageFwd`(드래그 시작 아님). 트랙 판정 = 세로 `x ≥ thumb.x−2` · 가로 `y ≥ thumb.y−2`. 바 밖 = None | `G/widgets/overlaybar.rs:54-63` · `219-236` | — | A | `track_click_pages_in_the_right_direction` |
| GUI-036 | 바 그리기 | 내용 위 마지막. 썸 = `theme.text` 라운드(반경 = 두께/2) 알파(hot 210 / 그 외 현재 알파). 드래그 중인 축은 트랙 음영(알파 28) | `G/widgets/overlaybar.rs:251-277` | `fill_round_rect_alpha` | A | — |

### 1-5. 탭 바 — `widgets/tabbar.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-040 | 탭 표시 | 배경 `tab_bar_bg` · 활성 탭 = `panel_bg` + **상단 2px 줄**(패널 포커스 = `accent`, 비포커스 = `border`) · hover = `header_bg` · 탭 폭 = `글자폭 + pad_x×2 + close_w`(`close_w = pad_x×3`) · 하단 1px `border` · `FontSlot::Base` | `G/widgets/tabbar.rs:286-388` · 상수 `63-67` | — | A | — |
| GUI-041 | 탭 전환 | 탭 본문 **MouseDown 즉시** `TabAction::Switch(i)` + 드래그 후보 기록 | `G/widgets/tabbar.rs:219-232` · 수거 `A/panel.rs:1509-1511` | — | A | `click_switches_close_zone_closes_and_plus_creates` |
| GUI-042 | 탭 닫기(×) | 탭 오른쪽 `pad_x×3` 존 MouseDown = `Close(i)`. **마지막 탭(1개) 또는 잠긴 탭은 닫기 대신 Switch** | `G/widgets/tabbar.rs:175-188` · `220-227` | — | A | 위 + `last_tab_close_zone_switches_instead` |
| GUI-043 | 새 탭 [+] | 마지막 탭 뒤 `"+"` 버튼(폭 = `"+"` 폭 + pad_x×2 · 줄 부족 시 다음 줄) MouseDown = `TabAction::New` | `G/widgets/tabbar.rs:190-196` · `233-236` · `362-378` | — | A | 위 |
| GUI-044 | 잠금 표지 | 잠긴 탭 제목 앞 `🔒` 접두 · × 자리는 공백(클릭은 전환). `set_locked(Vec<bool>)`(부족분 false · 변경 시만 무효화) | `G/widgets/tabbar.rs:97-103` · `303-316` · `345` | 이모지 글꼴 폴백 | A | — |
| GUI-045 | 고정(핀) 표지 | 고정 탭 제목 앞 `📌` 접두(잠금과 동시면 `🔒📌제목`). 핀 그룹 앞 정렬은 호스트 몫. `set_pinned` | `G/widgets/tabbar.rs:38-39` · `105-110` · `304-311` | 이모지 글꼴 폴백 | A | — |
| GUI-046 | 드래그 재정렬 | 프레스 후 **가로/세로 8px** 이동 시 시작. **대상 탭의 x 중간점을 통과한 순간에만** `Move{from,to}` 발화, 잡은 인덱스를 `to`로 갱신(연속 드래그 — 호스트가 `set_tabs`로 반영). MouseUp = 종료 | `G/widgets/tabbar.rs:244-281` · 상수 `66-67` | — | A | `drag_moves_across_lines` |
| GUI-047 | 멀티라인 탭 | 남은 폭에 안 들어가면 다음 줄(줄 첫 탭은 그대로). 한 줄 높이 = `set_line_height(h)`(0 = bounds 전체 단일 줄 폴백). paint가 필요 줄 수 측정 → `lines()` · 1회성 `take_lines_changed()` → 호스트 재레이아웃(1프레임 지연 수렴). bounds 아래로 넘친 줄은 그리지 않되 rect는 캐시 | `G/widgets/tabbar.rs:130-146` · `291-327` · `383-386` · 호스트 `A/win.rs:4941-4947` · `A/panel.rs:388` | — | A | `tabs_wrap_to_multiple_lines_and_report_count` |
| GUI-048 | 탭 우클릭 메뉴 | `RightDown` → `Context(i)` → 호스트 `pending_tab_menu` → **네이티브 팝업**: 잠금/잠금 해제 · 고정/고정 해제 · 복제 · 새 탭 · 닫기(잠김 또는 1개면 회색). State 참조 없이 표시 후 결과만 반영(재진입 규약) | `G/widgets/tabbar.rs:238-243` · `A/panel.rs:1515` · `A/win.rs:7115-7195` · `A/win.rs:8215-8220` | `CreatePopupMenu`/`TrackPopupMenuEx` | A | — |
| GUI-049 | 패널 간 탭 이동용 이양 API | `dragging()`(임계 통과분만) · `pressed_tab()`(후보 포함 — ESC 취소 스냅샷) · `cancel_drag()` · `begin_drag(index,x,y)`(호스트 주도 시작 — 임계 통과 취급) | `G/widgets/tabbar.rs:153-173` · 호스트 `A/win.rs:5969-5973` · `8079` · `8438-8460` · `8521-8545` | — | A | `host_drag_handoff_api` |
| GUI-050 | 더블클릭 라우팅용 히트 API | `tab_index_at(x,y)`(× 여부 무시) → 탭 더블클릭 = 설정 `tab_dblclick`(close 기본/pin/lock) · `empty_area_at(x,y)` → 빈 공간 더블클릭 = 새 탭 | `G/widgets/tabbar.rs:112-115` · `198-201` · 호스트 `A/win.rs:8649-8666` | 더블클릭 판정 = `WM_LBUTTONDBLCLK` | A | `tabs_wrap_…`(히트 부분) |
| GUI-051 | hover·포커스·지표 | MouseMove = hover 탭 추적(변경 시만 무효화). `set_focused(bool)`(패널 활성 표시). `set_metrics(row_h,pad_x)` · `set_tabs(titles, active)`(active 클램프 · hover 리셋) | `G/widgets/tabbar.rs:90-128` · `273-277` | — | A | `hover_tracks_tabs` |

### 1-6. 메뉴 바 — `widgets/menubar.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-060 | 메뉴 정의 모델 | `MenuItem{id:u32, label, shortcut(표시 전용), checked:Option<bool>, separator, submenu:Vec}` + 빌더 `new`/`checked`/`with_submenu`/`separator`. `Menu{title, items}`. **비활성(enabled) 항목 미지원**(클릭 시 상태바 안내로 대체 — `A/win.rs:403,454,523`) | `G/widgets/menubar.rs:14-65` | — | A | — |
| GUI-061 | 제목 스트립 | 배경 `chrome_bg` · 선두 여백 `pad_x` · 각 제목 폭 = 글자폭 + pad_x×2 · 열림 = `sel_bg` · hover = `header_bg` · 하단 1px `border` | `G/widgets/menubar.rs:304-330` | — | A | — |
| GUI-062 | 열기·닫기·전환 | 제목 클릭 = 토글(같은 제목 재클릭 = 닫기). **열린 상태에서 다른 제목 hover = 그 메뉴로 전환**. 외부 클릭 = 닫기. `close()`(Esc — 호스트 `A/win.rs:8811-8812`). `is_open()` | `G/widgets/menubar.rs:106-108` · `146-156` · `240-249` · `264-281` | — | A | `outside_click_closes_and_title_toggles` |
| GUI-063 | 드롭다운 표시 | 제목 x 아래(바 하단)에 오버레이. 폭 = max(라벨폭 + 단축키폭 + pad_x×6 + `✓`폭) · 행 높이 = `row_h` · 배경 `field_bg` · hover 행 `sel_bg` · 체크 열 `✓`(accent) · 라벨 x = `pad_x×2 + ✓폭` · 단축키 우측 정렬(`text_dim`) · 구분선 = 행 중앙 1px(좌우 pad_x 안쪽) · 1px `border` 테두리. **창 밖 넘침 보정 없음** | `G/widgets/menubar.rs:332-408` | — | A | — |
| GUI-064 | 항목 실행 | 항목 MouseDown = `take_command()`로 `id` 1회 통지 + 닫기. 구분선 클릭 = 명령 없이 닫기 | `G/widgets/menubar.rs:110-112` · `250-263` · 호스트 `A/win.rs:7973-7979` | — | A | `open_click_item_emits_command_and_closes` · `separator_click_closes_without_command` |
| GUI-065 | 하위 메뉴(1단계) | `submenu` 보유 항목 = 우측 `▸` · **hover 또는 클릭으로 오른쪽 플라이아웃 펼침**(클릭은 토글, 부모는 발화 안 함) · 플라이아웃 항목 클릭 = 발화 + 전체 닫기 · 드롭다운/플라이아웃 밖 hover는 펼침 유지. 중첩 미지원 | `G/widgets/menubar.rs:24-26` · `188-213` · `232-239` · `252-258` · `282-289` · `410-486` | — | A | `submenu_opens_on_parent_and_fires_on_sub_item` |
| GUI-066 | 체크 상태 갱신 | `set_checked(id, on)` — `checked.is_some()`인 항목만(하위 1단계 포함). 라디오 그룹은 호스트가 여러 id를 각각 갱신 | `G/widgets/menubar.rs:129-144` · 호스트 `A/win.rs:5056` · `5157-5158` · `5249-5250` · `5309` · `6569` | — | A | `set_checked_updates_toggle_items` · `set_checked_reaches_submenu_items` |
| GUI-067 | 메뉴 교체·지표 | `set_menus(menus)`(언어 전환 등 — 열린 드롭다운 닫기 + 캐시 초기화) · `set_metrics(row_h,pad_x)` | `G/widgets/menubar.rs:114-127` · 호스트 `A/win.rs:5579` · `5813` · `7728` · `9643` | — | A | — |
| GUI-068 | 첫 프레임 무효화 근사 | 열기/전환 시점엔 정확한 rect를 모른다(폭 = paint에서 측정) → `drop_area_estimate`(바 아래 전폭 × `항목 수×row_h+2`)를 무효화. 하위 플라이아웃도 보수적 추정(`drop.w.max(320)`) | `G/widgets/menubar.rs:168-175` · `200-213` | 부분 무효화 = GDI BitBlt 클립 | A | `open_invalidates_dropdown_area_on_first_frame` |

> 키보드 메뉴 탐색(↑↓←→·Enter·Alt 접근키)은 dir2에 **없다** — 위젯이 `Key` 이벤트를 처리하지 않고(`G/widgets/menubar.rs:229-302`), 호스트는 Esc 닫기만 한다(`A/win.rs:8811-8812`).

### 1-7. 도구 모음·상태바 — `widgets/chrome.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-070 | 도구 버튼 모델 | `ToolButton{id:u32, glyph, separator, checked, icon:Option<(key,hint)>, enabled, tip}` + 빌더 `new`/`sep`/`toggled`/`with_icon`/`enable`/`with_tip` | `G/widgets/chrome.rs:12-79` | — | A | — |
| GUI-071 | 버튼 배치 | 배경 `chrome_bg` · 하단 1px `border`. **글리프 폭 모드** = 선두 여백 `pad_x`, 버튼 폭 = 글리프폭 + pad_x×2 · **고정 폭 모드**(`with_button_width`) = bounds 시작부터 간격 없이 연속 · **아이콘 버튼** = 정사각(폭 = 바 높이) · 구분선 = 폭 `max(pad_x,4)`, 세로 1px(상하 3px 안쪽), 히트 없음 | `G/widgets/chrome.rs:96-126` · `243-273` | — | A | — |
| GUI-072 | 버튼 클릭 | **MouseDown 즉시** `take_command()`로 id 통지. 구분선·비활성 = 무시 | `G/widgets/chrome.rs:128-130` · `219-228` · 호스트 `A/win.rs:7981-7996` | — | A | `toolbar_click_emits_button_id` |
| GUI-073 | hover 강조 | hover 버튼 배경 = `sel_bg`(종전 `header_bg`는 명도 차 없어 식별 불가 — X-27). 비활성·구분선은 hover 없음. `hovered_id()` | `G/widgets/chrome.rs:183-186` · `229-238` · `283-286` | — | A | — |
| GUI-074 | 토글(켜짐) 표시 | `checked` = 배경을 **`chrome_bg`→`accent` 38% 블렌드**(라이트 ≈ #ABCAF9 · 다크 ≈ #2A4A7A) · 아이콘/글리프 색은 본문색 유지. `set_checked(id, on)` | `G/widgets/chrome.rs:132-140` · `273-282` · 호스트 `A/win.rs:5057` · `5159-5160` | — | A | — |
| GUI-075 | 활성/비활성 | 비활성 = 글리프 `text_dim` · 아이콘 키 `#dis`(알파 38%) · hover/클릭 무시. `set_enabled(id, on)` | `G/widgets/chrome.rs:142-150` · `290-295` · `312-316` | — | A | — |
| GUI-076 | 아이콘 버튼 그리기 | 아이콘 크기 `isz = max(바 높이−8, 8)`(도구 모음 28 → 20 · 런처 24 → 16), 셀 중앙. **키 변형** = `{key}[#dark][#dis]#RRGGBB`(잉크 = 테마 본문색 · `#dark` = 다크 변형 에셋 신호). `draw_icon`이 false(미로드/실패)면 **라벨 앞 2자 텍스트 폴백**(중앙) | `G/widgets/chrome.rs:296-337` | 백엔드 아이콘 큐(`A/dw.rs`·`A/icons` — 범위 밖) | A | — |
| GUI-077 | 글리프 버튼 그리기 | 고정 폭 모드 또는 **첫 글자가 MDL2 PUA(U+E700–U+F8FF)**면 `glyph_opaque`(셀 중앙), `with_large_glyphs()`면 `glyph_opaque_lg`. 그 외 = `text_opaque`(x = 셀+pad_x) | `G/widgets/chrome.rs:111-115` · `338-354` | Segoe MDL2 Assets 글꼴(Windows 전용) | P | — |
| GUI-078 | 툴팁 데이터·빈 영역 판정 | `hover_tip()` = `(id, 텍스트, 버튼 rect[클라이언트])`(구분선·빈 팁 = None) — **표시·타이밍은 호스트**(GUI-150/151). `is_button_at(x,y)`(구분선 제외) → 빈 영역 우클릭 = 도구 모음 편집 팝업 | `G/widgets/chrome.rs:177-203` · 호스트 `A/win.rs:8163-8170` | — | A | — |
| GUI-079 | 버튼 목록 교체·지표 | `set_buttons`(언어 전환·순서 변경·토글 반영 — hover/pending 리셋) · `set_button_width(Option)` · `set_metrics` | `G/widgets/chrome.rs:123-126` · `152-165` · 호스트 `A/win.rs:5231` · `5279` · `5311` · `5599` · `6598` · `6646` · `9006` | — | A | — |
| GUI-080 | 상태바 | 좌(본문색)·우(`text_dim`, 우측 정렬 — `x = max(right−pad_x−폭, x+pad_x)`) 텍스트 표시 전용. 배경 `status_bar_bg` · **상단** 1px `border` · `FontSlot::Status`. `set_text(l, r)` = 변경 시에만 무효화. 입력 이벤트 무시 | `G/widgets/chrome.rs:364-447` · 호스트 `A/win.rs:5010` · `7997-7999` | — | A | `statusbar_set_text_invalidates_only_on_change` |

### 1-8. 경로 바 — `widgets/pathbar.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-090 | 경로 → 세그먼트 분해 | `split_path`: `\`·`/` 모두 구분자, 빈 조각 제거, **조립은 `\`**. 첫 조각이 `:`로 끝나면(드라이브) full = `C:\`. UNC·VFS 스킴 미지원(후속 표기) | `G/widgets/pathbar.rs:16-52` | Windows 경로 문법 가정 | P | `split_drive_and_folders` |
| GUI-091 | 브레드크럼 표시 | 배경 `chrome_bg` · 세그먼트 좌우 여백 `SEG_PAD 2` · 구분자 `\`(`text_dim`) · **마지막(현재) = `text`, 나머지 = `text_dim`** · hover(클릭 가능 세그먼트만) = `sel_bg` · 하단 1px `border` · x 범위 캐시(히트) | `G/widgets/pathbar.rs:471-572` | 구분자 문자 | A | `hover_tracks_clickable_segments_only` |
| GUI-092 | 긴 경로 끝 정렬 | 총폭 > 가용(`w − SEG_PAD×2`)이면 **뒤(최근 폴더)부터 채워** 시작 인덱스 결정, 앞은 `…` + 구분자(클릭 불가 — 범위 캐시는 빈 항목으로 인덱스 정렬 유지). 최소 마지막 세그먼트는 그림 | `G/widgets/pathbar.rs:490-533` | — | A | — |
| GUI-093 | 세그먼트 클릭 이동 | 좌클릭 = `take_navigation()`로 그 세그먼트 full 경로 1회 통지. **마지막(현재) 세그먼트는 무동작·hover 없음**. 위젯은 파일시스템을 모른다(검증·이동은 호스트) | `G/widgets/pathbar.rs:115-118` · `398-408` · `434-440` · `454-460` | — | A | `segment_click_requests_navigation_but_last_is_inert` |
| GUI-094 | 편집 모드 진입·종료 | **우클릭(바 안, 비편집 시)** = 편집 진입, 현재 경로 **전체 선택**. `cancel_edit`(Esc·포커스아웃 — 입력 무시). `submit_edit`(Enter — trim 후 비어 있지 않으면 이동 통지). `set_path`(호스트 이동 완료) = 브레드크럼 갱신 + 편집 종료 | `G/widgets/pathbar.rs:120-161` · `441-445` · 호스트 `A/win.rs:8706-8716` · `8016-8019` | — | A | `right_click_edits_enter_submits_esc_cancels` · `set_path_rebuilds_and_exits_edit` · `edit_starts_fully_selected_and_click_places_caret` |
| GUI-095 | 편집 키 입력 | `edit_char(c)`(`\u{8}`=Backspace · 제어 문자 무시) · `edit_key(k, shift)`(←→·Home·End·Delete·Ctrl+A). 호스트 매핑: Enter=제출 · Esc=팝업 닫기 또는 취소 · ↑/↓=제안 이동 · Ctrl+C/X/V/Z · Delete 뒤 제안 갱신 | `G/widgets/pathbar.rs:321-340` · 호스트 `A/win.rs:8706-8738` · `A/win.rs:7636-7647` · `A/win.rs:9383-9385` | VK 코드·`WM_CHAR` | A | 위 3건 |
| GUI-096 | 편집 중 마우스 | 필드 안 MouseDown = 캐럿 배치 · MouseMove = 드래그 선택 · MouseUp = 종료. **필드 밖 클릭 = 호스트가 편집 취소**. 필드 더블클릭 = 전체 선택(호스트) | `G/widgets/pathbar.rs:426-433` · `447-466` · 호스트 `A/win.rs:8008-8019` · `8638-8647` | — | A | `edit_starts_fully_selected_and_click_places_caret` |
| GUI-097 | 편집 클립보드·실행 취소·삭제 | `edit_selected_text`(복사) · `edit_cut` · `edit_paste(s)`(호스트가 첫 줄만·제어 문자 제거 — `paste_line`) · `edit_undo` · `edit_delete` · `edit_menu_state()` = (undo 가능, 선택 있음, 비어 있음) · `edit_hit(x,y)` | `G/widgets/pathbar.rs:342-396` · 호스트 `A/win.rs:7246-7286` · `A/win.rs:7623-7633` | 클립보드 = OS | A | — |
| GUI-098 | 편집 필드 우클릭 메뉴 | 편집 필드 우클릭 **뗌**에 네이티브 팝업 6항목: 실행 취소 / ― / 잘라내기 · 복사 · 붙여넣기 · 삭제 / ― / 전체 선택(활성 = 표시 시점 상태). **편집을 시작시킨 첫 우클릭은 메뉴 억제**(`rclick_began_edit`) — 메뉴는 둘째 우클릭부터 | `A/win.rs:7460-7532` · `A/win.rs:8206-8209` · `8250-8255` | `TrackPopupMenuEx` | A | — |
| GUI-099 | 자동완성 팝업(PATH-SUG) | 호스트가 텍스트 변경마다 `set_suggestions(items)`(빈 목록 = 닫기 · 조회 시점 입력을 `base`로 보관). 팝업 = 바 아래 **전폭**, 높이 `n×row_h`, **하한 = `overlay_bottom`(리스트 바닥)**, 넘치는 항목 생략 · 배경 `panel_bg` + 1px `border` · 선택 행 `sel_bg`. `suggest_move(±1)`: 처음 ↓ = 1번째 · 선택 항목을 편집기에 미리 채움 · **첫 항목(또는 미선택)에서 ↑ = 선택 해제 + 조회 시점 입력 복원**. `suggest_click` = 그 경로로 **즉시 제출**. Esc = 팝업만 닫기(열려 있을 때). 패널 paint **마지막**에 `paint_suggest` | `G/widgets/pathbar.rs:54-59` · `163-319` · 호스트 `A/win.rs:7199-7206` · `A/panel.rs:415` · `A/panel.rs:492` · `A/win.rs:8000-8007` · `A/win.rs:8717-8720` | — | A | `suggest_cycle_and_restore_and_click` |
| GUI-100 | IME 조합 창 위치 | `edit_info()` = (캐럿 앞 텍스트, 필드 rect, pad_x) → 호스트가 캐럿 x 계산 후 조합 창 배치(`x = min(caret_x, right−pad)`, `y = field.y+2`) | `G/widgets/pathbar.rs:107-113` · 호스트 `A/win.rs:4792-4816` | `ImmGetContext`/`ImmSetCompositionWindow` | P | — |
| GUI-101 | 제출 파이프라인 | `take_navigation()` → ① 클라우드 표시 경로면 센티널로 환원 후 이동 ② `pathinput::expand_env` ③ (Windows만) `shellpath::resolve` ④ `navigate_to`. Enter 제출 후 `finish_input`(경로 변경 후처리) | `A/panel.rs:1518-1532` · `A/win.rs:8732-8733` | ③ = Win32 | P | — |

### 1-9. 하단 도크 — `widgets/dock.rs`(`InfoDock`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-110 | 종류 스트립(정보 \| 미리보기 \| 터미널) | 상단 1px `border` 아래 높이 `row_h` 스트립(배경 `header_bg`). 라벨 = 선두 `pad_x`, 각 폭 = 글자폭 + pad_x×2, 라벨 사이 간격 `pad_x`. 활성 = `sel_bg`(패널 비포커스면 `sel_bg_inactive`) · 비활성 = `text_dim`. **클릭 = 종류 전환**(선택 해제) — 내용 공급은 호스트(`active_kind()` 0/1/2). `set_kinds`(i18n) | `G/widgets/dock.rs:16-24` · `464-476` · `551-598` · `633-660` · 호스트 `A/win.rs:2486-2526` | — | A | — |
| GUI-111 | 터미널 "폴더로 이동"(→) | **마지막 종류 옆에 부착**(종류가 2개 이상일 때) · 폭 = `→`폭 + pad_x×2 · 활성+포커스 = `accent` 배경. 클릭 = `take_goto()` 1회 통지 + 마지막 종류(터미널)로 전환 → 호스트가 cd 전송 | `G/widgets/dock.rs:28-31` · `459-462` · `577-594` · `638-647` · 호스트 `A/win.rs:8082` | — | A | — |
| GUI-112 | 텍스트 내용 표시 | `set_lines(lines)`(키 = 첫 줄) / `set_content(key, lines)`. 한 줄 = `row_h`, x = `pad_x`, 색 `text`, 배경 `panel_bg`. 내용 영역 = 스트립 아래(`content_rect()`). 동일 내용 = 무비용 | `G/widgets/dock.rs:478-542` · `818-957` | — | A | `set_lines_invalidates_only_on_change` |
| GUI-113 | 같은 대상 갱신 시 스크롤·선택 유지 | **키가 같고 기존 내용이 있으면** 스크롤은 새 상한으로 클램프만, 선택은 범위 안이면 유지. **키가 다르면** 선택·세로/가로 스크롤 리셋 + 두 축 바 flash. 호스트 키 = `"{종류}\|{대상 경로}"` | `G/widgets/dock.rs:71-74` · `503-542` · 호스트 `A/win.rs:2516-2522` | — | A | `same_key_update_keeps_scroll_and_selection` |
| GUI-114 | 이미지 미리보기 | `set_image(Some(경로))` = 라인 대신 이미지: 영역 = 좌우 `pad_x`·상하 2px 안쪽, `draw_image`(비율 유지 가운데 — 디코드·캐시는 백엔드) | `G/widgets/dock.rs:544-550` · `832-842` | 백엔드 WIC(범위 밖) | A | `select_all_text_covers_whole_content`(끝부분) |
| GUI-115 | 인라인 이미지(다이어그램) | 라인 `"\u{1}img\|<경로>"` = 이미지 시작, 뒤따르는 `"\u{1}pad"` 줄 수만큼 높이 예약(`row_h × k`). 선택·복사 제외(마커 줄은 빈 문자열) · 가로 스크롤 불변 | `G/widgets/dock.rs:77-82` · `859-885` · `272-279` | — | A | — |
| GUI-116 | 세로 스크롤 | **노치(`\|delta\| ≥ 120`)** = 줄 단위(`wheel_lines()` 줄/노치) × 고속 배수, 부분 오프셋 0으로 스냅. **노치 미만(터치패드)** = 픽셀 단위(`wheel_lines×row_h` px/노치 → 행 + `scroll_frac`). 상한 = `줄 수 − 가시 행 수`. 이미지 종류·빈 내용 = 무시. 호스트는 **내용 영역 hover일 때만** 라우팅(터미널 종류 제외) | `G/widgets/dock.rs:133-178` · `756-780` · 호스트 `A/win.rs:6166-6176` | 휠 delta 규약(120) | A | `wheel_scrolls_and_selection_maps_absolute_lines` · `trackpad_small_deltas_accumulate_into_scroll` · `trackpad_half_line_scrolls_by_pixels_and_notch_snaps` |
| GUI-117 | 가로 스크롤 | `HWheel`(Shift+휠·틸트 — 노치당 `row_h × wheel_lines` px) × 고속 배수. 상한 = `가시 행 최대 폭 + pad_x×2 − 폭`(paint 측정 전 = 0). 텍스트는 **보이는 첫 문자 경계부터** 그림. `push_clip(content_rect)` | `G/widgets/dock.rs:180-197` · `781-794` · `848-854` · `917-940` | — | A | `hwheel_scrolls_horizontally_and_hit_test_follows` |
| GUI-118 | 오버레이 스크롤바 연동 | 세로 = 행 단위·가로 = px 단위 `AxisGeom`. 썸 드래그 = 비례 스크롤(선택·hover 불변) · 트랙 클릭 = 세로 `가시 행−1` 줄 / 가로 `폭` px 페이지 이동. 내용 교체·스크롤 시 flash | `G/widgets/dock.rs:199-242` · `668-693` · `796-798` · `954` | — | A | `vertical_bar_drag_scrolls_without_selecting` |
| GUI-119 | 텍스트 드래그 선택(문자 단위) | MouseDown = 앵커(라인, 문자 경계 — paint 오프셋 캐시 역참조). **마지막 줄 아래 빈 영역 = 마지막 줄 끝 앵커**. MouseMove = 확장 · 내용 영역 위/아래 밖 = 1행씩 자동 스크롤 · 좌/우 가장자리 밖 = `row_h` px씩 가로 자동 스크롤 · 영역 밖은 첫/끝으로 클램프. MouseUp = 이동 없었으면 선택 없음. 하이라이트 `sel_bg` | `G/widgets/dock.rs:50-58` · `250-339` · `671-679` · `702-754` · `807-812` · `901-916` | — | A | `drag_selects_char_region_and_click_alone_clears` · `drag_below_content_autoscrolls` · `drag_from_empty_area_anchors_at_end_of_last_line` |
| GUI-120 | 선택 복사·전체 선택 | `selected_text()` = 여러 줄은 **`\r\n`** 결합(마커 줄 제외). `select_all_text()` · `text_selectable()`(이미지·빈 내용·마커뿐 = false) · `clear_text_selection()`(다른 영역 클릭 시 호스트) · `content_hit(x,y)` | `G/widgets/dock.rs:263-301` · `414-448` · 호스트 `A/win.rs:7331-7341` · `A/panel.rs:1466-1468` | 줄바꿈 규약·RTF 동시 게시(`write_text_rich`) | P | `select_all_text_covers_whole_content` |
| GUI-121 | "크게"(↗) 버튼 | `set_popout(on)`(호스트가 **미리보기 종류일 때만** 켬). 내용 우상단 정사각 `side = row_h+4`(여유 부족 시 축소) · x = `right − side − pad_x` · y = 내용 top+2. 배경: pressed = `panel_bg`→accent 38% > hover = `sel_bg` > 기본 `header_bg`. 아이콘 `emb:popout[#dark]#RRGGBB`(크기 `max(side−8,8)`), 폴백 `↗` 글리프. **누름 = 아이콘 1px 우하 이동 · 버튼 안에서 뗄 때만** `take_popout()` 발화. **오버레이 바보다 먼저 판정** | `G/widgets/dock.rs:40-49` · `341-412` · `661-667` · `694-701` · `799-806` · 호스트 `A/win.rs:2524-2526` · `8565` | — | A | `popout_button_hover_press_release_semantics` · `popout_click_wins_over_flashed_bar` |
| GUI-122 | 패널 포커스 표시 | `set_focused(bool)` — 활성 종류·→ 버튼 강조색만 바뀜(비포커스 = 무채색). 호스트가 터미널 키 포커스와 동기 | `G/widgets/dock.rs:450-457` | — | A | — |
| GUI-123 | 호스트 직접 렌더 지원(터미널) | `content_rect()` = 스트립 아래 영역(호스트가 터미널 셀 그리드를 직접 그림 — 종류 2). `paint_strip()` = 부분 행이 스트립 위로 번졌을 때 스트립 재도장 | `G/widgets/dock.rs:478-487` · `600-613` · 호스트 `A/win.rs:4842-4884` | — | A | — |
| GUI-124 | 틱·`×N` 배지 | `tick(inv)` = 오버레이 바 페이드 + 속도 배지. 배지는 내용 영역 우상(설정 위치)에 paint 마지막 | `G/widgets/dock.rs:244-248` · `775-778` · `955-956` · 호스트 `A/win.rs:9500-9501` | — | A | `vertical_bar_drag_…`(틱 부분) |
| GUI-125 | 도크 텍스트 우클릭 메뉴 | Info/Preview 내용 우클릭 = 네이티브 팝업: 복사(선택 있을 때) / ― / 전체 선택(선택 가능한 텍스트가 있을 때). 터미널 종류는 별도 메뉴(터미널 문서) | `A/win.rs:7480-7489` · `7533-7540` · `7588-7604` | `TrackPopupMenuEx` | A | — |

### 1-10. 앱 쪽 — `pathinput.rs` · `shellpath.rs` · `tip.rs`

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|:--:|---|
| GUI-130 | 경로 입력의 환경변수 확장 | `expand_env`: ① trim ② 감싸는 `"…"`/`'…'` 제거 ③ `${env:NAME}` ④ `$env:NAME`(이름 = `[A-Za-z0-9_]+`) ⑤ `%NAME%`(`%%`는 원문 유지) ⑥ trim. 토큰은 대소문자 무시 · **미정의 변수는 원문 유지**(경로 검증에서 자연 실패) | `A/pathinput.rs:10-32` · `49-121` | `std::env::var`(OS 무관) · 문법은 CMD/PowerShell | P | `expand_cmd_ps_quotes_and_undefined` |
| GUI-131 | 비ASCII 대소문자 변환 panic 방지 | `find_ascii_ci` = **바이트 단위 길이 보존 비교**(`to_lowercase()`는 `İ`·`K`(U+212A)·`ǅ`에서 길이가 바뀌어 문자 경계 panic — G13-01). 반환 오프셋은 원문 기준 | `A/pathinput.rs:34-47` | — | N | `expand_env_non_ascii_case_change_no_panic` · `find_ascii_ci_offsets_are_in_haystack` |
| GUI-132 | 폴더 자동완성 제안 | `suggest_folders(text, enum_dirs, max)`: 마지막 구분자(`\` 또는 `/`)에서 "베이스 + 접두사"로 분해 → 베이스의 하위 폴더 중 **이름이 접두사로 시작(대소문자 무시)**하는 전체 경로 최대 `max`(호스트 = 20). 구분자 없음(`C:`)·빈 입력·열거 실패 = 빈 목록 | `A/pathinput.rs:123-149` · `165-172` · 호스트 `A/win.rs:7199-7206` | — | N | `suggest_splits_base_and_prefix_ci` |
| GUI-133 | 실제 폴더 열거자 | `fs_dirs(base)` = `read_dir` 중 디렉터리만 전체 경로(오류 = 빈 목록). **UI 스레드 동기 호출** · 정렬 없음(OS 열거 순) · 숨김 필터 없음 | `A/pathinput.rs:151-163` | `std::fs`(OS 무관) | N | — |
| GUI-140 | `shell:` 특수 폴더 스킴 | `shell:startup` · `shell:common startup` · `shell:downloads` · `shell:::{GUID}` 등 탐색기가 받는 전체 이름. 접두 판정 = `trimmed.get(..6)`(**바이트 슬라이스 금지 — 한글 경로 crash 수정 09-22**) · 미지 이름·FS 경로 없는 가상 폴더 = **원문 그대로**(열기 실패 = 위치 유지 격리). UI 스레드(COM 초기화됨) 전제 | `A/shellpath.rs:16-44` | `SHParseDisplayName` · `SHGetPathFromIDListEx` · `CoTaskMemFree` | P | `resolves_known_shell_names` · `passthrough_and_failure_keep_input` · `multibyte_prefix_does_not_panic` |
| GUI-150 | 툴팁 팝업 창 | `tip::show(owner, sx, sy, text, font, style)` = 별도 팝업 창(`WS_POPUP` + `TOOLWINDOW\|NOACTIVATE\|TOPMOST`) · 크기 = 글자 측정 + 여백 `PAD_X 8`/`PAD_Y 4` · 배경·1px 테두리·단일 행 세로 중앙 · **빈 텍스트는 표시 안 함**(`DrawTextW` AV 방지) · `x = max(sx,0)`(화면 우/하 넘침 보정 없음) · 폰트는 창이 소유(`WM_NCDESTROY`에서 해제). `tip::hide` | `A/tip.rs:27-161` | `CreateWindowExW` · GDI `DrawTextW`/`FillRect`/`FrameRect` | A | — |
| GUI-151 | 툴팁 hover 타이밍 | 전역 **도구 모음만** 대상(런처·네비 버튼 없음). MouseMove에서 `hover_tip()` 버튼이 바뀌면 무장(250ms 틱) → **2틱(500ms)** 뒤 버튼 하단 +2px에 표시 · 틱마다 커서가 버튼 화면 rect 안인지 실측(창 밖 이탈 대비) · 이탈/버튼 변경/클릭 = 즉시 파괴. 색 = `text`/`chrome_bg`/`border` | `A/win.rs:86-90` · `5842-5908` · `7962` | `SetTimer` · `GetCursorPos` · `ClientToScreen` | A | — |

**기능 수 = 95**(GUI-001~008 8 · 010~017 8 · 020~024 5 · 030~036 7 · 040~051 12 · 060~068 9 · 070~080 11 · 090~101 12 · 110~125 16 · 130~133 4 · 140 1 · 150~151 2).

---

## 2. 화면·컨트롤 배치

모든 px는 **96dpi 기준 논리값**이며 dir2는 `s(v) = v × dpi / 96`으로 물리 px를 만들어 위젯에 넘긴다(`A/win.rs:1346` · `1850`). 위젯은 DPI를 모른다.

### 2-1. 공통 지표(`PanelMetrics`)

| 지표 | 값 | 근거 |
|---|---|---|
| `row_h` 행 높이 | `s(20)`(최소 14) | `A/win.rs:1348` |
| `pad_x` 좌우 여백 | `s(6)` | `A/win.rs:1349` |
| `indent_w` | `s(16)` | `A/win.rs:1350` |
| `tab_h` 탭 한 줄 높이 | `s(22)` | `A/win.rs:1351` |
| `bar_h` 네비/경로 바 높이 | `s(24)` | `A/win.rs:1352` |
| 텍스트 세로 위치 | `ty = y + (h − h×4/5) / 2`(모든 위젯 공통 근사) | 예: `G/widgets/menubar.rs:307` |

### 2-2. 메인 창(위 → 아래) — `A/win.rs:1848-1934`

```
y=0      ┌────────────────────────────────────────────────────────────┐
         │ 메뉴 바  h=22   [File][Edit][View][Cloud][Help]              │  MenuBar
y=22     ├────────────────────────────────────────────────────────────┤
         │ 도구 모음 h=28  아이콘 20px 정사각 버튼(28×28) + 구분선        │  Toolbar
y=50     ├────────────────────────────────────────────────────────────┤
         │ 퀵 런처 바 h=24(숨김·항목 0이면 h=0) 아이콘 16px(24×24)        │  Toolbar
top      ├───────────────────────────┬─┬──────────────────────────────┤
         │ 좌 패널                    │ │ 우 패널                       │
         │  탭 바  h=22×줄 수         │스│  (동일)                       │
         │  [홈][←][→][↑] 경로 바 h=24│플│                              │
         │  파일 목록(잔여)           │리│                              │
         │                           │터│                              │
band_y   ├───────────────────────────┴─┴──────────────────────────────┤  가로 분리선 h=gap(3)
dock_y   │ 좌 도크(스트립 1+20, 내용)   │ │ 우 도크                      │  InfoDock ×2
bottom   ├────────────────────────────────────────────────────────────┤
         │ 상태바 h=22   좌: 항목/선택/탭        우: 필터·관측(dim)      │  StatusBar
         └────────────────────────────────────────────────────────────┘
```

| 영역 | 위치·크기 | 근거 |
|---|---|---|
| 메뉴 바 | `(0, 0, W, s(22))` | `A/win.rs:1858-1860` |
| 도구 모음 | `(0, 22, W, s(28))` — 아이콘 = 높이−8 = 20 | `A/win.rs:1861-1862` · `G/widgets/chrome.rs:302` |
| 퀵 런처 바 | `(0, 50, W, s(24) 또는 0)` — `launcher_visible && 실행 항목 ≥ 1`일 때만 | `A/win.rs:1863-1871` |
| 상태바 | `(0, H − s(22), W, s(22))`(`bottom = max(H−22, top)`) | `A/win.rs:1873-1875` |
| 패널 스플리터 x | `W × split` 클램프 `[min(s(200), W/2), max(W−s(200), W/2)]` | `A/win.rs:1840-1845` · `MIN_PANEL` `A/win.rs:205` |
| 스플리터 두께 `gap` | `s(3)`(최소 2) — 파일 좌/우·도크 좌/우·가로 분리선 **통일** · 히트 반폭 `SPLIT_HALF 3` | `A/win.rs:53` · `206` · `1879` |
| 좌 패널 | `(0, top, sx − gap/2, ph)` | `A/win.rs:1897` |
| 우 패널 | `(sx − gap/2 + gap, top, W − sx + gap/2 − gap, ph)` · 싱글 패널 = 좌 전폭, 우 = 0-rect | `A/win.rs:1892-1902` |
| 도크 밴드 높이 | 도크 표시 시 `area_h × dock_ratio` 클램프 `[min(row_h×3, area_h/2), area_h/2]` · 아니면 0 | `A/win.rs:1884-1891` |
| 가로 분리선 | `(0, dock_y − gap, W, gap)` — 평시 `text_dim` · 높이 드래그 중 `accent` | `A/win.rs:4917-4923` |
| 도크 좌/우 | `dsx = W × dock_split` 클램프 `[W/8, 7W/8]` · 좌 `(0, dock_y, dsx − gap/2, dock_h)` · 우 `(dsx − gap/2 + gap, …)` · 싱글 정보 = 좌 전폭 1개 | `A/win.rs:1903-1933` |
| 스플리터 색 | 평시 `border` · 드래그 중 `accent` · **폭이 양수일 때만 그림**(싱글 패널 음수 폭 방어) | `A/win.rs:4886-4916` |
| 스플리터 자석 스냅 | 창 50%·반대편 구분선에 `s(20)` 이내 = 정렬 · **Alt = 스냅 해제** | `A/win.rs:55` · `6147-6164` |

### 2-3. 패널 내부 — `A/panel.rs:386-416`

| 컨트롤 | 위치·크기 | 근거 |
|---|---|---|
| 탭 바 | `(x, y, w, tab_h × lines)`(패널 높이로 클램프) | `A/panel.rs:388-391` |
| 네비 버튼 | `(x, y+탭, nav_btn_w×4, bar_h)` — `nav_btn_w = row_h + pad_x`(=26) · 순서 **[홈 U+EA8A][← U+E72B][→ U+E72A][↑ U+E74A]** · 고정 폭 + 큰 글리프(13 DIP) · 간격 없이 연속 | `A/panel.rs:394-396` · `1546-1560` |
| 경로 바 | 네비 버튼 **바로 오른쪽에 붙여**(틈 0) `(x+nav_w, y+탭, w−nav_w, bar_h)` | `A/panel.rs:397-405` |
| 파일 목록 | 나머지 전부(도크는 패널 밖 전폭 밴드) | `A/panel.rs:406-413` |
| 자동완성 팝업 하한 | 리스트 바닥(`list_y + list_h`) | `A/panel.rs:415` |
| 진행 배지(클라우드) | 목록 우하단 12px 안쪽, 높이 22, 여백 10/5, `sel_bg` + 1px accent 테두리(목록 폭<80 또는 높이<24면 생략) | `A/panel.rs:495-517` |

### 2-4. 위젯 내부 배치 요약

| 위젯 | 배치 규칙 | 근거 |
|---|---|---|
| 메뉴 바 | 제목: x 시작 = `pad_x`, 폭 = 글자폭 + `pad_x×2`, 연속 배치. 드롭다운: 제목 x · 바 하단에서 시작 · 행 `row_h` · 열 = `[pad_x][✓][pad_x][라벨] … [단축키][pad_x]` | `G/widgets/menubar.rs:311-398` |
| 탭 | `[pad_x][🔒📌제목][pad_x]` + 닫기 존 `pad_x×3`(× 글자는 존 시작 + pad_x) | `G/widgets/tabbar.rs:296-348` |
| 도구 모음 | 선두 `pad_x`(글리프 폭 모드) · 버튼 간 간격 0 · 구분선 폭 `max(pad_x,4)` | `G/widgets/chrome.rs:250-264` |
| 경로 바 | 선두 `SEG_PAD 2` · `[2][라벨][2]\[2][라벨][2]…` · 넘치면 `…\`로 시작 | `G/widgets/pathbar.rs:485-568` |
| 편집 필드 | 텍스트 x = `rc.x + pad_x − dx` · 클립 = 필드에서 1px 안쪽 | `G/edit.rs:310-333` |
| 도크 | `y`: 경계선 1px · `y+1`: 스트립 `row_h` · 그 아래 내용. 스트립 라벨 `[pad_x]〈[pad_x]라벨[pad_x]〉[pad_x]〈…〉〈→〉` | `G/widgets/dock.rs:553-598` · `824-830` |
| 상태바 | 좌 x = `pad_x` · 우 = 우측 정렬(오른쪽 여백 `pad_x`) | `G/widgets/chrome.rs:420-446` |

### 2-5. 그리기 순서(Z) — `A/win.rs:4832-4931` · `A/panel.rs:480-493`

1. 좌 패널: 목록 → 도크 → 네비 버튼 → 경로 바 → 탭 바 → 진행 배지 → **자동완성 팝업**
2. 우 패널(폭 > 0일 때 — 동일 순서)
3. 싱글 정보 공유 도크 재도장(우 목록 부분 행 침범 방지)
4. 터미널 내용(종류 2) → 부분 행 번짐 시 스트립 재도장
5. 패널 스플리터 · 도크 스플리터 · 가로 분리선
6. 도구 모음 → 퀵 런처 바 → 상태바
7. **메뉴 바(맨 마지막 — 드롭다운 오버레이가 모든 것 위)**

툴팁은 별도 OS 창이라 이 순서 밖이다(GUI-150).

### 2-6. 마우스 라우팅 우선순위(좌클릭) — `A/win.rs:7960-8030` · `A/panel.rs:1458-1505`

1. 툴팁 즉시 파괴 → 2. **메뉴가 열려 있거나 y < 도구 모음 y** = 메뉴 바 전용(명령 실행 후 종료) → 3. y < 패널 top = 도구 모음/런처(y로 분기) → 4. y ≥ 상태바 = 무시 → 5. 경로바 편집 중이면: 제안 팝업 클릭 → 필드 안 = 캐럿, 필드 밖 = 편집 취소 → 6. 도크 가로 분리선(높이 드래그) → 7. 스플리터 → 8. 패널(`Panel::on_event` y 라우팅: 탭 바 / 네비 버튼·경로 바 / 도크 / 목록).
- `MouseMove`는 패널의 **모든** 위젯에 전달(hover 갱신) — `A/panel.rs:1484-1492`. `MouseUp`도 탭 바·경로 바·목록·도크 전부에 전달(드래그 상태 잔존 방지) — `A/panel.rs:1493-1502`.
- 도크 라우팅은 **실제 표시 중(`dock_shown()` = 표시 플래그 && 높이 > 0)**일 때만 — `A/panel.rs:431-437` · `1461-1464`.

### 2-7. 단축키·탭 순서

- **탭(Tab) 포커스 순회 없음** — 담당 위젯들 사이에 키보드 포커스 이동 개념이 없다. Tab = 활성 패널 전환(`A/win.rs:5910` 주석 — 상세는 창/입력 문서).
- 메뉴에 **표시되는** 단축키 문구(실제 처리는 호스트 키 처리 — 다른 문서): `Ctrl+T` 새 탭 · `Ctrl+W` 탭 닫기 · `Ctrl+Shift+N` 새 폴더 · `Ctrl+,` 설정 · `Ctrl+Z`/`Ctrl+Y` · `Ctrl+X`/`C`/`V` · `Ctrl+A` · `Ctrl+Shift+R` 일괄 이름변경 · `Ctrl+H` 숨김 · `Ctrl+.` 닷파일 · ``Ctrl+` `` 도크 · `F5` 새로고침 · `F6` 테마 순환(`A/win.rs:396-483`).
- 경로바 편집 중 키(`A/win.rs:8706-8738`): `Enter` 제출 · `Esc` 팝업 닫기 → 편집 취소 · `↑`/`↓` 제안 이동 · `←`/`→`/`Home`/`End`(+Shift 선택) · `Delete` · `Backspace` · `Ctrl+A`/`C`/`X`/`V`/`Z`.
- 메뉴: `Esc` = 열린 드롭다운 닫기(`A/win.rs:8811-8812`). 그 외 키보드 탐색 없음.
- 마우스: 경로 바 **우클릭 = 편집 진입** · 탭 **더블클릭 = 설정 동작**(기본 닫기) · 탭 바 빈 곳 더블클릭 = 새 탭 · 편집 필드 더블클릭 = 전체 선택.

### 2-8. 메뉴·도구 모음 구성(배치 재현용 — 명령 의미는 창/명령 문서)

- 메뉴 5개: **File**(새 탭 · 탭 닫기 ― 새 폴더 · 새 파일 ― 설정 ― 종료) · **Edit**(실행 취소 · 다시 실행 ― 잘라내기 · 복사 · 붙여넣기 ― 전체 선택 ― 일괄 이름변경) · **View**(보기 모드 라디오 3 ― 패널 듀얼/싱글 · 정보 듀얼/싱글 · 컬럼 너비 동기화 ― 숨김 · 닷파일 · 도크 · 런처 · 항상 위 ― 새로고침 ― 테마 3 ― 언어 시스템 + 발견된 언어 목록) · **Cloud**(연결별 하위 메뉴[바로 가기 · 온라인 보기 · URL 복사 ― 연결 해제] ― 연결 추가하기[후보 하위] ― Connect Cloud[OAuth 서비스 하위]) · **Help**(About) — `A/win.rs:380-538`.
- 도구 모음 블록(순서 = 설정 `toolbar_order`, 블록 사이 구분선): `panel`(toggle · dock · ontop · info · colsync) · `view`(tree · flat · tiles) · `refresh` · `settings` · `show`(hidden · dot · foldersfirst). 전 버튼 = 임베드 SVG 아이콘(`emb:*`) + 폴백 글리프 + i18n 툴팁 + 토글/활성 상태 — `A/win.rs:675-792`.
- 퀵 런처 바: 설정의 실행 항목마다 exe 셸 아이콘 16px 정사각 버튼 + 구분선(`launcherN=-`) — `A/win.rs:800-813`.
- 도크 종류 라벨 3개(i18n `dock.info`/`dock.preview`/`dock.terminal`) — `A/win.rs:2486-2489`.

---

## 3. nexa-ui 매핑

`U/controls/` 실재 확인 결과(Grep): `button` `carousel` `checkbox` `colorpanel` `colorpick` `combo` `ctxmenu` `editmenu` `flash` `glyphs` `icondrop` `listedit` `pairs` `posdrop` `posgrid` `pulldown` `radio` `scroll` `splitter` `switch` `tabbar` `textbox` `timeout_button` `toolbar` `tooldock` `tree`(`U/controls/mod.rs:16-74`).
nexa-ui 자체 현황: U-2 = "✅ tabbar(09-14) · ✅ menubar(pulldown) · ☐ dock"(`nexa-ui/docs/TODO.md:8`) · F-3 잔여 = "PathBar/FileTree 독립 컨트롤 승격 · 자동완성(dir2 `suggest_folders`)"(`nexa-ui/docs/TODO.md:21`).

### 3-1. 기반 타입

| dir2 | nexa-ui 대응 | 상태 | API 차이·필요 작업 |
|---|---|:--:|---|
| `Widget` trait(GUI-001) | `nexa_ctl::Widget` | 있음 | 시그니처 동일(`U/widget.rs:44-56`). 추가로 `Control`/`ControlBase`(bounds·scale·focused·active·enabled — `U/controls/mod.rs:226-411`) 위에 얹는 것이 nexa-ui 관례 |
| `Invalidations` rect(GUI-002) | `nexa_ctl::Invalidations` | 있음 | `push`/`is_empty`/`drain` 동일(`U/widget.rs:13-41`) |
| `Invalidations` 틱(GUI-003) | **없음** | 추가/대체 | nexa-ui는 컨트롤별 `tick(now_ms) -> bool`을 **호스트가 직접 호출**하는 모델(`ScrollBars::tick` `U/controls/scroll.rs:628` · `Splitter::tick` `U/controls/splitter.rs:131`). 결정 필요: (a) dir3 앱이 nexa-sql식 틱 루프를 쓰고 `request_tick` 호출부를 `tick() -> bool`로 바꾸거나 (b) nexa-ctl `Invalidations`에 `request_tick`/`take_tick` 추가(영향: nexa-sql) |
| `Point`/`Size`/`Rect`(GUI-004) | `nexa_ctl::geom` | 있음 | 동일 + `intersection` · `place_popup` · `place_popup_beside` · `nudge_into` · `popup_host`(`U/geom.rs:90-173`) |
| `InputEvent`/`Key`(GUI-005) | `nexa_ctl::event` | 있음(차이) | `ctrl` → **`primary`**(mac ⌘ / 그 외 Ctrl — `U/event.rs:60-69` · `84-94`). `Key`에 `Enter`·`Escape`·`Delete`·`WordLeft/Right`·`SubwordLeft/Right` 추가 · `Undo`/`Redo` 이벤트 추가(`U/event.rs:31-83`). 가운데 클릭은 이벤트에 없음(`TabBar::middle_down` 별도 호출) |
| `WheelAccum`/`WHEEL_DELTA`(GUI-006) | 동일 | 있음 | 코드 동일(`U/event.rs:8` · `118-134`) |
| `set_wheel_lines`/`wheel_lines`(GUI-007) | **없음** | 추가 필요 | nexa-ctl에 전역 `wheel_lines` 추가(또는 dir3 앱 전역). `ScrollBars`는 휠을 `delta/3` px로 고정 환산(`U/controls/scroll.rs:476`) — 줄 수 설정 미반영 |
| `DrawCtx`(GUI-008) | `nexa_ctl::DrawCtx` | 있음(어휘 차이) | ↓ 3-5 표 |
| `Theme` 토큰(GUI-008) | `nexa_ctl::Theme` | 있음(토큰 부족) | nexa-ctl에 **없는** 토큰: `tab_bar_bg` · `header_bg` · `bottom_dock_bg` · `status_bar_bg`(`U/theme.rs:12-57`). nexa-ctl에만 있는 것: `focus_ring` · `bubble_peer` · `syn_*` · `rainbow` · `danger`/`ok`/`warn`. 공통 토큰의 **값은 동일**(예: `chrome_bg` 0x1E2228 — `G/theme.rs:65` ↔ `U/theme.rs:65`). `Color`도 다름: dir2 `{r,g,b}` 구조체 ↔ nexa-gfx `Color(u32)`(`U/theme.rs:7`) |

### 3-2. 컨트롤

| dir2 위젯 | nexa-ui 대응 | 상태 | API 차이·필요 작업 |
|---|---|:--:|---|
| `TabBar`(GUI-040~051) | `nexa_ctl::TabBar`(`U/controls/tabbar.rs`) | **있음** | dir2에서 이식된 것(`U/controls/tabbar.rs:1`). 호스트가 쓰는 API 전부 존재: `set_tabs` `set_locked` `set_pinned` `set_metrics` `lines` `take_lines_changed` `take_action` `tab_index_at` `empty_area_at` `dragging` `cancel_drag` `pressed_tab` `begin_drag`. **차이**: ① `new()` 인자 없음 + `set_metrics`(논리 px, `set_scale`로 배율 — `U/controls/tabbar.rs:171` · `402`) ② 멀티라인은 **`set_multiline(true)` 필요**(기본 = 단일행 + ◀▶ 스크롤 — `U/controls/tabbar.rs:362`), `set_line_height` 없음(줄 높이 = `row_h`) ③ **전환은 MouseUp**(같은 탭 위에서 뗐을 때 — `U/controls/tabbar.rs:914-923`), 닫기도 프레스→릴리스(`924-931`) — dir2는 MouseDown 즉시 ④ 패널 포커스 = `Control::set_active`(비활성 줄 색 `text_dim` — dir2는 `border`) ⑤ 잠금 = 닫기 상자 자리에 자물쇠 글리프, 핀 = 6px 점(이모지 접두 아님 — `U/controls/tabbar.rs:651-669` · `1057-1063`) ⑥ 탭 폭 상한 `MAX_TAB_W 240`(긴 폴더 이름 잘림 — `U/controls/tabbar.rs:99`) · 닫기 상자 16px ⑦ 배경 `chrome_bg` + 상태 레이어(dir2 `tab_bar_bg`/`header_bg`) ⑧ 추가 기능: 드래그 고스트 · 가운데 클릭 닫기(`middle_down`) · 미저장 점 · 탭별 색 · 배지 · `set_close_last` · `set_show_new` |
| `MenuBar`(GUI-060~068) | `nexa_ctl::MenuBar`(`U/controls/pulldown.rs`) | **있음(기능 부족)** | 모델이 다르다: `MenuDef{label, entries}` + `MenuEntry::{Item(ComboItem), Emph, Disabled, Separator, Sub}`(`U/controls/pulldown.rs:25-73`), id = **String**(`take_picked() -> Option<String>` — `152`). **없는 것(추가 필요)**: ① **단축키 문구 열**(`ComboItem`에 필드 없음 — `U/controls/combo.rs:44-53`) ② **체크(✓)/라디오 표시**와 `set_checked(id, on)` ③ `set_metrics`(상수 `ITEM_H 26`·`SEP_H 7`·`LABEL_PAD 12`·`POPUP_PAD 4`·`POPUP_MIN_W 160` 고정 — `U/controls/pulldown.rs:76-80`). **더 나은 것**: 키보드 탐색(↑↓←→·Enter·Space·Esc — `521-582`) · 비활성 항목 · 하위 메뉴 표면 밖 뒤집기/`nudge_into` · 라벨 가운데 축약 · `dismiss()` · `popup_bounds()`. 참고: nexa-sql도 라벨만 쓴다(`S/nexa-sql/src/app/menus.rs:1039`). `ContextMenu`의 `CtxItem`에는 `shortcut`/`checked`/`mark`/`active`가 이미 있다(`U/controls/ctxmenu.rs:101-131`) → 같은 표현을 `MenuEntry`에 올리는 것이 자연스럽다 |
| `Toolbar`/`ToolButton`(GUI-070~079) | `nexa_ctl::Toolbar`/`ToolItem`(`U/controls/toolbar.rs`) | **있음(기능 부족)** | `ToolItem{id:String, icon:ToolIcon, right, visible, tip, enabled, tone, dropdown, separator, badge, label}`(`U/controls/toolbar.rs:93-121`). 대응: `set_item_enabled` · `set_item_tip` · `set_item_icon` · `set_item_visible` · `take_clicked` · `item_rect` · `set_icon_size` · `set_padding` · `preferred_height`(= 아이콘 + (slot_pad+bar_pad)×2 — `293-295`: 아이콘 20 + 여백 (2,2) = 28 · 아이콘 16 + (2,2) = 24로 dir2 높이 재현 가능). **없는 것(추가 필요)**: ① **토글(켜짐) 상태** `checked` 배경(accent 38% 블렌드)과 `set_item_checked` — `ToolTone`은 아이콘 색조일 뿐 ② 고정 버튼 폭 + 선두 여백 0 모드(네비 버튼) ③ 비동기 셸 아이콘(`draw_icon(key, hint)`) — nexa-ui는 호스트가 `ToolIcon::Image(Rc<IconImage>)`/`Mask`를 공급(런처 exe 아이콘은 `nexa_fs::shell::IconService` — `UF/shell.rs:108-323`) ④ `hover_tip()`(대신 `paint_tooltip`/`paint_tooltip_in` 내장 — `506-517`) ⑤ `is_button_at`(→ `item_rect`로 조립). 구분선 폭 = `SEP_W 9`(dir2 `max(pad_x,4)`) |
| `StatusBar`(GUI-080) | **없음** | **추가 필요** | `U/` 전체 Grep에 `StatusBar` 없음. 필요한 API: `new()` · `set_text(left, right, inv)`(변경 시만 무효화) · `Widget` 구현 · `FontSlot::Status` · 상단 1px 경계. 후속 확장 여지: 구획(segment) · 클릭 가능 항목 |
| `PathBar`(GUI-090~100) | **독립 컨트롤 없음** — nexa-dlg `FilePicker` 내부에 브레드크럼(`paint_crumbs` — `UD/lib.rs:872-930`, `crumbs()` `1204`) + `TextBox` 편집이 조립돼 있음 | **추가 필요**(승격) | nexa-ctl에 `PathBar` 추가: `new(path)` · `set_path` · `path` · `take_navigation` · `is_editing` · `begin_edit`/`cancel_edit`/`submit_edit` · 편집은 `TextBox` 위임(키·클립보드·IME·undo·우클릭 메뉴) · `set_suggestions`/`suggest_move`/`suggest_click`/`suggest_open`/`close_suggest`/`paint_suggest` · `set_overlay_bottom`(또는 `geom::popup_host` 규칙) · **세그먼터 주입**(OS별 구분자·루트 — GUI-090) · 오버플로 `…`. 팝업은 nexa-ui 팝업 배치 규칙(`nexa-ui/CLAUDE.md:44-50`)을 따라야 한다 |
| `EditState` + `paint_field`(GUI-010~017) | `nexa_ctl::EditState`(`U/edit.rs`) + **`TextBox`**(`U/controls/textbox.rs`) | **있음(상위 호환)** | `EditState::new(text, sel)` → `with_text(text, sel)`(`U/edit.rs:996`) · `with_selection_to` → `with_text` + `set_selection(0,n)`(`1521`) · `cut_selection` → `cut()`(`1486`) · `set_text_end` → `set_text`(`1531` — 히스토리 비움) · 히트/드래그/`paint_field`는 `EditState`에서 빠지고 **`TextBox`가 담당**(`U/edit.rs:8`). `TextBox`: `new(placeholder)` · `with_text` · `text` · `set_text` · `take_committed`(Enter) · `take_changed` · `copy_selection`/`cut_selection`/`paste` · `set_preedit`(IME) · `caret_point` · `take_edit_ctx`(우클릭 메뉴) · `set_char_filter` · `set_max_chars` · `set_focus_ring`(`U/controls/textbox.rs:812` · `2097-2102` · `2589-2600` · `3003-3063` · `3314-3410`). 실행 취소는 다단계 + 다시 실행(dir2 = 단일 단계) |
| `FastScroll`/`ScrollAccel`/`SpeedHud`(GUI-020~023) | `nexa_ctl::{FastScroll, ScrollAccel, SpeedHud}`(`U/controls/scroll.rs`) | **있음(차이)** | dir2가 여기서 이식해 온 것(`G/fastscroll.rs:1-2`). 차이: ① **부품 기본 = 끔 · step 5 · max 8**(`U/controls/scroll.rs:75-88`) — dir2 기본(켬·3·16)은 호스트가 `set_fast_scroll`로 주입 ② `hud_pos: HudPos` 열거(`U/typeahead.rs:180-191`) ↔ dir2 `u8` 0..8(변환표 필요 — 값 순서는 같은 3×3 행우선) ③ `SpeedHud::paint(ctx, theme, area, scale, cfg)`(글꼴 75% · 여백 6/3 · 가장자리 8 · 알파 `0.4×a` — `194-249`) ↔ dir2 `(…, area, row_h, pad_x, cfg)` ④ `drawn_rect()` 없음 ⑤ `SpeedHud::tick`이 hold 중엔 false(`180-191`) — dir2는 보이는 동안 true ⑥ 그리드 전용 = `ScrollBars::set_fast_override`(`358-362`) — dir2 `set_fast_scroll_grid` 전역 + `grid_extra_of` 없음(dir3 앱이 계산) |
| `FastScroller` 묶음(GUI-024) | **없음**(`ScrollBars`가 가속기+HUD를 내장 — `U/controls/scroll.rs:320-337` · `471-479`) | 추가/대체 | `ScrollBars::on_event`의 Wheel은 **노치 미만 delta에도 배수를 적용**한다(`474-476`) → dir2의 "정밀 터치패드 제외" 규칙(GUI-024)이 없다. 대안: (a) nexa-ctl에 `FastScroller` 이식(영향 없음 — 신규) (b) 호스트가 winit `PixelDelta`를 `Wheel` 이벤트로 넘기지 않고 오프셋에 직접 반영. 키 가속은 `ScrollAccel::factor` + `ScrollBars::note_fast(k)`(`371-374`) |
| `OverlayBars`(GUI-030~036) | `nexa_ctl::ScrollBars`(`U/controls/scroll.rs`) | **있음(동작 차이)** | 모델이 다르다: 오프셋 **px 단위 i32**(`content_w/h`, `off_x/y`) ↔ dir2 `AxisGeom`(i64 · 세로 = 행 단위). `on_event(ev, vp, cw, ch, ox, oy, scale) -> (ox, oy, consumed)`(`458-467`). 차이: ① 두께 6/**11** · `MIN_THUMB 28` · 알파 0.35/0.6 · 색 `text_dim`(dir2 6/10 · 24 · 120/210 · `text`) ② **숨김 = 벽시계 기반**(기본 2000ms, `set_hide_delay_ms` 전역, 페이드 없이 즉시 — `28-45` · `628-651`) ↔ dir2 틱 기반 유지 22틱 + 알파 페이드 ③ **트랙 클릭 = 그 자리로 점프 + 드래그 시작**(`508-537`) ↔ dir2 한 페이지 이동 ④ **가장자리 접근 = 바 깨움**(`564-575`) — dir2에 없음 ⑤ 드래그 중 트랙 음영 없음. 행 단위 스크롤 위젯(도크)은 `off_y = scroll×row_h + frac`로 환산해 넘기면 된다 |
| `InfoDock`(GUI-110~125) | **없음** — `ToolDock`은 **툴바 그룹 도크**로 전혀 다른 컨트롤(`U/controls/tooldock.rs:1-14`) | **추가 필요** | U-2의 남은 ☐ dock. nexa-ctl에 `InfoDock` 추가(dock.rs 1:1 이식 + `ScrollBars` 어댑터). 필요한 API: `new` · `set_kinds`/`active_kind` · `set_content(key, lines)`/`set_lines` · `set_image` · `set_popout`/`take_popout` · `take_goto` · `content_rect`/`paint_strip` · `selected_text`/`select_all_text`/`clear_text_selection`/`text_selectable`/`content_hit` · `set_focused` · `tick`. 이미지 = 경로 문자열이 아니라 **디코드된 `IconImage`**(`image_scaled` — `U/draw.rs:125`)를 받도록 바꿔야 한다(디코더 = `nexa-gfx/src/image.rs`·`jpeg.rs`). 대안(검토만): 텍스트부를 `TextBox::with_multiline` + `set_read_only(true)`로 대체 — 인라인 이미지 마커·키 유지 규칙을 따로 얹어야 해 1:1 이식보다 위험 |
| 스플리터(호스트 직접 그림 — `A/win.rs:4886-4923`) | `nexa_ctl::Splitter`(`U/controls/splitter.rs`) | **있음** | `new(SplitAxis)` · `set_rect` · `on_event -> SplitEvent::{Hover, Start, Drag(i32), End}` · `tick` · `paint` · `is_hover`(커서 모양은 호스트). dir2의 자석 스냅(50%·반대편 구분선·Alt 해제 — `A/win.rs:6147-6164`)과 최소 폭 클램프는 호스트가 `Drag(v)` 뒤에 적용. hover = 서서히 진해지는 accent 손잡이(dir2는 hover 표시 없음) |
| 탭/편집/도구 모음 우클릭 네이티브 팝업(GUI-048·098·125) | `nexa_ctl::ContextMenu`(`open_at`/`open_beside`/`take_picked`/`paint` — `U/controls/ctxmenu.rs:528-600`) · `EditMenu`(`U/controls/editmenu.rs`) | **있음(항목 부족)** | 탭 메뉴 = `CtxItem::item`/`maybe(id, label, enabled)`. 편집 메뉴: nexa-ui `EditMenu`는 **복사 · 잘라내기 · 붙여넣기 ― 전체 선택** 4종(`U/controls/editmenu.rs:96-124`) ↔ dir2 6종(**실행 취소**·**삭제** 추가 — GUI-098). → `EditMenu::open_at`의 `extra`로 끼우거나 `EditMenuAction`에 `Undo`/`Delete` 추가. `EditMenuCaps{has_sel, has_text, clip_has_text, read_only}`가 dir2 `edit_menu_state` + `clipboard::has_text`에 대응. 창 층에서 그려야 한다(`nexa-ui/CLAUDE.md:46-50`) |
| 툴팁(`A/tip.rs` — GUI-150/151) | `nexa_ctl::draw::draw_tooltip_in`(`U/draw.rs:240-290`) · `Toolbar::paint_tooltip(_in)` | **있음(방식 다름)** | 별도 OS 창이 아니라 **창 안 팝업 층에 그림**(역상 캡슐 · 여러 줄 · 표면 밖이면 위로 뒤집기). 창 밖으로 나갈 수 없다(창 상단 근처 버튼 = 아래로 그리므로 문제 없음). hover 지연 타이밍은 `Toolbar` 내부(추정 — `Toolbar`에 `Instant`·hover 상태 보유 `U/controls/toolbar.rs:20` · `235-239`; 500ms 재현 여부는 구현 단계에서 확인) |
| 진행 배지(`A/panel.rs:495-517`) | `nexa_ctl::Flash`(순간 메시지) 또는 직접 그리기 | 부분 | 상시 배지는 `Flash`(페이드 전용 — `U/controls/flash.rs`)와 성격이 달라 직접 그리기 유지가 단순(패널 문서 소관) |

### 3-3. 앱 로직

| dir2 | nexa-ui 대응 | 상태 | 차이·필요 작업 |
|---|---|:--:|---|
| `pathinput::expand_env`(GUI-130/131) | `nexa_fs::path::expand_env`(`UF/lib.rs:608-686`) · `nexa_fs::path::resolve`(`UF/lib.rs:580-606` — trim · 따옴표 · `~` · 환경변수 · `shell:` · 상대 경로) | **있음(차이)** | nexa-fs는 `Vec<char>` 인덱싱이라 GUI-131류 바이트 경계 panic이 구조적으로 없다. **차이**: nexa-fs는 sh식 **`$NAME`/`${NAME}`도 확장**(`UF/lib.rs:658-681`) — dir2는 `$env:`만. Windows에서 `$`로 시작하는 폴더명(`$Recycle.Bin` 등)은 같은 이름의 환경변수가 있을 때만 영향(미정의 = 원문 유지) |
| `pathinput::suggest_folders`/`fs_dirs`(GUI-132/133) | **없음**(TODO F-3 잔여) | **추가 필요** | nexa-fs에 `path::suggest_folders(text, enum, max)` 이식(순수 로직 그대로) + 열거자는 `nexa_fs::list_opts`(숨김·닷 필터 — `UF/lib.rs:125`) 재사용 검토. 구분자는 OS별(GUI-090과 같은 문제) |
| `shellpath::resolve`(GUI-140) | `nexa_fs::shell::resolve_alias`(`UF/shell.rs:333-343`) + `portable_alias`(`UF/shell.rs:346-386`) | **있음 — ⚠ 결함** | 3-OS 구현 존재(Windows = `SHParseDisplayName` `UF/shell.rs:530-554` · mac/Linux = 공통 이름 표). **단 `UF/shell.rs:335`가 `t[..6]` 바이트 슬라이스**다 — dir2가 09-22에 고친 바로 그 panic(`A/shellpath.rs:18-23`)이 nexa-ui에는 남아 있다(예: `"shelㅔ"` · `"C:\a한"` — 6번째 바이트가 한글 중간). 다중 바이트 테스트도 없다(`UF/shell.rs:1004-1015`). `path::resolve`가 모든 입력에 호출한다(`UF/lib.rs:597`). **nexa-ui 수정 필수**(`t.get(..6)`) + dir2 테스트 `multibyte_prefix_does_not_panic` 이식 |

### 3-4. `Control` 관례 차이(이식 시 공통 적용)

- dir2 위젯은 **물리 px**를 인자로 받는다(`row_h`, `pad_x` — 호스트가 DPI 곱). nexa-ctl 컨트롤은 **논리 px 상수 + `set_scale(배율)`**(`Control::s()` — `U/controls/mod.rs:284-287` · `323-325`).
- dir2 `set_focused(bool)`(패널 활성 표시)은 nexa-ctl에서 두 개로 갈린다: `set_focused`(키보드 포커스 링) · `set_active`(창/패널 활성 = 강조색 무채화 — `U/controls/mod.rs:307-321` · `386-393`). 패널 활성 표시는 `set_active`에 대응.
- i18n: 컨트롤 내장 문자열은 `set_ctl_labels`로 주입(`U/controls/mod.rs:106-143`).
- 테스트 백엔드: dir2는 위젯마다 `Probe`(8px/문자) — nexa-ctl은 공개 `ProbeCtx`(**7px/문자**, `U/controls/mod.rs:783-796`) → 이식 테스트의 좌표 기대값을 다시 계산해야 한다.

### 3-5. `DrawCtx` 어휘 대조(담당 위젯이 쓰는 호출만)

| dir2 호출(`G/draw.rs`) | nexa-ctl(`U/draw.rs`) | 상태 |
|---|---|:--:|
| `select_font(slot, bold, italic)` `:25` | `select_font(slot, bold)` `:45` — **italic 없음**(TODO U-5) · `select_font_sized` `:52` | 차이 |
| `FontSlot::{Base, List, Status}` `:11-19` | `FontSlot::{Base, PeerList, Message, Status, Mono}` `:16-28` — **`List` 없음** | 차이 |
| `fill_rect` `:30` | 동일 `:58` | 있음 |
| `text_opaque` `:35`(DW = 말줄임 트리밍) | `text_opaque` `:62`(clip 초과분 잘림) | 있음 |
| `text` `:53`(기본 no-op) | `text` `:65`(필수) | 있음 |
| `text_width` `:38` | 동일 `:68` + `text_prefix_widths` `:82`(접두사 폭 단일 패스) · `text_height` · `text_center_y` `:94-111` | 있음(상위) |
| `push_clip`/`pop_clip` `:43-48` | **없음** — 대신 `text*`의 `clip` 인자가 좌우 모두 자른다(래스터라이저 — 추정, `U/raster.rs:430`) | 대체 |
| `draw_image(rect, 경로)` `:71` | `image(x,y,&IconImage,clip)` `:119` · `image_scaled(dst,&IconImage,clip)` `:125` — **경로가 아니라 디코드된 이미지** | 차이 |
| `draw_icon(x,y,size,key,hint) -> bool` `:78` | **없음** — 호스트가 `IconImage`를 구해 `image`로 그림 | 차이 |
| `glyph_opaque`/`glyph_opaque_lg` `:85-95` | **없음** — `polyline`/`controls::glyphs::glyph`(`U/controls/glyphs.rs`)로 직접 그리거나 `ToolIcon::Glyph`/`Mask` | 차이 |
| `fill_round_rect_alpha(rect, r, color, alpha: u8)` `:113` | `fill_round_rect_alpha(rect, r, color, alpha: f32 0..=1)` `:188` | 차이(단위) |
| `term_text`/`term_cell_w` `:58-66` | **없음**(`FontSlot::Mono`로 대체 — 터미널 문서 소관) | 차이 |
| — | `surface_size()` `:37`(팝업 안전망) · `caret_on()` `:41` · `state_layer` `:155` · `shadow` `:165` · `fill_rect_alpha` `:181` | nexa-ctl 전용 |

---

## 4. OS 분기점

| # | 항목 | Windows(dir2 현 구현) | macOS | Linux | 관련 ID |
|---|---|---|---|---|---|
| OS-1 | 휠 줄 수 | `SystemParametersInfoW(SPI_GETWHEELSCROLLLINES)` + `WM_SETTINGCHANGE` 재조회(`A/win.rs:6327-6344` · `9663`) | 시스템 설정 없음 — winit `MouseScrollDelta::LineDelta`의 값을 그대로 쓰고 줄 수 = 기본 3 고정(추정 — 실기 확인 필요) | 전역 설정 없음(libinput/툴킷별) — 기본 3 고정 | GUI-007 |
| OS-2 | 터치패드 판정 | `\|delta\| < 120` = 정밀 터치패드(`G/fastscroll.rs:9-10`) | winit `PixelDelta` = 터치패드(관성 포함) · `LineDelta` = 휠 노치 → **PixelDelta는 가속 제외 + px 직접 반영** | Wayland = `PixelDelta`(터치패드) · X11 = 대부분 `LineDelta`(터치패드도 노치로 옴 — 구분 불가, 가속이 걸릴 수 있음) | GUI-024 · GUI-116 |
| OS-3 | 경로 문법 | 구분자 `\`(입력은 `/`도 허용) · 드라이브 `C:` → `C:\` · UNC 미지원(`G/widgets/pathbar.rs:23-52`) | 구분자 `/` · 루트 `/` · 드라이브 없음 · `/Volumes/…` | 구분자 `/` · 루트 `/` | GUI-090 · 091 · 092 · 132 |
| | → 방안 | 세그먼터를 **OS별 함수로 주입**: Windows = 현 `split_path`(+UNC `\\server\share`를 첫 세그먼트로) · Unix = 첫 세그먼트 라벨 `/`(full `/`), 이후 `/` 조립. 브레드크럼 구분자 표시 = `std::path::MAIN_SEPARATOR`. 가상 최상위는 `nexa_fs::VIRTUAL_ROOT "::PC::"`(`UF/lib.rs:84`) + `path::parent_chain`(`UF/lib.rs:690-701`) 재사용 검토 | | | |
| OS-4 | 환경변수 문법 | `%VAR%` · `$env:VAR` · `${env:VAR}`(`A/pathinput.rs:10-32`) | + `$VAR` · `${VAR}` · `~` | + `$VAR` · `${VAR}` · `~` | GUI-130 |
| | → 방안 | `nexa_fs::path::resolve`가 전 문법을 한 함수로 처리(전 OS 동일 동작 — `UF/lib.rs:580-686`). 3-OS 동일 UX 기조상 OS별 분기 없이 **전부 허용** | | | |
| OS-5 | `shell:` 스킴 | `SHParseDisplayName` → `SHGetPathFromIDListEx`(`A/shellpath.rs:27-43`) · 비Windows = 원문(`A/panel.rs:1528-1530`) | 공통 이름 표: home · desktop · documents · downloads · startup(`~/Library/LaunchAgents`) · common startup(`/Library/LaunchAgents`) · appdata(`~/Library/Application Support`) — `UF/shell.rs:346-386` | 같은 표: startup(`~/.config/autostart`) · common startup(`/etc/xdg/autostart`) · appdata(`~/.config`). XDG user-dirs(`~/.config/user-dirs.dirs`의 `XDG_DOWNLOAD_DIR` 등) 반영은 미구현(추가 검토) | GUI-140 |
| OS-6 | 도구 모음 툴팁 | 별도 팝업 창(`A/tip.rs`) + `SetTimer`/`GetCursorPos` | nexa-ui `draw_tooltip_in` 창 안 그리기(전 OS 동일) · 커서 이탈 = winit `CursorLeft` | 동일 | GUI-150 · 151 |
| OS-7 | 우클릭 팝업 메뉴(탭·편집·도크·도구 모음) | `CreatePopupMenu` + `TrackPopupMenuEx`(모달 — State 참조 끊기 규약) | nexa-ui `ContextMenu`/`EditMenu` 창 안 팝업(전 OS 동일 · 모달 루프 없음 → 재진입 규약 불필요) | 동일 | GUI-048 · 098 · 125 |
| OS-8 | IME 조합 창 위치 | `ImmSetCompositionWindow(CFS_POINT)`(`A/win.rs:4803-4816`) | winit `Window::set_ime_cursor_area` + `Ime::Preedit` → `TextBox::set_preedit`(`U/controls/textbox.rs:2600`). 한글은 nexa-ui "앱 조합" 스위치 존재(`set_hangul_app_compose` — `U/controls/textbox.rs:4549`, nexa-sys `input_source`) | winit IME(X11 XIM / Wayland text-input) — 동일 API. IBus/Fcitx 조합 창 위치는 `set_ime_cursor_area` 의존 | GUI-100 |
| OS-9 | 클립보드(편집 필드·도크 복사) | `A/clipboard.rs`(`write_text`·`read_text`·`has_text`·`write_text_rich` RTF 동시 게시) | NSPasteboard(평문 + RTF `public.rtf`) | X11 selection / Wayland data-device(nexa-sql `clipboard_x11.rs` 참고 — `S/nexa-sql/src/clipboard_x11.rs`) · RTF = `text/rtf` | GUI-097 · 120 |
| OS-10 | 도크 복사 줄바꿈 | `\r\n` 결합(`G/widgets/dock.rs:300`) | `\n` | `\n` | GUI-120 |
| OS-11 | 네비 버튼·설정 글리프 | **Segoe MDL2 Assets** PUA(U+EA8A/E72B/E72A/E74A/E713 — `A/panel.rs:1551-1560` · `A/win.rs:740`) | 글꼴 없음 → 자체 그림(SVG 유래 `ToolIcon::Mask` 또는 `controls::glyphs`) — **전 OS 동일 그림으로 통일** | 동일 | GUI-077 |
| OS-12 | 탭 잠금/핀 이모지 | `🔒`/`📌` 텍스트(글꼴 폴백 체인 의존) | Apple Color Emoji(컬러 비트맵 — 자체 래스터라이저 미지원 가능) → nexa-ui `TabBar`의 **그린 글리프**(자물쇠·점)로 대체 | 이모지 글꼴 유무 불확실 → 동일 대체 | GUI-044 · 045 |
| OS-13 | 더블클릭 판정 | `WM_LBUTTONDBLCLK`(OS 판정) · 간격 `GetDoubleClickTime()`(`A/win.rs:75-77`) | winit는 더블클릭 이벤트 없음 → 앱이 시간·거리로 판정. 간격 = `NSEvent.doubleClickInterval`(nexa-sys 추가 필요 — 추정) 또는 기본 500ms | 기본 400ms(GTK `gtk-double-click-time` 조회는 선택) | GUI-050 · 096 |
| OS-14 | 위젯 틱 타이머 | `SetTimer(TIMER_WIDGET_TICK, 40ms)`(`A/win.rs:1826`) | winit `ControlFlow::WaitUntil(now+40ms)` — 요청 있을 때만(유휴 0% 유지) | 동일 | GUI-003 · 032 |
| OS-15 | 마우스 캡처(드래그가 창 밖으로) | `SetCapture`/`ReleaseCapture`(`A/win.rs:7968`) | winit: 버튼 누른 동안 창 밖 `CursorMoved` 계속 전달(AppKit 드래그 추적) | X11 = 암묵 그랩으로 전달 · Wayland = 창 밖 좌표 미전달(포인터 leave) → 자동 스크롤은 **틱 기반**으로(nexa-ui TextBox가 이미 그렇게 함 — `nexa-ui/docs/STATUS.md` 52차) | GUI-046 · 119 |
| OS-16 | 수식키 | Ctrl 고정(`MK_CONTROL`) | **⌘ = primary**(`InputEvent.primary`) · 메뉴 단축키 문구도 `⌘T` 식으로 | Ctrl | GUI-005 · 2-7 |
| OS-17 | DPI | `s(v) = v×dpi/96` · `WM_DPICHANGED` 시 `set_metrics`(`A/win.rs:7728` · `9643`) | winit `scale_factor`(정수 2.0 위주) → `Control::set_scale` | 분수 배율 가능(Wayland fractional-scale) → 동일 | 2-1 |
| OS-18 | 편집 캐럿 | 항상 표시(깜빡임 없음 — `G/edit.rs:334-338`) | nexa-ui `DrawCtx::caret_on()`으로 호스트가 깜빡임 위상 주입(전 OS 동일) | 동일 | GUI-016 |
| OS-19 | 폴더 제안 대소문자 | 접두사 비교 = 대소문자 무시(`A/pathinput.rs:139-141`) | 기본 APFS = 대소문자 무시 → 그대로 | 대소문자 구분 FS지만 **제안 필터는 무시 유지**(편의 — 이동은 실제 이름으로) | GUI-132 |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 설정 키(dir2 `settings` 평문 `key=value` — `A/config.rs:434-510`)

| dir2 키 | 기본 | 클램프 | 쓰는 곳 | nexa-sql 설정 레지스트리 대응(참고) |
|---|---|---|---|---|
| `fast_scroll` | 1 | — | GUI-020 | `scroll.fast`(`S/nsql-settings/src/lib.rs:2536`) |
| `fast_scroll_step` · `fast_scroll_max` | 3 · 16 | 1..50 · 1..32 | GUI-020/022 | `scroll.fast_speed`(단계 프리셋 — `:2544`; `fast` = step 3·max 16 — `G/fastscroll.rs:44-45`) |
| `fast_scroll_window_ms` | 160 | 20..2000 | GUI-022 | `scroll.fast_window_ms`(`:2599`) |
| `fast_scroll_hud` · `_hud_pos` · `_hud_hold_ms` · `_hud_fade_ms` | 1 · 2 · 250 · 600 | pos 0..8 · ms 0..10000 | GUI-023 | `scroll.fast_hud` · `scroll.fast_hud_pos` · `scroll.fast_hud_hold_ms` · `scroll.fast_hud_fade_ms`(`:2560-2588`) |
| `fast_scroll_grid_extra` | 1 | — | GUI-021 | `scroll.fast_grid_extra`(`:2552`) |
| `tab_dblclick` | `close` | `close`\|`pin`\|`lock` | GUI-050 | (dir3 신규 키 — 추정) |
| `split` | (비율) | — | 2-2 패널 스플리터 | 〃 |
| `dock` · `dock_ratio` · `dock_split` | — · 0.3 · 0.5 | ratio 0.15..0.5 · split 0.15..0.85 | 2-2 도크 밴드 | 〃 |
| `launcher` · `always_on_top` · `panel_mode` · `info_mode` · `view_mode` | — | `single`\|`dual` 등 | 메뉴 체크·도구 모음 토글 표시 | 〃 |
| `toolbar_order` | `default_toolbar_order()` | `parse_toolbar_order` 검증·보충 | 도구 모음 구성(2-8) | 〃 |
| `launcherN` | — | `-` = 구분선 | 퀵 런처 바 | 〃 |

근거: `A/config.rs:77-79` · `134-140` · `160-169` · `179-186` · `212-229` · `260-314` · `575-753`.
세션(탭 목록·활성·잠금/고정·펼침)은 `SESSION_FILE`에 따로 저장(`A/win.rs:43` · `A/panel.rs:127-129` — 형식 상세는 세션 문서).

### 5-2. 위젯 내부 상태(영속 아님 — 재생성 시 초기화)

| 위젯 | 런타임 상태 | paint 캐시(`RefCell`/`Cell`) |
|---|---|---|
| `TabBar` | titles · locked · pinned · active · focused · hover · drag(인덱스, 프레스, 시작) · pending | 탭 rect·[+] rect · lines · lines_changed |
| `MenuBar` | menus · open · hover_title · hover_item · open_sub · hover_sub · pending | title_ranges · drop_rect · sub_rect |
| `Toolbar` | buttons · button_w · large_glyphs · hover · pending | ranges |
| `PathBar` | path · segments · hover · edit(`EditState`) · suggest(items, sel, base) · overlay_bottom · pending_nav | ranges |
| `EditState` | buf · caret · anchor · dragging · undo 스냅샷 | (rect, 원점 x, 문자 경계 오프셋) |
| `InfoDock` | kinds · active · lines · image · content_key · scroll · scroll_frac · scroll_x · sel · sel_drag · popout(on/hover/pressed) · focused · bars · wheel 누적기 3개 · fast | ranges · goto_range · popout_range · offsets(가시 행) · content_w |
| 전역 | `WHEEL_LINES_SYS`(AtomicI32) · `FAST`/`FAST_GRID`(RwLock) | — |

> **히트 테스트는 전부 직전 paint의 캐시에 의존한다**(텍스트 측정이 paint에서만 가능 — `G/widgets/tabbar.rs:5-7` · `G/edit.rs:5-7`). paint 전 클릭은 무반응이 정상.

### 5-3. 스레딩

- 담당 위젯·로직은 **전부 UI 스레드 단일 스레드**다(`RefCell`/`Cell` 사용 — `Sync` 아님). 워커 스레드 없음.
- 전역 설정만 스레드 안전(`AtomicI32`·`RwLock`) — `G/event.rs:8` · `G/fastscroll.rs:60` · `86`.
- `fs_dirs`(자동완성 열거)는 **UI 스레드 동기 `read_dir`**(`A/pathinput.rs:152-163` · 호출 `A/win.rs:7199-7206`) — 느린 네트워크 경로에서 타이핑이 멈출 수 있다(dir2에 완화책 없음 — 6절 L-22).
- `shellpath::resolve`는 호출 스레드 COM 초기화 필요(런타임 = UI 스레드 OLE 초기화, 테스트는 직접 초기화 — `A/shellpath.rs:50-59`). nexa-fs는 스레드별 `ensure_com`(`UF/shell.rs:556-559`).
- 툴팁 창은 자체 wndproc(`tip_proc`)를 갖지만 같은 UI 스레드에서 돈다(`A/tip.rs:113-161`).

### 5-4. 메시지 흐름

**입력 → 위젯 → 호스트 동작(1회성 수거 패턴)**
```
WM_*  →  InputEvent 번역  →  widget.on_event(ev, &mut inv)      (상태 변경 + 무효화 push)
      →  widget.take_action()/take_command()/take_navigation()/take_goto()/take_popout()
      →  호스트 실행(run_command / Panel::drain_actions)          A/panel.rs:1508-1542 · A/win.rs:7976-7978
      →  flush_invalidations(hwnd, &mut inv)                      A/win.rs:1824-1837
           ├ inv.take_tick() → SetTimer(TIMER_WIDGET_TICK, 40ms)
           └ inv.drain()     → InvalidateRect(rect)
WM_PAINT → 위젯 paint(백버퍼) → BitBlt → tabbar.take_lines_changed() → layout 재실행   A/win.rs:4820-4947
```

**틱(페이드)**: `TIMER_WIDGET_TICK` → `rows.tick` ×2 · `dock.tick` ×2 · 터미널 배지 → 재요청 없으면 `KillTimer`(`A/win.rs:9493-9520`).

**경로바 자동완성**: 편집 텍스트 변경(문자 입력·Delete·잘라내기·붙여넣기·undo) → `update_path_suggest` → `expand_env` → `suggest_folders(…, fs_dirs, 20)` → `pathbar.set_suggestions`(`A/win.rs:7199-7206` · `7280-7282` · `8728-8730`).

**편집 동작 디스패치(`do_clip`)**: ① 경로바 편집 → ② 인라인 리네임 → ③ 도크 터미널(키 포커스) → ④ 도크 Info/Preview 선택(복사만) → ⑤ 파일 목록(`A/win.rs:7231-7355`). Edit 메뉴·단축키·우클릭 메뉴가 **같은 경로**를 탄다.

**툴팁**: `WM_MOUSEMOVE` → `tip_on_mousemove`(버튼 변경 = 무장/해제) → `TIMER_TIP` 250ms 틱 ×2 → `tip::show` · 클릭/이탈 = `tip_cancel`(`A/win.rs:5842-5908`).

---

## 6. 이식 시 주의 — 실측 교훈·결함 수정 이력(회귀 방지)

| # | 교훈 | 근거 | 대상 ID |
|---|---|---|---|
| L-01 | **문자 경계 오프셋은 접두사 폭으로**(문자별 폭 합산은 커닝 때문에 렌더 위치와 어긋나 캐럿·클릭 매핑이 밀린다 — QA 07-13). nexa-ui는 `text_prefix_widths` 계약(`out[i] == text_width(접두사 i)`)으로 보장 | `G/edit.rs:291-299` · `U/draw.rs:77-90` | GUI-015 · 016 · 119 |
| L-02 | **선택 하이라이트 위 텍스트는 1회 호출로**(런 분할은 말줄임 트리밍과 이음새로 경계가 잘린다 — QA 07-13 2·3차) | `G/edit.rs:315-333` · `G/draw.rs:50-55` | GUI-016 |
| L-03 | 편집 진입 = **전체 선택** · 넘치면 **끝 정렬**(최근 폴더가 보이게) · 캐럿은 `_`가 아니라 **세로바**(사용자 지시 07-13) | `G/edit.rs:43-55` · `286-338` · `G/widgets/pathbar.rs:136-140` | GUI-010 · 016 · 094 |
| L-04 | 브레드크럼은 **구분자 `\`·여백 최소**로 실제 경로 문자열처럼 조밀하게, 넘치면 **끝 정렬 + `…`**(사용자 지시 07-13) | `G/widgets/pathbar.rs:483-513` | GUI-091 · 092 |
| L-05 | 메뉴 **열기 직후 첫 프레임 깨짐**: 정확한 rect는 paint에서야 확정되는데 이전 캐시(빈 rect)만 무효화하면 BitBlt가 잘린다 → 보수적 근사 영역을 무효화(QA 07-13). 팝업류(제안 목록·플라이아웃) 전부 "이전 영역 + 새 영역" 둘 다 push | `G/widgets/menubar.rs:168-175` · `246-249` · `G/widgets/pathbar.rs:195-212` | GUI-068 · 099 |
| L-06 | 탭 드래그 재정렬은 **대상 탭 x 중간점 통과 시에만** 스냅(탭 사이로 넘어갈 때 — QA 07-14). 멀티라인에서 아래 줄 = 항상 뒤 인덱스라 같은 규칙이 성립 | `G/widgets/tabbar.rs:244-272` | GUI-046 |
| L-07 | **MouseUp은 드래그 상태를 가진 모든 위젯에 전달**(탭 바에 안 주면 드래그 상태 잔존 — QA 07-14) | `A/panel.rs:1493-1502` | GUI-046 · 096 · 119 |
| L-08 | 마지막 탭·잠긴 탭은 **닫기 불가**(× 존 클릭 = 전환) — 패널 공백 방지 | `G/widgets/tabbar.rs:221-227` | GUI-042 |
| L-09 | 도구 모음 hover 색은 **`sel_bg`**(`header_bg`는 `chrome_bg`와 명도 차가 없어 식별 불가 — X-27). 토글 켜짐 = **accent 38% 블렌드 + 아이콘은 본문색 유지**("파랑 필 + 흰 선" 시안은 원복 — 07-19) | `G/widgets/chrome.rs:273-289` | GUI-073 · 074 |
| L-10 | 바 높이 이력: 도구 모음 24→26→24→32→**28**(아이콘 20) · 런처 **24**(아이콘 16 — "이전 크기가 낫다"로 원복). 두 바 높이를 **분리**한 이유. 아이콘 크기는 래스터 버킷과 정확히 일치해야 선명 | `A/win.rs:1851-1858` · `A/win.rs:794-799` | GUI-071 · 076 |
| L-11 | 네비 글리프 13 DIP(11 = 식별 어려움 · 15 = 과함 — 사용자 확정 08-01) · 순서 홈/이전/다음/상위 · 네비 버튼과 경로 바 사이 **틈 0**(4px 틈은 미도색 영역이 검게 비쳤다) | `A/panel.rs:392-394` · `1550-1554` | GUI-077 · 2-3 |
| L-12 | **`[..6]` 바이트 슬라이스 금지**: 6번째 바이트가 한글 중간이면 panic → 설치본 크래시(09-22). `get(..6)` 사용. ⚠ **nexa-ui `nexa-fs/src/shell.rs:335`에 같은 패턴이 남아 있다** | `A/shellpath.rs:18-23` · `UF/shell.rs:335` | GUI-140 |
| L-13 | **`to_lowercase()` 오프셋을 원문에 쓰지 않는다**(`İ` 2B→3B · `K` 3B→1B로 길이 변화 → 문자 경계 panic·엉뚱한 치환 — G13-01) | `A/pathinput.rs:34-47` | GUI-131 |
| L-14 | 툴팁 **빈 텍스트 금지**(빈 Vec → `DrawTextW` AV) — nexa-ui `draw_tooltip_in`도 빈 문자열 조기 반환 | `A/tip.rs:55-57` · `U/draw.rs:248-250` | GUI-150 |
| L-15 | 도크 ↗ 버튼은 **오버레이 바보다 먼저 판정**(버튼 오른쪽 8px가 세로 바 존과 겹쳐 '색은 바뀌는데 안 눌림' — G6-10). 명시적 버튼이 바보다 우선 | `G/widgets/dock.rs:661-667` | GUI-121 |
| L-16 | 버튼은 **안에서 뗄 때만 발화**(↗) — 누름 시각 = 배경 + 1px 오프셋 | `G/widgets/dock.rs:799-806` | GUI-121 |
| L-17 | **같은 대상의 내용 갱신은 스크롤·선택 유지**(상세 도착·시각 갱신마다 0으로 리셋되면 스크롤 중 "끊기거나 튄다" — 10-02 QA) | `G/widgets/dock.rs:71-74` · `503-525` | GUI-113 |
| L-18 | 트랙패드는 노치(120) 미만 delta를 잘게 보낸다 — **정수 나눗셈은 0줄이 돼 "천천히 움직이면 스크롤 안 됨"** → 분수 누적기 필수. 노치 미만은 **픽셀 스크롤**, 노치는 줄 단위 + 스냅 | `G/widgets/dock.rs:65-68` · `756-774` | GUI-006 · 116 |
| L-19 | 정밀 터치패드에는 **고속 배수를 걸지 않는다**(OS가 이미 가속·관성) | `G/fastscroll.rs:9-10` · `314-324` | GUI-024 |
| L-20 | 빈 영역에서 시작한 드래그 선택이 무시되면 안 된다 → 마지막 줄 끝 앵커(메모장 규약 — 10-02) · 이동 없는 클릭은 선택 없음 | `G/widgets/dock.rs:318-339` · `807-812` | GUI-119 |
| L-21 | 가로 스크롤 텍스트는 **보이는 첫 문자 경계부터** 그렸다(DW 글리프가 GDI 클립을 무시해 왼쪽으로 번짐) · 부분 행은 스트립을 **뒤에 다시** 그려 덮었다. → nexa-ui 래스터라이저는 `clip`으로 자르므로(추정) 이 우회가 불필요할 수 있으나 **픽셀 스크롤 번짐은 실기 캡처로 확인** | `G/widgets/dock.rs:917-948` · `600-613` | GUI-117 · 123 |
| L-22 | 0-rect 위젯 라우팅 금지: 숨은 도크(높이 0)는 `y >= 0`이 항상 참이라 **모든 클릭을 삼켰다**(X-20). 표시 플래그가 아니라 **실제 rect**로 판정. 음수 폭 rect를 채우면 GDI가 뒤집어 패널 전체를 덮었다(공백 화면) → 양수만 그림 | `A/panel.rs:431-437` · `1461-1464` · `A/win.rs:4886-4900` | 2-6 · GUI-110 |
| L-23 | 편집 필드 우클릭: **편집을 시작시킨 첫 우클릭은 메뉴를 띄우지 않는다**(10-01). 편집 중 다른 대상(터미널) 메뉴를 고르면 편집을 먼저 정리(G3-06) | `A/win.rs:8206-8209` · `8250-8255` · `7234-7244` | GUI-098 |
| L-24 | 자동완성 팝업은 **리스트 바닥을 넘지 않는다**(도크/터미널 침범 금지) · 줄어들 때 이전 영역 무효화(잔상) · Esc는 **팝업이 열려 있으면 팝업만** 닫는다(탐색기 규약) | `A/panel.rs:414-415` · `G/widgets/pathbar.rs:195-198` · `A/win.rs:8710-8716` | GUI-099 |
| L-25 | 붙여넣기는 **첫 줄만·제어 문자 제거**(편집 필드는 한 줄) | `A/win.rs:7622-7633` | GUI-097 |
| L-26 | 미정의 환경변수·미지 `shell:` 이름은 **원문 유지** → 열기 실패 = 위치 유지(자연 격리). 오류 대화상자를 띄우지 않는다 | `A/pathinput.rs:4-5` · `A/shellpath.rs:8-9` | GUI-130 · 140 |
| L-27 | (nexa-ui 쪽 규칙) 떠 있는 것은 **표면 밖으로 나가지 않는다** — `geom::place_popup`/`nudge_into`/`popup_host` 사용, 컨트롤 안에서 위치를 따로 계산하지 않는다. 새 팝업 컨트롤 = `geom.rs popup_tests`에 사례 추가 + 창 모서리 캡처 1장. dir2 메뉴·제안 팝업·툴팁에는 이 보정이 **없다** | `nexa-ui/CLAUDE.md:44-50` | GUI-063 · 099 · 150 |
| L-28 | (nexa-ui 쪽 규칙) 프로세스 전역 스위치를 만지는 시험은 **가드로 직렬화**(시험은 병렬) — `set_fast_scroll`·`set_wheel_lines`·`set_hide_delay_ms`·`std::env::set_var` 테스트가 해당. dir2 `pathinput` 테스트는 `set_var`를 가드 없이 쓴다(변수 이름을 달리해 회피) | `nexa-ui/CLAUDE.md:61` · `A/pathinput.rs:180` · `197` | GUI-007 · 020 · 130 |
| L-29 | (nexa-ui 쪽 규칙) 공개 API를 바꾸면 **같은 작업 안에서 nexa-sql·nexa-dlg를 빌드·테스트** + 커밋 본문에 `영향:` 표기 · push는 nexa-ui가 nexa-sql보다 먼저 | `nexa-ui/CLAUDE.md:41` · `54` · `62` | 3절 전체 |

---

## 7. 회귀 테스트 후보

자동화: **U** = 순수 단위 테스트(ProbeCtx · OS 무관 · 3-OS CI) · **S** = 오프스크린 렌더 스냅샷/픽셀 프로브(RasterCtx) · **I** = 창을 띄운 통합(입력 주입) · **M** = 수동 실기.

| # | 시나리오 | 대상 ID | 자동화 | 비고 |
|---|---|---|:--:|---|
| T-01 | 무효화: 교차 rect 병합 · 빈 rect 무시 | GUI-002 | U | dir2 테스트 2건 이식 |
| T-02 | 휠 분수 누적: 40×3 = 3줄 · 30×12 = 9줄(손실 없음) | GUI-006 | U | 이식 |
| T-03 | `set_wheel_lines(-1)` → 10 · `(100)` → 20 · `(0)` → 10 | GUI-007 | U | **신규**(dir2에 테스트 없음) · 전역 가드 필요(L-28) |
| T-04 | 편집: 전체 선택 진입 후 입력 = 대체 · Shift+←/Home/End 선택 · 비Shift ←/→ = 가장자리 접기 · Delete · 이름부 선택 | GUI-010~012 | U | dir2 4건 → `EditState`/`TextBox` API로 재작성 |
| T-05 | 편집 필드 클릭 = 최근접 경계 캐럿 · 드래그 = 범위 · 클릭만 = 선택 없음 | GUI-015 | U | `TextBox` 이벤트로 재작성 |
| T-06 | 긴 텍스트(40자, 필드 100px): 캐럿이 가시 범위 안 · 앞이 잘림 | GUI-016 | U/S | |
| T-07 | 실행 취소: 변경 후 undo = 내용·선택 복원 | GUI-014 | U | 다단계로 바뀌므로 기대값 갱신 |
| T-08 | 가속 배수: step 3 → `[1,1,1,2]` · 7번째 = 3 · 상한 16 · 방향 전환/간격 초과 = 1 · 끔 = 1 · 그리드 = (2, 32) | GUI-021 · 022 | U | 이식(시각 주입) |
| T-09 | 배지: hold 동안 1.0 → 페이드 중 0~1 → 소거 · 1배 복귀 = 즉시 페이드 · hud 끔 = 소거 · 3×3 자리 좌표 | GUI-023 | U | nexa-ui `SpeedHud` 시그니처에 맞춤 |
| T-10 | **노치 미만 delta 30회 = 배수 1·배지 없음** / 노치 8연타 = ×3 | GUI-024 | U | **핵심**(ScrollBars에 없는 규칙) |
| T-11 | 스크롤바: 내용이 맞으면 썸 없음 · 비례 길이·최소 길이 · 오프셋 클램프 · 숨은 바 호버 무반응 · 드래그 → 오프셋 매핑 · 트랙 클릭 방향 | GUI-030~035 | U | `ScrollBars`로 바꾸면 **트랙 클릭 = 점프**로 기대값 변경 — 결정 후 고정 |
| T-12 | 탭: 본문 클릭 = Switch · × = Close · [+] = New · 빈 곳 = 없음 · 마지막 탭 × = Switch · 잠긴 탭 × = Switch | GUI-041~044 | U | nexa-ui는 **MouseUp 발화** — 이벤트 시퀀스 down+up으로 |
| T-13 | 탭 멀티라인: 폭 150·4탭 → 3줄 · `take_lines_changed` 1회성 · 줄별 히트 | GUI-047 | U | ProbeCtx 7px로 수치 재계산 |
| T-14 | 탭 드래그: 임계 8px · 중간점 통과 = Move · 아래 줄로 이동 · 되돌리기 · 이양 API(pressed/dragging/cancel/begin) | GUI-046 · 049 | U | 이식 |
| T-15 | 탭 더블클릭 = 설정 동작(close/pin/lock) · 빈 곳 더블클릭 = 새 탭 | GUI-050 | I | 더블클릭 판정이 앱 몫(OS-13) — 판정 함수는 U |
| T-16 | 메뉴: 제목 클릭 열기 → 항목 클릭 = 명령 1회 + 닫힘 · 구분선 = 명령 없음 + 닫힘 · 외부 클릭 닫기 · 같은 제목 재클릭 토글 · 열린 채 다른 제목 hover = 전환 | GUI-062 · 064 | U | nexa-ui `take_picked`(String id) |
| T-17 | 메뉴 하위: 부모 클릭 = 펼침(발화 없음) → 하위 항목 클릭 = 발화 + 전체 닫힘 | GUI-065 | U | 이식 |
| T-18 | 메뉴 체크: `set_checked` 가 최상위·하위 항목 모두 갱신 · 라디오 그룹(보기 모드 3·테마 3·언어) 중 하나만 ✓ | GUI-066 | U/S | nexa-ui에 **기능 추가 후** |
| T-19 | 메뉴 단축키 문구가 우측 정렬로 그려진다(문구 누락 = 회귀) | GUI-063 | S | 〃 |
| T-20 | 메뉴 드롭다운·하위 메뉴가 창 오른쪽/아래에서 잘리지 않는다 | GUI-063 · L-27 | S | nexa-ui 팝업 규칙 |
| T-21 | 도구 모음: 클릭 = id 1회 · 비활성 = 무시·hover 없음 · 구분선 = 히트 없음 · 토글 켜짐 배경색 = 블렌드 값 | GUI-072~075 | U/S | 토글은 nexa-ui 추가 후 |
| T-22 | 도구 모음 아이콘 미로드 = 라벨 앞 2자 폴백 | GUI-076 | U | |
| T-23 | 툴팁: hover 500ms 뒤 표시 · 버튼 변경/이탈/클릭 = 사라짐 · 빈 팁 = 없음 | GUI-150 · 151 | I/U | 시각 주입형 상태기계로 만들면 U |
| T-24 | 상태바: 같은 텍스트 재설정 = 무효화 없음 | GUI-080 | U | 이식 |
| T-25 | `split_path`: `C:\Users\kiros33` 3세그먼트 · `C:/a/b/` · 빈 문자열 · **Unix `/home/u/x`** · **UNC** | GUI-090 | U | Unix·UNC = 신규 |
| T-26 | 브레드크럼: 세그먼트 클릭 = 그 경로 통지 · 마지막 세그먼트 무동작·hover 없음 · 영역 밖 hover 해제 | GUI-091 · 093 | U | 이식 |
| T-27 | 긴 경로: `…` 표시 + 마지막 세그먼트 가시 + 생략 세그먼트 클릭 불가 | GUI-092 | U/S | **신규**(dir2에 테스트 없음) |
| T-28 | 우클릭 = 편집(전체 선택) → 입력 = 대체 → Enter = 통지 · Esc = 원복 · `set_path` = 편집 종료 | GUI-094 · 095 | U | 이식 |
| T-29 | 자동완성: ↓ = 1번째 미리 채움 · 첫 항목에서 ↑ = 입력 복원 · 클릭 = 즉시 제출 · 빈 목록 = 닫힘 · Esc = 팝업만 닫힘 · 팝업이 리스트 바닥을 넘지 않음 | GUI-099 | U | 이식 + 하한 클램프 신규 |
| T-30 | `expand_env`: `%V%` · `$env:V` · `${env:V}` · 따옴표 · 미정의 원문 · 공백 trim · **비ASCII(`İ`·`K`·`ǅ`) 앞뒤에서 panic 없음** · 토큰 대소문자 무시 | GUI-130 · 131 | U | nexa-fs에 이식(가드 — L-28) |
| T-31 | `suggest_folders`: 구분자 직후 = 전체 · 접두사 대소문자 무시 · 상한 · 구분자 없음/빈 입력/실패 베이스 = 빈 목록 · **`/` 구분자** | GUI-132 | U | 이식 |
| T-32 | **`shell:` 접두 판정이 다중 바이트 입력에서 panic하지 않는다**(`C:\ㅔ` · `ㅔㅔ` · `다운로드` · `shelㅔ` · `D:\프로젝트\x`) | GUI-140 · L-12 | U | **최우선** — nexa-ui 현 코드는 실패(panic)할 것으로 판단(`UF/shell.rs:335`) |
| T-33 | `shell:startup`/`common startup`/`Downloads` 해석 · 미지 이름·일반 경로 = 통과 | GUI-140 | U(OS별) | Windows = 이식 · mac/Linux = 이름 표 검증 신규 |
| T-34 | 도크 스트립: 라벨 클릭 = 종류 전환 + 선택 해제 · → 클릭 = `take_goto` + 터미널 종류 | GUI-110 · 111 | U | **신규**(dir2에 테스트 없음) |
| T-35 | 도크 선택: 문자 단위 다중 라인(`"cdef\r\n012"`) · 한 줄 부분 · 클릭만 = 없음 · 빈 영역 앵커 · 역방향 | GUI-119 · 120 | U | 이식(줄바꿈은 OS-10 결정에 맞춤) |
| T-36 | 도크 스크롤: 1노치 = 3줄 · 상한/0 클램프 · 스크롤 후 히트 = 절대 라인 · 하단 밖 드래그 = 1행씩 자동 스크롤 | GUI-116 · 119 | U | 이식 |
| T-37 | 도크 터치패드: delta 8×15 = 3줄 · delta −60 = 1줄 + 10px · 노치 = 스냅 | GUI-116 | U | 이식 |
| T-38 | 도크 가로: 최대 폭 측정 · 상한 클램프 · 스크롤 후 히트 · 내용 교체 = 0 | GUI-117 | U | 이식 |
| T-39 | 도크 바: 썸 드래그 = 스크롤·선택 없음 · 틱 후 사라짐 | GUI-118 | U | 이식(ScrollBars 모델에 맞춤) |
| T-40 | 도크 ↗: hover 무효화 1회 · 누름 = 발화 전 · 밖 릴리스 = 취소 · 안 릴리스 = 1회 · 꺼짐 = 히트 없음 · **바와 겹쳐도 버튼 우선** | GUI-121 · L-15 | U | 이식 2건 |
| T-41 | 도크 내용: 같은 키 갱신 = 스크롤·선택 유지 · 줄 수 감소 = 클램프·선택 해제 · 다른 키 = 리셋 · 동일 내용 = 무비용 | GUI-112 · 113 | U | 이식 |
| T-42 | 도크 전체 선택: 빈 내용 = 불가 · 이미지 = 불가 · 마커 줄 복사 제외 | GUI-115 · 120 | U | 이식 + 마커 신규 |
| T-43 | 레이아웃: 창 크기·DPI·도크 on/off·싱글/듀얼·런처 on/off 조합에서 각 rect가 2절 수식과 일치 · 음수/0 rect에 클릭이 삼켜지지 않음 | 2-2 · 2-3 · L-22 | U | 레이아웃을 **순수 함수**로 분리하면 U |
| T-44 | 그리기 순서: 메뉴 드롭다운이 도구 모음·패널 위 · 제안 팝업이 목록 위 · 스트립이 부분 행 위 | 2-5 | S | 픽셀 프로브 |
| T-45 | 편집 디스패치 우선순위: 경로바 편집 중 Ctrl+C = 경로바 선택 복사(파일 복사 아님) · 도크 선택 중 Ctrl+C = 도크 텍스트 | 5-4 | I/U | 디스패치 판정을 순수 함수로 |
| T-46 | 한글 IME: 경로바 편집 중 조합 → 확정 → Enter(조합 중 음절 누락 없음) · 조합 창이 캐럿 위치 | GUI-100 | M | OS별 실기(nexa-ui `commit_composition`) |
| T-47 | 다크/라이트 전환: 탭 활성 줄·토글 버튼·선택 하이라이트·스크롤바가 식별 가능 | GUI-040 · 074 · 036 | S/M | 대비 프로브 |
| T-48 | 3-OS 화면 동일성: 같은 상태의 메뉴/탭/경로 바/도크 캡처가 OS간 픽셀 수준으로 같다(글꼴 차이 허용 범위) | 전체 | S | nexa-ui 기조(자체 래스터라이저) |

**기존 dir2 테스트 69건 중 이식 대상**: widget 2 · geom 4 · event 3 · edit 8 · fastscroll 5 · overlaybar 5 · tabbar 6 · menubar 7 · chrome 2 · pathbar 7 · dock 13 · pathinput 4 · shellpath 3. nexa-ui에 이미 대응 테스트가 있는 것(widget·geom·event·tabbar 일부·scroll 일부)은 중복 이식하지 않고 **차이 나는 규칙만** 추가한다(T-10·T-12·T-18·T-32).

---

## 부록 A. 결정이 필요한 항목(구현 단계 입력)

| # | 결정 | 선택지 | 권고 |
|---|---|---|---|
| D-1 | 탭 전환 시점 | (a) nexa-ui 그대로 MouseUp (b) dir2처럼 MouseDown 옵션 추가 | (a) — nexa-ui 09-28 사용자 확정("다른 프로그램처럼 Up에서" — `U/controls/tabbar.rs:890`). dir3는 "컨트롤만 nexa-ui용으로" 기조에 맞춰 수용, 패널 간 드래그 이양 로직만 재검증 |
| D-2 | 스크롤바 동작 | (a) `ScrollBars` 그대로(시간 숨김·트랙 점프) (b) dir2 규칙(틱 페이드·트랙 페이지) 옵션 추가 | (a) — nexa-sql과 동일 UX. 단 **터치패드 가속 제외(L-19)는 반드시 보존** |
| D-3 | 틱 모델 | (a) 컨트롤별 `tick() -> bool` (b) `Invalidations::request_tick` 추가 | (a) — nexa-sql 구조 기준 |
| D-4 | 메뉴 단축키·체크 표시 | nexa-ctl `MenuEntry`에 추가(영향: nexa-sql) | 추가 — dir2 기능 유지에 필수(GUI-063·066) |
| D-5 | 도구 모음 토글 상태 | nexa-ctl `ToolItem`에 `checked` + `set_item_checked` 추가 | 추가 — 필수(GUI-074) |
| D-6 | `InfoDock`·`PathBar`·`StatusBar` 위치 | nexa-ctl 신규 컨트롤 | 추가 — 사용자 방침("없으면 nexa-ui에 추가") |
| D-7 | 도크 복사 줄바꿈 | (a) 전 OS `\r\n` (b) OS별 | (b) — Windows `\r\n` / 그 외 `\n` |
| D-8 | `$NAME` 확장(Windows 포함) | (a) nexa-fs 그대로 전 OS 허용 (b) Windows에선 `$env:`만 | (a) — 3-OS 동일 동작(미정의 = 원문 유지라 위험 낮음) |
| D-9 | nexa-ctl `Theme` 토큰 | (a) `tab_bar_bg`·`header_bg`·`status_bar_bg`·`bottom_dock_bg` 추가 (b) 기존 토큰으로 대응(`chrome_bg`/`panel_bg_alt`) | (b) 우선 — nexa-ui `TabBar`가 이미 `chrome_bg` 사용. 시각 차이는 캡처 비교 후 판단 |
