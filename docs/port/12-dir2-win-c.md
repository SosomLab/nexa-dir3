# 12. nexa-dir2 인벤토리 — `win.rs` C 구간(설정 적용 · 편집/클립보드 디스패치 · `wndproc` 전체 · 테스트)

> 표기 규약: 이 문서의 `win.rs:N` 은 전부 `nexa-dir2/crates/nexa-app/src/win.rs:N` 이다. 그 밖의 근거는 `저장소/경로:줄` 전체 표기.
> ID 접두사 `WINC-NNN`. 확인하지 못한 것은 "추정"이라고 적었다.

## 0. 범위 — 읽은 파일과 줄 수

| 파일 | 읽은 구간 | 비고 |
|---|---|---|
| `nexa-dir2/crates/nexa-app/src/win.rs` | **6470~9952 (끝까지 전부)** | 담당 구간. 지시서는 "전체 9703줄"이라 했으나 **실제 파일은 9,951줄**(`wc -l`) — 테스트 모듈이 9716~9951. 끝까지 읽었다. |
| 같은 파일 | 44~207 | 상수(타이머 id·주기, `WM_APP_*`, 스플리터 상수) — 담당 구간이 참조 |
| 같은 파일 | 1236~1342 | `State::hit_zone`/`HitZone`/`HitGeom`/`hit_zone_impl` — 테스트 대상 |
| 같은 파일 | 2004~2034, 2205~2244, 3976~4016, 5086~5108, 5980~6269 | `should_trim`·스테이징 분리·리네임 만료 판정·`finish_input`·`reset_mouse_transients`~`show_bar_popup` 머리 — 담당 구간이 호출/테스트 |
| 같은 파일 | 전체 `fn` 목록(Grep) · 전체 `const` 목록(Grep) | 구조 파악 |
| `nexa-ui/crates/nexa-ctl/src/**` | `pub struct/enum/trait` 전수 Grep + `event.rs` 전문 + `editmenu.rs` 1~175 + `splitter.rs` 1~135 + `scroll.rs` 48~117 + `edit.rs` 10~39 + `tabbar.rs`·`ctxmenu.rs`·`pulldown.rs`·`toolbar.rs`·`tree.rs`·`widget.rs` 의 `pub fn` Grep | §3 매핑 근거 |
| `nexa-ui/crates/nexa-fs/src/**`, `nexa-sys/src/lib.rs`, `nexa-dlg/src/lib.rs` | `pub` 항목 Grep | §3·§4 근거 |
| `nexa-sql/crates/nexa-sql/src/clipboard.rs` | 전문(300줄) | §4 클립보드 대응 근거 |
| `nexa-sql/crates/nexa-sql/src/app/event_loop.rs` | 키워드 Grep(이벤트 종류) | §4 이벤트 대응 근거 |

담당 구간의 구성(줄 범위):

| 블록 | 줄 |
|---|---|
| `apply_prefs`(설정 창 값 적용 — 6475에서 시작) | 6475~6762 |
| 설정 문자열 변환·모드 판정 헬퍼 | 6764~6802 |
| 세션/설정 스냅샷·컬럼 레이아웃·컬럼 폭 동기 | 6804~7018 |
| 전송 잠금·poison 내성 lock·종결 통지 재시도 | 7020~7056 |
| 파일 실행(`guarded_shell_open`·`shell_open`·포그라운드 양도) | 7058~7110 |
| 탭 우클릭 메뉴 | 7112~7195 |
| 경로바 자동완성 갱신 | 7197~7206 |
| 편집 동작 디스패치(`ClipAct`·`do_clip`)·터미널 복사/붙여넣기/전체 선택 | 7208~7456 |
| 텍스트 편집 컨텍스트 메뉴(`EditMenuTarget`·`show_edit_popup`) | 7458~7620 |
| `paste_line`·`edit_key_of`·`vk_to_key` | 7622~7662 |
| **`wndproc`** | 7664~9714 |
| `#[cfg(test)] mod tests`(7건) | 9716~9951 |

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지(타 OS 대체·비활성).

### 1-A. 설정 값 적용 `apply_prefs`(설정 창 실시간 통지·닫기 공용, 멱등)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINC-001 | 설정 창에서 테마·언어·터미널 글꼴 변경 즉시 반영 | 테마: 값이 다를 때만 `run_command(CMD_THEME_LIGHT/DARK/SYSTEM)`(미지 값=system). 언어: `"system"`→`CMD_LANG_SYSTEM`, 그 외 `st.langs`에서 코드 위치 찾아 `CMD_LANG_BASE+i`. 터미널 글꼴/크기 변경 시 `st.dw=None`(백엔드 재생성). `run_command` 뒤마다 `state_of` 재획득(재진입 규약) | `win.rs:6475`~6500 | `state_of`(GWLP_USERDATA) | N(그리기 백엔드 재생성부는 A) | 없음 |
| WINC-002 | 기본/상태바/파일 목록 글꼴·크기 변경 | 6개 값 중 하나라도 다르면 저장 후 `st.dw=None`. 크기는 `clamp(8,32)` | `win.rs:6504`~6518 | DirectWrite 백엔드 재생성 | A | 없음 |
| WINC-003 | 우클릭 메뉴 글꼴 설정 | **저장만**(네이티브 HMENU는 OS 글꼴 규약이라 미적용 — "자체 그리기 메뉴 전환 시 적용" 주석). 크기 `clamp(8,32)` | `win.rs:6519`~6522 | HMENU | A(dir3은 자체 그리기 메뉴 → **실제 적용 필요**) | 없음 |
| WINC-004 | 폴더 굵게·헤더 굵게/이탤릭 | 변경 시 양 패널 `set_font_decor` — 전 탭 즉시 | `win.rs:6524`~6535 | — | A | 없음 |
| WINC-005 | 대화상자 글꼴 | `st.dlg_font = DlgFont{family,size_pt}` 무조건 대입 | `win.rs:6536`~6539 | — | A | 없음 |
| WINC-006 | 숨김 파일·닷파일 표시 체크 | **적용 범위(view_scope)와 무관하게 전체 일괄**: 메뉴바·툴바 체크 동기 → 양 패널 `set_view_filters(true,…)`(전 탭 기입) → 활성 탭 `reopen_filtered`. 비활성 탭은 전환 시 수렴 | `win.rs:6543`~6563 | — | N | 없음 |
| WINC-007 | 하단 도크 표시 토글(설정 창 경유) | 양 패널 `set_dock_visible` + 메뉴 체크 + `layout` + `update_dock_info` | `win.rs:6565`~6573 | — | N | 없음 |
| WINC-008 | 컬럼 auto-fit 최대 폭 | `clamp(50,2000)`(px @96dpi) | `win.rs:6575`~6577 | — | N | 없음 |
| WINC-009 | 컨텍스트 메뉴 순서/표시 | 문자열 저장만 — 다음 메뉴 표시부터 반영 | `win.rs:6579`~6581 | — | N | 없음 |
| WINC-010 | 컬럼 레이아웃(순서·표시) | 비어 있지 않고 **포커스 패널** 현재 레이아웃과 다르면 적용. `col_width_sync` on이면 `columns_snapshot`을 반대 패널에 `apply_columns` | `win.rs:6583`~6593 | — | N | 없음 |
| WINC-011 | 도구모음 순서 편집 즉시 반영 | `toolbar_order` 변경 시 `build_toolbar(11개 인자)`로 버튼 재구성 | `win.rs:6595`~6615 | — | A | 없음 |
| WINC-012 | 터미널 줄 바꿈·고정 열 수 | `term_cols clamp(80,1000)`; 전창 무효화 → 다음 페인트에서 cols 재계산·PTY resize | `win.rs:6616`~6620 | `InvalidateRect` | N | 없음 |
| WINC-013 | 터미널 테마(공통/다크/라이트)·복사 형식 | 3개 값 중 변경 시 저장 + 전창 무효화(셀이 기호 색이라 재도장만으로 스크롤백까지 새 팔레트). `term_copy_format`은 무조건 대입 | `win.rs:6622`~6631 | — | N | 없음 |
| WINC-014 | 폴더 우선 정렬 | 설정 창 = 전체 일괄. 툴바 체크 + 양 패널 `set_folders_first` | `win.rs:6633`~6641 | — | N | 없음 |
| WINC-015 | 보기 옵션 적용 범위(view_scope) | 변경 시 툴바 **재구성**(툴팁에 범위 문구가 실려 있음) | `win.rs:6643`~6663 | — | A | 없음 |
| WINC-016 | 빈 폴더 글리프 억제 | 양 패널 `set_hide_empty_glyph` — 전 탭 즉시 | `win.rs:6665`~6671 | — | N | 없음 |
| WINC-017 | 대소문자 구분 정렬 | 양 패널 `set_sort_case` — 전 탭 즉시 재정렬 | `win.rs:6673`~6679 | — | N | 없음 |
| WINC-018 | 탭 더블클릭 동작 | 문자열 저장(`"pin"`/`"lock"`/그 외=닫기 — 해석은 WINC-108) | `win.rs:6681`~6683 | — | N | 없음 |
| WINC-019 | 플러그인 사용 여부 | `plugins_disabled` 변경 시 `update_dock_info`로 도크 미리보기 즉시 갱신 | `win.rs:6685`~6690 | — | N | 없음 |
| WINC-020 | 전송 완료 창 닫기 시간 | `clamp(0,10_000)` ms. 0=창 미표시. 다음 전송부터 | `win.rs:6692`~6694 | — | N | 없음 |
| WINC-021 | DnD 호버 대기 시간 | `clamp(200,10_000)` ms. 다음 드래그부터 | `win.rs:6696`~6698 | — | N | 없음 |
| WINC-022 | 고속 스크롤(핫스왑) | `FastScroll{enabled, step clamp(1,50), max clamp(1,32), window_ms clamp(20,2000), hud, hud_pos clamp(0,8), hud_hold_ms clamp(0,10_000), hud_fade_ms clamp(0,10_000)}`. 값 또는 `fast_scroll_grid_extra`가 다르면 전역 `set_fast_scroll` + `set_fast_scroll_grid(grid_extra면 grid_extra_of(cfg))` | `win.rs:6700`~6720 | — | A | 없음 |
| WINC-023 | 타입어헤드 옵션 | scope/reset_ms `clamp(200,10_000)`/pos `clamp(0,8)`/special/space/backspace 6개 중 변경 시 양 패널 `set_typeahead_opts` | `win.rs:6722`~6749 | — | A | 없음 |
| WINC-024 | Alt+↑ 자동 선택 배치 | `"top"`/`"bottom"`/그 외=center → 양 패널 `set_nav_up_align` | `win.rs:6751`~6756 | — | N | 없음 |
| WINC-025 | 설정 즉시 영속 | 적용 끝에 **항상** `config::save(data_dir, SETTINGS_FILE, settings.serialize())` + 전창 무효화 + `update_title` | `win.rs:6758`~6761 | 파일 쓰기 | N(파일 형식은 nexa-sql 설정 구조로 교체 — §5) | 없음 |

### 1-B. 헬퍼·스냅샷·편집 디스패치

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINC-026 | 설정 문자열 → 열거 변환 | `scope_of`: `"global"`→GlobalFirst, `"level"`→CurrentLevel, 그 외 VisibleStream. `mode_of`: `"flat"`/`"tiles"`/그 외 Tree. `align_of`: `"top"`/`"bottom"`/그 외 Center | `win.rs:6765`, 6785, 6795 | — | N | 없음 |
| WINC-027 | 싱글 패널·싱글 정보 판정 | `single_panel` = `panel_mode=="single"`. `single_info` = 싱글 패널이면 **무조건** 싱글(선호값 `info_mode`는 보존 — 듀얼 복귀 시 원복) | `win.rs:6774`, 6780 | — | N | 없음 |
| WINC-028 | 세션 스냅샷 | `Session{active_panel, panels[2]: PanelSession{tabs, active, expanded, locked, pinned, modes, views, col_widths, col_layout}}` — 종료 저장·디바운스 자동 저장 공용 단일 원천 | `win.rs:6805`~6835 | — | N | 없음 |
| WINC-029 | 컬럼 경계 더블클릭 auto-fit | **가시 행 + 헤더** 텍스트(`rows().autofit_texts(col)` = (text, extra px)) 폭을 List 폰트로 실측한 최대값. 상한 = `col_autofit_max*dpi/96`. 반영 후 `sync_col_widths`. `st.dw`가 없으면 no-op | `win.rs:6846`~6867 | DirectWrite 텍스트 측정(`DwCtx.text_width`) | A | 없음 |
| WINC-030 | 컬럼 레이아웃 직렬화/적용 | key↔id: name/ext/size/modified/kind ↔ `COL_NAME/EXT/SIZE/MODIFIED/KIND`(미지 id→"name"). 문자열 형식 `cols[name:1,…]` — 표시 컬럼=표시 순, 숨김 컬럼=정의 순으로 말미 `:0`. 적용은 `config::parse_order_with(COLUMN_BLOCKS,…)` 첫 블록 → `Panel::apply_col_layout`(폭 보존) | `win.rs:6870`~6920 | — | N | 없음(추정: config 쪽 테스트는 다른 구간 담당) |
| WINC-031 | 컬럼 폭/순서 동기 | 각 패널: `take_col_reordered()` → 같은 패널 전 탭 상속(`apply_columns`) + sync on이면 반대 패널. `take_col_resized()` → `apply_col_widths` 동일 규약. MouseMove·MouseUp 뒤 폴링 호출 | `win.rs:6922`~6940 | — | N | 없음 |
| WINC-032 | 설정 스냅샷/영속 | `current_settings` = `Settings` 필드 61개 전수(§5 표). `persist_settings` = 즉시 저장(메뉴 라디오·토글도 변경 즉시 — 비정상 종료 유실 방지) | `win.rs:6942`~7018 | 파일 쓰기 | N | 없음 |
| WINC-033 | 전송 중 대상 잠금 | 진행 중 전송의 `shared.in_flight`(현재 쓰는 대상)와 같거나 그 하위 경로면 `true` → 실행·드래그 차단 | `win.rs:7021`~7029 | — | N | 없음 |
| WINC-034 | poison 내성 lock | `m.lock().unwrap_or_else(PoisonError::into_inner)` — 워커↔UI 공유 Mutex 전용 | `win.rs:7037`~7039 | — | N | 없음 |
| WINC-035 | 워커 종결 통지 유한 재시도 | `PostMessageW` 실패 시 100ms 간격 최대 50회(5초). 진행률 통지는 단발, **종결 통지만** 재시도 | `win.rs:7048`~7056 | `PostMessageW` | P(이벤트 루프 wake) | 없음 |
| WINC-036 | 파일 실행 게이트 | ① 클라우드 경로(`nexa_vfs::cloud_parts`)면 임시 폴더로 다운로드 후 열기(`start_cloud_download(...,None)`) ② 전송 중 대상이면 경고음 + 타이틀 `ops.itemBusy` 후 중단 ③ 그 외 `shell_open` | `win.rs:7059`~7078 | `MessageBeep(MB_ICONWARNING)` | P | 없음 |
| WINC-037 | 연 프로그램을 앞으로(포그라운드 양도) | 실행 위임 **직전** `AllowSetForegroundWindow(ASFW_ANY)`. 반환값 무시가 의도 | `win.rs:7091`~7094 | `AllowSetForegroundWindow` | W | 없음 |
| WINC-038 | 기본 연결 프로그램으로 열기 | `ShellExecuteW(hwnd,"open",path,…,SW_SHOWNORMAL)` | `win.rs:7097`~7110 | `ShellExecuteW` | P | 없음 |
| WINC-039 | 탭 우클릭 메뉴 | 항목 순서: 잠금/잠금 해제 → 고정/고정 해제 → 복제 → 새 탭 → 닫기(잠금 탭이거나 탭 1개면 비활성). 커서 위치에 표시. State 참조 없이 표시 후 재획득(재진입 규약). 실행 후 `update_title`·`update_status` | `win.rs:7115`~7195 | `CreatePopupMenu`·`AppendMenuW`·`TrackPopupMenuEx(TPM_RETURNCMD)`·`GetCursorPos` | A | 없음 |
| WINC-040 | 경로바 자동완성 갱신 | 편집 텍스트 → `pathinput::expand_env` → `suggest_folders(expanded, fs_dirs, 20)` → `pathbar.set_suggestions` | `win.rs:7199`~7206 | 환경변수 문법(%VAR%) | P(환경변수 문법) | 없음(추정: pathinput 자체 테스트는 타 구간) |
| WINC-041 | 편집 동작 열거·단축키 매핑 | `ClipAct{Undo,Cut,Copy,Paste,Delete,SelectAll}`. `clip_act_of`: C/X/V/Z → Copy/Cut/Paste/Undo(Ctrl+A는 `edit_key_of`가 처리) | `win.rs:7210`~7229 | VK 코드 | N | 없음 |
| WINC-042 | 진행 중 텍스트 편집 정리 | 활성 패널 경로바 편집 취소 + **양 패널** 인라인 리네임 취소. 터미널이 키 포커스/편집 메뉴 대상으로 확정되는 순간 호출 | `win.rs:7239`~7244 | — | N | 없음 |
| WINC-043 | 편집 동작 ① 경로바 편집 필드 | Undo/Cut(클립보드 쓰기)/Copy/Paste(`paste_line`)/Delete/SelectAll. 텍스트가 바뀐 경우에만 자동완성 갱신. 항상 `true` 반환 | `win.rs:7250`~7286 | 클립보드 텍스트 | P(클립보드) | 없음 |
| WINC-044 | 편집 동작 ② 인라인 이름변경 필드 | 동일 6동작(`rename_undo/cut/selected_text/paste/delete/key(SelectAll)`). 항상 `true` | `win.rs:7288`~7316 | 클립보드 텍스트 | P(클립보드) | 없음 |
| WINC-045 | 편집 동작 ③ 도크 터미널 | 키 포커스 터미널이 살아 있을 때: Copy·Cut=선택 복사, Paste=붙여넣기, SelectAll=전체 선택, Undo·Delete=무동작(false). 캐럿 표시 위상 리셋 + 도크 무효화 | `win.rs:7318`~7330 | — | N | 없음 |
| WINC-046 | 편집 동작 ④ 도크 Info/Preview 텍스트 복사 | Copy일 때만: 표시 중 도크 중 선택 텍스트가 있는 첫 패널의 텍스트를 **rich(모노 RTF 동시)** 로 복사 | `win.rs:7332`~7342 | 클립보드 텍스트+RTF | P(클립보드 다중 형식) | 없음 |
| WINC-047 | 편집 동작 ⑤ 파일 목록 | Undo=`do_undo_redo(false)`. SelectAll=`InputEvent::SelectAll`. Copy/Cut=선택 경로를 파일 목록으로 게시(선택 없으면 클립보드 유지·false). Paste=파일 목록 있으면 전송 시작(Move면 **클립보드 비움** — 잘라내기 1회성), 대상=`paste_dest`(폴더 1개 선택=그 안, 그 외 현재 폴더); 없고 가상 파일 있으면 `paste_virtual`. Delete=false | `win.rs:7344`~7384 | `CF_HDROP`·Preferred DropEffect·가상 파일(FileGroupDescriptor) | P(파일 클립보드) / 가상 파일은 W | 없음 |
| WINC-048 | 터미널 생존 판정 | `dock_shown && active_kind()==2 && terms[ti] 존재 && !exited` | `win.rs:7388`~7392 | — | N | 없음 |
| WINC-049 | 터미널 선택 복사 | 평문 + (설정 `term_copy_format`: `html`/`rtf`/`both`) HTML(CF_HTML 래핑)·RTF. 팔레트 = 현재 테마 해석, 글꼴 = `term_font` 체인 1순위(없으면 `Consolas`), 크기 = `term_font_size`. 복사 후 **선택 해제**(WT 규약). 선택 없으면 false | `win.rs:7396`~7429 | 클립보드 CF_HTML·RTF | P | 없음 |
| WINC-050 | 터미널 붙여넣기 | 클립보드 텍스트의 `\r\n`→`\r`, `\n`→`\r` 변환 후 PTY write. 스크롤백 보기 해제(`view_off=0`) | `win.rs:7433`~7443 | 클립보드·PTY | P | 없음 |
| WINC-051 | 터미널 전체 선택 | 스크롤백 첫 줄(0,0) ~ 마지막 줄(`line_count-1`, `cols-1`). 줄 0개면 false | `win.rs:7446`~7456 | — | N | 없음 |
| WINC-052 | 텍스트 편집 메뉴 대상 판정 | 우선순위: 활성 패널 경로바 편집 필드 hit → 활성 패널 리네임 필드 hit → 좌표 패널의 도크(표시 중·h>0·content hit): 종류 2(터미널)이면 살아 있을 때만 `Term(p)`, 아니면 `text_selectable()`일 때 `Dock(p)` | `win.rs:7460`~7490 | — | N | 없음 |
| WINC-053 | 텍스트 편집 컨텍스트 메뉴 | PathBar/Rename: 실행 취소(can_undo) ― 잘라내기(has_sel)·복사(has_sel)·붙여넣기(클립보드 텍스트 있음)·삭제(has_sel) ― 전체 선택(!empty). Dock: 복사(선택 있음) ― 전체 선택(text_selectable). Term: 복사(선택 있음)·붙여넣기 ― 전체 선택(항상). i18n 키 `menu.edit.undo/cut/copy/paste/delete/selectAll`. 결과 반영: Dock=대상 패널 직접, Term=편집 정리+포커스 확정 후 `do_clip`, PathBar/Rename=`do_clip` | `win.rs:7496`~7620 | `TrackPopupMenuEx` 등 | A | 없음 |
| WINC-054 | 붙여넣기 텍스트 정제(한 줄) | 클립보드 텍스트 첫 줄만, 제어 문자 제거. 빈 결과면 None | `win.rs:7623`~7633 | 클립보드 | N(클립보드 읽기만 P) | 없음 |
| WINC-055 | 편집 키 매핑 | ←/→/Home/End/Delete → `EditKey::Left/Right/Home/End/DeleteForward`, Ctrl+A → `SelectAll` | `win.rs:7636`~7647 | VK 코드 | N | 없음 |
| WINC-056 | 목록 탐색 키 매핑 | ↑↓/PgUp/PgDn/Home/End/→/←/Space → `Key::*` | `win.rs:7649`~7662 | VK 코드 | N | 없음 |

### 1-C. 메인 창 메시지 처리 `wndproc`

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | Win32/OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINC-057 | 입력 활동 기록(유휴 트림 해제) | 메시지 0x0100~0x0109(키보드)·0x0200~0x020E(마우스) 전 범위에서 `note_activity`(마지막 활동 시각 갱신, 트림 상태면 해제 + 자니터 재가동) | `win.rs:7666`~7670 | 메시지 번호 범위 | N | 없음 |
| WINC-058 | 창 생성 초기화 | `WM_NCCREATE`: State 포인터 설치 → `install_cloud_lister` → 메뉴 스레드 spawn(실패 시 None=동기 메뉴) → 도크 Info 상세 워커 spawn(결과는 `post_final_notify(WM_APP_INFO_DETAILS)`) → DPI 취득 → **양 패널 전 탭** `reopen_cloud_tabs`(세션 복원 탭이 콜백 등록 전에 만들어져 비어 있음) → 메트릭·컬럼 설정 → 세션 컬럼 복원(**레이아웃 먼저, 폭 다음**; `pending_cols`/`pending_colw` take) → `col_width_sync`면 좌 기준으로 우 맞춤 → 포커스 시각 동기 → 메뉴바/툴바/런처/상태바 메트릭 → 타이틀바 테마 → 타이틀·상태 갱신 → 자니터 타이머(10s) → `poll_fs_probe` 기준선 + FSPOLL 타이머(3s) | `win.rs:7672`~7743 | `SetWindowLongPtrW`·`GetDpiForWindow`·`GetCurrentThreadId`·`SetTimer` | P | 없음 |
| WINC-059 | 페인트 | `WM_PAINT`=`paint()`; `WM_ERASEBKGND`=1(배경 지우기 억제 — 깜빡임 방지) | `win.rs:7744`~7750 | GDI 페인트 사이클 | A | 없음 |
| WINC-060 | 앱 활성/비활성 시 갱신 | 활성화: `refresh_on_return`(밖에서 저장·동기화된 변경 반영). 비활성: 폴링을 **끄지 않고 감속**(FSPOLL 30s) | `win.rs:7755`~7768 | `WM_ACTIVATEAPP` | P | 없음 |
| WINC-061 | 드라이브 도착/제거 반영 | `DBT_DEVICEARRIVAL(0x8000)`/`DBT_DEVICEREMOVECOMPLETE(0x8004)` 시 **보이는(w>0)** 패널 중 가상 루트(내 PC)를 보는 패널만 `arm_watch_debounce` | `win.rs:7772`~7787 | `WM_DEVICECHANGE` | P | 없음 |
| WINC-062 | 리사이즈·최소화·복원 | 최소화: 즉시 `trim_resident` + `trimmed=true` + 자니터·FSPOLL 타이머 정지. 그 외: `from_minimized = st.trimmed`를 **`note_activity` 전에** 읽고 → `layout` → 최소화에서 복원이면 `refresh_on_return` | `win.rs:7788`~7813 | `WM_SIZE`·`SIZE_MINIMIZED` | P | 없음 |
| WINC-063 | 스플리터·컬럼 경계 커서 | 도크 상단 띠(`strip_top-SPLIT_HALF ≤ y ≤ strip_top+gap+SPLIT_HALF`, `gap=max(2, SPLIT_TH*dpi/96)`)=↕(SIZENS); 도크 좌우 스플리터(`y≥band.y && band.right()-2 ≤ x < 우도크.x+2`) 또는 파일 스플리터(패널 y 범위 && `abs(x-splitter_x) ≤ max(1,SPLIT_HALF*dpi/96)`)=↔(SIZEWE); 컬럼 경계 리사이즈 존(`rows().resize_hot`)=↔; 그 외 기본 | `win.rs:7815`~7854 | `WM_SETCURSOR`·`LoadCursorW`·`SetCursor`·`GetCursorPos`·`ScreenToClient` | P(winit `set_cursor`) | 없음 |
| WINC-064 | 터미널 Shift+휠 = 가로 스크롤 | 조건: Shift && `!term_wrap` && 커서가 터미널 그리드 위. 노치=4열(`hwheel.add(-delta, 4*셀폭)`), 트랙패드=비례 px | `win.rs:7863`~7871 | `WM_MOUSEWHEEL` | P(휠 이벤트 번역) | 없음 |
| WINC-065 | 터미널 휠 = TUI 전달/스크롤백 | Shift 없음 && 터미널 위: TUI 마우스 모드(SGR)면 휠 버튼 64(위)/65(아래)를 노치 수만큼 전송. 아니면 `abs(delta)≥WHEEL_DELTA`=행 단위(`wheel_lines()` × 고속 스크롤 배수 `t.fast.wheel`), 미만=정밀 터치패드 픽셀 스크롤. 고속 HUD 보이면 위젯 틱 타이머(40ms) 직접 무장 | `win.rs:7872`~7904 | 〃 | P | 없음 |
| WINC-066 | 도크 정보/미리보기 위 휠 = 내용 스크롤 | `dock_text_at`(양 패널 **실제 도크 rect**로 판정 — target 불신): Shift면 `HWheel{-delta}`, 아니면 `Wheel{delta}`를 도크에 | `win.rs:7909`~7919 | 〃 | N | 없음 |
| WINC-067 | 파일 목록 휠 = hover 패널 스크롤 | **마우스 아래 패널**(활성 패널 아님; 스플리터 존/밖이면 활성 폴백)에 `Wheel`/Shift면 `HWheel{-delta}` | `win.rs:7859`, 7920~7929 | lparam=화면 좌표 → `ScreenToClient` | P | 없음 |
| WINC-068 | 가로 휠(틸트) | 터미널(비줄바꿈)=가로 스크롤(부호 그대로 `delta`), 도크 텍스트=`HWheel{delta}`, 그 외 hover 패널에 `HWheel{delta}` | `win.rs:7931`~7959 | `WM_MOUSEHWHEEL` | P | 없음 |
| WINC-069 | 좌클릭 공통 진입 | 툴팁 즉시 파괴(`tip_cancel`) → `drag_press=None`(이전 OLE 후보 폐기) → 좌표·Shift·Ctrl 추출 → **`SetCapture`**(스플리터·리사이즈·러버밴드 공용) | `win.rs:7960`~7971 | `SetCapture`·`MK_SHIFT`·`MK_CONTROL` | P | 없음 |
| WINC-070 | 메뉴바 클릭 우선 | 메뉴가 열려 있거나 `y < toolbar.y`면 메뉴바에만 라우팅 → `take_command` 있으면 `run_command`. 즉시 반환 | `win.rs:7973`~7980 | — | A | 없음 |
| WINC-071 | 도구모음/퀵 런처 바 클릭 | `y < panels[0].y`: 런처 바가 보이고(`h>0`) `y ≥ 런처.y`면 런처, 아니면 툴바 → `take_command` → `run_command` | `win.rs:7981`~7996 | — | A | 없음 |
| WINC-072 | 상태바 클릭 무시 | `y ≥ statusbar.y`면 아무 동작 없음(표시 전용) | `win.rs:7997`~7999 | — | A | 없음 |
| WINC-073 | 경로바 편집 중 클릭 | 제안 항목 클릭=그 폴더로 즉시 이동(`drain_actions`+`finish_input`). 필드 안 클릭=캐럿 배치. 필드 밖 클릭=**편집 취소**(포커스아웃) — 단 취소 후 그 클릭은 다른 분기를 타지 않는다(else-if 사슬) | `win.rs:8000`~8019 | — | A | 없음 |
| WINC-074 | 도크 높이 드래그 시작 | 도크 표시·h>0 && y가 상단 띠(WINC-063과 같은 식) → `dock_drag=Some(0)` + 전창 무효화(강조색) | `win.rs:8020`~8027 | — | A | 없음 |
| WINC-075 | 도크 좌/우 스플리터 드래그 시작 | `dock_shown && y ≥ band.y && band.right()-2 ≤ x < 우도크.x+2` → `dock_split_drag=true` | `win.rs:8028`~8035 | — | A | 없음 |
| WINC-076 | 파일 좌/우 스플리터 드래그 시작 | **싱글 패널이 아니고** `y < panels[0].bottom()` && `abs(x-sx) ≤ max(1, half)` → `split_drag=true` | `win.rs:8036`~8042 | — | A | 없음 |
| WINC-077 | 패널 클릭 = 활성 전환 | `panel_at_pt`(히트 존)로 패널 결정. `term_focus=None` 기본 해제. **공유 도크(SharedDock)** 클릭은 활성 패널 유지(위젯 라우팅은 idx=0 그대로), 그 외 `set_active(idx)` | `win.rs:8043`~8051 | — | N | `hit_zone_table`(WINC-170) |
| WINC-078 | 프레스 시점 행·기선택 판정 | 선택 반영 **전에** `(was_renaming, hit=(is_selected, path))` 계산: 리네임 **필드 안** 클릭·마커 hit은 제외. **기선택 행만** OLE DnD 후보(`drag_press=Some((x,y))` — 8151), 미선택 행 드래그=러버밴드 | `win.rs:8057`~8072, 8151~8153 | — | N | 없음 |
| WINC-079 | 반대 패널 도크 텍스트 선택 해제·탭 드래그 스냅샷 | 반대 패널 `dock.clear_text_selection`(복사 대상은 한 곳만) → 패널 `on_event(MouseDown)` → `drain_actions` → `tab_drag_undo = tabbar.pressed_tab().map((idx,i))` | `win.rs:8073`~8079 | — | N | 없음 |
| WINC-080 | 터미널 [→] = 현재 폴더로 이동 | `dock.take_goto()` 시: 공유 도크면 "현재 폴더"=활성 패널. 터미널이 종료 상태면 `terms[idx]=None`(재시작, cwd=현재 폴더 lazy start), 살아 있으면 `cd "<dir>"\r` 전송 + 스크롤백 해제. 캐럿 타이머 무장, `term_focus=Some(idx)`, 편집 정리 | `win.rs:8080`~8099 | PTY·**셸 문법(`cd "…"`)** | P | 없음 |
| WINC-081 | 터미널 클릭 = 키 포커스·선택 시작 | 조건 `dock_shown && y ≥ dock.y && active_kind()==2`(숨은 0-rect 터미널이 포커스를 가져가지 않게). 포커스 설정 + 편집 정리 + 캐럿 타이머. 그리드 안: Shift 없고 TUI 마우스 모드면 버튼 0 press 전송(`term_mouse_btn=Some((idx,0))`), 아니면 로컬 선택 시작(`sel=(cell,cell)`, `term_drag`) | `win.rs:8100`~8131 | `GetCaretBlinkTime`·`SetTimer` | P(캐럿 주기) | 없음 |
| WINC-082 | 느린 재클릭 = 이름 바꾸기 예약 | 새 클릭마다 `rename_on_up=false` + 예약 폐기. 히트 행이 **기선택** && Shift/Ctrl 없음 && 직전에 리네임 중 아님 && 이전 `slow_click`이 같은 (패널, 경로) && 경과 ≥ `SLOW_CLICK_RENAME_MS`(1000ms) → `rename_on_up=true`. `slow_click` 시드는 항상 갱신(행 없으면 None) | `win.rs:8132`~8150 | — | N | 없음(만료 판정만 WINC-168) |
| WINC-083 | 클릭 후 포커스 강조 동기 | `sync_focus_visuals`(실제 키 포커스 영역만 강조) + `finish_input`(flush→title→status) | `win.rs:8155`~8157 | — | N | 없음 |
| WINC-084 | 도구모음 빈 영역·컬럼 헤더 우클릭 팝업 | 툴바 bounds 안이면서 버튼 위가 아님 → `show_bar_popup(true)`. 컬럼 헤더 영역 → `set_active` + `show_bar_popup(false)` | `win.rs:8161`~8178 | 네이티브 팝업 | A | 없음 |
| WINC-085 | TUI 마우스 모드 우클릭 전달 | Shift 없음 && 포커스 터미널 그리드 위 && 마우스 모드면 버튼 2 press 전송 + `SetCapture` + `term_mouse_btn=Some((ti,2))`(Shift=로컬 우회) | `win.rs:8185`~8199 | `GetKeyState(VK_SHIFT)`·`SetCapture` | P | 없음 |
| WINC-086 | 우클릭 누름 = 선택 규약 반영·메뉴 선행 구축·탭 메뉴 | 예약 리네임 폐기, `rbutton_down_seen=true`. 패널 결정(공유 도크는 활성 유지) → `on_event(RightDown)` → `rclick_began_edit = (이 우클릭이 경로바 편집을 시작시킴)` → `drain_actions` → `prebuild_ctx_menu`(선행 구축) → `take_tab_menu()` 있으면 State 참조 밖에서 `show_tab_menu` | `win.rs:8180`~8221 | — | A(탭 메뉴)·P(셸 메뉴 선행 구축) | 없음 |
| WINC-087 | 우클릭 뗌 — TUI 릴리스/짝 없는 뗌 무시 | `term_mouse_btn==(ti,2)`면 release 전송 + `ReleaseCapture`(State 참조 밖). `rbutton_down_seen`이 false(누름을 모달·다른 창이 흡수)면 **메뉴를 열지 않는다** | `win.rs:8223`~8246 | `ReleaseCapture` | P | 없음 |
| WINC-088 | 우클릭 뗌 — 텍스트 편집 메뉴 | `rclick_began_edit`이면 이번엔 메뉴 억제(다음 우클릭부터). `edit_menu_target_at` 대상이 있으면: Term이면 포커스 이동·캐럿 타이머·편집 정리·강조 동기 후 `show_edit_popup` | `win.rs:8247`~8270 | — | A | 없음 |
| WINC-089 | 우클릭 뗌 — 셸 컨텍스트 메뉴 | 좌표 패널이 활성 패널일 때만: 행 위=`show_row_context_menu(false)`, 본문 빈 영역(`in_body`)=`show_background_context_menu` | `win.rs:8271`~8283 | `IContextMenu`(shellmenu) | P(§4) | 없음 |
| WINC-090 | 셸 메뉴 동적 서브메뉴·아이콘 | `WM_INITMENUPOPUP`/`WM_DRAWITEM`/`WM_MEASUREITEM`/`WM_MENUCHAR`를 `shellmenu::forward_menu_msg`(IContextMenu2/3)로 포워딩, None이면 기본 처리 | `win.rs:8286`~8295 | IContextMenu2/3 | W | 없음 |
| WINC-091 | 마우스 이동 — TUI 모션 전달 | `term_mouse_btn=(ti,b)`이고 해당 버튼이 실제로 눌려 있으면(`tui_btn_held`) 모드 ≥1002일 때 `b+32`(모션 플래그)로 전송 후 **조기 반환**; 안 눌렸으면 상태 해제 | `win.rs:8301`~8314 | `MK_LBUTTON`/`MK_RBUTTON` 비트 | N(버튼 상태 취득만 P) | `tui_btn_held_per_button`(WINC-169) |
| WINC-092 | 마우스 이동 — 터미널 선택 드래그 | 좌버튼 유지 중: `term_drag_extend`(그리드 밖이면 1줄 자동 스크롤 후 클램프) + 도크 무효화. 그리드 상/하 밖이면 60ms 반복 타이머(`TIMER_TERM_SEL`), 안이면 타이머 해제. 버튼이 풀렸으면 드래그 종료 | `win.rs:8315`~8337 | `SetTimer` | N | 없음 |
| WINC-093 | 마우스 이동 — 파일 드래그(OLE) 발신 | 좌버튼 유지 && `drag_press` 있음 && 이동이 `max(4, SM_CXDRAG/SM_CYDRAG)` 이상 → `drag_press=None`, `rename_on_up=false`, 대상=`keyboard_targets`에서 전송 중 대상 제외. `ReleaseCapture` 후 `dnd::begin_drag(paths)`(모달). 종료 후 **양 패널 `abort_press()`**(DoDragDrop이 MouseUp을 소비) → 드롭 성공이면 `reload_both`, 취소면 재열거 생략 | `win.rs:8338`~8378 | `GetSystemMetrics(SM_CXDRAG)`·OLE `DoDragDrop` | P(§4) | 없음 |
| WINC-094 | 도크 높이 드래그 중 | `ratio=(bottom-y)/(bottom-top)`(top=패널 상단, bottom=도크 하단) → 양 패널 `set_dock_ratio` → `layout` → `update_dock_info` → 전창 무효화 | `win.rs:8383`~8394 | — | A | 없음 |
| WINC-095 | 도크 좌/우 스플리터 드래그 중 | `snap_split_x(x, 파일 스플리터 x)` → `dock_split = clamp(x/w, 0.15, 0.85)` → `layout` | `win.rs:8395`~8403 | `GetKeyState(VK_MENU)`(Alt=스냅 해제) | A | 없음 |
| WINC-096 | 파일 좌/우 스플리터 드래그 중 | `other = w*dock_split` → `snap_split_x(x, other)` → `split = clamp(x/w, 0.1, 0.9)` → `layout`. 스냅: 창 50%·반대편 구분선에 `SNAP_PX(20)*dpi/96` 이내면 정렬(반대편 스냅은 도크 표시 중일 때만) | `win.rs:8404`~8413, 6149~6164 | 〃 | A | 없음 |
| WINC-097 | 마우스 hover 라우팅 | 메뉴바·툴바에 `MouseMove` → `tip_on_mousemove` → 런처 바(h>0일 때) → 메뉴가 **닫혀 있을 때만** 양 패널에 전달(드롭다운 아래 hover 잔상 방지) → `sync_col_widths` → 양 패널 `drain_actions`(탭 드래그 재정렬 Move 즉시 반영) | `win.rs:8414`~8430 | — | A | 없음 |
| WINC-098 | 패널 간 탭 드래그 미리 보기 | 듀얼 패널에서 드래그 중인 탭을 **반대 패널 탭 바 위**(탭 또는 빈 영역)로 끌면 즉시 `cross_move_tab(adopt:false)` → 반대 패널에서 `begin_drag(landed)` 이양, 원 패널 `cancel_drag`. 본문 위는 미리 보기 없음(해제 시 끝 삽입) | `win.rs:8431`~8465 | — | A | 없음 |
| WINC-099 | 캡처 상실 시 과도 상태 일괄 정리 | `reset_mouse_transients`: 스플리터 3종 플래그·터미널 선택 타이머·`drag_press`·`rename_on_up`·`term_mouse_btn`·탭 드래그(원위치 복귀) 정리, 강조 중이었으면 재도장 | `win.rs:8472`~8480, 6002~6017 | `WM_CAPTURECHANGED` | P(포커스 상실/커서 이탈로 대체) | 없음 |
| WINC-100 | 좌클릭 뗌 — 드래그 종료 | `drag_press=None`; 도크 높이/도크 스플리터/파일 스플리터 드래그 종료 시 전창 무효화(강조 해제); TUI 버튼 0이면 release 전송(버튼 2면 상태 유지 — RBUTTONUP 몫); 터미널 선택 확정(선택 유지, 타이머 종료) | `win.rs:8481`~8511 | — | N | 없음 |
| WINC-101 | 좌클릭 뗌 — 패널 간 탭 이동 커밋 | 위젯이 MouseUp에서 드래그를 지우므로 **라우팅 전에** 판정. ① 미리 보기로 이미 반대 패널에 있으면 값 채택만(`view_scope != "tab"`일 때 `adopt_panel_view`) ② 원 패널에서 드래그 중 + 해제 지점이 반대 패널(본문 포함) → `cross_move_tab(adopt:true)`(탭 바 위면 그 위치, 그 외 끝) | `win.rs:8514`~8550 | — | N | 없음 |
| WINC-102 | 좌클릭 뗌 — 선택 확정 동기 | 양 패널에 `MouseUp` → `sync_col_widths` → 선택 시그니처(패널별 선택 수·캐럿)가 바뀐 경우에만 `finish_input`(아니면 flush만 — 클릭당 도크 재생성 2회 방지) | `win.rs:8551`~8562, 5099~5105 | — | N | 없음 |
| WINC-103 | 미리보기 "크게" 버튼 | 도크 `take_popout()` 시 독립 미리보기 창(모달). 싱글 정보면 원천=활성 패널 | `win.rs:8563`~8570 | 독립 창 | A | 없음 |
| WINC-104 | 느린 재클릭 리네임 — 지연 시작 | `rename_on_up`이면 대상 스냅샷 `pending_rename=(패널, 경로)` + `TIMER_RENAME`(**더블클릭 시간**만큼 지연) | `win.rs:8571`~8581 | `GetDoubleClickTime` | P(더블클릭 시간) | 없음 |
| WINC-105 | 좌클릭 뗌 — 캡처 해제 규칙 | TUI 우버튼이 아직 눌린 채(`term_mouse_btn=(_,2)`)면 캡처 유지, 아니면 `ReleaseCapture`(State 참조 밖 — 동기 재진입) | `win.rs:8583`~8590 | `ReleaseCapture` | P | 없음 |
| WINC-106 | 마우스 뒤로/앞으로 버튼 | XBUTTON1=`nav_back`, XBUTTON2=`nav_forward`(활성 패널) + `finish_input` | `win.rs:8592`~8604 | `WM_XBUTTONDOWN` | P(winit `MouseButton::Back/Forward`) | 없음 |
| WINC-107 | 더블클릭 — 가드 | `TIMER_RENAME` 해제·`pending_rename=None`. 공유 도크 더블클릭=무동작. 리네임 **필드 안**=전체 선택(열기 아님). 경로바 편집 필드 안=전체 선택 | `win.rs:8605`~8647 | `WM_LBUTTONDBLCLK`(CS_DBLCLKS) | P(더블클릭 자체 검출) | 없음 |
| WINC-108 | 탭 더블클릭 동작 | 설정 `tab_dblclick`: `"pin"`=고정 토글, `"lock"`=잠금 토글, 그 외(기본)=닫기 | `win.rs:8648`~8658 | — | N | 없음 |
| WINC-109 | 탭 바 빈 공간 더블클릭 = 새 탭 | `tabbar.empty_area_at` → `new_tab` + `finish_input` | `win.rs:8659`~8666 | — | N | 없음 |
| WINC-110 | 컬럼 경계 더블클릭 = auto-fit | `rows().autofit_col_at(x,y)` → `autofit_column`(WINC-029) | `win.rs:8667`~8674 | — | A | 없음 |
| WINC-111 | 행 더블클릭 = 열기 | 마커 hit 제외. `activate_row`(폴더=진입, 파일=경로 반환) → `finish_input` → 파일이면 State 참조 밖에서 `guarded_shell_open` | `win.rs:8675`~8692 | — | N(실행은 P) | 없음 |
| WINC-112 | 키 입력 = 예약 리네임 폐기 | Shift·Ctrl **단독**이 아닌 모든 키에서 `cancel_pending_rename` | `win.rs:8694`~8703 | `GetKeyState` | N | 없음 |
| WINC-113 | 경로바 편집 중 키 | Enter=제출(+`drain_actions`+`finish_input`), Esc=제안 팝업 열림이면 팝업만 닫기/아니면 편집 취소, ↑/↓=제안 이동(선택 미리 채움), Ctrl+C/X/V/Z=`do_clip`, 편집 키(WINC-055; Delete일 때만 자동완성 갱신). 처리 후 반환(목록 단축키 차단) | `win.rs:8706`~8739 | — | A | 없음 |
| WINC-114 | 터미널 포커스 중 키 라우팅 | `term_key_route`: Term=종료 상태면 아무 키로 재시작(`terms[ti]=None`), 아니면 비문자 키를 VT 시퀀스로(↑`ESC[A` ↓`ESC[B` →`ESC[C` ←`ESC[D` Home`ESC[H` End`ESC[F` Del`ESC[3~` PgUp`ESC[5~` PgDn`ESC[6~`). TermPending=키 삼킴 + 무효화. ClearFocus=낡은 포커스 해제 후 목록 경로 계속. List=통과 | `win.rs:8740`~8771, 6096~6145 | — | N | `key_route_mcdc_pairs`(WINC-173) |
| WINC-115 | 인라인 이름변경 중 키 | Enter=`submit_rename`→`apply_rename`, Esc=취소, Ctrl+C/X/V/Z=`do_clip`, 편집 키 | `win.rs:8772`~8792 | — | A | 없음 |
| WINC-116 | Esc 우선순위 사슬 | ① 탭 드래그 취소(원위치) ② 컬럼 드래그 취소 ③ 진행 중 전송 취소(`cancel` 플래그) ④ 열린 메뉴바 닫기 | `win.rs:8793`~8812 | — | N | 없음 |
| WINC-117 | F5/F6/F3 | F5=새로 고침. F6=테마 순환(다크→라이트→시스템→다크). F3=독립 미리보기 창(활성 패널), Shift+F3=스크롤 벤치(개발용, 200프레임) | `win.rs:8813`~8833 | — | N | 없음 |
| WINC-118 | Tab·탭 단축키 | Tab=패널 전환(`set_active(1-active)` — 싱글 패널이면 0 고정), Ctrl+Tab=다음 탭, Ctrl+T=새 탭, Ctrl+W=활성 탭 닫기 | `win.rs:8834`~8845 | — | N(주 수식키는 P) | 없음 |
| WINC-119 | 클립보드·실행 취소 단축키 | Ctrl+A/C/X=`do_clip`. Ctrl+V=`do_clip(Paste)` 성공 시 반환(실패 시 `finish_input`으로 낙하). Ctrl+Z=실행 취소, Ctrl+Shift+Z·Ctrl+Y=다시 실행 | `win.rs:8846`~8863 | — | N(주 수식키는 P) | 없음 |
| WINC-120 | Enter·F2·Delete·Apps | Enter=캐럿 행 활성화(파일이면 실행 게이트). F2=캐럿 행 인라인 이름변경. Delete=휴지통, Shift+Delete=완전 삭제. Apps 키=캐럿 행 컨텍스트 메뉴 | `win.rs:8864`~8878 | VK_APPS | N(키 관례는 P) | 없음 |
| WINC-121 | 기능 단축키 | Ctrl+Shift+N=새 폴더, Ctrl+Shift+R=일괄 이름변경, Ctrl+H=숨김 토글, Ctrl+.=닷파일 토글, Ctrl+,=설정 창, Ctrl+`=하단 도크 토글 | `win.rs:8879`~8896 | VK_OEM_* | N(키 관례는 P) | 없음 |
| WINC-122 | 목록 탐색 키 | `vk_to_key` 매핑 키를 `InputEvent::Key{key,shift,ctrl}`로 활성 패널에. 모든 미반환 경로 끝에 `finish_input` | `win.rs:8897`~8903 | — | N | 없음 |
| WINC-123 | Alt 조합(SYSKEY) | Shift+F10=캐럿 행 컨텍스트 메뉴. Alt+←=뒤로, Alt+→=앞으로, Alt+↑=상위, Alt+↓=캐럿 행 활성화(더블클릭 동등). 처리 시 `finish_input` 후 반환, 아니면 기본 처리 | `win.rs:8905`~8939 | `WM_SYSKEYDOWN` | P(키 관례) | 없음 |
| WINC-124 | 폴더 변경 통지 수신 | `wparam`=패널(`min(1)`), `lparam`=세대. 해당 패널 watcher 중 같은 세대가 있을 때만 `arm_watch_debounce` | `win.rs:8941`~8950 | `WM_APP_FSCHANGE` | P(워커→UI 통지) | 없음 |
| WINC-125 | 터미널 출력/종료 통지 | `wparam`=패널 + `EXIT_FLAG`, `lparam`=세대(같을 때만). 종료=`exited=true`. 출력=`pty.output` 버퍼를 take해 `screen.feed`. **스크롤백 보기 중이면 늘어난 줄 수만큼 `view_off` 증가**(보던 위치 고정). 도크 rect만 무효화 | `win.rs:8952`~8983 | `WM_APP_TERM` | P | 없음 |
| WINC-126 | 모달 창 지연 실행 | `WM_APP_PREFS`=설정 창, `WM_APP_EDIT_TOOLBAR`/`WM_APP_EDIT_COLS`=순서 편집 창, `WM_APP_BULK`=일괄 이름변경, `WM_APP_CTLDEMO`=컨트롤 갤러리(개발 전용, 메뉴 비노출), `WM_APP_ABOUT`=About. 전부 **State 차용 밖** 시점에서 모달 | `win.rs:8984`~8995, 9039~9057 | PostMessage 지연 | A | 없음 |
| WINC-127 | 순서 편집 창 실시간 값 | `WM_APP_ORDER_EDIT`: lparam=`*const String`(즉시 복사). field=툴바: `toolbar_order` 정규화(`parse`→`serialize`) + 툴바 재구성 + `persist_settings`. field=컬럼: 활성 패널에 레이아웃 적용 + sync on이면 반대 패널(**영속은 세션 경유 — 여기서 저장 안 함**) | `win.rs:8996`~9038 | SendMessage 포인터 | A | 없음 |
| WINC-128 | 설정 창 즉시 적용 통지 | `WM_APP_PREFS_APPLY`: lparam=`*const PrefValues`(SendMessage 동안만 유효 → 즉시 clone) → `apply_prefs` | `win.rs:9058`~9066 | 〃 | A | 없음 |
| WINC-129 | 파일별 아이콘 로딩 결과 | `WM_APP_ICON`: wparam=`Box<LoadResult>` 소유권 인수(항상 해제). 반영되면 **양 패널 목록 본문 + 런처 바만** 무효화. State가 없으면 HICON 파괴 | `win.rs:9067`~9090 | `DestroyIcon`·HICON | P(아이콘 표현은 RGBA로) | 없음 |
| WINC-130 | 전송 워커 통지 | `WM_APP_TRANSFER`: wparam=세대, lparam 1=완료/0=진행 → `on_transfer_message` | `win.rs:9091`~9097 | — | P(통지 경로) | 없음 |
| WINC-131 | 휴지통 삭제 워커 완료 | `WM_APP_DELETE`: wparam 1=성공 → `on_delete_message` | `win.rs:9098`~9104 | — | P | 없음 |
| WINC-132 | 클라우드 목록 적재 완료 | lparam=`Box<ListResult>`. **양 패널 전 클라우드·내 PC 탭** `reopen_cloud_tabs`(활성 탭만 갱신하면 다른 탭이 로딩 표시로 남음). 실패 시 타이틀에 ` · {err} ({폴더 또는 "/"})` | `win.rs:9105`~9133 | — | N(통지 경로 P) | 없음 |
| WINC-133 | 클라우드 전송 진행 틱 | `on_cloud_progress`(진행 창 갱신 + 취소 폴링) | `win.rs:9134`~9140 | — | N | 없음 |
| WINC-134 | 메뉴 스레드 결과 반영 | lparam=`Box<(MenuReq, Outcome)>`. `ctx_showing=false`; wparam 세대가 `ctx_gen`과 같을 때만 `apply_row_menu_outcome` + 상태 갱신. 닫힌 직후 **즉시 재구축**(연속 우클릭 지연 400~700ms → 수십 ms) | `win.rs:9144`~9162 | 전용 메뉴 스레드 | P/W | 없음 |
| WINC-135 | 도크 Info 상세 도착 | lparam=`Box<(gen, PathBuf, Vec<DetailLine>)>`, wparam=레인. 그 레인의 최신 요청과 세대·경로가 맞을 때만 `info_details[lane]` 반영 + `update_dock_info` | `win.rs:9163`~9184 | — | N(상세 수집은 P) | 없음 |
| WINC-136 | 셸 변경 통지 | `WM_APP_SHCHANGE_BASE+0/+1`: 페이로드는 해석 없이 해제(`shellnotify::release_payload`), 패널별 `arm_watch_debounce` | `win.rs:9185`~9192 | `SHChangeNotifyRegister` | W(타 OS는 watcher+폴링으로 흡수) | 없음 |
| WINC-137 | 가상 파일 붙여넣기 완료 | wparam 0 && 취소 플래그=`ops.canceled`, 그 외 `ops.doneClosing`. 진행 창 마감 → undo 기록(`vpaste_roots`) → `reload_both` | `win.rs:9193`~9211 | 가상 파일 | W | 없음 |
| WINC-138 | 클라우드 다운로드 완료 | `open_after`면 받은 파일마다 포그라운드 양도 + `ShellExecuteW("open")`, 아니면 `reload_both`. 문구: 취소=`ops.canceled`, 오류=그 문자열, 정상=`cloud.downloaded`(개수). 진행 창 마감 + 타이틀·상태 | `win.rs:9212`~9247 | `ShellExecuteW` | P | 없음 |
| WINC-139 | 클라우드 쓰기 완료 | 양 패널 클라우드 탭 재열기. 문구: 성공=`cloud.writeDone`(개수), 취소=`ops.canceled`, 오류에 "403"/"401" 포함=`cloud.err.reauth`(재연결 안내), 그 외 오류 문자열 | `win.rs:9248`~9278 | — | N | 없음 |
| WINC-140 | 클라우드 OAuth 완료 | lparam=`Box<CloudAuthResult>` 회수 → `on_cloud_auth` | `win.rs:9279`~9288 | — | N | 없음 |
| WINC-141 | 접근성 선택 요청 | `WM_APP_UIA_SELECT`: wparam=전역 행 인덱스, lparam=`SEL_SINGLE`/`SEL_ADD`(미선택일 때만 토글)/`SEL_REMOVE`(선택일 때만 토글). `select_program`이 범위 검사 | `win.rs:9289`~9312 | UI Automation | W(타 OS 대체는 §4) | 없음 |
| WINC-142 | 접근성 루트 프로바이더 | `WM_GETOBJECT`에서 `UIA_ROOT_OBJECT_ID`면 활성 패널 가시 행 스냅샷 프로바이더 반환 | `win.rs:9313`~9322 | UIA | W | 없음 |
| WINC-143 | IME 조합 창 위치 | `WM_IME_STARTCOMPOSITION`/`WM_IME_COMPOSITION`에서 `position_ime`(편집 캐럿 옆) 후 기본 처리(결과 문자열은 `WM_CHAR`로 수신) | `win.rs:9323`~9330 | IMM | P(winit `Ime`·`set_ime_cursor_area`) | 없음 |
| WINC-144 | 문자 입력 — 터미널 | Term/TermPending일 때: 종료 상태면 재시작. `0x03`(Ctrl+C) && 선택 있음=복사(없으면 인터럽트로 그대로 전송). `0x16`(Ctrl+V)=붙여넣기. **Backspace(0x08)→`0x7F`, `0x7F`(Ctrl+BS)→`0x08`** 교차 매핑. 그 외 UTF-8 그대로. 입력 시 스크롤백·선택 해제, 캐럿 위상 리셋. PTY 미기동이어도 문자를 **삼킨다**(목록 타입어헤드로 새지 않게) | `win.rs:9331`~9381 | ConPTY 키 해석 | P(§4) | `key_route_mcdc_pairs` |
| WINC-145 | 문자 입력 — 경로바 편집 | Backspace 또는 비제어 문자만 `edit_char` + 자동완성 갱신 | `win.rs:9383`~9387 | — | A | 없음 |
| WINC-146 | 문자 입력 — 목록 타입어헤드/리네임 | Ctrl 미누름 && (Backspace 또는 비제어 문자). **스페이스는 선택 토글 키**라 제외 — 단 리네임 중이거나 타입어헤드 버퍼가 비어 있지 않으면 문자로 전달. 버퍼가 있으면 `TIMER_TYPEAHEAD`(250ms). 편집 중이 아니면 `finish_input`(선택 이동 동기) | `win.rs:9388`~9419 | `GetKeyState(VK_CONTROL)` | N | 없음 |
| WINC-147 | 세션 디바운스 자동 저장 | `TIMER_SESSION_SAVE` 만료 시 타이머 해제 후 `current_session` 1회 저장(원자적 쓰기, 실패 무해) | `win.rs:9422`~9430 | `SetTimer`/`KillTimer` | N(타이머 P) | 없음 |
| WINC-148 | 툴팁 hover 틱 | `TIMER_TIP`(250ms) → `tip_tick` | `win.rs:9431`~9436 | — | A | 없음 |
| WINC-149 | 전송 진행 창 자동 닫기 | `TIMER_PROG_CLOSE` 만료 → `transfer_close=None`(drop=창 파괴) | `win.rs:9437`~9444 | — | A | 없음 |
| WINC-150 | 클라우드 취소 폴링 | `TIMER_CLOUD_POLL`(200ms): `cloud_shared` 있으면 `on_cloud_progress`, 없으면 타이머 해제 | `win.rs:9445`~9456 | — | N | 없음 |
| WINC-151 | watcher 디바운스 만료 = 무간섭 재로드 | `TIMER_WATCH_BASE+패널`(300ms). 그 패널이 리네임 중·경로바 편집 중이거나 전송·휴지통 삭제 진행 중이면 **미루고 재무장**. 아니면 `watch_since=0` → `reopen_filtered`(펼침·선택·캐럿·스크롤 보존) → 타이틀·상태 → `refresh_probe_baseline`(진동 차단) | `win.rs:9457`~9485 | — | N | 없음 |
| WINC-152 | 컨텍스트 메뉴 선행 구축 타이머 | `TIMER_CTX_PREBUILD`(300ms) 1회 → `prebuild_ctx_menu` | `win.rs:9486`~9492 | — | W(셸 메뉴 구축 비용 회피용) | 없음 |
| WINC-153 | 위젯 틱(페이드 애니메이션) | `TIMER_WIDGET_TICK`(40ms): 양 패널 rows·dock tick + 터미널 고속 스크롤 배지 tick. 더 이상 틱 요청이 없으면 타이머 해제(flush가 재무장) | `win.rs:9493`~9521 | — | A | 없음 |
| WINC-154 | 타입어헤드 타임아웃 틱 | 250ms마다 양 패널 rows tick, 양쪽 버퍼가 비면 타이머 해제 | `win.rs:9522`~9534 | — | N | 없음 |
| WINC-155 | 아이콘 로딩 큐 틱 | `icons.tick` 반영 시 양 패널 목록 본문 + 런처 바만 무효화. 대기 없으면 타이머 해제 | `win.rs:9535`~9549 | — | P | 없음 |
| WINC-156 | 터미널 캐럿 깜빡임 | 포커스 있으면 `term_caret_on` 토글 + 도크 무효화, 없으면 on으로 복원 후 타이머 해제 | `win.rs:9550`~9563 | `GetCaretBlinkTime`(0/MAX → 530ms) | P | 없음 |
| WINC-157 | 터미널 선택 엣지 자동 스크롤 | 60ms마다 커서 위치로 `term_drag_extend`; 드래그가 끝났으면 타이머 해제 | `win.rs:9564`~9582 | `GetCursorPos`·`ScreenToClient` | P(커서 위치 취득) | 없음 |
| WINC-158 | 지연 리네임 진입 | `TIMER_RENAME` 만료: `pending_rename`을 take, **지금의** 활성 패널·캐럿 행 경로와 일치할 때만 `begin_rename_caret` | `win.rs:9583`~9602, 3983~3995 | — | N | `rename_timer_fires_only_on_matching_row_and_panel`(WINC-168) |
| WINC-159 | DnD 추적 폴링 | `TIMER_DND`(100ms): 커서 위치로 `dnd_track_update`(정지 커서에서도 엣지 스크롤·호버 대기 판정) | `win.rs:9603`~9610 | `GetCursorPos` | P | 없음 |
| WINC-160 | 폴더 변경 프로브 폴링 | `TIMER_FSPOLL`(활성 3s/비활성 30s/최소화 정지): `sync_watchers`(죽은 watcher 자가 치유) + `poll_fs_probe` | `win.rs:9611`~9618 | — | N | 없음 |
| WINC-161 | 유휴 트림 | `TIMER_JANITOR`(10s): `should_trim`(유휴 60s && 미트림)이면 `trim_resident` + 타이머 해제(유휴 백그라운드 0%) | `win.rs:9619`~9628, 2012~2025 | `SetProcessWorkingSetSize` | P(§4) | `idle_trim_threshold_and_once`(WINC-174) |
| WINC-162 | DPI 변경 | 새 DPI 저장 → 백엔드 `set_dpi` → 양 패널·메뉴바·툴바·런처·상태바 메트릭 재설정 → 제안 rect로 `SetWindowPos` → `layout` | `win.rs:9631`~9661 | `WM_DPICHANGED` | P(winit `ScaleFactorChanged`) | 없음 |
| WINC-163 | OS 설정 변경 | `sync_wheel_lines`(마우스 휠 줄 수) + 테마가 System 모드면 `apply_theme` | `win.rs:9663`~9673 | `WM_SETTINGCHANGE` | P(winit `ThemeChanged`) | 없음 |
| WINC-164 | 종료 저장 | `WM_DESTROY`: 설정 → 세션 순으로 저장, **둘 다 성공 시에만** `config::purge_legacy`(구 .txt 정리), 실패는 stderr. `PostQuitMessage(0)` | `win.rs:9674`~9689 | — | N | 없음 |
| WINC-165 | 잘라내기 흐림 표시 동기 | `WM_CLIPBOARDUPDATE`: `clipboard::sync_cut_marks()`가 변경을 보고하면 양 패널 목록 본문 무효화 | `win.rs:9690`~9701 | `AddClipboardFormatListener` | P(§4) | 없음 |
| WINC-166 | 창 파괴 정리 | `WM_NCDESTROY`: 클립보드 구독 해제, `RevokeDragDrop`, State 포인터 회수·drop | `win.rs:9702`~9711 | OLE·클립보드 리스너 | P | 없음 |
| WINC-167 | 그 외 메시지 | `DefWindowProcW` 위임(시스템 메뉴·비클라이언트 등 OS 기본 동작) | `win.rs:9712` | — | P(winit 기본) | 없음 |

### 1-D. 기존 단위 테스트(`mod tests` — 7건)

| ID | 테스트 함수 | 검증 내용 | 대상 함수 | 진입점 | 분류 |
|---|---|---|---|---|---|
| WINC-168 | `rename_timer_fires_only_on_matching_row_and_panel` | 지연 리네임 만료 판정표 7케이스: 같은 패널·같은 행=발화 / **ASCII 대소문자 무시** / **끝 구분자 무시** / 다른 패널=불발 / 캐럿이 다른 행=불발 / 캐럿 없음=불발 / 예약 없음=불발 | `rename_timer_should_fire`(`win.rs:3983`) | `win.rs:9729` | N — 단 경로 리터럴이 `C:\…`이고 대소문자 무시가 Windows 규약(§4) |
| WINC-169 | `tui_btn_held_per_button` | TUI 버튼 유지 판정 6케이스: (좌,좌눌림)=T / (우,우눌림)=T / (우,없음)=F(종전 결함) / (좌,우만)=F / (우,좌만)=F / (미지 코드 1, 양쪽)=F | `tui_btn_held`(`win.rs:6023`) | `win.rs:9768` | N |
| WINC-170 | `hit_zone_table` | 히트 존 표 15단언: 듀얼 정보(밴드 안=도크 스플리터 기준·밴드 밖=파일 스플리터 기준, 밴드 좌측 끝은 파일 분할과 무관하게 좌), 싱글 정보(밴드 안 전부 `SharedDock`, 밴드 밖은 종전), `owner()`(SharedDock→Some(0), Split→None), 도크 숨김(밴드 None=파일 스플리터 기준) | `hit_zone_impl`(`win.rs:1320`) | `win.rs:9786` | N |
| WINC-171 | `split_staged_only_dnd_staging_sources` | `<tmp>\NexaDir\dnd-*\…` 출신만 스테이징으로 분리(`NexaDir\cloud`, 임시 폴더 밖, 다른 접두 `7zAbc`는 일반) | `is_dnd_staging`·`split_staged`(`win.rs:2211`, 2223) | `win.rs:9852` | N(경로 리터럴은 플랫폼 중립화 필요) |
| WINC-172 | `cleanup_staging_slots_removes_only_empty_dirs` | 실제 임시 폴더에 슬롯 2개 생성: 빈 슬롯은 제거, 잔존 파일이 있는 슬롯과 그 기반 폴더는 보존 | `cleanup_staging_slots`(`win.rs:2235`) | `win.rs:9892` | N |
| WINC-173 | `key_route_mcdc_pairs` | 키 라우팅 MC/DC 6단언: 기준(포커스+표시+종류2+PTY)=Term / 포커스 없음=List(2건) / 도크 숨김=ClearFocus / 종류≠2=ClearFocus / PTY 미기동=**TermPending**(목록 아님) | `route_key_with_term`(`win.rs:6096`) | `win.rs:9916` | N |
| WINC-174 | `idle_trim_threshold_and_once` | 유휴 트림 5단언: 59,999ms=미달 / 60,000ms=트림 / 이미 트림=재실행 없음 / 활동 후 40s=미달 / 시계 역전=포화 감산으로 안전 | `should_trim`(`win.rs:2012`) | `win.rs:9941` | N |

## 2. 화면·컨트롤 배치(이 구간이 규정하는 것)

창 전체 배치 계산(`layout`, `win.rs:1848`)은 다른 구간 담당이다. 이 구간은 **입력 라우팅 순서(= 사실상의 z-순서)·히트 존·스플리터 치수·팝업 메뉴 구성·단축키**를 규정한다.

### 2-1. 세로 영역 순서(좌클릭 라우팅이 전제하는 y 경계 — `win.rs:7973`~7999)

위에서 아래로: **메뉴바**(`y < toolbar.y`) → **도구모음** → **퀵 런처 바**(`launcherbar.bounds().h > 0`일 때만, `y ≥ launcherbar.y`) → **패널 영역**(`y ≥ panels[0].y`; 탭 바·경로바·파일 목록·도크) → **상태바**(`y ≥ statusbar.y`, 클릭 무반응).

좌클릭 판정 순서(먼저 맞는 것이 소비):
1. 메뉴바(열려 있으면 좌표 무관 전용 라우팅)
2. 도구모음/런처 바
3. 상태바(무시)
4. 활성 패널 경로바 편집 중: 제안 클릭 → 필드 안 → 필드 밖(취소)
5. 도크 상단 가로 분리선(높이 드래그)
6. 도크 좌/우 스플리터
7. 파일 좌/우 스플리터(싱글 패널이면 없음)
8. 패널(히트 존 → 위젯 소유 패널)

### 2-2. 스플리터 치수 상수(@96dpi, DPI 스케일은 `v*dpi/96`)

| 상수 | 값 | 용도 | 근거 |
|---|---|---|---|
| `SPLIT_TH` | 3 | 스플리터 공통 두께. 도크 상단 띠 `gap = max(2, SPLIT_TH*dpi/96)` | `win.rs:53`, 7824, 8021 |
| `SPLIT_HALF` | 3 | 히트 반폭. 파일 스플리터는 `max(1, SPLIT_HALF*dpi/96)`. **도크 상단 띠는 스케일하지 않은 `SPLIT_HALF`를 더한다**(`strip_top - SPLIT_HALF … strip_top + gap + SPLIT_HALF`) — 의도인지 누락인지 미확인(추정: 누락) | `win.rs:206`, 7834, 8023 |
| 도크 좌우 스플리터 히트 | `band.right()-2 ≤ x < 우도크.x+2` | 여유 2px(비스케일) | `win.rs:7828`, 8030 |
| `SNAP_PX` | 20 | 자석 스냅 임계(창 50%·반대편 구분선). **Alt 유지 = 스냅 해제** | `win.rs:55`, 6149 |
| `MIN_PANEL` | 200 | 패널 최소 폭(논리 px) — 사용처는 `layout`(타 구간) | `win.rs:205` |
| `split` 범위 | 0.1~0.9 | 파일 스플리터 비율 | `win.rs:8410` |
| `dock_split` 범위 | 0.15~0.85 | 도크 스플리터 비율 | `win.rs:8400` |
| 도크 높이 비율 | `(bottom-y)/(bottom-top)` | top=패널 상단, bottom=도크 하단. 클램프는 `set_dock_ratio` 내부(타 구간) | `win.rs:8385`~8390 |

### 2-3. 히트 존(`win.rs:1286`~1342)

- 도크 밴드(표시 중·h>0)의 `y ≥ band.y` 안: 싱글 정보 = 전부 `SharedDock`(위젯 소유=좌, 활성 유지). 듀얼 정보 = `x < band.right()`→좌, `x ≥ 우도크.x`→우, 사이=`Split`.
- 밴드 밖/도크 숨김: `x < 좌패널.right()`→좌, `x ≥ 우패널.x`→우, 사이=`Split`.
- 좌클릭·우클릭·더블클릭·휠 전부 같은 존을 쓴다(헤더 우클릭 판정만 `panel_at(x)` — `win.rs:8171`, 셸 메뉴 판정도 `panel_at(x)` — `win.rs:8272`).

### 2-4. 팝업 메뉴 구성(항목 순서 그대로 재현)

**탭 우클릭 메뉴**(`win.rs:7143`~7163): 좌상단 정렬, 커서 위치.

| 순서 | 항목(i18n 키) | 활성 조건 |
|---|---|---|
| 1 | `tab.lock` / `tab.unlock`(잠금 상태에 따라) | 항상 |
| 2 | `tab.pin` / `tab.unpin` | 항상 |
| 3 | `tab.duplicate` | 항상 |
| 4 | `tab.new`(활성 탭 경로 복제 — Ctrl+T·[+]와 동일 경로) | 항상 |
| 5 | `tab.close` | `!locked && 탭 수 > 1` |

**텍스트 편집 메뉴**(`win.rs:7522`~7549): 구분선 위치 포함.

| 대상 | 항목 |
|---|---|
| 경로바 편집 필드 / 리네임 필드 | 실행 취소 ― 잘라내기 · 복사 · 붙여넣기 · 삭제 ― 전체 선택 |
| 도크 Info/Preview 텍스트 | 복사 ― 전체 선택 |
| 도크 터미널 | 복사 · 붙여넣기 ― 전체 선택 |

**도구모음 빈 영역/컬럼 헤더 우클릭 팝업**(`show_bar_popup`, `win.rs:6264`~): 본문은 다른 구간 담당. 이 구간은 호출 조건만 규정(WINC-084).

### 2-5. 커서 모양

| 영역 | 커서 | 근거 |
|---|---|---|
| 도크 상단 가로 분리선 | ↕ | `win.rs:7833`~7837 |
| 도크 좌우 스플리터·파일 스플리터·컬럼 경계 리사이즈 존(드래그 중 포함) | ↔ | `win.rs:7838`~7842 |
| 그 외 | OS 기본 | `win.rs:7853` |

### 2-6. 단축키 전수(이 구간 정의분)

| 키 | 문맥 | 동작 | 근거 |
|---|---|---|---|
| Enter | 경로바 편집 | 제출 | `win.rs:8707` |
| Esc | 경로바 편집 | 제안 팝업 닫기 → 편집 취소 | `win.rs:8710` |
| ↑/↓ | 경로바 편집 | 제안 이동 | `win.rs:8717` |
| Ctrl+C/X/V/Z | 경로바 편집·리네임 | 복사/잘라내기/붙여넣기/실행 취소 | `win.rs:8721`, 8784 |
| ←/→/Home/End/Delete, Ctrl+A | 경로바 편집·리네임 | 캐럿 이동·삭제·전체 선택(Shift=선택 확장) | `win.rs:7636` |
| Enter / Esc | 리네임 | 확정 / 취소 | `win.rs:8774`, 8782 |
| (아무 키) | 종료된 터미널 포커스 | 재시작 | `win.rs:8747` |
| ↑↓←→ Home End Del PgUp PgDn | 터미널 포커스 | VT 시퀀스 전송 | `win.rs:6132` |
| Ctrl+C / Ctrl+V | 터미널 포커스 | 선택 있으면 복사·없으면 인터럽트 / 붙여넣기 | `win.rs:9354`~9361 |
| Esc | 목록 | 탭 드래그 취소 → 컬럼 드래그 취소 → 전송 취소 → 메뉴 닫기 | `win.rs:8793`~8812 |
| F5 | 목록 | 새로 고침 | `win.rs:8813` |
| F6 | 목록 | 테마 순환 | `win.rs:8816` |
| F3 / Shift+F3 | 목록 | 독립 미리보기 창 / 개발 벤치 | `win.rs:8825` |
| Tab / Ctrl+Tab | 목록 | 패널 전환 / 다음 탭 | `win.rs:8834` |
| Ctrl+T / Ctrl+W | 목록 | 새 탭 / 탭 닫기 | `win.rs:8841`, 8843 |
| Ctrl+A / C / X / V | 목록 | 전체 선택 / 복사 / 잘라내기 / 붙여넣기 | `win.rs:8846`~8857 |
| Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y | 목록 | 실행 취소 / 다시 실행 / 다시 실행 | `win.rs:8858`~8863 |
| Enter | 목록 | 캐럿 행 열기 | `win.rs:8864` |
| F2 | 목록 | 이름 바꾸기 | `win.rs:8870` |
| Delete / Shift+Delete | 목록 | 휴지통 / 완전 삭제 | `win.rs:8873` |
| Apps 키, Shift+F10 | 목록 | 캐럿 행 컨텍스트 메뉴 | `win.rs:8876`, 8910 |
| Ctrl+Shift+N | 목록 | 새 폴더 | `win.rs:8879` |
| Ctrl+Shift+R | 목록 | 일괄 이름변경 | `win.rs:8882` |
| Ctrl+H | 목록 | 숨김 파일 토글 | `win.rs:8885` |
| Ctrl+. | 목록 | 닷파일 토글 | `win.rs:8888` |
| Ctrl+, | 목록 | 설정 창 | `win.rs:8891` |
| Ctrl+` | 목록 | 하단 도크 토글 | `win.rs:8894` |
| ↑↓ PgUp PgDn Home End ←→ Space | 목록 | 탐색·선택 토글(Shift/Ctrl 수식) | `win.rs:7649`, 8897 |
| Alt+← / → / ↑ / ↓ | 목록 | 뒤로 / 앞으로 / 상위 / 캐럿 행 열기 | `win.rs:8913`~8929 |
| 마우스 X1 / X2 | 어디서나 | 뒤로 / 앞으로 | `win.rs:8596` |
| Shift+휠 | 목록·도크·터미널 | 가로 스크롤 | `win.rs:7863`, 7910, 7920 |
| Alt(드래그 중) | 스플리터 | 자석 스냅 해제 | `win.rs:6151` |

탭 순서: dir2 메인 창에는 포커스 체인(Tab 순회) 개념이 없다 — Tab은 패널 전환이고, 키 포커스 영역은 "활성 패널 목록 / 경로바 편집 / 인라인 리네임 / 도크 터미널" 4종을 `KeyRoute`와 편집 상태 플래그로 가른다(`win.rs:8706`, 8743, 8772).

## 3. nexa-ui 매핑

근거는 `nexa-ui/crates/nexa-ctl/src` Grep 결과. "없음"은 해당 이름·기능의 `pub` 항목이 Grep에 잡히지 않았다는 뜻이다.

| dir2 요소(이 구간에서 사용) | nexa-ui 대응 | 상태 | 필요한 작업 |
|---|---|---|---|
| `InputEvent`/`Key`(nexa-gui) | `InputEvent`/`Key` — `nexa-ui/crates/nexa-ctl/src/event.rs:49`, 12 | 있음(차이 있음) | 필드명이 `ctrl` → **`primary`**(주 수식키). nexa-ui에는 `Undo`/`Redo`/`Key::Enter/Escape/Delete`가 추가돼 있음. **더블클릭·가운데 버튼·X 버튼 이벤트는 없음** → 호스트가 검출해 위젯 메서드 호출 |
| `WheelAccum`·`WHEEL_DELTA` | `event.rs:120`, 8 | 있음 | 그대로 |
| `nexa_gui::wheel_lines()` 전역 | 없음(Grep `wheel_lines` 무결과) | **추가 필요** | 전역 `wheel_lines()/set_wheel_lines()` 또는 호스트 보유 |
| `Invalidations`(+`request_tick`/`tick_requested`) | `Invalidations` — `widget.rs:14`(`push`/`is_empty`/`drain`만) | 부분 | 틱 요청 API가 없음 → nexa-ui 관례(각 컨트롤 `tick(now) -> bool`)로 호스트가 집계하거나 `request_tick` 추가 |
| `EditKey`·편집 모델 | `EditKey`/`EditState` — `edit.rs:17`, 34 | 있음 | 변형 6종 동일(Left/Right/Home/End/SelectAll/DeleteForward) |
| 메뉴바(`st.menubar`: `is_open`/`on_event`/`take_command`/`set_checked`/`close`/`set_metrics`) | `MenuBar` — `controls/pulldown.rs:84`(`is_open` 147, `take_picked`→`Option<String>` 152, `set_menus` 132, `dismiss` 381) | 있음(API 차이) | 명령 id가 `u32` → **문자열 id**. `set_checked` 없음 → `set_menus` 재구성 또는 체크 갱신 API 추가(`MenuEntry` 체크 필드 유무는 미확인 — 추정) |
| 도구모음·런처 바(`set_buttons`/`set_checked`/`is_button_at`/`take_command`) | `Toolbar`/`ToolItem` — `controls/toolbar.rs:230`, 93(`take_clicked` 298, `item_rect` 373, `set_item_tone` 445, `set_item_enabled` 489, `set_item_icon` 423, `paint_tooltip` 506) | 있음(API 차이) | 항목 전체 교체(`set_items`)·체크 상태(톤으로 대체 가능한지 확인 필요)·`is_button_at`(→`item_rect` 순회로 구현 가능) 추가. 런처 바는 셸 아이콘(RGBA) 버튼 필요 — `ToolIcon`의 이미지 변형 지원 여부 미확인(추정) |
| 툴팁(`tip_*`, TIMER_TIP) | `Toolbar::paint_tooltip`(툴바 내장) | 부분 | 범용 툴팁 위젯은 없음. 툴바 툴팁은 내장 것 사용, 250ms×2틱 지연 규약 대조 필요 |
| 탭 바(`tabbar.pressed_tab/dragging/begin_drag/cancel_drag/tab_index_at/empty_area_at`, 잠금·고정) | `TabBar` — `controls/tabbar.rs:111`(`pressed_tab` 488, `dragging` 477, `begin_drag` 493, `cancel_drag` 482, `tab_index_at` 446, `empty_area_at` 461, `set_locked` 244, `set_pinned` 328, `middle_down` 466, `take_action` 434) | **있음** | 패널 간 이양에 필요한 API가 전부 존재. `take_tab_menu`(우클릭 탭 인덱스)는 `TabAction`에 있는지 확인 필요(추정) |
| 탭 우클릭 메뉴·도구모음/헤더 팝업(네이티브 `TrackPopupMenuEx`) | `ContextMenu`/`CtxItem` — `controls/ctxmenu.rs:272`, 101(`open_at` 528, `on_event` 812, `take_picked` 1134, `CtxItem::maybe` 155, `submenu` 173, `with_checked` 259, `with_shortcut` 233) | **있음** | 모달이 아닌 **오버레이 + 픽 폴링** 구조 → "State 참조 끊고 재획득" 규약이 불필요해지지만, 메뉴가 열린 동안 다른 입력 차단·바깥 클릭 닫기(`is_outside_click` 505)를 호스트가 처리 |
| 텍스트 편집 메뉴(`show_edit_popup`) | `EditMenu`/`EditMenuCaps`/`EditMenuAction` — `controls/editmenu.rs:70`, 42, 55 | 부분 | 표준 항목이 복사·잘라내기·붙여넣기·전체 선택 **4종뿐**이고 순서가 "복사 → 잘라내기 → 붙여넣기 ― 전체 선택". dir2는 **실행 취소·삭제** 포함 + "실행 취소 ― 잘라내기·복사·붙여넣기·삭제 ― 전체 선택" 순. → `extra`로 끼우거나(앞에만 붙음) `EditMenu`에 Undo/Delete·순서 옵션 추가. 도크/터미널용 축약 구성도 필요(`read_only` 캡으로 일부 대응) |
| 스플리터(파일·도크 좌우·도크 상단; 자석 스냅, Alt 해제) | `Splitter`/`SplitAxis`/`SplitEvent` — `controls/splitter.rs:40` | 있음(스냅 없음) | `SplitEvent::Drag(v)`를 받아 호스트가 스냅·클램프(`snap_split_x` 로직 이식). hover 페이드는 nexa-ui 쪽이 추가 제공 |
| 고속 스크롤 전역 설정 | `FastScroll`·`set_fast_scroll` — `controls/scroll.rs:56`, 103; `SpeedHud` 119; `ScrollAccel` 258; `ScrollBars::set_fast_override` 358 | 있음(차이 있음) | `hud_pos`가 `u8`(0~8) → **`HudPos` 열거**(`typeahead.rs:180`) 변환 필요. `set_fast_scroll_grid`/`grid_extra_of`는 없음 → `set_fast_override`로 대체 |
| 타입어헤드 옵션 | `TypeAhead`/`TypeAheadFilter`/`HudPos` — `typeahead.rs:33`, 147, 180 | 있음 | 범위(`FindScope`)·reset_ms·특수문자/공백/백스페이스 옵션 대응 여부는 세부 확인 필요(추정) |
| 파일 목록 행 위젯(`rows()`: 인라인 리네임·러버밴드·마커·컬럼 헤더 드래그 재배열/리사이즈/auto-fit·`abort_press`·`select_program`) | `TreeGrid`/`GridColumn`/`TreeModel` — `controls/tree.rs:675`, 645, 98(`set_column_width` 748, `set_marked_paths` 730) | **대부분 없음** | Grep에서 `begin_rename`·`autofit`·`col_reorder`·러버밴드 무결과. → dir2 `nexa-gui`의 가상 목록(Tree/Flat/Tiles 3모드) 위젯을 **nexa-ui에 신규 추가**하는 것이 최대 작업. 필요한 API: `row_at`, `marker_hit`, `rename_field_hit`, `begin_rename/submit_rename/cancel_rename/rename_key/rename_cut/rename_paste/rename_undo/rename_delete/rename_selected_text/rename_menu_state/rename_hit`, `is_renaming`, `caret`, `typeahead_text`, `tick`, `header_area`, `resize_hot`, `autofit_col_at`, `autofit_texts`, `set_col_width`, `cancel_col_drag`, `abort_press`, `in_body`, `bounds`, `select_program`, `columns` |
| 경로바(`pathbar`: 브레드크럼 + 편집 + 자동완성 팝업) | 없음(Grep `PathBar`/`Breadcrumb` 무결과). 재료: `TextBox`(`controls/textbox.rs:350`), `ContextMenu` | **추가 필요** | API: `is_editing`, `edit_text`, `edit_hit`, `edit_char`, `edit_key`, `edit_cut/paste/delete/undo/selected_text`, `edit_menu_state`, `submit_edit`, `cancel_edit`, `set_suggestions`, `suggest_open/close_suggest/suggest_move/suggest_click`, `bounds` |
| 상태바(`statusbar.set_metrics`, 표시 전용) | 없음(Grep `StatusBar` 무결과) | **추가 필요** | 세그먼트 텍스트 표시 위젯(작음) |
| 도크(Info/Preview/Terminal 스트립: `active_kind`, `content_hit`, `content_rect`, `text_selectable`, `selected_text`, `select_all_text`, `clear_text_selection`, `take_goto`, `take_popout`, `set_focused`, `tick`) | `ToolDock`/`DockLayout`(`controls/tooldock.rs:171`, 66)은 **툴바 도킹용**으로 용도가 다름(추정) | **추가 필요** | dir2 도크 위젯을 nexa-ui로 이식 |
| 터미널 뷰(그리드 그리기·선택·스크롤백·TUI 마우스) | 없음 | **추가 필요** | `nexa-term`(화면 모델)은 플랫폼 중립 crate로 재사용, 그리기·입력은 nexa-ui `DrawCtx` 위에 신규 |
| 텍스트 측정(`DwCtx.select_font`/`text_width`, `FontSlot::List`) | `DrawCtx`/`FontSlot` — `draw.rs:31`, 16; `RasterCtx` — `raster.rs:60` | 있음 | `FontSlot`에 List/Status/Base 슬롯 대응이 있는지 확인(추정) |
| 설정 창·순서 편집 창·일괄 이름변경·About·미리보기 창(모달 Win32 창) | `nexa-dlg`(`FilePicker` — `nexa-ui/crates/nexa-dlg/src/lib.rs:154`)는 파일 선택 전용 | 창 자체는 앱 몫 | nexa-sql 방식(`*_win.rs` — winit 보조 창 + nexa-ui 컨트롤)으로 구현. `SendMessage` 포인터 전달(WINC-127/128)은 **채널/콜백**으로 교체 |
| 환경변수 확장(`pathinput::expand_env`) | `nexa_fs::path::expand_env` — `nexa-ui/crates/nexa-fs/src/lib.rs:610` | 있음 | 문법 차이(%VAR% vs $VAR) 대응 여부 확인 |
| 셸 아이콘 캐시(`icons`, HICON) | `IconService`/`RgbaIcon` — `nexa-ui/crates/nexa-fs/src/shell.rs:108`, 12 | 있음 | RGBA 기반이라 HICON 파괴 분기(WINC-129) 불필요 |
| 폴더 감시(watcher+프로브) | `StatWatch` — `nexa-ui/crates/nexa-fs/src/watch.rs:134`(stat 기반) | 부분 | 디렉터리 변경 통지(OS별)는 §4 |

## 4. OS 분기점

| # | Windows 현 구현 | macOS 대응 | Linux 대응 | 관련 ID |
|---|---|---|---|---|
| 1 | `wndproc` 메시지 펌프·`SetWindowLongPtrW(GWLP_USERDATA)` State | winit `ApplicationHandler`(nexa-sql `app/event_loop.rs` 방식) — State는 앱 구조체 필드. 재진입(`state_of` 재획득) 규약 자체가 사라짐 | 동일 | WINC-058, 167 |
| 2 | 워커→UI `PostMessageW(WM_APP_*)` + `Box::into_raw` 페이로드 | `EventLoopProxy::send_event(Wake)` + `mpsc` 채널(페이로드는 채널로 — 원시 포인터·누수 위험 제거). `post_final_notify`의 재시도는 채널이 무손실이라 불필요하나 **"종결 통지는 반드시 도달"** 불변식은 유지 | 동일 | WINC-035, 124~140 |
| 3 | `SetTimer`/`KillTimer` 16종 | `ControlFlow::WaitUntil` 기반 앱 내 타이머 스케줄러(nexa-sql `event_loop.rs:499` 방식). id·주기 표는 §5 그대로 | 동일 | WINC-147~161 |
| 4 | `SetCapture`/`ReleaseCapture`/`WM_CAPTURECHANGED` | winit은 버튼 누른 동안 암묵 캡처. 캡처 강탈은 `WindowEvent::Focused(false)`·`CursorLeft`에서 `reset_mouse_transients` 호출로 대체 | X11: 암묵 그랩 동일. Wayland: 창 밖 좌표가 안 올 수 있음 → 포커스 상실 시 정리 | WINC-069, 099, 105 |
| 5 | `WM_SETCURSOR` + `LoadCursorW(IDC_SIZENS/SIZEWE)` | `Window::set_cursor(CursorIcon::RowResize/ColResize)` — `CursorMoved`마다 판정(nexa-sql `event_loop.rs:728`~743) | 동일 | WINC-063 |
| 6 | `WM_MOUSEWHEEL`(화면 좌표, delta 120 단위)·`WM_MOUSEHWHEEL`·`MK_SHIFT` | `MouseWheel{LineDelta/PixelDelta}` → `WHEEL_DELTA` 정규화. 좌표는 마지막 `CursorMoved`. **macOS는 Shift+휠을 OS가 이미 가로 delta로 바꿔 준다** → 이중 변환 금지. 트랙패드 `PixelDelta`는 dir2의 "정밀 터치패드=픽셀" 경로에 직결 | X11/Wayland `LineDelta` 중심, 터치패드는 `PixelDelta` | WINC-064~068 |
| 7 | `WM_XBUTTONDOWN` | `MouseButton::Back/Forward`(지원 마우스 한정) + 트랙패드 스와이프는 미지원(추정) | `MouseButton::Back/Forward` | WINC-106 |
| 8 | `WM_LBUTTONDBLCLK` + `GetDoubleClickTime()` | winit에 더블클릭 이벤트 없음 → 호스트가 (시간, 거리)로 검출. 시간 = `NSEvent.doubleClickInterval`(기본 0.5s) | GTK 설정 `gtk-double-click-time`(기본 400ms) 또는 고정 500ms 폴백 | WINC-104, 107~111, 158 |
| 9 | 드래그 임계 `GetSystemMetrics(SM_CXDRAG/SM_CYDRAG)`(최소 4) | 고정 4 논리 px × 배율 | 동일 | WINC-093 |
| 10 | `GetCaretBlinkTime()`(0/MAX→530ms) | `NSTextInsertionPointBlinkPeriodOn/Off` 사용자 기본값, 없으면 530ms | `gtk-cursor-blink-time`(기본 1200ms 주기) 또는 530ms 폴백 | WINC-081, 156 |
| 11 | `GetKeyState(VK_SHIFT/CONTROL/MENU)`·VK 코드 | `ModifiersChanged` 상태 보관 + `KeyEvent.logical_key/physical_key`. **주 수식키 = ⌘**(nexa-ui `primary`). Ctrl+, → ⌘, 등 | Ctrl 그대로 | WINC-055, 056, 112~123 |
| 12 | 키 관례: Alt+←/→/↑/↓, Delete=휴지통, F2=리네임, Apps/Shift+F10=메뉴 | ⌘[ / ⌘] / ⌘↑ / ⌘↓(열기), ⌘⌫=휴지통(Delete 키 병행 허용), Enter=리네임은 Finder 관례지만 **dir2 기조 유지 → F2 + Enter=열기**, Apps 키 없음(Ctrl+클릭=우클릭). F5/F6/F3은 fn 조합 필요 → 대체 단축키 병기 | Windows와 동일(Menu 키 존재) | WINC-117~123 |
| 13 | `WM_CHAR`(IME 결과 포함)·`WM_IME_*`·`position_ime` | `WindowEvent::Ime(Preedit/Commit)` + `set_ime_cursor_area`(nexa-sql `event_loop.rs:755`~773). 조합 중 프리에디트 표시는 nexa-ui 편집 필드의 `set_preedit` | 동일(IBus/Fcitx는 winit 경유). nexa-ui `hangul::Composer`(`hangul.rs:117`)는 IME 없는 환경 폴백 | WINC-143~146 |
| 14 | 터미널: ConPTY + pwsh. `cd "<dir>"\r` 전송, Backspace 0x08↔0x7F **교차 매핑**(ConPTY가 0x08을 Ctrl+BS로 해석) | PTY(`openpty`/`forkpty`) + `$SHELL`(없으면 `/bin/zsh`). `cd` 인자는 **POSIX 작은따옴표 이스케이프**(`'…'`, 내부 `'`는 `'\''`). Backspace는 `0x7F` 그대로, Ctrl+BS는 `0x17`(^W) 또는 `0x08` — **교차 매핑 금지** | `$SHELL`(없으면 `/bin/sh`), 나머지 동일 | WINC-080, 114, 144 |
| 15 | 터미널 Ctrl+C=선택 있으면 복사·없으면 SIGINT, Ctrl+V=붙여넣기(Windows Terminal 규약) | ⌘C/⌘V=복사/붙여넣기, **Ctrl+C는 항상 인터럽트**(Terminal.app 규약) | Ctrl+Shift+C/V=복사/붙여넣기, Ctrl+C=인터럽트(GNOME Terminal 규약). 붙여넣기 줄바꿈 `\n`→`\r` 변환은 공통 | WINC-045, 050, 144 |
| 16 | 텍스트 클립보드(`clipboard::read_text/write_text/has_text`) | `pbpaste`/`pbcopy`(nexa-sql `clipboard.rs:210`~213) | X11 selection 직접 → `wl-paste`/`xclip`/`xsel` 폴백(nexa-sql `clipboard.rs:215`~225, 228~234) | WINC-043, 044, 054 |
| 17 | rich 클립보드(평문 + CF_HTML + RTF) | `osascript`로 `«class HTML»` 동시 게시(nexa-sql `clipboard.rs:246`~262). RTF는 `public.rtf` — 네이티브 `NSPasteboard`(objc2) 권장 | 텍스트만(CLI 도구는 다중 형식 불가 — nexa-sql `clipboard.rs:26`). 다중 타깃(`text/html`, `text/rtf`)은 X11 selection 직접 구현 시 가능 | WINC-046, 049 |
| 18 | 파일 클립보드 `CF_HDROP` + `Preferred DropEffect`(복사/이동 구분), 잘라내기는 붙여넣기 후 `clear` | `NSPasteboard`의 `public.file-url`(다중). **잘라내기 개념이 없음** → 앱 내부 "이동 예약" 플래그로 보관(Finder의 ⌥⌘V 대응) | `x-special/gnome-copied-files`(`copy\n` 또는 `cut\n` + `file://` URI 목록) + `text/uri-list`; KDE는 `application/x-kde-cutselection` 추가 | WINC-047 |
| 19 | 가상 파일 붙여넣기(FileGroupDescriptor/FileContents, `paste_virtual`, `WM_APP_VPASTE`) | 해당 없음 → **비활성**(파일 프라미스 `NSFilePromiseReceiver`는 후순위 — 추정) | 해당 없음 → 비활성 | WINC-047, 137 |
| 20 | `WM_CLIPBOARDUPDATE`(`AddClipboardFormatListener`) → 잘라내기 흐림 동기 | 통지 없음 → `NSPasteboard.changeCount` 폴링(창 활성 시 + 1s 틱) | X11 `XFixesSelectSelectionInput` / Wayland data-device 이벤트, 폴백 = 활성 시 폴링 | WINC-165, 166 |
| 21 | 파일 실행 `ShellExecuteW("open")` + `AllowSetForegroundWindow` | `open <path>`(또는 `NSWorkspace.open`) — 포그라운드 양도는 OS가 처리(불필요) | `xdg-open <path>`(폴백 `gio open`). 실행 권한 있는 파일을 직접 실행할지는 정책 결정 필요 | WINC-036~038, 138 |
| 22 | 경고음 `MessageBeep(MB_ICONWARNING)` | `NSBeep()` | 없음(생략) 또는 `\a` — 타이틀 문구만으로 충분 | WINC-036 |
| 23 | 셸 컨텍스트 메뉴 `IContextMenu` + 전용 메뉴 스레드·선행 구축·`WM_INITMENUPOPUP` 포워딩 | **자체 그리기 `ContextMenu`**: 열기 / 연결 프로그램(`LSCopyApplicationURLsForURL`) / Finder에서 보기(`open -R`) / 정보 가져오기 / 휴지통 / 복사·잘라내기·붙여넣기 / 경로 복사 / 이름 복사 / 압축. 선행 구축·메뉴 스레드 불필요 | 자체 그리기 `ContextMenu`: 열기 / 연결 프로그램(`.desktop` + `mimeapps.list`, `gio mime`) / 파일 관리자에서 보기(`nexa_fs::shell::reveal_in_file_manager` — `nexa-ui/crates/nexa-fs/src/shell.rs:856`) / 휴지통 / 클립보드 / 경로 복사. **Windows에서도 dir2 커스텀 항목(CTX_* 7종, `win.rs:2804`~2812)은 공통 메뉴로**, 셸 확장 항목만 W | WINC-086, 089, 090, 134, 152 |
| 24 | OLE DnD 발신 `DoDragDrop`(모달, MouseUp 소비) / 수신 `RegisterDragDrop` | winit은 **파일 드롭 수신만**(`DroppedFile`/`HoveredFile`). 패널↔패널 드래그는 앱 내부 구현(N). 외부로 끌어내기는 `NSDraggingSource`(objc2) 필요 | 수신 = winit. 외부 발신 = XDND / `wl_data_device` 직접 구현 필요(후순위). nexa-sql `drop_win.rs` 참고 | WINC-093, 159, 166 |
| 25 | `WM_ACTIVATEAPP`(활성 3s / 비활성 30s 폴링) | `WindowEvent::Focused(bool)` | 동일 | WINC-060 |
| 26 | `WM_SIZE`+`SIZE_MINIMIZED`(트림·폴링 정지) | `WindowEvent::Occluded(true)` 또는 `Window::is_minimized()` | 동일(Wayland는 최소화 판정 불가 → `Occluded`만) | WINC-062 |
| 27 | `SetProcessWorkingSetSize(-1,-1)` 작업집합 반납 | 대응 없음 → 캐시 해제만(백버퍼·아이콘 캐시) | `malloc_trim(0)`(glibc) 선택적 | WINC-161 |
| 28 | `WM_DEVICECHANGE`(드라이브 도착/제거) | `NSWorkspace` `didMount`/`didUnmount` 통지 또는 `/Volumes` 감시 | `/proc/self/mountinfo` poll(POLLPRI) 또는 FSPOLL 틱에서 마운트 목록 비교 | WINC-061 |
| 29 | `WM_DPICHANGED` + 제안 rect `SetWindowPos` | `ScaleFactorChanged`(창 크기는 winit이 조정) + **프로그램 이동 뒤 배율 재확인**(nexa-sql `event_loop.rs:652` 교훈) | 동일 | WINC-162 |
| 30 | `WM_SETTINGCHANGE`(테마·휠 줄 수) | `ThemeChanged`. 휠 줄 수는 고정 3(OS가 delta에 이미 반영) | `ThemeChanged`(XDG portal `color-scheme`), 휠 줄 수 고정 3 | WINC-163 |
| 31 | 셸 변경 통지 `SHChangeNotifyRegister`(OneDrive 플레이스홀더) | FSEvents가 클라우드 변경도 통지 → watcher로 흡수 | inotify + FSPOLL 보험 | WINC-136 |
| 32 | UI Automation(`WM_GETOBJECT`, `WM_APP_UIA_SELECT`) | 1차: **비활성**(Windows 전용 유지). 후속: AccessKit(크로스플랫폼) 검토 | 동일 | WINC-141, 142 |
| 33 | 경로 비교: 끝 구분자 `\`·`/` 제거 + **ASCII 대소문자 무시** | APFS 기본은 대소문자 무시 → 유지 가능(대소문자 구분 볼륨 예외) | **대소문자 구분** → `eq_ignore_ascii_case` 사용 금지. 플랫폼별 `path_eq` 헬퍼로 분리 | WINC-158, 168 |
| 34 | 환경변수 문법 `%VAR%` | `$VAR`/`${VAR}`/`~` | 동일 | WINC-040 |
| 35 | 타이틀바 다크(DWM)·항상 위 | `Window::set_theme` / `set_window_level(AlwaysOnTop)` | 동일 | WINC-058 |

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 설정 스냅샷 필드 전수(`current_settings`, `win.rs:6947`~7018) — 61개

dir3은 nexa-sql 설정 레지스트리(nsql-settings) 구조로 옮기되 **아래 키·기본 의미·클램프 범위는 전부 계승**한다.

| 그룹 | 필드 | 클램프/값 도메인(이 구간에서 확인된 것) |
|---|---|---|
| 외관 | `theme`("system"/"light"/"dark"), `lang`("system" 또는 언어 코드), `always_on_top` | `win.rs:6480`~6492 |
| 보기 | `view_mode`("tree"/"flat"/"tiles"), `panel_mode`("single"/그 외 듀얼), `info_mode`("single"/그 외 듀얼), `view_scope`("tab" 외 값은 타 구간), `show_hidden`, `show_dotfiles`, `sort_folders_first`, `sort_case_sensitive`, `hide_empty_glyph` | `win.rs:6785`, 6775, 6781 |
| 글꼴 | `base_font`/`base_font_size`, `ctx_font`/`ctx_font_size`, `status_font`/`status_font_size`, `list_font`/`list_font_size`, `dlg_font`/`dlg_font_size`, `term_font`/`term_font_size` | 크기 8~32(터미널·대화상자는 이 구간에서 클램프 없음) |
| 목록 장식 | `list_folder_bold`, `header_bold`, `header_italic` | bool |
| 컬럼 | `col_width_sync`, `col_autofit_max` | 50~2000 |
| 배치 | `split`, `dock`, `dock_ratio`, `dock_split` | split 0.1~0.9, dock_split 0.15~0.85 |
| 탐색 | `nav_up_align`("top"/"bottom"/그 외 center), `tab_dblclick`("pin"/"lock"/그 외 close) | |
| 타입어헤드 | `typeahead_scope`("global"/"level"/그 외 visible), `typeahead_reset_ms`, `typeahead_pos`, `typeahead_special`, `typeahead_space`, `typeahead_backspace` | reset 200~10000, pos 0~8 |
| 고속 스크롤 | `fast_scroll`, `fast_scroll_step`, `fast_scroll_max`, `fast_scroll_window_ms`, `fast_scroll_hud`, `fast_scroll_hud_pos`, `fast_scroll_hud_hold_ms`, `fast_scroll_hud_fade_ms`, `fast_scroll_grid_extra` | step 1~50, max 1~32, window 20~2000, pos 0~8, hold/fade 0~10000 |
| 터미널 | `term_wrap`, `term_cols`, `term_theme`, `term_theme_dark`, `term_theme_light`, `term_copy_format`("html"/"rtf"/"both"/그 외 평문) | cols 80~1000 |
| 전송·DnD | `transfer_close_ms`, `dnd_hover_ms` | 0~10000, 200~10000 |
| 순서 편집 | `toolbar_order`, `ctx_menu_order` | 문자열(직렬화 형식은 config 구간) |
| 미리보기·플러그인 | `preview_map`, `plugins_disabled` | |
| 런처 | `launcher`(표시), `launcher_items`(Some), `launcher_seed`(= `launcher::SEED_VERSION`) | |
| 클라우드 | `cloud_conns`, `cloud_client_ids`, `cloud_client_secrets` | 비밀 값 — dir3에서는 vault 분리 검토(추정) |

### 5-2. 세션(`current_session`, `win.rs:6805`~6835)

`Session { active_panel, panels: [PanelSession; 2] }`, `PanelSession { tabs, active, expanded, locked, pinned, modes, views, col_widths, col_layout }`. 컬럼 레이아웃 문자열 형식 = `cols[name:1,ext:1,…,kind:0]`(`win.rs:6891`~6907).

저장 시점:
- 설정: `apply_prefs` 말미(`win.rs:6758`), `persist_settings`(메뉴 토글·순서 편집 즉시 — `win.rs:9022`), 종료(`win.rs:9677`).
- 세션: `TIMER_SESSION_SAVE` 디바운스 1000ms(`win.rs:9422`), 종료(`win.rs:9678`).
- 종료 시 설정·세션 **둘 다 성공해야** 구 형식 정리(`purge_legacy`) — `win.rs:9680`~9685.
- 복원: 컬럼은 `pending_cols`(레이아웃) → `pending_colw`(폭) 순, DPI 메트릭 설정 **이후**(`win.rs:7707`~7718).

### 5-3. 타이머 표(`win.rs:66`~132)

| id | 이름 | 주기 | 무장 조건 → 해제 조건 |
|---|---|---|---|
| 1 | `TIMER_TYPEAHEAD` | 250ms | 타입어헤드 버퍼 비어 있지 않음 → 양 패널 버퍼 빔 |
| 2 | `TIMER_ICONS` | `icons::shell::TICK_MS`(주석상 80ms — 추정) | 아이콘 로딩 대기 있음 → 대기 없음 |
| 3 | `TIMER_JANITOR` | 10s | 창 생성·입력 활동 → 트림 실행 또는 최소화 |
| 4 | `TIMER_RENAME` | 더블클릭 시간 | 느린 재클릭 확정 → 1회 만료/더블클릭/새 입력 |
| 5 | `TIMER_TERM_SEL` | 60ms | 터미널 선택 드래그가 그리드 상/하 밖 → 안으로 복귀/버튼 해제/캡처 상실 |
| 6 | `TIMER_TERM_CARET` | 캐럿 깜빡임 주기(폴백 530ms) | 터미널 키 포커스 → 포커스 해제 |
| 7 | `TIMER_PROG_CLOSE` | `transfer_close_ms` | 전송 완료 → 1회 |
| 8 | `TIMER_SESSION_SAVE` | 1000ms | 탭/경로 변경 → 1회 |
| 9 | `TIMER_TIP` | 250ms(2틱 후 표시) | 툴바 hover → 이탈 |
| 10, 11 | `TIMER_WATCH_BASE+패널` | 300ms(연장 상한 1000ms) | watcher/셸/드라이브 통지 → 1회(편집·전송·삭제 중이면 재무장) |
| 12 | `TIMER_DND` | 100ms | 드래그 추적 중 |
| 13 | `TIMER_CLOUD_POLL` | 200ms | 클라우드 전송 중 → `cloud_shared` 없음 |
| 14 | `TIMER_FSPOLL` | 활성 3s / 비활성 30s | 창 생성 → 최소화 시 정지 |
| 15 | `TIMER_WIDGET_TICK` | 40ms | 위젯 틱 요청 → 요청 없음 |
| 16 | `TIMER_CTX_PREBUILD` | 300ms | 선택이 머무름 → 1회 |

### 5-4. 워커 → UI 통지 표(`win.rs:71`~184 + 이 구간 핸들러)

| 메시지 | wparam | lparam | 소유권 | 세대 가드 |
|---|---|---|---|---|
| `WM_APP_TRANSFER` 0x8001 | 세대 | 0=진행/1=완료 | — | 있음(`on_transfer_message`) |
| `WM_APP_FSCHANGE` 0x8002 | 패널 | 세대 | — | watcher 세대 일치 |
| `WM_APP_TERM` 0x8003 | 패널 + `EXIT_FLAG` | 세대 | 출력은 공유 `Mutex` 버퍼 take | `pty.gen` 일치 |
| `WM_APP_ICON` 0x8004 | `Box<LoadResult>` | — | 수신 측 항상 해제 | — |
| `WM_APP_PREFS` 0x8005 / `EDIT_TOOLBAR` 0x800A / `EDIT_COLS` 0x800B / `BULK` 0x8008 / `CTLDEMO` 0x8009 / `ABOUT` 0x800C | — | — | — | UI 스레드 자기 게시(모달 지연) |
| `prefs::WM_APP_PREFS_APPLY`(0x8006 — 주석) | — | `*const PrefValues` | **SendMessage 동안만 유효** → 즉시 clone | — |
| `uia::WM_APP_UIA_SELECT`(0x8007 — 주석) | 행 인덱스 | SEL_SINGLE/ADD/REMOVE | — | 범위 검사 |
| `ordereditor::WM_APP_ORDER_EDIT` | field(1=툴바, 2=컬럼) | `*const String` | 즉시 clone | — |
| `WM_APP_DELETE` 0x800D | 1=성공 | — | 대상은 `pending_delete` | — |
| `WM_APP_CLOUD_AUTH` 0x800E | — | `Box<CloudAuthResult>` | 수신 회수 | — |
| `WM_APP_CLOUD_LIST` 0x800F | — | `Box<ListResult>` | 수신 회수 | — |
| `WM_APP_CLOUD_DOWNLOAD` 0x8010 | — | `Box<DownloadResult>` | 수신 회수 | — |
| `WM_APP_CLOUD_WRITE` 0x8011 | — | `Box<WriteResult>` | 수신 회수 | — |
| `WM_APP_CLOUD_PROGRESS` 0x8012 | — | — | 단발(유실 허용) | — |
| `WM_APP_VPASTE` 0x8013 | 전건 성공 여부 | — | — | — |
| `WM_APP_SHCHANGE_BASE` 0x8014/0x8015 | 셸 페이로드 | 셸 페이로드 | `release_payload`로 해제 | — |
| `WM_APP_INFO_DETAILS` 0x8016 | 레인 | `Box<(gen, PathBuf, Vec<DetailLine>)>` | 수신 회수 | 세대·경로 일치 |
| `WM_APP_CTXMENU_RESULT` 0x8017 | 세대 | `Box<(MenuReq, Outcome)>` | 수신 회수 | `ctx_gen` 일치 |

스레드: UI 스레드 1 + 전용 메뉴 스레드(`menuthread`) + 도크 Info 상세 워커(`fileinfo::DetailWorker`) + 전송·삭제·클라우드·아이콘·watcher·PTY 리더 워커. 공유 Mutex는 전부 `plock`(poison 내성)으로 잠근다.

### 5-5. 이 구간이 읽고 쓰는 과도 상태(State 필드)

`drag_press`, `rename_on_up`, `slow_click`, `pending_rename`, `tab_drag_undo`, `split_drag`, `dock_split_drag`, `dock_drag`, `term_focus`, `term_caret_on`, `term_drag`, `term_mouse_btn`, `rbutton_down_seen`, `rclick_began_edit`, `ctx_showing`, `ctx_gen`, `info_req[2]`, `info_details[2]`, `pending_cols[2]`, `pending_colw[2]`, `trimmed`, `last_activity_ms`, `watch_since[2]`, `transfer_close`, `cloud_shared`, `vpaste_roots`. dir3에서는 "마우스 과도 상태"를 한 구조체로 묶어 `reset_mouse_transients` 한 곳에서 초기화할 것.

## 6. 이식 시 주의 — 회귀 방지에 필요한 실측 교훈

1. **입력 후 동기 길목은 `finish_input` 하나**(flush → 타이틀 → 상태). 폴더 진입·선택 이동을 일으키는 모든 경로(더블클릭, Enter, Alt+방향키, X 버튼, 경로바 제출, 타입어헤드, 새 탭)가 이 함수를 지나야 한다. 빠뜨리면 상태바·도크 Info가 이전 폴더를 보이고 watcher 기준선이 낡는다(`win.rs:5086`~5094, 8601, 8656, 8685).
2. **MouseUp에서의 동기는 선택 시그니처가 바뀐 경우에만**(`selection_sig`) — 무조건 돌리면 클릭당 도크 재생성 2회(`win.rs:5096`~5105, 8551~8562).
3. **공유 도크 클릭은 활성 패널을 바꾸지 않는다**. 위젯 소유 패널과 포커스 줄 패널을 같은 값으로 다루면 싱글 정보에서 우 패널 미리보기가 좌 패널 선택으로 뒤바뀐다(`win.rs:1281`~1285, 8045~8051, 8201~8204).
4. **터미널 포커스 중에는 PTY가 없어도 키·문자를 삼킨다**(`TermPending`). 목록으로 새면 Delete가 보이지 않는 캐럿 행을 휴지통으로 보낸다(`win.rs:6079`~6082, 9340~9341).
5. **숨은(0-rect) 도크의 터미널이 키 포커스를 가져가면 안 된다** — `dock_shown()`(실제 표시·h>0) 조건 필수(`win.rs:8100`~8107).
6. **터미널이 포커스/편집 메뉴 대상이 되는 순간 경로바 편집·리네임을 정리**(`cancel_text_edits`). 안 하면 터미널에서 고른 붙여넣기가 경로바·리네임 필드에 들어간다(`win.rs:7234`~7238, 8110~8112, 7606~7613).
7. **느린 재클릭 리네임**: 예약은 MouseDown, 진입은 MouseUp 뒤 더블클릭 시간만큼 지연. 새 클릭·우클릭·키 입력·명령·드래그 시작·더블클릭이 전부 예약을 폐기한다. 만료 시 (패널, 경로)가 **지금의** 활성 패널·캐럿 행과 일치할 때만 진입(`win.rs:8132`~8150, 8573~8581, 9583~9602, 3997~3998).
8. **리네임을 끝낸 그 클릭은 새 리네임을 예약하지 않는다**(`!was_renaming`). 단 리네임 중 **다른 행** 클릭은 정상 행 클릭으로 취급해 `slow_click` 시드를 남긴다 — 히트 가드는 "필드 안 클릭"만(`win.rs:8052`~8056, 8138~8140).
9. **편집 필드 안 더블클릭은 열기가 아니라 전체 선택**. 가드가 없으면 편집 중인 그 행이 실행/진입된다(`win.rs:8621`~8647).
10. **드래그 발신 후에는 양 패널 목록의 눌림 보류를 반드시 소거**(`abort_press`). 드래그가 MouseUp을 소비하므로 낡은 보류가 다음 클릭에서 재로드 후 다른 파일을 단일 선택한다(`win.rs:8363`~8369). dir3의 자체 DnD에서도 같은 불변식 유지.
11. **드래그 취소 시 양 패널 재열거 금지** — 실제 변경은 watcher·폴링이 반영(`win.rs:8374`~8375).
12. **캡처(또는 그에 준하는 입력 독점)가 끊기면 모든 마우스 과도 상태를 일괄 정리**. 하나라도 남으면: 터미널 선택 타이머가 계속 돌거나, 다음 이동에서 의도치 않은 드래그 발신, 모든 MouseMove 조기 반환(`win.rs:5994`~6017).
13. **TUI 버튼 유지 판정은 버튼별**(`tui_btn_held`). 우버튼 코드만 보고 유지로 판단하면 창의 모든 마우스 이동이 죽는다. 좌·우 동시 누름에서 좌를 뗄 때 우 캡처를 풀면 안 된다(`win.rs:6019`~6029, 8583~8589).
14. **짝 없는 우클릭 뗌은 메뉴를 열지 않는다**(`rbutton_down_seen`) — 누름을 모달/다른 창이 흡수한 경우 선택 규약이 반영되지 않은 선택에 메뉴가 뜬다(`win.rs:8237`~8241).
15. **우클릭이 경로바 편집을 시작시킨 경우 그 뗌에서는 편집 메뉴를 띄우지 않는다**(`rclick_began_edit`)(`win.rs:8208`~8209, 8251~8253).
16. **패널 간 탭 드래그 미리 보기는 값 채택을 미룬다**(`adopt:false`) → Esc 복귀가 무손실. 커밋(해제) 때 채택(`win.rs:8431`~8435, 8514~8550).
17. **탭 드래그 커밋 판정은 위젯에 MouseUp을 넘기기 전에** — 위젯이 MouseUp에서 드래그 상태를 지운다(`win.rs:8514`~8515).
18. **메뉴가 열려 있을 때 패널에 MouseMove를 넘기지 않는다**(드롭다운 아래 hover 잔상)(`win.rs:8422`~8423).
19. **세션 컬럼 복원은 DPI 메트릭 설정 이후, 레이아웃 → 폭 순서**. `col_width_sync`가 켜져 있으면 시작 시 좌 기준으로 맞춘다(`win.rs:7707`~7726).
20. **시작 시 클라우드 탭은 비활성 탭까지 재열기** — 세션 복원 탭이 열거 콜백 등록 전에 만들어져 비어 있다(`win.rs:7676`~7702). 목록 적재 완료 시에도 활성 탭만이 아니라 전 탭 갱신(`win.rs:9114`~9117).
21. **watcher 디바운스 만료 시 편집·전송·삭제 중이면 미루고 재무장**, 재로드 후에는 프로브 기준선 재설정(이중 계기 진동 차단)(`win.rs:9458`~9482).
22. **비활성 창도 폴링을 끄지 않고 감속**(30s), 정지는 최소화에서만(`win.rs:7760`~7764, 7795). 최소화 복원 판정은 `trimmed` 플래그를 `note_activity` **전에** 읽는다(`win.rs:7797`~7802).
23. **리사이즈 이벤트마다 폴더를 재열거하지 않는다** — 최소화 복원만 갱신 계기(`win.rs:7797`~7799).
24. **종결 통지는 유실되면 상태가 영구 고착**(`pending_delete`·`st.transfer`) → dir3 통지 경로는 무손실 채널이어야 한다(`win.rs:7041`~7047).
25. **공유 Mutex는 poison 내성으로 잠근다** — 디버그에서 워커 panic이 UI 연쇄 panic으로 번지지 않게(`win.rs:7031`~7036).
26. **스크롤백 보기 중 새 출력이 오면 보던 위치를 고정**(늘어난 줄 수만큼 오프셋 증가)(`win.rs:8965`~8969).
27. **터미널 복사 후 선택 해제, 입력 시 스크롤백·선택 해제, 입력 중 캐럿 상시 표시**(`win.rs:7427`, 9369~9370, 9376).
28. **잘라내기는 1회성** — 붙여넣기 시작 시 클립보드를 비운다(`win.rs:7369`~7371).
29. **아이콘·터미널·도크 갱신은 해당 rect만 무효화**. 전창 무효화는 로딩 중 80ms마다 전체 재도장을 유발한다(`win.rs:9073`, 9538~9539, 8971~8978).
30. **설정 창 체크박스(숨김·닷파일·폴더 우선)는 적용 범위와 무관하게 전체 일괄** — 값의 SSOT는 탭이고 비활성 탭은 전환 시 수렴(`win.rs:6540`~6542, 6632).
31. **메뉴 체크 갱신에 쓰는 무효화 수집기를 버리지 않는다**(재도장 힌트 유실)(`win.rs:6546`~6548).
32. **휠 라우팅은 활성 패널이 아니라 커서 아래 패널**. 도크 텍스트 위 휠은 양 패널의 실제 도크 rect로 판정(싱글 정보·h=0 도크 오판 방지)(`win.rs:7858`, 7905~7908).
33. **연속 우클릭 지연 대책(선행 구축·닫힌 직후 재구축)은 셸 메뉴 구축 비용(400~700ms) 때문** — 자체 그리기 메뉴에서는 불필요하나, Windows에서 셸 확장 항목을 유지한다면 그대로 필요(`win.rs:9156`~9158).
34. 참고(코드 정리 대상): `WM_APP_CTXMENU_RESULT` 핸들러 위에 셸 변경 통지 설명 주석이 잘못 붙어 있다(`win.rs:9141`~9143 주석은 9185 핸들러의 것). `WM_APP_CLOUD_LIST`/`WRITE`의 `let _ = panel_shows_cloud(...)`는 결과를 쓰지 않는 사문(`win.rs:9116`, 9259). Ctrl+V가 실패(붙여넣을 것 없음)하면 `finish_input`으로 낙하한다(`win.rs:8854`~8857) — 동작은 무해.

## 7. 회귀 테스트 후보

자동화 표기: **U**=순수 단위 테스트(즉시 가능) / **H**=헤드리스 하네스(합성 `InputEvent` 주입 + 상태 단언, 창 없이) / **M**=수동 또는 OS 통합 필요.

| # | 시나리오 | 기대 | 자동화 | 관련 ID |
|---|---|---|---|---|
| 1 | 기존 7개 단위 테스트 이식(WINC-168~174) | 전부 통과. 경로 리터럴은 플랫폼별 변형, 대소문자 무시 단언은 Windows/macOS 한정 | U | WINC-168~174 |
| 2 | 설정 문자열 변환 표(`scope_of`/`mode_of`/`align_of`, 미지 값 폴백) | 표대로 | U | WINC-026 |
| 3 | 설정 클램프 표(§5-1 범위 전수: 경계값 ±1) | 범위 밖 입력이 경계로 수렴 | U | WINC-002, 008, 012, 020~023 |
| 4 | 컬럼 레이아웃 왕복(`panel_col_layout` ↔ `apply_col_layout_str`), 숨김 컬럼 말미 `:0`, 미지 키 무시 | 왕복 불변 | U | WINC-030 |
| 5 | 컬럼 폭/순서 동기: 한 패널 리사이즈 → 같은 패널 전 탭 상속, sync on이면 반대 패널 동일, off면 불변 | 폭 벡터 일치 | H | WINC-031 |
| 6 | 설정 스냅샷 왕복: `current_settings` → 직렬화 → 로드 → 61개 필드 전부 동일 | 동일 | U | WINC-032 |
| 7 | 세션 왕복: 탭·활성·펼침·잠금·고정·모드·컬럼 | 동일 | U | WINC-028 |
| 8 | 편집 동작 디스패치 우선순위: 경로바 편집 중 Paste → 경로바 / 리네임 중 → 리네임 필드 / 터미널 포커스 → PTY / 도크 텍스트 선택 + Copy → 도크 텍스트 / 그 외 → 파일 목록 | 대상별 정확히 1곳 | H(클립보드 가짜 구현 주입) | WINC-043~047 |
| 9 | 터미널 우클릭 메뉴에서 붙여넣기(경로바 편집이 살아 있는 상태) | 편집이 취소되고 PTY에만 전달 | H | WINC-042, 053, 088 |
| 10 | `paste_line`: 여러 줄·제어 문자 포함 텍스트 | 첫 줄만, 제어 문자 제거, 빈 결과=None | U | WINC-054 |
| 11 | 터미널 붙여넣기 줄바꿈 변환(`\r\n`·`\n` → `\r`) | 변환 결과 일치 | U | WINC-050 |
| 12 | 터미널 문자 매핑: Windows에서 BS↔DEL 교차, Unix에서 교차 없음 | OS별 기대 바이트 | U(cfg 분기) | WINC-144 |
| 13 | 터미널 `cd` 인자 인용: 공백·작은따옴표·`$` 포함 경로 | 셸별로 안전한 인용 | U | WINC-080 |
| 14 | 느린 재클릭 리네임: 기선택 행 1s 후 재클릭 → 더블클릭 시간 후 리네임 진입. 변형: 그 사이 키 입력/우클릭/다른 행 클릭/드래그/더블클릭 → 진입 안 함 | 각 변형별 진입/불발 | H(시각 주입) | WINC-082, 104, 107, 112, 158 |
| 15 | 리네임 필드 안 더블클릭 | 전체 선택, 행이 실행되지 않음 | H | WINC-107 |
| 16 | 좌클릭 라우팅 순서(§2-1의 8단계 각 1건) | 각 영역이 자기 핸들러만 탐 | H | WINC-070~077 |
| 17 | 스플리터 자석 스냅: 50% ± 임계, 반대편 구분선 ± 임계, Alt 유지 시 해제, 도크 숨김 시 반대편 스냅 없음 | 표대로 | U(`snap_split_x`를 순수 함수로 분리) | WINC-095, 096 |
| 18 | 스플리터 비율 클램프(0.1~0.9, 0.15~0.85) | 경계 수렴 | U | WINC-095, 096 |
| 19 | 패널 간 탭 드래그: 반대 탭 바 위 미리 보기 → Esc → 원 패널 원위치(값 무손실) / 해제 → 커밋(값 채택) / 본문 위 해제 → 끝 삽입 | 탭 목록·활성 일치 | H | WINC-098, 101, 116 |
| 20 | Esc 우선순위 사슬(탭 드래그 > 컬럼 드래그 > 전송 취소 > 메뉴 닫기) | 한 번에 하나만 소비 | H | WINC-116 |
| 21 | 입력 독점 상실(포커스 아웃) 중 스플리터 드래그·터미널 선택·TUI 버튼·탭 드래그 | 전부 정리, 타이머 0 | H | WINC-099 |
| 22 | 우클릭: 짝 없는 뗌=메뉴 없음 / 편집 시작 우클릭=메뉴 억제 / 행 위=행 메뉴 / 빈 본문=배경 메뉴 / 비활성 패널=없음 | 표대로 | H | WINC-086~089 |
| 23 | 탭 우클릭 메뉴 항목·활성 조건(잠금 탭·탭 1개에서 닫기 비활성) | §2-4 표 | H(메뉴 항목 검사) | WINC-039 |
| 24 | 편집 메뉴 항목·활성 조건(대상 4종 × 선택 유무 × 클립보드 유무) | §2-4 표 | H | WINC-053 |
| 25 | 휠 라우팅: 비활성 패널 위 휠=그 패널 스크롤 / 도크 텍스트 위=도크 / 터미널 위=스크롤백 / TUI 모드=버튼 64·65 / Shift=가로 | 대상·방향 일치 | H | WINC-064~068 |
| 26 | 터미널 스크롤백 보기 중 출력 도착 | 화면 고정(오프셋 = 이전 + 증가분) | U | WINC-125 |
| 27 | watcher 디바운스: 리네임 중 만료 → 재무장, 편집 종료 후 재로드 1회 | 재로드 횟수 1 | H(시각 주입) | WINC-151 |
| 28 | 전송 중 대상 실행·드래그 차단 | 실행 안 됨, 드래그 목록에서 제외 | H | WINC-033, 036, 093 |
| 29 | 탭 더블클릭 설정 3종(close/pin/lock) + 빈 영역=새 탭 | 표대로 | H | WINC-108, 109 |
| 30 | 컬럼 auto-fit: 가시 행 최대 폭, 상한 `col_autofit_max` 적용, 하한 min_width | 폭 일치 | H(가짜 `DrawCtx`) | WINC-029, 110 |
| 31 | 키 단축키 표(§2-6) 전수 — 명령 id 발화 검사, macOS는 ⌘ 변형 | 표대로 | H | WINC-113~123 |
| 32 | 종료 저장: 설정 저장 실패 시 구 형식 정리 미실행 | 정리 호출 0 | U | WINC-164 |
| 33 | 유휴 트림: 60s 무입력 → 트림 1회·타이머 정지, 입력 → 재가동 | 상태 전이 | U/H | WINC-057, 161 |
| 34 | 파일 실행(기본 연결 프로그램)·휴지통·파일 클립보드·외부 DnD·드라이브 도착 | OS별 실제 동작 | M | WINC-038, 047, 061, 093 |
| 35 | IME 조합(한글) — 경로바·리네임·타입어헤드·터미널 | 조합 창 위치·확정 문자 전달 | M | WINC-143~146 |
| 36 | DPI/배율 변경, 모니터 간 이동 | 메트릭·컬럼·스플리터 재계산 | M | WINC-162 |
