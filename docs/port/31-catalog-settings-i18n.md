# 31. 전수 카탈로그 — 설정 키 · i18n 문자열 · 영속 파일 · 환경/인자/레지스트리 · nexa-sql 방식 키 대응표

> 대상: nexa-dir2 0.22.0 (`D:/Projects/kiros33/nexa-dir2/crates`) 전체 횡단 조사 · 기준: nexa-sql `nsql-settings` / nexa-ui `nexa-conf`.
> 조사 방식: Grep 전수(설정 접근 · `tr(`/`trf(` · `env::` · `Reg*` · 파일명) → `config.rs`(1689줄) · `i18n.rs`(289줄) · `en.lang`(543줄) · `ko.lang`(542줄) 전문 통독 → 그 밖 접근 지점 구간 통독.
> 표기: 근거는 `저장소/경로:줄`. 확인하지 못한 것은 **추정**이라고 적는다. 이 문서는 읽기 전용 조사 산출물이며 소스는 건드리지 않았다.

## 0. ID 체계와 개수 요약

| ID 범위 | 내용 | 개수 |
|---|---|---|
| KEY-001 ~ KEY-071 | `settings.cfg` 키(현행 69 + 구 키 1 + 머리 주석 1) | 71 |
| KEY-101 ~ KEY-112 | `session.cfg` 키 | 12 |
| KEY-121 ~ KEY-140 | 값 안에 들어 있는 하위 문법 토큰(도구모음 · 컨텍스트 메뉴 · 컬럼 · 가상 경로 · 터미널 스킴) | 20 |
| KEY-141 ~ KEY-150 | 이름 변경 프리셋(`renames/*.cfg`) 필드 | 10 |
| KEY-201 ~ KEY-222 | 영속 · 임시 파일 | 22 |
| KEY-301 ~ KEY-345 | 환경 변수 · 명령행 인자 · 레지스트리 · OS 설정 조회 · 내장 상수 | 45 |
| KEY-501 ~ KEY-571 | nexa-sql 방식 대응표(KEY-5nn ↔ KEY-0nn 1:1) | 71 |
| KEY-581 ~ KEY-590 | 세션 파일 대응 | 10 |
| KEY-591 ~ KEY-600 | dir2에 없어 새로 필요한 키(후보) | 10 |
| KEY-601 ~ KEY-615 | nexa-sql 설정 구조 규칙(차용 계약) | 15 |
| KEY-1001 ~ KEY-1498 | i18n 문자열 키(번호 = 1000 + `en.lang` 등장 순서) | 498 |
| KEY-1501 ~ KEY-1506 | `.lang` 메타 키 | 6 |
| KEY-1601 ~ KEY-1627 | i18n 접두 분류 | 27 |

핵심 사실 8줄:

1. dir2 설정은 **단일 구조체** `Settings`(68 필드)이고 파일 키는 69종이다(`nexa-dir2/crates/nexa-app/src/config.rs:65-209` · 직렬화 `:432-563` · 파싱 `:566-833`). 키는 전부 **1레벨 snake_case**(`term_font_size`)다.
2. 설정 창에 등록된 항목은 46개 `Entry`(`nexa-dir2/crates/nexa-app/src/prefs.rs:445-779`)이고, 나머지 키는 메뉴 · 도구모음 · 드래그 · 파일 직접 편집으로만 바뀐다.
3. 세션은 별도 파일 `session.cfg`(패널 2개 · 탭 · 펼침 집합 · 컬럼)이고 **창 위치 · 크기는 영속하지 않는다**(`GetWindowPlacement`/`SetWindowPlacement` Grep 0건).
4. 목록 구분자 `|`는 "Windows 경로에 등장 불가 문자"라는 전제(`nexa-dir2/crates/nexa-app/src/config.rs:837`)에 서 있다 — macOS · Linux에서는 `|`가 파일명에 올 수 있어 **그대로 옮기면 깨진다**.
5. i18n은 `.lang`(properties) 파일 3개(en · ko · ja) 내장 + `data\lang\*.lang` 사용자 덮어쓰기(`nexa-dir2/crates/nexa-app/src/i18n.rs:14-26` · `:97-151`). 키 498개 · 세 언어 키 일치(테스트 `:217-232`).
6. `del.lockedMsg` · `del.failMsg`는 `.lang`에서 값이 여러 줄로 적혀 있어 **`{1}`(파일 이름 목록) 줄이 파서에서 버려진다**(§2-3 · 현행 결함).
7. 레지스트리는 **읽기 2곳뿐**(앱 테마 · OneDrive 계정) · 쓰기 0. 환경 변수는 전부 Windows 이름(`USERPROFILE` · `LOCALAPPDATA` · `SystemRoot` · `ComSpec` …)이다.
8. nexa-sql 방식은 **2레벨 `<접두>.<이름>` · 레지스트리 단일 원천 · 기본값과 다른 값만 저장 · 모르는 키 보존 · `_schema` 키**다(§5-1).

---

## 1. 설정 키 전수

### 1-1. `settings.cfg` (현행 69키 + 구 키 + 머리 주석)

- 파일 형식: UTF-8 · 한 줄 = `key=value` · `#` 주석 · 첫 `=`에서 분리 · 줄 전체 trim(`nexa-dir2/crates/nexa-app/src/config.rs:421-429`).
- 불리언은 `0`/`1`로 쓰고 읽을 때 `v != "0"`이면 참이다(예 `nexa-dir2/crates/nexa-app/src/config.rs:573`).
- 관용 파싱: 손상 · 미지 키 무시, 범위 밖 숫자는 클램프, 같은 키가 여러 번이면 마지막 유효 값(`nexa-dir2/crates/nexa-app/src/config.rs:565-566` · 시험 `:1223-1280`).
- **전 키를 항상 쓴다**(기본값과 같아도 기록). 예외 = `preview_map` · `plugins_disabled`(빈 값이면 생략 `:462-467`) · `cloud_client_*`(빈 값 생략 `:538-548`) · `launcher*`(`None`이면 생략 `:549`).
- 저장 시점: 변경 즉시(`persist_settings` `nexa-dir2/crates/nexa-app/src/win.rs:6942-6945` · 호출 지점 `:5190` `:5208` `:5254` `:5298` `:6759`) + 종료 시(`:9674-9685`).
- 스냅숏 조립: `current_settings` (`nexa-dir2/crates/nexa-app/src/win.rs:6947-7018`).
- 기본값 근거는 전부 `impl Default for Settings`(`nexa-dir2/crates/nexa-app/src/config.rs:251-324`)이다.

| ID | 키 | 타입 · 값 | 기본값 | 검증 · 클램프 | 파싱 근거 | 바꾸는 곳(UI) |
|---|---|---|---|---|---|---|
| KEY-001 | `theme` | 열거 `system`\|`light`\|`dark` | `dark` | 3값만 허용 | `nexa-dir2/crates/nexa-app/src/config.rs:570` | 설정 창 appearance · Radio(`nexa-dir2/crates/nexa-app/src/prefs.rs:454-460`) + 보기 메뉴 |
| KEY-002 | `lang` | 문자열 `system`\|언어 코드 | `system` | 비어 있지 않음 · ≤16바이트 · 코드 검증은 `resolve_code` | `nexa-dir2/crates/nexa-app/src/config.rs:572` | 설정 창 lang · LangRadio(`nexa-dir2/crates/nexa-app/src/prefs.rs:771-777`) + 보기 메뉴 |
| KEY-003 | `show_hidden` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:573` | 설정 창 list · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:524-530`) + 메뉴 + 도구모음 `hidden` |
| KEY-004 | `show_dotfiles` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:574` | 설정 창 list · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:531-537`) + 메뉴 + 도구모음 `dot` |
| KEY-005 | `split` | f32(좌 패널 폭 비율 · 소수 3자리) | `0.500` | 0.1 ~ 0.9 · 유한수 | `nexa-dir2/crates/nexa-app/src/config.rs:604-610` | 분할선 드래그(설정 창 없음) |
| KEY-006 | `dock` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:575` | 설정 창 dock · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:764-770`) + 메뉴 + 도구모음 `dock` |
| KEY-007 | `dock_ratio` | f32(도크 높이 비율) | `0.300` | 0.15 ~ 0.5 | `nexa-dir2/crates/nexa-app/src/config.rs:590-596` | 분할선 드래그 |
| KEY-008 | `dock_split` | f32(도크 좌/우 분할) | `0.500` | 0.15 ~ 0.85 | `nexa-dir2/crates/nexa-app/src/config.rs:597-603` | 분할선 드래그 |
| KEY-009 | `term_font` | 문자열(쉼표 목록 = 폴백 체인) | `Consolas` | 비어 있지 않음 · ≤128바이트 | `nexa-dir2/crates/nexa-app/src/config.rs:576-578` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:468-474`) |
| KEY-010 | `term_font_size` | i32(DIP) | `12` | 8 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:579-583` | 위 Font 행의 크기 칸 |
| KEY-011 | `dlg_font` | 문자열 | `Segoe UI` | 비어 있지 않음 · ≤64바이트 | `nexa-dir2/crates/nexa-app/src/config.rs:584` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:517-523`) |
| KEY-012 | `dlg_font_size` | i32(**pt**) | `9` | 7 ~ 24 | `nexa-dir2/crates/nexa-app/src/config.rs:585-589` | 위 Font 행의 크기 칸 |
| KEY-013 | `launcher` | bool(퀵 런처 바 표시) | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:758` | 보기 메뉴(설정 창 없음) |
| KEY-014 | `term_wrap` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:638` | 설정 창 terminal · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:721-727`) |
| KEY-015 | `term_cols` | i32(고정 열 수 · `term_wrap=0`일 때만 의미) | `240` | 80 ~ 1000 | `nexa-dir2/crates/nexa-app/src/config.rs:639-643` | 설정 창 terminal · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:728-734`) |
| KEY-016 | `term_theme` | 문자열 `system`\|`dark`\|`light`\|스킴 id | `system` | 비어 있지 않음 · ≤64 · 모르는 id는 해석 시 `system` | `nexa-dir2/crates/nexa-app/src/config.rs:645-647` | 설정 창 terminal · Select(`nexa-dir2/crates/nexa-app/src/prefs.rs:736-742`) |
| KEY-017 | `term_theme_dark` | 스킴 id | `campbell`(`nexa-dir2/crates/nexa-term/src/lib.rs:147`) | ≤64 | `nexa-dir2/crates/nexa-app/src/config.rs:648-650` | 설정 창 terminal · Select(`nexa-dir2/crates/nexa-app/src/prefs.rs:743-749`) |
| KEY-018 | `term_theme_light` | 스킴 id | `github-light`(`nexa-dir2/crates/nexa-term/src/lib.rs:149`) | ≤64 | `nexa-dir2/crates/nexa-app/src/config.rs:651-653` | 설정 창 terminal · Select(`nexa-dir2/crates/nexa-app/src/prefs.rs:750-756`) |
| KEY-019 | `term_copy_format` | 열거 `text`\|`html`\|`rtf`\|`both` | `text` | 4값만 | `nexa-dir2/crates/nexa-app/src/config.rs:654-656` | 설정 창 terminal · Select(`nexa-dir2/crates/nexa-app/src/prefs.rs:757-763`) |
| KEY-020 | `transfer_close_ms` | i32(ms · 0 = 진행 창 미표시) | `2000` | 0 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:657-661` | 설정 창 transfer · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:559-565`) |
| KEY-021 | `dnd_hover_ms` | i32(ms) | `3000` | 200 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:662-666` | 설정 창 transfer · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:566-572`) |
| KEY-022 | `preview_map` | 문자열 `ext:플러그인ID\|…` | 빈 값(줄 생략) | ≤512바이트 | `nexa-dir2/crates/nexa-app/src/config.rs:706` · 사용 `nexa-dir2/crates/nexa-app/src/preview/mod.rs:209-238` | **UI 없음** — 파일 직접 편집(`PrefValues`에 필드 없음 `nexa-dir2/crates/nexa-app/src/prefs.rs:51-126`) |
| KEY-023 | `plugins_disabled` | 문자열 `id\|…` | 빈 값(줄 생략) | ≤512바이트 · `builtin.*`은 면역 | `nexa-dir2/crates/nexa-app/src/config.rs:708` · 판정 `nexa-dir2/crates/nexa-app/src/preview/mod.rs:204-206` | 설정 창 plugins 페이지 체크(`nexa-dir2/crates/nexa-app/src/prefs.rs:124-125`) |
| KEY-024 | `sort_folders_first` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:673` | 설정 창 list · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:573-579`) + 도구모음 `foldersfirst` |
| KEY-025 | `sort_case_sensitive` | bool | `0` | — | `nexa-dir2/crates/nexa-app/src/config.rs:679` | 설정 창 list · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:587-593`) |
| KEY-026 | `nav_up_align` | 열거 `top`\|`center`\|`bottom` | `center` | 3값만 | `nexa-dir2/crates/nexa-app/src/config.rs:680-682` | 설정 창 list · Radio(`nexa-dir2/crates/nexa-app/src/prefs.rs:601-607`) |
| KEY-027 | `tab_dblclick` | 열거 `close`\|`pin`\|`lock` | `close` | 3값만 | `nexa-dir2/crates/nexa-app/src/config.rs:702-704` | 설정 창 tabs · Radio(`nexa-dir2/crates/nexa-app/src/prefs.rs:608-614`) |
| KEY-028 | `view_mode` | 열거 `tree`\|`flat`\|`tiles`(새 탭 기본 = 마지막 선택) | `tree` | 3값만 | `nexa-dir2/crates/nexa-app/src/config.rs:697-699` | 보기 메뉴 + 도구모음 `view[…]`(`nexa-dir2/crates/nexa-app/src/win.rs:5181-5191`) |
| KEY-029 | `panel_mode` | 열거 `single`\|`dual` | `dual` | 2값만 | `nexa-dir2/crates/nexa-app/src/config.rs:700` | 보기 메뉴 + 도구모음 `toggle` |
| KEY-030 | `info_mode` | 열거 `single`\|`dual`(싱글 패널에선 싱글 강제 · 값은 보존) | `dual` | 2값만 | `nexa-dir2/crates/nexa-app/src/config.rs:701` | 보기 메뉴 + 도구모음 `info` |
| KEY-031 | `view_scope` | 열거 `global`\|`panel`\|`tab` | `panel` | 3값만 | `nexa-dir2/crates/nexa-app/src/config.rs:674-676` | 설정 창 list · Radio(`nexa-dir2/crates/nexa-app/src/prefs.rs:594-600`) |
| KEY-032 | `hide_empty_glyph` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:677` | 설정 창 list · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:580-586`) |
| KEY-033 | `always_on_top` | bool | `0` | — | `nexa-dir2/crates/nexa-app/src/config.rs:678` | 보기 메뉴 + 도구모음 `ontop` |
| KEY-034 | `col_width_sync` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:683` | 보기 메뉴 + 도구모음 `colsync`(`nexa-dir2/crates/nexa-app/src/win.rs:5193-5209`) |
| KEY-035 | `col_autofit_max` | i32(px @96dpi) | `400` | 50 ~ 2000 | `nexa-dir2/crates/nexa-app/src/config.rs:684-688` | 설정 창 list · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:538-544`) |
| KEY-036 | `toolbar_order` | 순서 문법 문자열(§1-3) | `refresh:1\|panel:1[toggle:1,dock:1,info:1,colsync:1,ontop:1]\|view:1[tree:1,flat:1,tiles:1]\|show:1[hidden:1,dot:1,foldersfirst:1]\|settings:1`(`TOOLBAR_BLOCKS`에서 파생 `nexa-dir2/crates/nexa-app/src/config.rs:1036-1066`) | 읽을 때 재직렬화로 정규화(미지 토큰 제거 · 누락 보충) | `nexa-dir2/crates/nexa-app/src/config.rs:690-692` | 설정 창 appearance · 별도 편집 창(`nexa-dir2/crates/nexa-app/src/prefs.rs:447-453`) |
| KEY-037 | `ctx_menu_order` | 순서 문법 문자열 | `row:1[new:1,deletePermanent:1,copyName:1,pasteInto:1]\|bg:1[paste:1,undo:1,redo:1]`(`CTXMENU_BLOCKS` `nexa-dir2/crates/nexa-app/src/config.rs:1078-1082`) | 위와 같음 | `nexa-dir2/crates/nexa-app/src/config.rs:693-696` | 설정 창 ctxmenu · 별도 편집 창(`nexa-dir2/crates/nexa-app/src/prefs.rs:552-558`) |
| KEY-038 | `typeahead_scope` | 열거 `global`\|`level`\|`visible` | `visible` | 3값만 | `nexa-dir2/crates/nexa-app/src/config.rs:709-711` | 설정 창 typeahead · Radio(`nexa-dir2/crates/nexa-app/src/prefs.rs:615-621`) |
| KEY-039 | `typeahead_reset_ms` | i32(ms) | `1000` | 200 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:712-716` | 설정 창 typeahead · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:622-628`) |
| KEY-040 | `typeahead_pos` | i32 0..8(3×3 행 우선 · 6 = 좌하) | `6` | 0 ~ 8 | `nexa-dir2/crates/nexa-app/src/config.rs:717-721` | 설정 창 typeahead · PosGrid(`nexa-dir2/crates/nexa-app/src/prefs.rs:650-656`) |
| KEY-041 | `typeahead_special` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:722` | 설정 창 typeahead · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:629-635`) |
| KEY-042 | `typeahead_space` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:723` | 설정 창 typeahead · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:636-642`) |
| KEY-043 | `typeahead_backspace` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:724` | 설정 창 typeahead · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:643-649`) |
| KEY-044 | `fast_scroll` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:725` | 설정 창 scroll · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:658-664`) |
| KEY-045 | `fast_scroll_step` | i32(연속 N회마다 배수 +1) | `3` | 1 ~ 50 | `nexa-dir2/crates/nexa-app/src/config.rs:726-730` | 설정 창 scroll · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:672-678`) |
| KEY-046 | `fast_scroll_max` | i32(배수 상한) | `16` | 1 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:731-735` | 설정 창 scroll · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:679-685`) |
| KEY-047 | `fast_scroll_window_ms` | i32(ms) | `160` | 20 ~ 2000 | `nexa-dir2/crates/nexa-app/src/config.rs:736-740` | 설정 창 scroll · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:686-692`) |
| KEY-048 | `fast_scroll_hud` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:741` | 설정 창 scroll · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:693-699`) |
| KEY-049 | `fast_scroll_hud_pos` | i32 0..8(2 = 우상) | `2` | 0 ~ 8 | `nexa-dir2/crates/nexa-app/src/config.rs:743-747` | 설정 창 scroll · PosGrid(`nexa-dir2/crates/nexa-app/src/prefs.rs:700-706`) |
| KEY-050 | `fast_scroll_hud_hold_ms` | i32(ms) | `250` | 0 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:748-752` | 설정 창 scroll · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:707-713`) |
| KEY-051 | `fast_scroll_hud_fade_ms` | i32(ms) | `600` | 0 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:753-757` | 설정 창 scroll · Number(`nexa-dir2/crates/nexa-app/src/prefs.rs:714-720`) |
| KEY-052 | `fast_scroll_grid_extra` | bool | `1` | — | `nexa-dir2/crates/nexa-app/src/config.rs:742` | 설정 창 scroll · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:665-671`) |
| KEY-053 | `base_font` | 문자열 | `Segoe UI` | 비어 있지 않음(길이 상한 없음) | `nexa-dir2/crates/nexa-app/src/config.rs:611` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:461-467`) |
| KEY-054 | `base_font_size` | i32(DIP) | `12` | 8 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:612-616` | 위 Font 행 |
| KEY-055 | `ctx_font` | 문자열 | `Segoe UI` | 비어 있지 않음 | `nexa-dir2/crates/nexa-app/src/config.rs:617` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:475-481`) |
| KEY-056 | `ctx_font_size` | i32(DIP) | `12` | 8 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:618-622` | 위 Font 행 |
| KEY-057 | `status_font` | 문자열 | `Segoe UI` | 비어 있지 않음 | `nexa-dir2/crates/nexa-app/src/config.rs:623` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:482-488`) |
| KEY-058 | `status_font_size` | i32(DIP) | `12` | 8 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:624-628` | 위 Font 행 |
| KEY-059 | `list_font` | 문자열 | `Segoe UI` | 비어 있지 않음 | `nexa-dir2/crates/nexa-app/src/config.rs:629` | 설정 창 fonts · Font 행(`nexa-dir2/crates/nexa-app/src/prefs.rs:489-495`) |
| KEY-060 | `list_font_size` | i32(DIP) | `12` | 8 ~ 32 | `nexa-dir2/crates/nexa-app/src/config.rs:630-634` | 위 Font 행 |
| KEY-061 | `list_folder_bold` | bool | `0` | — | `nexa-dir2/crates/nexa-app/src/config.rs:635` | 설정 창 fonts · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:496-502`) |
| KEY-062 | `header_bold` | bool | `0` | — | `nexa-dir2/crates/nexa-app/src/config.rs:636` | 설정 창 fonts · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:503-509`) |
| KEY-063 | `header_italic` | bool | `0` | — | `nexa-dir2/crates/nexa-app/src/config.rs:637` | 설정 창 fonts · CheckBox(`nexa-dir2/crates/nexa-app/src/prefs.rs:510-516`) |
| KEY-064 | `launcher_seed` | u32(적용된 시드 버전) | `0` · 저장은 늘 `SEED_VERSION`=2(`nexa-dir2/crates/nexa-app/src/launcher.rs:35` · `nexa-dir2/crates/nexa-app/src/win.rs:7013`) | 파싱 실패 = 0 | `nexa-dir2/crates/nexa-app/src/config.rs:759` | 없음(기동 시 시드 판정 `nexa-dir2/crates/nexa-app/src/win.rs:1553-1555`) |
| KEY-065 | `launcher_count` | usize(존재 자체 = "목록 확정" · 값은 안 읽음) | 키 부재 = 첫 실행(시드 주입) | — | `nexa-dir2/crates/nexa-app/src/config.rs:761-765` | 없음 |
| KEY-066 | `launcher<N>` | `라벨\|exe\|인자`(인자 안의 `\|`는 보존 · `-` = 그룹 구분선 · 인자의 `%path%` = 활성 폴더) | 시드 = VS Code · pwsh/PowerShell · cmd(`nexa-dir2/crates/nexa-app/src/launcher.rs:89-108`) | 상한 32 · 라벨 · exe 비면 버림 | `nexa-dir2/crates/nexa-app/src/config.rs:766-790` | **UI 없음** — 파일 직접 편집 |
| KEY-067 | `cloud<N>` | `kind\|라벨\|경로\|account`(kind = `onedrive`\|`googledrive`\|`dropbox` · 경로 빔 = API 직접 연결) | 없음 | 상한 32 · kind · 라벨 필수 · 경로 또는 account 중 하나 필수 · 저장 시 라벨의 `\|`→`/` | `nexa-dir2/crates/nexa-app/src/config.rs:808-828` · 저장 `:528-536` | 클라우드 메뉴(추가 · 해제) |
| KEY-068 | `cloud_client_id_<kind>` | 문자열(OAuth client_id 재정의) | 없음(내장 기본 사용 `nexa-dir2/crates/nexa-app/src/oauth.rs:137-147`) | ≤256 · 비어 있지 않음 | `nexa-dir2/crates/nexa-app/src/config.rs:799-805` | **UI 없음** — 파일 직접 편집(안내문 = KEY-1053) |
| KEY-069 | `cloud_client_secret_<kind>` | 문자열(OAuth client_secret 재정의) | 없음(내장 기본 사용 `nexa-dir2/crates/nexa-app/src/oauth.rs:150-160`) | ≤256 · 비어 있지 않음 | `nexa-dir2/crates/nexa-app/src/config.rs:792-798` | **UI 없음** — 파일 직접 편집 |
| KEY-070 | `transfer_close_secs`(**구 키** · 읽기 전용) | i32(초) → `transfer_close_ms` = ×1000 | — | 결과 0 ~ 10000 | `nexa-dir2/crates/nexa-app/src/config.rs:667-672` | — |
| KEY-071 | 머리 주석 `# nexa-dir settings v1` | 버전 표식(주석이라 파서가 읽지 못함) | — | — | `nexa-dir2/crates/nexa-app/src/config.rs:434` | — |

구조체에만 있고 파일 키가 다른 필드 3종: `launcher_items: Option<Vec<LauncherItem>>`(= KEY-065 · 066), `cloud_conns`(= KEY-067), `cloud_client_ids`/`cloud_client_secrets`(= KEY-068 · 069) — `nexa-dir2/crates/nexa-app/src/config.rs:196-208`.

설정 창 등록 없이 존재하는 라벨: `pref.termFontSize` · `pref.dlgFontSize`(크기가 Font 행에 합쳐져 단독 항목이 없다 — §2-4).

### 1-2. `session.cfg`

- 형식은 `settings.cfg`와 같다. 머리 주석 `# nexa-dir session v1`.
- 저장: 탭 변경 시 1000ms 디바운스(`nexa-dir2/crates/nexa-app/src/win.rs:91` · `:5072-5078` · `:9421-9429`) + 종료 시(`:9674-9685`).
- 복원: 명령행 인자가 있으면 세션을 무시하고 그 경로로 시작한다(`nexa-dir2/crates/nexa-app/src/win.rs:1441-1475`).

| ID | 키 | 타입 · 값 | 기록 조건 | 근거(쓰기 / 읽기) |
|---|---|---|---|---|
| KEY-101 | 머리 주석 `# nexa-dir session v1` | 버전 표식 | 항상 | `nexa-dir2/crates/nexa-app/src/config.rs:839` |
| KEY-102 | `active_panel` | usize 0\|1(읽을 때 `min(1)`) | 항상 | `nexa-dir2/crates/nexa-app/src/config.rs:840` / `:902` |
| KEY-103 | `panel{0,1}.tabs` | 경로 목록 · 구분자 `\|` | 항상 | `nexa-dir2/crates/nexa-app/src/config.rs:847` / `:903-910` |
| KEY-104 | `panel{i}.active` | usize(활성 탭 인덱스) | 항상 | `nexa-dir2/crates/nexa-app/src/config.rs:848` / `:911-914` |
| KEY-105 | `panel{i}.exp{j}` | 탭 j의 펼침 경로 목록 · 구분자 `\|` | 빈 목록은 생략 | `nexa-dir2/crates/nexa-app/src/config.rs:850-858` / `:959-976`(j > 64 무시) |
| KEY-106 | `panel{i}.locked` | 탭별 `0`/`1` · 구분자 `\|` | 하나라도 잠겼을 때만 | `nexa-dir2/crates/nexa-app/src/config.rs:860-867` / `:915-918` |
| KEY-107 | `panel{i}.pinned` | 탭별 `0`/`1` · 구분자 `\|` | 하나라도 고정일 때만 | `nexa-dir2/crates/nexa-app/src/config.rs:869-876` / `:919-922` |
| KEY-108 | `panel{i}.modes` | 탭별 `tree`\|`flat`\|`tiles` · 구분자 `\|` | 전부 tree면 생략 | `nexa-dir2/crates/nexa-app/src/config.rs:878-880` / `:946-958` |
| KEY-109 | `panel{i}.views` | 탭별 u8 비트(bit0 숨김 · bit1 Dot · bit2 폴더 우선 · `& 0x7`) · 구분자 `\|` | 비어 있지 않을 때 | `nexa-dir2/crates/nexa-app/src/config.rs:882-885` / `:923-931` |
| KEY-110 | `panel{i}.cols` | 컬럼 레이아웃 문법 `cols:1[name:1,ext:1,size:1,modified:1,kind:1]` | 비어 있지 않을 때 | `nexa-dir2/crates/nexa-app/src/config.rs:887-889` / `:932-937` · 조립 `nexa-dir2/crates/nexa-app/src/win.rs:6893-6907` |
| KEY-111 | `panel{i}.colw` | 컬럼 폭 px 목록 · 구분자 `,`(표시 컬럼의 표시 순) | 비어 있지 않을 때 | `nexa-dir2/crates/nexa-app/src/config.rs:890-893` / `:938-945` |
| KEY-112 | `panel{i}.colw{j}`(**예약** · 미구현) | 향후 탭별 폭 | — | 주석 `nexa-dir2/crates/nexa-app/src/config.rs:343-344` |

영속하지 **않는** 상태(확인): 창 위치 · 크기(관련 API Grep 0건) · 탭별 정렬 컬럼/방향(세션 키 없음 `nexa-dir2/crates/nexa-app/src/config.rs:326-349`) · 실행 취소 기록(`nexa-dir2/crates/nexa-ops/src/history.rs`에 파일 I/O 없음 — 시험 코드 제외) · 압축 암호(타입 수준 금지 `nexa-dir2/crates/nexa-core/src/secret.rs:1-18`) · 설정 창 크기/선택 카테고리(추정 — `prefs.rs` 상태 구조체 `:781-800`에 영속 경로 없음).

### 1-3. 값 안의 하위 문법 토큰

순서 문법: `블록:vis[자식:vis,…]|블록:vis|…`(단일 블록은 대괄호 생략 · vis 생략 = 표시 · 미지/중복 토큰 버림 · 누락분은 정의 순서 자리에 보충) — `nexa-dir2/crates/nexa-app/src/config.rs:1084-1174`.

| ID | 쓰이는 키 | 토큰 | 뜻 | 근거 |
|---|---|---|---|---|
| KEY-121 | `toolbar_order` | 블록 `refresh` | 새로 고침 버튼 | `nexa-dir2/crates/nexa-app/src/config.rs:1038` |
| KEY-122 | `toolbar_order` | 블록 `panel` + 자식 `toggle` · `dock` · `info` · `colsync` · `ontop` | 패널 제어 그룹 | `nexa-dir2/crates/nexa-app/src/config.rs:1040` |
| KEY-123 | `toolbar_order` | 블록 `view` + 자식 `tree` · `flat` · `tiles` | 보기 모드 그룹 | `nexa-dir2/crates/nexa-app/src/config.rs:1041` |
| KEY-124 | `toolbar_order` | 블록 `show` + 자식 `hidden` · `dot` · `foldersfirst` | 표시 항목 그룹 | `nexa-dir2/crates/nexa-app/src/config.rs:1042` |
| KEY-125 | `toolbar_order` | 블록 `settings` | 설정 버튼 | `nexa-dir2/crates/nexa-app/src/config.rs:1043` |
| KEY-126 | `ctx_menu_order` | 블록 `row` + 자식 `new` · `deletePermanent` · `copyName` · `pasteInto` | 항목 메뉴(앱 고유 항목만) | `nexa-dir2/crates/nexa-app/src/config.rs:1080` |
| KEY-127 | `ctx_menu_order` | 블록 `bg` + 자식 `paste` · `undo` · `redo` | 배경 메뉴 | `nexa-dir2/crates/nexa-app/src/config.rs:1081` |
| KEY-128 | `panel{i}.cols` | 블록 `cols` + 자식 `name` · `ext` · `size` · `modified` · `kind` | 파일 목록 컬럼(name = 상시 표시) | `nexa-dir2/crates/nexa-app/src/config.rs:1075` · id 대응 `nexa-dir2/crates/nexa-app/src/win.rs:6870-6889` |
| KEY-129 | `panel{i}.tabs` · `exp{j}` | 가상 경로 `::PC::` | 내 PC(최상위 가상 폴더) | `nexa-dir2/crates/nexa-vfs/src/lib.rs:106` |
| KEY-130 | `panel{i}.tabs` · `exp{j}` | 가상 경로 `::CLOUD:<idx>::/<경로>` | 클라우드 API 연결(idx = `cloud<N>`의 N) | `nexa-dir2/crates/nexa-vfs/src/lib.rs:176-194` |
| KEY-131 | 경로 입력 | 스킴 `shell:<이름>` | 셸 특수 폴더(세션에 저장되는 형태는 해석 후 실경로 — 추정) | `nexa-dir2/crates/nexa-app/src/shellpath.rs:23` |
| KEY-132 | `term_theme` | 선택자 `system` · `dark` · `light` | 앱 테마 추종 / 모드 기본 강제 | `nexa-dir2/crates/nexa-app/src/prefs.rs:178-186` |
| KEY-133 | `term_theme*` | 어두운 스킴 id 9종: `campbell` · `one-half-dark` · `solarized-dark` · `tango-dark` · `dracula` · `nord` · `gruvbox-dark` · `catppuccin-mocha` · `tokyo-night` | 터미널 색 스킴 | `nexa-dir2/crates/nexa-term/src/lib.rs:153-350` |
| KEY-134 | `term_theme*` | 밝은 스킴 id 6종: `github-light` · `one-half-light` · `solarized-light` · `tango-light` · `gruvbox-light` · `catppuccin-latte` | 터미널 색 스킴 | `nexa-dir2/crates/nexa-term/src/lib.rs:377-491` |
| KEY-135 | `preview_map` · `plugins_disabled` | 내장 공급자 id `builtin.text` · `builtin.image` · 압축 내장(id는 `preview/archive.rs` — 미확인) | 미리보기 공급자 | `nexa-dir2/crates/nexa-app/src/preview/mod.rs:147-200` |
| KEY-136 | `launcher<N>` | 자리표 `%path%` | 활성 패널 현재 폴더 | `nexa-dir2/crates/nexa-app/src/launcher.rs:146-148` |
| KEY-137 | `cloud<N>` | kind `onedrive` · `googledrive` · `dropbox` | 서비스 종류 | `nexa-dir2/crates/nexa-app/src/oauth.rs:54-126` |
| KEY-138 | `typeahead_pos` · `fast_scroll_hud_pos` | 정수 0..8 = 행 우선 3×3(0 좌상 · 2 우상 · 6 좌하 · 8 우하) | 배지 자리 | `nexa-dir2/crates/nexa-app/src/config.rs:152-153` · `:165` |
| KEY-139 | `panel{i}.views` | 비트 1 = 숨김 · 2 = Dot · 4 = 폴더 우선 | 탭별 보기 옵션 | `nexa-dir2/crates/nexa-app/src/config.rs:339-341` |
| KEY-140 | `lang` | 내장 코드 `en` · `ko` · `ja` + `data\lang\<code>.lang` 발견분 | 언어 | `nexa-dir2/crates/nexa-app/src/i18n.rs:19-26` · `:122-151` |

### 1-4. 이름 변경 프리셋 필드(`data\renames\<이름>.cfg`)

- 머리 주석 `# nexa-dir rename preset v2` · 한 줄 = 블록 1개 · 필드 구분자 `|` · 값 안의 `|` · `\`는 역슬래시 이스케이프 · 상한 64블록(`nexa-dir2/crates/nexa-ops/src/batch_rename.rs:805-959`).
- 파일 I/O: 불러오기 `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1911-1913` · 저장 `:1923-1927` · 삭제 `:1675` · 폴더 `:234-236`.

| ID | 줄 형식 | 필드 | 근거 |
|---|---|---|---|
| KEY-141 | `op=replace` | `scope` · `find` · `with` · `case`(0/1) · `regex`(0/1) · `mode` | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:810-825` / `:904-911` |
| KEY-142 | `op=case` | `scope` · `mode` | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:826-828` / `:912-915` |
| KEY-143 | `op=insert` | `scope` · `text` · `off` · `dir`(`start`\|`end`) | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:829-834` / `:916-920` |
| KEY-144 | `op=number` | `scope` · `start` · `step` · `pad` · `off` · `dir` · `pre` · `suf`(기본 1 · 1 · 3) | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:835-844` / `:921-931` |
| KEY-145 | `op=date` | `scope` · `kind` · `fmt` · `off` · `dir` · `pre` · `suf` | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:845-853` / `:932-944` |
| KEY-146 | `op=move` | `start` · `len` · `dest`(`front`\|`end`) | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:854-861` / `:945-949` |
| KEY-147 | `op=ext` | `from` · `to` | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:862-864` / `:950-953` |
| KEY-148 | 위치 필드 `off=<n>\|dir=start\|end` | 공통 삽입 위치 | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:797-803` |
| KEY-149 | v1 호환 `pos=prefix\|suffix` | 구 위치 표기(읽기만) | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:872-901` |
| KEY-150 | 날짜 포맷 이행 | 구 `yyyy-MM-dd` → `${YYYY}` 문법 자동 변환 | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:936-939` |

---

## 2. i18n 키 전수

### 2-1. 구조

- 형식: properties 스타일 — BOM 무시 · `#` 주석 · 빈 줄 무시 · 첫 `=`에서 분리 · 키/값 trim · 같은 키는 마지막이 이김 · `=` 없는 줄은 버림 · `@`로 시작하면 메타(`nexa-dir2/crates/nexa-app/src/i18n.rs:58-79`).
- 값 이스케이프: `\n` · `\t` · `\\`만 해석, 그 밖의 `\x`는 글자 그대로(`nexa-dir2/crates/nexa-app/src/i18n.rs:35-56`). 그래서 `data\settings.cfg` 같은 값의 `\s`는 그대로 남는다.
- 자리표: `{0}` `{1}` …(`trf` `nexa-dir2/crates/nexa-app/src/i18n.rs:190-197`).
- 폴백: 현재 언어 → en → 키 문자열 그대로(`nexa-dir2/crates/nexa-app/src/i18n.rs:181-188`).
- 내장: `lang/en.lang` · `ko.lang` · `ja.lang`을 `include_str!`로 임베드(`nexa-dir2/crates/nexa-app/src/i18n.rs:14-17`).
- 사용자 덮어쓰기: `data\lang\<code>.lang`을 **키 단위**로 병합 · 내장에 없는 코드는 새 언어로 등재(`nexa-dir2/crates/nexa-app/src/i18n.rs:97-151`).
- `system` 해석: OS 로캘의 1차 서브태그(`ko-KR` → `ko`) · 없으면 `en`(`nexa-dir2/crates/nexa-app/src/i18n.rs:154-165`). OS 로캘은 `GetUserDefaultLocaleName`(`nexa-dir2/crates/nexa-app/src/win.rs:321-331`).
- 활성 표는 UI 스레드 `thread_local`(`nexa-dir2/crates/nexa-app/src/i18n.rs:167-178`) — 워커 스레드에서 `tr()`를 부르면 내장 en이 나온다(구조상 귀결 · 추정).
- 언어 전환은 재시작 없이 표 교체 + 메뉴/도구모음 재구성(`nexa-dir2/crates/nexa-app/src/win.rs:5570-5620`).

메타 키(파일 머리):

| ID | 메타 키 | en | ko | ja | 근거 |
|---|---|---|---|---|---|
| KEY-1501 | `@code` | `en` | `ko` | `ja` | `nexa-dir2/crates/nexa-app/lang/en.lang:3` · `ko.lang:2` · `ja.lang:3` |
| KEY-1502 | `@name` | `English` | `한국어` | `日本語` | `en.lang:4` · `ko.lang:3` · `ja.lang:4` |
| KEY-1503 | `@name.en` | `English` | `Korean` | `Japanese` | `en.lang:5` · `ko.lang:4` · `ja.lang:5` |
| KEY-1504 | `@author` | `SosomLab` | `SosomLab` | `SosomLab` | `en.lang:6` · `ko.lang:5` · `ja.lang:6` |
| KEY-1505 | `@app` | `0.22.0` | `0.22.0` | `0.22.0` | `en.lang:7` · `ko.lang:6` · `ja.lang:7` |
| KEY-1506 | `@fallback` | `en` | `en` | `en` | `en.lang:8` · `ko.lang:7` · `ja.lang:8` |

코드가 실제로 읽는 메타는 `@name`(발견 목록 표기 `nexa-dir2/crates/nexa-app/src/i18n.rs:141-144`)과 시험의 `@code`뿐이다. 나머지는 문서용이다.

### 2-2. 개수와 분류

- 문자열 키 **498개**(en = ko = ja · 중복 0 · `grep -c` 집계와 스크립트 대조).
- 코드 참조 대조(정적 스캔 · `.rs` 전수): `tr("…")`/`trf("…")` 리터럴 253개 · 표(배열) 경유 리터럴 184개 · **코드에서 전체 문자열로 찾을 수 없는 키 61개**(§2-4 · 사문 추정). 코드가 쓰는데 정의가 없는 키는 없다(`no.such.key`는 시험용 `nexa-dir2/crates/nexa-app/src/i18n.rs:259` · `:287`).

| ID | 접두 | 개수 | 내용 |
|---|---|---|---|
| KEY-1601 | `about.` | 8 | 정보 창 |
| KEY-1602 | `archive.` | 35 | 압축 미리보기 · 압축 그리드 창 · 암호 입력 |
| KEY-1603 | `bulk.` | 98 | 일괄 이름 변경(v1 잔재 포함) |
| KEY-1604 | `cloud.` | 41 | 클라우드 링크 · OAuth · 전송 |
| KEY-1605 | `col.` | 7 | 파일 목록 컬럼 제목(내 PC 컬럼 2 포함) |
| KEY-1606 | `ctx.` | 8 | 앱 고유 컨텍스트 메뉴 항목 |
| KEY-1607 | `del.` | 17 | 삭제 확인 · 잠금 · 실패 |
| KEY-1608 | `dock.` | 3 | 도크 탭 제목 |
| KEY-1609 | `drive.` | 1 | 드라이브 여유 공간 |
| KEY-1610 | `history.` | 3 | 실행 취소 실패 사유 |
| KEY-1611 | `info.` | 11 | 정보 패널 |
| KEY-1612 | `kind.` | 5 | 종류 셀 |
| KEY-1613 | `launcher.` | 2 | 퀵 런처 실행 결과 |
| KEY-1614 | `menu.` | 38 | 메뉴 바 |
| KEY-1615 | `nav.` | 1 | 내 PC |
| KEY-1616 | `new.` | 5 | 새 폴더/파일 |
| KEY-1617 | `op.` | 2 | 작업 설명(실행 취소 라벨) |
| KEY-1618 | `ops.` | 19 | 전송 진행 · 덮어쓰기 |
| KEY-1619 | `panel.` | 2 | 패널 표기(L/R) |
| KEY-1620 | `pref.` | 159 | 설정 창(카테고리 · 라벨 · 설명 · 선택지) |
| KEY-1621 | `preview.` | 7 | 미리보기 상태 · 플러그인 오류 |
| KEY-1622 | `redo.` | 3 | 다시 실행 |
| KEY-1623 | `rename.` | 2 | 이름 변경 |
| KEY-1624 | `status.` | 9 | 상태바 · 제목 |
| KEY-1625 | `tab.` | 7 | 탭 메뉴 |
| KEY-1626 | `term.` | 2 | 터미널 상태 |
| KEY-1627 | `undo.` | 3 | 실행 취소 |

### 2-3. 형식 결함(현행 · 이식 때 바로잡을 것)

`del.lockedMsg`(KEY-1107)와 `del.failMsg`(KEY-1109)는 세 언어 파일 모두에서 값 뒤에 **빈 줄 + `{1}` 단독 줄**이 이어진다(`nexa-dir2/crates/nexa-app/lang/en.lang:128-130` · `:132-134` · `ko.lang:127-129` · `:131-133` · `ja.lang:130` · `:134`). 파서는 줄 단위이고 `=` 없는 줄을 버리므로(`nexa-dir2/crates/nexa-app/src/i18n.rs:67-69`) `{1}`은 표에 들어가지 않는다. 호출부는 파일 이름 목록을 두 번째 인자로 넘기지만(`nexa-dir2/crates/nexa-app/src/win.rs:3798-3801` · `:3958-3961`) 치환될 자리표가 없어 **목록이 대화상자에 나오지 않는다**. 바른 표기는 한 줄 `…:\n\n{1}`이다. (실기 확인은 하지 않았다 — 코드 독해 결론.)

### 2-4. 코드 미참조 키 61개(사문 추정)

정적 스캔에서 키 전체 문자열이 `.rs` 어디에도 없다. 접두 조립(`format!`) 사용처도 찾지 못했다(Grep 0건). 일괄 이름 변경 v1 화면 · 3×3 위치 라벨(이미지 드롭다운으로 바뀜 `nexa-dir2/crates/nexa-app/src/prefs.rs:138-140`) · Font 행 통합의 잔재로 보인다. 아래 표의 비고 `X`가 이들이다.

- `archive.pw.remember`
- `bulk.`(45): `add` `apply` `case` `case.lower` `case.none` `case.sentence` `case.title` `case.upper` `date.fmt` `destEnd` `destFront` `dirEnd` `dirStart` `extFrom` `extTo` `find` `grid.apply` `grid.name` `insert` `kind.ext` `kind.move` `lbl.dest` `lbl.ext` `lbl.range` `lbl.wrap` `matchCase` `moveLen` `moveStart` `number` `pad` `pipeline` `posPrefix` `posSuffix` `preset.close` `preset.delete` `preset.load` `preset.name` `preset.save` `preset.savePrompt` `regex` `start` `step` `with` `wrapPre` `wrapSuf`
- `ops.applyAll`
- `pref.`(14): `dlgFontSize` `dlgFontSize.desc` `taPos.tl` `taPos.tc` `taPos.tr` `taPos.ml` `taPos.mc` `taPos.mr` `taPos.bl` `taPos.bc` `taPos.br` `termFont.desc` `termFontSize` `termFontSize.desc`

### 2-5. 플랫폼 종속 문구(크로스플랫폼에서 OS별 문자열 또는 문구 수정이 필요한 키)

| ID | 키 | 종속 표현 |
|---|---|---|
| KEY-1053 | `cloud.err.noClientIdMsg` | `data\settings.cfg` 경로 · 구 키 이름 `cloud_client_id_{1}`(새 키 이름으로 바뀜) |
| KEY-1074 | `about.desc` | "for Windows" / "Windows 파일 탐색기" |
| KEY-1089 | `kind.drive` | 드라이브 개념(macOS · Linux = 볼륨/마운트) |
| KEY-1101 | `del.kindRecycle` | "Recycle Bin"(macOS = Trash/휴지통 · Linux = Trash) |
| KEY-1147 | `pref.plugins.desc` | `data\plugins\` · "exe 옆 plugins\" |
| KEY-1148 | `pref.plugins.empty` | 위와 같음 |
| KEY-1151 | `term.fail` | "(ConPTY)" — Unix는 PTY |
| KEY-1272 | `pref.showHidden.desc` | "hidden attribute"(Unix에는 숨김 속성이 없다 — macOS `UF_HIDDEN`만) |
| KEY-1287 | `pref.ctxMenuOrder.desc` | "shell verbs" |
| KEY-1314 | `pref.termCopy.desc` | "Ctrl+C"(macOS = ⌘C) |
| KEY-1322 | `pref.navUpAlign.desc` | "Alt+Up" |
| KEY-1356 | `pref.fsGridExtra.desc` | "nexa-sql result grid" 언급(제품 간 참조) |
| KEY-1381 | `nav.mypc` | "This PC" |
| KEY-1389 | `pref.ctxFont.desc` | "current right-click menus follow the OS font"(dir3는 전부 직접 그림 → 문구 폐기) |
| KEY-1471 | `archive.openHint` | "F3" 단축키 |

### 2-6. 전체 목록

행 순서 = `nexa-dir2/crates/nexa-app/lang/en.lang` 등장 순서. ko는 `nexa-dir2/crates/nexa-app/lang/ko.lang`의 같은 키 값. 값은 파일에 적힌 그대로(`\n`은 이스케이프 표기). 비고: `X` = 코드 미참조(§2-4) · `P` = 플랫폼 종속 문구(§2-5) · `B` = 형식 결함(§2-3).

#### 메뉴 바 (`en.lang:10-40`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1001 | `menu.file` | `File` | `파일` | |
| KEY-1002 | `menu.file.newTab` | `New Tab` | `새 탭` | |
| KEY-1003 | `menu.file.closeTab` | `Close Tab` | `탭 닫기` | |
| KEY-1004 | `menu.file.newFolder` | `New Folder` | `새 폴더 만들기` | |
| KEY-1005 | `menu.file.newFile` | `New File` | `새 파일 만들기` | |
| KEY-1006 | `menu.file.exit` | `Exit` | `종료` | |
| KEY-1007 | `menu.edit` | `Edit` | `편집` | |
| KEY-1008 | `menu.edit.undo` | `Undo` | `실행 취소` | |
| KEY-1009 | `menu.edit.redo` | `Redo` | `다시 실행` | |
| KEY-1010 | `menu.edit.cut` | `Cut` | `잘라내기` | |
| KEY-1011 | `menu.edit.copy` | `Copy` | `복사` | |
| KEY-1012 | `menu.edit.paste` | `Paste` | `붙여넣기` | |
| KEY-1013 | `menu.edit.delete` | `Delete` | `삭제` | |
| KEY-1014 | `menu.edit.selectAll` | `Select All` | `전체 선택` | |
| KEY-1015 | `menu.view` | `View` | `보기` | |
| KEY-1016 | `menu.view.modeTree` | `Tree View` | `트리 보기` | |
| KEY-1017 | `menu.view.modeFlat` | `Flat View` | `플랫 보기` | |
| KEY-1018 | `menu.view.modeTiles` | `Tile View` | `타일 보기` | |
| KEY-1019 | `menu.view.hidden` | `Show Hidden Files` | `숨김 파일 표시` | |
| KEY-1020 | `menu.view.dot` | `Show Dot Files` | `점 파일 표시` | |
| KEY-1021 | `menu.view.dock` | `Bottom Dock` | `하단 도크` | |
| KEY-1022 | `menu.view.launcher` | `Quick Launcher Bar` | `퀵 런처 바` | |
| KEY-1023 | `menu.view.alwaysOnTop` | `Always on Top` | `항상 맨 위에 표시` | |
| KEY-1024 | `menu.view.refresh` | `Refresh` | `새로 고침` | |
| KEY-1025 | `menu.view.theme.system` | `Theme: System` | `테마: 시스템` | |
| KEY-1026 | `menu.view.theme.light` | `Theme: Light` | `테마: 라이트` | |
| KEY-1027 | `menu.view.theme.dark` | `Theme: Dark` | `테마: 다크` | |
| KEY-1028 | `menu.view.lang.system` | `Language: System` | `언어: 시스템` | |
| KEY-1029 | `menu.help` | `Help` | `도움말` | |
| KEY-1030 | `menu.help.about` | `About Nexa Dir` | `Nexa Dir 정보` | |

#### 클라우드 (`en.lang:42-85`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1031 | `menu.cloud` | `Cloud` | `클라우드` | |
| KEY-1032 | `cloud.add` | `Add Link…` | `링크 추가하기…` | |
| KEY-1033 | `cloud.goto` | `Go to Folder` | `바로 가기` | |
| KEY-1034 | `cloud.web` | `Open Online` | `온라인 보기` | |
| KEY-1035 | `cloud.copyUrl` | `Copy URL` | `URL 복사` | |
| KEY-1036 | `cloud.disconnect` | `Disconnect` | `연결 해제` | |
| KEY-1037 | `cloud.unlink` | `Remove Link` | `링크 해제` | |
| KEY-1038 | `cloud.linked` | `{0} linked` | `{0} 링크됨` | |
| KEY-1039 | `cloud.unlinked` | `{0} unlinked` | `{0} 링크 해제됨` | |
| KEY-1040 | `cloud.connectTo` | `Link {0}` | `{0} 링크` | |
| KEY-1041 | `cloud.addNone` | `No cloud sync clients detected` | `감지된 클라우드 동기화 클라이언트가 없습니다` | |
| KEY-1042 | `cloud.connected` | `{0} connected` | `{0} 연결됨` | |
| KEY-1043 | `cloud.removed` | `{0} disconnected` | `{0} 연결 해제됨` | |
| KEY-1044 | `cloud.urlCopied` | `URL copied — paste into a private window to sign in with another account` | `URL 복사됨 — 프라이빗 창에 붙여넣으면 다른 계정으로 접속할 수 있습니다` | |
| KEY-1045 | `cloud.gone` | `Folder not found — {0}` | `폴더를 찾을 수 없음 — {0}` | |
| KEY-1046 | `cloud.loading` | `Loading…` | `불러오는 중…` | |
| KEY-1047 | `cloud.connect` | `Connect Cloud` | `클라우드 직접 연결` | |
| KEY-1048 | `cloud.auth.prompt` | `Sign in to {0} in your browser.\n\nTo use a DIFFERENT account than the one already signed in, copy the URL below and open it in a private/incognito window.\n\n{1}` | `브라우저에서 {0}에 로그인하세요.\n\n이미 로그인된 계정이 아닌 **다른 계정**으로 연결하려면 아래 URL을 복사해 프라이빗(시크릿) 창에서 여세요.\n\n{1}` | |
| KEY-1049 | `cloud.auth.openBrowser` | `Open Browser` | `브라우저 열기` | |
| KEY-1050 | `cloud.auth.waiting` | `Waiting for browser sign-in…` | `브라우저 로그인 대기 중…` | |
| KEY-1051 | `cloud.account.unknown` | `account` | `계정` | |
| KEY-1052 | `cloud.err.noClientId` | `Client ID is not set` | `Client ID가 설정되지 않았습니다` | |
| KEY-1053 | `cloud.err.noClientIdMsg` | `No Client ID is available for {0} yet.\n\nNexaDir ships default IDs where possible; this service still needs your own (or none is bundled yet).\n\nRegister a desktop/native app in the provider console, then add this line to data\settings.cfg:\n\ncloud_client_id_{1}=<YOUR CLIENT ID>\n\nRedirect URI (enter exactly this in the console):\n  OneDrive / Google Drive -> http://localhost\n  Dropbox -> http://localhost:53682 (must match exactly)\n\nGoogle and Dropbox also need a secret:\n  cloud_client_secret_{1}=<YOUR SECRET>\n\nSee the "Cloud" page in the wiki for the full walkthrough.` | `{0}에 사용할 Client ID가 아직 없습니다.\n\nNexaDir는 가능한 서비스에 기본 ID를 동봉하지만, 이 서비스는 아직 미동봉이거나 직접 발급이 필요합니다.\n\n제공자 콘솔에서 데스크톱/네이티브 앱으로 등록한 뒤, data\settings.cfg에 다음 줄을 추가하세요:\n\ncloud_client_id_{1}=<발급받은 CLIENT ID>\n\n리디렉션 URI(등록 화면에 그대로 입력):\n  OneDrive · Google Drive → http://localhost\n  Dropbox → http://localhost:53682 (정확히 일치해야 함)\n\nGoogle·Dropbox는 시크릿도 필요합니다:\n  cloud_client_secret_{1}=<발급받은 SECRET>\n\n자세한 절차는 위키 "클라우드" 문서를 참고하세요.` | P |
| KEY-1054 | `cloud.err.listener` | `Could not open the local callback listener` | `로컬 콜백 리스너를 열 수 없습니다` | |
| KEY-1055 | `cloud.err.timeout` | `Sign-in timed out or was cancelled` | `로그인 시간이 초과되었거나 취소되었습니다` | |
| KEY-1056 | `cloud.err.denied` | `Sign-in was denied` | `로그인이 거부되었습니다` | |
| KEY-1057 | `cloud.err.exchange` | `Token exchange failed` | `토큰 교환에 실패했습니다` | |
| KEY-1058 | `cloud.err.tokenSave` | `Connected, but the token could not be saved — sign-in will be required again` | `연결됨 — 다만 토큰을 저장하지 못해 다시 로그인이 필요합니다` | |
| KEY-1059 | `cloud.err.full` | `Too many cloud connections` | `클라우드 연결이 너무 많습니다` | |
| KEY-1060 | `cloud.progressLabel` | `Transferring with the cloud…` | `클라우드와 전송 중…` | |
| KEY-1061 | `cloud.downloading` | `Downloading {0} file(s) from the cloud…` | `클라우드에서 {0}개 파일 내려받는 중…` | |
| KEY-1062 | `cloud.downloaded` | `{0} file(s) downloaded` | `{0}개 파일 내려받음` | |
| KEY-1063 | `cloud.err.noToken` | `Not signed in — reconnect this cloud account` | `로그인 정보가 없습니다 — 이 클라우드를 다시 연결하세요` | |
| KEY-1064 | `cloud.uploading` | `Uploading {0} file(s)…` | `{0}개 파일 업로드 중…` | |
| KEY-1065 | `cloud.copying` | `Copying {0} item(s) within the cloud…` | `클라우드 안에서 {0}개 항목 복사 중…` | |
| KEY-1066 | `cloud.deleting` | `Deleting {0} item(s) in the cloud…` | `클라우드에서 {0}개 항목 삭제 중…` | |
| KEY-1067 | `cloud.renaming` | `Renaming in the cloud…` | `클라우드 항목 이름 변경 중…` | |
| KEY-1068 | `cloud.creating` | `Creating folder in the cloud…` | `클라우드에 폴더 만드는 중…` | |
| KEY-1069 | `cloud.writeDone` | `{0} item(s) updated in the cloud` | `클라우드 {0}개 항목 반영됨` | |
| KEY-1070 | `cloud.err.filesOnly` | `Only files can be uploaded — folder upload is not supported yet` | `파일만 업로드할 수 있습니다 — 폴더 업로드는 아직 지원하지 않습니다` | |
| KEY-1071 | `cloud.err.foldersOnly` | `Only folders can be created in the cloud for now` | `클라우드에는 현재 폴더만 만들 수 있습니다` | |
| KEY-1072 | `cloud.err.reauth` | `Permission denied — reconnect this cloud account (write access was added)` | `권한 거부됨 — 이 클라우드를 다시 연결하세요(쓰기 권한이 추가되었습니다)` | |

#### 정보 창 · 컬럼 · 종류 (`en.lang:87-109`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1073 | `about.title` | `About Nexa Dir` | `Nexa Dir 정보` | |
| KEY-1074 | `about.desc` | `Ultra-low-memory portable file explorer for Windows` | `초저메모리 포터블 Windows 파일 탐색기` | P |
| KEY-1075 | `about.version` | `Version` | `버전` | |
| KEY-1076 | `about.link.repo` | `Nexa Dir repository (GitHub)` | `Nexa Dir 저장소 (GitHub)` | |
| KEY-1077 | `about.link.releases` | `Download (GitHub Releases)` | `다운로드 (GitHub Releases)` | |
| KEY-1078 | `about.license` | `License: PolyForm Noncommercial 1.0.0` | `라이선스: PolyForm Noncommercial 1.0.0` | |
| KEY-1079 | `about.copyright` | `© 2026 SosomLab · Sangyong Bae` | `© 2026 SosomLab · Sangyong Bae` | |
| KEY-1080 | `about.ok` | `OK` | `확인` | |
| KEY-1081 | `col.name` | `Name` | `이름` | |
| KEY-1082 | `col.ext` | `Ext` | `확장자` | |
| KEY-1083 | `col.size` | `Size` | `크기` | |
| KEY-1084 | `col.modified` | `Date modified` | `수정한 날짜` | |
| KEY-1085 | `col.kind` | `Kind` | `종류` | |
| KEY-1086 | `kind.folder` | `Folder` | `폴더` | |
| KEY-1087 | `kind.link` | `Link` | `바로가기` | |
| KEY-1088 | `kind.file` | `File` | `파일` | |
| KEY-1089 | `kind.drive` | `Drive` | `드라이브` | P |
| KEY-1090 | `kind.extFile` | `{0} file` | `{0} 파일` | |

#### 파일 작업 · 삭제 · 이름 변경 · 새로 만들기 (`en.lang:111-143`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1091 | `ops.progress` | `transferring {0}%` | `전송 {0}%` | |
| KEY-1092 | `ops.done` | `transferred {0}` | `전송 {0}` | |
| KEY-1093 | `ops.skipped` | `{0} skipped` | `건너뜀 {0}` | |
| KEY-1094 | `ops.errors` | `{0} failed` | `실패 {0}` | |
| KEY-1095 | `ops.busy` | `A transfer is in progress — try again when it finishes` | `전송 진행 중 — 끝난 뒤 다시 시도하세요` | |
| KEY-1096 | `ops.canceled` | `canceled` | `취소됨` | |
| KEY-1097 | `ops.overwriteTitle` | `Confirm Overwrite` | `덮어쓰기 확인` | |
| KEY-1098 | `ops.overwrite` | `'{0}' already exists in the destination. Overwrite?\n(No = skip this item · Cancel = stop all)` | `'{0}'이(가) 대상 폴더에 이미 있습니다. 덮어쓸까요?\n(아니오=이 항목 건너뜀 · 취소=전체 중단)` | |
| KEY-1099 | `del.title` | `Delete` | `삭제` | |
| KEY-1100 | `del.confirm` | `Permanently delete {0} item(s)?` | `{0}개 항목을 영구 삭제할까요?` | |
| KEY-1101 | `del.kindRecycle` | `Recycle Bin` | `휴지통` | P |
| KEY-1102 | `del.kindPermanent` | `permanent` | `완전 삭제` | |
| KEY-1103 | `del.done` | `{0}: {1} deleted` | `{0}: {1}개 삭제` | |
| KEY-1104 | `del.recycleOp` | `recycle {0} item(s)` | `휴지통 삭제 {0}개` | |
| KEY-1105 | `del.partialFail` | `{0} failed` | `실패 {0}개` | |
| KEY-1106 | `del.lockedTitle` | `Items In Use` | `삭제할 수 없는 항목` | |
| KEY-1107 | `del.lockedMsg` | `The following {0} item(s) are in use by another program:` (+ 버려지는 줄 `{1}`) | `다음 {0}개 항목이 다른 프로그램에서 사용 중입니다:` (+ 버려지는 줄 `{1}`) | B |
| KEY-1108 | `del.failTitle` | `Delete Failed` | `삭제 실패` | |
| KEY-1109 | `del.failMsg` | `The following {0} item(s) could not be deleted (in use or inaccessible):` (+ 버려지는 줄 `{1}`) | `다음 {0}개 항목을 삭제하지 못했습니다 (사용 중이거나 접근할 수 없음):` (+ 버려지는 줄 `{1}`) | B |
| KEY-1110 | `del.skipLocked` | `Skip and Delete ({0})` | `건너뛰고 삭제({0}개)` | |
| KEY-1111 | `del.retry` | `Retry` | `다시 시도` | |
| KEY-1112 | `del.cancel` | `Cancel` | `취소` | |
| KEY-1113 | `del.close` | `Close` | `닫기` | |
| KEY-1114 | `del.listMore` | `…and {0} more` | `…외 {0}개` | |
| KEY-1115 | `rename.fail` | `rename failed: {0}` | `이름변경 실패: {0}` | |
| KEY-1116 | `new.folderBase` | `New Folder` | `새 폴더` | |
| KEY-1117 | `new.fileBase` | `New File` | `새 파일` | |
| KEY-1118 | `new.fail` | `create failed: {0}` | `만들기 실패: {0}` | |

#### 실행 취소 · 컨텍스트 메뉴 (`en.lang:145-165`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1119 | `undo.none` | `nothing to undo` | `실행 취소할 작업 없음` | |
| KEY-1120 | `redo.none` | `nothing to redo` | `다시 실행할 작업 없음` | |
| KEY-1121 | `undo.done` | `undone: {0}` | `실행 취소: {0}` | |
| KEY-1122 | `redo.done` | `redone: {0}` | `다시 실행: {0}` | |
| KEY-1123 | `undo.fail` | `undo failed ({0}): {1}` | `실행 취소 실패({0}): {1}` | |
| KEY-1124 | `redo.fail` | `redo failed ({0}): {1}` | `다시 실행 실패({0}): {1}` | |
| KEY-1125 | `history.failedItems` | `{0} item(s) failed` | `{0}개 항목 실패` | |
| KEY-1126 | `history.missingSource` | `source missing: {0}` | `원본 없음: {0}` | |
| KEY-1127 | `history.nameExists` | `name already exists: {0}` | `같은 이름이 이미 있음: {0}` | |
| KEY-1128 | `op.moveCount` | `move {0} item(s)` | `이동 {0}개` | |
| KEY-1129 | `op.copyCount` | `copy {0} item(s)` | `복사 {0}개` | |
| KEY-1130 | `rename.done` | `rename: {0} → {1}` | `이름 변경: {0} → {1}` | |
| KEY-1131 | `new.folderOp` | `new folder` | `새 폴더` | |
| KEY-1132 | `new.fileOp` | `new file` | `새 파일` | |
| KEY-1133 | `ctx.new` | `New` | `새로 만들기` | |
| KEY-1134 | `ctx.deletePermanent` | `Delete Permanently` | `완전 삭제` | |
| KEY-1135 | `ctx.pasteInto` | `Paste into Folder` | `폴더에 붙여넣기` | |
| KEY-1136 | `ctx.paste` | `Paste` | `붙여넣기` | |

#### 하단 도크 · 미리보기 · 터미널 · 정보 (`en.lang:167-195`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1137 | `dock.info` | `Info` | `정보` | |
| KEY-1138 | `dock.preview` | `Preview` | `미리보기` | |
| KEY-1139 | `preview.none` | `nothing to preview` | `미리보기할 항목 없음` | |
| KEY-1140 | `preview.empty` | `(empty file)` | `(빈 파일)` | |
| KEY-1141 | `preview.binary` | `binary file — no text preview` | `이진 파일 — 텍스트 미리보기 없음` | |
| KEY-1142 | `preview.fail` | `read failed` | `읽기 실패` | |
| KEY-1143 | `preview.plugin.error` | `plugin error ({0}): {1}` | `플러그인 오류({0}): {1}` | |
| KEY-1144 | `preview.plugin.disabled` | `plugin isolated ({0}): {1} consecutive failures — not run again this session` | `플러그인 격리({0}): 연속 {1}회 실패 — 이 세션에서는 실행하지 않습니다` | |
| KEY-1145 | `preview.window.image` | `image file — shown in the bottom dock preview` | `이미지 파일 — 하단 도크 미리보기에서 표시됩니다` | |
| KEY-1146 | `pref.cat.plugins` | `Plugins` | `플러그인` | |
| KEY-1147 | `pref.plugins.desc` | `Preview plugins (*.wasm in data\plugins\ or plugins\ next to the exe) — unchecked plugins fall back to the built-in preview.` | `미리보기 플러그인(data\plugins\ 또는 exe 옆 plugins\의 *.wasm) 사용 여부 — 체크 해제 시 내장 미리보기로 대체됩니다.` | P |
| KEY-1148 | `pref.plugins.empty` | `No plugins installed — copy .wasm files into data\plugins\ (or plugins\ next to the exe) and restart the app.` | `설치된 플러그인이 없습니다 — data\plugins\(또는 exe 옆 plugins\) 폴더에 .wasm 파일을 복사한 뒤 앱을 다시 시작하세요.` | P |
| KEY-1149 | `dock.terminal` | `Terminal` | `터미널` | |
| KEY-1150 | `term.exited` | `[shell exited — press any key to restart]` | `[셸 종료됨 — 아무 키나 누르면 재시작]` | |
| KEY-1151 | `term.fail` | `terminal start failed (ConPTY)` | `터미널 시작 실패(ConPTY)` | P |
| KEY-1152 | `info.selected` | `{0} selected` | `{0}개 선택` | |
| KEY-1153 | `info.name` | `Name: {0}` | `이름: {0}` | |
| KEY-1154 | `info.kind` | `Kind: {0}` | `종류: {0}` | |
| KEY-1155 | `info.path` | `Path: {0}` | `경로: {0}` | |
| KEY-1156 | `info.size` | `Size: {0}` | `크기: {0}` | |
| KEY-1157 | `info.sizeOnDisk` | `Size on disk: {0}` | `디스크 할당 크기: {0}` | |
| KEY-1158 | `info.created` | `Created: {0}` | `만든 날짜: {0}` | |
| KEY-1159 | `info.modified` | `Modified: {0}` | `수정한 날짜: {0}` | |
| KEY-1160 | `info.accessed` | `Accessed: {0}` | `액세스한 날짜: {0}` | |
| KEY-1161 | `info.loadingDetails` | `Loading details…` | `자세한 정보 불러오는 중…` | |
| KEY-1162 | `info.currentFolder` | `Current folder: {0}` | `현재 폴더: {0}` | |
| KEY-1163 | `ctx.undoOf` | `Undo: {0}` | `실행 취소: {0}` | |
| KEY-1164 | `ctx.redoOf` | `Redo: {0}` | `다시 실행: {0}` | |

#### 상태바 · 탭 · 전송 확인 (`en.lang:197-223`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1165 | `status.itemCount` | `{0} items` | `{0}개 항목` | |
| KEY-1166 | `status.selectedCount` | `{0} selected` | `선택 {0}` | |
| KEY-1167 | `status.tab` | `Tab {0}/{1}` | `탭 {0}/{1}` | |
| KEY-1168 | `status.avg` | `avg {0}µs` | `평균 {0}µs` | |
| KEY-1169 | `status.firstRender` | `first render {0}ms` | `첫렌더 {0}ms` | |
| KEY-1170 | `status.filters` | `Hidden: {0} · Dot: {1}` | `숨김 {0} · 점 {1}` | |
| KEY-1171 | `status.show` | `shown` | `표시` | |
| KEY-1172 | `status.hide` | `hidden` | `감춤` | |
| KEY-1173 | `panel.left` | `L` | `좌` | |
| KEY-1174 | `panel.right` | `R` | `우` | |
| KEY-1175 | `tab.lock` | `Lock tab` | `탭 잠금` | |
| KEY-1176 | `tab.unlock` | `Unlock tab` | `탭 잠금 해제` | |
| KEY-1177 | `tab.duplicate` | `Duplicate tab` | `탭 복제` | |
| KEY-1178 | `tab.new` | `New tab` | `새 탭` | |
| KEY-1179 | `tab.close` | `Close tab` | `탭 닫기` | |
| KEY-1180 | `ops.applyAll` | `(Yes=overwrite all / No=skip all / Cancel=abort - the choice applies to all remaining conflicts)` | `(예=모두 덮어쓰기 · 아니오=모두 건너뛰기 · 취소=중단 — 선택은 남은 충돌 전체에 적용됩니다)` | X |
| KEY-1181 | `ctx.copyPath` | `Copy as path` | `경로 복사` | |
| KEY-1182 | `ops.yes` | `Overwrite` | `덮어쓰기` | |
| KEY-1183 | `ops.yesAll` | `Overwrite all` | `모두 덮어쓰기` | |
| KEY-1184 | `ops.skip` | `Skip` | `건너뛰기` | |
| KEY-1185 | `ops.cancel` | `Cancel` | `취소` | |
| KEY-1186 | `ops.progressTitle` | `File transfer` | `파일 전송` | |
| KEY-1187 | `ops.progressLabel` | `Transferring...` | `전송 중...` | |

#### 설정 창 1 · 런처 · 일괄 이름 변경 v1 (`en.lang:225-299`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1188 | `pref.title` | `Preferences` | `설정` | |
| KEY-1189 | `pref.theme` | `Theme` | `테마` | |
| KEY-1190 | `pref.theme.system` | `System` | `시스템` | |
| KEY-1191 | `pref.theme.light` | `Light` | `라이트` | |
| KEY-1192 | `pref.theme.dark` | `Dark` | `다크` | |
| KEY-1193 | `pref.lang` | `Language` | `언어` | |
| KEY-1194 | `pref.lang.system` | `System` | `시스템` | |
| KEY-1195 | `pref.termFont` | `Terminal font (comma = fallback chain)` | `터미널 글꼴(쉼표=폴백 체인)` | |
| KEY-1196 | `pref.termFontSize` | `Terminal font size (8-32)` | `터미널 글꼴 크기(8~32)` | X |
| KEY-1197 | `pref.dlgFont` | `Dialog font` | `대화상자 글꼴` | |
| KEY-1198 | `pref.dlgFontSize` | `Dialog font size (7-24)` | `대화상자 글꼴 크기(7~24)` | X |
| KEY-1199 | `menu.file.prefs` | `Preferences...` | `설정...` | |
| KEY-1200 | `ctx.copyName` | `Copy as name` | `이름 복사` | |
| KEY-1201 | `pref.cat.appearance` | `Appearance` | `모양` | |
| KEY-1202 | `pref.cat.fonts` | `Fonts` | `글꼴` | |
| KEY-1203 | `pref.cat.terminal` | `Terminal` | `터미널` | |
| KEY-1204 | `pref.cat.list` | `File List` | `파일 목록` | |
| KEY-1205 | `pref.cat.ctxmenu` | `Context Menu` | `컨텍스트 메뉴` | |
| KEY-1206 | `pref.cat.dock` | `Bottom Dock` | `하단 도크` | |
| KEY-1207 | `pref.cat.lang` | `Language` | `언어` | |
| KEY-1208 | `pref.search.placeholder` | `Search settings` | `설정 검색` | |
| KEY-1209 | `pref.showHidden` | `Show hidden files` | `숨김 파일 표시` | |
| KEY-1210 | `pref.showDotfiles` | `Show dot (.) files` | `점(.) 파일 표시` | |
| KEY-1211 | `pref.dock` | `Show bottom dock` | `하단 도크 표시` | |
| KEY-1212 | `launcher.ran` | `{0} launched` | `{0} 실행` | |
| KEY-1213 | `launcher.failed` | `{0} launch failed` | `{0} 실행 실패` | |
| KEY-1214 | `menu.edit.bulkRename` | `Bulk Rename...` | `일괄 이름 변경...` | |
| KEY-1215 | `bulk.title` | `Bulk Rename` | `일괄 이름 변경` | |
| KEY-1216 | `bulk.find` | `Find` | `찾기` | X |
| KEY-1217 | `bulk.with` | `Replace with` | `바꾸기` | X |
| KEY-1218 | `bulk.matchCase` | `Match case` | `대소문자 일치` | X |
| KEY-1219 | `bulk.insert` | `Insert text` | `텍스트 삽입` | X |
| KEY-1220 | `bulk.posPrefix` | `Before name` | `이름 앞` | X |
| KEY-1221 | `bulk.posSuffix` | `After name` | `이름 뒤` | X |
| KEY-1222 | `bulk.case` | `Change case` | `대소문자 변경` | X |
| KEY-1223 | `bulk.case.none` | `Keep` | `유지` | X |
| KEY-1224 | `bulk.case.upper` | `UPPER` | `대문자` | X |
| KEY-1225 | `bulk.case.lower` | `lower` | `소문자` | X |
| KEY-1226 | `bulk.case.title` | `Title` | `단어별` | X |
| KEY-1227 | `bulk.case.sentence` | `Sentence` | `문장형` | X |
| KEY-1228 | `bulk.number` | `Number sequence` | `연번 붙이기` | X |
| KEY-1229 | `bulk.start` | `Start` | `시작` | X |
| KEY-1230 | `bulk.step` | `Step` | `증가` | X |
| KEY-1231 | `bulk.pad` | `Digits` | `자릿수` | X |
| KEY-1232 | `bulk.apply` | `Apply` | `적용` | X |
| KEY-1233 | `bulk.cancel` | `Cancel` | `취소` | |
| KEY-1234 | `bulk.conflict.empty` | `empty name` | `빈 이름` | |
| KEY-1235 | `bulk.conflict.invalid` | `invalid characters` | `금지 문자` | |
| KEY-1236 | `bulk.conflict.dup` | `duplicate` | `중복` | |
| KEY-1237 | `bulk.conflict.exists` | `already exists` | `이미 존재` | |
| KEY-1238 | `bulk.conflict.nested` | `selected with its parent folder` | `상위 폴더와 함께 선택됨` | |
| KEY-1239 | `bulk.done` | `renamed {0}` | `이름변경 {0}` | |
| KEY-1240 | `bulk.fail` | `failed {0}` | `실패 {0}` | |
| KEY-1241 | `bulk.noSelection` | `no selection` | `선택 없음` | |
| KEY-1242 | `bulk.regex` | `Regular expression` | `정규식` | X |
| KEY-1243 | `bulk.add` | `Add block` | `블록 추가` | X |
| KEY-1244 | `bulk.pipeline` | `Pipeline (applied top to bottom)` | `파이프라인 (위에서 아래 순서로 적용)` | X |
| KEY-1245 | `bulk.date.fmtHelpTitle` | `Date format tokens` | `날짜 포맷 토큰` | |
| KEY-1246 | `bulk.date.fmtHelpNote` | `Characters outside tokens are kept as-is. e.g. ${YYYY}-${MM}-${DD} -> 2026-07-17` | `토큰 외 문자는 그대로 사용됩니다. 예: ${YYYY}-${MM}-${DD} → 2026-07-17` | |
| KEY-1247 | `bulk.kind.replace` | `Replace Text` | `치환 (텍스트)` | |
| KEY-1248 | `bulk.kind.replaceRx` | `Replace RegEx` | `치환 (정규식)` | |
| KEY-1249 | `bulk.kind.case` | `Change Case` | `대소문자 변경` | |
| KEY-1250 | `bulk.kind.insert` | `Insert Text` | `텍스트 삽입` | |
| KEY-1251 | `bulk.kind.number` | `Add Number Sequence` | `연번 붙이기` | |
| KEY-1252 | `bulk.kind.move` | `Move segment` | `구간 이동` | X |
| KEY-1253 | `bulk.kind.ext` | `Change extension` | `확장자 변경` | X |
| KEY-1254 | `bulk.moveStart` | `Start pos (1-based)` | `시작 위치(1부터)` | X |
| KEY-1255 | `bulk.moveLen` | `Length` | `길이` | X |
| KEY-1256 | `bulk.destFront` | `To front` | `맨 앞으로` | X |
| KEY-1257 | `bulk.destEnd` | `To end` | `맨 뒤로` | X |
| KEY-1258 | `bulk.extFrom` | `From ext (empty=any)` | `기존 확장자(빈값=전체)` | X |
| KEY-1259 | `bulk.extTo` | `To ext` | `새 확장자` | X |
| KEY-1260 | `bulk.preset.name` | `Preset name` | `프리셋 이름` | X |
| KEY-1261 | `bulk.preset.save` | `Save` | `저장` | X |
| KEY-1262 | `bulk.preset.load` | `Load` | `불러오기` | X |

#### 설정 창 2 — 그룹 · 설명 · 순서 편집 · 터미널 · 정렬 · 탭 · 타입어헤드 · 고속 스크롤 (`en.lang:300-417`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1263 | `pref.grp.general` | `General` | `일반` | |
| KEY-1264 | `pref.grp.panel` | `Bottom Dock` | `하단 도크` | |
| KEY-1265 | `pref.search.results` | `{0} matches` | `{0}개 일치` | |
| KEY-1266 | `pref.theme.desc` | `Color theme for the app — System follows the OS setting.` | `앱 색 테마 — 시스템은 OS 설정을 따릅니다.` | |
| KEY-1267 | `pref.lang.desc` | `Display language — System follows the OS language.` | `표시 언어 — 시스템은 OS 언어를 따릅니다.` | |
| KEY-1268 | `pref.termFont.desc` | `Terminal font. Comma-separated list becomes the fallback chain.` | `터미널 글꼴. 쉼표로 나열하면 폴백 체인이 됩니다.` | X |
| KEY-1269 | `pref.termFontSize.desc` | `Terminal font size in DIP (8~32).` | `터미널 글꼴 크기(DIP, 8~32).` | X |
| KEY-1270 | `pref.dlgFont.desc` | `Font for dialogs (confirmations, progress).` | `대화상자(확인창·진행 창) 글꼴.` | |
| KEY-1271 | `pref.dlgFontSize.desc` | `Dialog font size in points (7~24).` | `대화상자 글꼴 크기(pt, 7~24).` | X |
| KEY-1272 | `pref.showHidden.desc` | `Show files and folders with the hidden attribute.` | `숨김 속성 파일·폴더를 표시합니다.` | P |
| KEY-1273 | `pref.showDotfiles.desc` | `Show files and folders whose name starts with a dot.` | `이름이 점(.)으로 시작하는 파일·폴더를 표시합니다.` | |
| KEY-1274 | `pref.colAutofitMax` | `Column auto-fit max width (50-2000)` | `컬럼 자동 맞춤 최대 폭 (50-2000)` | |
| KEY-1275 | `pref.colAutofitMax.desc` | `Maximum width in pixels when double-clicking a column border to auto-fit contents.` | `컬럼 경계를 더블클릭해 내용에 맞출 때 허용되는 최대 폭(px).` | |
| KEY-1276 | `pref.toolbarOrder` | `Toolbar Order` | `도구 모음 순서` | |
| KEY-1277 | `pref.toolbarOrder.desc` | `Reorder toolbar buttons. Select a group or an item, then use the up/down buttons - groups move as a whole, items move within their group (Shift-click = contiguous multi-select).` | `도구 모음 버튼 순서를 바꿉니다. 그룹/항목을 선택하고 위/아래 버튼 사용 — 그룹은 통째로, 항목은 그룹 안에서만 이동(Shift 클릭 = 연속 다중 선택).` | |
| KEY-1278 | `pref.tbo.grpPanel` | `Panel Controls` | `패널 제어` | |
| KEY-1279 | `pref.tbo.grpView` | `View Mode` | `보기 모드` | |
| KEY-1280 | `pref.tbo.grpShow` | `Visibility` | `표시 항목` | |
| KEY-1281 | `pref.tbo.panelToggle` | `File Panel Toggle` | `파일 패널 토글` | |
| KEY-1282 | `pref.tbo.infoToggle` | `Info Panes Toggle` | `정보 패널 토글` | |
| KEY-1283 | `pref.editBtn` | `Edit...` | `편집...` | |
| KEY-1284 | `pref.colLayout` | `File Columns` | `파일 컬럼` | |
| KEY-1285 | `pref.colLayout.desc` | `Show/hide and reorder file list columns in a separate window (applies to the focused panel; follows column sync).` | `파일 목록 컬럼의 표시/숨김과 순서를 별도 창에서 변경합니다(포커스 패널에 적용 — 컬럼 동기화 규약을 따름).` | |
| KEY-1286 | `pref.ctxMenuOrder` | `Context Menu Items` | `컨텍스트 메뉴 항목` | |
| KEY-1287 | `pref.ctxMenuOrder.desc` | `Show/hide and reorder the app's own context menu items (shell verbs are not affected).` | `앱 고유 우클릭 메뉴 항목의 표시/숨김과 순서를 별도 창에서 변경합니다(셸 제공 항목은 대상 아님).` | P |
| KEY-1288 | `pref.ctxm.grpRow` | `Item Menu` | `항목 메뉴` | |
| KEY-1289 | `pref.ctxm.grpBg` | `Background Menu` | `배경 메뉴` | |
| KEY-1290 | `pref.dock.desc` | `Show the bottom dock (info, preview, terminal).` | `하단 도크(정보·미리보기·터미널)를 표시합니다.` | |
| KEY-1291 | `pref.sortFoldersFirst` | `Sort folders first` | `폴더 우선 정렬` | |
| KEY-1292 | `pref.sortFoldersFirst.desc` | `Group folders before files regardless of sort order.` | `정렬 방향과 무관하게 폴더를 파일보다 앞에 모읍니다.` | |
| KEY-1293 | `pref.hideEmptyGlyph` | `Hide expand glyph for empty folders` | `빈 폴더의 펼침 글리프 숨김` | |
| KEY-1294 | `pref.hideEmptyGlyph.desc` | `Hide the tree expand glyph when a folder has nothing to show under the current view (hidden/dot filters). Re-appears when the view options reveal content.` | `현재 보기(숨김/점 파일 필터)로 보여줄 내용이 없는 폴더는 트리 펼침 글리프를 그리지 않습니다. 보기 옵션 변경으로 내용이 보이면 다시 표시됩니다.` | |
| KEY-1295 | `pref.viewScope` | `View options scope` | `보기 옵션 적용 범위` | |
| KEY-1296 | `pref.viewScope.desc` | `Where the hidden/dot/folders-first toggles apply. The checkboxes in this window always apply to all panels regardless of scope.` | `숨김 파일·Dot 파일·폴더 우선 토글이 적용되는 범위입니다. 설정 창의 체크박스는 범위와 무관하게 전체에 적용됩니다.` | |
| KEY-1297 | `pref.viewScope.global` | `All panels` | `전체` | |
| KEY-1298 | `pref.viewScope.panel` | `Left/right panel` | `좌/우 패널` | |
| KEY-1299 | `pref.viewScope.tab` | `Active tab` | `활성 탭` | |
| KEY-1300 | `pref.termWrap` | `Terminal word wrap` | `터미널 줄 바꿈` | |
| KEY-1301 | `pref.termWrap.desc` | `Wrap lines to the view width. Off = fixed columns with horizontal scroll.` | `뷰 폭에 맞춰 줄을 바꿉니다. 끄면 고정 열+가로 스크롤.` | |
| KEY-1302 | `pref.termCols` | `Terminal columns (80~1000)` | `터미널 열 수(80~1000)` | |
| KEY-1303 | `pref.termCols.desc` | `Fixed column count when word wrap is off. Scroll horizontally with Shift+wheel.` | `줄 바꿈을 껐을 때의 고정 열 수. Shift+휠로 가로 스크롤합니다.` | |
| KEY-1304 | `pref.termTheme` | `Terminal theme` | `터미널 테마` | |
| KEY-1305 | `pref.termTheme.desc` | `System follows the app theme using the dark/light defaults below. Picking a specific theme pins it regardless of the app theme (a light terminal in dark mode works too).` | `시스템 = 앱 테마를 따라 아래 다크/라이트 기본 테마를 씁니다. 개별 테마를 고르면 앱 테마와 무관하게 고정됩니다(다크 모드에서 라이트 터미널도 가능).` | |
| KEY-1306 | `pref.termTheme.system` | `System (follow app theme)` | `시스템 (앱 테마 따름)` | |
| KEY-1307 | `pref.termTheme.dark` | `Dark default theme` | `다크 기본 테마` | |
| KEY-1308 | `pref.termTheme.light` | `Light default theme` | `라이트 기본 테마` | |
| KEY-1309 | `pref.termThemeDark` | `Default terminal theme in dark mode` | `다크 모드 기본 터미널 테마` | |
| KEY-1310 | `pref.termThemeDark.desc` | `Used when the app is dark (or when "Dark default theme" is selected above).` | `앱이 다크일 때(또는 위에서 "다크 기본 테마"를 골랐을 때) 쓰는 테마.` | |
| KEY-1311 | `pref.termThemeLight` | `Default terminal theme in light mode` | `라이트 모드 기본 터미널 테마` | |
| KEY-1312 | `pref.termThemeLight.desc` | `Used when the app is light (or when "Light default theme" is selected above).` | `앱이 라이트일 때(또는 위에서 "라이트 기본 테마"를 골랐을 때) 쓰는 테마.` | |
| KEY-1313 | `pref.termCopy` | `Text format to copy to the clipboard` | `클립보드에 복사할 텍스트 형식` | |
| KEY-1314 | `pref.termCopy.desc` | `Formats added when copying a terminal selection with Ctrl+C. Plain text is always included.` | `터미널 선택을 Ctrl+C로 복사할 때 함께 올릴 서식. 일반 텍스트는 항상 포함됩니다.` | P |
| KEY-1315 | `pref.termCopy.text` | `Plain text only` | `일반 텍스트만` | |
| KEY-1316 | `pref.termCopy.html` | `HTML` | `HTML` | |
| KEY-1317 | `pref.termCopy.rtf` | `RTF` | `RTF` | |
| KEY-1318 | `pref.termCopy.both` | `Both HTML and RTF` | `HTML 및 RTF 모두` | |
| KEY-1319 | `pref.sortCaseSensitive` | `Case-sensitive sort` | `대소문자 구분 정렬` | |
| KEY-1320 | `pref.sortCaseSensitive.desc` | `Sort case-sensitively - names starting with uppercase group first.` | `대소문자를 구분해 정렬합니다 — 대문자로 시작하는 항목이 위 그룹.` | |
| KEY-1321 | `pref.navUpAlign` | `Selection position after Go Up` | `위로 이동 시 선택 위치` | |
| KEY-1322 | `pref.navUpAlign.desc` | `Where the left folder is placed in the view after Alt+Up.` | `Alt+↑로 상위 폴더로 이동했을 때 떠난 폴더를 뷰의 어디에 둘지 정합니다.` | P |
| KEY-1323 | `pref.align.top` | `Top` | `상단` | |
| KEY-1324 | `pref.align.center` | `Center` | `중단` | |
| KEY-1325 | `pref.align.bottom` | `Bottom` | `하단` | |
| KEY-1326 | `ops.doneClosing` | `Done - closing shortly` | `완료 — 잠시 후 닫힙니다` | |
| KEY-1327 | `pref.cat.tabs` | `Tabs` | `탭` | |
| KEY-1328 | `pref.tabDblclick` | `Tab double-click action` | `탭 더블클릭 동작` | |
| KEY-1329 | `pref.tabDblclick.desc` | `What double-clicking a tab does. More options later.` | `탭을 더블클릭했을 때의 동작입니다. 옵션은 추후 추가됩니다.` | |
| KEY-1330 | `pref.tabDbl.close` | `Close tab` | `탭 닫기` | |
| KEY-1331 | `pref.tabDbl.pin` | `Pin tab` | `탭 고정` | |
| KEY-1332 | `pref.tabDbl.lock` | `Lock tab` | `탭 잠금` | |
| KEY-1333 | `tab.pin` | `Pin tab` | `탭 고정` | |
| KEY-1334 | `tab.unpin` | `Unpin tab` | `탭 고정 해제` | |
| KEY-1335 | `pref.taScope` | `Type-ahead search scope` | `타입어헤드 검색 범위` | |
| KEY-1336 | `pref.taScope.desc` | `Where prefix typing searches.` | `접두사 입력이 검색하는 범위입니다.` | |
| KEY-1337 | `pref.taScope.global` | `Global first` | `전체에서 처음부터` | |
| KEY-1338 | `pref.taScope.level` | `Current level` | `현재 계층(형제)` | |
| KEY-1339 | `pref.taScope.visible` | `Visible stream (default)` | `가시 스트림(기본)` | |
| KEY-1340 | `pref.taReset` | `Type-ahead input reset (ms)` | `타입어헤드 입력 리셋(ms)` | |
| KEY-1341 | `pref.taReset.desc` | `Idle time before the typed prefix resets (200~10000).` | `입력이 없으면 접두사를 초기화하는 시간(200~10000).` | |
| KEY-1342 | `pref.taPos` | `Type-ahead badge position` | `타입어헤드 배지 위치` | |
| KEY-1343 | `pref.taPos.desc` | `Where the find badge appears in the list.` | `찾기 배지가 목록의 어디에 표시될지 정합니다.` | |
| KEY-1344 | `pref.taPos.tl` | `Top left` | `좌상` | X |
| KEY-1345 | `pref.taPos.tc` | `Top center` | `중상` | X |
| KEY-1346 | `pref.taPos.tr` | `Top right` | `우상` | X |
| KEY-1347 | `pref.taPos.ml` | `Middle left` | `좌중` | X |
| KEY-1348 | `pref.taPos.mc` | `Middle center` | `중중` | X |
| KEY-1349 | `pref.taPos.mr` | `Middle right` | `우중` | X |
| KEY-1350 | `pref.taPos.bl` | `Bottom left (default)` | `좌하(기본)` | X |
| KEY-1351 | `pref.taPos.bc` | `Bottom center` | `중하` | X |
| KEY-1352 | `pref.taPos.br` | `Bottom right` | `우하` | X |
| KEY-1353 | `pref.fsEnabled` | `Enable fast scroll` | `고속 스크롤 사용` | |
| KEY-1354 | `pref.fsEnabled.desc` | `Rapid wheel notches or ↑/↓ auto-repeat multiply the step (precision touchpads excluded).` | `휠 노치나 ↑/↓ 자동 반복이 짧은 간격으로 이어지면 이동량을 배수로 키웁니다(정밀 터치패드 제외).` | |
| KEY-1355 | `pref.fsGridExtra` | `File list one step faster` | `파일 목록은 한 단계 더 빠르게` | |
| KEY-1356 | `pref.fsGridExtra.desc` | `In the file list the multiplier grows one event earlier and the cap is doubled (same rule as the nexa-sql result grid).` | `파일 목록에서는 배수가 한 번 먼저 오르고 상한이 두 배가 됩니다(nexa-sql 결과 그리드와 같은 규약).` | P |
| KEY-1357 | `pref.fsStep` | `Events per multiplier step` | `배수 상승 간격(회)` | |
| KEY-1358 | `pref.fsStep.desc` | `After this many consecutive events in one direction the multiplier grows by 1 (1–50).` | `같은 방향으로 이 횟수만큼 연속되면 배수가 1 올라갑니다(1~50).` | |
| KEY-1359 | `pref.fsMax` | `Maximum multiplier` | `최대 배수` | |
| KEY-1360 | `pref.fsMax.desc` | `Upper bound of the multiplier (1–32).` | `배수의 상한입니다(1~32).` | |
| KEY-1361 | `pref.fsWindow` | `Streak window (ms)` | `연속 판정 간격(ms)` | |
| KEY-1362 | `pref.fsWindow.desc` | `Only events within this interval count as consecutive (20–2000).` | `이 시간 안에 이어진 입력만 연속으로 봅니다(20~2000).` | |
| KEY-1363 | `pref.fsHud` | `Show speed badge` | `속도 배지 표시` | |
| KEY-1364 | `pref.fsHud.desc` | `While accelerating, a ×N badge is shown in the scrolled area; it fades when you stop.` | `가속 중이면 그 영역에 ×N 배지를 보이고, 멈추면 서서히 사라집니다.` | |
| KEY-1365 | `pref.fsHudPos` | `Speed badge position` | `속도 배지 위치` | |
| KEY-1366 | `pref.fsHudPos.desc` | `Where the badge appears in the scrolled area (3×3).` | `배지가 스크롤 영역의 어디에 표시될지 정합니다(3×3).` | |
| KEY-1367 | `pref.fsHudHold` | `Badge hold (ms)` | `배지 유지 시간(ms)` | |
| KEY-1368 | `pref.fsHudHold.desc` | `How long the badge stays fully visible after the last accelerated event (0–10000).` | `마지막 가속 입력 뒤 배지가 그대로 보이는 시간입니다(0~10000).` | |
| KEY-1369 | `pref.fsHudFade` | `Badge fade (ms)` | `배지 사라짐 시간(ms)` | |
| KEY-1370 | `pref.fsHudFade.desc` | `How long the badge takes to fade out after the hold (0–10000).` | `유지 시간이 지난 뒤 서서히 사라지는 시간입니다(0~10000).` | |
| KEY-1371 | `pref.taSpecial` | `Include special characters (filename-safe)` | `특수문자 포함(파일명 안전)` | |
| KEY-1372 | `pref.taSpecial.desc` | `Off = letters and digits only.` | `끄면 영숫자만 접두사로 입력됩니다.` | |
| KEY-1373 | `pref.taSpace` | `Include Space (while typing a prefix)` | `공백 포함(접두사 입력 중)` | |
| KEY-1374 | `pref.taSpace.desc` | `Space toggles selection when no prefix is active.` | `접두사가 없을 때의 공백은 선택 토글로 동작합니다.` | |
| KEY-1375 | `pref.taBackspace` | `Backspace erases last character` | `Backspace로 마지막 글자 지우기` | |
| KEY-1376 | `pref.taBackspace.desc` | `Off = Backspace is ignored during type-ahead.` | `끄면 타입어헤드 중 Backspace를 무시합니다.` | |
| KEY-1377 | `pref.cat.listGeneral` | `View & Sort` | `보기·정렬` | |
| KEY-1378 | `pref.cat.typeahead` | `Type-ahead` | `타입어헤드` | |
| KEY-1379 | `pref.cat.scroll` | `Fast Scroll` | `고속 스크롤` | |
| KEY-1380 | `ops.close` | `Close` | `닫기` | |

#### 내 PC · 글꼴 슬롯 · 패널/정보 모드 (`en.lang:418-445`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1381 | `nav.mypc` | `This PC` | `내 PC` | P |
| KEY-1382 | `col.total` | `Total size` | `전체 크기` | |
| KEY-1383 | `col.free` | `Free space` | `사용 가능한 공간` | |
| KEY-1384 | `drive.freeOf` | `{1} free of {0}` | `{0} 중 {1} 사용 가능` | |
| KEY-1385 | `pref.baseFont` | `Base font` | `기본 글꼴` | |
| KEY-1386 | `pref.baseFont.desc` | `Used where nothing specific is set - menus, tabs, path, descriptions, dock panes.` | `특정 슬롯이 없는 모든 곳 — 메뉴·탭·경로 바·설명·하단 도크.` | |
| KEY-1387 | `pref.consoleFont.desc` | `Terminal only. Separate two or more with commas for fallback order.` | `터미널 전용. 쉼표로 구분하면 폴백 순서가 됩니다.` | |
| KEY-1388 | `pref.ctxFont` | `Context menu font` | `우클릭 메뉴 글꼴` | |
| KEY-1389 | `pref.ctxFont.desc` | `Reserved for custom-drawn menus - current right-click menus follow the OS font.` | `자체 그리기 메뉴 전환 시 적용 예정 — 현재 우클릭 메뉴는 OS 글꼴을 따릅니다.` | P |
| KEY-1390 | `pref.statusFont` | `Status bar font` | `상태바 글꼴` | |
| KEY-1391 | `pref.statusFont.desc` | `Bottom status bar.` | `하단 상태바.` | |
| KEY-1392 | `pref.listFont` | `File list font` | `파일 목록 글꼴` | |
| KEY-1393 | `pref.listFont.desc` | `Left/right panel file lists + column headers (shared family/size).` | `좌/우 패널 파일 목록 + 컬럼 헤더(패밀리·크기 공유).` | |
| KEY-1394 | `pref.folderBold` | `Bold folder names` | `폴더 이름 굵게` | |
| KEY-1395 | `pref.folderBold.desc` | `File list folders are drawn semi-bold.` | `파일 목록의 폴더 이름을 세미볼드로 표시합니다.` | |
| KEY-1396 | `pref.hdrBold` | `Bold column headers` | `컬럼 헤더 굵게` | |
| KEY-1397 | `pref.hdrBold.desc` | `Decoration only - same family/size as the file list.` | `장식만 — 패밀리·크기는 파일 목록과 동일.` | |
| KEY-1398 | `pref.hdrItalic` | `Italic column headers` | `컬럼 헤더 이탤릭` | |
| KEY-1399 | `pref.hdrItalic.desc` | `Decoration only - same family/size as the file list.` | `장식만 — 패밀리·크기는 파일 목록과 동일.` | |
| KEY-1400 | `menu.view.panelDual` | `Dual File Panels` | `듀얼 파일 패널` | |
| KEY-1401 | `menu.view.panelSingle` | `Single File Panel` | `싱글 파일 패널` | |
| KEY-1402 | `menu.view.infoDual` | `Dual Info Panes` | `듀얼 정보 패널` | |
| KEY-1403 | `menu.view.infoSingle` | `Single Info Pane` | `싱글 정보 패널` | |
| KEY-1404 | `menu.view.colWidthSync` | `Synchronize Column Widths` | `컬럼 너비 동기화` | |
| KEY-1405 | `status.infoLocked` | `Info pane is fixed to single in single file panel mode` | `싱글 파일 패널에서는 정보 패널이 싱글로 고정됩니다` | |

#### 일괄 이름 변경 v2 (`en.lang:446-496`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1406 | `bulk.kind.date` | `Add Date` | `날짜 삽입` | |
| KEY-1407 | `bulk.scope.name` | `Name` | `이름` | |
| KEY-1408 | `bulk.scope.nameext` | `Name with extension` | `이름+확장자` | |
| KEY-1409 | `bulk.scope.ext` | `Extension` | `확장자` | |
| KEY-1410 | `bulk.scope.extdot` | `Extension with dot` | `확장자(점 포함)` | |
| KEY-1411 | `bulk.lbl.applyTo` | `Apply to:` | `적용 대상:` | |
| KEY-1412 | `bulk.preset.saveSeq` | `Save Renaming Sequence…` | `이름 변경 순서 저장…` | |
| KEY-1413 | `bulk.preset.editSeq` | `Edit Renaming Sequences…` | `이름 변경 순서 편집…` | |
| KEY-1414 | `bulk.preset.savePrompt` | `Preset name` | `프리셋 이름` | X |
| KEY-1415 | `bulk.preset.savedSeq` | `Saved Renaming Sequence` | `저장된 이름 변경 순서` | |
| KEY-1416 | `bulk.rename` | `Rename` | `이름 변경` | |
| KEY-1417 | `bulk.preset.delete` | `Delete` | `삭제` | X |
| KEY-1418 | `bulk.preset.close` | `Close` | `닫기` | X |
| KEY-1419 | `bulk.preset.ok` | `OK` | `확인` | |
| KEY-1420 | `bulk.preset.cancel` | `Cancel` | `취소` | |
| KEY-1421 | `bulk.grid.name` | `Name` | `이름` | X |
| KEY-1422 | `bulk.grid.apply` | `Apply` | `적용` | X |
| KEY-1423 | `bulk.grid.before` | `Before` | `이전` | |
| KEY-1424 | `bulk.grid.after` | `After` | `이후` | |
| KEY-1425 | `bulk.lbl.mode` | `Mode:` | `모드:` | |
| KEY-1426 | `bulk.lbl.matchCase` | `Case sensitive:` | `대소문자 일치:` | |
| KEY-1427 | `bulk.lbl.find` | `Replace text:` | `찾기:` | |
| KEY-1428 | `bulk.lbl.findRx` | `Replace RegEx:` | `정규식:` | |
| KEY-1429 | `bulk.lbl.with` | `with text:` | `바꾸기:` | |
| KEY-1430 | `bulk.lbl.pos` | `Position:` | `위치:` | |
| KEY-1431 | `bulk.lbl.text` | `Text:` | `텍스트:` | |
| KEY-1432 | `bulk.lbl.caseTo` | `Change case to:` | `케이스:` | |
| KEY-1433 | `bulk.lbl.padding` | `Padding:` | `자릿수:` | |
| KEY-1434 | `bulk.lbl.start` | `Start value:` | `시작 값:` | |
| KEY-1435 | `bulk.lbl.step` | `Step value:` | `증가 값:` | |
| KEY-1436 | `bulk.lbl.type` | `Type:` | `종류:` | |
| KEY-1437 | `bulk.lbl.fmt` | `Format:` | `포맷:` | |
| KEY-1438 | `bulk.lbl.wrap` | `Wrap:` | `감싸기:` | X |
| KEY-1439 | `bulk.lbl.prefix` | `Prefix:` | `접두:` | |
| KEY-1440 | `bulk.lbl.suffix` | `Suffix:` | `접미:` | |
| KEY-1441 | `bulk.lbl.range` | `Range:` | `구간:` | X |
| KEY-1442 | `bulk.lbl.dest` | `Destination:` | `대상:` | X |
| KEY-1443 | `bulk.lbl.ext` | `Extension:` | `확장자:` | X |
| KEY-1444 | `bulk.mode.all` | `Every occurrence` | `모든 일치` | |
| KEY-1445 | `bulk.mode.first` | `First occurrence` | `첫 번째 일치` | |
| KEY-1446 | `bulk.mode.last` | `Last occurrence` | `마지막 일치` | |
| KEY-1447 | `bulk.mode.entire` | `Entire text` | `전체 교체` | |
| KEY-1448 | `bulk.dirStart` | `From start` | `앞에서부터` | X |
| KEY-1449 | `bulk.dirEnd` | `From end` | `뒤에서부터` | X |
| KEY-1450 | `bulk.wrapPre` | `Prefix (around value)` | `값 앞 텍스트` | X |
| KEY-1451 | `bulk.wrapSuf` | `Suffix (around value)` | `값 뒤 텍스트` | X |
| KEY-1452 | `bulk.date.modified` | `Modification date` | `수정한 날짜` | |
| KEY-1453 | `bulk.date.created` | `Creation date` | `만든 날짜` | |
| KEY-1454 | `bulk.date.fmt` | `Format (yyyy-MM-dd, MMM, HHmmss...)` | `포맷(yyyy-MM-dd, MMM, HHmmss…)` | X |
| KEY-1455 | `bulk.count` | `{0} items will be renamed.` | `{0}개 항목이 변경됩니다.` | |

#### 전송 진행 창 · DnD (`en.lang:497-506`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1456 | `ops.fileCount` | `File {0}/{1}` | `파일 {0}/{1}` | |
| KEY-1457 | `ops.itemBusy` | `Item is being transferred - available when complete` | `전송 중인 항목 — 완료 후 열 수 있습니다` | |
| KEY-1458 | `pref.cat.transfer` | `File Transfer` | `파일 전송` | |
| KEY-1459 | `pref.transferClose` | `Close delay after transfer (ms, 0-10000)` | `완료 창 닫기 시간(ms, 0~10000)` | |
| KEY-1460 | `pref.transferClose.desc` | `How long (in milliseconds) the progress window stays open after a copy/move completes. 0 hides the progress window entirely.` | `복사/이동 완료 후 진행 창이 닫히기까지의 시간(밀리초)입니다. 0이면 진행 창을 표시하지 않습니다.` | |
| KEY-1461 | `pref.dndHover` | `Drag hover switch delay (ms, 200-10000)` | `드래그 호버 전환 시간(ms, 200~10000)` | |
| KEY-1462 | `pref.dndHover.desc` | `How long (in milliseconds) to hover over a tab or a collapsed folder while dragging before it switches or expands. Default 3000 ms (3 s).` | `드래그 중 탭이나 접힌 폴더 위에 머물렀을 때 탭 전환·폴더 펼침까지 기다리는 시간(밀리초)입니다. 기본 3000ms(3초).` | |
| KEY-1463 | `del.progress` | `Deleting...` | `삭제 중...` | |

#### 압축 미리보기 (`en.lang:508-543`)

| ID | 키 | en | ko | 비고 |
|---|---|---|---|---|
| KEY-1464 | `archive.summary` | `{0} archive · {1} files · {2} folders` | `{0} 압축 · 파일 {1} · 폴더 {2}` | |
| KEY-1465 | `archive.sizes` | `original {0} · packed {1} ({2} saved)` | `원본 {0} · 압축 {1}({2} 절감)` | |
| KEY-1466 | `archive.encrypted` | `contains password-protected entries (names visible, contents locked)` | `암호로 보호된 항목이 있습니다(이름은 보이고 내용은 잠김)` | |
| KEY-1467 | `archive.solid` | `solid archive (entries cannot be extracted one by one)` | `솔리드 압축 — 항목 하나만 따로 풀 수 없습니다` | |
| KEY-1468 | `archive.multivolume` | `multi-volume (split) archive` | `분할(멀티볼륨) 압축 파일` | |
| KEY-1469 | `archive.truncated` | `too many entries — only the first ones are listed` | `항목이 너무 많아 앞부분만 표시합니다` | |
| KEY-1470 | `archive.comment` | `comment: {0}` | `주석: {0}` | |
| KEY-1471 | `archive.openHint` | `F3 / the dock's expand button opens the archive grid` | `F3 · 도크의 크게 보기(↗)로 압축 그리드를 엽니다` | P |
| KEY-1472 | `archive.more` | `… and {0} more entries` | `… 외 {0}개 항목` | |
| KEY-1473 | `archive.needPassword` | `password required to read this archive's listing` | `목록을 읽으려면 암호가 필요합니다` | |
| KEY-1474 | `archive.needPlugin` | `listing a {0} archive needs the {1} codec — install a preview plugin` | `{0} 목록은 {1} 코덱이 필요합니다 — 미리보기 플러그인을 설치하세요` | |
| KEY-1475 | `archive.failed` | `could not read the archive: {0}` | `압축 파일을 읽지 못했습니다: {0}` | |
| KEY-1476 | `archive.notArchive` | `not a known archive format` | `알려진 압축 형식이 아닙니다` | |
| KEY-1477 | `archive.window.title` | `{0} — Archive` | `{0} — 압축 미리보기` | |
| KEY-1478 | `archive.col.name` | `Name` | `이름` | |
| KEY-1479 | `archive.col.path` | `Path` | `경로` | |
| KEY-1480 | `archive.col.size` | `Size` | `크기` | |
| KEY-1481 | `archive.col.packed` | `Packed` | `압축 크기` | |
| KEY-1482 | `archive.col.ratio` | `Ratio` | `압축률` | |
| KEY-1483 | `archive.col.method` | `Method` | `방식` | |
| KEY-1484 | `archive.col.modified` | `Modified` | `수정한 날짜` | |
| KEY-1485 | `archive.col.flags` | `Flags` | `표시` | |
| KEY-1486 | `archive.flag.dir` | `folder` | `폴더` | |
| KEY-1487 | `archive.flag.locked` | `locked` | `잠김` | |
| KEY-1488 | `archive.flag.unsafe` | `unsafe path` | `위험 경로` | |
| KEY-1489 | `archive.status` | `{0} · {1} items · original {2} · packed {3}` | `{0} · 항목 {1}개 · 원본 {2} · 압축 {3}` | |
| KEY-1490 | `archive.pw.title` | `Archive password` | `압축 파일 암호` | |
| KEY-1491 | `archive.pw.prompt` | `"{0}" is protected. Enter the password to read its contents.` | `'{0}'은(는) 보호되어 있습니다. 내용을 읽으려면 암호를 입력하세요.` | |
| KEY-1492 | `archive.pw.label` | `Password` | `암호` | |
| KEY-1493 | `archive.pw.show` | `Show password` | `암호 표시` | |
| KEY-1494 | `archive.pw.remember` | `Remember for this session only` | `이번 세션에만 기억` | X |
| KEY-1495 | `archive.pw.wrong` | `Wrong password — try again` | `암호가 맞지 않습니다 — 다시 입력하세요` | |
| KEY-1496 | `archive.pw.ok` | `OK` | `확인` | |
| KEY-1497 | `archive.pw.cancel` | `Cancel` | `취소` | |
| KEY-1498 | `archive.pw.note` | `The password is passed to the reader only — never written to disk or logs.` | `입력한 암호는 읽기에만 전달되며 디스크·로그에 기록되지 않습니다.` | |

일본어(`ja.lang`)는 같은 키 498개를 모두 가진다(`grep -c` = 498 · 파리티 시험 `nexa-dir2/crates/nexa-app/src/i18n.rs:217-232`). 값은 이 문서에 옮기지 않았다 — 자원 파일을 그대로 가져가면 된다.

i18n이 아닌 **하드코딩 사용자 문자열**(번역 대상 밖 · 이식 때 확인할 것): 런처 시드 라벨 `VS Code` · `pwsh` · `PowerShell` · `cmd`(`nexa-dir2/crates/nexa-app/src/launcher.rs:50-96`) · 클라우드 후보 라벨 `OneDrive – …` · `Google Drive (X:)` · `Dropbox – Personal/Business`(`nexa-dir2/crates/nexa-app/src/cloud.rs:96` · `:179` · `:203`) · 서비스 표시명(`nexa-dir2/crates/nexa-app/src/oauth.rs:56` · `:76` · `:106`) · 언어 이름 `English` · `한국어` · `日本語`(`nexa-dir2/crates/nexa-app/src/i18n.rs:123-127`) · 터미널 스킴 이름(`nexa-dir2/crates/nexa-term/src/lib.rs:153-491`) · 표준 오류 출력 한국어 문장(`nexa-dir2/crates/nexa-app/src/win.rs:1392` · `:9684` · `nexa-dir2/crates/nexa-app/src/main.rs:112` · `:119-122`).

---

## 3. 영속 파일

### 3-1. 데이터 폴더 결정 규칙

`data_dir()`(`nexa-dir2/crates/nexa-app/src/config.rs:362-373`) — 프로세스당 1회 판정:

1. `<exe 폴더>\data`에 폴더 생성 + 쓰기 프로브가 성공하면 그곳(포터블).
2. 실패하면 `%LOCALAPPDATA%\NexaDir\data`(설치형). 구 폴더 `%LOCALAPPDATA%\NexaDir2\data`가 있고 새 폴더가 없으면 rename으로 이주(`nexa-dir2/crates/nexa-app/src/config.rs:381-402`).
3. `LOCALAPPDATA`도 없으면 1번 후보를 그대로 쓴다.

쓰기 방식: `<이름>.<pid>.tmp`에 쓰고 `sync_all` 뒤 rename(`nexa-dir2/crates/nexa-app/src/config.rs:994-1011`).

### 3-2. 파일 목록

| ID | 경로 | 형식 | 내용 | 쓰는 때 | 근거 |
|---|---|---|---|---|---|
| KEY-201 | `<data>\settings.cfg` | `key=value` 텍스트(UTF-8) | 앱 설정 전체(§1-1) | 변경 즉시 + 종료 | `nexa-dir2/crates/nexa-app/src/config.rs:1016` · `nexa-dir2/crates/nexa-app/src/win.rs:1414-1416` · `:6942-6945` · `:9680` |
| KEY-202 | `<data>\session.cfg` | `key=value` 텍스트 | 패널 · 탭 · 펼침 · 컬럼(§1-2) | 탭 변경 1초 디바운스 + 종료 | `nexa-dir2/crates/nexa-app/src/config.rs:1017` · `nexa-dir2/crates/nexa-app/src/win.rs:1417-1419` · `:9427` · `:9681` |
| KEY-203 | `<data>\settings.txt` · `<data>\session.txt`(구 이름 ~0.5.0) | 위와 같음 | 새 이름이 없을 때만 읽고, 새 이름 저장 성공 뒤 삭제 | 읽기 전용 → 삭제 | `nexa-dir2/crates/nexa-app/src/config.rs:1019-1032` · `nexa-dir2/crates/nexa-app/src/win.rs:9683` |
| KEY-204 | `<data>\crash.txt` | 텍스트(panic 정보 1건 · 덮어쓰기) | panic 위치 · 메시지(릴리스는 `panic=abort`) | panic 후크 | `nexa-dir2/crates/nexa-app/src/win.rs:1404-1413` |
| KEY-205 | `<data>\lang\<code>.lang` | properties 텍스트 | 사용자 번역 덮어쓰기 · 추가 언어 | 앱은 읽기만(사용자가 넣음) | `nexa-dir2/crates/nexa-app/src/i18n.rs:97-151` |
| KEY-206 | `<data>\plugins\*.wasm` | WASM(`wasm32-unknown-unknown`) | 사용자 설치 미리보기 플러그인(우선) | 앱은 읽기만 | `nexa-dir2/crates/nexa-app/src/preview/mod.rs:263-282` · `nexa-dir2/crates/nexa-app/src/preview/wasm.rs:373-383` |
| KEY-207 | `<exe 폴더>\plugins\*.wasm` | WASM | 동봉 플러그인(같은 id면 KEY-206이 이김) | 앱은 읽기만 | `nexa-dir2/crates/nexa-app/src/preview/mod.rs:273-281` · 설치 `nexa-dir2/installer/nexa.iss:70-75` |
| KEY-208 | `<data>\renames\<이름>.cfg` | 프리셋 텍스트(§1-4) | 일괄 이름 변경 순서 | 저장 · 삭제 시 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:234-236` · `:1378-1386` · `:1675` · `:1911-1927` |
| KEY-209 | `<data>\secrets\cloud<N>.tok` | DPAPI blob의 소문자 hex 1줄 | 클라우드 refresh 토큰(연결 인덱스 N과 짝 · 상한 32) | 연결 성공 · 해제 시 | `nexa-dir2/crates/nexa-app/src/secret.rs:13-71` · 암호화 `:87-121` |
| KEY-210 | `<data>\.w<pid>` | 빈 파일 | 쓰기 가능 프로브(즉시 삭제) | 기동 1회 | `nexa-dir2/crates/nexa-app/src/config.rs:405-417` |
| KEY-211 | `<data>\<이름>.<pid>.tmp` | 임시 | 원자적 저장 중간 파일(rename으로 사라짐) | 매 저장 | `nexa-dir2/crates/nexa-app/src/config.rs:997` |
| KEY-212 | `%LOCALAPPDATA%\NexaDir2\data\`(구 설치형 폴더) | 폴더 | `NexaDir\data`로 1회 이주 | 기동 1회 | `nexa-dir2/crates/nexa-app/src/config.rs:378-399` |
| KEY-213 | `%TEMP%\NexaDir\dnd-<pid>-<seq>\<i>\` | 폴더 | DnD로 들어온 임시 폴더 출신 항목 확보(스테이징) | 드롭 시 | `nexa-dir2/crates/nexa-app/src/dnd.rs:111-119` · 판정 `nexa-dir2/crates/nexa-app/src/win.rs:2209-2217` |
| KEY-214 | `%TEMP%\NexaDir\vpaste-<pid>-<seq>\` | 폴더 | 가상 붙여넣기 스테이징 | 붙여넣기 시 | `nexa-dir2/crates/nexa-app/src/win.rs:2185-2194` |
| KEY-215 | `%TEMP%\NexaDir\cloud\` | 폴더 | 클라우드 파일 더블클릭 열기용 내려받기 | 열 때 | `nexa-dir2/crates/nexa-app/src/win.rs:4181-4184` |
| KEY-216 | `%TEMP%\NexaDir\xcopy\` | 폴더 | 클라우드 간 복사 중계 | 복사 시 | `nexa-dir2/crates/nexa-app/src/cloudfs.rs:613-614` |
| KEY-217 | `%TEMP%\nexa-preview\d<해시16자리>.bmp` | 32비트 BMP | 플러그인 SVG 렌더 캐시(256KB 초과 SVG는 거부) | 미리보기 시 | `nexa-dir2/crates/nexa-app/src/preview/mod.rs:101-134` |
| KEY-218 | `%TEMP%\nexa-ctxmenu-timing.log` | 텍스트(덧붙이기) | 컨텍스트 메뉴 단계별 시간(`NEXA_CTX_TIMING=1`일 때만) | 메뉴 열 때 | `nexa-dir2/crates/nexa-app/src/shellmenu.rs:122-187` |
| KEY-219 | `%APPDATA%\Dropbox\info.json`(없으면 `%LOCALAPPDATA%\…`) | JSON(남의 파일) | Dropbox 동기화 폴더 탐지 | 읽기만 | `nexa-dir2/crates/nexa-app/src/cloud.rs:186-211` |
| KEY-220 | 내장 `lang/en.lang` · `ko.lang` · `ja.lang` | properties(빌드 임베드) | 내장 문자열 | 빌드 시 | `nexa-dir2/crates/nexa-app/src/i18n.rs:14-17` |
| KEY-221 | 내장 `assets/nexa-dir.ico` + 버전 리소스 | ICO · `.rc` | exe 아이콘 · 버전 정보(Windows 타깃만) | 빌드 시 | `nexa-dir2/crates/nexa-app/build.rs:13-60` |
| KEY-222 | 설치 위치 `{autopf}\Nexa Dir` · AppId `{7E4B1C9D-…}` | Inno Setup | 설치형 배치(쓰기 불가 → KEY-212 규칙으로 데이터 분리) | 설치 시 | `nexa-dir2/installer/nexa.iss:26-32` |

파일로 남지 **않는** 것: 로그 파일(없음 — `eprintln!`만) · 실행 취소 기록 · 창 배치 · 최근 경로 목록(없음) · 라이선스 파일(dir2에는 라이선스 발급/검증 코드가 없다 — `license` Grep 결과는 `about.license` 문자열과 `Cargo.toml`뿐).

---

## 4. 환경 · 인자 · 레지스트리

### 4-1. 명령행 인자

| ID | 대상 | 인자 | 동작 | 근거 |
|---|---|---|---|---|
| KEY-301 | 앱(`nexa-app`) | `argv[1]` = 시작 폴더 경로(선택) | 있으면 세션 복원 대신 두 패널 모두 그 경로로 시작 · 없으면 세션 · 세션도 없으면 `%USERPROFILE%` → `C:\` | `nexa-dir2/crates/nexa-app/src/win.rs:1381-1388` · `:1440-1475` |
| KEY-302 | 앱 | 그 밖의 옵션(`--…`) | **없음** — `argv[1]`만 읽는다. 단일 인스턴스 처리도 없다(`CreateMutex` Grep 0건) | `nexa-dir2/crates/nexa-app/src/win.rs:1383` · `:1441` |
| KEY-303 | 예제 `audit_enum` | `<dir> [reps]` | 열거 성능 점검(개발용) | `nexa-dir2/crates/nexa-vfs/examples/audit_enum.rs:8-9` |
| KEY-304 | 예제 `ctxmenu_probe` · `ctxmenu_handlers` · `preview_image` | 경로 · 반복 수 · 플래그 | 컨텍스트 메뉴/미리보기 점검(개발용 · Windows 전용) | `nexa-dir2/crates/nexa-app/examples/ctxmenu_probe.rs:162-172` · `ctxmenu_handlers.rs:77` · `preview_image.rs:162` |

### 4-2. 환경 변수(실행 시)

| ID | 변수 | 쓰임 | OS 분기 메모 | 근거 |
|---|---|---|---|---|
| KEY-305 | `USERPROFILE` | 시작 폴더 폴백 · 최후 루트 후보 | macOS · Linux = `HOME` | `nexa-dir2/crates/nexa-app/src/win.rs:1386` · `nexa-dir2/crates/nexa-app/src/panel.rs:22` |
| KEY-306 | `LOCALAPPDATA` | 설치형 데이터 폴더 · VS Code 탐색 · Dropbox 탐지 폴백 | `nexa_conf::user_config_dir`로 대체 | `nexa-dir2/crates/nexa-app/src/config.rs:385` · `nexa-dir2/crates/nexa-app/src/launcher.rs:22` · `nexa-dir2/crates/nexa-app/src/cloud.rs:190` |
| KEY-307 | `APPDATA` | Dropbox `info.json` 위치 | macOS · Linux = `~/.dropbox/info.json`(추정 — 확인 필요) | `nexa-dir2/crates/nexa-app/src/cloud.rs:190-191` |
| KEY-308 | `OneDrive` | OneDrive 폴더 폴백(레지스트리 계정 키가 없을 때) | Windows 전용 | `nexa-dir2/crates/nexa-app/src/cloud.rs:102-110` |
| KEY-309 | `PATH` | 터미널 기본 셸 탐색(`pwsh.exe` → `powershell.exe`) · 런처 exe 탐색 | 상대 경로 항목은 제외(플랜팅 방지) | `nexa-dir2/crates/nexa-app/src/conpty.rs:281-296` · `nexa-dir2/crates/nexa-app/src/launcher.rs:38-45` |
| KEY-310 | `SystemRoot` | 최후 셸 `%SystemRoot%\System32\cmd.exe` · Windows PowerShell 경로 | Unix = `$SHELL` → `/bin/sh` | `nexa-dir2/crates/nexa-app/src/conpty.rs:297-300` · `nexa-dir2/crates/nexa-app/src/launcher.rs:52-57` |
| KEY-311 | `ComSpec` | 런처 시드 `cmd` 경로 | Windows 전용 | `nexa-dir2/crates/nexa-app/src/launcher.rs:60-66` |
| KEY-312 | `ProgramFiles` · `ProgramFiles(x86)` | VS Code 시드 탐색 | macOS = `/Applications/Visual Studio Code.app` · Linux = PATH의 `code`(제안) | `nexa-dir2/crates/nexa-app/src/launcher.rs:19-31` |
| KEY-313 | `NEXA_CTX_TIMING`(=`1`) | 컨텍스트 메뉴 계측 켜기 → KEY-218 기록 | 진단용 | `nexa-dir2/crates/nexa-app/src/shellmenu.rs:122-125` |
| KEY-314 | 임의 이름(경로 입력 확장) | 경로바 입력의 `%VAR%` · `$env:VAR` · `${env:VAR}` 치환(미정의는 원문 유지) | Unix는 `$VAR` · `${VAR}` · `~` 표기를 더해야 한다 | `nexa-dir2/crates/nexa-app/src/pathinput.rs:9-121` |
| KEY-315 | `TEMP`/`TMP`(`std::env::temp_dir`) | 스테이징 · 캐시 · 계측 로그 | 표준 라이브러리가 OS별 처리 | `nexa-dir2/crates/nexa-app/src/dnd.rs:114` · `nexa-dir2/crates/nexa-app/src/win.rs:2189` · `:4183` |
| KEY-316 | (변수 아님) `std::env::current_exe` | 포터블 `data\` · 동봉 `plugins\` 기준 | macOS `.app` 번들 · Homebrew 자리는 `nexa_conf::is_replaced_on_upgrade`로 걸러야 한다 | `nexa-dir2/crates/nexa-app/src/config.rs:365-369` · `nexa-dir2/crates/nexa-app/src/preview/mod.rs:273-275` |
| KEY-317 | 시험 전용 `NEXA_T1` · `NEXA_T2` | 경로 확장 단위 시험 | — | `nexa-dir2/crates/nexa-app/src/pathinput.rs:180` · `:197` |

### 4-3. 환경 변수(빌드 시)

| ID | 변수 | 쓰임 | 근거 |
|---|---|---|---|
| KEY-318 | `CARGO_PKG_VERSION` | 정보 창 버전 · exe 버전 리소스 · `CORE_VERSION` | `nexa-dir2/crates/nexa-app/src/about.rs:241` · `nexa-dir2/crates/nexa-app/build.rs:38` · `nexa-dir2/crates/nexa-core/src/lib.rs:9` · `nexa-dir2/crates/nexa-app/src/main.rs:121` |
| KEY-319 | `CARGO_CFG_TARGET_OS` · `CARGO_MANIFEST_DIR` · `OUT_DIR` | 리소스 임베드(Windows 타깃만) | `nexa-dir2/crates/nexa-app/build.rs:17-22` |
| KEY-320 | `PATH` · `ProgramFiles(x86)` · `ProgramFiles` | `rc.exe`(Windows SDK) 탐색 — 없으면 경고 후 생략 | `nexa-dir2/crates/nexa-app/build.rs:133-161` |

### 4-4. 레지스트리

| ID | 키 · 값 | 읽기/쓰기 | 쓰임 | 대체(macOS · Linux) | 근거 |
|---|---|---|---|---|---|
| KEY-321 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` · `AppsUseLightTheme`(DWORD) | 읽기 | `theme=system`일 때 OS 라이트/다크 판정(조회 실패 = 라이트) | macOS `AppleInterfaceStyle` · Linux 포털 `color-scheme`(nexa-sql 구현 재사용 — 위치는 미확인) | `nexa-dir2/crates/nexa-app/src/win.rs:288-303` |
| KEY-322 | `HKCU\Software\Microsoft\OneDrive\Accounts\*` · `UserFolder` · `DisplayName` · `UserEmail`(REG_SZ) | 읽기 | OneDrive 동기화 폴더 · 라벨 탐지(다계정) | macOS `~/Library/CloudStorage/OneDrive-*`(추정) | `nexa-dir2/crates/nexa-app/src/cloud.rs:48-148` |
| KEY-323 | (쓰기) | **없음** | 앱은 레지스트리에 아무것도 쓰지 않는다(`RegSet*` · `RegCreate*` Grep 0건) · 설치기에도 `[Registry]` 절 없음(Grep 0건) | — | `nexa-dir2/installer/nexa.iss` |
| KEY-324 | `HKCR\*` · `AllFilesystemObjects` 등 | 읽기(예제만) | 컨텍스트 메뉴 처리기 CLSID 나열(개발용 예제 · 앱 본체 아님) | — | `nexa-dir2/crates/nexa-app/examples/ctxmenu_handlers.rs:4` |

### 4-5. 레지스트리 밖의 OS 설정 조회(크로스플랫폼 분기점)

| ID | API | 쓰임 | 근거 |
|---|---|---|---|
| KEY-325 | `GetUserDefaultLocaleName` | `lang=system` 해석 | `nexa-dir2/crates/nexa-app/src/win.rs:320-331` |
| KEY-326 | `SystemParametersInfoW(SPI_GETWHEELSCROLLLINES)` | 휠 노치당 줄 수(기동 + `WM_SETTINGCHANGE`) | `nexa-dir2/crates/nexa-app/src/win.rs:6325-6335` · `:9663-9664` · `nexa-dir2/crates/nexa-gui/src/event.rs:6-18` |
| KEY-327 | `GetDoubleClickTime` | 느린 더블클릭 이름 변경 타이머 주기 | `nexa-dir2/crates/nexa-app/src/win.rs:76` · `:8580` |
| KEY-328 | `GetCaretBlinkTime` | 터미널 캐럿 깜빡임 주기 | `nexa-dir2/crates/nexa-app/src/win.rs:80` · `:6191` |
| KEY-329 | `GetTimeZoneInformation` | 날짜 표시용 UTC 오프셋(분) | `nexa-dir2/crates/nexa-app/src/win.rs:1370-1378` |
| KEY-330 | `WM_SETTINGCHANGE` | OS 테마 변경 추종(시스템 모드일 때) | `nexa-dir2/crates/nexa-app/src/win.rs:9662-9673` |
| KEY-331 | `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` | 제목 표시줄 다크 | `nexa-dir2/crates/nexa-app/src/win.rs:348-354` |
| KEY-332 | `GetVolumeInformationW`(볼륨 라벨 = `Google Drive`) | Google Drive 마운트 드라이브 탐지(A~Z 순회) | `nexa-dir2/crates/nexa-app/src/cloud.rs:150-184` |
| KEY-333 | `SHGetSpecialFolderLocation(CSIDL_BITBUCKET)` | 휴지통 접근 | `nexa-dir2/crates/nexa-app/src/recycle.rs:21` · `:49` |
| KEY-334 | `CryptProtectData` · `CryptUnprotectData`(DPAPI) | 토큰 암호화 — 비Windows 빌드는 `None`(= 클라우드 인증 미지원) | `nexa-dir2/crates/nexa-app/src/secret.rs:87-130` |
| KEY-335 | 드라이브 문자 `A:`~`Z:` 순회 | 최후 루트 폴백 | `nexa-dir2/crates/nexa-app/src/panel.rs:19-24` |
| KEY-336 | `ShellExecuteW("open")` | 런처 실행 · 브라우저로 인증 URL 열기 | `nexa-dir2/crates/nexa-app/src/launcher.rs:145-169` · `nexa-dir2/crates/nexa-app/src/win.rs:5689-5698` |

### 4-6. 설정 파일로 재정의할 수 있는 내장 상수

| ID | 상수 | 값 | 재정의 키 | 근거 |
|---|---|---|---|---|
| KEY-337 | OneDrive 기본 client_id | 소스에 내장(공개 클라이언트) | `cloud_client_id_onedrive` | `nexa-dir2/crates/nexa-app/src/oauth.rs:65` |
| KEY-338 | Google Drive 기본 client_id · client_secret | 소스에 내장(값은 이 문서에 옮기지 않는다) | `cloud_client_id_googledrive` · `cloud_client_secret_googledrive` | `nexa-dir2/crates/nexa-app/src/oauth.rs:90` · `:97` |
| KEY-339 | Dropbox 기본 App key · secret | 소스에 내장(값은 옮기지 않는다) | `cloud_client_id_dropbox` · `cloud_client_secret_dropbox` | `nexa-dir2/crates/nexa-app/src/oauth.rs:116-117` |
| KEY-340 | Dropbox 리디렉션 포트 | 53682 · 53683 · 53684(등록값과 정확 일치) | 없음 | `nexa-dir2/crates/nexa-app/src/oauth.rs:120` |
| KEY-341 | 루프백 선호 포트 | 42813(점유 시 임의 포트) | 없음 | `nexa-dir2/crates/nexa-app/src/oauth.rs:333-344` |
| KEY-342 | 로그인 대기 시간 | 300초 | 없음 | `nexa-dir2/crates/nexa-app/src/win.rs:5704` |
| KEY-343 | 세션 저장 디바운스 | 1000ms | 없음 | `nexa-dir2/crates/nexa-app/src/win.rs:91` |
| KEY-344 | 런처 시드 버전 | 2 | `launcher_seed`(상태) | `nexa-dir2/crates/nexa-app/src/launcher.rs:35` |
| KEY-345 | 목록 상한 | 런처 32 · 클라우드 32 · 토큰 슬롯 32 · 펼침 탭 인덱스 64 · 프리셋 블록 64 | 없음 | `nexa-dir2/crates/nexa-app/src/config.rs:770` · `:809` · `:964` · `nexa-dir2/crates/nexa-app/src/secret.rs:66` · `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:879` |

---

## 5. nexa-sql 방식 키 대응표

### 5-1. nexa-sql 설정 구조(차용 계약)

| ID | 규칙 | 근거 |
|---|---|---|
| KEY-601 | **레지스트리가 단일 원천** — `Entry { key, cat, label, desc, kind, default }`를 `REGISTRY` 한 곳에 적는다. 설정 창 · CLI · 검색이 모두 이 표를 읽는다 | `nexa-sql/crates/nsql-settings/src/lib.rs:10-13` · `:265-279` · `:717-718` |
| KEY-602 | 종류 7가지: `Choice(&[(값, 라벨)])` · `Lang` · `Int{min,max}` · `Size{min,max}`(`13`·`13px`·`10pt`) · `Bool`(`on`/`off`) · `Text` · `Position`(3×3 이름 9개) | `nexa-sql/crates/nsql-settings/src/lib.rs:233-263` |
| KEY-603 | 값 정규화: Bool은 `on/true/1/yes` · `off/false/0/no`를 받아 `on`/`off`로 저장 · Int는 **범위 밖이면 거부**(클램프 아님) · Choice는 소문자 일치 | `nexa-sql/crates/nsql-settings/src/lib.rs:6408-6455` |
| KEY-604 | 파일 `settings.conf` — `_schema=1` 머리 키 + `key=value` · `#` 주석 · 첫 `=` 분리 · 값 속 개행은 U+2028/2029로 치환해 한 줄 유지 | `nexa-ui/crates/nexa-conf/src/lib.rs:39-114` · `nexa-sql/crates/nsql-settings/src/lib.rs:3-8` · `:27` |
| KEY-605 | **기본값과 다른 값만 저장**(변경분 모델) — 기본값을 바꾸면 손대지 않은 사용자는 따라온다 | `nexa-sql/crates/nsql-settings/src/lib.rs:14` · `:6696-6728` · `:6738-6749` |
| KEY-606 | **모르는 키는 보존**해 다시 쓴다(구판이 저장해도 신판 키가 산다) | `nexa-sql/crates/nsql-settings/src/lib.rs:15` · `:6508-6509` · `:6563` · `nexa-ui/crates/nexa-conf/src/lib.rs:79-113` |
| KEY-607 | 손상 값은 읽을 때 기본값으로(파일은 건드리지 않음) | `nexa-sql/crates/nsql-settings/src/lib.rs:16` · `:6551-6562` |
| KEY-608 | 설정 폴더 = 환경 변수 `NSQL_HOME` \| `user_config_dir("nexa-sql")`(Windows `%APPDATA%\<app>` · macOS `~/Library/Application Support/<app>` · 그 외 `$XDG_CONFIG_HOME/<app>` 또는 `~/.config/<app>`) | `nexa-sql/crates/nsql-settings/src/lib.rs:154-161` · `nexa-ui/crates/nexa-conf/src/lib.rs:310-334` |
| KEY-609 | 키 이름 = **2레벨 `<접두>.<이름>`** 소문자 · `_` · 접두 ≤ 9자 · 접두 ↔ 설정 창 카테고리 1:1 지향 · 뜻이 위치보다 우선 | `nexa-sql/docs/94-settings-key-naming-and-location.md:8-11` · `:57-70` |
| KEY-610 | 단위 접미는 늘 붙인다(`_ms` `_secs` `_min` `_mb` `_kb` `_pct`) · 색은 `_color` · **기본값이 10초 이하인 시간 = ms** | `nexa-sql/docs/94-settings-key-naming-and-location.md:11` · `:80-81` · `:113-118` |
| KEY-611 | 이름 바꿈은 `RENAMED (옛, 새)` · 단위 변경은 `RESCALED (옛, 새, 배수)` 표로 읽기 이주 | `nexa-sql/crates/nsql-settings/src/lib.rs:38-122` · `:6597-6620` |
| KEY-612 | 자동 기억 값(창 크기 · 최근 목록 · 분할 비율)은 **같은 파일의 `HIDDEN` 키**로 둔다 · 드물게 바꾸는 값은 `ADVANCED` | `nexa-sql/crates/nsql-settings/src/lib.rs:6165-6240` · `:6242-6371` |
| KEY-613 | 종속 잠금 `DEPENDS (자식, 부모, On\|NotEmpty\|Eq)` — 부모 조건이 거짓이면 설정 창에서 잠근다 | `nexa-sql/crates/nsql-settings/src/lib.rs:5941-5963` · `:6021-6045` |
| KEY-614 | OS별 기본값 `OS_DEFAULTS (키, macOS, Linux)` — Windows는 레지스트리 `default` · `ui.lang` 기본은 OS 언어 | `nexa-sql/crates/nsql-settings/src/lib.rs:5873-5912` |
| KEY-615 | 카테고리 트리 `CATEGORY_TREE (그룹, [카테고리])` · 카드 순서 `display_order` = 그룹 → 카테고리 → 접두 묶음 → 등재 순 · 순서 규칙 = 마스터 → 종속 → 범위 → 표시 → 한계 | `nexa-sql/crates/nsql-settings/src/lib.rs:5792-5871` · `:6373-6388` · `nexa-sql/docs/78-settings-reorganization.md:79-85` |

원자적 쓰기는 `nexa_conf::write_atomic`(temp → `sync_all` → rename → Unix는 부모 폴더 fsync · Unix 모드 0600)이다(`nexa-ui/crates/nexa-conf/src/lib.rs:121-164`). dir2의 `config::save`와 같은 계약이라 그대로 갈아 끼울 수 있다.

### 5-2. 옮길 때 정해야 하는 것(결정 필요 5건)

1. **데이터 폴더** — dir2는 포터블 우선(exe 옆 `data\`), nexa-sql은 사용자 설정 폴더 고정이다. `nexa-conf`에 포터블 판정 부품(`dir_writable` · `is_replaced_on_upgrade`)이 이미 있다(`nexa-ui/crates/nexa-conf/src/lib.rs:285-372`). 제안: `NDIR_HOME` → exe 옆 `data/`(쓰기 가능하고 관리형 설치 자리가 아닐 때) → `user_config_dir("nexa-dir")`.
2. **기본 테마** — dir2 `dark`(`nexa-dir2/crates/nexa-app/src/config.rs:254`) 대 nexa-sql `system`(`nexa-sql/crates/nsql-settings/src/lib.rs:727-734`). 대응표는 dir2 계승(`dark`)으로 적었다.
3. **i18n 방식** — nexa-sql 레지스트리의 `label`/`desc`는 컴파일 타임 `Msg` 열거(en · ko 2열)다(`nexa-sql/crates/nsql-i18n/src/lib.rs:1-28`). dir2 자원(`.lang` 3개 언어 + 사용자 덮어쓰기)을 그대로 쓰려면 dir3 레지스트리의 `label`/`desc`는 **문자열 키**(`pref.theme` · `pref.theme.desc`)여야 한다. `Lang` 종류도 고정 열거가 아니라 발견 목록(`en`·`ko`·`ja`+추가)을 받아야 한다.
4. **고속 스크롤 속도** — dir2는 `step`·`max` 숫자 2개, nexa-sql은 `scroll.fast_speed` 프리셋 하나(`slow`(6,×4) · `normal`(5,×8) · `fast`(3,×16) · `turbo`(2,×32) `nexa-sql/crates/nsql-settings/src/lib.rs:416-433`). dir2 기본(3, 16) = `fast`와 같다. 대응표는 dir2 기능 유지(숫자 2개)로 적고 프리셋은 대안으로 남겼다.
5. **목록 값의 구분자** — `|`는 POSIX 파일명에 올 수 있다. 경로 목록(탭 · 펼침 · 런처 · 클라우드)은 **줄 분리**(nexa-conf가 U+2028로 왕복 보존)로 바꾸는 안을 적었다.

### 5-3. `settings.cfg` → `settings.conf` 대응표

- 새 키 접두 ↔ 설정 창 카테고리(dir2 트리 `nexa-dir2/crates/nexa-app/src/prefs.rs:280-296` 계승): `ui.`(모양 · 글꼴 · 언어) · `list.`(보기·정렬) · `typeahead.` · `scroll.` · `ctxmenu.` · `transfer.` · `tabs.` · `dock.` · `term.` · `plugins.`/`preview.` · `toolbar.` · `launcher.` · `cloud.` · `window.` · `layout.`(숨김 · 자동 기억).
- 불리언 `0`/`1`은 nexa-sql `normalize`가 그대로 받아 `on`/`off`로 바꾼다(KEY-603) — 값 변환 코드가 필요 없다.
- 위치 정수 0..8 → `POSITIONS[n]`(`top_left` `top_center` `top_right` `mid_left` `center` `mid_right` `bottom_left` `bottom_center` `bottom_right` `nexa-sql/crates/nsql-settings/src/lib.rs:252-263`).
- 비율 f32 → 정수 퍼센트(`layout.editor_split_pct` 선례 `Int{10,90}` 기본 50 · HIDDEN). 소수 3자리 → 1% 단위로 정밀도가 준다.
- "동일" = nexa-sql에 같은 이름 · 같은 뜻의 키가 이미 있다.

| ID | 이전 키(KEY-0nn) | 새 키 제안 | 타입(SettingKind) | 기본값 | 비고 |
|---|---|---|---|---|---|
| KEY-501 | `theme` | `ui.theme` | Choice `system`\|`light`\|`dark` | `dark` | 동일 키(`nexa-sql/crates/nsql-settings/src/lib.rs:727-734`) · 기본값은 결정 필요(§5-2 ②) |
| KEY-502 | `lang` | `ui.lang` | Lang(발견 목록) | OS 언어(미지원이면 `en`) | 동일 키(`:719-726`) · `system` 값은 없앤다 — **키 부재 = OS 추종**, 고르면 기본과 같아도 저장(`:5885-5898` · `:6720-6726`) |
| KEY-503 | `show_hidden` | `list.show_hidden` | Bool | `on` | nexa-sql `file.show_hidden`(파일 대화상자용 · 기본 off)과 대상이 달라 접두를 나눈다 |
| KEY-504 | `show_dotfiles` | `list.show_dot` | Bool | `on` | `file.show_dot` 낱말 선례 |
| KEY-505 | `split` | `layout.panel_split_pct` | Int 10..90 | `50` | HIDDEN · f32 0.5 → 50 |
| KEY-506 | `dock` | `dock.visible` | Bool | `on` | `explorer.visible` 낱말 선례 |
| KEY-507 | `dock_ratio` | `layout.dock_height_pct` | Int 15..50 | `30` | HIDDEN |
| KEY-508 | `dock_split` | `layout.dock_split_pct` | Int 15..85 | `50` | HIDDEN |
| KEY-509 | `term_font` | `term.font_face` | Text(≤128 · 쉼표 = 폴백 체인) | Windows `Consolas` · macOS `Menlo` · Linux `DejaVu Sans Mono`(OS_DEFAULTS 제안) | `*.font_face` 낱말 선례 |
| KEY-510 | `term_font_size` | `term.font_size` | Size 8..32 | `12` | DIP = px |
| KEY-511 | `dlg_font` | `ui.dialog_font_face` | Text(≤64) | `` (빈 값 = `ui.font_face` 따름) | dir2 기본 `Segoe UI`는 Windows 전용 — 빈 값 기본이 nexa-sql 선례(`ui.font_face` 기본 `""` `:743-750`) |
| KEY-512 | `dlg_font_size` | `ui.dialog_font_size` | Size 9..32(px 기준 = 7pt..24pt) | `9pt` | dir2는 pt 단위 — Size가 `pt` 접미를 그대로 저장한다 |
| KEY-513 | `launcher` | `launcher.visible` | Bool | `on` | |
| KEY-514 | `term_wrap` | `term.wrap` | Bool | `on` | |
| KEY-515 | `term_cols` | `term.cols` | Int 80..1000 | `240` | DEPENDS(`term.cols`, `term.wrap`, Eq(`off`)) |
| KEY-516 | `term_theme` | `term.theme` | Text(≤64 · `system`\|`dark`\|`light`\|스킴 id) | `system` | 선택지가 스킴 표에서 나오므로 Text + 해석 시 폴백(동적 Choice 지원 시 Choice) |
| KEY-517 | `term_theme_dark` | `term.theme_dark` | Choice(어두운 스킴 9종 KEY-133) | `campbell` | DEPENDS 없음(선택자가 `light`여도 값 유지) |
| KEY-518 | `term_theme_light` | `term.theme_light` | Choice(밝은 스킴 6종 KEY-134) | `github-light` | |
| KEY-519 | `term_copy_format` | `term.copy_format` | Choice `text`\|`html`\|`rtf`\|`both` | `text` | |
| KEY-520 | `transfer_close_ms` | `transfer.close_ms` | Int 0..10000 | `2000` | 0 = 진행 창 미표시 · 시간 규칙(≤10초 = ms) 부합 |
| KEY-521 | `dnd_hover_ms` | `transfer.dnd_hover_ms` | Int 200..10000 | `3000` | dir2 설정 창에서 transfer 카테고리 소속(`nexa-dir2/crates/nexa-app/src/prefs.rs:566-572`) |
| KEY-522 | `preview_map` | `preview.map` | Text(≤512) | `` | ADVANCED · 형식 `ext:id|…` 유지 |
| KEY-523 | `plugins_disabled` | `plugins.disabled` | Text(≤512) | `` | nexa-sql `extensions.disabled` 선례(`nexa-sql/crates/nsql-settings/src/lib.rs:1069-1074`) · HIDDEN(플러그인 페이지 체크로 편집) |
| KEY-524 | `sort_folders_first` | `list.folders_first` | Bool | `on` | |
| KEY-525 | `sort_case_sensitive` | `list.case_sort` | Bool | `off` | |
| KEY-526 | `nav_up_align` | `list.nav_up_align` | Choice `top`\|`center`\|`bottom` | `center` | |
| KEY-527 | `tab_dblclick` | `tabs.dblclick` | Choice `close`\|`pin`\|`lock` | `close` | nexa-sql `tabs.` 접두 · `dblclick` 표기 선례(`ui.dblclick_ms`) |
| KEY-528 | `view_mode` | `list.view_mode` | Choice `tree`\|`flat`\|`tiles` | `tree` | "마지막 선택 = 새 탭 기본" — 메뉴로 바뀌는 값(HIDDEN 후보) |
| KEY-529 | `panel_mode` | `layout.panel_mode` | Choice `single`\|`dual` | `dual` | 메뉴 · 도구모음으로 바뀜(HIDDEN 후보) |
| KEY-530 | `info_mode` | `dock.info_mode` | Choice `single`\|`dual` | `dual` | 싱글 패널이면 유효값만 강제(`FORCES` 안 `nexa-sql/docs/78-settings-reorganization.md:114-130` — nexa-sql 구현 여부 미확인) |
| KEY-531 | `view_scope` | `list.view_scope` | Choice `global`\|`panel`\|`tab` | `panel` | |
| KEY-532 | `hide_empty_glyph` | `list.hide_empty_glyph` | Bool | `on` | |
| KEY-533 | `always_on_top` | `window.always_on_top` | Bool | `off` | 동일 키(nexa-sql `window.always_on_top` Bool `off`) |
| KEY-534 | `col_width_sync` | `list.col_sync` | Bool | `on` | |
| KEY-535 | `col_autofit_max` | `list.col_autofit_max` | Int 50..2000 | `400` | 대상이 접두에 있으므로 `_max` 접미(KEY-609 낱말 규칙 `nexa-sql/docs/94-settings-key-naming-and-location.md:78`) |
| KEY-536 | `toolbar_order` | `toolbar.layout` | Text | `` (빈 값 = 내장 기본 순서) | 키 이름은 nexa-sql과 같지만 값 문법은 dir2 것(`블록:vis[자식:vis,…]|…`)을 유지 — nexa-sql 문법은 그룹 순서 · 행 `;` · 플로팅 좌표(`nexa-sql/crates/nsql-i18n/src/lib.rs:4560`). 기본값을 빈 문자열로 두면 "변경분만 저장"과 "신규 버튼 자동 보충"이 함께 성립한다 |
| KEY-537 | `ctx_menu_order` | `ctxmenu.layout` | Text | `` (빈 값 = 내장 기본) | 위와 같은 이유 |
| KEY-538 | `typeahead_scope` | `typeahead.scope` | Choice `global`\|`level`\|`visible` | `visible` | nexa-sql에는 범위 키가 없다(`explorer.typeahead*` 5키뿐) |
| KEY-539 | `typeahead_reset_ms` | `typeahead.reset_ms` | Int 200..10000 | `1000` | nexa-sql 대응 `explorer.typeahead_timeout_ms`(200..60000 · 2000 `nexa-sql/crates/nsql-settings/src/lib.rs:2490-2500`) — 범위 · 기본은 dir2 계승 |
| KEY-540 | `typeahead_pos` | `typeahead.pos` | Position | `bottom_left`(← 6) | nexa-sql `explorer.typeahead_pos` 기본과 같다 |
| KEY-541 | `typeahead_special` | `typeahead.special` | Bool | `on` | |
| KEY-542 | `typeahead_space` | `typeahead.space` | Bool | `on` | |
| KEY-543 | `typeahead_backspace` | `typeahead.backspace` | Bool | `on` | nexa-sql에 없는 키 |
| KEY-544 | `fast_scroll` | `scroll.fast` | Bool | `on` | 동일 |
| KEY-545 | `fast_scroll_step` | `scroll.fast_step` | Int 1..50 | `3` | 대안 = `scroll.fast_speed` 프리셋(§5-2 ④) |
| KEY-546 | `fast_scroll_max` | `scroll.fast_max` | Int 1..32 | `16` | 위와 같음 |
| KEY-547 | `fast_scroll_window_ms` | `scroll.fast_window_ms` | Int 20..2000 | `160` | 동일(범위 · 기본까지) · nexa-sql은 HIDDEN, dir2는 설정 창 노출 |
| KEY-548 | `fast_scroll_hud` | `scroll.fast_hud` | Bool | `on` | 동일 |
| KEY-549 | `fast_scroll_hud_pos` | `scroll.fast_hud_pos` | Position | `top_right`(← 2) | 동일 |
| KEY-550 | `fast_scroll_hud_hold_ms` | `scroll.fast_hud_hold_ms` | Int 0..10000 | `250` | 동일 · nexa-sql은 HIDDEN |
| KEY-551 | `fast_scroll_hud_fade_ms` | `scroll.fast_hud_fade_ms` | Int 0..10000 | `600` | 동일 키 · 하한만 다름(nexa-sql 50 `nexa-sql/crates/nsql-settings/src/lib.rs:2575-2585`) |
| KEY-552 | `fast_scroll_grid_extra` | `scroll.fast_grid_extra` | Bool | `on` | 동일 · DEPENDS 5행도 그대로(`nexa-sql/crates/nsql-settings/src/lib.rs:6022-6026`) |
| KEY-553 | `base_font` | `ui.font_face` | Text | `` (빈 값 = OS 기본 UI 글꼴) | 동일 키 · dir2 기본 `Segoe UI`는 Windows 전용 |
| KEY-554 | `base_font_size` | `ui.font_size` | Size 8..32 | `12` | 동일 키(nexa-sql 8..40 · 15) — 범위 · 기본은 dir2 계승 |
| KEY-555 | `ctx_font` | `ui.menu_font_face` | Text | `` | dir3는 메뉴를 직접 그리므로 이 글꼴이 실제로 적용된다(dir2에서는 예약 값 — KEY-1389) |
| KEY-556 | `ctx_font_size` | `ui.menu_font_size` | Size 8..32 | `12` | 동일 키(nexa-sql 10..32 · 17) |
| KEY-557 | `status_font` | `statusbar.font_face` | Text | `` | nexa-sql `statusbar.` 접두 선례 |
| KEY-558 | `status_font_size` | `statusbar.font_size` | Size 8..32 | `12` | |
| KEY-559 | `list_font` | `list.font_face` | Text | `` | `grid.font_face` 선례 |
| KEY-560 | `list_font_size` | `list.font_size` | Size 8..32 | `12` | |
| KEY-561 | `list_folder_bold` | `list.folder_bold` | Bool | `off` | |
| KEY-562 | `header_bold` | `list.header_bold` | Bool | `off` | |
| KEY-563 | `header_italic` | `list.header_italic` | Bool | `off` | |
| KEY-564 | `launcher_seed` | `launcher.seed` | Int 0..9999 | `0` | HIDDEN · "시드를 넣었는가"의 유일한 표식(변경분 모델에서는 빈 목록 = 기본값이라 줄이 남지 않는다) |
| KEY-565 | `launcher_count` | (폐지) | — | — | `launcher.seed ≥ 1` + `launcher.items`로 "첫 실행"과 "비움"을 구분 |
| KEY-566 | `launcher<N>` | `launcher.items` | Text(여러 줄 · 한 줄 = `라벨|exe|인자` 또는 `-`) | `` | HIDDEN · 상한 32 · 줄 분리는 nexa-conf U+2028 왕복(KEY-604). exe 경로에 `|`가 올 수 있는 OS에서는 필드 구분자를 탭으로 바꾸는 안도 가능(결정 필요) |
| KEY-567 | `cloud<N>` | `cloud.conns` | Text(여러 줄 · 한 줄 = `kind|라벨|경로|account`) | `` | HIDDEN · 상한 32 · **토큰 파일이 줄 번호(N)에 묶여 있다** — 연결마다 고정 id를 두는 편이 안전(§6 위험) |
| KEY-568 | `cloud_client_id_<kind>` | `cloud.client_id_onedrive` · `cloud.client_id_googledrive` · `cloud.client_id_dropbox` | Text(≤256) | `` (빈 값 = 내장 기본) | ADVANCED · 레지스트리는 정적 키만 받으므로 3키로 펼친다 |
| KEY-569 | `cloud_client_secret_<kind>` | `cloud.client_secret_googledrive` · `cloud.client_secret_dropbox`(· `_onedrive`) | Text(≤256) | `` | ADVANCED · 설정 파일에 비밀이 실린다 — nexa-conf는 Unix에서 0600으로 쓴다(`nexa-ui/crates/nexa-conf/src/lib.rs:140-146`) |
| KEY-570 | `transfer_close_secs`(구 키) | RESCALED 한 줄 `(transfer_close_secs, transfer.close_ms, 1000)` | — | — | dir2 파일을 가져올 때만 필요 |
| KEY-571 | 머리 주석 `# nexa-dir settings v1` | `_schema=1` | — | — | 주석이 아니라 실제 키(`nexa-ui/crates/nexa-conf/src/lib.rs:39-40` · `:67-68`) |

dir2 설정 파일 가져오기(Windows 기존 사용자): 위 표의 (이전 키, 새 키)를 그대로 `RENAMED`에 넣으면 `migrate_renamed`가 옮긴다(KEY-611). 다만 dir2 파일은 **1레벨 키**라 dir3 `settings.conf`에 직접 들어 있지 않다 — 기동 때 `settings.conf`가 없고 dir2의 `settings.cfg`가 보이면 한 번 읽어 넣는 가져오기 단계가 따로 필요하다(파일 이름 · 폴더가 다르다). 비율(KEY-505 · 507 · 508)과 위치(KEY-540 · 549)는 값 변환이 있어 `RENAMED`만으로는 안 된다.

### 5-4. `session.cfg` 대응

nexa-sql은 세션 성격의 값을 `settings.conf`의 HIDDEN 키(창 크기 · 최근 목록 `nexa-sql/crates/nsql-settings/src/lib.rs:6171-6220`)와 프로젝트 파일(`.nsql-project` `nexa-sql/crates/nsql-settings/src/projfile.rs:1-15`)로 나눠 둔다. dir3의 세션은 탭 수 × 펼침 경로로 커지고 1초 디바운스로 자주 써지므로 **별도 파일을 유지**하는 편이 맞다(설정 파일을 매번 다시 쓰지 않는다).

| ID | 이전 | 새 제안 | 타입 | 비고 |
|---|---|---|---|---|
| KEY-581 | 파일 `session.cfg` | `session.conf`(nexa-conf 형식 · `_schema=1` · 설정과 같은 폴더) | — | `nexa_conf::Store` + `SaveScheduler`(조용해진 지 1초 또는 첫 변경 후 10초 `nexa-ui/crates/nexa-conf/src/lib.rs:166-226`)가 dir2 디바운스(KEY-343)를 대신한다 |
| KEY-582 | `active_panel` | `session.active_panel` | Int 0..1 | 2레벨 맞춤 |
| KEY-583 | `panel{i}.tabs` | `panel{i}.tabs` | 경로 목록 · **줄 분리** | 이미 2레벨 꼴 · 구분자만 `|` → 개행(U+2028 저장) |
| KEY-584 | `panel{i}.active` | `panel{i}.active` | Int | 그대로 |
| KEY-585 | `panel{i}.exp{j}` | `panel{i}.exp{j}` | 경로 목록 · 줄 분리 | 그대로 · j 상한 64 유지 |
| KEY-586 | `panel{i}.locked` · `.pinned` | 그대로 | `0`/`1` 목록(`|` 유지 가능 — 경로가 아니다) | |
| KEY-587 | `panel{i}.modes` · `.views` | 그대로 | 목록(`|` 유지 가능) | |
| KEY-588 | `panel{i}.cols` · `.colw` | 그대로 | 순서 문법 · 쉼표 목록 | 폭은 논리 px(DPI 무관)로 저장할지 확인 필요 — dir2는 물리 px(추정 · `col_widths()` 반환값 그대로 `nexa-dir2/crates/nexa-app/src/win.rs:6819`) |
| KEY-589 | (없음) | `panel{i}.sort{j}`(후보) | 탭별 정렬 컬럼 · 방향 | dir2는 영속하지 않는다 — 기능 유지 원칙상 추가하지 않아도 된다 |
| KEY-590 | 가상 경로 `::PC::` · `::CLOUD:<idx>::/…` | 값 형식 유지 | — | `::CLOUD:<idx>`는 연결 순번에 묶여 있다(KEY-567과 같은 위험) |

### 5-5. dir2에 없어 새로 필요한 키(후보)

| ID | 새 키 | 타입 · 기본 | 이유 | 선례 |
|---|---|---|---|---|
| KEY-591 | `term.shell` | Text · `` (빈 값 = 자동: Windows `pwsh` → `powershell` → `cmd` · macOS/Linux `$SHELL` → `/bin/sh`) | 터미널 셸 OS 분기(요구 사항) — dir2는 하드코딩(`nexa-dir2/crates/nexa-app/src/conpty.rs:281-301`) | 없음(신설) |
| KEY-592 | `window.main_size` · `window.main_pos` | Text `w,h` · `x,y` · HIDDEN | winit 창은 OS가 배치를 기억해 주지 않는다 — dir2는 미영속 | `nexa-sql/crates/nsql-settings/src/lib.rs:3479-3487` |
| KEY-593 | `window.prefs_size` · `window.prefs_pos`(및 일괄 이름 변경 · 압축 그리드 · 미리보기 창) | Text · HIDDEN | 보조 창 배치 기억 | nexa-sql `window.prefs_size` |
| KEY-594 | `license.gates` | Bool `off` · HIDDEN | 라이선스 게이트 스위치(nexa-sql 적용 방식) — dir2에는 라이선스 코드가 없다 | `nexa-sql/crates/nsql-settings/src/lib.rs:4531-4538` |
| KEY-595 | `ui.prefs_advanced` | Bool `off` · HIDDEN | 설정 창 Advanced 토글 상태 | nexa-sql HIDDEN 첫 줄(`:6166`) |
| KEY-596 | `ui.text_gdi` · `ui.text_hint` · `ui.text_snap` · `ui.text_weight` · `ui.text_contrast` | nexa-sql 그대로 + OS_DEFAULTS | 직접 그리는 글자 래스터의 OS별 기본 | `nexa-sql/crates/nsql-settings/src/lib.rs:5876-5883` |
| KEY-597 | `gfx.linux_backend` · `gfx.mac_present` · `clipboard.x11_native` | nexa-sql 그대로 | softbuffer/winit 플랫폼 층 스위치 | nexa-sql Performance 카테고리 |
| KEY-598 | `input.scroll_natural` · `ui.dblclick_ms` | Bool `off` · Int `400`(HIDDEN) | dir2는 OS 값(KEY-326 · 327)을 읽었다 — winit에는 더블클릭 시간 조회가 없다 | nexa-sql `input.scroll_natural` · `ui.dblclick_ms` |
| KEY-599 | `list.trash`(후보) 또는 OS 고정 | — | 휴지통 동작(Windows 휴지통 · macOS Trash · Linux `~/.local/share/Trash`)에 선택지가 필요한지 결정 필요 | 없음 |
| KEY-600 | 환경 변수 `NDIR_HOME` | 설정 폴더 재지정(시험 · 격리) | 회귀 시험 하네스가 사용자 설정을 건드리지 않게 | `NSQL_HOME`(`nexa-sql/crates/nsql-settings/src/lib.rs:154-161`) |

---

## 6. 위험과 점검 항목(이 조사에서 드러난 것)

1. **`|` 구분자** — 탭 · 펼침 · 런처 · 클라우드 값이 모두 `|`로 나뉜다(KEY-066 · 067 · 103 · 105). macOS · Linux에서는 `|`가 파일명 문자다.
2. **인덱스 결합** — `cloud<N>` ↔ `secrets\cloud<N>.tok` ↔ 세션의 `::CLOUD:<N>::`이 순번으로 묶여 있다(KEY-067 · 130 · 209). 연결을 지우면 재배치가 필요하다(`clear_from` `nexa-dir2/crates/nexa-app/src/secret.rs:38-41`).
3. **토큰 보관이 Windows 전용** — DPAPI 밖 OS에서는 `protect`가 `None`이라 클라우드 인증이 통째로 동작하지 않는다(KEY-334). macOS Keychain · Linux Secret Service 또는 nexa-sql `nsql-vault`(`device.key` 방식 `nexa-sql/crates/nsql-vault/src/devkey.rs:251`) 중 하나로 대체해야 한다.
4. **글꼴 기본값이 Windows 전용**(`Segoe UI` · `Consolas` — KEY-009 · 011 · 053~059). 빈 값 기본 + OS_DEFAULTS로 바꿔야 한다.
5. **Int 검증 차이** — dir2는 범위 밖 값을 클램프하고, nexa-sql `normalize`는 거부(= 기본값)한다(KEY-603). dir2 파일을 가져올 때는 클램프 후 넣어야 사용자 값이 사라지지 않는다.
6. **변경분 모델과 목록형 값** — "빈 목록"과 "한 번도 설정 안 함"이 파일에서 구분되지 않는다(KEY-564~566).
7. **i18n 결함 2건**(KEY-1107 · 1109)과 **사문 키 61개**(§2-4) — 자원을 그대로 쓰되 결함은 고치고, 사문 키는 지우기 전에 nexa-ui 컨트롤로 다시 만든 화면에서 쓰는지 확인한다.
8. **플랫폼 문구 15건**(§2-5) — OS별 문자열 분기 또는 중립 문구로 바꾼다.
9. **`tr()`는 UI 스레드 전용**(`thread_local`) — winit 이벤트 루프 밖 워커에서 부르면 en이 나온다(추정 · KEY-1501 위 설명).
10. **`settings.cfg`에 평문 비밀** — `cloud_client_secret_*`가 설정 파일에 들어간다(KEY-069). 포터블 폴더를 통째로 복사하면 함께 나간다.
