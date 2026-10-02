# 16 · nexa-dir2 인벤토리 — 앱 내장 컨트롤(ctl)과 보조 창(대화상자)

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 기준 커밋 = 조사 시점 작업 트리(nexa-dir2 0.22.0 · nexa-ui `df75f5a` · nexa-sql `c1970ae`).
> ID 접두사 = **DLG-NNN**(고정 — 구현·교차 검증 체크리스트). 근거 표기 = `저장소/경로:줄`.
> "추정" 표기 = 코드로 확정하지 못한 것(실기 미확인 포함).

## 0. 범위 — 읽은 파일과 줄 수

### 0-1. nexa-dir2(원본 — 전부 끝까지 읽음)

| 파일 | 줄 | 내용 |
|---|---:|---|
| `nexa-dir2/crates/nexa-app/src/ctl/mod.rs` | 51 | 컨트롤 색인·명명/판매/AA 규약 |
| `nexa-dir2/crates/nexa-app/src/ctl/base.rs` | 78 | 상태 박스·통지·클래스 등록 공통 |
| `nexa-dir2/crates/nexa-app/src/ctl/style.rs` | 119 | 팔레트 `Style`·자동 높이·실측 |
| `nexa-dir2/crates/nexa-app/src/ctl/button.rs` | 258 | NxButton |
| `nexa-dir2/crates/nexa-app/src/ctl/checkbox.rs` | 246 | NxCheckBox(2단/3단) |
| `nexa-dir2/crates/nexa-app/src/ctl/label.rs` | 147 | NxLabel |
| `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs` | 491 | NxComboBox + 팝업 |
| `nexa-dir2/crates/nexa-app/src/ctl/textbox.rs` | 287 | NxTextBox(암호 모드 포함) |
| `nexa-dir2/crates/nexa-app/src/ctl/searchbox.rs` | 271 | NxSearchBox(내장 ✕) |
| `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs` | 845 | NxFontBox(글꼴 피커) |
| `nexa-dir2/crates/nexa-app/src/ctl/segmented.rs` | 356 | NxSegmented |
| `nexa-dir2/crates/nexa-app/src/ctl/spin.rs` | 378 | NxSpin |
| `nexa-dir2/crates/nexa-app/src/ctl/groupcard.rs` | 250 | NxGroupCard |
| `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs` | 338 | NxIconButton(벡터/PNG/SVG) |
| `nexa-dir2/crates/nexa-app/src/ctl/menubutton.rs` | 469 | NxMenuButton + 팝업 |
| `nexa-dir2/crates/nexa-app/src/ctl/grid.rs` | 1156 | NxGrid |
| `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs` | 933 | NxOrderTree |
| `nexa-dir2/crates/nexa-app/src/ctldemo.rs` | 512 | 컨트롤 검증 갤러리 창 |
| `nexa-dir2/crates/nexa-app/src/dialog.rs` | 792 | 임의 버튼 모달·전송 진행 창 |
| `nexa-dir2/crates/nexa-app/src/pwprompt.rs` | 305 | 암호 입력 모달 |
| `nexa-dir2/crates/nexa-app/src/about.rs` | 394 | About 모달 |
| `nexa-dir2/crates/nexa-app/src/ordereditor.rs` | 427 | 순서/표시 편집 공통 모달 |
| `nexa-dir2/crates/nexa-app/src/bulkrename.rs` | 2252 | 일괄 이름 변경 창(UI) |
| **합계** | **11,355** | (`ctl/gdipctx.rs` 647줄은 범위 제외 — 시그니처만 확인) |

보조로 확인한 것(호출부·규약): `nexa-dir2/crates/nexa-app/src/win.rs`(대화상자 호출부 3778-3812 · 3955-3975 · 4535-4575 · 4262-4304 · 5358-5371 · 6296-6321 · 6418-6460 · 8995-9056), `prefs.rs`(1011-1028 · 2366-2391 · 3040-3066), `archivewnd.rs:160-193`, `config.rs`(362 · 986-994 · 1069-1118), `nexa-dir2/docs/ctl/overview.md`, `nexa-dir2/docs/22-batch-rename-v2.md`, `nexa-dir2/docs/23-cross-platform-feasibility.md:106-115`.

### 0-2. nexa-ui(대응 확인 — Grep + 부분 정독)

`nexa-ui/crates/nexa-ctl/src/lib.rs`(80) · `controls/mod.rs`(817 전부) · `controls/button.rs`(1-470) · `controls/checkbox.rs`(1-200) · `controls/combo.rs`(100-330 + API 목록) · `controls/tree.rs`(195-495 · 640-955) · `controls/textbox.rs`(API 목록 + 2095-2135 · 2612-2626 · 3250-3266 · 3314-3324) · `event.rs`(1-120) · `draw.rs`/`widget.rs`/`scroll.rs`/`ctxmenu.rs`/`radio.rs`/`switch.rs`/`timeout_button.rs`/`listedit.rs`/`pulldown.rs`(공개 API 목록) · `nexa-ui/crates/nexa-dlg/src/lib.rs`(공개 API 목록) · `nexa-ui/crates/nexa-font/src/lib.rs`(공개 API 목록) · `nexa-ui/docs/21-grid-family.md`(1-140).
nexa-sql 참고(보조 창 골격): `nexa-sql/crates/nexa-sql/src/winhost.rs:1-60` · `about_win.rs:1-40` · `input_win.rs:1-60` · `winfocus.rs:16-24`.

---

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지.
"기존 테스트" = dir2 저장소에 있는 자동 테스트(범위 파일 안의 테스트는 `fontbox.rs:831` 1건뿐이다).

### 1-1. 공통 인프라(ctl/base · ctl/style · 공통 패턴)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-001 | 컨트롤 상태 수명 | 컨트롤마다 `Box<State>`를 `GWLP_USERDATA`에 설치(생성) → `WM_DESTROY`에서 회수(Drop 실행 — HFONT·HBRUSH·GpImage 해제). 창 클래스는 `Once`로 1회 등록(화살표 커서) | `nexa-dir2/crates/nexa-app/src/ctl/base.rs:23` · `:31` · `:39` · `:68` | `Get/SetWindowLongPtrW` · `RegisterClassW` · `LoadCursorW` | A | 없음 |
| DLG-002 | 컨트롤 → 호스트 통지 | 부모에 `WM_COMMAND(MAKEWPARAM(id, code), lparam=컨트롤)` 동기 재발행. 부모 없으면 무시. 모든 컨트롤의 "변경/클릭/선택" 보고 경로 | `nexa-dir2/crates/nexa-app/src/ctl/base.rs:52` | `SendMessageW` · `GetParent` · `GetDlgCtrlID` | A | 없음 |
| DLG-003 | 팔레트 `Style` 8색 | `bg` 0xFFFFFF · `border` #A4A8AC(BGR 0x00ACA8A4) · `text` #202020 · `text_dim` #686E78 · `accent` #0078D4 · `sel_bg` #E4E7EC · `behind` 0xFFFFFF(부모 배경 — AA 모서리 블렌드 기준) · `danger` #FF3B30. 컨트롤은 색을 하드코딩하지 않고 생성 인자로 받는다(예외: searchbox·fontbox는 라이트 고정) | `nexa-dir2/crates/nexa-app/src/ctl/style.rs:12` · `:33-46` | `COLORREF`(0x00BBGGRR) | A | 없음 |
| DLG-004 | 공통 자동 높이 | `h<=0` → `auto_height = 글꼴 높이(tmHeight, 최소 12) + 상하 4px×2`. 예외 = 버튼·세그먼트 **컴팩트**(글꼴+4). 텍스트는 세로 중앙 **+1px 하향** 보정(콤보·글상자·세그·버튼·라벨 공통) | `nexa-dir2/crates/nexa-app/src/ctl/style.rs:75` · `:79` · `:105-118` · `button.rs:87-91` | `GetTextMetricsW` | A | 없음 |
| DLG-005 | 텍스트 폭 실측·fill/frame | `text_width`(라벨 열 폭을 현재 언어로 실측 — i18n 변경에도 정렬 유지) · `fill`(단색 사각) · `frame`(1px 4변) | `nexa-dir2/crates/nexa-app/src/ctl/style.rs:49` · `:56` · `:87` | `GetTextExtentPoint32W` · `FillRect` | A | 없음 |
| DLG-006 | Tab 포커스 이동 | 호스트 모달 루프의 `IsDialogMessageW` → Tab = **생성 순서** 이동. 컨테이너(GroupCard·TextBox·Spin·카드 호스트)는 `WS_EX_CONTROLPARENT`로 내부 EDIT에 착지. 방향키를 쓰는 컨트롤은 `WM_GETDLGCODE`=`DLGC_WANTARROWS`(콤보·세그·메뉴버튼·그리드[+WANTCHARS]). 이 문서 범위에서 적용 창 = bulkrename·이름 팝업·관리 팝업(pwprompt·ordereditor·dialog·about은 미적용 — Tab 이동 없음) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:2235` · `:1614` · `:1800` · `ctl/textbox.rs:90` · `ctl/combobox.rs:279` · `ctl/grid.rs:740` | `IsDialogMessageW` · `WS_EX_CONTROLPARENT` · `WM_GETDLGCODE` | A | 없음 |
| DLG-007 | 드롭 팝업 공통 패턴 | 팝업 = `WS_POPUP` + `NOACTIVATE/TOOLWINDOW/TOPMOST` 별도 창(포커스는 본체 유지 — `WM_MOUSEACTIVATE`→`MA_NOACTIVATE`). owner는 팝업 `GWLP_USERDATA`에 저장(WS_POPUP owner 승격 → `GetParent` 오독 회피). **바깥 클릭 닫기 = 60ms 타이머가 `GetAsyncKeyState(VK_LBUTTON)`+커서 좌표로 본체·팝업 밖 눌림 감지**. 팝업 모서리 = `CreateRoundRectRgn` | `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs:151-195` · `:306-328` · `menubutton.rs:141-190` · `fontbox.rs:228-305` | `CreateWindowExW(WS_POPUP)` · `SetTimer` · `GetAsyncKeyState` · `GetCursorPos` · `SetWindowRgn` | A | 없음 |

### 1-2. NxButton · NxCheckBox · NxLabel

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-008 | 푸시 버튼 모양 3상태 | 라운드 반경 6. **Normal** = `sel_bg` 필 + `text` 글자 + 1px `border` 외곽선 · **Default** = `accent` 필 + `bg` 글자(무테) · **Disabled** = `sel_bg` 필 + `text_dim` 글자 + 외곽선. 모서리 = `behind`. 라벨 가운데 정렬. hover/pressed 시각 없음 | `nexa-dir2/crates/nexa-app/src/ctl/button.rs:46` · `:206-255` | GDI+ AA(gdipctx) · `DrawTextW` | A | 없음 |
| DLG-009 | 버튼 클릭·키·자동 크기 | 클릭 통지는 **`WM_LBUTTONDOWN` 즉시**(뗄 때가 아님) + `SetFocus`. Space/Enter(포커스 시) = 클릭. 비활성은 전부 무시. `h<=0` = 글꼴+4 · `w<=0` = 라벨 폭 + 좌우 16px×2 | `nexa-dir2/crates/nexa-app/src/ctl/button.rs:87-97` · `:188-205` | `SetFocus` | A | 없음 |
| DLG-010 | 버튼 상태 API | `NXBTN_SETENABLE/GETENABLE`(WM_USER+106/105) · `NXBTN_SETDEFAULT`(+107 — 회색↔accent) · 라벨 = `WM_SETTEXT`(재도장). `EnableWindow`는 그리기 미반영이라 쓰지 않는다 | `nexa-dir2/crates/nexa-app/src/ctl/button.rs:36-42` · `:162-187` · `bulkrename.rs:507-513` | `WM_SETTEXT` | A | 없음 |
| DLG-011 | 체크박스 모양(2단/3단) | 박스 = **글꼴 높이 정사각**(10..컨트롤 높이 클램프)을 세로 중앙, 반경 `max(side/3,4)`. 해제 = `sel_bg` 필 + 1px 외곽선 · 체크 = `accent` 필 + `bg` ✓(2px 폴리라인) · **부분(3단 값 2)** = `accent` 필 + **흐릿한 ✓**(bg·accent 50% 블렌드). 라벨 = 박스 오른쪽 +6px(빈 문자열 = 박스만). 배경 = `style.bg` | `nexa-dir2/crates/nexa-app/src/ctl/checkbox.rs:43` · `:170-243` | GDI+ AA | A | 없음 |
| DLG-012 | 체크 토글·API | 컨트롤 영역 어디든 클릭 / Space = 순환(2단 0↔1 · 3단 0→1→2→0) → `NXCHK_CHANGED` 통지. `NXCHK_GETCHECK`(+95) · `NXCHK_SETCHECK`(+96 — 통지 없음, 모드 최대값 클램프). `w<=0` = 높이와 같은 정사각 | `nexa-dir2/crates/nexa-app/src/ctl/checkbox.rs:35-39` · `:79` · `:117-169` | — | A | 없음 |
| DLG-013 | 폼 라벨 | 좌/우 정렬(기본 Right — 콜론이 컨트롤에 붙는 구도) · `h<=0` 자동 높이 · 배경 = `behind` · +1px 하향 · **클릭 투과**(`HTTRANSPARENT`) · `WM_SETTEXT` 재도장 · 통지 없음. 그리기 버퍼 256자(초과분 잘림) | `nexa-dir2/crates/nexa-app/src/ctl/label.rs:28` · `:45` · `:113` · `:114-144` | `WM_NCHITTEST` · `GetWindowTextW` | A | 없음 |

### 1-3. NxComboBox · NxTextBox · NxSearchBox

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-014 | 콤보 본체 | 라운드(6) `sel_bg` 필 + 1px 외곽선 · 현재 항목 라벨(좌 +10, 우 −22) · 우측 **이중 셰브론 ⌃⌄**(존 = 오른쪽 −20..−6, 선폭 1.4) · 자동 높이 | `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs:215-228` · `:329-369` | GDI+ AA | A | 없음 |
| DLG-015 | 콤보 팝업 표시 | 본체 아래 +2px · 폭 = `max(본체 폭, 80)` · 가시 행 = `min(항목 수, 12)` · 행 높이 = 글꼴+10 · 높이 = 행×수+6 · 라운드. 현재 선택 = ✓(열 폭 22) · hot 행 = `accent` 라운드 필(반경 4) + `bg` 글자. 12개 초과 = hot 중심 표시 구간 이동(`first_visible`). **화면 하단에서 위로 뒤집는 로직 없음**(항상 아래) | `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs:44-50` · `:147` · `:151-195` · `:413-491` | DLG-007 패턴 | A | 없음 |
| DLG-016 | 콤보 조작·API | 본체 클릭 = 열기/닫기 토글(+SetFocus). 닫힘 ↓ = 열기. 열림: ↑/↓ = hot 이동(끝 클램프) · Enter = 확정 · Esc = 닫기(선택 불변) · 그 외 키 삼킴. 팝업 마우스 이동 = hot · **버튼 UP = 확정**. 확정 시 값이 **바뀐 경우만** `NXCB_CHANGED`. `NXCB_GETSEL`(+90) · `NXCB_SETSEL`(+91 — 통지 없음·범위 밖 무시). 휠 미지원. 항목은 생성 후 불변 | `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs:35-39` · `:204-212` · `:257-305` · `:386-412` | — | A | 없음 |
| DLG-017 | 글상자 모양 | 라운드(6) `bg` 필 + 평시 1px `border` / **포커스 = `accent` 2px 링**. 좌우 내부 여백 8. 내부 입력은 글꼴+2 높이로 세로 중앙(+1px) | `nexa-dir2/crates/nexa-app/src/ctl/textbox.rs:43-47` · `:172-189` · `:263-284` | GDI+ AA · 자식 `EDIT` | A | 없음 |
| DLG-018 | 글상자 편집·텍스트 API | 실입력 = 무테두리 `EDIT`(`ES_AUTOHSCROLL` 단일행) → 캐럿·선택·클립보드·Undo·IME·우클릭 메뉴는 OS EDIT 기본 동작. 위임: `WM_SETTEXT/GETTEXT/GETTEXTLENGTH` · `EM_SETCUEBANNER`(플레이스홀더) · `EM_SETSEL`(전체 선택 등). 통지 재발행: `EN_CHANGE`(0x0300) · `EN_SETFOCUS`(0x0100) · `EN_KILLFOCUS`(0x0200) | `nexa-dir2/crates/nexa-app/src/ctl/textbox.rs:105-119` · `:214-252` | `EDIT` 클래스 · `WM_CTLCOLOREDIT` | A | 없음 |
| DLG-019 | 암호 입력 모드 | `set_password_char(Some('●'))` = 마스킹(EDIT `EM_SETPASSWORDCHAR` — 복사/잘라내기 차단 포함) · `None` = 평문 표시. `clear_secret` = 내용 + 되돌리기 버퍼 제거(`EM_EMPTYUNDOBUFFER`) | `nexa-dir2/crates/nexa-app/src/ctl/textbox.rs:145-169` | `EM_SETPASSWORDCHAR` · `EM_EMPTYUNDOBUFFER` | A | 없음 |
| DLG-020 | 검색 입력(내장 ✕) | 각진 1px 테두리(라이트 고정 — 테두리 #A4A8AC · ✕ #686E78) · 좌 여백 4 · 우측 ✕ 영역 폭 20. **입력이 있을 때만 ✕ 표시**, ✕ 클릭 = 전체 지우기 + 포커스 복귀(`EN_CHANGE` 경유 통지). EDIT 높이 = 글꼴+4(한글 글리프 여유)로 세로 중앙. 실시간 검색 전제(버튼 없음). 텍스트 API·플레이스홀더 위임. 사용처 = 설정 창 카테고리 검색 | `nexa-dir2/crates/nexa-app/src/ctl/searchbox.rs:34-44` · `:88-100` · `:177-224` · `:226-268` · `prefs.rs:2955` | 자식 `EDIT` | A | 없음 |

### 1-4. NxFontBox · NxSegmented · NxSpin · NxGroupCard

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-021 | 설치 글꼴 목록 | 프로세스당 1회 열거 → `@`(세로쓰기) 제외 → 소문자 기준 정렬 → 중복 제거. `fontchain`(폴백 체인 판정)도 이 목록을 쓴다 | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:55-91` · `fontchain.rs:38` | `EnumFontFamiliesExW` | P | 없음(`fontchain.rs` 1건은 체인 로직) |
| DLG-022 | 글꼴 드롭다운 | 입력란 클릭(또는 테두리 여백 클릭 = 토글)·닫힘 ↓ = 열기. 목록 = 전체 글꼴, **각 항목을 그 글꼴로 렌더**(행 높이 = 글꼴+10 · 미리보기 글꼴 높이 = 행−8 · 항목별 HFONT 지연 생성·캐시). 12행 가시 + 세로 스크롤바 · 폭 = `max(컨트롤 폭, 180)` · 선택 행 = #E4E7EC · hover = 선택 이동 | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:228-292` · `:626-683` · `:685-775` · `:778-824` | `LISTBOX`(owner-draw) · `CreateFontW` | A+P | 없음 |
| DLG-023 | 글꼴 타입어헤드·키 | 입력 변경 시 **마지막 `,` 뒤 조각**과 접두 일치(대소문자 무시)하는 첫 글꼴로 선택 이동 + 상단 인덱스 = 일치−2. 열림: ↑/↓ ±1 · PgUp/PgDn ±12 · Enter = 확정 · Esc = 닫기. 닫힘 Enter = 호스트가 처리(비프만 억제) | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:205` · `:314-331` · `:381-419` | EDIT 서브클래스(`GWLP_WNDPROC`) | A | 없음 |
| DLG-024 | 글꼴 선택 반영·확정 | `apply_pick`: 입력에 `,`가 있으면 마지막 조각만 선택 글꼴로 교체(`"A, B"` 체인), 없으면 **전체 교체**. 확정 후 캐럿 = 맨 끝·포커스 유지 → 부모에 `EN_KILLFOCUS`(= 즉시 적용 경로). 포커스 이탈 = 닫기 + `EN_KILLFOCUS`(새 포커스가 드롭다운이면 유지). 목록 클릭 = DOWN은 삼키고 UP 좌표로 확정. `FBM_HAS_DROP`(WM_USER+40) = 호스트가 Enter를 넘길지 판정 | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:41` · `:212-217` · `:334-364` · `:425-436` · `:646-667` · `prefs.rs:3053-3063` | `LB_ITEMFROMPOINT` · `EM_SETSEL` | A(로직 N) | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:831`(`apply_pick_segment_rules`) |
| DLG-025 | 세그먼트 라디오 모양 | 항목 등폭(마지막 칸이 나머지 흡수). `gap=0`(기본) = `sel_bg` 컨테이너 + 선택 칸 **accent 필(1px 인셋)** · `gap>0` = 칸별 독립 블록. `corner` 기본 6(0 = 각짐). 선택 글자 = `bg`, 비선택 = `text`. `h<=0` = 글꼴+4(컴팩트) | `nexa-dir2/crates/nexa-app/src/ctl/segmented.rs:42-53` · `:115-119` · `:223-353` | GDI+ AA | A | 없음 |
| DLG-026 | 세그먼트 화살표 라벨 | 라벨이 `"→ "`/`"← "`로 시작하면 화살표를 **Segoe MDL2 Assets 글리프**(U+E72A/U+E72B, 본문 글꼴 높이−3)로 그리고 [글리프][5px][텍스트] 묶음을 칸 중앙 정렬. 일괄 이름변경 Position(`→ abc`/`← abc`)에서 사용 | `nexa-dir2/crates/nexa-app/src/ctl/segmented.rs:76-93` · `:314-338` | `Segoe MDL2 Assets` 글꼴 | A(글리프 P) | 없음 |
| DLG-027 | 세그먼트 선택·API | 클릭 = 그 칸 선택 · ←/→ = 이웃(끝 클램프). **값이 바뀔 때만** `SEG_CHANGED`. `SEG_GETSEL`(+50) · `SEG_SETSEL`(+51 — 통지 없음) | `nexa-dir2/crates/nexa-app/src/ctl/segmented.rs:34-38` · `:161-218` | — | A | 없음 |
| DLG-028 | 숫자 스피너 모양 | **독립 라운드 글상자**(bg+border, 숫자 **우측 정렬**) + 오른쪽 `GAP 4` 뒤 **분리된 ⌃⌄ 버튼 블록**(폭 = `max(h×2/3, 14)` · `sel_bg` 라운드). min/max 도달 방향 셰브론 = `border`색(흐림). 포커스 링 없음 | `nexa-dir2/crates/nexa-app/src/ctl/spin.rs:41-48` · `:119-133` · `:330-375` | 자식 `EDIT`(`ES_NUMBER`·`ES_RIGHT`) | A | 없음 |
| DLG-029 | 스피너 값 동작 | 타이핑 = 숫자만(`ES_NUMBER` — **음수 부호 직접 입력 불가**, 음수는 ⌄로만 도달) · ↑/↓ 키 ±1 · 블록 상/하 절반 클릭 ±1(한계 방향 무시, 자동 반복 없음) · 포커스 이탈 = 범위 클램프 확정 + `EN_KILLFOCUS`. 값 변화는 `EN_CHANGE` 통지. `SPIN_GETVAL`(+60 — 클램프 반환, 파싱 실패 = 0) · `SPIN_SETVAL`(+61 — 클램프). 휠 미지원. **주의**: `SPIN_SETVAL`도 내부 EDIT `WM_SETTEXT`가 `EN_CHANGE`를 발화해 부모에 재발행된다(추정 — `spin.rs:163` 주석·단일행 EDIT 규약) | `nexa-dir2/crates/nexa-app/src/ctl/spin.rs:33-35` · `:135-168` · `:175-214` · `:274-326` | EDIT 서브클래스 | A | 없음 |
| DLG-030 | 그룹 카드 컨테이너 | 타이틀 밴드(`sel_bg` + 하단 1px 구분선 + 좌 +10 라벨) + 본문(`bg`) + 1px 외곽선. `corner>0` = 창 리전 라운드(+RoundRect 외곽선). 타이틀/본문 높이 각각 지정. `title_rect`/`body_rect`로 자식 배치(타이틀 밴드에도 자식 가능 — 타이틀 텍스트 비우면 라벨 생략). 자식 통지(`WM_COMMAND/NOTIFY/DRAWITEM/MEASUREITEM/CTLCOLOR*`)는 부모로 투과. `GC_GETTITLEH`(+80) | `nexa-dir2/crates/nexa-app/src/ctl/groupcard.rs:36-47` · `:61-128` · `:135-150` · `:182-246` | `SetWindowRgn` · `RoundRect` | A | 없음 |

### 1-5. NxIconButton · NxMenuButton

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-031 | 원형 벡터 아이콘 버튼 | 지름 `d<=0` = 글꼴 높이(최소 10 — 체크박스 박스와 동일 크기). 원판 = 활성 `text_dim` / 비활성 `border`, 글리프 = `bg`색 2px(＋ · − · ∧ · ∨) 또는 `?`(텍스트). 모서리 = `behind` | `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs:56-70` · `:101-148` · `:269-331` | GDI+ AA | A | 없음 |
| DLG-032 | PNG 이미지 버튼 | `create_image(active_png, disabled_png, fit)` — 활성/비활성 **이미지 쌍** 전환. `ImageFit::Stretch`(컨트롤 크기로 바이큐빅 스케일 — 기본) / `Native`(원본 크기 중앙). 사용처 = 일괄 이름변경 카드 ±(32px 임베드 → 글꼴 높이로 축소) | `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs:45-51` · `:155-176` · `:252-268` · `bulkrename.rs:55-58` | GDI+ PNG 디코드(`gdipctx.rs:55`) | A | 없음 |
| DLG-033 | SVG 이미지 버튼 | `create_svg(svg 문자열)` — 컨트롤 크기 2배(`max(d,12)×2`)로 래스터 후 Stretch. 잉크 = `style.text`, 비활성 = 같은 색 알파 0x61(38%). 파싱 실패 = ＋ 벡터 폴백. 사용처 = 순서 편집기 ▲▼ | `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs:182-207` · `ordereditor.rs:304-325` | `crate::svg::parse` + `gdipctx.rs:102` | A | `nexa-dir2/crates/nexa-app/src/svg.rs`(13건 — 파서) |
| DLG-034 | 아이콘 버튼 클릭·활성 | `WM_LBUTTONDOWN`(활성일 때만) → `NXIB_CLICK`. `NXIB_SETENABLE/GETENABLE`(+101/+100). **키보드 처리 없음**(탭스톱이지만 Space/Enter 미구현)·클릭 시 포커스 이동 없음 | `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs:36-40` · `:220-243` | — | A | 없음 |
| DLG-035 | 오버플로 메뉴 버튼 모양 | 본체 = 라운드 필 + 외곽선 + 가운데 `…` + 우측 ⌄ 셰브론. 기본 폭 48 · 자동 높이. 팝업 폭 = `max(최대 라벨 폭+28, 본체 폭)` · 가시 행 = `min(항목, 12)` · 항목 `"-"` = 구분선(수평선만). ✓ 없음(선택 상태 없는 순수 액션). **12행 초과분은 그려지지 않는다(스크롤 없음)** — 프리셋 10개 이상이면 하단 Save/Edit 항목이 보이지 않는다(코드상 한계) | `nexa-dir2/crates/nexa-app/src/ctl/menubutton.rs:42-46` · `:94-99` · `:141-190` · `:316-358` · `:398-466` | DLG-007 패턴 | A | 없음 |
| DLG-036 | 메뉴 버튼 조작·API | 클릭 = 토글 · 닫힘 ↓ = 열기(hot=0) · 열림 ↑/↓ = 구분선 건너뛰며 이동 · Enter = 실행 · Esc = 닫기 · 팝업 hover = hot(구분선 제외) · 버튼 UP = 실행(구분선 무시). 실행 → `NXMB_PICK` 통지 → `NXMB_GETPICK`(+110)으로 **마지막 실행 인덱스**(구분선 포함 인덱스, 실행 전 −1). 항목 갱신 = 컨트롤 재생성 | `nexa-dir2/crates/nexa-app/src/ctl/menubutton.rs:38-40` · `:200-215` · `:239-292` · `:373-397` | — | A | 없음 |

### 1-6. NxGrid

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-037 | 그리드 코어 | 컬럼 = (제목, 폭[최소 40]). 헤더 높이 = 행 높이+2(밴드 `sel_bg` + 하단 구분선 + 컬럼 경계선[상하 3px 인셋]). 행 높이 = `opts.row_h` 또는 글꼴+8. 셀 = 텍스트(좌 +6 · 우 −4 · 말줄임 · +1px 하향). **빈 문자열 셀은 그리지 않는다** | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:75-79` · `:168-229` · `:286-300` · `:880-903` · `:1090-1141` | `DrawTextW(DT_END_ELLIPSIS)` | A | 없음 |
| DLG-038 | 컬럼 경계 드래그 리사이즈 | 헤더에서 경계 ±4px = 좌우 리사이즈 커서, 드래그로 폭 변경(최소 40, 가로 오프셋 재클램프), 마우스 캡처 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:521-531` · `:605-621` · `:648-650` · `:806-809` | `IDC_SIZEWE` · `SetCapture` | A(커서 P) | 없음 |
| DLG-039 | 오버레이 스크롤바(세로/가로) | 평소 숨김 → 스크롤 순간 얇은 썸(6px, 최소 길이 24) 표시 → **900ms 뒤 소등**. 표시 중 썸을 잡으면 넓은 바(10px)+트랙(`sel_bg`) 모드로 드래그, 놓으면 다시 오버레이. 가로 바 = 컬럼 합 > 폭일 때 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:81-96` · `:439-494` · `:560-569` · `:626-644` · `:811-849` · `:1052-1087` | `SetTimer` | A | 없음 |
| DLG-040 | 휠·픽셀 스크롤 | 노치(≥120) = 시스템 줄 수 × 고속 스크롤 가속, **행 단위 스냅** · Shift+휠 = 가로(48px/노치 + 가속) · 정밀 터치패드(120 미만) = 픽셀 누적 스크롤(`top_frac` 부분 오프셋). 끝 = 마지막 행 정렬 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:49-57` · `:496-518` · `:570-604` | `WM_MOUSEWHEEL` · `nexa_gui::wheel_lines` | A(휠 단위 P) | `nexa-dir2/crates/nexa-gui/src/fastscroll.rs`(5건) · `event.rs`(3건) |
| DLG-041 | 행 선택(마우스) | 클릭 = 단일 선택(+포커스·앵커) · Shift+클릭 = 앵커~대상 연속(기존 대체) · Ctrl+클릭 = 비연속 토글 + 앵커 이동 · **빈 영역 클릭 = 전체 해제**. 변경 시 `NXGR_SELCHANGE`(2). 조회 = `selected_rows()` | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:308-325` · `:684-736` · `:272-280` | `MK_SHIFT`/`MK_CONTROL` | A | 없음 |
| DLG-042 | 행 선택(키보드) | ↑/↓ · PgUp/PgDn(가시 행 수) · Home/End = 단일 선택 이동 · Shift+이동 = 범위 · **Ctrl+이동 = 포커스만**(선택 유지) · Space = 포커스 행 단일 선택 / Ctrl+Space = 토글 · Ctrl+A = 전체. 포커스 행 = accent 1px 테두리(컨트롤이 포커스일 때만). 포커스 행 가시 스크롤 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:327-335` · `:741-801` · `:988-993` | `GetKeyState` | A | 없음 |
| DLG-043 | 체크 마크 열 + 헤더 3상태 | `Mark::Check` = 컬럼 0이 체크박스(행별 `Option<bool>` — `None` = 표시 없음). 클릭 = 토글(**선택은 바꾸지 않음**) → `NXGR_TOGGLE`(1) + `NXGR_GETROW`(+120). 헤더 컬럼 0 = 전체 상태 체크(해제/전체/부분[흐릿한 ✓]), 클릭 = 전체가 체크면 전체 해제·그 외 전체 체크 → `NXGR_GETROW == −2`(`NXGR_ROW_ALL`). 셀 텍스트는 컬럼 1부터 대응 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:60-72` · `:339-358` · `:651-665` · `:699-717` · `:904-956` · `:1000-1029` · `:1116` | — | A | 없음 |
| DLG-044 | 행 우측 ⊖ 삭제 마크 | `Mark::Minus` = `check=Some` 행의 우측 끝(오른쪽 −10, 글꼴 높이 지름)에 `danger` 원 + 흰 −. 클릭 = `NXGR_TOGGLE` 통지(삭제 판단은 호스트). 가로 스크롤과 무관하게 우측 고정 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:401-413` · `:704-707` · `:1030-1049` | — | A | 없음 |
| DLG-045 | 헤더 클릭 정렬 | 클릭 = 단일 정렬 3상태 순환(없음→▲→▼→없음) · **Shift+클릭 & 기존 정렬 있음** = 키 추가/방향 순환/제거. 헤더 라벨 = `▲ `/`▼ ` 접두 + 이름 + 순번 배지(①②… — 단일도 ① 표시). 비교는 호스트(`NXGR_SORT`(3) → `sort_spec()` 읽어 재정렬 후 `set_rows`) | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:360-399` · `:666-682` | `nexa_gui::order_badge` | A(로직 N) | `nexa-dir2/crates/nexa-gui/src/columns.rs`(1건) |
| DLG-046 | 그리드 옵션 | `no_header`(목록 모드) · `zebra`(홀수 행 = bg·sel_bg 50% 블렌드, **빈 슬롯까지** 줄무늬) · `outline`(1px 외곽선) · `row_h`(파일 목록과 행 높이 맞춤) · `mark` | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:118-130` · `:962-979` · `:1143-1145` | — | A | 없음 |
| DLG-047 | 데이터 갱신·깜빡임 방지 | `set_rows` = 전체 교체, **선택·포커스·앵커는 새 행 수로 클램프해 유지**(실시간 미리보기 갱신 대응), 스크롤 클램프. `row_check(i)`. 페인트 = 메모리 DC 더블버퍼 + `WM_ERASEBKGND=1` | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:233-261` · `:851-864` · `:1147-1150` | `CreateCompatibleDC` · `BitBlt` | A | 없음 |

### 1-7. NxOrderTree

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-048 | 순서 편집 트리 모델·그리기 | 행 = `(라벨, 레벨 0/1, 체크 Option<bool>)`, 부모 = 직전 레벨 0 행. 행 높이 22 · 체크 열 18(레벨 무관 첫 컬럼 고정) · 셰브론 존 16 · 레벨당 들여쓰기 18. 레벨 0 글자 = `text` / 레벨 1 = `text_dim`. 체크박스 = 12px 각진 테두리 + 안쪽 accent 사각. 선택 행 = `sel_bg`. 1px 외곽선 | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:49-61` · `:145-154` · `:767-877` | GDI(`DrawTextW`) | A | 없음 |
| DLG-049 | 트리 선택 | 클릭 = 단일(이미 선택된 행을 누르면 선택 유지 — 블록 드래그 후보) · Shift+클릭 = **같은 레벨·같은 부모 형제 범위만**(혼합이면 무시). 변경 시 `NXOT_SELCHANGE`(1). `selection()`/`set_selection()`(가시 스크롤 포함) | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:366-399` · `:653-698` | `GetKeyState(VK_SHIFT)` | A(로직 N) | 없음 |
| DLG-050 | 트리 체크 | 체크 존(x 4..22) 클릭 = 토글 → `NXOT_TOGGLE`(2) + `take_toggled()`(1회성). **부모 그룹 체크 해제 시 자식 체크는 값 유지 + 비활성**(흐림·클릭/Space 무시). `checks()` = 전체 상태 | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:139-142` · `:507-519` · `:639-652` · `:804-843` | — | A(로직 N) | 없음 |
| DLG-051 | 그룹 펼침/접기 | 하위가 있는 그룹 행의 셰브론(체크 뒤 — 닫힘 U+E76C · 펼침 U+E70D, Segoe MDL2 12px) 클릭 = 토글. 기본 전부 펼침. 접힘 상태는 **라벨 기준** 유지(`set_rows` 재구성에도 보존), 접힌 자식 선택은 해제. 토글 시 `NXOT_SELCHANGE` 통지 | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:108-132` · `:158-207` · `:628-638` · `:844-861` | `Segoe MDL2 Assets` | A(글리프 P) | 없음 |
| DLG-052 | 드래그 재배치 | 선택 블록(그룹 = 자식 포함)을 5px 넘게 끌면 활성 → **같은 부모 안에서만** 이웃 형제 블록과 스왑(라이브 미리보기) · 커서 y 추종 **고스트 박스**(`sel_bg`+accent 테두리, 다중 = `라벨 (+N)`) · 상하 14px 가장자리 = 60ms 타이머 자동 스크롤(행 높이 단위). 놓으면 뷰는 **원상 복원**하고 `NXOT_DRAGMOVE`(3) + `take_drag_delta()`(형제 칸 이동량, 음수 = 위) — 모델 이동은 호스트. ESC = `cancel_drag()`(시작 상태 복원) | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:63-78` · `:209-298` · `:521-547` · `:581-602` · `:675-693` · `:720-765` · `:893-930` | `SetCapture` · `SetTimer` | A(로직 N) | 없음 |
| DLG-053 | 트리 세로 스크롤 | 내용 > 높이면 우측 오버레이 썸(5px, 드래그 중 8px, 최소 24 — **항상 표시**, 페이드 없음) · 우측 10px 프레스 = 썸 드래그 · 휠 = `delta/120 × 44px`(정수 나눗셈 — **120 미만 터치패드 델타는 버려진다**) | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:177-184` · `:558-568` · `:621-626` · `:709-719` · `:878-892` | `WM_MOUSEWHEEL` | A | 없음 |
| DLG-054 | 트리 키보드 API | 컨트롤 자체는 포커스를 받지 않고(탭스톱 없음) 호스트가 호출: `key_move(up)`(이전/다음 가시 행 단일 선택, 선택 없음 = 첫/끝) · `key_extend(up)`(형제 범위 확장) · `key_toggle()`(단일 선택 행 체크 토글) · `height_for(rows)` | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:401-504` | — | A(로직 N) | 없음 |

### 1-8. 검증 갤러리(ctldemo)

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-055 | 컨트롤 검증 갤러리 창 | 개발 전용(메뉴 비노출) — `CMD_CTLDEMO` → `WM_APP_CTLDEMO`(0x8009)로만 연다. 비모달 740×480 @ (120,120), 이미 열려 있으면 앞으로(중복 가드). 카드 A(라운드·타이틀에 콤보+±)·카드 B(각진·버튼 3상태·메뉴버튼·스핀·세그)·카드 C(전 컨트롤 수평 배치 — 세로 중심 정렬 검증) + 하단 상태줄(받은 통지 `id/code` 표기 = 통지 투과 증명) | `nexa-dir2/crates/nexa-app/src/ctldemo.rs:54-95` · `:123-452` · `:454-512` · `win.rs:5365-5367` · `win.rs:9043-9047` | `FindWindowW` · `STATIC` | A | 없음(창 자체가 수동 검증 도구) |

### 1-9. 공통 대화상자(dialog.rs)

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-056 | 대화상자 글꼴 | `DlgFont{family, size_pt}`(설정 `dlg_font`/`dlg_font_size`) — 크기 7..24pt 클램프 × DPI/72, family는 **쉼표 폴백 체인의 설치된 첫 글꼴**(없으면 `Segoe UI`). 모든 지표(버튼·여백·창 크기)가 글꼴에서 파생. `make_font_pub`으로 설정·툴팁·순서 편집·일괄 이름변경이 공유 | `nexa-dir2/crates/nexa-app/src/dialog.rs:40-43` · `:59-84` · `:295` · `prefs.rs:1008` | `CreateFontW` · `GetDpiForWindow` | A+P(기본 글꼴) | `nexa-dir2/crates/nexa-app/src/fontchain.rs`(1건) |
| DLG-057 | 임의 버튼 모달(메시지 상자 대체) | `show_buttons(owner, title, message, buttons, font)` → 클릭한 버튼 id(1 이상), 닫힘(X) = 0. 메시지 워드랩 전체 표시(높이 실측) · 버튼 폭 = `max(라벨+20, 56)` · 버튼 높이 = 줄 높이+10 · 클라이언트 폭 = `max(버튼 합+24, 380)` · 버튼은 **우측 정렬**(왼→오 순서 유지) · **첫 버튼 = 기본 강조**. 소유자 중앙 **+110px 아래**(진행 창과 비겹침). 소유자 입력 차단. 키 처리 코드 없음 — Enter/Esc/Tab 동작 없음(추정: 루프가 Translate/Dispatch뿐 `dialog.rs:283-287`, 초기 포커스도 버튼에 주지 않음) | `nexa-dir2/crates/nexa-app/src/dialog.rs:33-36` · `:140-177` · `:181-292` · `:300-311` | `AdjustWindowRectEx` · `BUTTON` · `EnableWindow` · 자체 `GetMessageW` 루프 | A | 없음 |
| DLG-058 | 모달의 워커 스레드 호출 | `show_buttons`는 자체 메시지 루프라 **워커 스레드에서 직접 호출**된다 — 전송 덮어쓰기 충돌 4버튼([예]/[모두 예]/[건너뛰기]/[취소])이 전송 워커에서 뜨고 UI 스레드는 진행 표시를 계속 펌프. 그 밖의 호출: 잠긴 파일 삭제(건너뛰기 N/재시도/취소) · 삭제 실패(재시도/닫기) · 클라우드 연결 안내 | `nexa-dir2/crates/nexa-app/src/win.rs:4539-4575` · `:3783-3810` · `:3962-3975` · `:5655` · `:5682` · `:5757` | 스레드별 메시지 큐 | **P** | 없음 |
| DLG-059 | 전송 진행 창 | 비모달(앱 조작 가능) · 클라이언트 폭 400 · 1행 = 작업 라벨 · 2행 = `진행 / 전체 (pct%) · 파일 cur/count`(`ops.fileCount`) · 진행 바(높이 = 줄 높이, 회색 1px 테두리 + #F0F0F0 바탕) · 우하단 [취소]. 소유자 중앙 **−110px 위**. `update(done,total,items,cur,count)` = 값 갱신 + 전체 무효화(erase=true — 텍스트 중첩 방지). 설정 `transfer_close_ms > 0`일 때만 생성 | `nexa-dir2/crates/nexa-app/src/dialog.rs:334-350` · `:531-603` · `:616-740` · `win.rs:2173-2180` · `win.rs:4262-4281` | `BUTTON` · `SetTimer` | A | 없음 |
| DLG-060 | 세그먼트 진행 바 | 항목별 **크기 비례 구간**(누적 경계 — 오차 비누적) · 파일별 5색 순환(#267BD4 · #34C759 · #FF9F0A · #AF52DE · #FF375F) · 완료 = 전체 채움 · 진행 = 부분 채움 · 건너뜀 = 회색 #A0A0A0 · 실패 = 적색 #DC3C3C · 구간 경계선 1px #808080. **모든 항목 최소 3px 보장**(부족분은 가장 넓은 구간에서 1px씩 차감 — 공간이 `n×4 ≤ 폭`일 때). `total==0`·항목 없음·512개 초과 = 단색 바 폴백 | `nexa-dir2/crates/nexa-app/src/dialog.rs:317-331` · `:368-480` | `FillRect` | A(배분 로직 N) | 없음 |
| DLG-061 | 완료 카운트다운 닫기 | `set_done(label, ms)` = 라벨 교체 + [취소] → **[닫기 (N)]**(N = ms 올림 초). 첫 틱 = ms 나머지(최소 50ms), 이후 1초 주기로 N 감소·재라벨, 0 = 자동 닫힘 → 총 대기 = 정확히 ms. 버튼/X = 즉시 닫기. 진행값은 실제 바이트 유지(취소·실패면 부분 진행 그대로), `total==0` 완료는 100% 표기 | `nexa-dir2/crates/nexa-app/src/dialog.rs:357-364` · `:513-530` · `:542-548` · `:754-765` · `win.rs:4295-4303` | `SetTimer`/`KillTimer` | A | 없음 |
| DLG-062 | 진행 창 취소 의미 | 진행 중 [취소] 또는 X = **`cancelled=true`만 기록**(창은 닫지 않음) → 호스트가 `cancelled()` 폴링해 워커 취소 플래그에 반영. `Drop` = 창 파괴 + 글꼴 해제 | `nexa-dir2/crates/nexa-app/src/dialog.rs:490-512` · `:743-745` · `:768-776` · `win.rs:4277-4279` | — | A | 없음 |
| DLG-063 | 바이트 표기 | `fmt_bytes`: 1024 미만 = `N B`, 그 위 = 소수 1자리 KB/MB/GB/TB(1024 기준) | `nexa-dir2/crates/nexa-app/src/dialog.rs:779-792` | — | N | 없음 |

### 1-10. 암호 입력(pwprompt) · About

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-064 | 암호 입력 모달 | 압축 파일이 암호를 요구할 때. 안내(`archive.pw.prompt` + 파일명) · 재시도 시 "암호가 맞지 않습니다"(`danger` 색) 추가 · 라벨+마스킹 입력(●) · [암호 표시] 체크 · 주의 문구(`text_dim`) · [확인(Default)] [취소]. 폭 448(폼 420 + 여백 14×2), 소유자 가로 중앙·세로 1/3. 초기 포커스 = 입력란 | `nexa-dir2/crates/nexa-app/src/pwprompt.rs:33-43` · `:53-219` | `EnableWindow` · 자체 루프 | A | 없음 |
| DLG-065 | 암호 취급 규약 | ① 마스킹 중 복사/잘라내기 불가 ② 확인 시 값을 `Secret`으로 **한 번만** 이동, 경유 UTF-16 버퍼는 즉시 0으로 덮음(`Secret::take_from_u16`) ③ 컨트롤 내용·Undo 버퍼 제거(`clear_secret` — 확인·취소·닫기 전 경로) ④ 창 상태·로그·설정·제목 어디에도 남기지 않음 ⑤ 길이 상한 1024 ⑥ [암호 표시]는 화면 표시만 전환(값 불변)·토글 후 포커스를 입력란으로 | `nexa-dir2/crates/nexa-app/src/pwprompt.rs:1-12` · `:245-265` · `:276-300` | `EM_SETPASSWORDCHAR` | A(규약 N) | `nexa-dir2/crates/nexa-core/src/secret.rs`(4건) |
| DLG-066 | 암호 창 키·재시도 루프 | 펌프가 `WM_KEYDOWN`을 가로챔: **Enter = 확인**(포커스 위치 무관) · **Esc = 취소**. 빈 입력으로 확인 = `None`(취소와 동일). Tab 이동 없음. 호출부 루프: 취소 = 아무것도 열지 않음 · 틀리면 `retry=true`로 재질문 · 성공한 암호만 세션 메모리에 기억 | `nexa-dir2/crates/nexa-app/src/pwprompt.rs:221-242` · `archivewnd.rs:172-190` | — | A | 없음 |
| DLG-067 | About 모달 | 내용(위→아래): 제품명 `Nexa Dir`(본문의 170% · semibold) · 설명(`about.desc`) · `버전 {CARGO_PKG_VERSION}` · 빈 줄(반 줄) · 링크 3개 · 빈 줄 · 라이선스(`about.license`, 흐림 #666666) · 저작권(`about.copyright`, 흐림). 여백 16 · 줄 간격 = 줄 높이/3 · 클라이언트 폭 = `max(최대 줄 폭+32, 360)` · 우하단 [확인](폭 `max(라벨+24, 72)`). 소유자 중앙. 키 처리 없음(DLG-057과 동일 — 추정) | `nexa-dir2/crates/nexa-app/src/about.rs:36-63` · `:235-394` · `win.rs:5369-5371` · `win.rs:9050-9055` | `AdjustWindowRectEx` · `BUTTON` | A | 없음 |
| DLG-068 | About 링크 | accent 색(#267BD4) + 밑줄(히트 존 하단 1px) · 링크 위 = 손 커서 · 클릭 = 기본 브라우저로 열기. URL: 저장소 `https://github.com/SosomLab/nexa-dir2` · 릴리스 `…/nexa-dir2/releases` · 조직 `https://github.com/SosomLab`(홈페이지 확정 시 교체 예정 주석) | `nexa-dir2/crates/nexa-app/src/about.rs:29-34` · `:126-134` · `:154-183` · `:210-223` | `ShellExecuteW("open")` · `IDC_HAND` · `AllowSetForegroundWindow` | **P** | 없음 |

### 1-11. 순서/표시 편집기(ordereditor)

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-069 | 공통 편집 창 + 어댑터 | 창 구현 1개가 `EditorSpec{title, defs, with_vis, flat, locked, label}`로 3용도 구성: **도구 모음**(그룹+자식 · 잠금 없음) · **파일 컬럼**(`flat` — 그룹 헤더 생략 · `name` 잠금) · **컨텍스트 메뉴**(`row`/`bg` 그룹). 진입: 설정 창 필드 3개, 툴바/헤더 우클릭 팝업 | `nexa-dir2/crates/nexa-app/src/ordereditor.rs:36-47` · `:242-348` · `prefs.rs:1011-1028` · `prefs.rs:2368-2391` · `win.rs:6305-6321` | `EnableWindow` · 자체 루프 | A | 없음 |
| DLG-070 | 이동 규칙 | ▲▼ 버튼 / Ctrl+↑↓ / 드래그. 그룹 행 선택 = **블록 통째** 이동 · 자식 = **그룹 안에서만** · 연속 다중 선택 = 일괄 한 칸(`shift_range` — 경계면 무동작). 이동 후 트리 재구성 + 같은 항목 재선택. 드래그 확정 = delta만큼 한 칸 이동을 반복 적용. ▲▼는 **선택이 없으면 비활성** | `nexa-dir2/crates/nexa-app/src/ordereditor.rs:114-186` · `:222-236` · `:358-364` | — | A(모델 로직 N) | 없음 |
| DLG-071 | 표시 체크 규칙 | 자식 체크 = 표시/숨김(잠금 key는 거부 — 트리 재설정으로 원복) · **그룹 체크 = 통째 숨김(자식 상태 보존)** · `with_vis=false`면 체크 열 없음 | `nexa-dir2/crates/nexa-app/src/ordereditor.rs:62-86` · `:189-218` | — | A(모델 로직 N) | `nexa-dir2/crates/nexa-app/src/config.rs`(6건 중 순서 파싱 관련 — 건수 추정) |
| DLG-072 | 편집기 키보드 | ↑/↓ = 행 선택 · Shift+↑/↓ = 형제 범위 확장 · Ctrl+↑/↓ = 순서 이동 · Space = 체크 토글 · **Esc = 드래그 취소(진행 중이면) → 아니면 창 닫기**. 키는 대화상자 창이 받는다(생성 직후 `SetFocus(dlg)`) | `nexa-dir2/crates/nexa-app/src/ordereditor.rs:339` · `:368-411` | `GetKeyState` | A | 없음 |
| DLG-073 | 실시간 적용 통지 | 변경할 때마다 직렬화 문자열을 소유자에 동기 통지: `WM_APP_ORDER_EDIT`(0x8007, wparam = field 태그, lparam = `*const String` — 통지 동안만 유효). OK/취소 없음 — 닫기 = 완료. 수신측이 즉시 적용·저장(툴바 재구성·컬럼 레이아웃·설정 값) | `nexa-dir2/crates/nexa-app/src/ordereditor.rs:27-29` · `:102-111` · `win.rs:8997-9037` · `prefs.rs:2335` | `SendMessageW` | A | 없음 |

### 1-12. 일괄 이름 변경 창(bulkrename — UI)

| ID | 기능 | 동작 상세 | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| DLG-074 | 창·3영역 구성 | 모달 · 클라이언트 880×620 고정(리사이즈 불가) · 소유자 중앙. 좌 = 카드 스택(폭 320 + 썸 트랙 10) · 우 = 미리보기 그리드 · 하단 행 = `[… ⌄]` + 건수 / 검증 오류 / [취소][Rename]. 선택 없음이면 창을 열지 않고 제목 표시줄에 `bulk.noSelection` | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:39-47` · `:2027-2252` · `win.rs:6424-6444` | `AdjustWindowRectEx` · 자체 루프 | A | 없음 |
| DLG-075 | 카드 스택 = 파이프라인 | 카드 1장 = 동작 1블록(위→아래 순서 적용). 타이틀 = 동작 종류 콤보(6종) + 우측 **＋**(이 카드 **아래에** 종류 0 카드 추가, 보이도록 스크롤) / **−**(이 카드 삭제). **마지막 1장은 삭제 불가**(− 비활성 이미지). 종류 변경 = 그 카드 본문만 재구성 + 높이 재배치 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:104-111` · `:385-416` · `:810-880` · `:1835-1893` | PNG 자산 4종(`assets/rename/*-32.png`) | A | 없음 |
| DLG-076 | 동작 6종 본문 | ① Replace Text: 대상·Mode(모든/첫/마지막/전체)·대소문자·찾기·바꾸기 ② Replace RegEx: 대상·대소문자·정규식·바꾸기 ③ Insert Text: 대상·위치(스핀 0..999 + `→ abc`/`← abc`[기본 뒤])·텍스트 ④ Change Case: 대상·세그 `AB CD`/`Ab Cd`/`Ab cd`/`ab cd` ⑤ Add Number: 대상·Padding 콤보 6종(기본 `001…`)·위치·Start(기본 1, −9999..9999)+Prefix·Step(기본 1)+Suffix ⑥ Add Date: 대상·종류(수정/생성)·위치·Prefix+Suffix·Format(기본 `${YYYY}-${MM}-${DD}`)+`?` 도움말. 공통 대상(Apply to) = 이름/이름+확장자/확장자/확장자(점 포함) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:886-1247` | — | A | 없음 |
| DLG-077 | 폼 → 동작 수확 규칙 | 카드별 `op_from_form`: Replace = 찾기 빈 값이면 무효(단 Mode=전체는 허용) · RegEx = 항상 All·찾기 빈 값 무효 · Insert = 텍스트 빈 값 무효 · Case/Number/Date = 항상 유효 · Date 포맷 빈 값 = 기본, 구식 포맷은 `${}` 문법으로 자동 이행. 무효 카드는 파이프라인에서 건너뜀. 프리셋 복원 = `set_form_from_op`(Move/ChangeExt는 카드가 없어 스킵) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:238-356` · `:377-382` · `:518-528` · `:1250-1351` | — | N(컨트롤 읽기만 A) | `nexa-dir2/crates/nexa-ops/src/batch_rename.rs`(14건 — 코어) |
| DLG-078 | 카드 영역 스크롤 | 카드 합이 뷰포트보다 길면 세로 스크롤(하단 행 불침범). 휠 = 48px/노치 + 분수 누적·고속 가속 · 오버레이 썸(6px, 드래그 시 10px+트랙, 900ms 소등, 최소 24). 카드 내부 입력에 포커스가 있을 때의 휠은 대화상자가 받아 **커서 x < 342**면 카드 호스트로 전달 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:545-806` · `:1817-1827` | `WM_MOUSEWHEEL` · `SetCapture` | A | 없음 |
| DLG-079 | 실시간 미리보기 | 폼 편집(코드 1 또는 `EN_CHANGE`)마다 재계산: 수확 → 정규식 검증(오류 = 하단 `⚠ #블록번호: 메시지`) → 새 이름 → 충돌 검사(빈 이름/금지 문자/중복/기존 파일 존재[파일시스템 조회]/중첩). 그리드 행: 변경 = 체크 + 새 이름 · 충돌 = 체크 없음 + `⚠ 새이름 (사유)` · 무변경 = 이후 칸 빈칸. 건수 = `bulk.count`(체크된 변경 행 수). **[Rename] 활성 = 동작 ≥1 · 변경 ≥1 · 검증 통과 · 충돌 0** | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:358-367` · `:420-514` · `:1998-2004` | `Path::exists` | N(표시 A) | 코어 14건(상동) |
| DLG-080 | 행별 적용 제외 | 적용 열 체크 해제 = 그 항목 제외(건수·활성 재계산) · 헤더 체크 = 전체 토글. 표시 행 ↔ 항목 인덱스는 `order` 맵으로 왕복(정렬 중에도 정확) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:132-137` · `:1937-1961` | — | N(표시 A) | 없음 |
| DLG-081 | 미리보기 정렬 | 헤더 클릭(DLG-045) → 컬럼 1(이전)·2(이후) 기준 대소문자 무시·안정 정렬·다중 키. 표시 순서만 재배열(연번은 원래 선택 순서 기준 유지) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:471-497` · `:1962-1966` | — | N | 없음 |
| DLG-082 | 프리셋 메뉴·불러오기 | 하단 좌 `[… ⌄]` 항목 = 저장된 프리셋들(이름순, 최대 64) → 구분선 → `Save Renaming Sequence…` → `Edit Renaming Sequences…`. 프리셋 클릭 = 카드 스택 전체 교체(빈/전부 스킵 = 기본 카드 1장) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1354-1422` · `:1894-1918` | `std::fs::read_dir` | A(파일 I/O N·경로 P) | 없음 |
| DLG-083 | 프리셋 저장(이름 팝업) | 파이프라인이 비어 있으면 무시. 캡션 없는 라운드 팝업 360×150: 상단 라벨 · 글상자(기본 이름 `bulk.preset.savedSeq` **전체 선택**) · 우하단 [취소][OK]. 이름에서 `<>:"/\|?*` 제거 + trim, 빈 값이면 저장 안 함. 같은 이름 = 덮어쓰기(확인 없음). 저장 후 메뉴 재구성 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1434-1474` · `:1521-1623` · `:1919-1930` | `DwmSetWindowAttribute`(Win11 라운드) | A(라운드 P) | 없음 |
| DLG-084 | 프리셋 관리 팝업 | 캡션 없는 라운드 팝업 340×360: 무헤더·지브라·외곽선 목록(행 높이 = 파일 목록과 동일 20px@96dpi) + 행별 빨간 ⊖ · 우하단 [취소][OK]. ⊖ = 목록에서 즉시 제거(**스테이징**) · OK = 실제 파일 삭제 · 취소 = 폐기 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1476-1481` · `:1628-1809` · `:1931-1935` | `std::fs::remove_file` | A | 없음 |
| DLG-085 | 날짜 포맷 도움말 | `?` 버튼 → `${YYYY}`·`${YY}`·`${MMMM}`·`${MMM}`·`${MM}`·`${M}`·`${DD}`·`${D}`·`${DDD}`·`${HH}`·`${H}`·`${mm}`·`${m}`·`${ss}`·`${s}` 예시 + i18n 안내 한 줄을 **OS 메시지 상자**로 표시 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1843-1865` | `MessageBoxW` | A(OS 상자 → 자체 모달) | 없음 |
| DLG-086 | 적용/취소 결과 | [Rename] = 변경되고 제외되지 않은 항목의 `(전체 경로, 새 이름)` 목록 반환 → 호출부가 순차 rename + **Undo 1건**(배치 전체 되돌림), 실패는 개별 격리. [취소]/X/빈 결과 = `None`. Enter/Esc 미동작(추정 — `IsDialogMessageW`가 IDOK/IDCANCEL을 code 0으로 보내는데 핸들러는 code 1만 처리) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:1967-1997` · `:2245-2251` · `win.rs:6447-6460` | — | N(버튼 A) | 없음 |
| DLG-087 | 라벨 열 폭 i18n 실측 | 라벨 열 폭 = 전 종류 공통 라벨 15개 중 **현재 언어 최대 폭**(56..140 클램프)+6 → 카드 간 열 위치 일치. 둘째 라벨(Prefix/Suffix) 폭 = 28..90 클램프+6. 라벨 = 우측 정렬 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:907-954` | DLG-005 | A | 없음 |
| DLG-088 | 대상 수집 | 선택 순서 보존(연번 기준). 항목 = (부모, 이름, 폴더 여부, 수정 ms, 생성 ms) — 메타 조회 실패/미지원 = 0(Date 동작이 무변경으로 격리). 날짜 표기 TZ 오프셋(분)은 호스트 전달 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:2054-2079` | `std::fs::metadata().created()` | N(생성 시각 P) | 없음 |

---

## 2. 화면·컨트롤 배치

단위 = px @96dpi(dir2는 이 창들에서 좌표를 DPI로 스케일하지 않고, 글꼴만 DPI 반영 — `dialog.rs:60-61`). `fh` = 글꼴 높이(tmHeight ≥ 12), `auto` = fh+8, `compact` = fh+4.

### 2-1. 일괄 이름 변경(`NexaBulkRename`) — `nexa-dir2/crates/nexa-app/src/bulkrename.rs:2114-2228`

```
클라이언트 880 × 620 · PAD 12 · 창 배경 = 흰색(COLOR_WINDOW)
┌──────────────────────────────────────────────────────────────────────────┐
│(12,12) 카드 호스트 330×567        │(344,12) 미리보기 그리드 524×567        │
│  ┌ 카드 0 (x0, 폭 320) ───────┐  │  [☑][ 이전          ][ 이후          ] │
│  │[종류 콤보 170×24]   (＋)(−)│  │   체크 열 폭 = fh+14                   │
│  │ 라벨열 │ 컨트롤열          │  │   이전/이후 = (524−체크열)/2 씩        │
│  └────────────────────────────┘  │   행 높이 = 20(@96dpi, 최소 14)        │
│   … 카드 간격 8 …      ┆썸 10px  │                                        │
├──────────────────────────────────┴────────────────────────────────────────┤
│(12,589)[… ⌄]48  (70,593) N개 항목 변경  (344,593) ⚠오류 304w   [취소][Rename]│
└──────────────────────────────────────────────────────────────────────────┘
```

- 하단 기준선 `by = 620−12−21 = 587`. 버튼·메뉴버튼 y = `by+2`, STATIC(건수·오류) y = `by+6`, 높이 18.
- [Rename](Default)·[취소] = 자동 폭(라벨+32)·컴팩트 높이, **우하단 정렬**: Rename 최우측(`880−12−폭`), 취소는 그 왼쪽 8px(`place_buttons_br` — `bulkrename.rs:1498-1517`).
- 카드 높이(종류별, `bulkrename.rs:531-540`) = `34 + 8 + 28×마지막행 + 23 + 8`: Replace 185 · RegEx 157 · Insert 129 · Case 101 · Number 185 · Date 185.
- 카드 내부(`bulkrename.rs:810-954`): 타이틀 밴드 34(콤보 x=8, 세로 중앙 · ± 는 지름 `fh`, 우측에서 `8+fh` / `8+2fh+6`) · 본문 `bx=10`, `bw=300`, 행 y = `42 + 28×k` · 라벨 열 폭 `lbl_w`(DLG-087) · 컨트롤 열 `cx = bx+lbl_w+6`, `cw = bw−lbl_w−6`. 타이틀 밴드 위 컨트롤은 `behind = sel_bg`, 필 = #D2D6DC(한 단계 진하게).
- 종류별 행 배치(행 0 = 대상 콤보 공통):

| 종류 | 행 1 | 행 2 | 행 3 | 행 4 |
|---|---|---|---|---|
| Replace Text | Mode 콤보(cw) | 대소문자 체크(박스만) | 찾기 글상자(cw) | 바꾸기 글상자(cw) |
| Replace RegEx | 대소문자 체크 | 정규식 글상자 | 바꾸기 글상자 | — |
| Insert Text | 위치: 스핀 70 + 세그(x=cx+76, 폭 cw−76) | 텍스트 글상자 | — | — |
| Change Case | 세그 4칸(cw) | — | — | — |
| Add Number | Padding 콤보 | 위치: 스핀+세그 | Start 스핀 70 · Prefix 라벨(x=cx+76)+글상자 | Step 스핀 70 · Suffix 라벨+글상자 |
| Add Date | 종류 콤보 | 위치: 스핀+세그 | Prefix 글상자 70 · Suffix 라벨+글상자 | Format 글상자(cw−fh−6) + `?`(지름 fh, 우측 끝) |

- 컨트롤 id(카드 지역 — 같은 id가 카드마다 반복): `ID_KIND 1` · `ID_ADD 2` · `ID_DEL 5` · `ID_SCOPE 7` · `ID_FIND 10` · `ID_WITH 11` · `ID_MC 12` · `ID_RXF_MC 13` · `ID_RX_MODE 14` · `ID_RXF_FIND 15` · `ID_RXF_WITH 16` · `ID_CASE_BASE 20` · `ID_INS_TEXT 30` · `ID_INS_OFF 33` · `ID_INS_DIR 34` · `ID_NUM_START 40` · `ID_NUM_STEP 41` · `ID_NUM_PAD 42` · `ID_NUM_OFF 45` · `ID_NUM_DIR 46` · `ID_NUM_WPRE 47` · `ID_NUM_WSUF 48` · `ID_DT_KIND 90` · `ID_DT_FMT 91` · `ID_DT_OFF 92` · `ID_DT_DIR 93` · `ID_DT_PRE 94` · `ID_DT_SUF 95` · `ID_DT_HELP 96` · 창 직속 `ID_PRESET_MENU 71` · `ID_APPLY 80` · `ID_CANCEL 81` · `ID_COUNT 82` · `ID_PREV 84`(`bulkrename.rs:50-99`).
- 탭 순서 = 생성 순서: 카드 0(콤보 → ＋ → − → 본문 컨트롤 위→아래) → Rename → 취소 → 그리드 → `[… ⌄]`. 나중에 추가한 카드는 화면 위치와 무관하게 **호스트 자식 목록 끝**에 붙는다(삽입 위치 ≠ 탭 순서 — dir3에서는 화면 순서로 정리 권장).
- 단축키: 없음(Enter/Esc 미동작 — DLG-086). 콤보/세그/스핀/그리드의 방향키·Space는 각 컨트롤 규약.

### 2-2. 프리셋 이름 팝업(`NexaPromptName`) · 관리 팝업(`NexaManagePresets`)

- 이름 팝업 360×150(캡션 없음 · 1px 테두리 · Win11 라운드) — `bulkrename.rs:1536-1606`: PAD 20 · 라벨 (20,20, 폭 320) · 글상자 (20,48, 폭 320, auto) · 버튼 y = `150−20−24 = 106`, OK 최우측·취소 왼쪽 8px. 탭 순서: 글상자 → OK → 취소.
- 관리 팝업 340×360 — `bulkrename.rs:1713-1787`: PAD 20 · 목록 (20,20, 폭 300, 높이 `316−40 = 276`) · 버튼 y = 316. 목록 = 단일 컬럼(폭 300)·무헤더·지브라·외곽선·행 우측 ⊖.

### 2-3. 순서 편집기(`NexaOrderEditor`) — `nexa-dir2/crates/nexa-app/src/ordereditor.rs:262-325`

- PAD 14 · 트리 (14,14) 폭 260 · 높이 = `min(행 수, 12)×22 + 2`(12행 초과 = 내부 스크롤).
- ▲ = (284, 16) 16×16 · ▼ = (284, 40) 16×16(SVG `assets/ui/arrow-up.svg`/`arrow-down.svg`).
- 창 클라이언트 근사 = 폭 `14×2+260+40 = 328` · 높이 `14×2+24+트리높이+8`, 창 높이에 캡션 보정 +30(AdjustWindowRect 미사용). 소유자 중앙. 제목 = 스펙별(`pref.toolbarOrder`/`pref.colLayout`/`pref.ctxMenuOrder`).
- 트리 행 내부(`ordertree.rs:786-876`): 체크박스 x=6(12px) · 셰브론 존 x=21..37 · 라벨 x = `41 + 레벨×18`.
- 단축키: DLG-072.

### 2-4. 암호 입력(`NexaPwPrompt`) — `nexa-dir2/crates/nexa-app/src/pwprompt.rs:67-213`

`lh = max(fh,12)`, `row = lh+12`, PAD 14, 폼 폭 420, 라벨 폭 72, 버튼 폭 88.

| 순서 | 컨트롤 | 위치(x, y) | 크기 |
|---|---|---|---|
| 1 | 안내 라벨(좌) | (14, 14) | 420 × lh |
| 2 | (재시도 시) 오류 라벨(danger) | (14, 14+lh) | 420 × lh |
| 3 | "암호" 라벨 | (14, y₁ = 14+lh×줄수+10) | 72 × row |
| 4 | 글상자(마스킹) | (86, y₁) | 348 × row |
| 5 | [암호 표시] 체크 | (86, y₁+row+8) | 348 × row |
| 6 | 주의 문구(dim) | (14, y₁+2row+14) | 420 × lh |
| 7 | [확인] Default | (250, y₁+2row+14+lh+14) | 88 × row |
| 8 | [취소] | (346, 동일) | 88 × row |

창 = 448 × (클라이언트 높이+30). 위치 = 소유자 가로 중앙·세로 1/3. Enter = 확인, Esc = 취소.

### 2-5. About(`NexaAbout`) · 메시지 모달(`NexaDlg`) · 진행 창(`NexaProgress`)

- About — DLG-067 표 참조. 창 배경 = COLOR_BTNFACE(회색). 본문 x=16 시작, 버튼 우하단(여백 16).
- 메시지 모달(`dialog.rs:190-280`): PAD 12 · 본문 (12,12, 폭 `client_w−24`, 워드랩) · 버튼 y = `12+text_h+12` · 버튼 간격 6 · 우측 정렬. 배경 = COLOR_BTNFACE. 창 아이콘 = 앱 아이콘.
- 진행 창(`dialog.rs:644-720` · `:549-598`): 클라이언트 400 × `(12 + 2lh+4 + 6 + lh + 8 + btn_h + 12)` · 라벨 (12,12) · 정보 (12, 12+lh+4) · 바 (12, 정보 아래+6, 폭 376, 높이 lh) · [취소] 우하단(폭 `max(라벨+20, 64)`, 높이 lh+10).

### 2-6. 검증 갤러리(`NexaCtlDemo`) — `nexa-dir2/crates/nexa-app/src/ctldemo.rs:123-452`

- 카드 A (16,16, 폭 330, 타이틀 34 + 본문 240, 라운드 10): 타이틀에 동작 콤보 170×24 + ＋/−(− 비활성) · 본문 = 적용 대상 콤보(자동 높이) · Mode 콤보(26) · 3단 체크 · 찾기 글상자.
- 카드 B (366,16, 폭 330, 타이틀 26 + 본문 140, 각짐): 위치 스핀 90×26 · 세그(앞에서/뒤에서) · 버튼 3상태(Cancel/Rename/비활성) · 메뉴 버튼.
- 카드 C (16,300, 폭 680, 타이틀 24 + 본문 54): 콤보·글상자·체크·아이콘버튼·버튼·세그·스핀을 한 줄에(세로 중심 정렬 검증).
- 상태줄 라벨 (16,410, 폭 680).

---

## 3. nexa-ui 매핑

nexa-ui 컨트롤 모델(확인): **OS 자식 창이 아니라** `Widget`(bounds/on_event/paint) + `Control`(공통 베이스) 구조체를 호스트가 소유하고, 이벤트는 `InputEvent`로 라우팅, 결과는 `take_*()` **1회성 폴링**으로 읽는다(`nexa-ui/crates/nexa-ctl/src/widget.rs:44` · `controls/mod.rs:278` · `event.rs:49`). 팝업은 별도 창이 아니라 **같은 표면 위 오버레이**로 그린다(`controls/combo.rs:309` popup_rect · `controls/ctxmenu.rs:1139`).

| dir2 | nexa-ui 대응 | 상태 | 차이 / 추가 필요 API |
|---|---|---|---|
| `base`(상태 박스·클래스 등록) | `ControlBase` + `Control` 트레이트(`controls/mod.rs:228` · `:278`) | 있음 | 구조체 소유로 대체 — 이식 대상 아님 |
| `notify` → `WM_COMMAND` | `take_clicked`/`take_toggled`/`take_changed` 폴링(`controls/button.rs:324` · `checkbox.rs:60` · `combo.rs:282`) | 있음 | 호스트 이벤트 루프가 폴링 → 액션 enum(nexa-sql `AboutAction` 방식 `nexa-sql/crates/nexa-sql/src/about_win.rs:17`) |
| `Style` 8색 | `Theme`(`nexa-ui/crates/nexa-ctl/src/theme.rs:12`) | 있음(부분) | 대응: bg→`field_bg`/`panel_bg` · border→`border` · text→`text` · text_dim→`text_dim` · accent→`accent` · sel_bg→`sel_bg` · danger→`danger`. **`behind` 없음**(SDF AA가 배경과 직접 블렌드 — 불필요, 추정). 라이트/다크는 Theme가 담당 |
| 자동 높이·실측 | `DrawCtx::text_height`/`text_width`/`text_center_y`(`draw.rs:94` · `:68` · `:109`) | 있음 | "auto = 글꼴+8 / compact = 글꼴+4" 규칙은 호스트 레이아웃 헬퍼로 재현(nexa-ui에 preferred-size API 없음 — 버튼 자동 폭 포함). **추가 권장**: `preferred_size(ctx)` 류 헬퍼 |
| Tab 내비(`IsDialogMessageW`) | 없음 — `Key`에 Tab 없음(`event.rs:12-45`), 호스트가 직접 포커스 인덱스 관리(nexa-sql `input_win.rs:465`) | **없음** | **추가 필요**: 폼 호스트용 포커스 체인(순서 목록·Tab/Shift+Tab·Enter=기본 버튼·Esc=취소) 공용 모듈 |
| NxButton | `Button`(`controls/button.rs:90`) | 있음 | Default 강조 = `ButtonTone::Accent`(`:84`) · 비활성 = `set_enabled`. 차이: nexa-ui는 **MouseUp에서 클릭**(`:397-405`)·hover 페이드·**연타 가드 350ms**(`:22` — 카드 ＋/스핀 연타용은 `set_rapid` `:329`)·자동 폭 없음 |
| NxCheckBox(2단) | `Checkbox`(`controls/checkbox.rs:19`) | 있음 | 박스 = 12px×배율 고정(글꼴 높이 연동 아님 `:13`) · Enter도 토글(`:131`) · `LabelSide::None`으로 박스만 |
| NxCheckBox(3단 — 부분) | — | **없음** | **추가 필요**: tri-state(값 0/1/2 · 순환 · 흐릿한 ✓ 글리프). `draw_checkbox_glyph`(`controls/mod.rs:516`) 확장 — 그리드 헤더 체크와 공용 |
| NxLabel | — (호스트가 `ctx.text` 직접) | **없음** | **추가 권장**: `Label`(좌/우 정렬·자동 높이·말줄임) 또는 폼 레이아웃 헬퍼. 클릭 투과는 Widget 모델에서 자연 충족 |
| NxComboBox | `Combo`(`controls/combo.rs:534`) + `ComboControl`(`:189`) | 있음 | ✓ 표시(`:231`) · ↓ 열기 · ↑↓/Enter/Esc(`:1115-1132`) · **창 하단에서 위로 펼침**(`:320-329`, dir2에 없던 개선). 차이: 항목 = (value,label) · Enter/Space도 열기 · 12행 창 없음(전 항목 표시 — 항목 많으면 넘침, 추정) · 팝업이 창 표면 안으로 제한 |
| NxTextBox | `TextBox`(`controls/textbox.rs:812`) | 있음 | placeholder(`:812`) · 포커스 링(`:1735`) · 선택/Undo/클립보드/IME 조합 자체 구현 · `select_range`(`:3093`) · `take_changed`(`:3063`). 우측 정렬 없음 |
| 암호 모드 | `TextBox::set_masked`(`:2129`) · `take_secret_text`(`:3259`) · `wipe`(`:3253`) · 마스킹 중 복사 차단(`:3314-3318`) | 있음 | nexa-sql `InputWin::open_password`(`nexa-sql/crates/nexa-sql/src/input_win.rs:97`)가 같은 용도의 선례 |
| NxSearchBox | `TextBox::with_clearable`(`:2620`) + placeholder | 있음 | ×는 값 있을 때만 표시 — 동작 일치 |
| NxFontBox | — | **없음** | **추가 필요**: `FontPicker` = 편집 입력 + 드롭 목록(항목을 **그 글꼴로** 렌더 · 접두 타입어헤드 · PgUp/PgDn · 쉼표 체인 `apply_pick`). 선행 조건: ① 설치 글꼴 **패밀리 목록 API**(nexa-font에는 `find_font_by_family`(`nexa-ui/crates/nexa-font/src/lib.rs:304`)만 있고 열거 공개 API 없음 — 내부 `walk_font_dirs`(`:288`) 활용) ② `DrawCtx`에 **임의 패밀리 글꼴로 텍스트** 그리기(현재 `select_font(slot,bold)`뿐 `draw.rs:45`) |
| NxSegmented | — (`RadioGroup`은 세로 나열 `controls/radio.rs:1`) | **없음** | **추가 필요**: `Segmented`(가로 등폭·컨테이너+accent 필·gap/corner 옵션·←/→·선행 화살표 글리프). 화살표는 `GlyphKind::ArrowBack/ArrowForward`(`controls/glyphs.rs:13`) 벡터로 대체 |
| NxSpin | — | **없음** | **추가 필요**: `Spin`/`NumberStepper`(TextBox 조합 + ⌃⌄ 블록 · min/max 클램프 · ↑/↓ · 한계 방향 비활성 · **우측 정렬** · 숫자 필터 = `set_char_filter`(`controls/textbox.rs:2097`)). 음수 입력 허용 여부는 결정 필요(dir2 = 타이핑 불가) |
| NxGroupCard | — | **없음** | **추가 필요**: `GroupCard`(타이틀 밴드+본문 그리기 · `title_rect`/`body_rect`). 자식 창이 없으므로 통지 투과·리전 클립은 불필요 — 순수 페인터+기하 |
| NxIconButton(벡터) | `Button::glyph(MenuIcon)`(`controls/button.rs:156`) · `Button::icon`(`:165`) | 있음(부분) | 원판(disc) 스타일·＋/−/?/∧/∨ 글리프 세트 없음 → `GlyphKind` 추가 또는 원형 톤 추가 |
| NxIconButton(PNG 쌍) | `Button::icon` + `image_fill(ImageFit)`(`:263`) · 디코드 = `nexa_gfx::image::decode`(`nexa-ui/crates/nexa-gfx/src/image.rs:73`) | 있음(부분) | **비활성 이미지 쌍 전환 없음**(→ 호스트가 enabled에 따라 이미지 교체하거나 `set_disabled_image` 추가) · `ImageFit` = Contain/Cover(`:48`)로 dir2 Native/Stretch와 다름 · 배경 필/테두리를 끄는 "이미지만" 모드 필요(추정 — `:433` 항상 `field_bg` 필) |
| NxIconButton(SVG) | — (nexa-ui는 SVG 파서 없음 — 글리프는 코드 도형) | **없음** | dir2 `svg.rs`(692줄 파서 — 플랫폼 중립, 추정) 이식 + 래스터(알파 마스크 → `IconImage::from_alpha_tinted` `nexa-ui/crates/nexa-gfx/src/surface.rs:81`). 또는 ▲▼를 `GlyphKind::ArrowUp` 계열로 대체 |
| NxMenuButton | `ContextMenu`(`controls/ctxmenu.rs:272` — `open_at` `:528` · `open_beside` `:563` · `CtxItem::Separator` `:131` · `set_max_rows` `:385` · `take_picked` `:1134`) + `Button` | 조합 가능 | **얇은 `MenuButton` 추가 권장**(… ⌄ 본체 + 메뉴 열기/닫기 + 픽 보고). 픽 결과가 인덱스가 아니라 **id 문자열** — 프리셋은 `preset:<이름>`/`save`/`edit` id로 매핑. dir2의 12행 잘림 한계는 `set_max_rows`+스크롤로 해소 |
| NxGrid | `TreeGrid`(`controls/tree.rs:675`) = 헤더·배지(`:651`)·단일 선택·`set_marked_paths`(`:730`)·오버레이 `ScrollBars`·`set_column_width`(`:748`) | **부족** | 없는 것: 체크 마크 열+헤더 3상태 · 행 우측 ⊖ · Shift/Ctrl 다중 선택 로직(이벤트 처리는 단일 선택뿐 `:447-457`) · 헤더 클릭 정렬 · 헤더 경계 드래그(호스트 몫) · 지브라 · 무헤더 · 외곽선 · 픽셀 스크롤 · 빈 영역 클릭 해제. 계획된 `nexa-grid`(`VirtualRows<S: RowSource>` + `CellKind::Check/Button`)는 **문서만 있고 크레이트 미존재**(`nexa-ui/docs/21-grid-family.md:23-36` · `:74-82`; `nexa-ui/crates`에 nexa-grid 없음). → **추가 필요**: 평면 `Grid`(dir2 NxGrid 계약) 신설 또는 nexa-grid G-1/G-2 선행 |
| 오버레이 스크롤바 | `ScrollBars`(`controls/scroll.rs:320`) | 있음 | 얇음 6 / 두꺼움 11 / 최소 썸 28 / 숨김 지연 2000ms 전역(`:20-29` · `:37`) — dir2는 6/10/24/900ms. 값 차이는 설정으로 흡수 |
| NxOrderTree | — (`TreeView` `controls/tree.rs:493` = 체크·드래그·형제 범위 없음) | **없음** | **추가 필요**: `OrderTree`(행 = 라벨/레벨/체크 · 형제 범위 선택 · 부모 해제 시 자식 비활성 · 접기 · 드래그 스왑+고스트+가장자리 자동 스크롤 · delta 보고). 드래그 재배치 선례 = `TabBar`(`controls/tabbar.rs:1545`) · `ToolDock::reorder_to`(`controls/tooldock.rs:632`). 셰브론 = `draw_chevron_90`(`controls/mod.rs:729`) |
| 메시지 모달(`show_buttons`) | — (nexa-dlg는 `FilePicker`뿐 `nexa-ui/crates/nexa-dlg/src/lib.rs:154`) | **없음** | **추가 필요**: `MessageBox`(워드랩 본문 + N버튼 우측 정렬 + 기본 버튼) — nexa-dlg 또는 앱 공통. 워드랩 = `wrap_text`(`controls/mod.rs:490`) |
| 진행 창 + 세그먼트 바 | — | **없음** | **추가 필요**: `SegmentedProgress`(크기 비례 구간·5색·최소 3px·상태색) 페인터. 카운트다운 닫기 버튼 = `TimeoutButton`(`controls/timeout_button.rs:33` — 남은 시간 표기·만료 자동 발화)로 대응 가능 |
| 암호 모달 | nexa-sql `InputWin`(앱 코드 — nexa-ui 아님) | 선례 있음 | dir3 앱에 보조 창으로 구현(nexa-sql `winhost::open_window` 골격 `nexa-sql/crates/nexa-sql/src/winhost.rs:36`) |
| About | nexa-sql `AboutWin`(`nexa-sql/crates/nexa-sql/src/about_win.rs:27`) | 선례 있음 | 링크 텍스트 = `LinkStyle`(TextBox용)은 있으나 단독 하이퍼링크 컨트롤 없음 → 호스트가 히트 존 직접 처리 또는 `LinkLabel` 추가 |
| 갤러리(ctldemo) | — | 없음 | dir3 개발용 갤러리 창 + 스냅샷 테스트 하네스로 재구성(§7) |

---

## 4. OS 분기점

| # | Windows 현 구현 | macOS | Linux | 비고 |
|---|---|---|---|---|
| 1 | 모달 = 소유 창 + `EnableWindow(owner,false)` + **중첩 메시지 루프**(`dialog.rs:281-289`) | winit 보조 창(소유 관계는 `NSWindow addChildWindow` — winit `with_parent_window` 사용 가능 여부는 추정), 모달성은 앱이 흉내(메인 창 입력 무시 + 포커스 되돌림) | winit 보조 창(X11 `WM_TRANSIENT_FOR` / Wayland `xdg_toplevel.set_parent`), 동일 | winit은 중첩 루프를 쓸 수 없다 → **모든 `show() -> 결과` 동기 API를 "열기 + 결과 이벤트" 비동기로 재설계**. nexa-sql은 Windows에서만 owner 지정(`nexa-sql/crates/nexa-sql/src/winfocus.rs:16-24`) — 타 OS 처리 확인 필요(추정). 대안 = 창 안 오버레이 모달(`nexa-ui/docs/20-file-management-and-dialogs.md` §4 권장안) |
| 2 | 워커 스레드에서 모달 직접 표시(`win.rs:4564`) | **불가**(AppKit 창은 메인 스레드 전용) | winit 창 생성은 이벤트 루프 스레드 | 워커 → UI 요청 채널 + 응답 대기(`EventLoopProxy::send_event` + `mpsc`/`oneshot`). 덮어쓰기 4버튼·"모두 예" 기억 로직은 워커 쪽 유지 |
| 3 | 드롭 팝업 = 별도 `WS_POPUP` 창(창 밖으로 나갈 수 있음) | 같은 표면 오버레이 | 동일 | 팝업이 창 경계 안으로 제한 → 하단 `[… ⌄]`는 **위로 펼침** 필요(`ContextMenu::open_at`의 host 클램프), 360×150 이름 팝업처럼 작은 창은 콤보 배치 주의 |
| 4 | 설치 글꼴 열거 `EnumFontFamiliesExW`(`fontbox.rs:77-91`) | CoreText `CTFontManagerCopyAvailableFontFamilyNames` 또는 폰트 디렉터리 스캔(`/System/Library/Fonts` · `/Library/Fonts` · `~/Library/Fonts`) + name 테이블 | fontconfig(`fc-list : family`) 또는 `/usr/share/fonts` · `~/.local/share/fonts` · `~/.fonts` 스캔 | nexa-font의 `walk_font_dirs`(`nexa-ui/crates/nexa-font/src/lib.rs:288`)를 패밀리 목록 API로 공개하는 방식이 OS API 의존 0(기조 부합) |
| 5 | 대화상자 기본 글꼴 `Segoe UI`(`dialog.rs:64-67`) | 시스템 UI 글꼴 | 시스템 UI 글꼴 | `nexa_font::system_ui_font()`(`nexa-ui/crates/nexa-font/src/lib.rs:332`) |
| 6 | 글리프 글꼴 `Segoe MDL2 Assets`(세그 화살표 E72A/E72B · 트리 셰브론 E76C/E70D) | 없음 | 없음 | 전 OS 공통 **벡터 글리프**로 교체(`GlyphKind` · `draw_chevron_90`) — Windows도 동일 경로 |
| 7 | URL 열기 `ShellExecuteW("open")` + `AllowSetForegroundWindow`(`about.rs:168-178`) | `open <url>` | `xdg-open <url>`(실패 시 `gio open`) | 공용 `open_url()` 한 곳. nexa-fs `shell.rs`에 유사 기능이 있는지 확인 필요(추정) |
| 8 | PNG 디코드 = GDI+(`gdipctx.rs:55`) · SVG 래스터 = GDI+ | 자체 디코더 | 동일 | `nexa_gfx::image::decode`(전 OS 공통). SVG는 파서 이식 + nexa-ctl 래스터 |
| 9 | 캡션 없는 팝업 라운드 = DWM `DWMWA_WINDOW_CORNER_PREFERENCE`(Win11만, Win10 = 각짐 `bulkrename.rs:1484-1495`) | 무테 창 + 자체 라운드(투명 창) 또는 표준 타이틀바 창 | 컴포지터 의존 — 표준 장식 창 권장 | 권장: 이름/관리 팝업을 **창 안 오버레이 모달**로 바꾸면 분기 소멸 |
| 10 | 날짜 도움말 = `MessageBoxW`(`bulkrename.rs:1859`) | — | — | 자체 메시지 모달(DLG-057)로 통일 — OS 상자 사용 금지 기조 |
| 11 | 수식키 `GetKeyState(VK_SHIFT/VK_CONTROL)` · `MK_*` | ⌘ = 주 수식키 | Ctrl | `InputEvent{shift, primary}`(`nexa-ui/crates/nexa-ctl/src/event.rs:62-69`) — 그리드 Ctrl+클릭/Ctrl+A/Ctrl+↑↓(순서 이동)는 mac에서 ⌘ |
| 12 | 휠 = `WM_MOUSEWHEEL` 120 단위 + `SPI_GETWHEELSCROLLLINES` | 트랙패드 픽셀 델타(관성) | 줄/픽셀 혼재 | winit `MouseScrollDelta::{LineDelta,PixelDelta}` → `Wheel{delta}` 정규화 + `WheelAccum`(`event.rs:120`). ordertree의 정수 나눗셈 휠(DLG-053)은 누적기로 교체 |
| 13 | 커서 `IDC_HAND`·`IDC_SIZEWE` | — | — | winit `CursorIcon::Pointer`/`EwResize` |
| 14 | 프리셋 폴더 `data_dir()\renames`(exe 옆 `data\` → 불가 시 `%LOCALAPPDATA%\NexaDir\data` `config.rs:357-372`) | `~/Library/Application Support/<앱>/renames`(추정) | `$XDG_CONFIG_HOME/<앱>/renames`(추정) | 설정 구조는 nexa-sql 차용 방침 — 실제 경로 규칙은 설정 인벤토리 문서 소관. 이 문서는 "프리셋 = 설정 폴더 하위 `renames/*.cfg`"만 고정 |
| 15 | 파일명 금지 문자 필터 `<>:"/\|?*`(`bulkrename.rs:1448-1451`) | `/`·NUL만 금지 | 동일 | 프리셋 파일을 OS 간에 옮길 수 있도록 **Windows 집합을 전 OS 공통 유지** 권장 |
| 16 | 생성 시각 `metadata().created()`(`bulkrename.rs:2064-2067`) | 지원(birthtime) | 파일시스템·커널에 따라 미지원 → `Err` | 미지원 = 0 → Date(Created) 동작이 무변경으로 격리되는 기존 규약 유지. UI에 "생성일 미지원" 안내 추가 검토 |
| 17 | DPI `GetDpiForWindow` + 글꼴만 스케일 | Retina 배율 2.0 | 분수 배율 | `ControlBase.scale` + `Control::s()`(`controls/mod.rs:285`)로 **좌표까지** 배율 적용 — §2의 px는 전부 논리 px로 취급 |
| 18 | 암호 칸 = OS EDIT 마스킹 | 자체 TextBox | 동일 | IME가 마스킹 칸에 조합 문자를 넣는 문제 — nexa-sql `imehint`(`nexa-sql/crates/nexa-sql/src/input_win.rs:53`) 선례 참고 |
| 19 | 창 아이콘 `crate::icon::load(32)` · 시스템 색 `COLOR_BTNFACE`/`COLOR_WINDOW` | — | — | winit 아이콘 + `Theme`(`window_bg`/`panel_bg`). dir2 대화상자는 라이트 고정 — dir3는 테마 추종 |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 영속

| 대상 | 형식·위치 | 근거 |
|---|---|---|
| 일괄 이름변경 프리셋 | `data\renames\<이름>.cfg` — 텍스트(라인 순서 = 적용 순서, `nexa_ops::batch_rename::serialize_ops`/`parse_ops` · 관용 파싱·64블록 상한·v1 하위호환). 원자적 쓰기(temp → rename · `sync_all`). 목록 = 이름순·64개 절단 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:233-236` · `:1379-1392` · `nexa-dir2/crates/nexa-app/src/config.rs:986-994` · `nexa-dir2/crates/nexa-ops/src/batch_rename.rs:806` · `:875` |
| 순서/표시 문자열 | 문법 `블록:vis[자식:vis,…]\|블록:vis\|…`(단일 블록 = 대괄호 생략, vis 생략 = 표시). 파싱 시 미지·중복 폐기 + **누락분은 기본 정의 순으로 보충**(전방 호환). 저장 위치 = 설정 값(`toolbar_order`·`col_layout`·`ctx_menu_order`) | `nexa-dir2/crates/nexa-app/src/config.rs:1069-1118` · `prefs.rs:2376-2389` |
| 대화상자 글꼴·닫기 지연 | 설정 `dlg_font`·`dlg_font_size`(7..24) · `transfer_close_ms`(0..10000, 0 = 진행 창 생략) | `nexa-dir2/crates/nexa-app/src/prefs.rs:1005-1008` · `win.rs:2173` |
| 영속하지 않는 것 | 대화상자 위치·크기(매번 소유자 중앙) · 그리드 컬럼 폭/정렬 · 행별 제외 · 트리 접힘 상태(창 수명 동안만) · 마지막 사용 파이프라인(창을 열 때마다 기본 카드 1장) · 암호(세션 메모리만) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:2207-2224` · `ordertree.rs:341` · `pwprompt.rs:10-12` |

### 5-2. 스레딩·메시지 흐름

- **재진입 규약**: 모달은 메뉴 명령 처리 중 바로 열지 않고 `PostMessageW(WM_APP_*)`로 지연해 앱 `State` 차용 밖에서 연다(`WM_APP_BULK` · `WM_APP_CTLDEMO` 0x8009 · `WM_APP_ABOUT` 0x800C · `WM_APP_PREFS` — `nexa-dir2/crates/nexa-app/src/win.rs:5358-5371` · `:9039-9056`). 중첩 루프 동안 메인 wndproc가 재진입되기 때문.
- **모달 수명**: 창 상태는 호출 함수 스택의 `Box`(포인터를 `GWLP_USERDATA`에 저장) → 창 파괴 후 루프 종료 → 결과 회수. ordereditor는 `WM_DESTROY`에서 `PostMessageW(None, 0…)`로 루프를 깨운다(`ordereditor.rs:416-424`).
- **실시간 적용**: ordereditor → 소유자 `SendMessageW(WM_APP_ORDER_EDIT)` 동기(같은 스레드) → 소유자가 문자열 복사 후 즉시 적용·저장. 드래그 한 번이 delta 칸 수만큼 **여러 번 통지**한다(`ordereditor.rs:165-170`).
- **워커 스레드 모달**: 전송 워커가 충돌 시 `show_buttons`를 직접 호출(자체 루프) — UI 스레드는 계속 진행 창을 갱신(`win.rs:4539-4575`).
- **진행 창**: UI 스레드 소유. 워커 → `WM_APP_TRANSFER` 통지 → `Progress::update` → `cancelled()` 폴링 → 공유 `AtomicBool cancel` → 워커가 청크 사이에서 감지. 완료 = `set_done` + 호스트 백스톱 타이머(ms+500)로 구조체 해제(`win.rs:4262-4304`).
- **bulkrename**: 전부 UI 스레드 동기. 폼 편집 통지마다 전체 재계산(수확·검증·미리보기·충돌[`Path::exists` 항목 수만큼]·정렬·그리드 교체) — 디바운스 없음(`bulkrename.rs:420-514`).
- **타이머**: 팝업 바깥 클릭 60ms · 그리드/카드 호스트 페이드 900ms · 트리 자동 스크롤 60ms · 진행 창 카운트다운 1s.

dir3 대응(권장 흐름): 보조 창 = 구조체(`XxxWin`) + `on_window_event() -> XxxAction` 반환(nexa-sql 방식), 타이머 = 호스트 `tick(now_ms)`, 실시간 적용 = 액션 enum 또는 콜백, 워커 모달 = 요청/응답 채널.

---

## 6. 이식 시 주의(실측 교훈·결함 이력 중 회귀 방지에 필요한 것)

| # | 교훈 | 근거 | dir3 적용 |
|---|---|---|---|
| 1 | **팝업이 포커스를 뺏으면 입력란 kill-focus → 확정 전에 팝업이 파괴**된다(목록 클릭 미반영의 진범). 팝업은 비활성화, 목록은 DOWN을 삼키고 UP에서 확정 | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:646-667` · `:692-695` | 오버레이 팝업에서도 "팝업 클릭이 본체 blur로 해석되지 않게" — 포커스 이탈 확정(FontPicker·Spin)은 팝업 내부 클릭을 예외 처리 |
| 2 | 호스트 펌프가 Enter를 가로채면 **열린 드롭다운의 Enter 확정이 죽는다** → `FBM_HAS_DROP`으로 양보 | `nexa-dir2/crates/nexa-app/src/prefs.rs:3053-3063` | 포커스 체인의 Enter=기본 버튼 규칙은 "포커스 컨트롤의 팝업이 열려 있으면 컨트롤 우선"(`Combo::is_open` · `TextBox::popup_open`) |
| 3 | WS_POPUP owner 승격으로 `GetParent`가 빗나가 **타이머 누수(60ms)** | `nexa-dir2/crates/nexa-app/src/ctl/fontbox.rs:294-305` · `combobox.rs:178-180` | Win32 전용 함정 — 오버레이 모델에서는 소멸. 타이머/틱 해제 누락 방지만 계승 |
| 4 | **빈 문자열 그리기 = 크래시**(빈 Vec 포인터 → user32 AV) | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:1120` · `segmented.rs:333-335` · `ordertree.rs:867-869` | nexa-ui `ctx.text`는 안전(추정)하나, 빈 셀/빈 라벨 케이스를 테스트에 포함 |
| 5 | 잦은 갱신 컨트롤의 깜빡임 → 더블버퍼 + 배경 지우기 생략 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:850-864` | softbuffer 전면 재그리기 구조라 자연 해소 — 대신 무효화 영역(Invalidations) 누락에 주의 |
| 6 | 1비트 리전 클립은 계단 가장자리 → 모서리는 배후색 + AA 도형 | `nexa-dir2/crates/nexa-app/src/ctl/iconbutton.rs:145-146` · `style.rs:25-28` | nexa-ui SDF AA로 해소. 카드 타이틀 밴드 위 컨트롤이 **밴드와 같은 색이라 묻히는 문제**(필 한 단계 진하게 + 외곽선)는 테마 색 선택 시 재현 주의(`bulkrename.rs:828-834`) |
| 7 | 텍스트가 위에 붙어 보임 → 세로 중앙 +1px 하향 · 한글 글리프 클리핑 → 입력 높이 글꼴+4 | `nexa-dir2/crates/nexa-app/src/ctl/combobox.rs:356` · `searchbox.rs:92-97` | `text_center_y`(잉크 기준 중앙)로 대체 — 한글·영문 혼합 스냅샷으로 확인 |
| 8 | 진행 바: 작은 파일 구간이 반올림으로 **소멸**(4파일이 3구간) → 최소 3px 보장 · 완료 시 진행값을 1/1로 강제하면 "1 B / 1 B" 오표기 · 0바이트 완료는 100% | `nexa-dir2/crates/nexa-app/src/dialog.rs:376-379` · `:542-548` · `:752-753` | 배분 함수를 순수 함수로 분리해 단위 테스트(§7 T-09) |
| 9 | 진행 창 재도장은 지우고 그리기(이전 텍스트 중첩) · 확인창과 진행 창은 ±110px로 **서로 겹치지 않게** | `nexa-dir2/crates/nexa-app/src/dialog.rs:738-739` · `:213` · `:674` | 진행 창(위)·충돌 모달(아래) 동시 표시 배치 유지 |
| 10 | 닫기 카운트다운은 ms 정밀(첫 틱 = 나머지) — 올림 초 표기 | `nexa-dir2/crates/nexa-app/src/dialog.rs:754-765` | `TimeoutButton` 사용 시 "총 대기 = 정확히 ms" 검증 |
| 11 | 메시지 잘림 → 워드랩 높이 실측 + 비클라이언트 보정 | `nexa-dir2/crates/nexa-app/src/dialog.rs:197-212` | winit `inner_size` 기준이라 보정 불필요 — 긴 메시지(파일명 목록) 케이스 테스트 |
| 12 | 글꼴 체인 문자열을 그대로 얼굴 이름으로 넘기면 기본 글꼴로 대체 → 설치된 첫 패밀리 선택 | `nexa-dir2/crates/nexa-app/src/dialog.rs:62-67` | nexa-font 폴백 체인으로 해소 — FontPicker 쉼표 체인 규칙과 일관 유지 |
| 13 | 버튼 활성은 **내부 상태**로(OS Enable은 그리기 미반영) | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:507-513` | `Control::set_enabled` 단일 경로 |
| 14 | 그리드 `set_rows`는 선택·포커스를 **유지**(미리보기가 키 입력마다 교체) · 체크 클릭은 선택 불변 | `nexa-dir2/crates/nexa-app/src/ctl/grid.rs:231-243` · `:711` | Grid 계약에 명시 + 테스트 |
| 15 | 정렬 중에도 체크 토글이 올바른 항목에 반영되게 표시행→항목 `order` 맵 유지 | `nexa-dir2/crates/nexa-app/src/bulkrename.rs:471-497` · `:1943-1958` | 그대로 이식(N) |
| 16 | 트리 드래그: 미리보기는 뷰만 재배열, 확정 시 **뷰 원복 후 delta만 보고**(모델–뷰 정합) | `nexa-dir2/crates/nexa-app/src/ctl/ordertree.rs:753-762` | OrderTree 계약 유지 |
| 17 | 카드 삽입 후 탭 순서가 화면 순서와 어긋남 · Enter/Esc 미동작 · 메뉴 12행 잘림 · 스핀 음수 타이핑 불가 · ordertree 터치패드 휠 손실 · 프리셋 덮어쓰기 무확인 | §1 DLG-035·029·053·083·086, §2-1 | **dir2의 한계**(기능 계승 대상 아님) — dir3에서 개선할지 결정 필요(결정 전까지 "동작 유지"가 기본) |
| 18 | 암호: 값을 창 상태에 남기지 않고 한 번만 이동, 모든 종료 경로에서 컨트롤 잔상 제거 | `nexa-dir2/crates/nexa-app/src/pwprompt.rs:244-300` | `take_secret_text` + `wipe`, 취소·닫기 경로 테스트 |

---

## 7. 회귀 테스트 후보

자동화 표기: **U** = 단위(순수 함수/컨트롤에 `InputEvent` 주입 + `ProbeCtx`(`nexa-ui/crates/nexa-ctl/src/controls/mod.rs:787`)) · **S** = 스냅샷(래스터 버퍼 비교 — `RasterCtx`, 하네스 구성 필요) · **M** = 수동/실기.

| # | 시나리오 | 대상 ID | 자동화 |
|---|---|---|---|
| T-01 | Checkbox 3단 순환 0→1→2→0, 2단은 0↔1, set은 보고 없음·클램프 | DLG-011·012 | U |
| T-02 | Combo: ↓ 열기 → ↓↓ → Enter = 변경 보고 1회 / 같은 항목 확정 = 보고 없음 / Esc = 선택 불변 | DLG-016 | U |
| T-03 | Segmented: 클릭 칸 계산(마지막 칸 나머지 흡수)·←→ 끝 클램프·같은 칸 재선택 = 보고 없음 | DLG-025·027 | U |
| T-04 | Spin: ↑↓ ±1·한계 방향 무시·포커스 이탈 클램프·빈/비숫자 = 0·set 클램프 | DLG-028·029 | U |
| T-05 | FontPicker `apply_pick` 7케이스(dir2 기존 테스트 이식) + 접두 타입어헤드(마지막 `,` 조각) | DLG-023·024 | U |
| T-06 | Grid 선택: 클릭/Shift/Ctrl/빈 영역 해제/키보드(↑↓·Pg·Home/End·Ctrl 이동·Space·Ctrl+A)·`set_rows` 후 선택 유지 | DLG-041·042·047 | U |
| T-07 | Grid 체크: 행 토글이 선택을 바꾸지 않음 · 헤더 3상태 계산(None/0/1/2) · 전체 토글 규칙 · ⊖ 히트 | DLG-043·044 | U |
| T-08 | Grid 정렬 상태 기계: 단일 3상태 순환 · Shift 추가/방향/제거 · 헤더 라벨(▲/▼ + ①②) | DLG-045 | U |
| T-09 | 세그먼트 진행 바 폭 배분: 합 = 전체 폭 · 최소 3px · 0바이트 항목 · 512개 초과/total 0 폴백 | DLG-060 | U |
| T-10 | 카운트다운: ms=2500 → 표기 3 → 첫 틱 500ms → 2 → 1 → 닫힘(총 2500ms) · 버튼 클릭 즉시 닫힘 | DLG-061 | U |
| T-11 | `fmt_bytes` 경계(1023 B · 1.0 KB · 1.0 TB) | DLG-063 | U |
| T-12 | OrderTree: 형제 범위 선택(혼합 차단)·부모 해제 시 자식 토글 무시·접힘 시 자식 선택 해제·`swap_step` 위/아래·그룹 블록 이동·drag delta·cancel 복원 | DLG-049~052 | U |
| T-13 | ordereditor 모델: `shift_range` 경계 · 그룹/자식 `move_sel` 후 직렬화 문자열 · 잠금 key 토글 거부 · 그룹 체크가 자식 상태 보존 · 직렬화↔파싱 왕복(누락 보충) | DLG-070·071·073 | U |
| T-14 | bulkrename 수확: 종류별 유효/무효 조건(빈 찾기·Entire 예외·빈 텍스트) · 프리셋 → 카드 복원 → 재수확 왕복 · Move/ChangeExt 스킵 | DLG-077·082 | U(폼 값 구조체로 분리 시) |
| T-15 | bulkrename 미리보기: 충돌 행 = 체크 없음+사유 · 무변경 = 빈칸 · 제외 반영 건수 · Rename 활성 조건 4개 · 정렬 중 체크 토글이 올바른 항목에 반영 | DLG-079~081 | U |
| T-16 | 카드 스택: ＋ = 아래 삽입·− = 삭제·1장일 때 − 비활성·종류 변경 시 높이 재계산(185/157/129/101)·스크롤 클램프 | DLG-075·078 | U |
| T-17 | 암호 창: Enter = 확인(Secret 반환·입력란 비워짐) · Esc/닫기 = None(입력란 비워짐) · 빈 입력 확인 = None · 표시 토글이 값 불변 · 마스킹 중 복사 불가 | DLG-064~066 | U |
| T-18 | 메시지 모달: 버튼 폭/우측 정렬/최소 폭 380 · 긴 메시지 워드랩 높이 · 닫힘 = 0 · 워커 요청 → UI 표시 → 응답 전달 왕복 | DLG-057·058 | U + M(스레드) |
| T-19 | 프리셋 이름 정제(금지 문자 제거·trim·빈 값 거부) · 관리 팝업 스테이징(취소 = 파일 보존 · OK = 삭제) | DLG-083·084 | U(임시 폴더) |
| T-20 | 컨트롤 갤러리 스냅샷: 라이트/다크 × 배율 1.0/1.5/2.0 × 3 OS — 버튼 3상태·체크 3상태·콤보(닫힘/열림)·세그·스핀(한계)·카드·그리드(지브라/체크/정렬 배지)·트리(접힘/드래그 고스트) | DLG-008~054 전반·055 | S |
| T-21 | 대화상자 배치 스냅샷: bulkrename(종류 6종 각 카드)·순서 편집기(평면/그룹)·암호(재시도 유/무)·About·진행 창(진행/완료) — i18n ko/en 라벨 폭 차이 포함 | DLG-056~088 | S |
| T-22 | About 링크: 히트 존·커서 변경·URL 열기 명령(OS별 `open`/`xdg-open`/ShellExecute)은 주입 가능한 함수로 대체해 호출 인자 검증 | DLG-068 | U + M |
| T-23 | 휠: 노치/분수(트랙패드) 누적·Shift 가로·고속 가속 — 그리드·카드 호스트·트리 공통 | DLG-040·053·078 | U |
| T-24 | 모달성 실기: 보조 창이 열린 동안 메인 창 입력 차단·닫으면 포커스 복귀·보조 창이 메인 뒤로 숨지 않음(3 OS) | §4-1 | M |
| T-25 | 글꼴 목록: OS별 열거 결과 비어 있지 않음·정렬·중복 없음·`@`류 제외(Windows) | DLG-021 | U(OS별 CI) |

핵심 기능 즉시 점검(스모크) 후보: T-06 · T-07 · T-12 · T-13 · T-14 · T-15 · T-17 · T-09 — 전부 창 없이 도는 단위 테스트로 구성 가능(로직을 컨트롤/모델 구조체에 두는 nexa-ui 방식의 이점).
