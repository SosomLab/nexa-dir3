# 51 — nexa-ui API 카탈로그: 컨트롤(nexa-ctl `controls`) + nexa-sql 앱 내 범용 UI (UIK-NNN)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 기준일 2026-10-03 · 대상 = nexa-ui `df75f5a`(102차) · nexa-sql 작업 트리(같은 날).
> 이 문서의 `UIK-NNN` ID는 이후 구현·교차 검증의 체크리스트다. ID는 고정이며 재번호하지 않는다(결번 허용).
> 번호대: `UIK-001~029` = nexa-ctl 컨트롤 · `UIK-101~116` = nexa-sql 앱이 직접 만든 범용 UI · `UIK-201~222` = 파일 탐색기에 필요한데 없는 것(추가 후보).

**경로 약어**

| 약어 | 실제 경로 |
|---|---|
| `U/` | `nexa-ui/crates/nexa-ctl/src/` |
| `UC/` | `nexa-ui/crates/nexa-ctl/src/controls/` |
| `UD/` | `nexa-ui/crates/nexa-dlg/src/` |
| `UDoc/` | `nexa-ui/docs/` |
| `S/` | `nexa-sql/crates/nexa-sql/src/` |
| `P/` | `nexa-dir3/docs/port/` (이 저장소의 선행 인벤토리 문서) |

---

## 0. 범위

### 0-1. 조사 방법과 깊이(정직하게 적는다)

| 구분 | 파일 | 줄(전체/테스트 제외) | 조사 깊이 |
|---|---|---:|---|
| 끝까지 읽음 | `UC/mod.rs` · `U/lib.rs` · `U/widget.rs` · `U/event.rs` | 817 · 80 · 81 · 164 | 전부 |
| 끝까지 읽음 | `UC/tree.rs` | 1244 / 954 | 본문 전부(테스트 모듈 제외) |
| 공개 API 전수 + 이벤트 처리 본문 | `UC/button.rs` · `checkbox.rs` · `radio.rs` · `switch.rs` · `combo.rs` · `ctxmenu.rs` · `pulldown.rs` · `tabbar.rs` · `toolbar.rs` · `tooldock.rs` · `editmenu.rs` · `carousel.rs` · `icondrop.rs` · `listedit.rs` · `posdrop.rs` · `posgrid.rs` · `timeout_button.rs` · `colorpick.rs` | 아래 표 | `pub` 항목 전수(스크립트 추출 + 문서 주석 첫 줄) + 모듈 머리말 + `on_event` 본문 정독. **`paint` 본문과 `#[cfg(test)]` 모듈은 읽지 않았다**(MenuBar `paint`/`paint_panel`만 예외로 읽음) |
| 공개 API 전수(본문 미독) | `UC/colorpanel.rs` · `UC/pairs.rs` | 697/590 · 606/411 | `pub` 항목 + 모듈 머리말. 이벤트 본문은 읽지 않음 → 동작 칸은 머리말 근거 |
| 구조 조사 | `S/grid.rs`(8080) · `S/explorer.rs`(8908) | — | 모듈 머리말 · 최상위 항목 전수 · 구조체 필드 전수 · 메서드 이름 전수 · `on_event`의 분기(Grep). **본문을 줄 단위로 정독하지 않았다** → §3의 세부는 필요한 곳에 "추정" 표기 |
| 구조 조사 | `S/palette.rs`(704) · `S/findbar.rs`(927) · `S/filterbar.rs`(1043) · `S/toast.rs`(345) · `S/toolfloat.rs`(187) | — | 모듈 머리말 · 공개 항목 전수 · 핵심 타입 정의 |
| 맥락 확인(범위 밖) | `S/main.rs` 895-1010 · `S/activity.rs` 1-30 · `S/app/paint.rs`(Grep) · `S/project_panel.rs`(머리말+항목) · `UD/lib.rs`(머리말+항목) · `UDoc/TODO.md` 1-34 · `UDoc/21-grid-family.md` 20-126 · `P/14-dir2-gui-widgets.md` 1-185 | — | "없는 컨트롤" 판정이 틀리지 않도록 이웃만 확인 |

담당 컨트롤 파일 줄 수(전체 / 테스트 시작 줄): button 695/544 · carousel 365/257 · checkbox 246/161 · colorpanel 697/591 · colorpick 297/210 · combo 1363/1142 · ctxmenu 2145/1437 · editmenu 256/190 · icondrop 406/310 · listedit 543/435 · pairs 606/412 · posdrop 320/245 · posgrid 290/220 · pulldown 967/732 · radio 253/190 · switch 284/206 · tabbar 1618/1172 · timeout_button 394/306 · toolbar 926/818 · tooldock 1175/937 · tree 1244/955.

### 0-2. 범위 밖(다른 문서 몫 — 여기서는 존재만 적는다)

`UC/textbox.rs`(7646) · `UC/scroll.rs`(1175 — `ScrollBars`·`FastScroll`·`ScrollAccel`·`SpeedHud`) · `UC/splitter.rs`(216) · `UC/flash.rs`(213) · `UC/glyphs.rs`(254) · `U/draw.rs` · `U/theme.rs` · `U/tokens.rs` · `U/typeahead.rs` · `U/view_mode.rs` · `U/gridedit/*` · `U/edit*` · nexa-gfx/nexa-font/nexa-fs/nexa-sys/nexa-conf. 재수출 목록은 `UC/mod.rs:42-74` · `U/lib.rs:47-80`.

### 0-3. 공통 계약 한눈에(모든 컨트롤이 따르는 규약)

- **Widget**: `bounds()` · `set_bounds(bounds, &mut Invalidations)` · `on_event(&InputEvent, &mut Invalidations)` · `paint(&self, &mut dyn DrawCtx, &Theme)` — `U/widget.rs:44-56`. 페인트는 `&self`(상태 변경 없음 — 측정 캐시는 `Cell`/`RefCell`).
- **Control**: `ControlBase{bounds, scale, focused, active, enabled, help, show_help, help_open, halign, valign, last_value}`(`UC/mod.rs:228-255`) + 기본 메서드 `s()`·`set_enabled`·`set_focused`·`set_active`·`set_scale`·`set_halign/valign`·`set_help`·`note_value/last_value`·`accent_now`·`draw_focus_ring`·도움말 배지/툴팁(`UC/mod.rs:278-487`).
- **보고 방식 = 1회성 `take_*`**: 컨트롤은 실행하지 않고 호스트가 수거한다(`take_clicked`·`take_toggled`·`take_changed`·`take_picked`·`take_action`). 콜백·채널 없음.
- **입력 어휘**: `InputEvent{Wheel, HWheel, Key{key,shift,primary}, Char{c,now_ms}, SelectAll, Undo, Redo, MouseDown{x,y,shift,primary}, RightDown, MouseMove, MouseUp}`(`U/event.rs:49-116`) · `Key` 16종(`U/event.rs:12-45`). **더블클릭·가운데 버튼·Alt 수식키·F키·Tab·Backspace(Key)는 어휘에 없다**(Backspace = `Char('\u{8}')` — `U/event.rs:35`).
- **팝업 z순서**: 팝업을 가진 컨트롤은 호스트가 "다른 것을 다 그린 뒤" 다시 그린다(`paint_popup`/`paint` 재호출). 공용 Overlay 스택은 아직 없다(`UDoc/TODO.md` F-2 ☐).
- **시간 주입**: 페이드·타이머는 `tick(now_ms) -> bool`(다시 그릴지)로 호스트 시계를 받는다.
- **앱 주입**: 문자열(`set_ctl_labels` — `UC/mod.rs:135`) · 크기 배율(`set_control_size_mult` — `UC/mod.rs:85`) · 편집 메뉴 아이콘/단축키(`set_edit_menu_decor` — `UC/editmenu.rs:28`) · 메뉴 아이콘 on/off(`set_menu_icons` — `UC/ctxmenu.rs:28`) · 버튼 연타 가드(`set_default_click_guard_ms` — `UC/button.rs:25`).

---

## 1. 컨트롤 표

"nexa-sql 사용" = `S/` 안에서 그 타입 이름이 나오는 파일 수(Grep `-w`, 2026-10-03).

| ID | 컨트롤 | 공개 API 요약 | 동작 | 한계 | 위치 |
|---|---|---|---|---|---|
| UIK-001 | 공통 베이스 `Widget`/`Control`/`ControlBase`/`Invalidations`/`InputEvent` | §0-3 | 포커스 링 · 창 활성 무채화 · "?" 도움말 배지/툴팁 · 직전 확정값 | 포커스 이동(Tab 순서)·레이아웃·컨테이너·이벤트 버블 없음(전부 호스트) · 더블클릭/가운데 버튼/Alt 없음 | `UC/mod.rs:228-487` · `U/widget.rs:14-56` · `U/event.rs:12-134` |
| UIK-002 | 공통 헬퍼(글리프·측정) | `draw_checkbox_glyph`·`draw_radio_glyph`·`draw_updown_chevrons`·`draw_check_mark`·`draw_chevron_down/right/90(_in)`·`image_fit_contain/cover`·`wrap_text`·`fallback_file_icon(is_dir)`·`ProbeCtx`·`ctl_size`·`LEADING_ICON=13`·`BorderSpec`·`HAlign/VAlign/LabelSide` | 순수 그리기/계산 | `fallback_file_icon`은 16px 2종(폴더/파일)뿐 | `UC/mod.rs:147-796` |
| UIK-003 | `Button`(+`ButtonMode`·`ButtonTone`·`ImageFit`) | `new(label)`·`glyph(MenuIcon)`·`icon(Rc<IconImage>)`·`with_tone/font/image/label`·`image_fill(fit)`·`image_front(on)`·`set_label`·`set_tone`·`take_clicked()->bool`·`set_rapid`·`set_click_guard_ms`·`tick`·`is_animating`·`clear_transient`·`is_pressed` | MouseDown=눌림+포커스 · **MouseUp 안쪽=클릭** · Enter/Space(포커스 시) · hover 페이드 · 350ms 연타 가드 · `enabled=false`면 입력 무시 | 토글(체크) 상태 없음 · 드롭다운 분할 버튼 없음 · 툴팁 없음(도움말 배지만) | `UC/button.rs:22-413` |
| UIK-004 | `Carousel` | `new(item_px, gap)`·`set_count`·`item_rect(i)->Option<Rect>`·`left_rect/right_rect`·`take_clicked()->Option<usize>`·`set_scroll_inverted` | 가로 픽셀 스크롤 띠 · 좌/우 고정 오버레이 버튼(페이지) · HWheel(hover 중) · MouseDown=아이템 클릭 | 아이템 그리기는 소유자 몫 · 세로 휠·키보드 없음 · 정사각 아이템 전제 | `UC/carousel.rs:24-217` |
| UIK-005 | `Checkbox` | `new(label, checked)`·`with_label_side`·`is_checked`·`set_checked`·`take_toggled()->Option<bool>` | MouseDown(글리프+라벨 전역)=토글 · Space/Enter | **3상태 없음** · `on_event`에 `enabled` 검사 없음(`UC/checkbox.rs:118-137`) | `UC/checkbox.rs:19-137` |
| UIK-006 | `ColorPanel`(+`hsv_to_rgb`·`rgb_to_hsv`·`rgba_from_hex`·`rgba_to_hex`) | `new(hex)`·`rgba`·`value_hex`·`set_rgba/set_value`·`take_changed()->Option<String>`·`recent/set_recent`·`hex_focused`·`preferred_size`·`tick` | SV 사각형·색상 막대·투명도 막대 드래그 · `#RRGGBBAA` 입력 · 프리셋 · 최근 8색 | 이벤트 본문 미독(머리말 근거) | `UC/colorpanel.rs:31-256` · 이벤트 `:425` |
| UIK-007 | `ColorPicker` | `new(hex)`·`value_hex`·`set_value`·`take_changed`·`hex_focused`·`preferred_width` | 스와치 + `#RRGGBB` 입력(Enter 확정) + 프리셋 6 클릭=즉시 | 알파 없음 · 프리셋 고정 | `UC/colorpick.rs:36-188` |
| UIK-008 | `Combo`(+`ComboControl`·`ComboCore`·`ComboItem`·`PopupHit`) | `new(items, selected)`·`selected_value`·`select_value`·`set_custom_entry(label, suffix)`·`set_custom_text`·`is_editing`·`editing_input(_ref)`·`editing_popup_open` · 트레이트: `is_open`·`toggle_open`·`close`·`value`·`take_changed()->Option<String>`·`popup_rect`·`popup_hit`·`set_viewport_bottom`·`tick_hover`·`paint_dropdown` | 클릭=열기/닫기 · 항목 클릭=선택 · 바깥 클릭=닫기 · Enter/Space/↓=열기 · ↑↓ 이동 · Enter 확정 · Esc 닫기 · "직접 입력…" 인라인 TextBox | **드롭다운 스크롤 없음**(항목 수×26px 그대로) · 항목 런타임 교체 API 없음(새로 만든다) · 타입어헤드 없음 · 팝업 폭 = 박스 폭 | `UC/combo.rs:44-748` · `:1080-1140` |
| UIK-009 | `Choose`(+`ChoosePicker`) | `new(items, selected, choose_label)`·`with_choose_icon`·`set_picker(Box<dyn ChoosePicker>)`·`is_picking`·`text/set_text`·`take_chose`·`take_text_changed` | 박스 직접 타이핑(문자 단위 삽입/백스페이스) + "Choose…" 항목(어댑터 오버레이 최대 8행 또는 호스트 신호) | 편집은 캐럿 이동·선택·IME 없는 단순 `EditState` · nexa-sql 미사용 | `UC/combo.rs:34-39` · `:777-1010` |
| UIK-010 | `ContextMenu`(+`CtxItem`·`MenuIcon`) | `new`·`open_at(x,y,items,host,text_w)`·`open_beside(x,y,avoid,items,host,text_w)`·`on_event(&ev)->bool`·`take_picked()->Option<String>`·`paint`·`is_open`·`close`·`bounds`·`is_outside_click`·`select(i)`·`set_default(i)`·`hovered`·`set_max_rows/min_rows/max_width`·`set_click_selects`·`replace_items`·`take_reached_end`·`item_ids`·`row_rect_of` | 아이콘·단축키 문구·다단 하위 메뉴·체크(✓)/토글/라디오(active) 표시 · **MouseUp 확정** · ↑↓ PgUp/PgDn Home/End · → 하위 · ←/Esc 닫기 · Enter(hover 또는 기본 항목) · 휠 스크롤(상한 행) · 경계 접기 | **니모닉(가속 글자) 없음 — `Char`는 메뉴를 닫는다**(`UC/ctxmenu.rs:1087-1090`) · `Widget`/`Control` 미구현(자체 API) · 폭 힌트 `text_w`를 호스트가 준다 | `UC/ctxmenu.rs:55-1139` |
| UIK-011 | `EditMenu`(+`EditMenuCaps`·`EditMenuAction`·`EditMenuDecor`) | `new`·`open_at(x,y,scale,host,caps,extra)`·`on_event`·`take_action()->Option<EditMenuAction>`·`is_open`·`close`·`bounds`·`paint` · 전역 `set_edit_menu_decor` | 복사·잘라내기·붙여넣기·―·전체 선택 고정 순서 · 활성 게이트(선택 유무·클립보드·읽기 전용) · `extra` 항목 앞에 끼움 | 실행 취소/삭제 항목 없음(4종 고정) · 클립보드는 호스트 | `UC/editmenu.rs:16-185` |
| UIK-012 | `IconDropdown`(+`IconDropItem`) | `new(items, value)`·`value()->&'static str`·`set_value`·`take_changed`·`is_open`·`paint_popup` | 버튼(현재 아이콘+▾) 클릭=팝업 · 행 클릭=선택 · 밖=닫기 · Esc | 항목이 `&'static`(값·알파 마스크) — **런타임 목록 불가** · 키보드 이동 없음 | `UC/icondrop.rs:26-270` |
| UIK-013 | `ListEditor` | `new(joined, placeholder)`·`with_empty_label`·`value/set_value`·`take_changed`·`is_editing`·`editing_input(_ref)`·`wants_keys`·`overflows`·`preferred_height`·`tick` | 행 클릭=선택 · **선택 행 재클릭=인라인 편집** · ＋/− 버튼 · Delete · Enter 편집 · Esc 취소 · ↑↓ · 휠 | 값 모델 `;` 구분 한 줄 고정 · 보이는 창 5행 고정 · 드래그 재정렬 없음 | `UC/listedit.rs:37-363` |
| UIK-014 | `PairTable`(+`Pair`·`PairKind`·`PairOpts`) | `build(text, hl, opts)`·`marks_in`·`pair_at`·`enclosing`·`parent`·`first_child`·`sibling`·`get/len/is_empty` | 괄호·인용부호 쌍 표(편집기용 순수 로직) | **시각 컨트롤 아님** — 파일 탐색기에서 직접 쓸 일 없음(TextBox 내부용) | `UC/pairs.rs:12-407` |
| UIK-015 | `PositionDropdown` | `new(code)`·`value`·`select_value`·`take_changed`·`is_open`·`set_max_bottom`·`paint_popup` | 머리 클릭=3×3 팝업 · 셀 클릭=선택+닫기 · 방향키·Enter·Esc · Space/Enter로 열기 | 9칸 위치 전용 | `UC/posdrop.rs:24-200` |
| UIK-016 | `PositionPicker` | `new`·`value`·`select_value`·`take_changed`·`preferred_size` · 자유 함수 `box_rect_in`·`paint_cell` · `CODES` | 3×3 셀 클릭 · 방향키(경계에서 멈춤) | 9칸 위치 전용 | `UC/posgrid.rs:16-202` |
| UIK-017 | `MenuBar`(+`MenuDef`·`MenuEntry`) | `new(menus)`·`set_menus`·`set_max_label_width`·`is_open`·`take_picked()->Option<String>`·`dismiss`·`popup_bounds` | 라벨 클릭=토글 · 열린 채 다른 라벨 hover=전환 · 항목 **MouseDown=실행** · ↑↓(순환) ←→(메뉴 전환) Enter/Space Esc · 1단계 하위 메뉴 · 강조/비활성 항목 | **단축키 칸 없음 · 체크/라디오 표시 없음 · 니모닉(Alt) 없음** · 하위 1단계 · 주 드롭다운 창 밖 클램프 없음(`UC/pulldown.rs:195-212`) · 스크롤 없음 | `UC/pulldown.rs:25-729` |
| UIK-018 | `RadioGroup`(+`RadioOption`) | `new(options, selected)`·`selected`·`selected_value`·`select_value`·`take_changed`·`preferred_height` | 세로 나열 · 행 클릭=선택 · ↑↓ | 가로 배치 없음 · 히트가 y만 본다(`opt_at` — `UC/radio.rs:100-107`) · `enabled` 검사 없음 | `UC/radio.rs:14-162` |
| UIK-019 | `Switch` | `new(label, on)`·`with_label_side`·`set_track_scale`·`is_on`·`set_on`·`take_toggled` | MouseDown=토글 · Space/Enter | `enabled` 검사 없음(`UC/switch.rs:145-164`) | `UC/switch.rs:32-164` |
| UIK-020 | `TabBar`(+`TabAction`·`TabBadge`) | `new`·`set_tabs(titles, active, inv)`·`set_active`·`active/len`·`set_locked/pinned/group/dirty/badges/tab_colors/title_colors`·`set_multiline`·`set_show_new`·`set_close_last`·`set_close_always`·`set_accent`·`set_metrics(row_h,pad_x)`·`preferred_height`·`lines`·`take_lines_changed`·`take_action()->Option<TabAction>`·`last_click_mods`·`tab_index_at`·`tab_rect`·`empty_area_at`·`middle_down`·`dragging/pressed_tab/begin_drag/cancel_drag`·`scroll_x/scroll_by`·`badge_rect_of` | 단일행(◀▶+휠 스크롤) / 다중행(줄바꿈) · **탭 전환 = MouseUp**(같은 탭 위) · × 누름→뗌=닫기 · [+] · 드래그 재정렬(8px 임계·고스트) · 우클릭=`Context(i)` · 잠금/핀/미저장/묶음/색 | **탭별 아이콘(이미지) 없음**(`TabBadge` 4종 고정) · 툴팁 없음 · 키보드 없음 · 더블클릭은 호스트가 `tab_index_at`/`empty_area_at`로 | `UC/tabbar.rs:29-968` |
| UIK-021 | `TimeoutButton`(+`FiredBy`) | `new(label, total_ms)`·`with_show_remaining/two_line/warn/suffix`·`start(now)`·`tick(now)`·`remaining_ms/secs`·`expired`·`take_fired()->Option<FiredBy>`·`fired_by_timeout` | 카운트다운 게이지+초 표시 · 만료=자동 발화 · 클릭 · **Esc=즉시 발화(포커스 무관)** | Esc를 포커스 검사 없이 먹는다(`UC/timeout_button.rs:219-226`) — 호스트가 라우팅 주의 | `UC/timeout_button.rs:24-229` |
| UIK-022 | `Toolbar`(+`ToolItem`·`ToolIcon`·`ToolTone`·`DropClick`) | `new(items)`·`set_icon_size`·`set_padding`·`preferred_height/width`·`take_clicked()->Option<String>`·`items`·`item_rect(id)`·`left_items_end`·`slot_px`·`set_item_icon/visible/tone/enabled/badge/label/tip`·`item_enabled`·`clear_hover`·`set_tooltip_above`·`paint_tooltip(_in)`·`paint_offset` | hover=반투명 배경 · 눌림 · **MouseUp=클릭** · ▾ 분리(`"<id>#drop"`) · 구분자 · 배지 · 글자 항목 · 오른쪽 정렬 · 툴팁(팝업 층) · 연타 가드 | **토글(체크) 상태 필드 없음**(색조 `tone`으로 대체) · 폭 넘침 처리(오버플로 ») 없음 · 키보드 없음 | `UC/toolbar.rs:24-649` |
| UIK-023 | `ToolDock`(+`ToolGroup`·`DockLayout`·`DockAction`) | `new(groups)`·`set_icon_size`·`preferred_height`·`rows`·`bar/bar_mut(id)`·`groups`·`title`·`is_floating`·`float/dock/set_floating_pos/floating_pos`·`layout/apply_layout/reset`·`take_actions()->Vec<DockAction>`·`take_clicked`·`set_item_*`·`item_rect`·`group_of`·`is_dragging/cancel_drag`·`relayout_if_stale`·`paint_drag_overlay`·`paint_tooltip` · `DockLayout::parse/serialize` | 그룹 그립 드래그=순서/행 이동(고스트·Esc 취소) · 멀리 끌면 `Float{id,x,y}` · 배치 문자열 1개로 저장 | **툴바 그룹 전용**(패널 도크 아님) · 플로팅 창은 호스트가 만든다(UIK-108) · 세로 도크 없음 | `UC/tooldock.rs:25-881` |
| UIK-024 | `TreeModel`/`TreeNode`/`FlatRow`/`TreeControl` | `TreeNode::leaf/branch/with_cells/with_image` · `TreeModel::new/roots/roots_mut/invalidate/flatten()->Rc<Vec<FlatRow>>/toggle/set_expanded` · 트레이트: `rows`·`move_selection`·`reveal_row`·`page_rows`·`toggle_row`·`expand_selected`·`row_hit(x,y)->Option<(usize,bool)>`·`paint_tree_cell`·`tick_hover` | 평탄화 캐시 · 클릭=선택 · 셰브론 클릭=펼침 · ↑↓ PgUp/PgDn Home/End · →펼침 · ←접기/상위 이동 · Enter/Space=토글 | **즉시(eager) 모델** — 지연 로딩·로딩/오류 상태 없음 · `flatten`이 행마다 라벨·셀 `String` 복제 · 행 식별 = 인덱스 경로(`Vec<usize>`) | `UC/tree.rs:27-487` |
| UIK-025 | `TreeView` | `new(model)`·`set_border`·`selected_label`·`tick` + `TreeControl` | 단일 열 · hover 페이드 · 오버레이 스크롤바 | 단일 선택 · 더블클릭/우클릭 보고 없음 · 행 높이 24 고정 · **부분적으로 보이는 행은 그리지 않는다**(`UC/tree.rs:603-605`) | `UC/tree.rs:493-629` |
| UIK-026 | `TreeGrid`(+`GridColumn`) | `new(model, columns)`·`set_fit_columns`·`set_caret_outline`·`set_marked_paths`·`is_marked`·`set_border`·`set_column_width(i,w)`·`clear_selection`·`has_selection`·`tick` · `GridColumn::new(title,width).with_badge(b)` | 첫 열=트리 · 나머지=문자열 셀 · 헤더(제목+정렬 배지) · 가로/세로 스크롤 · 다중 선택 "표시"(marked) | **헤더 클릭/경계 드래그를 스스로 처리하지 않는다**(호스트가 히트·`set_column_width` — `UD/lib.rs:1981`·`:2449`) · 다중 선택 로직 없음(호스트가 `marked` 계산) · 셀 왼쪽 정렬·텍스트만 · 행 24/헤더 26 고정 · 보기 모드 1종 · 가상화는 "보이는 구간만 그림"이지만 **모델은 전량 메모리** | `UC/tree.rs:645-952` |
| UIK-027 | (범위 밖) `ScrollBars`·`Splitter`·`TextBox`·`Flash`·`glyph`·`TypeAhead`·`ViewMode`·`draw_tooltip`·`ellipsize_middle` | 재수출만 확인 | — | 상세는 다른 문서 | `UC/mod.rs:53-70` · `U/lib.rs:64-80` · `U/draw.rs:227-309` |
| UIK-028 | 컨트롤 크기 배율 | `set_control_size_mult`·`control_size_mult`·`control_size_mult_from_code("s/m/l/xl")`·`ctl_size` | 체크·라디오·스위치 글리프만 0.8/1.0/1.3/1.6배 | 행 높이·여백은 불변 | `UC/mod.rs:82-151` |
| UIK-029 | 컨트롤 내장 문자열 주입 | `CtlMsg{CtxSelectAll,CtxCopy,CtxCut,CtxPaste}`·`set_ctl_labels(fn)`·`ctl_label` | 미주입 = 영어 | `OnceLock` — **1회만 주입**(이후 호출 무시). 공급자가 앱 i18n `t()`를 부르면 언어 전환은 반영 | `UC/mod.rs:110-143` |

**nexa-sql의 실제 사용 빈도**(파일 수): TextBox 30 · Button 14 · ContextMenu 14 · ScrollBars 12 · Switch 8 · Toolbar 6 · TypeAhead 4 · Checkbox 3 · ToolDock 3 · TabBar 2 · Combo 2 · TimeoutButton 2 · MenuBar 1 · TreeView 1(`S/prefs_win.rs`) · Splitter 1 · ColorPanel 1 · PositionDropdown 1 · Flash 1 · **TreeGrid 0 · EditMenu 0(직접 사용) · Choose 0 · RadioGroup 0 · ListEditor 0 · ColorPicker 0 · IconDropdown 0 · PositionPicker 0(직접) · Carousel 0 · PairTable 0(직접)**. → 사용 0인 컨트롤은 nexa-sql에서 실전 검증이 없다(TreeGrid는 nexa-dlg `FilePicker`가 쓴다 — `UD/lib.rs:1398-1416`).

---

## 2. 컨트롤별 상세

### UIK-001 공통 베이스

- `Widget`(`U/widget.rs:44-56`) · `Invalidations::push/is_empty/drain` — 교차 rect는 union 병합(`U/widget.rs:20-40`).
- `Control`(`UC/mod.rs:278-487`): `base()/base_mut()`만 구현하면 나머지 상속. `set_enabled(false)`는 포커스도 내린다(`:295-300`). `set_scale`은 0.5 하한(`:323-325`).
- 포커스 링 = 본체 바깥 2px·반경 6·`theme.focus_ring` 50%(`:396-410`). 도움말: `set_help`+`set_show_help(true)` → "?" 배지(지름 18·본체 오른쪽 8px) → 클릭 토글 → 말풍선(폭 280 줄바꿈)(`:413-486`). 배지 밖 클릭은 툴팁만 닫고 이벤트는 흘린다(`:447-457`).
- `InputEvent`(`U/event.rs:49-116`): 좌표는 창 클라이언트 물리 px. `Key`는 `Up/Down/PageUp/PageDown/Home/End/Right/Left/Space/Enter/Escape/Delete/WordLeft/WordRight/SubwordLeft/SubwordRight`(`:12-45`). `primary` = mac ⌘ / 그 외 Ctrl(번역은 호스트).
- `WheelAccum::add(delta, lines_per_notch)` — 분수 노치 누적(`:120-134`).
- **한계(파일 탐색기 관점)**: ① 더블클릭 사건 없음 → 각 소비자가 `Instant`로 400ms 판정(예 `S/explorer.rs:949`·`UD/lib.rs:2643`) ② 가운데 버튼 없음 → `TabBar::middle_down`처럼 별도 메서드 ③ Alt/Ctrl+Shift 조합·F2/F5/Tab/Backspace 키 없음 → 명령 키는 전부 호스트 키맵 ④ 드래그 앤 드롭(외부) 사건 없음.

### UIK-003 Button

- 생성: `Button::new(label)`(`UC/button.rs:134`) · `Button::glyph(MenuIcon)`(`:156` — 글자색 틴트 도형) · `Button::icon(Rc<IconImage>)`(`:165`).
- 빌더: `with_tone(ButtonTone::{Default,Safe,Danger,Accent})`(`:76-86`·`:225`) · `with_font(FontSlot)`(`:237`) · `with_image`(`:244`) · `with_label`(`:256`) · `image_fill(ImageFit::{Contain,Cover})`(`:263`) · `image_front(bool)`(`:271` — 이미지 맨 앞 고정 + 텍스트는 `HAlign`대로) · `with_fade_speed`(`:203`).
- 이벤트(`:372-413`): 비활성 → hover/눌림 해제 후 무시 · MouseMove=hover 목표 · MouseDown(안쪽)=`pressed`+**포커스 획득**(도움말 배지 우선) · MouseUp(눌린 상태+안쪽)=`fire_click` · Enter/Space(포커스).
- 연타 가드(`:339-350`): `rapid=false`이고 가드>0이면 직전 클릭 후 가드 시간 안의 클릭은 버림. 전역 기본 350ms(`:22-33`), 버튼별 `set_click_guard_ms(Some(ms))`, 연타가 뜻인 버튼은 `set_rapid(true)`.
- `clear_transient()`(`:217`) — 가려지거나 교체될 때 hover/눌림/포커스 초기화(호스트가 부른다).

### UIK-008 Combo / UIK-009 Choose

- `ComboItem{value, label, icon: Option<String>, image: Option<Rc<IconImage>>}` + `new`/`with_icon`/`with_image`(`UC/combo.rs:44-77`). **MenuBar·pulldown도 이 타입을 항목으로 쓴다**(`UC/pulldown.rs:27-35`).
- `ComboCore`(`:113-163`): `items()`·`selected_index()`만 공개, 생성은 `Combo::new`/`Choose::new` 경유.
- 공통 이벤트 `combo_event`(`:1080-1140`): MouseMove=박스 hover + 열린 목록 항목 hover(의도 코얼레싱) · MouseDown: 열려 있으면 항목=`choose_index` / 확장 항목=`on_extra` / 밖=닫기, 닫혀 있으면 박스 클릭=포커스+열기 · Key(포커스): Esc 닫기 · Enter/Space 확정 또는 열기 · ↓ 이동 또는 열기 · ↑ 이동.
- 팝업 배치(`:309-338`): 박스 아래 2px · 높이 = 항목×26 + 여백 · 하한(`set_viewport_bottom` 또는 마지막 paint가 본 표면 높이)을 넘으면 **시작 y를 위로 옮긴다**(스크롤은 없음).
- `Combo` 직접 입력(`:583-592`·`:673-698`): `set_custom_entry("직접 입력…", "ms")` → 구분선 아래 항목 → 내장 `TextBox`(숫자 6자 또는 `set_custom_text(true)`면 ASCII 253자) · Enter/바깥 클릭=확정 · Esc=취소(`:720-748`). 호스트는 `editing_input()`으로 IME·클립보드를 잇는다.
- `Choose`(`:777-1010`): 박스에 직접 타이핑(`Char`를 `EditState`에 삽입 — `:997-1008`) · "Choose…"는 `set_picker`가 있으면 인라인 오버레이(제목+최대 8행+스크롤바 — `:840-872`), 없으면 `take_chose()` 신호.

### UIK-010 ContextMenu

- 항목 `CtxItem::Item{id, label, enabled, icon, shortcut, sub, emph, marks, children, checked, active, mark}` / `Separator`(`UC/ctxmenu.rs:101-132`).
  - 생성자: `item(id,label)`(`:137`) · `maybe(id,label,enabled)`(`:155`) · `submenu(id,label,children)`(`:173` — 활성 자식이 없으면 비활성).
  - 빌더: `with_icon(Option<MenuIcon>)`(`:198`) · `with_shortcut(s)`(`:233` — **표시 전용**) · `with_checked(bool)`(`:259` — 토글 아이콘) · `with_mark(bool)`(`:252` — ✓ 칸 예약) · `with_active(bool)`(`:243` — 라디오 "지금 것" 강조) · `with_emphasis`·`with_marks`·`with_sub`(완성 팝업용).
- `MenuIcon{w,h,alpha: Rc<[u8]>, rgba: Option<Rc<[u8]>>}` — `from_alpha`(상태색 틴트) / `from_rgba`(OS 아이콘 그대로)(`:55-97`).
- 열기: `open_at(x, y, items, host, text_w)`(`:528-559`) — 빈 목록이면 닫힘 · `geom::place_popup`으로 경계 접기. `open_beside(x, y, avoid, …)`(`:563-582`) — 대상 행을 가리지 않게 아래/위. 호출 전 `set_scale`(`:380`).
- 이벤트 계약 `on_event(&ev) -> bool`(`:812-1093`): `true` = 소비.
  - 마우스: MouseMove=hover(+하위 메뉴 자동 펼침) · MouseDown=누름 기억 · **MouseUp=놓인 자리의 활성 항목 확정**(Down과 달라도) · 메뉴 밖에서 놓으면 닫힘 · 바깥 MouseDown/RightDown=닫고 **소비** → 호스트는 `is_outside_click(ev)`(`:505-515`)로 먼저 판정해 바깥 클릭을 아래로 흘린다.
  - 하위 메뉴: hover/클릭/→/Enter로 펼침 · ←/Esc로 접힘 · 대각선 이동 유예 400ms(`:329`·`:843-861`).
  - 키: ↑↓ · PgUp/PgDn · Home/End · → · ←(하위 없으면 닫기) · Enter(hover → 없으면 `set_default` 항목) · 그 외 키와 **모든 `Char`는 닫기**(`:1083-1090`).
  - 휠: 커서가 팝업 안일 때만 소비(`:976-1002`) · HWheel=넘친 라벨 가로 스크롤(`set_max_width` 사용 시).
- 긴 목록: `set_max_rows(Some(n))` + 휠/키 스크롤 · `take_reached_end()` → `replace_items()`로 페이지 이어 붙이기(`:385`·`:451-456`).
- 결과: `take_picked() -> Option<String>`(`:1134`). 그리기는 **맨 마지막 층**에서 `paint`(`:1139`).

### UIK-017 MenuBar(pulldown)

- 모델: `MenuDef{label, entries}` · `MenuEntry::{Item(ComboItem), Emph(ComboItem), Disabled(ComboItem), Separator, Sub(ComboItem, Vec<MenuEntry>)}`(`UC/pulldown.rs:25-70`). 하위는 **1단계만**(안의 `Sub`는 일반 항목처럼 그려지고 열리지 않음 — `:35`).
- 이벤트(`:455-585`): 라벨 MouseDown=토글 · 열린 상태 MouseDown: 하위 항목 → `pick_sub`, 항목 → `pick`(실행은 **MouseDown**), 그 밖 = 닫기 · MouseMove: 열린 채 다른 라벨 = 전환, `Sub` 행 = 펼침, 다른 행 = 접힘 · 키(열렸을 때만): Esc · ↑↓(구분선·비활성 건너뛰고 순환 — `:398-419`) · → (`Sub`면 펼침, 아니면 다음 메뉴) · ← (이전 메뉴 / 하위 접기) · Enter/Space.
- 그리기(`:587-729`): 바 = `chrome_bg` + 하단 1px · 드롭다운 폭 = 최대 라벨 폭(상한 `set_max_label_width` 기본 480 → 가운데 `…`) + 아이콘 칸 · 항목 = [아이콘 13px][라벨][`›`]. **행에 그리는 것은 아이콘·라벨·하위 화살표뿐**이다 — 단축키·체크 칸이 없다(`paint_panel` `:665-729`).
- 결과: `take_picked() -> Option<String>`(= `ComboItem.value`) · 호스트는 `is_open()`/`popup_bounds()`로 모달 캡처·재도색 범위를 정한다.
- **dir2 대비 차이**(`P/14-dir2-gui-widgets.md` GUI-060·GUI-063·GUI-066): dir2 `MenuItem{id:u32, label, shortcut, checked:Option<bool>, submenu}` + `set_checked(id,on)` + 드롭다운의 단축키 열 → nexa-ctl에는 **shortcut/checked 필드와 `set_checked`가 없다**. nexa-sql도 메뉴에 단축키를 표시하지 않는다(`S/app/menus.rs:1039-1072` — `ComboItem::new(id, t(m))`만). → UIK-205.

### UIK-020 TabBar

- 이식 원본 = dir2 `widgets/tabbar.rs`(`UC/tabbar.rs:1`).
- 동작 보고 `TabAction::{Switch(i), Close(i), New, Move{from,to}, Context(i), Badge(i), BadgeContext(i)}`(`:29-49`) — `take_action()` 1회성.
- 이벤트(`:877-968`): MouseDown 탭 본체 = 수식키 기억 + 드래그 후보(전환은 아직) · × 상자 = 누름 기억 · [+]/◀▶ = 누름(◀▶는 즉시 스크롤) · **MouseUp = 같은 탭 위거나 드래그가 시작됐으면 `Switch`** · × 같은 상자면 `Close` · `Plus`면 `New` · RightDown = `Context(i)`/`BadgeContext(i)` · MouseMove = 드래그 이동 또는 hover · Wheel/HWheel = 단일행 스크롤(노치당 48px).
- 표시 속성: 잠금(자물쇠·닫기 불가) · 핀(점 표식) · 묶음(상단 줄) · 미저장(동그라미↔×) · 탭별 상단 줄 색 · 제목 글자 색 · 앞 표식 `TabBadge::{None,Link,LinkOff,Shared}`(`:54-64`).
- 지표: 줄 높이 28 · 탭 최대 폭 240 · 드래그 임계 8 · 닫기 상자 16(`:95-107`). 다중행은 paint가 줄 수를 재고 `take_lines_changed()`로 호스트 재배치 요청(`:424-432`).
- 패널 간 탭 이동용: `dragging()`·`pressed_tab()`·`begin_drag(index,x,y)`·`cancel_drag()`(`:477-498`).
- **dir2 대비 차이**(`P/14…` GUI-041·042·044·045): dir2는 탭 전환·닫기가 **MouseDown 즉시**, 잠금/핀 표지가 제목 앞 `🔒`/`📌` 접두. nexa-ctl은 **MouseUp 전환**·자물쇠 상자·핀 점. → dir3에서 "컨트롤 배치·기능 유지" 기준으로는 허용 가능한 UX 차이지만 회귀 테스트 기대값을 nexa-ctl 규약으로 다시 써야 한다.

### UIK-022 Toolbar / UIK-023 ToolDock

- `ToolItem{id, icon, right, visible, tip, enabled, tone, dropdown, drop_click, separator, badge, label}`(`UC/toolbar.rs:93-120`) · 생성 `new(id, icon)`·`text(id,label)`·`separator()` · 빌더 `with_dropdown`·`drop_click(DropClick::{Same,Separate})`·`tone`·`disabled`·`align_right`·`hidden`·`tip`(`:125-202`).
- `ToolIcon::{Glyph(String), Image(Rc<IconImage>), Mask{..}, StatusMask{..}, Avatar{..}}`(`:24-66`) — `Mask` = 테마색 틴트·hover=accent, `Image` = 원본 색.
- 이벤트(`:593-640`): MouseDown=눌림 · MouseUp(같은 항목)=`clicked = id` 또는 `"<id>#drop"` · 같은 id 연타는 전역 가드로 1회 · MouseMove=hover(툴팁 띠 40px까지 무효화).
- 아이콘 크기 `set_icon_size(px)` 기본 32(`:219`·`:281`) · 권장 높이 = 아이콘 + (슬롯 여백+바 여백)×2(`:293`).
- 툴팁은 팝업 층에서 `paint_tooltip` / `paint_tooltip_in(clamp_x)`(`:506-517`). 지연 시간 유무는 본문 미독(추정: 지연 없음).
- `ToolDock`: 그룹마다 `Toolbar`를 도크가 소유(`UC/tooldock.rs:7-9`) · 배치 문자열 `a,b;c@120:340,d`(`,` 같은 행 · `;` 다음 행 · `@x:y` 플로팅 — `:78-120`) · `DockAction::{Float{id,x,y}, LayoutChanged, Resized}`(`:144-150`) · 드래그 시작 4px · 떼어 내기 22px(`:25-31`) · 이벤트는 "커서 아래 그룹에만" 라우팅(`:795-800`).
- **dir2 대비 차이**(`P/14…` GUI-070·072·074): dir2 `ToolButton{id:u32, glyph, checked, icon, enabled, tip}`는 **MouseDown 즉시** 통지 + `checked` = accent 38% 배경. nexa-ctl `ToolItem`에는 **`checked`가 없다**(상태는 `tone`·`badge`로만) → UIK-206.

### UIK-024~026 트리 계열

- 이벤트 `tree_event`(`UC/tree.rs:420-487`): ① 스크롤바 먼저(`ScrollBars::on_event`) — 소비되면 끝 ② MouseMove=행 hover(페이드) ③ MouseDown=`row_hit` → 선택(+셰브론이면 토글) ④ Key(**포커스일 때만**): ↑↓ · PgUp/PgDn(`page_rows`) · Home/End · →=펼침 · ←=접기 또는 상위 행으로 · Enter/Space=토글.
- `row_hit`(`:337-353`)은 x·y 둘 다 검사 · 셰브론 영역 = 깊이×16 들여쓰기 지점의 16px.
- `TreeGrid` 추가 계약: `has_sel`(빈 곳 클릭 해제는 호스트가 `clear_selection()` 호출) · `marked`(노드 경로 집합 — 가시 인덱스가 아니라서 펼침/접힘에도 유지 — `:691-693`) · `caret_outline`(다중 선택 모드에서 캐럿 = 테두리만) · `fit_columns`(열 합 밖 = 빈 공간).
- 한계 요약(파일 탐색기 목록으로 쓰기에 부족한 점):
  1. 데이터가 `TreeNode{label: String, cells: Vec<String>}` 전량 보유 + `flatten` 복제(`:146-163`) → 10만 행 폴더에 부적합(nexa-ui 자체 평가도 동일: `UDoc/21-grid-family.md` — 가상화 그리드는 U-3 `nexa-grid`로 예정).
  2. 헤더 정렬 클릭·경계 리사이즈·열 순서 이동·열 숨김이 컨트롤 밖(호스트).
  3. Shift/Ctrl 다중 선택·러버밴드·전체 선택·타입어헤드·인라인 이름 변경·드래그 시작 없음.
  4. 셀 정렬(크기 열 오른쪽 정렬)·셀 색·셀 아이콘·행 상태(잘라내기 고스트 등) 없음.
  5. 보기 모드(상세/목록/아이콘/썸네일) 없음.

### 나머지 소형 컨트롤(요약)

- **UIK-004 Carousel**: 버튼 영역도 내용 영역(아이템이 밑으로 지나감) · 경계 마스크를 `paint`가 그린다 → 호출 순서 = 소유자가 `item_rect`로 아이템 그림 → `Carousel::paint`(`UC/carousel.rs:1-12`·`:219-`).
- **UIK-005 Checkbox / UIK-019 Switch**: `LabelSide::{None,Left,Right}`(`UC/mod.rs:216-224`) · 히트 = bounds 전체 · 글리프 12px(체크)·20×12(트랙) × 크기 배율.
- **UIK-013 ListEditor**: 편집 중 다른 행/바깥 클릭 = 지금 값으로 확정 후 그 클릭 계속 처리(`UC/listedit.rs:273-282`) · 빈 값 확정 = 그 행 삭제(머리말).
- **UIK-021 TimeoutButton**: `start(now_ms)` 이후 `tick(now_ms)`가 만료 시 `fired = Timeout`(`UC/timeout_button.rs:113-134`) · 클릭/만료 구분 = `FiredBy`.

---

## 3. nexa-sql 앱 내 범용 UI — "nexa-ui에 없어서 앱이 직접 만든 것"

모두 `pub(crate)`이며 nexa-sql 도메인 타입에 묶여 있다. **그대로 가져다 쓸 수 없고**, 범용 부분을 nexa-ui로 승격하거나 dir2 원본을 이식해야 한다.

| ID | 앱 UI | 범용으로 쓸 만한 부분 | 도메인 결합(그대로 못 쓰는 이유) | 파일 탐색기 대응 | 위치 |
|---|---|---|---|---|---|
| UIK-101 | 결과 그리드 `Grid` | 픽셀 단위 가상 스크롤 · 오버레이 스크롤바 · 컬럼 폭 실측(앞 200행) · 헤더 경계 드래그 리사이즈 + 더블클릭 자동 맞춤(추정 — `autofit`/`edge_click` 필드) · 헤더 드래그 컬럼 이동(고스트 · Esc 복원) · 헤더 클릭 정렬(Shift = 다중 키) · 다중 사각 영역 선택(클릭/드래그/Shift/primary 토글) · 행번호 거터 · 키 이동(↑↓←→ Home End PgUp PgDn Space) · 우클릭 메뉴 · 복사(TSV 등) · 필터 · 푸터 · 고속 스크롤 | `ResultData`·`nsql_core::RowSource`·`Value`·`Dialect`·페치/편집/SQL 생성이 한 구조체(필드 약 130개)에 섞여 있음 | 파일 목록 상세 보기의 컬럼 조작 UX 참고. **코드 재사용 불가** — nexa-ui 계획도 "U-3 `nexa-grid`가 오면 교체"(`S/grid.rs:1-3`) | `S/grid.rs:131-153`(DragSel·HdrDrag) · `:274-482`(구조체) · `:4461-4570`(정렬·헤더 히트) · `:4745-5203`(on_event) · `:5203-5860`(paint) |
| UIK-102 | 오브젝트 탐색기 `Explorer` | **지연 로딩 트리**(노드 아레나 `Vec<Node>` + `LoadState{Idle,Loading,Loaded,Partial,Error}`) · 워커 스레드 요청/응답 채널 · 클릭 선택 / 400ms 더블클릭 = 활성 / 글리프 = 펼침 · 우클릭 = `ContextMenu::open_beside` · ↑↓←→ Home End PgUp PgDn Enter Esc · `+`/`-`/`*` 펼침 · **타입어헤드**(nexa-ctl `TypeAhead`) · 필터(일치+조상만) · hover 툴팁 · 가로 스크롤(내용 폭) · 아이콘 캐시 · 소프트 새로고침(펼침 유지 diff) · 새 노드 강조 | 카탈로그 SQL·세션·DBMS 종류에 전부 결합(필드 약 110개) | 폴더 트리(지연 열거 · 오류 노드 · 새로고침 diff)의 설계 참고. nexa-ctl `TreeView`가 이를 못 해서 앱이 직접 만든 대표 사례 | `S/explorer.rs:194-201`(LoadState) · `:248-255`(Node) · `:944-949`(지표) · `:951-1133`(구조체) · `:6975-7315`(on_event) · `:7315-7400`(on_char/타입어헤드) · `:7821-`(paint) |
| UIK-103 | 탐색기 묶음 `Explorers` | 여러 트리를 **한 스크롤**로 이어 붙이기(`set_clip`) · 칸 경계 키 이동(`cross_pane`) | 서버·세션 수명 규칙 | 드라이브/장소 여러 루트를 한 트리로 잇는 방식 참고 | `S/explorers.rs:1-12` |
| UIK-104 | 명령 팔레트 `Palette` | 입력 한 줄(`TextBox`) + 퍼지 필터 목록(최대 12행 · ↑↓ · 휠 · Enter · Esc · 바깥 클릭) · 프롬프트 모드(대상 rect 옆에 붙는 한 줄 입력 = **인라인 이름 바꾸기 대용**) · `fuzzy_score` | i18n `Msg` · 명령 id 문자열 | 명령 팔레트 · 빠른 이동(경로 점프) · 탭 이름 바꾸기 프롬프트 | `S/palette.rs:12-52` · `:54`(MAX_ROWS) · `:261-`(on_event) · `:525`(fuzzy_score) |
| UIK-105 | 찾기 막대 `FindBar` + 아이콘 토글 `FindBtn` | **아이콘 토글 버튼**(On/hover 페이드/포커스 3상태 · 툴팁 · 마스크 아이콘 지연 생성) · 플로팅 위젯 배치 · 검색어 이력 | 편집기 찾기/바꾸기 전용 배치(VS Code 치수) · `BtnKind`가 고정 열거 | 파일 내용 찾기(미리보기) · **토글 버튼 부품**(nexa-ctl `Button`에 토글이 없어 앱이 만든 것) | `S/findbar.rs:34-66` · `:129-146`(FindBtn) · `:298-`(FindBar) · `:693`(on_event) |
| UIK-106 | 필터 틀 `FilterBar` + `Matcher` | TextBox + 안쪽 토글(Aa·ab·.*) + 오른쪽 부가 토글(숨김 파일·점 파일·경로 포함) · 이력 드롭다운 · 진행 표시(테두리 회전선) · `Matcher`(낱말 AND / 정규식 / 구조 질의 `size>1G` `type:` `ext`) | `fancy_regex` · `nsql_core::filterq` | **패널 빠른 필터** — 프로젝트 탐색기(파일 트리)가 이미 이 부품을 쓴다 | `S/filterbar.rs:28-60` · `:300-331` · `:547`(on_event) · `:698-719`(matches) |
| UIK-107 | 토스트 `Toasts` | 우하단 쌓임(최대 5) · 수명·페이드(마지막 300ms) · 남은 시간 막대 · 클릭 닫기 · 클릭 동작(`push_action` → `take_action`) · 종류 Error/Warn/Info | 설정 키 이름뿐(거의 범용) | 작업 완료/오류 알림 · 실행 취소 토스트 | `S/toast.rs:19-36` · `:41-60` · `:101-162` |
| UIK-108 | 플로팅 툴바 창 `ToolFloatWin` | `ToolDock`에서 떼어 낸 그룹을 담는 소유 창(winit + softbuffer) · `FloatAction::{Paint, Input, Moved, Close}` | winit·`winfocus::owned_by`·`present::Presenter`(앱 모듈) | ToolDock을 쓰면 **호스트가 반드시 만들어야 하는 짝** | `S/toolfloat.rs:20-80` · `:118` · `:161` |
| UIK-109 | 활동 막대 `ActivityBar` | 48px 세로 아이콘 열 · 패널 토글/교체 · 아래 정렬 동작 버튼 · 숨김 항목 | 항목 id가 `&'static str` · 아이콘 = 앱 `toolicons` | dir2에 대응 UI가 있는지는 미확인(추정: 없음 — dir2는 이중 패널) | `S/activity.rs:1-30` · `:42-202` |
| UIK-110 | 상태바(호스트 직접 그림) | 세그먼트별 rect 기억 → 클릭 히트(`status_*_rect`) · 좌/우 정렬 · 설정으로 세그먼트 on/off | `App` 필드에 흩어진 코드(컨트롤 아님) | **StatusBar 컨트롤이 nexa-ui에 없다는 증거** — dir2 `StatusBar`(`P/14…` GUI-080) 이식 필요 | `S/main.rs:902`(status_h 24) · `S/app/paint.rs:10` · `:110-113` · `:265-326` |
| UIK-111 | 프로젝트 탐색기 `ProjectPanel`(파일 트리) | 지연 열거 폴더 트리(`nexa_fs::list_opts`) · 필터 · OS 아이콘 워커 · 클릭 = 미리보기 / 더블클릭·Enter = 열기 / Space · 타입어헤드 · 우클릭 메뉴 · 툴팁 · 펼친 폴더 저장/복원(`expanded_dirs`/`expand_dirs`) · `reveal(path)` | 프로젝트 파일·편집기 탭 연동 | **파일 탐색기 폴더 트리에 가장 가까운 기존 구현**(범위 밖이라 머리말·항목만 확인) | `S/project_panel.rs:1-14` · `:57` · `:91` · `:177-179` · `:182-849` · `:1315` · `:1595` |
| UIK-112 | 패널들의 자체 가상 행 목록(북마크·아웃라인·검색) | "머리 + 필터 상자 + 행 목록(가상 스크롤)" 뼈대가 패널마다 복제됨 | 각 패널 도메인 | **범용 가상 리스트 컨트롤 부재**의 증거(같은 뼈대 3~4벌) | `S/bookmarks_panel.rs:1-9` · `S/outline_panel.rs:1-3` · `S/search_panel.rs:1-14` |
| UIK-113 | 실행 상태 카드 `runtoast` | 여러 장 누적 · 진행/속도/경과 · 중지 버튼 · 휠 스크롤 | 쿼리 실행 상태 | 파일 복사/이동 **진행 카드** 설계 참고 | `S/runtoast.rs:1-14`(머리말만 확인) |
| UIK-114 | 호스트 레이아웃(`App::layout`) | 메뉴바 → 툴 도크 → [활동 막대 \| 패널 \| 본문] → 상태바 순의 **수작업 rect 계산** · `Splitter` 3개 | `App` 한 구조체 | 레이아웃 매니저/도크 컨테이너 없음 → dir3도 dir2의 수작업 레이아웃을 그대로 이식하는 것이 자연스럽다 | `S/main.rs:895-1010` · `:1502` · `:1726-1729` |
| UIK-115 | (nexa-ui 쪽) 파일 대화상자 `FilePicker` | 경로 상자(브레드크럼+편집) · 장소 사이드바(`TreeView`) · 목록(`TreeGrid` · 헤더 정렬 · 열 리사이즈 · 폴더 인라인 펼침 · 다중 선택 표시) · 필터 콤보 · 새 폴더 · 덮어쓰기 확인 — **조립층**(`cfg(target_os)` 없음) | 대화상자 전용 복합 컨트롤(3,200줄 단일 파일) · PathBar/FileTree가 독립 컨트롤로 분리되지 않음(`UDoc/TODO.md` F-3 잔여) | dir3 "열기/저장/폴더 선택" 대화상자에 **그대로 사용 가능** · 본체 파일 목록으로는 부족 | `UD/lib.rs:1-11`(머리말) · `:154` · `:1398-1416` · `:1981` · `:2449` · `:2643-2693` |
| UIK-116 | 편집기/결과 탭 툴팁 등 앱 측 보강 | 탭 hover 1초 툴팁 카드(TabBar에 툴팁이 없어 호스트가 그림) | 편집기 정보 | TabBar 툴팁 부재의 증거 → UIK-207 | `S/editors.rs:1-5`(머리말) |

**관찰(설계 결론)**

1. nexa-sql은 **가상 스크롤 목록/그리드를 nexa-ctl에서 받지 못해** 6곳(그리드·탐색기·프로젝트·북마크·아웃라인·검색)에서 각자 구현했다. nexa-ui 로드맵도 이를 인정하고 `nexa-grid`(dir2 `rows.rs`+`columns.rs` 이식)를 P0로 올려 두었으나 **미착수**다(`UDoc/TODO.md:10-11` U-3·G-1~G-6 ☐).
2. 메뉴바·툴바·탭바·트리는 nexa-ctl 것을 쓰되, **상태바·경로바·하단 도크·툴팁·토스트·필터 상자·토글 버튼**은 앱 코드다.
3. 창·표면·이벤트 번역(winit → `InputEvent`)·포커스·팝업 z순서·모달·IME는 전부 앱(`S/app/*`·`S/main.rs`) — nexa-ctl 범위 밖.

---

## 4. 파일 탐색기에 필요한데 없는 컨트롤(추가 후보와 권장 API)

우선순위: **P0** = 없으면 본체가 성립하지 않음 · **P1** = dir2 기능 유지에 필요 · **P2** = 품질/편의.
"원본" = dir2에서 이식할 위젯(`P/14-dir2-gui-widgets.md`의 GUI-ID). 원본 세부는 그 문서와 `P/13-dir2-panel-filelist.md`가 기준이며, 여기 적은 dir2 내용 중 직접 확인하지 않은 것은 "추정"으로 표기한다.

| ID | 후보 | 우선 | 현재 상태(근거) | 원본 | 권장 배치 |
|---|---|:--:|---|---|---|
| UIK-201 | **가상 행 그리드 `VirtualRows` + 컬럼 모델**(파일 목록 본체) | P0 | 없음. `TreeGrid`는 전량 모델·기능 부족(UIK-026 한계 1~5) · `S/grid.rs`는 도메인 결합 | dir2 `widgets/rows.rs`(3,548줄)·`columns.rs`·`typeahead.rs`(`P/14…:51`) | 신규 크레이트 `nexa-ui/crates/nexa-grid`(`UDoc/21-grid-family.md` G-1·G-2·G-5 계획 그대로) |
| UIK-202 | **PathBar**(브레드크럼 + 편집 + 자동완성 팝업) | P0 | 독립 컨트롤 없음. nexa-dlg 내부 구현만(`UDoc/TODO.md` F-3 "잔여 = PathBar 독립 컨트롤 승격 · 자동완성") | dir2 `widgets/pathbar.rs` GUI-090~101 | `nexa-ctl::controls::pathbar` |
| UIK-203 | **StatusBar** | P0 | 없음(nexa-sql은 호스트가 직접 그림 — UIK-110) | dir2 `widgets/chrome.rs` GUI-080 | `nexa-ctl::controls::statusbar` |
| UIK-204 | **InfoDock**(하단 도크: 정보/미리보기/터미널 스트립 + 텍스트 뷰) | P0 | 없음(`UDoc/TODO.md:8` U-2 "☐ dock") | dir2 `widgets/dock.rs` GUI-110~ | `nexa-ctl::controls::dock`(텍스트 뷰는 `TextBox` 읽기 전용 재사용 검토) |
| UIK-205 | **MenuBar 확장** — 단축키 열 · 체크/라디오 · `set_checked` · (선택) Alt 니모닉 · 창 밖 클램프 | P0 | `MenuEntry`는 `ComboItem`만(UIK-017 한계) | dir2 `widgets/menubar.rs` GUI-060·063·066 | `UC/pulldown.rs` 수정(하위 호환 유지) |
| UIK-206 | **Toolbar 확장** — 항목 `checked`(토글 표시) · 폭 넘침 오버플로 | P1 | `ToolItem`에 `checked` 없음(UIK-022) | dir2 `chrome.rs` GUI-074 | `UC/toolbar.rs` 수정 |
| UIK-207 | **TabBar 확장** — 탭 아이콘(폴더/드라이브) · hover 툴팁(전체 경로) · (선택) MouseDown 전환 옵션 | P1 | 아이콘·툴팁 없음(UIK-020) · 전환 시점이 dir2와 다름 | dir2 `widgets/tabbar.rs` GUI-040~051 | `UC/tabbar.rs` 수정 |
| UIK-208 | **Tooltip 관리자**(지연 · 위치 접기 · 팝업 층) | P1 | 그리기 함수만 있음(`U/draw.rs:227-240` `draw_tooltip(_in)`) · Toolbar만 자체 툴팁 | dir2 `A/tip.rs`(Win32 팝업 창 — `P/14…:40`) | `nexa-ctl::controls::tooltip` |
| UIK-209 | **Overlay z 스택**(Base→Popup→Modal · 모달 입력 독점) | P1 | 없음(`UDoc/TODO.md` F-2 ☐) — 지금은 컨트롤마다 `paint_popup`을 호스트가 수동 호출 | — | `nexa-ctl::overlay` |
| UIK-210 | **FolderTree**(지연 로딩 트리: 로딩/오류 노드 · 새로고침 diff · 경로 reveal) | P1 | `TreeView`는 eager(UIK-024) · nexa-dlg 사이드바/`S/project_panel.rs`가 각자 구현 | dir2 폴더 트리(추정 — `P/13…` 확인 필요) | `nexa-ctl::controls::tree` 확장 또는 `nexa-dlg`에서 분리 승격(F-3 잔여 "FileTree 독립 컨트롤") |
| UIK-211 | **InputEvent 확장** — `DoubleClick` · `MiddleDown` · `alt` 수식키 · 드래그 시작/드롭 어휘 | P0 | 없음(UIK-001 한계) — 소비자마다 400ms 판정 복제 | dir2 `nexa-gui/event.rs` GUI-005(어휘 차이는 그 문서 기준) | `U/event.rs`(기존 변형은 유지하고 **추가만**) |
| UIK-212 | **Dialog 프레임 · MessageBox · Prompt · Progress** | P0 | 없음(`UDoc/TODO.md` F-4 잔여 · F-5 ☐) | dir2 대화상자(`P/16-dir2-app-controls-dialogs.md`) | `nexa-dlg` |
| UIK-213 | **Toast**(범용 승격) | P1 | 앱 코드(UIK-107) | — | `nexa-ctl::controls::toast`(nexa-sql `toast.rs` 이관) |
| UIK-214 | **FilterBox + ToggleIconButton**(범용 승격) | P1 | 앱 코드(UIK-105·106) | dir2 빠른 필터(추정) | `nexa-ctl::controls::{filterbox, toggle_button}` |
| UIK-215 | **ProgressBar / Gauge**(드라이브 용량 · 복사 진행) | P1 | 없음(`TimeoutButton` 게이지만) | dir2(추정 — `P/16…`·`P/22…`) | `nexa-ctl::controls::progress` |
| UIK-216 | **인라인 이름 바꾸기 편집기**(행 위 `TextBox` 오버레이 · 확장자 제외 선택) | P0 | 그리드에 통합된 것 없음(`ListEditor`가 같은 문법을 목록 전용으로 구현 — `UC/listedit.rs:14-15`) | dir2 `rows.rs`(추정) | UIK-201의 `LiveEditor` 훅(`UDoc/21…` G-2b) |
| UIK-217 | **보기 모드**(상세/목록/아이콘/썸네일) | P1 | `U/view_mode.rs`의 `ViewMode{Rich,Compact,Plain}`는 **메신저 목록용**(다른 뜻) | dir2 `ViewMode`(`UDoc/21…:§2` 표 "뷰 모드") | UIK-201 안의 별도 열거(`FileView`) — 기존 `ViewMode`와 이름 충돌 주의 |
| UIK-218 | **드래그 고스트/드롭 표시**(행 드래그 · 패널 간 · 외부 DnD 수신 강조) | P1 | TabBar/ToolDock 내부 고스트만 | dir2 `rows.rs`+Win32 OLE DnD(`P/19…`) | UIK-201 + 호스트 OS 분기 |
| UIK-219 | **Checkbox 3상태**(속성 대화상자의 혼합 상태) | P2 | 2상태뿐(UIK-005) | dir2(추정) | `UC/checkbox.rs` 수정 |
| UIK-220 | **Combo 드롭다운 스크롤 + 타입어헤드 + `set_items`** | P2 | 없음(UIK-008 한계) — 드라이브/인코딩처럼 긴 목록에서 문제 | — | `UC/combo.rs` 수정 |
| UIK-221 | **ContextMenu 니모닉 글자 키** | P2 | `Char` = 닫기(UIK-010) | dir2는 네이티브 팝업(`P/14…` GUI-048) — 가속 글자 동작이 OS 제공이었음 | `UC/ctxmenu.rs` 수정 |
| UIK-222 | **패널 포커스/Tab 순서 관리자** | P2 | 없음 — 호스트가 `set_focused`를 직접 돌린다 | dir2 `A/win.rs`(호스트 코드) | 호스트(dir3 앱) — nexa-ui에 넣지 않는 것을 권장 |

### 4-1. 권장 API(초안 — 구현 단계에서 dir2 원본 시그니처와 대조해 확정)

**UIK-201 `nexa-grid`** — nexa-ui 계획(`UDoc/21-grid-family.md:26-36`·G-1~G-5)을 그대로 따른다.

```rust
pub struct Column { pub key: u32, pub title: String, pub width: i32, pub min_width: i32,
                    pub align: HAlign, pub sortable: bool, pub resizable: bool, pub visible: bool }
pub trait RowSource {
    fn len(&self) -> usize;
    fn write_cell(&self, row: usize, key: u32, out: &mut String);   // 할당 0 페인트
    fn icon(&self, row: usize) -> Option<Rc<IconImage>> { None }
    fn depth(&self, row: usize) -> u16 { 0 }                        // 인라인 폴더 펼침
    fn is_dir(&self, row: usize) -> bool { false }
    fn is_ghosted(&self, row: usize) -> bool { false }              // 잘라내기 대기
}
pub enum GridAction { Activate(usize), Context{row: Option<usize>, x: i32, y: i32}, Sort{key: u32, add: bool},
                      ColumnResized{key: u32, width: i32}, ColumnMoved{from: usize, to: usize},
                      SelectionChanged, RenameCommit{row: usize, name: String}, Toggle(usize),
                      DragStart{rows: Vec<usize>}, ReachedEnd }
pub struct VirtualRows<S: RowSource> { /* ControlBase + ScrollBars + TypeAhead + 선택 집합 */ }
impl<S: RowSource> VirtualRows<S> {
    pub fn new(source: S, columns: Vec<Column>) -> Self;
    pub fn source(&self) -> &S;  pub fn source_mut(&mut self) -> &mut S;  pub fn rows_changed(&mut self);
    pub fn set_columns(&mut self, c: Vec<Column>);  pub fn columns(&self) -> &[Column];
    pub fn set_sort_badges(&mut self, keys: &[(u32, bool)]);
    pub fn set_view(&mut self, v: FileView);                 // Details | List | Icons | Thumbs
    pub fn set_row_height(&mut self, logical_px: i32);
    pub fn selection(&self) -> impl Iterator<Item = usize>;  pub fn caret(&self) -> Option<usize>;
    pub fn select_only(&mut self, row: usize);  pub fn select_all(&mut self);  pub fn clear_selection(&mut self);
    pub fn reveal(&mut self, row: usize);  pub fn begin_rename(&mut self, row: usize);
    pub fn editing_input(&mut self) -> Option<&mut TextBox>; // Combo/ListEditor와 같은 호스트 계약
    pub fn take_actions(&mut self) -> Vec<GridAction>;
    pub fn tick(&mut self, now_ms: u64) -> bool;
}
// + Widget/Control 구현. 선택 규칙 = 클릭/Shift 범위/primary 토글/Space/Ctrl+A/러버밴드(dir2 규약 유지).
```

**UIK-202 PathBar**

```rust
pub enum PathBarAction { Navigate(String), EditBegan, EditCancelled, TextChanged(String), Context{x: i32, y: i32} }
pub struct PathBar { /* ControlBase + TextBox(편집) + 제안 팝업 */ }
impl PathBar {
    pub fn new() -> Self;
    pub fn set_path(&mut self, display: &str, sep: char);      // OS별 구분자는 호스트가 준다(\ 또는 /)
    pub fn set_segments(&mut self, segs: Vec<(String /*label*/, String /*full*/)>); // 가상 루트·클라우드용
    pub fn begin_edit(&mut self, select_all: bool);  pub fn is_editing(&self) -> bool;
    pub fn editing_input(&mut self) -> Option<&mut TextBox>;
    pub fn set_suggestions(&mut self, items: Vec<String>);     // 빈 목록 = 닫기(dir2 GUI-099)
    pub fn popup_open(&self) -> bool;  pub fn paint_popup(&self, ctx: &mut dyn DrawCtx, theme: &Theme);
    pub fn take_action(&mut self) -> Option<PathBarAction>;
}
```
경로 분해·환경변수 확장·`shell:` 스킴은 컨트롤에 넣지 않는다(이미 `nexa_fs::path`가 `~`·환경변수·`$env:`·`shell:`·상대 경로를 처리 — `UDoc/TODO.md` F-1). OS 분기점은 구분자와 루트 표기뿐이다.

**UIK-203 StatusBar**

```rust
pub struct StatusSeg { pub id: String, pub text: String, pub right: bool, pub clickable: bool,
                       pub tone: ToolTone, pub tip: String, pub icon: Option<MenuIcon> }
impl StatusBar {
    pub fn new() -> Self;  pub fn set_segments(&mut self, segs: Vec<StatusSeg>);
    pub fn set_text(&mut self, id: &str, text: &str, inv: &mut Invalidations);
    pub fn seg_rect(&self, id: &str) -> Option<Rect>;  pub fn take_clicked(&mut self) -> Option<String>;
    pub fn preferred_height(&self) -> i32;
}
```
dir2 원본은 좌/우 텍스트 2개(`P/14…` GUI-080)이므로 `set_left(&str)`/`set_right(&str)` 편의 메서드를 함께 두면 이식이 1:1이 된다.

**UIK-204 InfoDock** — dir2 `dock.rs`의 종류 스트립(정보·미리보기·터미널)·텍스트/이미지 내용·스크롤을 그대로 옮기되, 터미널 본체는 별도 위젯으로 끼워 넣는 슬롯만 둔다.

```rust
pub enum DockKind { Info, Preview, Terminal }
pub enum DockAction { Switch(DockKind), TerminalCdHere, Context{x: i32, y: i32}, Copy(String) }
impl InfoDock {
    pub fn set_kinds(&mut self, kinds: &[(DockKind, String /*label*/)]);  pub fn set_active(&mut self, k: DockKind);
    pub fn set_content(&mut self, key: &str, lines: Vec<String>);          // 키가 같으면 스크롤·선택 유지(GUI-113)
    pub fn set_image(&mut self, img: Option<Rc<IconImage>>);
    pub fn content_rect(&self) -> Rect;                                    // 터미널/플러그인 뷰를 호스트가 여기에 배치
    pub fn take_action(&mut self) -> Option<DockAction>;  pub fn tick(&mut self, now_ms: u64) -> bool;
}
```

**UIK-205 MenuBar 확장**(하위 호환: 기존 `MenuEntry::Item(ComboItem)` 유지)

```rust
pub struct MenuItemSpec { pub item: ComboItem, pub shortcut: String, pub checked: Option<bool>,
                          pub radio: bool, pub enabled: bool, pub emph: bool }
pub enum MenuEntry { /* 기존 5종 */ Spec(MenuItemSpec), SubSpec(MenuItemSpec, Vec<MenuEntry>) }
impl MenuBar {
    pub fn set_checked(&mut self, id: &str, on: bool, inv: &mut Invalidations);   // dir2 GUI-066
    pub fn set_enabled(&mut self, id: &str, on: bool, inv: &mut Invalidations);
    pub fn set_shortcut(&mut self, id: &str, text: &str);                          // 키맵 변경 반영
    pub fn open_menu_index(&mut self, i: usize, inv: &mut Invalidations);          // Alt/F10 진입(호스트 키)
}
```
dir2의 명령 id는 `u32`(GUI-060)이고 nexa-ctl은 `String` → dir3 명령 카탈로그(`P/30-catalog-commands-shortcuts.md`)의 문자열 id로 통일하는 것을 권장.

**UIK-206 Toolbar 확장**: `ToolItem.checked: bool` + `Toolbar::set_item_checked(id, on, inv)` + `ToolItem::checked(self, on) -> Self`(그림 = accent 블렌드 배경 — dir2 GUI-074). 오버플로는 `Toolbar::overflow_ids() -> Vec<&str>` + 끝의 `»` 항목(클릭 = 호스트가 `ContextMenu`로 표시).

**UIK-207 TabBar 확장**: `set_icons(Vec<Option<Rc<IconImage>>>, inv)` · `set_tips(Vec<String>)` + `hover_tip() -> Option<(usize, &str, Rect)>`(호스트/Tooltip 관리자가 그림) · `set_switch_on_press(bool)`(dir2 호환 옵션 · 기본 false).

**UIK-208 Tooltip**: `Tooltip::new()` · `set_delay_ms(u64)` · `hover(Option<(Rect /*anchor*/, String)>, now_ms)` · `tick(now_ms) -> bool` · `paint(ctx, theme, surface: Rect)`(내부에서 `draw_tooltip_in` 사용).

**UIK-211 InputEvent 추가 변형**(기존 변형·필드는 그대로 — nexa-sql 호환):

```rust
InputEvent::DoubleClick { x: i32, y: i32, shift: bool, primary: bool }   // 호스트가 OS 더블클릭 시간으로 합성
InputEvent::MiddleDown  { x: i32, y: i32 }
InputEvent::KeyEx { key: KeyEx, shift: bool, primary: bool, alt: bool }  // F1~F12 · Tab · Backspace · Insert · Apps
```
`match`에 `_ => {}` 팔이 없는 소비자가 있으면 컴파일이 깨지므로 nexa-sql 전체 빌드 확인이 필요하다(위험).

### 4-2. 구현 순서 제안(의존 기준)

1. UIK-211(이벤트 어휘) → UIK-205(MenuBar) · UIK-206(Toolbar) · UIK-207(TabBar) · UIK-203(StatusBar): 기존 컨트롤 보강 — 창 골격이 dir2와 같은 모양으로 선다.
2. UIK-201(nexa-grid: dir2 `rows`/`columns`/`typeahead` 이식 + `DrawCtx` 어댑터) → UIK-216(이름 바꾸기) · UIK-217(보기 모드) · UIK-218(드래그).
3. UIK-202(PathBar) · UIK-210(FolderTree) — nexa-dlg 내부 구현을 분리 승격하면 `FilePicker`도 같이 얇아진다.
4. UIK-204(InfoDock) · UIK-208(Tooltip) · UIK-209(Overlay) · UIK-212(대화상자/진행) · UIK-213~215.

### 4-3. 주의(교차 검증 때 확인할 것)

- nexa-ui는 nexa-sql·nexa-beep 등 **여러 앱이 공유**한다. 기존 공개 시그니처를 바꾸면 nexa-sql 빌드가 깨진다 → **추가만** 하고, 바꿔야 하면 nexa-sql 쪽도 같은 커밋 묶음에서 고친다(`UDoc/TODO.md` U-6 "공개 API 변경 시 영향 표기").
- `set_ctl_labels`는 1회 주입(`UC/mod.rs:135-137`) — dir3 부팅 순서에서 i18n 초기화 뒤에 부른다.
- 팝업을 가진 컨트롤(Combo·IconDropdown·PositionDropdown·MenuBar·ContextMenu·PathBar 제안)은 **모달 캡처와 최상위 재도색을 호스트가** 해야 한다(Overlay 도입 전까지). nexa-sql의 규칙 = "바깥 클릭은 닫고 그대로 아래로 흘린다"(`UC/ctxmenu.rs:501-515`).
- 클릭 확정 시점이 컨트롤마다 다르다: Button·Toolbar·TabBar·ContextMenu = **MouseUp**, MenuBar 항목·Checkbox·Switch·Radio·Combo 항목·TreeView 행 = **MouseDown**. dir2는 대부분 MouseDown 즉시(`P/14…` GUI-041·064·072) → 회귀 테스트의 입력 합성은 Down+Up 쌍으로 작성한다.
- `TreeView`/`TreeGrid`는 부분적으로 보이는 행을 그리지 않는다(`UC/tree.rs:603-605`·`:889-892`) — 픽셀 스크롤 시 위아래 가장자리에 빈 띠가 생긴다(파일 목록에 쓰면 눈에 띈다).
