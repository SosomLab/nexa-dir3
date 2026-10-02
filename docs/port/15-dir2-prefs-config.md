# 15 · nexa-dir2 인벤토리 — 설정 창(prefs) · 설정/세션 영속(config) · i18n

> 단계: 이해(인벤토리) · 읽기 전용 조사 결과. 작성 2026-10-03.
> 대상 원본: nexa-dir2 0.22.0(Windows 전용). 이 문서의 `PREFS-NNN` ID는 이후 구현·교차 검증의 체크리스트다.
>
> **경로 표기 약속**(모든 근거는 `저장소/경로:줄`):
> - `dir2:X` = `nexa-dir2/crates/nexa-app/src/X` (예: `dir2:prefs.rs:1284`)
> - `dir2-lang:X` = `nexa-dir2/crates/nexa-app/lang/X`
> - `dir2-doc:X` = `nexa-dir2/docs/X`
> - `ui:X` = `nexa-ui/crates/X` (예: `ui:nexa-ctl/src/controls/radio.rs:38`)
> - `sql:X` = `nexa-sql/crates/X`
>
> **ID 대역**: 001~ 기능 · 101~ settings 키 · 201~ session 키 · 301~ 화면 배치 · 401~ nexa-ui 매핑 · 501~ OS 분기 · 601~ 이식 주의 · 701~ 회귀 테스트 후보.

## 0. 범위 — 읽은 파일과 줄 수

| 파일 | 줄 수 | 읽은 범위 |
| --- | --- | --- |
| `dir2:prefs.rs` | 3078 (과제 표기 3003 — 실측 3078) | 전체(1~3078) |
| `dir2:config.rs` | 1689 | 전체(테스트 포함) |
| `dir2:i18n.rs` | 289 | 전체(테스트 포함) |
| `dir2:ordereditor.rs` | 427 | 전체(설정 창의 [편집…] 버튼이 여는 공통 창) |
| `dir2:win.rs` | 9951 | 부분 — 설정/세션 배선만: 316~345 · 1399~1700 · 5060~5082 · 5160~5372 · 5556~5615 · 6300~6420 · 6475~6762 · 6795~7018 · 7708~7718 · 8891 · 8984~9028 · 9060~9066 · 9422~9430 · 9662~9689 |
| `dir2:panel.rs` | (미계수) | 부분 — 세션 스냅샷/복원 함수: 198~289 · 687~717 · 1081~1104 · 1175 · 1252~1264 |
| `dir2-lang:ko.lang` / `en.lang` | 542 / 543 | `pref.*` 키 전부(Grep) + 머리 메타 |
| `dir2-doc:audit/20260904-165351/02-settings.md` | 29 | 전체(설정·세션 감사 결과) |
| `dir2-doc:20-session-coalescing.md` | 122 | 전체 |
| `dir2-doc:wiki/기능-설정.md` | 89 | 전체 |
| 대조용 | | `ui:nexa-conf/src/lib.rs`(1~372) · `sql:nsql-settings/src/lib.rs`(1~300 · 716~800 · 2530~2606 · 5786~5965 · 6380~6815) · `sql:nexa-sql/src/prefs_win.rs`(1~200 + 함수 윤곽) · `ui:nexa-ctl/src/controls/*`(공개 타입 Grep + 모듈 머리말) |

확인하지 못한 것은 본문에 **"추정"** 으로 표시했다. `prefs.rs`에는 단위 테스트가 없다(`#[test]` 0건).

---

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 / **W**=Windows 전용 유지.

### 1-1. 설정 창(prefs.rs)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PREFS-001 | 설정 창 열기 | `Ctrl+,` 직접 호출 · 파일 메뉴 `설정...`(CMD_PREFS=60, 단축키 표기 "Ctrl+,") · 툴바 ⚙(`settings` 블록, MDL2 `E713`) · 툴바 우클릭 팝업 경유. 메뉴/툴바는 `PostMessage(WM_APP_PREFS)`로 **State 차용 밖에서 지연 실행**(재진입 규약) | `dir2:win.rs:8891` · `:5357` · `:8984` · `:6296` · `:446` · `:740` | PostMessageW · WM_APP+5 | A (단축키는 P: macOS `Cmd+,`) | 없음 |
| PREFS-002 | 모달 창 | 소유자 `EnableWindow(false)` → 자체 `GetMessage` 루프 → 닫히면 소유자 복원. 반환 = `Option<PrefValues>`(창 생성 실패 시 None) | `dir2:prefs.rs:2855-3078` | CreateWindowExW · EnableWindow · GetMessageW · SetForegroundWindow | A/P | 없음 |
| PREFS-003 | 창 틀 | 제목 `pref.title` · 스타일 POPUP+CAPTION+SYSMENU+THICKFRAME+MAXIMIZEBOX+CLIPCHILDREN · ex DLGMODALFRAME · **리사이즈 가능, 기본 클라이언트 760×560 = 최소 크기** · 소유자 창 중앙 배치 · 창 아이콘 32px · **라이트 고정**(다크 테마 미추종) | `dir2:prefs.rs:2805-2812` · `:2885-2912` · `:2622-2636` | AdjustWindowRectEx · WM_GETMINMAXINFO | A (dir3는 테마 추종 가능 — 결정 필요) | 없음 |
| PREFS-004 | 즉시 적용(VS Code식 · 저장 버튼 없음) | 체크박스·라디오·드롭다운·위치 = 클릭 즉시 / 숫자·글꼴 = 포커스 이탈(EN_KILLFOCUS 0x0200 · CBN_KILLFOCUS 4) 또는 Enter / 크기 콤보 목록 선택 = 즉시. 매번 `harvest()`→`sanitize`→소유자에 `WM_APP_PREFS_APPLY`(0x8006) **동기** 통지(lparam=`*const PrefValues`, 통지 동안만 유효) | `dir2:prefs.rs:1110-1119` · `:2453-2533` · `:47` | SendMessageW(동기) | A (→ `PrefsAction::Changed{key,value}` 패턴 `sql:nexa-sql/src/prefs_win.rs:31`) | 없음 |
| PREFS-005 | 닫기 = 확정 | `WM_CLOSE`에서 미확정 편집 값 수거(`harvest`) → `show` 반환 후 호스트가 `apply_prefs` 한 번 더(멱등). **Esc 닫기 처리 없음**(코드에 VK_ESCAPE 없음 — 추정: X 버튼·Alt+F4만) | `dir2:prefs.rs:2637-2641` · `:3074-3077` · `dir2:win.rs:6415-6418` | DestroyWindow | A | 없음 |
| PREFS-006 | 값 정규화 | 빈 `term_font`→"Consolas" · 빈 `dlg_font`→"Segoe UI" · term 크기 8~32 · term_cols 80~1000 · autofit 50~2000 · 순서 문자열 3종 재직렬화 정규화 · ta_reset 200~10000 · ta_pos 0~8 · transfer 0~10000 · dnd 200~10000 · dlg 크기 7~24. **base/ctx/status/list 글꼴 빈 값과 크기는 여기서 안 막음**(apply_prefs가 크기만 8~32 클램프) | `dir2:prefs.rs:982-1008` · `dir2:win.rs:6511-6521` | — | N (레지스트리 `normalize`로 대체 `sql:nsql-settings/src/lib.rs:6410`) | 없음 |
| PREFS-007 | 사이드바 계층 트리 | 정적 pre-order 15노드(§2-2). 그룹 클릭 = 펼침 토글 + 그룹 페이지 / leaf 클릭 = 그 카테고리. 기본 = 전부 펼침, 첫 화면 = `general` 그룹. 선택 전 현재 편집 값 `harvest`, 스크롤 0으로 | `dir2:prefs.rs:280-296` · `:2394-2414` · `:2925-2928` | 오너드로 LISTBOX · LBN_SELCHANGE | A | 없음 |
| PREFS-008 | 그룹 페이지(드릴다운) | 그룹 = **하위 메뉴 링크 목록**(본문 폰트, 클릭 = 그 메뉴로 이동 + 조상 펼침) + 그룹 직속 항목(현재 직속 항목 가진 그룹 없음). 세부는 하위 선택 시 | `dir2:prefs.rs:1333-1361` · `:2415-2436` | STATIC SS_NOTIFY | A | 없음 |
| PREFS-009 | 설정 검색 | 실시간(EN_CHANGE). 소문자·공백 분리 **토큰 AND**, 대상 = **항목 라벨·카테고리 라벨**(설명문은 대상 아님 — 코드 기준; wiki는 "라벨/설명"이라 적었으나 `label_hits`는 라벨만). 입력 = 선택 해제(전역 결과 페이지), 비우면 `general` 복귀 | `dir2:prefs.rs:323-347` · `:2353-2367` | EN_CHANGE | A | 없음 |
| PREFS-010 | 검색 중 트리 필터 | 노드 라벨 매치 **또는** 하위 항목 라벨 매치 노드만 표시 · 조상 경로 유지 · 그룹 라벨 자체 매치 = 하위 전체 표시 · 노드 뒤 매치 수 `" (N)"`(그룹 = 하위 합, 0이면 생략) · 검색 중 그룹은 강제 펼침 표시 · 검색 중 그룹 클릭은 펼침 토글 안 함 | `dir2:prefs.rs:352-382` · `:1946-1998` · `:2119` · `:2408` | — | A | 없음 |
| PREFS-011 | 전역 검색 결과 페이지 | 제목 `“검색어” — {N}개 일치`(`pref.search.results`) · 항목 라벨에 `"카테고리: 항목"` 접두 · 전 카테고리에서 라벨 매치 항목 나열 | `dir2:prefs.rs:1305-1317` · `:1373-1376` · `:1393-1398` | — | A | 없음 |
| PREFS-012 | 검색어 유지 탐색 | 검색 중 트리/링크로 메뉴 이동해도 검색어 유지(명시적 삭제만 지움). leaf에서는 라벨 매치 항목만, **매치 0건이면 그 카테고리 전체** 표시(메뉴명 매치로 진입한 경우) | `dir2:prefs.rs:1377-1389` · `:2406-2411` | — | A | 없음 |
| PREFS-013 | 수정됨 표시 | 기본값(`Settings::default()` 단일 원천)과 다른 항목 좌측에 3px accent 세로 바. `col_layout`은 `default_order(COLUMN_BLOCKS)`와 비교 | `dir2:prefs.rs:1768-1783` · `:1881-1942` | STATIC + WM_CTLCOLORSTATIC | A | 없음 |
| PREFS-014 | 설명 문장 | 항목 아래 회색 한 줄(`라벨키.desc`). 번역 키가 없으면(`tr`이 키를 그대로 반환) 생략 | `dir2:prefs.rs:1750-1767` | — | A | 없음 |
| PREFS-015 | 본문 스크롤 | 휠 1노치 = 60px, **잔량 누적**(트랙패드 미세 델타 보존), 고속 스크롤 가속기 적용(배지는 생략). 사이드바에 포커스가 있어도 본문이 스크롤(메인 wndproc도 휠 처리) | `dir2:prefs.rs:1272-1281` · `:2570-2573` · `:2693-2696` | WM_MOUSEWHEEL | A | 없음 |
| PREFS-016 | 오버레이 스크롤바 | 평소 숨김 → 스크롤 순간 반투명(α120) → 900ms 유지 → 40ms 틱마다 α−24 페이드. 호버/드래그 = 폭 10px·α210·페이드 보류. 썸 드래그(트랙 α28 표시)·트랙 클릭 = 페이지 이동(뷰−30px). 콘텐츠가 뷰에 들어가면 없음 | `dir2:prefs.rs:1144-1224` · `:2697-2794` · `:884-907` | GDI+ 알파 · SetTimer · SetCapture · TrackMouseEvent | A (`ScrollBars`) | 없음 |
| PREFS-017 | 리사이즈 추종 | WM_SIZE → 구분선 높이·트리 높이·본문 컨테이너 재배치 → `harvest` → 본문 재구성(폭 추종) | `dir2:prefs.rs:2594-2621` | MoveWindow | A | 없음 |
| PREFS-018 | Enter 즉시 적용 | 모달 펌프가 WM_KEYDOWN VK_RETURN을 가로채 포커스가 편집 컨트롤(또는 콤보 내부 EDIT)이면 `harvest+apply` 후 소비(비프 억제). **글꼴 드롭다운이 열려 있으면 가로채지 않음**(Enter = 목록 확정) | `dir2:prefs.rs:3032-3067` | GetFocus · GetParent · FBM_HAS_DROP | A | 없음 |
| PREFS-019 | 종속 항목 비활성 | `fast_scroll` 끔 → step·max·window·hud·grid_extra 비활성 / `fast_scroll && fast_scroll_hud` 아님 → hud_pos·hold·fade 비활성. 비활성 항목은 회색 글자·위치 타일 회색. 체크 클릭 즉시 연동 | `dir2:prefs.rs:1090-1108` · `:2530-2532` · `:2586-2590` · `:2238` | EnableWindow · IsWindowEnabled | A (`DEPENDS` 표 `sql:nsql-settings/src/lib.rs:6021`) | 없음 |
| PREFS-020 | 체크박스 항목 | 라벨 일체형. 16개 필드(§5-1) | `dir2:prefs.rs:1464-1501` | BUTTON BS_AUTOCHECKBOX | A | 없음 |
| PREFS-021 | 라디오 그룹 항목 | 캡션 + 세로 라디오. 정적 5종(테마·보기 범위·Alt+↑ 배치·탭 더블클릭·타입어헤드 범위) + 언어(동적: `system` + 발견 언어 — **라벨은 언어 코드 그대로**(`en`,`ko`,`ja`) · system만 번역) | `dir2:prefs.rs:1502-1567` · `:934-940` · `:404-443` | BS_AUTORADIOBUTTON · WS_GROUP | A | 없음 |
| PREFS-022 | 드롭다운 목록 항목 | 터미널 테마(시스템/다크 기본/라이트 기본 + 스킴 15종, 라벨 `"이름 (다크/라이트)"`(스킴 이름 뒤 괄호에 다크 또는 라이트)) · 다크 기본 스킴(다크 9종) · 라이트 기본 스킴(라이트 6종) · 복사 서식 4택. 선택 = 즉시. 현재 값이 목록에 없으면 선택 없음 표시 | `dir2:prefs.rs:156-203` · `:1568-1630` · `:2473-2501` | COMBOBOX CBS_DROPDOWNLIST | A (`Combo`) | 없음 |
| PREFS-023 | 숫자 입력 항목 | `[입력 200px][라벨]` 한 줄. 숫자만(ES_NUMBER). 파싱 실패 = 필드별 기본값으로 대체(§5-1) 후 sanitize/apply에서 클램프. 범위 밖 입력에 대한 오류 표시는 없음(조용히 클램프) | `dir2:prefs.rs:1697-1748` · `:2001-2035` | EDIT ES_NUMBER | A | 없음 |
| PREFS-024 | 글꼴 행 항목 | 캡션 + `[패밀리 입력 200px][크기 콤보 64px]`. 패밀리 = `ctl::fontbox`(설치 글꼴 드롭다운·각 항목 자기 글꼴 렌더·접두 매칭 이동·**쉼표 = 폴백 체인** 선택 규칙). 크기 = 입력 가능한 콤보, 프리셋 8·9·10·11·12·14·16·18·20·24·28·32. 6슬롯(기본·터미널·우클릭 메뉴·상태바·파일 목록·대화상자) | `dir2:prefs.rs:1631-1696` · `dir2:ctl/fontbox.rs:1-41` | 커스텀 fontbox · COMBOBOX CBS_DROPDOWN | A (**fontbox 대응 없음 — 추가 필요**) | 없음 |
| PREFS-025 | 위치(3×3) 이미지 드롭다운 | 머리 = 선택 위치 미니 화면 타일 + ▾(56×26), 클릭 = 3×3 이미지 셀 팝업(셀 40×32, 현재 값 = accent, 호버 = 옅은 accent 배경). 값 0..8 행우선. 타입어헤드 배지 위치·고속 스크롤 배지 위치 2곳 | `dir2:prefs.rs:1432-1463` · `:2164-2322` · `:2437-2452` | 오너드로 BUTTON · TrackPopupMenuEx(오너드로 메뉴) | A (`PositionDropdown` 있음) | 없음 |
| PREFS-026 | [편집…] 별도 창 항목 | 캡션 + 우측 `편집...` 버튼(76×24). 3종: 도구 모음 순서 · 파일 컬럼 · 컨텍스트 메뉴 항목. 클릭 = `ordereditor::show` 모달, 변경은 `WM_APP_ORDER_EDIT`(0x8007)로 실시간 수신 → 즉시 적용 | `dir2:prefs.rs:1402-1431` · `:2368-2393` · `:2335-2348` | BUTTON · SendMessageW | A | 없음 |
| PREFS-027 | 순서/표시 편집 창(공통) | `EditorSpec`(제목·정의·표시 열·평면·잠금 key·라벨 함수) 어댑터 1구현으로 3편집기. 그룹(레벨0) = 블록 통째 이동, 자식(레벨1) = 그룹 안에서만. Shift = 같은 레벨·같은 부모 연속 다중 선택. 체크 = 표시 토글(그룹 체크 해제 = 통째 숨김·자식 상태 보존). 잠금 key(`name` 컬럼)는 해제 불가. 드래그 이동(고스트·자동 스크롤). ▲▼ 버튼(선택 없으면 비활성). 12행 초과 = 내부 스크롤 | `dir2:ordereditor.rs:36-47` · `:114-236` · `:242-348` | 커스텀 `ctl::ordertree` · `ctl::iconbutton` | A (**ordertree 대응 없음 — 추가 필요**) | 없음 |
| PREFS-028 | 편집 창 키보드 | ↑/↓ 선택 이동 · Shift+↑/↓ 형제 범위 확장 · Ctrl+↑/↓ 순서 이동 · Space 체크 토글 · Esc = 드래그 취소(있으면) 아니면 창 닫기 | `dir2:ordereditor.rs:368-411` | WM_KEYDOWN · GetKeyState | A | 없음 |
| PREFS-029 | 편집 창 재사용(설정 창 밖) | 툴바 우클릭 → 도구 모음 편집, 컬럼 헤더 우클릭 → 컬럼 편집. 같은 spec 공유(`toolbar_editor_spec`/`col_editor_spec`). 결과는 메인 창이 직접 수신·영속 | `dir2:prefs.rs:1011-1032` · `dir2:win.rs:6306-6323` · `:8988-9028` | — | A | 없음 |
| PREFS-030 | 플러그인 페이지 | 레지스트리 밖 **동적 목록**: 설명 1줄(`pref.plugins.desc`) + 로드된 플러그인마다 체크박스 `"{name} ({id}) — {ext, ext}"`. 체크 해제 = `plugins_disabled`에 id 추가(파이프 구분). 목록 없으면 `pref.plugins.empty` 안내. **검색 중에는 표시 안 함**. 내장(builtin.*)은 대상 아님 | `dir2:prefs.rs:1786-1845` · `:2078-2088` · `dir2:preview/mod.rs:251-261` | BUTTON | A/P(안내 문구 경로가 OS별) | 없음 |
| PREFS-031 | 호스트 적용 파이프라인 | `apply_prefs`: 항목별 **동등 비교로 게이트** 후 적용(테마·언어 = 기존 명령 경로 재사용 / 글꼴 = 렌더 백엔드 재생성 / 보기 필터 = 양 패널 전 탭 기입 + 재열람 …) → 끝에서 **무조건** settings.cfg 저장 + 전체 무효화 + 제목 갱신 | `dir2:win.rs:6475-6762` · `:9060-9066` | InvalidateRect | N(로직)/A(무효화) | 없음 |
| PREFS-032 | 값 스냅샷 왕복 | 열 때 `State → PrefValues`(58필드), 닫을 때 `PrefValues → State`. `langs`(선택지)·`col_layout`(포커스 패널의 세션 상태)은 설정 파일 키가 아님 | `dir2:win.rs:6349-6419` · `dir2:prefs.rs:51-126` | — | N (레지스트리 스냅샷으로 대체) | 없음 |

### 1-2. 설정/세션 영속(config.rs + 호스트 배선)

| ID | 기능(사용자 관점) | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PREFS-040 | 데이터 폴더 결정(포터블 우선) | exe 옆 `data\`에 생성+쓰기 프로브(`.w{pid}`) 성공 → 그대로. 실패 → `%LOCALAPPDATA%\NexaDir\data`. LOCALAPPDATA도 없으면 후보 유지. 프로세스당 1회 판정(OnceLock) | `dir2:config.rs:362-417` | `LOCALAPPDATA` 환경변수 · current_exe | **P** | `choose_data_dir_portable_first_installed_fallback` `dir2:config.rs:1626` |
| PREFS-041 | 구 폴더 마이그레이션 | 신 경로 없음 + 구 `%LOCALAPPDATA%\NexaDir2\data` 있음 → rename(실패 시 구 경로 그대로 사용) | `dir2:config.rs:386-399` | 〃 | P(Windows 한정 이력) | 위 테스트 일부 |
| PREFS-042 | 설정 파일 형식 | `data\settings.cfg` · UTF-8 · 한 줄 `key=value`(첫 `=` 분할, 줄 trim) · `#` 주석 · 머리 `# nexa-dir settings v1` · 외부 crate 0. **BOM 미제거**(BOM이 붙으면 첫 줄 키가 깨져 무시됨) · `=` 주변 공백 불허(키에 공백이 남아 무시) | `dir2:config.rs:421-429` · `:432-563` · `:1016` | — | N (→ nexa-conf 형식 `ui:nexa-conf/src/lib.rs:53`) | `settings_roundtrip_and_lenient_parse` `dir2:config.rs:1283` |
| PREFS-043 | 관용 파싱 | 기본값 위에 덮어씀 · 미지 키/형식 위반 값 무시 · 수치 클램프 · 같은 키 중복 = **마지막 유효 값** · 불리언 = `v != "0"`(빈 값·임의 문자열도 참) · 손상 파일도 패닉 없이 기동 | `dir2:config.rs:566-833` | — | N | `parse_garbage_is_harmless_and_clamped` `dir2:config.rs:1224` |
| PREFS-044 | 원자적 저장 | `{name}.{pid}.tmp`에 쓰기 → `sync_all` → `rename`(Windows = REPLACE_EXISTING). 옛 파일 선삭제 금지 · 실패 시 임시만 정리·옛 파일 보존 | `dir2:config.rs:994-1011` | std rename 의미 | N (→ `write_atomic` `ui:nexa-conf/src/lib.rs:125` — Unix 0600·부모 fsync 포함) | `save_is_atomic_and_load_roundtrips` `dir2:config.rs:1648`(잠금 케이스는 `cfg(windows)`) |
| PREFS-045 | 구 파일명 마이그레이션 | `settings.cfg` 없으면 `settings.txt`, `session.cfg` 없으면 `session.txt` 로드. 종료 저장 성공 후 구 `.txt` 삭제 | `dir2:config.rs:1016-1032` · `dir2:win.rs:1414-1419` · `:9683` | — | N (dir3는 dir2 `data\*.cfg` **가져오기**로 치환 — 결정 필요) | 없음 |
| PREFS-046 | 구 키 마이그레이션 | `transfer_close_secs=N` → `transfer_close_ms = N×1000`(0~10000 클램프) | `dir2:config.rs:667-672` | — | N (→ `RESCALED` 표 `sql:nsql-settings/src/lib.rs:90`) | `dir2:config.rs:1422-1426` |
| PREFS-047 | 순서 문자열 문법 | `블록:vis[자식:vis,…]` 을 파이프(`\|`)로 연결(단일 블록 = 대괄호 생략). 파싱: 미지 블록/자식·중복 제거, **누락 자식은 정의상 앞 형제 뒤에 삽입**, 누락 블록은 끝에 추가, vis 생략 = 표시. 3정의: `TOOLBAR_BLOCKS` · `COLUMN_BLOCKS` · `CTXMENU_BLOCKS` | `dir2:config.rs:1036-1185` | — | N | `toolbar_order_tests::roundtrip_and_merge` `dir2:config.rs:1192` |
| PREFS-048 | 퀵 런처 항목 영속 | `launcher_count=N` + `launcherN=라벨\|exe\|인자`(인자 안의 파이프 보존 — 3분할) · 구분선 `launcherN=-` · 상한 32 · **키 부재(None) = 첫 실행(시드 주입) / count만 있고 0개 = 사용자가 비움(재주입 금지)** · `launcher_seed` < `SEED_VERSION(2)`이면 누락 시드 1회 추가 | `dir2:config.rs:549-561` · `:758-790` · `dir2:win.rs:1550-1556` · `dir2:launcher.rs:35` | 시드 내용이 Windows(pwsh/cmd/VS Code) | N(형식)/P(시드) | `dir2:config.rs:1540-1549` |
| PREFS-049 | 클라우드 연결 영속 | `cloudN=kind\|라벨\|경로\|account`(라벨의 파이프 → `/` 치환, 상한 32, 경로·account 둘 다 없으면 무시) · `cloud_client_id_<kind>` · `cloud_client_secret_<kind>`(≤256, 빈 값 미기록). 설정 창 UI 없음(Cloud 메뉴·파일 직접 편집) | `dir2:config.rs:527-548` · `:791-828` · `dir2-doc:wiki/기능-설정.md:62-88` | — | N | `dir2:config.rs:1438-1470` |
| PREFS-050 | 세션 파일 | `data\session.cfg` · 머리 `# nexa-dir session v1` · 활성 패널 + 패널 2개(탭 경로·활성 탭·탭별 펼침/잠금/고정/보기 모드/보기 플래그·패널 컬럼 레이아웃/폭) — §5-2 | `dir2:config.rs:836-982` · `:1017` | 경로 구분자 파이프(`\|`)가 Windows 불가 문자라는 전제 | **P** | `session_roundtrip_with_pipe_separator` `dir2:config.rs:1560` |
| PREFS-051 | 세션 디바운스 자동 저장 | 탭/경로/보기 변경 시 패널이 `session_dirty` 플래그만 세움 → `update_status`가 양 패널 플래그를 **비단락 OR(`\|`)** 로 수거 → 타이머 재무장(1000ms) → 만료 시 1회 스냅샷·저장(KillTimer 선행). 실패 무시 | `dir2:win.rs:85-91` · `:5072-5079` · `:9422-9430` · `dir2:panel.rs:1175` · `dir2-doc:20-session-coalescing.md` | SetTimer/KillTimer | A/N (→ `SaveScheduler` quiet+max_delay `ui:nexa-conf/src/lib.rs:174`) | `panel::tests::session_dirty_flag_on_tab_ops`(문서 `dir2-doc:20-session-coalescing.md:118` 근거 — 본 조사에서 코드 미확인) |
| PREFS-052 | 종료 저장 | `WM_DESTROY`에서 settings + session 저장 → 성공 시 구 `.txt` 삭제, 실패는 `eprintln!`(GUI라 안 보임). split·dock 비율·펼침·컬럼 폭은 사실상 여기(또는 다른 저장에 편승)에서만 저장. `WM_ENDSESSION` 핸들러 없음 | `dir2:win.rs:9674-9689` · `dir2-doc:audit/20260904-165351/02-settings.md:12` | WM_DESTROY | **P** (winit 종료 경로·macOS Cmd+Q·SIGTERM) | 없음 |
| PREFS-053 | 설정 즉시 영속 | 설정 창 변경(매 통지) + 메뉴/툴바 토글(보기 범위 토글·항상 위·보기 모드·컬럼 동기·패널/정보 모드·도크·런처·테마·언어·클라우드 변경·툴바 편집) 직후 `current_settings`→저장. 디바운스 없음·동기 | `dir2:win.rs:6942-7018` · `:5163` · `:5173` · `:5190` · `:5208` · `:5254` · `:5298` · `:5329` · `:5338` · `:5416` · `:5421` · `:5426` · `:5810` · `:6759` · `:9022` | — | N | 없음 |
| PREFS-054 | 기동 로드·복원 순서 | data_dir → panic 후크 → settings/session 로드 → 고속 스크롤 전역 주입 → 휠 줄 수 → i18n 활성 → 클라우드 루트 동기(**패널 복원보다 먼저**) → 패널 복원(argv 경로가 있으면 세션 무시) → 잠금/고정/도크 비율·표시/정렬/폰트 장식/보기 모드/보기 플래그/타입어헤드 시드 → 런처 시드 → State 구성 → (창 생성 후 DPI 반영 뒤) 컬럼 레이아웃 → 컬럼 폭 순으로 적용 | `dir2:win.rs:1399-1700` · `:7708-7718` | — | N(순서 계승) | 없음 |
| PREFS-055 | 세션 복원 규칙 | 탭 경로를 순서대로 열고 **열기 실패 탭은 건너뜀**, 전부 실패면 fallback 루트(그것도 실패면 가용 루트). 활성 탭 = `min(저장값, 탭 수−1)`. 펼침 경로는 소실 폴더 무시. 보기 모드 = 세션 값 우선·없으면 설정 `view_mode`. 보기 플래그 부족분 = 설정 기본 | `dir2:panel.rs:198-266` · `dir2:win.rs:1447-1531` | 경로 형식 | P(경로) | 없음 |
| PREFS-056 | 크래시 로그 | panic 후크가 `data\crash.txt`에 위치·메시지 기록(릴리스 panic=abort 전) — `config::save` 재사용 | `dir2:win.rs:1404-1413` | — | N | 없음 |
| PREFS-057 | 창 위치/크기 미영속 | 메인 창은 항상 `CW_USEDEFAULT` 위치·1400×800으로 시작. 영속되는 배치 값은 `split`·`dock_ratio`·`dock_split`뿐 | `dir2:win.rs:1750-1753` · `dir2-doc:audit/20260904-165351/02-settings.md:26` | — | N (dir3는 nexa-sql `wingeom` 방식 채택 검토 `sql:nexa-sql/src/wingeom.rs:1`) | 없음 |
| PREFS-058 | 데이터 폴더 구성 | `settings.cfg` · `session.cfg` · `crash.txt` · `lang\*.lang` · `plugins\*.wasm`(+exe 옆 `plugins\`) · `secrets\`(DPAPI 토큰) · `renames\*.cfg`(일괄 이름변경 프리셋) | `dir2:i18n.rs:99` · `dir2:preview/mod.rs:272-275` · `dir2:secret.rs:15` · `dir2:bulkrename.rs:235` | secrets = DPAPI | P | 없음 |

### 1-3. i18n(i18n.rs)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PREFS-070 | `.lang` 파일 형식 | properties 스타일: BOM 스킵 · `#` 주석 · 빈 줄 무시 · 첫 `=` 분리 · 키/값 trim · 중복 = 마지막 승리 · `=` 없는 줄 스킵 · `@key = value` 메타(후행 `# 주석` 제거) · 값 이스케이프 `\n` `\t` `\\`(그 외는 리터럴 유지) | `dir2:i18n.rs:36-79` | — | N | `parse_rules_meta_comment_escape_dup` `dir2:i18n.rs:204` |
| PREFS-071 | 내장 3언어 임베드 | `en`·`ko`·`ja`를 `include_str!`로 내장(약 530키). 메타: `@code` `@name` `@name.en` `@author` `@app` `@fallback` | `dir2:i18n.rs:14-26` · `dir2-lang:en.lang:1-8` | — | N (자원 그대로 사용) | `builtin_langs_parse_and_key_parity` `dir2:i18n.rs:217` |
| PREFS-072 | 사용자 오버라이드 | `data\lang\{code}.lang`을 내장 위에 **키 단위** 덮어쓰기. 내장에 없는 코드는 신규 언어 | `dir2:i18n.rs:97-106` | 경로 | N/P(폴더 위치) | `merge_override_fallback_and_resolve` `dir2:i18n.rs:235` |
| PREFS-073 | 폴백 체인 | 현재 언어 → en(en 자신은 폴백 없음) → 키 문자열 그대로 | `dir2:i18n.rs:82-118` · `:181-188` | — | N | 위 + `trf_placeholders` `dir2:i18n.rs:284` |
| PREFS-074 | 언어 발견 | 내장 3개(en, ko, ja 순) + `data\lang\*.lang` 중 내장 아닌 코드(코드순), 표기 = `@name` 없으면 코드 | `dir2:i18n.rs:122-151` | — | N | 〃 |
| PREFS-075 | 설정값 → 언어 코드 | `system` = OS 로캘의 1차 서브태그(`ko-KR`→`ko`, `-`/`_` 분리), 미보유면 `en`. 명시 코드도 미보유면 `en` | `dir2:i18n.rs:154-165` | — | N | 〃 |
| PREFS-076 | OS UI 언어 조회 | `GetUserDefaultLocaleName`(BCP-47), 실패 시 "en" | `dir2:win.rs:321-330` | Win32 Globalization | **P** | 없음 |
| PREFS-077 | 활성 테이블·조회 | UI 스레드 `thread_local`(기본 = 내장 en). `tr(key)` · `trf(key, args)`(`{0}` `{1}` 치환). **워커 스레드에서 `tr`을 부르면 항상 내장 en**(thread_local 초기값) | `dir2:i18n.rs:167-197` | — | N (스레드 주의 — PREFS-612) | `trf_placeholders` |
| PREFS-078 | 동적 언어 전환 | 재시작 없음: 발견 재실행 → 테이블 스왑 → 메뉴·툴바(툴팁)·컬럼 제목 재구성 → 전체 재그리기. 한계: 컬럼 폭이 기본값으로 재설정 | `dir2:win.rs:5570-5615` | — | N | 없음 |
| PREFS-079 | 번역 파리티 보증 | en을 기준으로 ko·ja 키 집합이 완전히 같아야 함(양방향) | `dir2:i18n.rs:217-232` | — | N (dir3에 그대로 이식) | 테스트 자체 |

---

## 2. 화면·컨트롤 배치

모든 상수는 **px 그대로 CreateWindow에 전달**된다(DPI 배율 미적용 — 글꼴만 DPI 반영, `dir2:prefs.rs:2885-2891` · `:2828`). dir3는 논리 px × 배율로 옮긴다.

### 2-1. 설정 창 골격 (PREFS-301)

상수(`dir2:prefs.rs:908-917`): `PAD=16` · `ROW_H=30` · `CAT_W=180` · `CAT_H=32` · `SEARCH_H=26` · `EDIT_W=200` · `CLIENT_W=760` · `CLIENT_H=560`.

```
┌──────────────── 설정 (760×560 최소·리사이즈 가능) ────────────────┐
│16                                                                 │
│ ┌검색박스 172×26┐ │ ┌─ 본문 컨테이너(x=212 … 우측 끝, y=0 … 하단) ─┐│
│ └───────────────┘ │ │ 16  제목(큰 글꼴, h=28)  y=16               ││
│   (간격 10)       │ │     항목들 … y=56부터                       ││
│ ┌트리 172×가변──┐ │ │                               오버레이 바 ▐ ││
│ │ ▾ 일반         │ │ │                                             ││
│ │    모양        │ │ │                                             ││
│ └───────────────┘ │ └─────────────────────────────────────────────┘│
│16                 ↑ 세로 구분선 x=210, 폭 2, y=16 … 하단−16        │
└───────────────────────────────────────────────────────────────────┘
```

| ID | 영역 | 컨트롤 | 위치·크기 | 근거 |
| --- | --- | --- | --- | --- |
| PREFS-302 | 검색 | `ctl::searchbox`(내장 ✕ 지우개·세로 중앙 정렬), 플레이스홀더 `pref.search.placeholder`("설정 검색"), id 1002 | x=16, y=16, w=`CAT_W−8`=172, h=26 | `dir2:prefs.rs:2954-2968` |
| PREFS-303 | 트리 | 오너드로 LISTBOX(세로 스크롤·정수 높이 아님), id 1100, 행 높이 `CAT_H−4`=28 | x=16, y=`PAD+SEARCH_H+10`=52, w=172, h=`클라이언트 높이−52−16`(최소 40) | `dir2:prefs.rs:2971-2986` · `:2612-2613` · `:1965-1970` |
| PREFS-304 | 구분선 | STATIC 식각 세로선 | x=`PAD+CAT_W+PAD−2`=210, y=16, w=2, h=`높이−32` | `dir2:prefs.rs:2988-2999` · `:2603-2610` |
| PREFS-305 | 본문 컨테이너 | 자식 창(스크롤 컨테이너·더블버퍼 배경) | x=`PAD+CAT_W+PAD`=212, y=0, w=`폭−212`, h=전체 | `dir2:prefs.rs:3002-3019` · `:2615-2616` |
| PREFS-306 | 본문 좌표계 | 항목 시작 x0=16 · 항목 폭 `pane_w = 컨테이너 폭 − 16 − 16`(최소 120) · 우측 16px 띠 = 스크롤바 자리 | — | `dir2:prefs.rs:1296-1297` · `:1164-1172` |

### 2-2. 사이드바 트리 구성 (PREFS-307)

`TREE`(`dir2:prefs.rs:280-296`) — (key, 라벨 키, 깊이). 한국어 라벨은 `dir2-lang:ko.lang` 기준.

| # | key | 라벨 키 | ko 라벨 | 깊이 | 종류 |
| --- | --- | --- | --- | --- | --- |
| 0 | `general` | `pref.grp.general` | 일반 | 0 | 그룹 |
| 1 | `appearance` | `pref.cat.appearance` | 모양 | 1 | leaf |
| 2 | `fonts` | `pref.cat.fonts` | 글꼴 | 1 | leaf |
| 3 | `lang` | `pref.cat.lang` | 언어 | 1 | leaf |
| 4 | `filelist` | `pref.cat.list` | 파일 목록 | 0 | 그룹 |
| 5 | `list` | `pref.cat.listGeneral` | 보기·정렬 | 1 | leaf |
| 6 | `typeahead` | `pref.cat.typeahead` | 타입어헤드 | 1 | leaf |
| 7 | `scroll` | `pref.cat.scroll` | 고속 스크롤 | 1 | leaf |
| 8 | `ctxmenu` | `pref.cat.ctxmenu` | 컨텍스트 메뉴 | 1 | leaf |
| 9 | `transfer` | `pref.cat.transfer` | 파일 전송 | 1 | leaf |
| 10 | `tabs` | `pref.cat.tabs` | 탭 | 0 | leaf(최상위) |
| 11 | `panel` | `pref.grp.panel` | 하단 도크 | 0 | 그룹 |
| 12 | `dock` | `pref.cat.dock` | 하단 도크 | 1 | leaf |
| 13 | `terminal` | `pref.cat.terminal` | 터미널 | 1 | leaf |
| 14 | `plugins` | `pref.cat.plugins` | 플러그인 | 0 | leaf(최상위·동적) |

트리 행 그리기(PREFS-308, `dir2:prefs.rs:2094-2161`): 들여쓰기 `base_left = 좌 + 10 + 깊이×14` · **마커 존 14px 상시 예약**(하위 유무와 무관하게 같은 레벨 라벨 x 일치) · 그룹만 셰브론(펼침 `E70D`, 접힘 `E76C` — Segoe MDL2 Assets 높이 9px) · 라벨 우측 여백 4 · 선택 = 연회색 배경 RGB(E4,E7,EC) · 선택 판정은 리스트 선택이 아니라 `category == key` · `&`를 접두 문자로 해석하지 않음("View & Sort").

### 2-3. 본문 항목 배치 규칙 (PREFS-309)

제목: 컨테이너 (x0, 16), 폭 `pane_w`, 높이 28, 글꼴 = 대화상자 글꼴 **+5pt(9~30 클램프)·세미볼드**(`dir2:prefs.rs:1318-1331` · `:2827-2848`). 항목은 y=`PAD+40`=56부터 아래 규칙으로 쌓는다(스크롤 오프셋만큼 위로).

| 종류 | 구성(좌표는 x0·현재 y 기준) | 다음 y | 근거 |
| --- | --- | --- | --- |
| 그룹 링크 | 본문 글꼴 텍스트 (x0, y, pane_w×20), 클릭 가능 | +28 (링크 묶음 끝에 +6) | `dir2:prefs.rs:1337-1360` |
| [편집…] | 캡션 (x0, y+3, `pane_w−90`×20) + 버튼 `pref.editBtn` (x0+`pane_w−84`, y, 76×24) | +28 | `:1402-1431` |
| 위치 드롭다운 | 캡션 (x0, y, pane_w×20) → y+24 → 머리 (x0, y, 56×26) | +24, +`26+8` | `:1432-1463` |
| 체크박스 | (x0, y, pane_w×24) 라벨 일체 | +30 | `:1464-1501` |
| 라디오 그룹 | 캡션 (x0, y, pane_w×20) → y+26 → 옵션마다 (x0+8, y, `pane_w−8`×24) | 옵션당 +28, 그룹 끝 +8 | `:1502-1567` |
| 드롭다운 | 캡션 20 → y+24 → 콤보 (x0, y, `EDIT_W+72`=272 × 닫힘 24), 목록 최대 12행 | +24, +34 | `:1568-1630` |
| 글꼴 행 | 캡션 20 → y+24 → 패밀리 (x0, y, 200×24) + 크기 콤보 (x0+208, y, 64, 드롭 높이 240) | +24, +34 | `:1631-1696` |
| 숫자 입력 | 입력 (x0, y, 200×24, 테두리) + 라벨 (x0+208, y+3, `max(pane_w−208, 40)`×20) | +34 | `:1697-1748` |
| 설명 | 회색 (x0+2, y, `pane_w−2`×18), 글자색 RGB(68,6E,78) | +26 | `:1750-1767` · `:2589` |
| 수정됨 바 | (x0−8, 항목 시작 y+2, 폭 3, 높이 `max(항목 높이−8, 18)`), 색 RGB(00,78,D4) | — | `:1768-1783` · `:2932` |
| 플러그인 설명/빈 안내 | (x0, y, pane_w×36) 회색 | +40 | `:1789-1820` |
| 플러그인 체크 | (x0, y, pane_w×24) | +30 | `:1821-1844` |

콘텐츠 전체 높이 = 마지막 y + 스크롤 오프셋 + 16(`dir2:prefs.rs:1846`).

### 2-4. 카테고리별 항목 순서 (PREFS-310)

레지스트리 등록 순서가 곧 화면 순서(`dir2:prefs.rs:445-779`). 전부 [라벨 키 → 종류 → 필드].

- **일반 › 모양**(`appearance`): ① `pref.toolbarOrder` [편집…] ② `pref.theme` 라디오(시스템/라이트/다크)
- **일반 › 글꼴**(`fonts`): ① `pref.baseFont` 글꼴 행 ② `pref.termFont` 글꼴 행(설명 키만 `pref.consoleFont.desc`) ③ `pref.ctxFont` 글꼴 행 ④ `pref.statusFont` 글꼴 행 ⑤ `pref.listFont` 글꼴 행 ⑥ `pref.folderBold` 체크 ⑦ `pref.hdrBold` 체크 ⑧ `pref.hdrItalic` 체크 ⑨ `pref.dlgFont` 글꼴 행
- **일반 › 언어**(`lang`): ① `pref.lang` 라디오(시스템 + 발견 언어 코드)
- **파일 목록 › 보기·정렬**(`list`): ① `pref.showHidden` 체크 ② `pref.showDotfiles` 체크 ③ `pref.colAutofitMax` 숫자 ④ `pref.colLayout` [편집…] ⑤ `pref.sortFoldersFirst` 체크 ⑥ `pref.hideEmptyGlyph` 체크 ⑦ `pref.sortCaseSensitive` 체크 ⑧ `pref.viewScope` 라디오(전체/좌·우 패널/활성 탭) ⑨ `pref.navUpAlign` 라디오(상단/중단/하단)
- **파일 목록 › 타입어헤드**(`typeahead`): ① `pref.taScope` 라디오(전체에서 처음부터/현재 계층/가시 스트림) ② `pref.taReset` 숫자 ③ `pref.taSpecial` 체크 ④ `pref.taSpace` 체크 ⑤ `pref.taBackspace` 체크 ⑥ `pref.taPos` 위치 드롭다운
- **파일 목록 › 고속 스크롤**(`scroll`): ① `pref.fsEnabled` 체크 ② `pref.fsGridExtra` 체크 ③ `pref.fsStep` 숫자 ④ `pref.fsMax` 숫자 ⑤ `pref.fsWindow` 숫자 ⑥ `pref.fsHud` 체크 ⑦ `pref.fsHudPos` 위치 드롭다운 ⑧ `pref.fsHudHold` 숫자 ⑨ `pref.fsHudFade` 숫자
- **파일 목록 › 컨텍스트 메뉴**(`ctxmenu`): ① `pref.ctxMenuOrder` [편집…]
- **파일 목록 › 파일 전송**(`transfer`): ① `pref.transferClose` 숫자 ② `pref.dndHover` 숫자
- **탭**(`tabs`): ① `pref.tabDblclick` 라디오(탭 닫기/탭 고정/탭 잠금)
- **하단 도크 › 하단 도크**(`dock`): ① `pref.dock` 체크
- **하단 도크 › 터미널**(`terminal`): ① `pref.termWrap` 체크 ② `pref.termCols` 숫자 ③ `pref.termTheme` 드롭다운 ④ `pref.termThemeDark` 드롭다운 ⑤ `pref.termThemeLight` 드롭다운 ⑥ `pref.termCopy` 드롭다운
- **플러그인**(`plugins`): 동적 목록(PREFS-030)

> wiki 표(`dir2-doc:wiki/기능-설정.md:17-27`)는 일부가 코드와 다르다(예: 숨김/점 파일·도크를 "모양"에, 터미널 글꼴을 "터미널"에 적음). **코드가 기준**이다.

### 2-5. 위치 드롭다운 그리기 (PREFS-311)

`dir2:prefs.rs:2164-2322`. 머리 56×26: 흰 바탕 + 1px 테두리(평소 RGB(C8,C8,C8), 포커스/눌림 = accent RGB(00,78,D4)) · 타일 = 좌 5px, 높이 `max((h−8)×4/5, 6)`, 폭 = 높이×4/3 · ▾ = 우측 10px 지점 4줄 삼각형(폭 7→1, 색 RGB(30,30,30)). 타일: 4:3 화면 테두리 1px + 위치 박스(화면의 30%×28%, 최소 3px, 안쪽 여백 2) — 선택 = accent, 아니면 회색 RGB(A0,A0,A0). 팝업: 3열×3행, 셀 40×32(타일은 셀에서 좌우 4·상하 3 안쪽), 머리 좌측 정렬·머리 아래 2px, 호버 배경 RGB(CC,E6,F7). 값 = `행×3+열`(0=좌상 … 8=우하).

### 2-6. 순서 편집 창 (PREFS-312)

`dir2:ordereditor.rs:262-336`. 창: 캡션+시스템 메뉴+테두리(리사이즈 불가), 소유자 중앙, 크기 `폭 = 14×2 + 260 + 40 = 328`, `높이 = 14×2 + 24 + 트리 높이 + 8 (+30 캡션 보정)`. 트리: (14, 14), 폭 260, 높이 = `ordertree::height_for(min(행 수, 12))`. ▲ 버튼: (284, 16) 16px SVG `assets/ui/arrow-up.svg` · ▼ 버튼: (284, 40) `arrow-down.svg` — 둘 다 선택 전 비활성. 트리 행 = (라벨, 레벨 0/1, 체크 유무): 체크 박스는 **첫 컬럼 고정**(레벨 무관 x), 그룹 행은 체크 뒤 셰브론(접기), 컬럼 편집(`flat`)은 그룹 헤더 없이 자식만(`dir2:ordereditor.rs:62-86` · `dir2:ctl/ordertree.rs:1-23`).

세 편집기 스펙:

| 편집기 | 제목 키 | 정의 | 평면 | 잠금 | 라벨 원천 | 근거 |
| --- | --- | --- | --- | --- | --- | --- |
| 도구 모음 | `pref.toolbarOrder` | `TOOLBAR_BLOCKS` | 아니오 | 없음 | `tbo_label` | `dir2:prefs.rs:1011-1020` · `:1063-1083` |
| 파일 컬럼 | `pref.colLayout` | `COLUMN_BLOCKS` | 예 | `name` | `col_label`(`col.name` 등) | `dir2:prefs.rs:1023-1044` |
| 컨텍스트 메뉴 | `pref.ctxMenuOrder` | `CTXMENU_BLOCKS` | 아니오 | 없음 | `ctxm_label` | `dir2:prefs.rs:2378-2388` · `:1047-1060` |

### 2-7. 탭 순서·단축키 (PREFS-313)

- 탭 순서: 검색 → 트리 → 본문 컨트롤(생성 순서, 모두 탭 정지 지정). 단 모달 루프에 대화상자 키 처리(`IsDialogMessage`) 호출이 없어 **Tab 키 순회는 동작하지 않는 것으로 추정**(`dir2:prefs.rs:3032-3067`). dir3는 nexa-sql 설정 창의 포커스 순회 규약을 따른다.
- `Enter` = 편집 중 값 즉시 적용(PREFS-018) · `Ctrl+,` = 열기 · 휠 = 본문 스크롤 · 순서 편집 창 키는 PREFS-028.
- 컨트롤 id 대역(이식 시 이벤트 라우팅 참고, `dir2:prefs.rs:847-873`): 검색 1002 · 트리 1100 · 필드 1200+필드 id · 라디오 1400+순번 · 그룹 링크 1600+트리 인덱스 · 수정됨 바 1997 · 설명 1998 · 플러그인 2100+순번.

---

## 3. nexa-ui 매핑

`ui:nexa-ctl/src/controls`를 Grep으로 확인한 결과다. nexa-sql 설정 창(`sql:nexa-sql/src/prefs_win.rs`)이 이미 같은 골격(검색 + 트리 + 스크롤 카드)을 nexa-ui 컨트롤로 구현하고 있어 **구조 기준**으로 삼는다.

| ID | dir2 컨트롤/그리기 | nexa-ui 대응 | 상태 | 비고 |
| --- | --- | --- | --- | --- |
| PREFS-401 | 설정 창 자체(전용 창 클래스·모달 루프) | winit 별도 창 + softbuffer 표시(`PrefsWin` 패턴 `sql:nexa-sql/src/prefs_win.rs:112`) | 앱 구현(조립층은 nexa-ui에 없음 — `ui:nexa-dlg/src/lib.rs:1`은 FilePicker뿐) | dir3 앱 코드로 작성. 공용화하려면 nexa-ui에 "설정 창 조립층" 추가 검토 |
| PREFS-402 | 검색박스 `ctl::searchbox` | `TextBox::new(placeholder)` + `with_clearable()` | **있음** `ui:nexa-ctl/src/controls/textbox.rs:812` · `:2620` | 변경 통지 `take_changed` `:3063` |
| PREFS-403 | 사이드바 트리(오너드로 LISTBOX) | `TreeView` + `TreeModel`/`TreeNode` | **있음** `ui:nexa-ctl/src/controls/tree.rs:493` · `:98` · `:27` | 검색 중 `(N)` 접미·강제 펼침은 모델 재구성으로. 셰브론은 코드 도형(`draw_chevron_right`/`draw_chevron_down` `ui:nexa-ctl/src/controls/mod.rs:767` · `:661`) |
| PREFS-404 | 구분선 | `Splitter`(드래그 가능) 또는 고정 선 그리기 | **있음** `ui:nexa-ctl/src/controls/splitter.rs:40` | dir2는 고정폭(180) — 배치 유지 원칙이면 고정 선 |
| PREFS-405 | 본문 스크롤 컨테이너 + 오버레이 바 | `ScrollBars`(오프셋은 호스트 소유) | **있음** `ui:nexa-ctl/src/controls/scroll.rs:320` | 숨김 지연은 `set_hide_delay_ms` `:37`(dir2 900ms) |
| PREFS-406 | 휠 가속(`nexa_gui::fastscroll`) | `FastScroll` 전역 + `ScrollAccel` + `SpeedHud` | **있음** `ui:nexa-ctl/src/controls/scroll.rs:56` · `:258` · `:119` | 필드 구성 동일(enabled·step·max·window_ms·hud·hud_pos·hold·fade). 그리드 추가 가속은 `ScrollBars::set_fast_override`(주석 `:54`) |
| PREFS-407 | 체크박스(BS_AUTOCHECKBOX) | `Checkbox`(라벨 좌/우) | **있음** `ui:nexa-ctl/src/controls/checkbox.rs:19` | nexa-sql은 Bool에 `Switch`를 씀(`switch.rs:32`) — "배치 유지" 원칙이면 Checkbox |
| PREFS-408 | 라디오 그룹 | `RadioGroup`/`RadioOption` | **있음** `ui:nexa-ctl/src/controls/radio.rs:38` · `:14` | `take_changed` `:78` |
| PREFS-409 | 드롭다운 목록(CBS_DROPDOWNLIST) | `Combo` + `ComboItem(value, label)` | **있음** `ui:nexa-ctl/src/controls/combo.rs:534` · `:44` | 15~18항목 — 팝업 스크롤 동작 확인 필요(추정: 지원) |
| PREFS-410 | 크기 콤보(CBS_DROPDOWN — 프리셋 + 직접 입력) | `Choose`(값 직접 편집 콤보) | **있음** `ui:nexa-ctl/src/controls/combo.rs:777` | "Choose…" 항목이 붙는 형태 — 프리셋+자유 입력만 쓰는 모드가 있는지 확인 필요(추정). 없으면 옵션 추가 |
| PREFS-411 | 숫자 입력(ES_NUMBER) | `TextBox`(+ 호스트 검증) | **부분** `ui:nexa-ctl/src/controls/textbox.rs:350` | **숫자 전용 입력 필터·범위 클램프·증감 버튼 없음**(Grep 무결과). nexa-sql은 TextBox + `normalize` 즉시 검증·오류 표시. dir2 `ctl::spin`(⌃⌄ 스테퍼 `dir2:ctl/spin.rs:1`) 대응이 필요하면 **추가 필요: NumberBox/Spin**(min·max·step·숫자만·`take_committed`) — `draw_updown_chevrons`(`ui:nexa-ctl/src/controls/mod.rs:580`)는 이미 있음 |
| PREFS-412 | 글꼴 입력 `ctl::fontbox` | 없음 | **추가 필요: FontBox** | 필요한 API: 텍스트 입력 + 설치 글꼴 드롭다운(각 항목을 그 글꼴로 렌더·스크롤·↑↓PgUp/PgDn/Enter/Esc) · 입력 접두 매칭 위치로 자동 이동 · **쉼표 체인 규칙**(마지막 `,` 뒤 조각이 선택 글꼴의 접두사면 교체, 아니면 `, `로 추가, 빈 입력이면 그대로) · `is_open()`(Enter 가로채기 판단) · 확정 통지(`dir2:ctl/fontbox.rs:1-14`) |
| PREFS-413 | 설치 글꼴 목록 | 없음 | **추가 필요: nexa-font 열거 API** | 현재 공개 API는 `find_font_by_family`·`ui_font`·`mono_font`뿐(`ui:nexa-font/src/lib.rs:304` · `:417` · `:473`), 글꼴 파일 순회는 비공개(`:283`). `families() -> Vec<String>`(중복 제거·정렬) 필요 |
| PREFS-414 | 위치 이미지 드롭다운 | `PositionDropdown`(+ `PositionPicker`) | **있음** `ui:nexa-ctl/src/controls/posdrop.rs:24` · `posgrid.rs:30` | 값 코드 `tl…br`(행우선) ↔ dir2 정수 0..8 변환 필요. 설정 표현은 nexa-sql `POSITIONS` 이름(`sql:nsql-settings/src/lib.rs:253`) |
| PREFS-415 | [편집…] 버튼 | `Button` | **있음** `ui:nexa-ctl/src/controls/button.rs:90` | |
| PREFS-416 | 순서 편집 트리 `ctl::ordertree` | 없음 | **추가 필요: OrderTree** | `TreeView`에 체크 열·형제 범위 다중 선택·드래그 재배열이 없음(Grep 무결과 — 드래그 고스트는 `tooldock`/`tabbar`에만). 필요한 API: `set_rows((라벨, 레벨, Option<체크>))` · `selection()`/`set_selection()` · 같은 레벨·같은 부모 Shift 범위 · 체크 토글 통지(`take_toggled`) · 그룹 접기 · 드래그 이동(같은 부모 안·고스트·가장자리 자동 스크롤·Esc 취소) → `take_drag_delta()` · 키 이동/확장/토글 · 내용 초과 시 오버레이 스크롤(`dir2:ctl/ordertree.rs:1-23` · `:303-531`) |
| PREFS-417 | 순서 편집 창 | 앱 조립(winit 보조 창) | 앱 구현 | 모델 이동 알고리즘 `shift_range`(`dir2:ordereditor.rs:222-236`)는 그대로 이식 |
| PREFS-418 | ▲▼ 아이콘 버튼(SVG) | `Button::icon(image)` | **있음** `ui:nexa-ctl/src/controls/button.rs:165` | SVG 래스터 경로는 dir3 자원 로더 몫(dir2 `assets/ui/arrow-up.svg`·`arrow-down.svg` 그대로 사용). `GlyphKind`에는 위/아래 화살표 중 `ArrowUp`만 있음(`ui:nexa-ctl/src/controls/glyphs.rs:13-40`) |
| PREFS-419 | 라벨·제목·설명·수정됨 바 | `DrawCtx` 직접 그리기 + `FontSlot` | **있음**(nexa-sql 사용 `sql:nexa-sql/src/prefs_win.rs:8`) | 줄바꿈은 `wrap_text`(`ui:nexa-ctl/src/controls/mod.rs:490`) — dir2는 설명 1줄 고정(넘치면 잘림) |
| PREFS-420 | 종속 비활성 | `Control::set_enabled` | **있음** `ui:nexa-ctl/src/controls/mod.rs:295` | 규칙은 nexa-sql `DEPENDS`/`Dep`(`sql:nsql-settings/src/lib.rs:5944` · `:6021`) |
| PREFS-421 | 설정 저장소 | `nexa-conf`(`parse`/`serialize`/`write_atomic`/`SaveScheduler`/`Store`/`user_config_dir`/`dir_writable`/`is_replaced_on_upgrade`) | **있음** `ui:nexa-conf/src/lib.rs:53` · `:86` · `:125` · `:174` · `:235` · `:314` · `:294` · `:348` | 형식: `_schema=1` + `key=value`, 미지 키 보존, 값 속 개행은 U+2028/2029 |
| PREFS-422 | 설정 레지스트리 | nexa-sql `nsql-settings` 구조를 **dir3 전용 크레이트로 복제**(예: `ndir-settings`) | nexa-ui에는 없음(앱 소유) | `Entry{key, cat, label, desc, kind, default}` · `SettingKind{Choice, Lang, Int, Size, Bool, Text, Position}` · `CATEGORY_TREE` · `DEPENDS` · `HIDDEN` · `ADVANCED` · `OS_DEFAULTS` · `RENAMED`/`RESCALED`/`OLD_DEFAULTS`(`sql:nsql-settings/src/lib.rs:235-279` · `:5794` · `:5876` · `:41` · `:90` · `:31`) |
| PREFS-423 | 목록형 설정(런처 항목 등) | `ListEditor`(`;` 구분 한 줄 ↔ 목록 편집) | **있음** `ui:nexa-ctl/src/controls/listedit.rs:37` | dir2는 런처 UI CRUD 없음(파일 직접 편집) — 선택 사항 |

---

## 4. OS 분기점

| ID | 항목 | Windows 현 구현 | macOS | Linux | 근거 |
| --- | --- | --- | --- | --- | --- |
| PREFS-501 | 설정 폴더 | exe 옆 `data\` 우선 → `%LOCALAPPDATA%\NexaDir\data` | `~/Library/Application Support/<app>` · `.app` 번들/Homebrew 자리는 exe 옆 금지(`is_replaced_on_upgrade`) | `$XDG_CONFIG_HOME/<app>` 또는 `~/.config/<app>` · `/usr`·`/opt`·`/snap`·`/nix`는 exe 옆 금지 | `dir2:config.rs:362-402` · `ui:nexa-conf/src/lib.rs:314-372`. nexa-sql 규약 = `<APP>_HOME` 환경변수 우선 → `user_config_dir`(`sql:nsql-settings/src/lib.rs:156`). **결정 필요**: dir2의 포터블 우선 정책을 Windows에서 유지할지(권장: 환경변수 → [포터블 `data/`가 이미 있고 쓰기 가능하며 교체형 설치 자리가 아니면 그것] → 사용자 설정 폴더) |
| PREFS-502 | OS UI 언어 | `GetUserDefaultLocaleName` | `CFLocaleCopyPreferredLanguages` 첫 항목 | `LANGUAGE`(첫 항목) → `LC_ALL` → `LC_MESSAGES` → `LANG`(`C`/`POSIX` 제외) | `dir2:win.rs:321-330` · `sql:nsql-i18n/src/syslang.rs:1-17` |
| PREFS-503 | 시스템 테마 판정·변경 감지 | 레지스트리 + `WM_SETTINGCHANGE` 재해석 + DWM 다크 타이틀바 | `defaults read -g AppleInterfaceStyle` + winit `ThemeChanged` | `gsettings get org.gnome.desktop.interface color-scheme`(변경 감지 = 다음 실행 또는 Wayland `ThemeChanged`) | `dir2:win.rs:9663-9673` · `:5558-5568` · `sql:nexa-sql/src/theme.rs:5-11` |
| PREFS-504 | 기본 글꼴 이름 | UI "Segoe UI" · 터미널 "Consolas" · 아이콘 "Segoe MDL2 Assets" | UI = 시스템 한글 UI(Apple SD Gothic Neo) · 고정폭 = Menlo 계열 | UI = Noto Sans CJK KR 등 · 고정폭 = DejaVu Sans Mono 등 | `dir2:config.rs:262-281` · `ui:nexa-font/src/lib.rs:1-10`. 방안: 기본값을 빈 문자열(= 시스템 기본, nexa-sql `ui.font_face` 선례 `sql:nsql-settings/src/lib.rs:744-750`) 또는 `OS_DEFAULTS` 표로. MDL2 글리프는 코드 도형으로 대체 |
| PREFS-505 | 글꼴 크기 단위 | 대화상자 = pt(GDI, `dpi/72` 환산) · 나머지 = DIP | 동일 규약으로 통일 필요 | 〃 | `dir2:prefs.rs:2828-2829` · `dir2:config.rs:83` · `:106`. 방안: nexa-sql `Size` 종류(`13`·`13px`·`10pt`, `sql:nsql-settings/src/lib.rs:242` · `:6459`) |
| PREFS-506 | 설치 글꼴 열거 | GDI 열거(fontbox) | CoreText 가용 패밀리 목록 | 글꼴 폴더 순회(fontconfig 경로) | `dir2:ctl/fontbox.rs:3` · `ui:nexa-font/src/lib.rs:254-304`(순회는 있으나 비공개) |
| PREFS-507 | 모달 창 | 소유자 비활성 + 중첩 메시지 루프 | winit는 중첩 루프·소유자 비활성 없음 → 별도 창 + 메인 창 입력 차단을 앱이 흉내 | 〃 | `dir2:prefs.rs:3029-3069`. nexa-sql 설정 창은 **비모달 별도 창** 구조(`sql:nexa-sql/src/prefs_win.rs:798` — 모달성 여부는 본 조사에서 미확인, 추정) |
| PREFS-508 | 위치 팝업 | OS 팝업 메뉴(오너드로) | 창 안 오버레이 팝업(`PositionDropdown::paint_popup`) | 〃 | `dir2:prefs.rs:2293-2322` · `ui:nexa-ctl/src/controls/posdrop.rs:6` |
| PREFS-509 | 휠 줄 수·델타 | `SPI_GETWHEELSCROLLLINES` + 120 단위 델타 누적 | 픽셀 델타(트랙패드) 직접 사용 — 고속 스크롤은 "정밀 터치패드 제외" 규약 | 줄 델타/픽셀 델타 혼재 | `dir2:win.rs:6327-6344` · `dir2:prefs.rs:1272-1281` · `dir2-lang:ko.lang:388` |
| PREFS-510 | 세션 경로 직렬화 | 구분자 파이프(`\|`, Windows 경로 불가 문자) · `to_string_lossy` | **파이프·개행이 파일명에 올 수 있음** → 구분자 방식 불가. 탭마다 키 분리(`panelN.tabM=`) 또는 퍼센트 인코딩 | 〃 + 비UTF-8 파일명(바이트 보존 인코딩 필요) | `dir2:config.rs:837-847` · `:903-909`. 값 속 개행은 nexa-conf가 U+2028/2029로 보존(`ui:nexa-conf/src/lib.rs:79-84`) |
| PREFS-511 | 가상/특수 경로 | 세션에 `::PC::`(내 PC) 탭·드라이브 문자·UNC 저장 | 루트 `/`·`/Volumes` 대응 가상 경로 규약 필요 | `/`·마운트 지점 | `dir2:win.rs:1443-1446` |
| PREFS-512 | 숨김 파일 의미 | `show_hidden` = 숨김 속성 · `show_dotfiles` = 점 파일(별개 토글) | 숨김 플래그(`UF_HIDDEN`) + 점 파일이 관례상 숨김 | 숨김 속성 없음 — 점 파일만 | `dir2:config.rs:70-71` · `dir2-lang:ko.lang:306-307`. 방안: Linux에서 `show_hidden`은 효과 없음을 설명문에 명시하거나 항목 숨김 |
| PREFS-513 | 종료 저장 경로 | `WM_DESTROY`만 | 창 닫기 + 앱 종료(Cmd+Q) + 로그아웃 | 창 닫기 + SIGTERM/SIGINT | `dir2:win.rs:9674-9689`. 방안: 주기 저장(`SaveScheduler` quiet 1s·max 10s) + 종료 시 `flush_now` |
| PREFS-514 | 항상 맨 위 | TOPMOST 밴드 전환 | winit 창 레벨 AlwaysOnTop | X11 지원 · Wayland 미지원(비활성 표시 필요) | `dir2:win.rs:333-345` |
| PREFS-515 | 터미널 복사 서식 | 평문 + HTML/RTF 클립보드 형식 | NSPasteboard `public.html`/`public.rtf` | X11/Wayland `text/html`·`text/rtf` 타깃 | `dir2:config.rs:97-99` |
| PREFS-516 | 퀵 런처 시드·항목 | VS Code · pwsh · cmd, `exe` 필드 + `%path%` 치환 | Terminal/iTerm·`open -a` 계열 | `x-terminal-emulator` 등 | `dir2:config.rs:10-18` · `dir2:launcher.rs:90-112`(상세는 런처 인벤토리 문서 소관) |
| PREFS-517 | 안내 문구의 경로 | "data\plugins\ 또는 exe 옆 plugins\" | 실제 설정 폴더 경로를 치환해 표시 | 〃 | `dir2-lang:ko.lang:177-178` |
| PREFS-518 | 설정 열기 단축키 | `Ctrl+,` | `Cmd+,` | `Ctrl+,` | `dir2:win.rs:8891` |
| PREFS-519 | 우클릭 메뉴 글꼴 | 저장만(OS 메뉴는 OS 글꼴) | nexa-ui `ContextMenu` 자체 그리기 → **실제 적용 가능** | 〃 | `dir2:win.rs:6501-6522` · `dir2-lang:ko.lang:425` |
| PREFS-520 | 저장 파일 권한 | 해당 없음 | 소유자 전용(0600) + 부모 디렉터리 fsync | 〃 | `ui:nexa-conf/src/lib.rs:140-162`(client_secret 등 민감 값이 설정에 실림 — `dir2:config.rs:206-208`) |
| PREFS-521 | 쓰기 프로브 파일 | `.w{pid}` | `.probe.{pid}.{seq}`(nexa-conf — 스레드 경합 해소) | 〃 | `dir2:config.rs:405-417` · `ui:nexa-conf/src/lib.rs:285-308` |

---

## 5. 상태·영속(설정 키·파일 형식) · 스레딩·메시지 흐름

### 5-1. settings.cfg 키 전체 (PREFS-101~)

- 타입 표기: `bool` = `0`/`1`(읽기는 `≠"0"`이면 참) · `enum` = 나열 값만 허용(그 외 = 기본 유지) · `int` = 파싱 후 클램프(파싱 실패 = 기본 유지) · `f32` = 소수 3자리 기록, 유한 값만·클램프.
- "설정 창 입력 실패값" = 숫자 칸이 비거나 파싱 불가일 때 `harvest`가 넣는 값(`dir2:prefs.rs:2001-2048`).
- "즉시 반영" = 설정 창에서 바꿨을 때의 반영 시점(`apply_prefs` `dir2:win.rs:6475-6762`). 모든 키는 변경 직후 파일에 저장된다(PREFS-053).
- "dir3 제안 키" = nexa-sql 규약(`<카테고리>.<이름>`)에 맞춘 **제안**이며 확정 아님.

| ID | 키 | 타입 | 기본값 | 범위/허용 값 | UI 페이지 / 컨트롤 | 즉시 반영 | dir3 제안 키 | 근거(기본·파싱) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PREFS-101 | `theme` | enum | `dark` | system · light · dark | 일반›모양 / 라디오 (+보기 메뉴·F6 순환) | 즉시(전체 재도장·타이틀바) | `ui.theme` | `dir2:config.rs:254` · `:570` |
| PREFS-102 | `lang` | str | `system` | system 또는 코드(1~16바이트) | 일반›언어 / 라디오 (+메뉴) | 즉시(동적 전환) | `ui.lang` | `:255` · `:572` |
| PREFS-103 | `show_hidden` | bool | 1 | — | 파일 목록›보기·정렬 / 체크 (+메뉴·툴바) | 즉시 — **범위 설정과 무관하게 양 패널 전 탭** 기입 후 재열람 | `list.show_hidden` | `:256` · `:573` |
| PREFS-104 | `show_dotfiles` | bool | 1 | — | 〃 / 체크 | 〃 | `list.show_dotfiles` | `:257` · `:574` |
| PREFS-105 | `split` | f32 | 0.500 | 0.1~0.9 | 없음(패널 분할선 드래그) | — (종료 저장·다른 저장에 편승) | `layout.split` | `:258` · `:604` |
| PREFS-106 | `dock` | bool | 1 | — | 하단 도크›하단 도크 / 체크 (+메뉴 Ctrl+백틱·툴바) | 즉시(양 패널·레이아웃) | `dock.visible` | `:259` · `:575` |
| PREFS-107 | `dock_ratio` | f32 | 0.300 | 0.15~0.5 | 없음(드래그) | — | `dock.ratio` | `:260` · `:590` |
| PREFS-108 | `dock_split` | f32 | 0.500 | 0.15~0.85 | 없음(드래그) | — | `dock.split` | `:261` · `:597` |
| PREFS-109 | `term_font` | str(쉼표 = 폴백 체인) | `Consolas` | 비어 있지 않음·≤128바이트 | 일반›글꼴 / 글꼴 행(패밀리) | 확정 시(포커스 이탈·Enter·목록 선택) — 렌더 백엔드 재생성 | `term.font_face` | `:262` · `:576` |
| PREFS-110 | `term_font_size` | int(DIP) | 12 | 8~32 (입력 실패값 12) | 〃 / 크기 콤보 | 〃 | `term.font_size` | `:263` · `:579` |
| PREFS-111 | `dlg_font` | str | `Segoe UI` | 비어 있지 않음·≤64바이트 | 일반›글꼴 / 글꼴 행 | 확정 시(다음 대화상자부터) | `ui.dlg_font_face` | `:272` · `:584` |
| PREFS-112 | `dlg_font_size` | int(**pt**) | 9 | 7~24 (입력 실패값 9) | 〃 / 크기 콤보 | 〃 | `ui.dlg_font_size` | `:273` · `:585` |
| PREFS-113 | `launcher` | bool | 1 | — | 없음(보기 메뉴 토글) | — | `launcher.visible` | `:316` · `:758` |
| PREFS-114 | `term_wrap` | bool | 1 | — | 하단 도크›터미널 / 체크 | 즉시(다음 페인트에서 열 재계산·PTY 크기 변경) | `term.wrap` | `:264` · `:638` |
| PREFS-115 | `term_cols` | int | 240 | 80~1000 (입력 실패값 240) | 〃 / 숫자 | 확정 시 | `term.cols` | `:265` · `:639` |
| PREFS-116 | `term_theme` | str | `system` | system · dark · light · 스킴 id(비어 있지 않음·≤64, 모르는 id는 해석 시 system) | 〃 / 드롭다운(3 + 스킴 15) | 즉시(재도장만으로 스크롤백까지) | `term.theme` | `:266` · `:645` |
| PREFS-117 | `term_theme_dark` | str | `campbell` | 스킴 id(≤64) — UI는 다크 9종 | 〃 / 드롭다운 | 즉시 | `term.theme_dark` | `:267` · `:648` · `nexa-dir2/crates/nexa-term/src/lib.rs:147` |
| PREFS-118 | `term_theme_light` | str | `github-light` | 스킴 id(≤64) — UI는 라이트 6종 | 〃 / 드롭다운 | 즉시 | `term.theme_light` | `:268` · `:651` · `nexa-dir2/crates/nexa-term/src/lib.rs:149` |
| PREFS-119 | `term_copy_format` | enum | `text` | text · html · rtf · both | 〃 / 드롭다운 | 즉시(다음 복사부터) | `term.copy_format` | `:269` · `:654` |
| PREFS-120 | `transfer_close_ms` | int(ms) | 2000 | 0~10000, **0 = 진행 창 미표시** (입력 실패값 2000) | 파일 목록›파일 전송 / 숫자 | 확정 시(다음 전송부터) | `transfer.close_ms` | `:270` · `:657` |
| PREFS-121 | `dnd_hover_ms` | int(ms) | 3000 | 200~10000 (입력 실패값 3000) | 〃 / 숫자 | 확정 시(다음 드래그부터) | `transfer.dnd_hover_ms` | `:271` · `:662` |
| PREFS-122 | `preview_map` | str | 빈 값(빈 값이면 줄 생략) | ≤512바이트 · `ext:플러그인ID` 파이프 나열 | 없음(파일 직접 편집) | — (재시작) | `preview.map` | `:292` · `:706` |
| PREFS-123 | `plugins_disabled` | str | 빈 값(생략) | ≤512바이트 · id 파이프 나열 | 플러그인 / 플러그인별 체크 | 도크 미리보기 갱신(**클릭 즉시가 아닐 수 있음 — PREFS-618**) | `plugins.disabled` | `:293` · `:708` |
| PREFS-124 | `sort_folders_first` | bool | 1 | — | 파일 목록›보기·정렬 / 체크 (+툴바) | 즉시(양 패널 전 탭 — 범위 무관) | `list.folders_first` | `:285` · `:673` |
| PREFS-125 | `sort_case_sensitive` | bool | 0 | — | 〃 / 체크 | 즉시(전 탭 재정렬) | `list.sort_case_sensitive` | `:289` · `:679` |
| PREFS-126 | `nav_up_align` | enum | `center` | top · center · bottom | 〃 / 라디오 | 즉시 | `list.nav_up_align` | `:290` · `:680` |
| PREFS-127 | `tab_dblclick` | enum | `close` | close · pin · lock | 탭 / 라디오 | 즉시 | `tabs.dblclick` | `:291` · `:702` |
| PREFS-128 | `view_mode` | enum | `tree` | tree · flat · tiles | 없음(보기 메뉴·툴바) — 마지막 선택 = 새 세션/새 탭 기본 | — | `list.view_mode` | `:309` · `:697` · `dir2:win.rs:5186` |
| PREFS-129 | `panel_mode` | enum | `dual` | single · dual | 없음(메뉴·툴바) | — | `layout.panel_mode` | `:314` · `:700` |
| PREFS-130 | `info_mode` | enum | `dual` | single · dual (싱글 패널에서는 효과만 싱글 강제·값 보존) | 없음(메뉴·툴바) | — | `layout.info_mode` | `:315` · `:701` |
| PREFS-131 | `view_scope` | enum | `panel` | global · panel · tab | 파일 목록›보기·정렬 / 라디오 | 즉시(툴바 재구성 — 툴팁에 범위 표기) | `list.view_scope` | `:286` · `:674` |
| PREFS-132 | `hide_empty_glyph` | bool | 1 | — | 〃 / 체크 | 즉시(전 탭) | `list.hide_empty_glyph` | `:287` · `:677` |
| PREFS-133 | `always_on_top` | bool | 0 | — | 없음(보기 메뉴·툴바 `ontop`) | — | `window.always_on_top` | `:288` · `:678` |
| PREFS-134 | `col_width_sync` | bool | 1 | — | 없음(메뉴·툴바 `colsync`) | — | `list.col_width_sync` | `:310` · `:683` |
| PREFS-135 | `col_autofit_max` | int(px @96dpi) | 400 | 50~2000 (입력 실패값 400) | 파일 목록›보기·정렬 / 숫자 | 확정 시 | `list.col_autofit_max` | `:311` · `:684` |
| PREFS-136 | `toolbar_order` | 순서 문자열 | 아래 ① | `TOOLBAR_BLOCKS` 문법(읽을 때 정규화) | 일반›모양 / [편집…] (+툴바 우클릭) | 편집 창 조작마다 즉시(툴바 재구성) | `toolbar.order` | `:312` · `:690` |
| PREFS-137 | `ctx_menu_order` | 순서 문자열 | 아래 ② | `CTXMENU_BLOCKS` 문법 | 파일 목록›컨텍스트 메뉴 / [편집…] | 즉시(다음 메뉴 표시부터) | `ctxmenu.order` | `:313` · `:693` |
| PREFS-138 | `typeahead_scope` | enum | `visible` | global · level · visible | 파일 목록›타입어헤드 / 라디오 | 즉시(전 탭) | `typeahead.scope` | `:294` · `:709` |
| PREFS-139 | `typeahead_reset_ms` | int | 1000 | 200~10000 (입력 실패값 1000) | 〃 / 숫자 | 확정 시 | `typeahead.reset_ms` | `:295` · `:712` |
| PREFS-140 | `typeahead_pos` | int | 6(좌하) | 0~8(3×3 행우선) | 〃 / 위치 드롭다운 | 즉시 | `typeahead.hud_pos` | `:296` · `:717` |
| PREFS-141 | `typeahead_special` | bool | 1 | — | 〃 / 체크 | 즉시 | `typeahead.special` | `:297` · `:722` |
| PREFS-142 | `typeahead_space` | bool | 1 | — | 〃 / 체크 | 즉시 | `typeahead.space` | `:298` · `:723` |
| PREFS-143 | `typeahead_backspace` | bool | 1 | — | 〃 / 체크 | 즉시 | `typeahead.backspace` | `:299` · `:724` |
| PREFS-144 | `fast_scroll` | bool | 1 | — | 파일 목록›고속 스크롤 / 체크 | 즉시(전역 핫스왑) + 하위 8항목 활성 연동 | `scroll.fast` | `:300` · `:725` |
| PREFS-145 | `fast_scroll_step` | int | 3 | 1~50 (입력 실패값 3) | 〃 / 숫자(부모 = fast_scroll) | 확정 시 | `scroll.fast_step` | `:301` · `:726` |
| PREFS-146 | `fast_scroll_max` | int | 16 | 1~32 (입력 실패값 16) | 〃 / 숫자(부모 = fast_scroll) | 확정 시 | `scroll.fast_max` | `:302` · `:731` |
| PREFS-147 | `fast_scroll_window_ms` | int | 160 | 20~2000 (입력 실패값 160) | 〃 / 숫자(부모 = fast_scroll) | 확정 시 | `scroll.fast_window_ms` | `:303` · `:736` |
| PREFS-148 | `fast_scroll_hud` | bool | 1 | — | 〃 / 체크(부모 = fast_scroll) | 즉시 + 하위 3항목 활성 연동 | `scroll.fast_hud` | `:304` · `:741` |
| PREFS-149 | `fast_scroll_hud_pos` | int | 2(우상) | 0~8 | 〃 / 위치 드롭다운(부모 = fast_scroll && hud) | 즉시 | `scroll.fast_hud_pos` | `:305` · `:743` |
| PREFS-150 | `fast_scroll_hud_hold_ms` | int | 250 | 0~10000 (입력 실패값 250) | 〃 / 숫자(부모 = 〃) | 확정 시 | `scroll.fast_hud_hold_ms` | `:306` · `:748` |
| PREFS-151 | `fast_scroll_hud_fade_ms` | int | 600 | 0~10000 (입력 실패값 600) | 〃 / 숫자(부모 = 〃) | 확정 시 | `scroll.fast_hud_fade_ms` | `:307` · `:753` |
| PREFS-152 | `fast_scroll_grid_extra` | bool | 1 | — (step−1·상한 2배) | 〃 / 체크(부모 = fast_scroll) | 즉시 | `scroll.fast_grid_extra` | `:308` · `:742` |
| PREFS-153 | `base_font` | str | `Segoe UI` | 비어 있지 않음(길이 상한 없음) | 일반›글꼴 / 글꼴 행 | 확정 시(백엔드 재생성) | `ui.font_face` | `:274` · `:611` |
| PREFS-154 | `base_font_size` | int(DIP) | 12 | 8~32 (입력 실패값 12) | 〃 / 크기 콤보 | 〃 | `ui.font_size` | `:275` · `:612` |
| PREFS-155 | `ctx_font` | str | `Segoe UI` | 비어 있지 않음 | 〃 / 글꼴 행 | 저장만(현재 효과 없음 — PREFS-519) | `ctxmenu.font_face` | `:276` · `:617` |
| PREFS-156 | `ctx_font_size` | int | 12 | 8~32 (입력 실패값 12) | 〃 / 크기 콤보 | 저장만 | `ctxmenu.font_size` | `:277` · `:618` |
| PREFS-157 | `status_font` | str | `Segoe UI` | 비어 있지 않음 | 〃 / 글꼴 행 | 확정 시(백엔드 재생성) | `status.font_face` | `:278` · `:623` |
| PREFS-158 | `status_font_size` | int | 12 | 8~32 (입력 실패값 12) | 〃 / 크기 콤보 | 〃 | `status.font_size` | `:279` · `:624` |
| PREFS-159 | `list_font` | str | `Segoe UI` | 비어 있지 않음 | 〃 / 글꼴 행 | 확정 시(백엔드 재생성) | `list.font_face` | `:280` · `:629` |
| PREFS-160 | `list_font_size` | int | 12 | 8~32 (입력 실패값 12) | 〃 / 크기 콤보 | 〃 | `list.font_size` | `:281` · `:630` |
| PREFS-161 | `list_folder_bold` | bool | 0 | — | 〃 / 체크 | 즉시(전 탭) | `list.folder_bold` | `:282` · `:635` |
| PREFS-162 | `header_bold` | bool | 0 | — | 〃 / 체크 | 즉시 | `list.header_bold` | `:283` · `:636` |
| PREFS-163 | `header_italic` | bool | 0 | — | 〃 / 체크 | 즉시 | `list.header_italic` | `:284` · `:637` |
| PREFS-164 | `launcher_seed` | u32 | 0(저장 시 항상 현재 시드 버전 2) | 파싱 실패 = 0 | 없음 | — | `launcher.seed` | `:318` · `:759` · `dir2:win.rs:7013` |
| PREFS-165 | `launcher_count` | usize | 키 부재 | 존재만으로 "항목 목록 확정"(값은 읽지 않음) | 없음(파일 직접 편집) | — | `launcher.count` | `:550` · `:761-765` |
| PREFS-166 | `launcher{N}` | 복합 | 키 부재 → 시드 | 아래 ③ · 상한 32 | 없음 | — | `launcher.item{N}` | `:549-561` · `:766-790` |
| PREFS-167 | `cloud{N}` | 복합 | 없음 | 아래 ④ · 상한 32 | 없음(Cloud 메뉴) | — | `cloud.conn{N}` | `:528-536` · `:808-828` |
| PREFS-168 | `cloud_client_id_{kind}` | str | 없음 | ≤256바이트, 빈 값 미기록 | 없음(파일 직접 편집) | — (재시작) | `cloud.client_id.{kind}` | `:538-542` · `:799-805` |
| PREFS-169 | `cloud_client_secret_{kind}` | str | 없음 | ≤256바이트, 빈 값 미기록 | 없음 | — | `cloud.client_secret.{kind}` | `:544-548` · `:792-798` |
| PREFS-170 | `transfer_close_secs`(구 키) | int(초) | — | 읽기 전용 이행: ×1000 후 0~10000 | — | — | (이행 표에만) | `:667-672` |

복합 값 형식(표 안에 파이프를 쓸 수 없어 분리):

```text
① toolbar_order 기본값
   refresh:1|panel:1[toggle:1,dock:1,info:1,colsync:1,ontop:1]|view:1[tree:1,flat:1,tiles:1]|show:1[hidden:1,dot:1,foldersfirst:1]|settings:1
② ctx_menu_order 기본값
   row:1[new:1,deletePermanent:1,copyName:1,pasteInto:1]|bg:1[paste:1,undo:1,redo:1]
   (컬럼 레이아웃 기본값 = cols:1[name:1,ext:1,size:1,modified:1,kind:1] — session.cfg 소속)
③ launcherN=라벨|exe|인자      (인자 안의 | 는 보존 · 라벨/exe는 trim · 둘 중 하나라도 비면 무시)
   launcherN=-                 (그룹 구분선)
④ cloudN=kind|라벨|경로|account (kind·라벨 필수 + 경로 또는 account 중 하나 이상 · 라벨의 | 는 저장 시 / 로 치환)
   kind = onedrive | googledrive | dropbox · 경로 빔 = API 직접 연결
```

정의 원천: `TOOLBAR_BLOCKS`(`dir2:config.rs:1036-1044`) · `COLUMN_BLOCKS`(`:1075`) · `CTXMENU_BLOCKS`(`:1078-1082`). 직렬화 순서(파일 내 키 순서)는 `dir2:config.rs:432-563` — dir3는 nexa-conf 형식을 쓰므로 순서 재현은 불필요하나, **dir2 설정 가져오기**를 만들 경우 위 표가 변환표다(불리언 0/1 → on/off는 nexa-sql `normalize`가 그대로 수용 `sql:nsql-settings/src/lib.rs:6441-6445`, 위치 0..8 → `POSITIONS[n]`).

**설계 주의**: `launcher{N}`·`cloud{N}`·`cloud_client_*_{kind}`는 **동적 키 군**이라 정적 레지스트리(`REGISTRY: &[Entry]`)에 그대로 들어가지 않는다. 방안: (a) 별도 파일(`launcher.conf`·`cloud.conf`) (b) 한 키에 목록 직렬화 (c) nexa-conf의 "미지 키 보존"에 기대지 말고 접두사 군을 레지스트리 밖 전용 섹션으로 관리.

### 5-2. session.cfg 키 전체 (PREFS-201~)

| ID | 키 | 형식 | 기록 조건 | 읽기 규칙 | 근거 |
| --- | --- | --- | --- | --- | --- |
| PREFS-201 | `active_panel` | 0/1 | 항상 | 파싱 실패 0 · `min(1)` | `dir2:config.rs:840` · `:902` |
| PREFS-202 | `panel{i}.tabs` | 탭 루트 경로를 파이프로 연결 | 항상(i=0,1) | 빈 조각 제거 | `:842-847` · `:903-910` |
| PREFS-203 | `panel{i}.active` | 정수 | 항상 | 파싱 실패 0 · 복원 시 `min(탭 수−1)` | `:848` · `:911-914` · `dir2:panel.rs:251` |
| PREFS-204 | `panel{i}.exp{j}` | 탭 j의 펼침 경로 파이프 연결 | 비어 있지 않을 때만 · 저장 시 탭당 최대 200개 | j > 64 무시 · 인덱스 정렬 유지(빈 자리 허용) | `:850-858` · `:959-976` · `dir2:panel.rs:279-289` |
| PREFS-205 | `panel{i}.locked` | `0`/`1` 파이프 연결(탭 순) | 하나라도 잠겼을 때만 | `"1"`만 참 | `:860-867` · `:915-918` |
| PREFS-206 | `panel{i}.pinned` | 〃 | 하나라도 고정일 때만 | 〃 | `:869-876` · `:919-922` |
| PREFS-207 | `panel{i}.modes` | `tree`/`flat`/`tiles` 파이프 연결 | 전부 tree면 생략 | 미지 값 = tree | `:878-880` · `:946-958` |
| PREFS-208 | `panel{i}.views` | 탭별 플래그 정수 파이프 연결(bit0 숨김 · bit1 점 파일 · bit2 폴더 우선) | 비어 있지 않을 때 | 파싱 실패 0 · `& 0x7` · 부족분 = 설정 기본 | `:882-885` · `:923-931` · `dir2:panel.rs:1081-1104` |
| PREFS-209 | `panel{i}.cols` | 컬럼 순서 문자열(`COLUMN_BLOCKS`) | 비어 있지 않을 때 | 읽을 때 정규화 | `:887-889` · `:932-937` |
| PREFS-210 | `panel{i}.colw` | 컬럼 폭(px) **쉼표** 연결 — 표시 컬럼의 표시 순 대응 | 비어 있지 않을 때 | 숫자만 수집 | `:890-893` · `:938-945` |
| PREFS-211 | `panel{i}.colw{j}` | (예약) 탭별 폭 | 미구현 | — | `dir2:config.rs:343-345` |

세션 값의 소유: 탭 경로·활성·펼침·잠금·고정·보기 모드·보기 플래그 = **탭**, 컬럼 레이아웃·폭 = **패널**(탭은 패널 상속). 설정 창의 "파일 컬럼" 편집은 포커스 패널의 `panel{i}.cols`를 바꾼다(`col_width_sync`면 반대 패널에도 — `dir2:win.rs:6583-6593`).

### 5-3. 삼중 매핑 구조(현 구조의 비용)

같은 값이 세 구조체에 따로 있다: `config::Settings`(파일) ↔ 호스트 `State` 필드 ↔ `prefs::PrefValues`(창). 값 하나를 추가하려면 직렬화·파싱·기본값·`open_prefs`·`apply_prefs`·`current_settings`·`harvest`·`is_modified`·컨트롤 초기값 등 **9~15곳**을 손으로 맞춰야 한다(`dir2-doc:audit/20260904-165351/02-settings.md:21`). 클램프도 3곳(`dir2:config.rs` 파서 · `dir2:prefs.rs:982` · `dir2:win.rs` apply)에 중복된다. dir3는 nexa-sql 레지스트리 단일 원천으로 이 구조를 없앤다(PREFS-422).

### 5-4. 스레딩·메시지 흐름

- **전부 UI 단일 스레드**. 설정 창·편집 창은 중첩 메시지 루프, 통지는 같은 스레드 동기 호출(포인터는 통지 동안만 유효 → 수신 측 즉시 복사 — `dir2:prefs.rs:45-47` · `dir2:win.rs:9060-9066`).
- i18n 활성 테이블 = UI 스레드 `thread_local`(`dir2:i18n.rs:167-173`).
- 고속 스크롤 설정 = 전역 값(모든 스크롤 영역이 사건마다 읽음 — `dir2:win.rs:1421-1423` · `:6699-6720`).

```
[설정 창 열기]  Ctrl+, / 메뉴·툴바(PostMessage 지연) ─▶ open_prefs
     State ─(스냅샷)─▶ PrefValues ─▶ prefs::show(모달)
[값 변경]  컨트롤 사건 ─▶ harvest() ─▶ sanitize ─▶ (동기 통지) ─▶ apply_prefs
     apply_prefs: 항목별 동등 비교 → 적용 → current_settings → config::save(settings.cfg) → 전체 무효화
[순서 편집] [편집…] ─▶ ordereditor::show(모달) ─(조작마다 동기 통지)─▶ 설정 창 values 갱신 ─▶ apply_now
[닫기]  WM_CLOSE: harvest ─▶ show 반환 ─▶ apply_prefs(최종·멱등)
[세션]  패널: session_dirty=true ─▶ update_status: 양 패널 수거 → 타이머(1s) 재무장 ─▶ 만료: current_session → save(session.cfg)
[종료]  WM_DESTROY: settings + session 저장 → 구 .txt 삭제
```

dir3 대응(nexa-sql 구조): 창은 `PrefsAction::Changed{key,value}`를 호스트에 **반환**하고, 호스트가 `Settings::set`(검증·정규화) → 적용 → 저장한다. 저장은 `SaveScheduler`(변경 = 플래그, quiet 1s 또는 max 10s에 1회 쓰기) + 직전 저장분과 같으면 쓰지 않음(`ui:nexa-conf/src/lib.rs:166-281`).

---

## 6. 이식 시 주의 — 실측 교훈·결함 이력(회귀 방지)

| ID | 교훈 / 결함 | 내용 | 근거 |
| --- | --- | --- | --- |
| PREFS-601 | 재구성 중 편집 값 유실 | 컨트롤을 파괴하면 포커스 이탈 통지가 **동기 재진입**해 파괴된 컨트롤에서 빈 문자열을 수확 → 글꼴 이름이 공백이 됨. 수확 목록을 파괴 전에 비워 해결. dir3: 스크롤·페이지 전환·리사이즈로 **편집 중 값이 빈 값으로 덮이면 안 된다** | `dir2:prefs.rs:1285-1291` |
| PREFS-602 | 스크롤 = 이동이지 재구성이 아님 | 휠마다 전 컨트롤을 재생성하던 구조가 창 전체 깜박임의 원인. 스크롤 중 편집 값·포커스·캐럿 유지가 요구 사항 | `dir2:prefs.rs:811-817` · `:1226-1228` |
| PREFS-603 | 트랙패드 미세 휠 | `델타/120` 정수 나눗셈이 작은 델타를 0으로 버려 느린 스크롤이 죽음 → 잔량 누적 | `dir2:prefs.rs:1270-1275` |
| PREFS-604 | 트리 선택 하이라이트 잔상 | 선택 판정이 `category` 상태 기준이라, 선택 변경 직후 트리 전체를 다시 그려야 옛 행 하이라이트가 지워짐 | `dir2:prefs.rs:1854-1857` |
| PREFS-605 | `&` 문자 표시 | "View & Sort" 같은 라벨의 `&`를 단축키 접두로 해석하면 안 됨(제목·링크·트리·캡션 전부) | `dir2:prefs.rs:1323` · `:2153` |
| PREFS-606 | 크기 콤보 선택 시점 | 목록 선택 통지 시점에는 입력부가 아직 이전 값 → 선택 항목 텍스트를 직접 반영한 뒤 수확 | `dir2:prefs.rs:2502-2523` |
| PREFS-607 | Enter와 드롭다운 충돌 | 창 수준 Enter 가로채기가 글꼴 드롭다운의 "Enter = 목록 확정"을 죽였음 → 드롭다운 열림 여부 확인 후 가로챔 | `dir2:prefs.rs:3052-3061` |
| PREFS-608 | 플러그인 목록 덮어쓰기 방지 | 플러그인 체크박스가 **현재 페이지에 있을 때만** `plugins_disabled`를 재구성(다른 페이지의 수확이 값을 지우지 않게) | `dir2:prefs.rs:2078-2088` |
| PREFS-609 | 비활성 항목 가시성 | 종속으로 비활성된 입력의 글자가 검정이라 비활성이 안 보였음 → 회색 | `dir2:prefs.rs:2584-2590` |
| PREFS-610 | 같은 레벨 라벨 정렬 | 트리 마커 존은 하위 유무와 무관하게 고정 폭 예약(파일 목록과 동일 규약) | `dir2:prefs.rs:2114-2117` |
| PREFS-611 | 직렬화 누락 = 저장 시 유실 | `cloud_client_secret_*`가 파싱만 되고 직렬화되지 않아 다음 저장에 사라졌음. **모든 키가 왕복 테스트에 포함**돼야 함(레지스트리 단일 원천이면 구조적으로 해소) | `dir2:config.rs:543-548` · `dir2-doc:audit/20260904-165351/02-settings.md:8` |
| PREFS-612 | i18n 스레드 | `tr()`은 스레드 로컬 — 워커 스레드에서 부르면 항상 영어. 워커는 키만 넘기고 UI 스레드에서 번역하거나, dir3에서는 전역 테이블로 바꿀 것 | `dir2:i18n.rs:7` · `:167-173` |
| PREFS-613 | 원자적 저장 3원칙 | ① 옛 파일 선삭제 금지 ② `sync_all` ③ 임시 파일명에 pid(다중 인스턴스 교차 방지) | `dir2:config.rs:990-1011` |
| PREFS-614 | 런처 "부재"와 "비움" 구분 | 키 부재 = 첫 실행(시드 주입) · count=0 = 사용자가 비움(재주입 금지). 시드 버전으로 신규 시드 1회만 추가 | `dir2:config.rs:193-198` · `:1543-1549` |
| PREFS-615 | 순서 문자열 전방 호환 | 신규 버튼/항목은 저장된 구 순서에서도 **정의상 앞 형제 뒤**에 삽입(끝에 붙지 않게) · 미지 토큰 제거 · 구형(vis 생략) 수용 | `dir2:config.rs:1150-1161` · `:1206-1213` |
| PREFS-616 | 세션 플래그 수거는 비단락 | 양 패널 플래그를 단락 OR(`\|\|`)로 수거하면 우 패널 플래그가 남음 → 항상 둘 다 소진 | `dir2:win.rs:5072` · `dir2-doc:20-session-coalescing.md:64-65` |
| PREFS-617 | 디바운스 기아 | 변경이 1초 미만 간격으로 계속되면 저장이 무한 연기 → dir3는 최대 지연 상한(`max_delay`) 사용 | `dir2-doc:20-session-coalescing.md:109-111` · `ui:nexa-conf/src/lib.rs:168-172` |
| PREFS-618 | (추정 결함) 플러그인 체크가 즉시 적용되지 않음 | 플러그인 체크 id(2100+)가 명령 분기에서 **라디오 분기(id ≥ 1400)에 먼저 걸려** 아무 일도 하지 않음 → 다른 항목 변경·페이지 이동·닫기 때에야 반영. 주석은 "즉시"를 의도. dir3는 의도(즉시 적용)대로 구현하고 테스트로 고정 | `dir2:prefs.rs:871-873` · `:2453-2471` · `:2527` |
| PREFS-619 | (추정 결함) 도구 모음 편집 창 라벨 누락 | `tbo_label`에 `panel/ontop`·`show/foldersfirst` 매핑이 없어 원시 key가 그대로 표시됨. dir3는 `menu.view.alwaysOnTop`·`pref.sortFoldersFirst` 등으로 연결 | `dir2:prefs.rs:1063-1083` · `dir2:config.rs:1040-1042` |
| PREFS-620 | (추정 결함) 탭 인덱스 어긋남 | 세션 복원이 열기 실패 탭을 건너뛰는데, 잠금·고정·보기 모드·보기 플래그는 **건너뛴 뒤의 인덱스**로 적용됨 → 중간 탭이 사라지면 뒤 탭들의 속성이 한 칸씩 밀림(펼침만 원래 인덱스 사용). dir3는 탭별 레코드로 묶어 저장/복원 | `dir2:panel.rs:209-216` · `:710-717` · `:692-697` · `:1081-1092` · `dir2:win.rs:1480-1531` |
| PREFS-621 | 빈 글꼴 값 불일치 | 정규화가 `base/ctx/status/list_font` 빈 값을 막지 않아 State는 빈 문자열, 파일 파서는 거부 → 재기동 시 기본값으로 되돌아감 | `dir2:prefs.rs:982-988` · `dir2:config.rs:611` · `dir2-doc:audit/20260904-165351/02-settings.md:21` |
| PREFS-622 | 무조건 저장·전체 무효화 | `apply_prefs` 끝의 저장과 전체 무효화는 게이트가 없음(포커스 이동마다 파일 쓰기) → 직전 저장분 비교 또는 스케줄러로 | `dir2:win.rs:6757-6761` · `dir2-doc:audit/20260904-165351/02-settings.md:10` · `:14` |
| PREFS-623 | 비UTF-8 설정 파일 | 손편집으로 인코딩이 깨지면 전체 기본값으로 뜨고 첫 저장이 덮어씀(백업·경고 없음) → 손실 방지책(`.bad` 사본) 권장 | `dir2:config.rs:986-988` · `dir2-doc:audit/20260904-165351/02-settings.md:15` |
| PREFS-624 | 복원이 창 생성 전 동기 | 탭·펼침 경로를 창이 뜨기 전에 전부 연다 → 끊긴 네트워크 경로면 창 없이 대기. dir3: 창 먼저·활성 탭만·나머지는 첫 전환 시 | `dir2:win.rs:1447-1475` · `dir2-doc:audit/20260904-165351/02-settings.md:11` |
| PREFS-625 | 복원 순서 의존 | ① 클라우드 루트 동기는 패널 복원보다 먼저(가상 탭이 세션에 있음) ② 컬럼 폭은 DPI 반영(기본 폭 리셋) **이후**, 레이아웃(순서·표시) 먼저 → 폭 | `dir2:win.rs:1443-1446` · `:894-896` · `:7708-7718` |
| PREFS-626 | 언어 전환 시 컬럼 폭 초기화 | 컬럼 제목 재구성이 폭을 기본값으로 되돌림(알려진 한계) — dir3에서 폭 보존 필요 | `dir2:win.rs:5572` |
| PREFS-627 | 세션 dirty 범위 | 플래그는 탭/경로/보기 변경에만 서고 펼침·컬럼 폭/순서·분할 비율에는 서지 않음. 설정 창의 컬럼 편집도 settings만 저장하고 session은 저장하지 않음 | `dir2-doc:audit/20260904-165351/02-settings.md:16` · `dir2:win.rs:6583-6593` |
| PREFS-628 | 메뉴 토글도 즉시 영속 | 종료 저장에만 의존하면 비정상 종료 시 유실(언어 변경 유실 사고) | `dir2:win.rs:6837-6839` |
| PREFS-629 | 기본값 정책 차이 | dir2 테마 기본 = `dark`(DR-5) · nexa-sql 기본 = `system`. "정책은 dir2 계승"이므로 dir3 기본값은 dir2 값(§5-1 표)을 따른다 | `dir2:config.rs:254` · `sql:nsql-settings/src/lib.rs:728-733` |
| PREFS-630 | 고속 스크롤 표현 차이 | nexa-sql은 `scroll.fast_speed` 프리셋(선택) + hold/window 비노출, dir2는 step·max·window·hold·fade를 **전부 숫자로 노출**. dir2 구성 유지 | `dir2:prefs.rs:657-720` · `sql:nsql-settings/src/lib.rs:2543-2605` |
| PREFS-631 | 죽은 코드 정리 | `Kind::Text`(미사용) · 라디오 분기의 `F_TA_POS` 처리 · 숫자 분기의 글꼴 필드 · 필드 id 22 결번 — 이식하지 않는다 | `dir2:prefs.rs:141-143` · `:1536` · `:2465` · `:1700-1713` · `:236-238` |
| PREFS-632 | 언어 라디오 표기 | 발견 결과에는 자기 언어 표기(`한국어`·`日本語`)가 있으나 설정 창은 코드만 표시. dir3는 표기명 사용 권장(메뉴와 일치) | `dir2:prefs.rs:934-940` · `dir2:i18n.rs:122-127` · `dir2:win.rs:6355` |
| PREFS-633 | i18n 방식 차이 | nexa-sql은 컴파일 타임 `Msg` 열거(en·ko 2언어), dir2는 런타임 `.lang`(en·ko·ja + 사용자 언어팩). "자원 그대로 사용" 요구에 따라 **dir2의 `.lang` 방식 유지**가 자연스럽고, 설정 레지스트리의 라벨/설명은 `Msg` 대신 문자열 키로 둔다(결정 필요) | `dir2:i18n.rs:1-7` · `sql:nsql-i18n/src/lib.rs:20-28` |

---

## 7. 회귀 테스트 후보

자동화: **U** = 단위(순수 함수) · **H** = 헤드리스 UI(입력 사건 주입 + 상태/래스터 검증, nexa-sql 설정 창 테스트 방식 `sql:nexa-sql/src/prefs_win.rs:2233`) · **M** = 수동/실기.

| ID | 시나리오 | 기대 | 자동화 |
| --- | --- | --- | --- |
| PREFS-701 | 레지스트리 전 키 왕복 | 모든 키를 기본값이 아닌 값으로 설정 → 저장 → 재로드 시 동일(키 누락 0). dir2 `settings_roundtrip_and_lenient_parse` 이식 | U |
| PREFS-702 | 기본값 고정 | §5-1의 기본값 전부를 표 그대로 단언(dir2 정책 계승 확인) | U |
| PREFS-703 | 범위 클램프/거부 | 각 수치 키의 하한−1·상한+1·음수·초대형·`NaN`·빈 값 → 범위 안 값 또는 기본 유지 | U |
| PREFS-704 | 적대적 입력 | 빈 파일·BOM·`=`만 있는 줄·이진·1MB 한 줄·중복 키·CRLF → 패닉 없음·"마지막 유효 값" | U |
| PREFS-705 | 순서 문자열 | 기본 왕복 · 재배열/숨김 보존 · 구형(vis 생략) · 미지 토큰 제거 · 누락 자식의 정의 위치 삽입(3정의 모두) | U |
| PREFS-706 | 런처 부재/비움 | 키 부재 = 시드 대상 · count=0 = 빈 목록 유지 · 구분선 · 인자 안 구분 문자 보존 · 33번째 항목 무시 | U |
| PREFS-707 | 클라우드 연결 왕복 | 경로형·API형 구분 · 라벨 구분 문자 치환 · 경로/계정 모두 없는 행 무시 · client id/secret 왕복 | U |
| PREFS-708 | 구 키·구 파일 이행 | `transfer_close_secs=3` → 3000 · dir2 `settings.cfg` 가져오기 변환(0/1 → on/off, 위치 0..8 → 이름) | U |
| PREFS-709 | 원자적 저장 | 저장 후 임시 파일 0개 · 덮어쓰기 · 대상 잠금 시 실패하되 옛 내용 보존(Windows) · Unix 권한 0600 | U (OS별 `cfg`) |
| PREFS-710 | 설정 폴더 결정 | 쓰기 가능 후보 = 그대로 · 불가 = 사용자 폴더 · macOS 번들/Homebrew/시스템 프리픽스 경로 = exe 옆 건너뜀 · 환경변수 재지정 | U |
| PREFS-711 | 세션 왕복 | 탭 2개+펼침+잠금+고정+보기 모드+보기 플래그+컬럼 레이아웃/폭 왕복 · 빈 목록 생략 직렬화 후에도 인덱스 정렬 유지 | U |
| PREFS-712 | 세션 경로 특수 문자 | 경로에 공백·한글·파이프(`\|`)·개행·(Unix) 비UTF-8 바이트가 있어도 왕복 | U (OS별) |
| PREFS-713 | 실패 탭 건너뜀 후 속성 정합 | 3탭 중 가운데 경로가 사라진 세션 복원 → 남은 탭의 잠금/고정/모드/플래그가 **원래 탭의 것**(PREFS-620 회귀 방지) | U/H |
| PREFS-714 | 세션 저장 코얼레싱 | 변경 100회 연속 → 쓰기 1회 · 연속 변경이 길어도 최대 지연 안에 1회 · 종료 시 강제 flush · 내용 동일하면 쓰지 않음 | U |
| PREFS-715 | i18n 파서 규칙 | BOM·주석·메타 후행 주석·중복 키·이스케이프·파손 줄(dir2 테스트 이식) | U |
| PREFS-716 | 번역 키 파리티 | en ↔ ko ↔ ja 키 집합 일치 + **레지스트리의 모든 라벨/설명 키가 en에 존재** | U |
| PREFS-717 | 언어 병합·해석 | 사용자 오버라이드 키 단위 병합 · 신규 언어 발견 · `system` 해석(`ko-KR`→ko, 미보유→en) · OS별 로캘 문자열(`ko_KR.UTF-8`, `C`) | U |
| PREFS-718 | 트리 구성·가시성 | 15노드 순서/깊이 · 접힌 그룹의 하위 숨김 · 그룹 클릭 = 펼침 토글 | U/H |
| PREFS-719 | 검색 필터 | 토큰 AND · 대소문자 무시 · 라벨 매치/하위 상세 매치 · 조상 유지 · 그룹 라벨 매치 = 하위 전체 · 매치 수 · 전역 결과 제목과 "카테고리: 항목" 접두 · 비우면 일반 복귀 · leaf 0건이면 전체 | U/H |
| PREFS-720 | 카테고리별 항목·순서 | §2-4의 항목 종류와 순서가 그대로(스냅샷 비교) | U |
| PREFS-721 | 즉시 적용 | 체크 클릭 → 변경 사건 1회 + 값 저장 · 숫자 입력 후 포커스 이탈/Enter → 1회 · 입력 중(타이핑)에는 적용 안 됨 | H |
| PREFS-722 | 종속 비활성 | `fast_scroll` 끔 → 5항목 잠김 · hud 끔 → 3항목 잠김 · 다시 켜면 복구 · 잠긴 항목 값 보존 | H |
| PREFS-723 | 수정됨 표시 | 기본값과 다르면 표시, 기본값으로 되돌리면 사라짐(전 키) | H |
| PREFS-724 | 스크롤 중 편집 값 보존 | 숫자/글꼴 입력 중 휠 스크롤·리사이즈·페이지 전환 → 값 유실/공백화 없음(PREFS-601 회귀 방지) | H |
| PREFS-725 | 글꼴 쉼표 체인 선택 규칙 | 접두 입력 중 선택 = 조각 교체 · 완결 이름 뒤 선택 = `, ` 추가 · 빈 입력 = 그대로 · 드롭다운 열림 중 Enter = 목록 확정 | U/H |
| PREFS-726 | 위치 드롭다운 | 9칸 각각 선택 → 값 0..8/이름 일치 · 비활성 시 회색 | H |
| PREFS-727 | 순서 편집기 조작 | 그룹 이동 = 블록 통째 · 자식 = 그룹 안 · 경계에서 무동작 · Shift 범위(같은 부모만) · 잠금 key 체크 해제 거부 · 그룹 체크 해제 시 자식 상태 보존 · 조작마다 변경 사건 · Esc 동작 | U(모델)/H |
| PREFS-728 | 플러그인 체크 | 해제 → `plugins.disabled`에 id 추가·**즉시** 변경 사건 · 다른 페이지 조작이 목록을 지우지 않음 · 검색 중 미표시 · 빈 목록 안내 | H |
| PREFS-729 | 설정 적용 효과 | 테마/언어 전환 · 글꼴 슬롯 변경 후 렌더 반영 · 보기 필터가 양 패널 전 탭에 적용 · 툴바 순서 변경 즉시 반영 | H/M |
| PREFS-730 | 기동 복원 통합 | 설정+세션 파일을 준비해 기동 → 탭·활성·펼침·컬럼 폭·도크 비율·분할 비율이 저장값과 일치 · argv 경로가 있으면 세션 무시 | H/M |
| PREFS-731 | 손상 파일 기동 | settings/session이 쓰레기여도 창이 뜨고 기본값으로 동작, 다음 저장으로 정상화 | H |
| PREFS-732 | OS 3종 스모크 | 설정 폴더 위치 · 시스템 언어/테마 추종 · 기본 글꼴 해석이 Windows/macOS/Linux에서 각각 기대값 | M(CI 매트릭스 일부 U) |
