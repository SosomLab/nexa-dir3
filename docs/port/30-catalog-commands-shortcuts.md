# nexa-dir2 전수 카탈로그 — 메뉴 · 명령 ID · 단축키 · 마우스 제스처 (CMD)

> 목적: nexa-dir2(Windows 전용 · 0.22.0)의 **모든 메뉴 항목 · 명령 ID · 키보드 단축키 · 마우스 제스처**를 개체 기준으로 횡단 조사한 표. nexa-dir3(크로스플랫폼)로 옮길 때 "하나도 빠지지 않았는가"를 대조하는 체크리스트다.
> 이 문서의 `CMD-NNN` ID는 구현·교차 검증에서 그대로 쓴다. **번호는 절(節)별 대역**이라 연속이 아니다(결번 허용): §1 = 001~116 · §2 = 120~259 · §3 = 260~356 · §4 = 400~456 · §5 = 480~499.

## 0. 범위 · 조사 방법 · 표기

| 항목 | 내용 |
|---|---|
| 조사 대상 | `nexa-dir2/crates` 전체(nexa-app · nexa-gui · nexa-term · nexa-ops · nexa-tree · nexa-vfs · nexa-core) |
| 윤곽 잡기 | `const (IDM\|CMD\|ID\|IDC\|CTX\|BTN\|SEL\|WM_APP\|TIMER)_*` Grep(win.rs 59건 + 그 외 파일 전부) · `VK_*`/`WM_KEYDOWN`/`WM_SYSKEYDOWN`/`WM_CHAR`/`GetKeyState`/`MK_*`/`WM_*BUTTON*`/`WM_MOUSE*WHEEL`/`TrackPopupMenuEx`/`IsDialogMessageW` Grep · `InputEvent::`/`Key::`/`EditKey::` 매치 지점 Grep |
| 끝까지 읽은 구간 | `win.rs` 1~300(상수) · 376~820(메뉴/도구모음) · 2739~3420(컨텍스트 메뉴·DnD 훅) · 3716~3812 · 5100~5575(`run_command`) · 5931~6350 · 7059~9714(`wndproc` 전 메시지) / `nexa-gui/src/event.rs` 전부 · `widgets/rows.rs` 1690~2125(`on_event`) · `tabbar.rs` 1~310 · `pathbar.rs` 1~500 · `dock.rs` 560~816 · `menubar.rs` 1~330 · `chrome.rs` 1~260 · `edit.rs` 180~220 · `overlaybar.rs` 195~250 / `panel.rs` 94~104 · 529~555 · 718~762 · 983~1024 · 1346~1375 · 1440~1565 / `shellmenu.rs` 1~130 + 함수 윤곽 / `dnd.rs` 150~290 / `previewwnd.rs` 325~400 · 640~860 / `archivewnd.rs` 270~305 / `pwprompt.rs` 205~240 / `ordereditor.rs` 300~427 / `prefs.rs` 845~875 · 2290~2320 · 3015~3060 / `bulkrename.rs` 46~100 · 1355~1425 · 1596~1625 · 1785~2010 · 2222~2252 / `ctl/grid.rs` 560~860 · `ctl/ordertree.rs` 1~58 · 640~700 · 그 외 `ctl/*.rs`는 키 처리 arm Grep / `config.rs` 1028~1090 / `lang/ko.lang` · `lang/en.lang`(메뉴·컨텍스트 키) |
| 읽지 않은 것 | 설정 창(`prefs.rs`) 내부 필드별 동작 · 일괄 이름변경 카드 폼 내부 · `ctldemo.rs` 본문 — **컨트롤 ID와 키/마우스 진입점만** 수록했다. 필드 단위 상세는 각 담당 인벤토리 문서 소관("추정" 표기 항목 참조) |

표기 약속: `win.rs:N` = `nexa-dir2/crates/nexa-app/src/win.rs:N`. `panel.rs` · `shellmenu.rs` · `dnd.rs` · `previewwnd.rs` · `archivewnd.rs` · `pwprompt.rs` · `ordereditor.rs` · `prefs.rs` · `bulkrename.rs` · `dialog.rs` · `about.rs` · `config.rs` = `nexa-dir2/crates/nexa-app/src/` 아래. `ctl/x.rs` = `nexa-dir2/crates/nexa-app/src/ctl/x.rs`. `gui/x.rs` = `nexa-dir2/crates/nexa-gui/src/x.rs`. 표시 문자열은 `ko.lang`(괄호 = `en.lang`) 값이며 근거는 `nexa-dir2/crates/nexa-app/lang/ko.lang` · `en.lang`.

이식 분류: **N** = 플랫폼 중립 / **A** = nexa-ui 컨트롤로 교체 / **P** = OS별 분기 필요 / **W** = Windows 전용(타 OS 대체 구현) / **X** = 이식 제외 후보(죽은 코드·개발 전용).

핵심 구조 요약(이 문서 전체의 전제):

- 메뉴바 · 도구 모음 · 퀵 런처는 **같은 u32 명령 ID**를 `run_command`(win.rs:5108) 하나로 실행한다. 메뉴/도구모음 클릭은 `take_command()` 수거 후 `run_command` 호출(win.rs:7973-7996).
- **가속기 표(ACCEL)는 없다.** 단축키는 `wndproc`의 `WM_KEYDOWN`(win.rs:8694) · `WM_SYSKEYDOWN`(win.rs:8905) · `WM_CHAR`(win.rs:9331) 안의 if 사슬이다. 메뉴의 단축키 문구는 **표시 전용 문자열**(win.rs:412-465)이라 실제 처리와 따로 관리된다.
- 팝업 메뉴는 전부 **OS 네이티브 `TrackPopupMenuEx`**(탭 · 편집 · 도구모음/헤더 · 내 PC 클라우드 · 미리보기 창 · 설정 위치 팝업) 또는 **셸 `IContextMenu` 호스팅**(행/배경). 메뉴바 드롭다운만 자체 그리기(gui/widgets/menubar.rs).
- 메뉴바 항목은 **비활성 표시를 지원하지 않는다**(win.rs:403, win.rs:454, win.rs:523). 조건 불충족은 실행 시점에 상태 줄 안내로 처리한다.
- 키 입력 우선순위(win.rs:8706-8900): ① 경로바 편집 → ② 도크 터미널 포커스 → ③ 인라인 이름변경 → ④ Esc 계열 취소 → ⑤ 전역/목록 단축키 → ⑥ 목록 네비 키.

---

## 1. 메뉴 트리

### 1-1. 메뉴바 (자체 그리기 드롭다운 · `build_menus` win.rs:380-484)

메뉴바 자체 조작: 제목 클릭 = 열기/닫기 토글 · 열린 상태에서 다른 제목 hover = 전환 · 하위 메뉴 보유 항목 hover/클릭 = 오른쪽 플라이아웃(1단계만) · 외부 클릭/Esc = 닫기(gui/widgets/menubar.rs:229-302, win.rs:8811). **키보드 접근(Alt 니모닉 · F10 · 방향키 이동) 없음.**

| ID | 경로 | 표시 문자열 ko (en) | 단축키 표기 | 명령 상수 | 체크/라디오 | 활성 조건 · 실행 시 가드 | 근거 |
|---|---|---|---|---|---|---|---|
| CMD-001 | 파일 | 새 탭 (New Tab) | Ctrl+T | `CMD_NEW_TAB` | — | 항상. 활성 패널에 현재 경로 복제 탭 | win.rs:440, win.rs:5113 |
| CMD-002 | 파일 | 탭 닫기 (Close Tab) | Ctrl+W | `CMD_CLOSE_TAB` | — | 탭 1개뿐이거나 잠긴 탭이면 무동작 | win.rs:441, win.rs:5114, panel.rs:720-723 |
| CMD-003 | 파일 | 새 폴더 만들기 (New Folder) | Ctrl+Shift+N | `CMD_NEW_FOLDER` | — | 생성 후 그 행 인라인 이름변경 진입. 클라우드 경로 = API 폴더 생성 | win.rs:443, win.rs:5175, win.rs:4068-4086 |
| CMD-004 | 파일 | 새 파일 만들기 (New File) | (없음) | `CMD_NEW_FILE` | — | 클라우드 경로에서는 거부(`cloud.err.foldersOnly` 안내) | win.rs:444, win.rs:4072-4075 |
| CMD-005 | 파일 | 설정... (Preferences...) | Ctrl+, | `CMD_PREFS` | — | 모달 설정 창(지연 실행) | win.rs:446, win.rs:5357 |
| CMD-006 | 파일 | 종료 (Exit) | (없음) | `CMD_EXIT` | — | `WM_CLOSE` 게시 = 창 닫기와 같은 저장 경로 | win.rs:448, win.rs:5120 |
| CMD-007 | 편집 | 실행 취소 (Undo) | Ctrl+Z | `CMD_UNDO` | — | 경로바 편집/이름변경 중이면 **필드 내용 복귀**, 아니면 파일 작업 undo. 이력 없으면 `undo.none` 안내 | win.rs:455, win.rs:5373-5384, win.rs:3514-3530 |
| CMD-008 | 편집 | 다시 실행 (Redo) | Ctrl+Y | `CMD_REDO` | — | 이력 없으면 `redo.none` 안내 | win.rs:456, win.rs:5373 |
| CMD-009 | 편집 | 잘라내기 (Cut) | Ctrl+X | `CMD_CUT` | — | 포커스 문맥 디스패치(`do_clip`) | win.rs:459, win.rs:5385 |
| CMD-010 | 편집 | 복사 (Copy) | Ctrl+C | `CMD_COPY` | — | 포커스 문맥 디스패치 | win.rs:460 |
| CMD-011 | 편집 | 붙여넣기 (Paste) | Ctrl+V | `CMD_PASTE` | — | 포커스 문맥 디스패치 | win.rs:461 |
| CMD-012 | 편집 | 전체 선택 (Select All) | Ctrl+A | `CMD_SELECT_ALL` | — | 포커스 문맥 디스패치 | win.rs:463 |
| CMD-013 | 편집 | 일괄 이름 변경... (Bulk Rename...) | Ctrl+Shift+R | `CMD_BULK_RENAME` | — | 선택(없으면 캐럿) 0개면 `bulk.noSelection` 안내 | win.rs:465, win.rs:6424-6441 |
| CMD-014 | 보기 | 트리 보기 (Tree View) | (없음) | `CMD_VIEW_TREE` | 라디오 `view_mode=="tree"` | **탭별** 적용(활성 패널의 활성 탭) | win.rs:398, win.rs:5178 |
| CMD-015 | 보기 | 플랫 보기 (Flat View) | (없음) | `CMD_VIEW_FLAT` | 라디오 | 탭별 | win.rs:399 |
| CMD-016 | 보기 | 타일 보기 (Tile View) | (없음) | `CMD_VIEW_TILES` | 라디오 | 탭별 | win.rs:400 |
| CMD-017 | 보기 | 듀얼 파일 패널 (Dual File Panels) | (없음) | `CMD_PANEL_DUAL` | 라디오 `panel_mode=="dual"` | 항상 | win.rs:404, win.rs:5210 |
| CMD-018 | 보기 | 싱글 파일 패널 (Single File Panel) | (없음) | `CMD_PANEL_SINGLE` | 라디오 | 싱글 진입 시 활성 = 좌 강제 · 우 패널 상태 보존 | win.rs:405, win.rs:5222 |
| CMD-019 | 보기 | 듀얼 정보 패널 (Dual Info Panes) | (없음) | `CMD_INFO_DUAL` | 라디오 `!info_single_eff`(효과 기준) | **싱글 패널에서는 변경 불가** → `status.infoLocked` 안내 | win.rs:407, win.rs:5259-5264 |
| CMD-020 | 보기 | 싱글 정보 패널 (Single Info Pane) | (없음) | `CMD_INFO_SINGLE` | 라디오 | 위와 동일 | win.rs:408 |
| CMD-021 | 보기 | 컬럼 너비 동기화 (Synchronize Column Widths) | (없음) | `CMD_COLW_SYNC` | 체크 `col_width_sync` | 켜는 순간 활성 패널 컬럼 구성을 반대 패널에 적용 | win.rs:410, win.rs:5193-5209 |
| CMD-022 | 보기 | 숨김 파일 표시 (Show Hidden Files) | Ctrl+H | `CMD_TOGGLE_HIDDEN` | 체크 `show_hidden` | 전파 폭 = `view_scope`(tab/panel/global) | win.rs:412, win.rs:5127-5164 |
| CMD-023 | 보기 | 점 파일 표시 (Show Dot Files) | Ctrl+. | `CMD_TOGGLE_DOTFILES` | 체크 `show_dotfiles` | 위와 동일 | win.rs:413 |
| CMD-024 | 보기 | 하단 도크 (Bottom Dock) | Ctrl+` | `CMD_TOGGLE_DOCK` | 체크 `dock` | 양 패널 동시 토글 | win.rs:414, win.rs:5304-5330 |
| CMD-025 | 보기 | 퀵 런처 바 (Quick Launcher Bar) | (없음) | `CMD_TOGGLE_LAUNCHER` | 체크 `launcher` | 항상 | win.rs:415, win.rs:5331 |
| CMD-026 | 보기 | 항상 맨 위에 표시 (Always on Top) | (없음) | `CMD_TOGGLE_TOPMOST` | 체크 `always_on_top` | 항상 | win.rs:417, win.rs:5166 |
| CMD-027 | 보기 | 새로 고침 (Refresh) | F5 | `CMD_REFRESH` | — | 클라우드/내 PC면 캐시 무효화 후 재조회 | win.rs:419, win.rs:5396-5405 |
| CMD-028 | 보기 | 테마: 시스템 (Theme: System) | F6 | `CMD_THEME_SYSTEM` | 라디오 `ThemeMode::System` | 항상 | win.rs:422, win.rs:5409 |
| CMD-029 | 보기 | 테마: 라이트 (Theme: Light) | F6 | `CMD_THEME_LIGHT` | 라디오 | 항상 | win.rs:424 |
| CMD-030 | 보기 | 테마: 다크 (Theme: Dark) | F6 | `CMD_THEME_DARK` | 라디오 | 항상 | win.rs:426 |
| CMD-031 | 보기 | 언어: 시스템 (Language: System) | (없음) | `CMD_LANG_SYSTEM` | 라디오 `lang_setting=="system"` | 항상 · 재시작 없이 전환 | win.rs:429, win.rs:5418 |
| CMD-032 | 보기 | (동적) 발견된 언어 표기명 × N | (없음) | `CMD_LANG_BASE + i` | 라디오 `lang_setting==code` | `i < st.langs.len()` | win.rs:432-435, win.rs:5423 |
| CMD-033 | 클라우드 | (동적) 연결 라벨 × N — 하위 메뉴 부모 | — | id 0(발화 없음) | — | 연결 수 상한 32(`CLOUD_MAX`) | win.rs:494-511 |
| CMD-034 | 클라우드 ▸ 연결 | 바로 가기 (Go to Folder) | (없음) | `CMD_CLOUD_GOTO_BASE + i` | — | 동기화 폴더가 사라졌으면 `cloud.gone` 안내 | win.rs:497, win.rs:5435-5455 |
| CMD-035 | 클라우드 ▸ 연결 | 온라인 보기 (Open Online) | (없음) | `CMD_CLOUD_WEB_BASE + i` | — | 서비스 URL이 비면 무동작 | win.rs:498, win.rs:5456-5474 |
| CMD-036 | 클라우드 ▸ 연결 | URL 복사 (Copy URL) | (없음) | `CMD_CLOUD_COPYURL_BASE + i` | — | 복사 후 `cloud.urlCopied` 안내 | win.rs:499, win.rs:5475-5488 |
| CMD-037 | 클라우드 ▸ 연결 | 연결 해제 (Disconnect) — API 연결 / 링크 해제 (Remove Link) — 동기화 폴더 | (없음) | `CMD_CLOUD_DISC_BASE + i` | — | API 연결이면 토큰도 폐기 | win.rs:502-510, win.rs:5489-5515 |
| CMD-038 | 클라우드 | 링크 추가하기… (Add Link…) | (없음) | 후보 0개 = `CMD_CLOUD_ADD_NONE` 단독 항목 / 후보 있음 = 하위 메뉴 부모(id 0) | — | 후보 0개 클릭 = `cloud.addNone` 안내 | win.rs:516-527, win.rs:5429 |
| CMD-039 | 클라우드 ▸ 링크 추가 | (동적) 감지된 미연결 후보 라벨 × N | (없음) | `CMD_CLOUD_ADD_BASE + i` | — | 연결 수 < 32일 때만 추가 | win.rs:520, win.rs:5527-5549 |
| CMD-040 | 클라우드 | 클라우드 직접 연결 (Connect Cloud) — 하위 메뉴 부모 | — | id 0 | — | 항상(앞에 구분선) | win.rs:535-536 |
| CMD-041 | 클라우드 ▸ 직접 연결 | (동적) `oauth::SERVICES[i].display` × 서비스 수 | (없음) | `CMD_CLOUD_OAUTH_BASE + i` | — | client_id 미설정이면 안내 대화상자 | win.rs:530-534, win.rs:5516-5526, win.rs:5647-5657 |
| CMD-042 | 도움말 | Nexa Dir 정보 (About Nexa Dir) | (없음) | `CMD_ABOUT` | — | 모달 About 창(지연 실행) | win.rs:481, win.rs:5369 |

메뉴 구분선 위치(순서 보존용): 파일 = 탭 2종 │ 새로 만들기 2종 │ 설정 │ 종료(win.rs:439-449). 편집 = Undo/Redo │ 클립보드 3종 │ 전체 선택 │ 일괄 이름변경(win.rs:453-466). 보기 = 보기 모드 3 │ 패널/정보 4 + 컬럼 동기 │ 표시 토글 5 │ 새로 고침 │ 테마 3 │ 언어(win.rs:396-431). 클라우드 = 연결들 │ 링크 추가 │ 직접 연결(win.rs:513-536).

### 1-2. 도구 모음 (`build_toolbar` win.rs:675-792)

순서·표시 = 설정 `toolbar_order`(블록 정의 `TOOLBAR_BLOCKS` config.rs:1036-1044). 블록 사이 구분선 자동 삽입. 숨긴 블록/항목은 제외(win.rs:762-790).

| ID | 블록/항목 키 | 폴백 글리프 · 아이콘 | 툴팁 | 명령 상수 | 토글 표시 | 활성 조건 | 근거 |
|---|---|---|---|---|---|---|---|
| CMD-043 | `refresh` | ⟳ · `emb:refresh` | 새로 고침 | `CMD_REFRESH` | — | 항상 | win.rs:737-739 |
| CMD-044 | `panel/toggle` | ▌▐ · `emb:panel-toggle` | 현재 모드명(듀얼/싱글 파일 패널) | `CMD_PANEL_TOGGLE` | 켜짐 = 듀얼 | 항상 | win.rs:693-700 |
| CMD-045 | `panel/dock` | ▂ · `emb:dock` | 하단 도크 | `CMD_TOGGLE_DOCK` | 켜짐 = 도크 표시 | 항상 | win.rs:701-704 |
| CMD-046 | `panel/info` | ⓘ · `emb:info-toggle` | 현재 모드명(듀얼/싱글 정보 패널) | `CMD_INFO_TOGGLE` | 켜짐 = 듀얼 정보 | **`panel_mode=="dual" && dock`** 일 때만 활성 | win.rs:710-719 |
| CMD-047 | `panel/colsync` | ⇔ · `emb:colsync` | 컬럼 너비 동기화 | `CMD_COLW_SYNC` | 켜짐 = 동기 | **`panel_mode=="dual"`** 일 때만 활성 | win.rs:720-724 |
| CMD-048 | `panel/ontop` | 📌 · `emb:always-on-top` | 항상 맨 위에 표시 | `CMD_TOGGLE_TOPMOST` | 켜짐 = 최상위 | 항상 | win.rs:706-709 |
| CMD-049 | `view/tree` | ├─ · `emb:view-tree` | 트리 보기 | `CMD_VIEW_TREE` | 라디오식 | 항상 | win.rs:725-728 |
| CMD-050 | `view/flat` | ☰ · `emb:view-flat` | 플랫 보기 | `CMD_VIEW_FLAT` | 라디오식 | 항상 | win.rs:729-732 |
| CMD-051 | `view/tiles` | ▦ · `emb:view-tiles` | 타일 보기 | `CMD_VIEW_TILES` | 라디오식 | 항상 | win.rs:733-736 |
| CMD-052 | `show/hidden` | 👁 · `emb:hidden` | "숨김 파일 표시 — 적용 범위" | `CMD_TOGGLE_HIDDEN` | 켜짐 = 표시 | 항상 | win.rs:745-748, win.rs:668-670 |
| CMD-053 | `show/dot` | … · `emb:dotfiles` | "점 파일 표시 — 적용 범위" | `CMD_TOGGLE_DOTFILES` | 켜짐 = 표시 | 항상 | win.rs:749-752 |
| CMD-054 | `show/foldersfirst` | ▲ · `emb:folders-first` | "폴더 우선 정렬 — 적용 범위" | `CMD_TOGGLE_FOLDERS_FIRST` | 켜짐 = 폴더 우선 | 항상. **메뉴바에는 없는 명령**(도구모음·설정 전용) | win.rs:754-757 |
| CMD-055 | `settings` | U+E713 · `emb:settings` | "설정"(말줄임표 제거) | `CMD_PREFS` | — | 항상 | win.rs:740-742 |

### 1-3. 퀵 런처 바 · 패널 네비 바 · 탭 바 · 도크 스트립

| ID | 위치 | 항목 | 명령/동작 | 활성 조건 | 근거 |
|---|---|---|---|---|---|
| CMD-056 | 퀵 런처 바 | (동적) 런처 항목 × N(상한 32) — exe 셸 아이콘 16px 정사각 버튼, `launcherN=-` 는 구분선 | `CMD_LAUNCHER_BASE + i` → `launcher::launch`(`%path%` = 활성 패널 현재 폴더). 결과는 `launcher.ran`/`launcher.failed` 안내 | 바 표시 = `CMD_TOGGLE_LAUNCHER` | win.rs:800-813, win.rs:5340-5356 |
| CMD-057 | 패널 네비 바 | [홈] U+EA8A | `BTN_HOME` → `nav_home`(내 PC로 한 번에) | 내 PC에서는 비활성 | panel.rs:1555-1565, panel.rs:1538, panel.rs:554 |
| CMD-058 | 패널 네비 바 | [←] U+E72B | `BTN_BACK` → `nav_back` | 히스토리 뒤로 가능할 때만 | panel.rs:1535, panel.rs:551 |
| CMD-059 | 패널 네비 바 | [→] U+E72A | `BTN_FORWARD` → `nav_forward` | 히스토리 앞으로 가능할 때만 | panel.rs:1536, panel.rs:552 |
| CMD-060 | 패널 네비 바 | [↑] U+E74A | `BTN_UP` → `nav_up`(드라이브 루트 → 내 PC) | 내 PC에서는 비활성 | panel.rs:1537, panel.rs:553 |
| CMD-061 | 탭 바 | [+] 버튼 | `TabAction::New` → `new_tab` | 항상 | gui/widgets/tabbar.rs:233-236, panel.rs:1513 |
| CMD-062 | 탭 바 | 탭 오른쪽 × 존(폭 = pad_x × 3) | `TabAction::Close(i)` | 탭이 2개 이상이고 잠기지 않은 탭만(아니면 전환으로 처리) | gui/widgets/tabbar.rs:183, gui/widgets/tabbar.rs:220-227 |
| CMD-105 | 도크 스트립 | 종류 라벨 "정보" | `active_kind = 0` | 도크 표시 중 | gui/widgets/dock.rs:648-660, win.rs:2475 |
| CMD-106 | 도크 스트립 | 종류 라벨 "미리보기" | `active_kind = 1` | 도크 표시 중 | gui/widgets/dock.rs:648-660 |
| CMD-107 | 도크 스트립 | 종류 라벨 "터미널" | `active_kind = 2` + 터미널 영역 클릭 시 키 포커스 | 도크 표시 중 | gui/widgets/dock.rs:648-660, win.rs:8104-8114 |
| CMD-108 | 도크 스트립 | [→] 폴더로 이동(터미널 라벨 옆 부착) | 터미널로 전환 + `cd "<현재 폴더>"\r` 전송. 종료 상태면 재시작(cwd = 현재 폴더) | 종류가 2개 이상일 때 표시 | gui/widgets/dock.rs:577-594, gui/widgets/dock.rs:638-647, win.rs:8082-8099 |
| CMD-109 | 도크 내용 우상단 | ↗ "크게" 버튼 | 누른 채 버튼 **안에서 뗄 때만** 발화 → `open_preview_window`(독립 미리보기 창/압축 그리드 창) | `popout_on` 일 때 | gui/widgets/dock.rs:661-667, gui/widgets/dock.rs:799-806, win.rs:8563-8570 |

### 1-4. 탭 우클릭 메뉴 (네이티브 팝업 · `show_tab_menu` win.rs:7115-7195)

| ID | 순서 | 표시 문자열 ko (en) | 지역 ID | 동작 | 활성 조건 | 근거 |
|---|---|---|---|---|---|---|
| CMD-063 | 1 | 탭 잠금 / 탭 잠금 해제 (Lock tab / Unlock tab) | `CMD_LOCK`=1 | `toggle_tab_lock` | 항상 | win.rs:7143-7151, win.rs:7178 |
| CMD-064 | 2 | 탭 고정 / 탭 고정 해제 (Pin tab / Unpin tab) | `CMD_PIN`=4 | `toggle_tab_pin` | 항상 | win.rs:7152-7160, win.rs:7179 |
| CMD-065 | 3 | 탭 복제 (Duplicate tab) | `CMD_DUP`=2 | `duplicate_tab` | 항상 | win.rs:7161, win.rs:7180-7183 |
| CMD-066 | 4 | 새 탭 (New tab) | `CMD_NEW`=5 | `new_tab`(Ctrl+T · [+] 동일 경로) | 항상 | win.rs:7162, win.rs:7184-7188 |
| CMD-067 | 5 | 탭 닫기 (Close tab) | `CMD_CLOSE`=3 | `close_tab` | **잠기지 않았고 탭 수 > 1** 일 때만(회색 처리) | win.rs:7163, win.rs:7189 |

### 1-5. 행(파일/폴더) 컨텍스트 메뉴 (셸 `IContextMenu` 호스팅 + 고유 항목 병합)

진입: 행 우클릭 뗌(win.rs:8278) · Apps 키(win.rs:8876) · Shift+F10(win.rs:8910). 대상 = 선택(없으면 캐럿)을 **캐럿 행과 같은 부모 폴더**로 축소(win.rs:2781-2801). Shift를 누른 채 열면 확장 동사(`CMF_EXTENDEDVERBS`, win.rs:3019, shellmenu.rs:515-518). 고유 항목의 순서/표시 = 설정 `ctx_menu_order` 의 `row` 블록(config.rs:1078-1082). 메뉴 구축은 전용 메뉴 스레드에서 선행 구축(win.rs:3037-3052, win.rs:3144-3178).

| ID | 항목 | 표시 문자열 ko (en) | ID/동사 | 동작 | 표시·활성 조건 | 근거 |
|---|---|---|---|---|---|---|
| CMD-068 | 셸 제공 항목 전체 | OS·설치 프로그램이 결정(열기 · 연결 프로그램 · 보내기 · 7-Zip · Git · 속성 등) | 대역 1~0x6FFF | 셸 `InvokeCommand` 후 양 패널 재로드(`Outcome::Shell`) | 셸이 결정 | shellmenu.rs:36-38, shellmenu.rs:679-704, win.rs:3083 |
| CMD-069 | 가로채기: delete | 셸 "삭제" | 동사 `delete` | 앱 경로 `do_delete`(undo 기록). Shift로 연 메뉴면 완전 삭제 | 셸이 표시 | win.rs:3020, win.rs:3086-3089 |
| CMD-070 | 가로채기: rename | 셸 "이름 바꾸기" | 동사 `rename` | 인라인 이름변경 진입 | 셸이 표시 | win.rs:3090-3092 |
| CMD-071 | 가로채기: copy | 셸 "복사" | 동사 `copy` | **교차 폴더 전체 선택**을 앱 클립보드(파일 목록)로 | 셸이 표시 | win.rs:3093-3104 |
| CMD-072 | 가로채기: cut | 셸 "잘라내기" | 동사 `cut` | 위와 같음(이동) | 셸이 표시 | win.rs:3093-3104 |
| CMD-073 | 제자리 대체: 경로 복사 | 경로 복사 (Copy as path) | 동사 `copyaspath` → `CTX_COPY_PATH` | 전체 선택(표시 순서) 경로를 `\r\n` 구분 텍스트로 복사 | 셸이 `copyaspath` 를 낼 때(보통 Shift 확장 메뉴 — 추정) 그 자리를 대체 | win.rs:3024, win.rs:3105-3114, shellmenu.rs:527-557 |
| CMD-074 | 고유: 이름 복사 | 이름 복사 (Copy as name) | `CTX_COPY_NAME` | 파일/폴더 **이름만** `\r\n` 구분 텍스트로 복사 | `copyName` 표시 설정 켜짐. "경로 복사" 바로 아래 고정(`after_id`), 없으면 하단 고유 섹션 | win.rs:2998-3003, win.rs:3115-3124 |
| CMD-075 | 고유: 완전 삭제 | 완전 삭제 (Delete Permanently) | `CTX_DELETE_PERMANENT` | `do_delete(permanent=true)` — 확인 대화상자 | `deletePermanent` 표시 설정 켜짐 | win.rs:2990-2995, win.rs:3125 |
| CMD-076 | 고유: 폴더에 붙여넣기 | 폴더에 붙여넣기 (Paste into Folder) | `CTX_PASTE_INTO` | 대상 폴더 안으로 전송 시작(잘라내기면 클립보드 비움) · 가상 파일 폴백 | **대상 1개이고 폴더 + 클립보드에 파일 있음** 일 때만 항목 생성 | win.rs:2959, win.rs:3004-3011, win.rs:3126-3137 |
| CMD-077 | 고유: 새로 만들기 ▸ | 새로 만들기 (New) — 하위는 셸 ShellNew 템플릿(OS 로케일) | 대역 0x7000~0x7FFF | 생성 감지 시 캐럿 이동 + 인라인 이름변경(`Outcome::Created`) | **대상 1개** 일 때만(파일 = 부모 폴더 · 폴더 = 자신). `new` 표시 설정 켜짐 | win.rs:2976-2989, shellmenu.rs:39-41, shellmenu.rs:664-677, win.rs:3085 |

### 1-6. 빈 본문(폴더 배경) 컨텍스트 메뉴 (`show_background_context_menu` win.rs:2817-2912)

| ID | 항목 | 표시 문자열 ko (en) | ID/동사 | 동작 | 표시·활성 조건 | 근거 |
|---|---|---|---|---|---|---|
| CMD-078 | 셸 배경 항목 전체 | 셸이 결정(보기 · 정렬 · 새로 만들기 ▸ · 속성 등) | 대역 1~0x6FFF | 셸 실행 후 재로드. 새 항목 생성 감지 시 인라인 이름변경 | 실경로 폴더일 때 | win.rs:2880-2885, shellmenu.rs:401-411 |
| CMD-079 | 가로채기: paste | 셸 "붙여넣기" | 동사 `paste` | OS 클립보드 파일 목록 → 전송 엔진(undo 포함) · 없으면 가상 파일 폴백 | 셸이 표시 | win.rs:2880, win.rs:2886-2896 |
| CMD-080 | 고유: 붙여넣기 | 붙여넣기 (Paste) | `CTX_PASTE_BG` | 위와 같은 경로 | `paste` 표시 설정 켜짐. **클립보드에 파일이 있을 때만 활성**(없으면 회색) | win.rs:2854-2859, win.rs:2897-2907 |
| CMD-081 | 고유: 실행 취소 | "실행 취소: {설명}" 또는 "실행 취소" (Undo: {0}) | `CTX_UNDO` | `do_undo_redo(false)` | `can_undo()` 일 때만 활성 | win.rs:2830-2833, win.rs:2860-2865, win.rs:2908 |
| CMD-082 | 고유: 다시 실행 | "다시 실행: {설명}" 또는 "다시 실행" (Redo: {0}) | `CTX_REDO` | `do_undo_redo(true)` | `can_redo()` 일 때만 활성 | win.rs:2834-2837, win.rs:2866-2871, win.rs:2909 |

### 1-7. 내 PC 배경 메뉴 (네이티브 팝업 · `show_mypc_cloud_menu` win.rs:3205-3249)

활성 패널이 가상 최상위(내 PC)일 때 배경 우클릭은 셸 메뉴 대신 이 메뉴가 뜬다(win.rs:2821-2826).

| ID | 항목 | 표시 문자열 | 지역 ID | 동작 | 조건 | 근거 |
|---|---|---|---|---|---|---|
| CMD-083 | 감지된 미연결 후보 × N | "{라벨} 링크" (Link {0}) | `1 + i` | `CMD_CLOUD_ADD_BASE + i` 실행 | 상한 32 | win.rs:3219-3222, win.rs:3243-3247 |
| CMD-084 | 이미 연결된 항목 × N | 연결 라벨(체크 표시) | 0 | 없음(정보성) | 회색 + 체크 | win.rs:3223-3226 |
| CMD-085 | 안내 1줄 | "감지된 클라우드 동기화 클라이언트가 없습니다" | 0 | 없음 | 후보·연결이 모두 없을 때만 · 회색 | win.rs:3227-3230 |

### 1-8. 텍스트 편집 컨텍스트 메뉴 (네이티브 팝업 · `show_edit_popup` win.rs:7496-7620)

대상 판정 `edit_menu_target_at`(win.rs:7472-7490). 경로바를 우클릭해 **편집에 막 진입한 그 클릭**에서는 메뉴를 띄우지 않는다(win.rs:8209, win.rs:8251-8253).

| ID | 대상 | 표시 문자열 ko (en) | 지역 ID | 동작 | 활성 조건 | 근거 |
|---|---|---|---|---|---|---|
| CMD-086 | 경로바 편집 필드 · 이름변경 필드 | 실행 취소 (Undo) | `ID_UNDO`=1 | 필드 내용 복귀(단일 단계) | `can_undo` | win.rs:7523 |
| CMD-087 | 〃 | 잘라내기 (Cut) | `ID_CUT`=2 | 선택 텍스트 잘라내기 | 선택 있음 | win.rs:7525 |
| CMD-088 | 〃 | 복사 (Copy) | `ID_COPY`=3 | 선택 텍스트 복사 | 선택 있음 | win.rs:7526 |
| CMD-089 | 〃 | 붙여넣기 (Paste) | `ID_PASTE`=4 | 클립보드 **첫 줄만**·제어 문자 제거 후 삽입 | 클립보드에 텍스트 있음 | win.rs:7527, win.rs:7623-7633 |
| CMD-090 | 〃 | 삭제 (Delete) | `ID_DELETE`=5 | 선택 구간 삭제(파일 삭제와 무관) | 선택 있음 | win.rs:7528 |
| CMD-091 | 〃 | 전체 선택 (Select All) | `ID_SELECT_ALL`=6 | 필드 전체 선택 | 필드가 비어 있지 않음 | win.rs:7530 |
| CMD-092 | 도크 정보/미리보기 텍스트 | 복사 (Copy) | `ID_COPY`=3 | 평문 + 모노 RTF 동시 게시 | 선택 텍스트 있음 | win.rs:7536, win.rs:7593-7597 |
| CMD-093 | 〃 | 전체 선택 (Select All) | `ID_SELECT_ALL`=6 | 도크 텍스트 전체 선택 | 텍스트 선택 가능(이미지 미리보기 아님) | win.rs:7538, win.rs:7598-7600 |
| CMD-094 | 도크 터미널 | 복사 (Copy) | `ID_COPY`=3 | 평문 + HTML + RTF(설정 `term_copy_format`) 후 선택 해제 | 터미널 선택 있음 | win.rs:7544, win.rs:7396-7429 |
| CMD-095 | 〃 | 붙여넣기 (Paste) | `ID_PASTE`=4 | 줄바꿈을 CR로 바꿔 셸 stdin에 전달 | 클립보드에 텍스트 있음 | win.rs:7545, win.rs:7433-7443 |
| CMD-096 | 〃 | 전체 선택 (Select All) | `ID_SELECT_ALL`=6 | 스크롤백 첫 줄 ~ 화면 끝 선택 | 항상 | win.rs:7547, win.rs:7446-7456 |

### 1-9. 도구 모음 빈 영역 · 컬럼 헤더 우클릭 팝업 (네이티브 팝업 · `show_bar_popup` win.rs:6264-6303)

| ID | 위치 | 표시 문자열 | 지역 ID | 동작 | 근거 |
|---|---|---|---|---|---|
| CMD-097 | 도구 모음 빈 영역(버튼 아닌 곳) | "도구 모음 순서..." (`pref.toolbarOrder` + "...") | 1 | `WM_APP_EDIT_TOOLBAR` → 순서 편집 창 | win.rs:6271-6272, win.rs:6292-6294, win.rs:8165-8170 |
| CMD-098 | 〃 | "설정..." (`menu.file.prefs`) | 2 | `WM_APP_PREFS` → 설정 창 | win.rs:6273-6274, win.rs:6295-6297 |
| CMD-099 | 파일 목록 컬럼 헤더 | "파일 컬럼..." (`pref.colLayout` + "...") | 1 | `WM_APP_EDIT_COLS` → 컬럼 표시/순서 편집 창(활성 패널 대상) | win.rs:6276-6277, win.rs:6298-6300, win.rs:8171-8176 |

### 1-10. 하위 창의 메뉴 · 대화상자 버튼

| ID | 창 | 항목 | 지역 ID | 동작 · 조건 | 근거 |
|---|---|---|---|---|---|
| CMD-100 | 독립 미리보기 창 우클릭 | 복사 (Copy) | `ID_COPY`=1 | 선택 텍스트를 rich(평문 + RTF) 복사 · 선택 있을 때만 활성 | previewwnd.rs:341, previewwnd.rs:358, previewwnd.rs:372-377 |
| CMD-101 | 〃 | 전체 선택 (Select All) | `ID_SELECT_ALL`=2 | 텍스트 있을 때만 활성 | previewwnd.rs:342, previewwnd.rs:360, previewwnd.rs:378-381 |
| CMD-102 | 일괄 이름변경 창 `…` 메뉴 | (동적) 저장된 프리셋 이름 × N(상한 64) | 인덱스 0..n | 프리셋 불러오기(카드 스택 재구성) | bulkrename.rs:1377-1391, bulkrename.rs:1905-1920 |
| CMD-103 | 〃 | 이름 변경 순서 저장… (Save Renaming Sequence…) | n + 구분선 | 이름 입력 팝업 후 `data\renames\<이름>.cfg` 저장 · 동작이 비어 있으면 무동작 | bulkrename.rs:1403, bulkrename.rs:1921-1932 |
| CMD-104 | 〃 | 이름 변경 순서 편집… (Edit Renaming Sequences…) | n + 구분선 + 1 | 프리셋 관리 팝업(체크 삭제) | bulkrename.rs:1404, bulkrename.rs:1933-1937 |
| CMD-110 | 완전 삭제 확인(`MessageBoxW`) | 예 / 아니요(기본 = 아니요) | `IDYES` | 예일 때만 삭제 | win.rs:3732-3747 |
| CMD-111 | 잠긴 항목 대화상자 | "{N}개 건너뛰고 삭제"(나머지 > 0일 때만) / 다시 시도 / 취소 | 1 / 2 / 3 | 1 = 잠긴 것 제외 삭제 · 2 = 재프로브 · 그 외 = 아무것도 안 지움 | win.rs:3783-3811 |
| CMD-112 | 삭제 실패 대화상자 | 다시 시도 / 닫기 | 1 / 2 | 1 = 실패분만 재삭제 | win.rs:3962-3975 |
| CMD-113 | 덮어쓰기 충돌 대화상자 | 예 / 모두 예 / 건너뛰기 / 취소 | 1 / 2 / 3 / 4 | 2 = 이후 충돌 전부 덮어쓰기 · 4 또는 닫힘 = 전체 중단 | win.rs:4545-4575 |
| CMD-114 | 클라우드 직접 연결 안내 | 브라우저 열기 / URL 복사 / 취소 | 1 / 2 / 3 | 2 = 인증 URL을 클립보드로(프라이빗 창용) | win.rs:5667-5690 |
| CMD-115 | client_id 미설정 안내 · OAuth 오류 | 닫기 | 1 | 안내만 | win.rs:5647-5657, win.rs:5752-5757 |
| CMD-116 | 설정 창 위치 드롭다운(타입어헤드 배지 · 고속 스크롤 배지) | 3×3 이미지 팝업(오너드로 네이티브 메뉴 3열) | 1..=9 | 고른 칸 = 위치 0..8 | prefs.rs:2292-2320, prefs.rs:857-859 |

---

## 2. 명령 ID 표

### 2-1. 메인 창 명령 (`run_command` 대상 · win.rs:208-278)

"진입점" 열: M = 메뉴바 · T = 도구 모음 · K = 단축키 · C = 컨텍스트 메뉴 · L = 런처 바.

| ID | 값 | 상수 | 동작 | 진입점 | 처리 위치 | 분류 |
|---|---|---|---|---|---|---|
| CMD-120 | 1 | `CMD_NEW_TAB` | 활성 패널에 새 탭(현재 경로 복제) | M · K(Ctrl+T는 직접 호출) | win.rs:5113 (정의 win.rs:209) | N |
| CMD-121 | 2 | `CMD_CLOSE_TAB` | 활성 탭 닫기(마지막·잠긴 탭 제외) | M · K(직접 호출) | win.rs:5114-5117 | N |
| CMD-122 | 3 | `CMD_EXIT` | 창 닫기(`WM_CLOSE` 게시 → 설정·세션 저장) | M | win.rs:5120-5122 | P |
| CMD-123 | 4 | `CMD_NEW_FOLDER` | 새 폴더 + 인라인 이름변경 | M · K | win.rs:5175-5177, win.rs:4068 | N |
| CMD-124 | 5 | `CMD_NEW_FILE` | 새 파일(`{기본이름}.txt`) + 인라인 이름변경 | M | win.rs:5175-5177, win.rs:4089 | N |
| CMD-125 | 6 | `CMD_UNDO` | 실행 취소(필드 편집 중 = 필드 undo) | M · K(직접 호출) | win.rs:5373-5384 | N |
| CMD-126 | 7 | `CMD_REDO` | 다시 실행 | M · K(직접 호출) | win.rs:5373-5384 | N |
| CMD-127 | 8 | `CMD_TOGGLE_DOCK` | 하단 도크 토글(양 패널 동시) | M · T · K | win.rs:5304-5330 | N |
| CMD-128 | 9 | `CMD_TOGGLE_LAUNCHER` | 퀵 런처 바 표시 토글 | M | win.rs:5331-5339 | N |
| CMD-129 | 10 | `CMD_TOGGLE_HIDDEN` | 숨김 파일 표시 토글 | M · T · K | win.rs:5127-5164 | N(숨김 속성 정의는 P) |
| CMD-130 | 11 | `CMD_TOGGLE_DOTFILES` | 점 파일 표시 토글 | M · T · K | win.rs:5127-5164 | N |
| CMD-131 | 12 | `CMD_REFRESH` | 현 위치 재열람(클라우드 캐시 무효화) | M · T · K | win.rs:5396-5405 | N |
| CMD-132 | 13 | `CMD_VIEW_TREE` | 보기 모드 = 트리(탭별) | M · T | win.rs:5178-5192 | N |
| CMD-133 | 14 | `CMD_VIEW_FLAT` | 보기 모드 = 플랫 | M · T | win.rs:5178-5192 | N |
| CMD-134 | 15 | `CMD_VIEW_TILES` | 보기 모드 = 타일 | M · T | win.rs:5178-5192 | N |
| CMD-135 | 16 | `CMD_PANEL_SINGLE` | 싱글 파일 패널 | M | win.rs:5210-5258 | N |
| CMD-136 | 17 | `CMD_PANEL_DUAL` | 듀얼 파일 패널 | M | win.rs:5210-5258 | N |
| CMD-137 | 18 | `CMD_INFO_SINGLE` | 싱글 정보 패널(싱글 패널에서는 잠김) | M | win.rs:5259-5303 | N |
| CMD-138 | 19 | `CMD_INFO_DUAL` | 듀얼 정보 패널 | M | win.rs:5259-5303 | N |
| CMD-139 | 20 | `CMD_NAV_BACK` | 활성 패널 뒤로 | **없음**(메뉴·도구모음·단축키 어디에도 연결 안 됨 — 정의와 처리 arm만 존재) | win.rs:5406 (정의 win.rs:231) | X |
| CMD-140 | 21 | `CMD_NAV_FORWARD` | 활성 패널 앞으로 | 없음(위와 같음) | win.rs:5407 | X |
| CMD-141 | 22 | `CMD_NAV_UP` | 활성 패널 상위 | 없음(위와 같음) | win.rs:5408 | X |
| CMD-142 | 30 | `CMD_THEME_SYSTEM` | 테마 = 시스템 | M · K(F6 순환) | win.rs:5409-5417 | P(OS 테마 조회) |
| CMD-143 | 31 | `CMD_THEME_LIGHT` | 테마 = 라이트 | M · K(F6 순환) | win.rs:5409-5417 | N |
| CMD-144 | 32 | `CMD_THEME_DARK` | 테마 = 다크 | M · K(F6 순환) | win.rs:5409-5417 | N |
| CMD-145 | 40 | `CMD_LANG_SYSTEM` | 언어 = 시스템 | M | win.rs:5418-5422 | P(OS 언어 조회) |
| CMD-146 | 41 + i | `CMD_LANG_BASE` | 언어 = `st.langs[i]`(발견 목록 순) | M | win.rs:5423-5427 | N |
| CMD-147 | 60 | `CMD_PREFS` | 설정 창 열기(`WM_APP_PREFS` 게시) | M · T · K(Ctrl+, 는 `open_prefs` 직접 호출) | win.rs:5357-5360 | A |
| CMD-148 | 61 | `CMD_BULK_RENAME` | 일괄 이름변경 창(`WM_APP_BULK` 게시) | M · K | win.rs:5361-5364 | A |
| CMD-149 | 62 | `CMD_CTLDEMO` | ctl 갤러리(개발 검증 전용 — 메뉴·도구모음 비노출, 주입 전용) | 없음 | win.rs:5365-5368 (정의 win.rs:246) | X |
| CMD-150 | 63 | `CMD_COLW_SYNC` | 컬럼 너비 동기화 토글 | M · T | win.rs:5193-5209 | N |
| CMD-151 | 64 | `CMD_PANEL_TOGGLE` | 패널 듀얼↔싱글 반전 | T | win.rs:5210-5258 | N |
| CMD-152 | 65 | `CMD_INFO_TOGGLE` | 정보 듀얼↔싱글 반전 | T | win.rs:5259-5303 | N |
| CMD-153 | 66 | `CMD_ABOUT` | About 창(`WM_APP_ABOUT` 게시) | M | win.rs:5369-5372 | A |
| CMD-154 | 67 | `CMD_TOGGLE_FOLDERS_FIRST` | 폴더 우선 정렬 토글(메모리 재정렬) | T | win.rs:5127-5164 | N |
| CMD-155 | 68 | `CMD_TOGGLE_TOPMOST` | 항상 맨 위에 표시 토글 | M · T | win.rs:5166-5174 | P(창 레벨) |
| CMD-156 | 69 | `CMD_CUT` | 잘라내기(문맥 디스패치) | M · K(직접 `do_clip`) | win.rs:5385-5395 | P(클립보드) |
| CMD-157 | 70 | `CMD_COPY` | 복사(문맥 디스패치) | M · K | win.rs:5385-5395 | P |
| CMD-158 | 71 | `CMD_PASTE` | 붙여넣기(문맥 디스패치) | M · K | win.rs:5385-5395 | P |
| CMD-159 | 72 | `CMD_SELECT_ALL` | 전체 선택(문맥 디스패치) | M · K | win.rs:5385-5395 | N |
| CMD-160 | 200 + i | `CMD_LAUNCHER_BASE` | 런처 항목 실행(i < 항목 수 · 상한 32) | L | win.rs:5340-5356 | P(프로세스 실행·셸 종류) |
| CMD-161 | 299 | `CMD_CLOUD_ADD_NONE` | "감지된 클라이언트 없음" 안내 | M | win.rs:5429-5434 | N |
| CMD-162 | 300 + i | `CMD_CLOUD_GOTO_BASE` | 연결 i 폴더로 이동(API 연결 = `::CLOUD:i::` 센티널) | M | win.rs:5435-5455 | N |
| CMD-163 | 340 + i | `CMD_CLOUD_WEB_BASE` | 연결 i 서비스 웹을 기본 브라우저로 열기 | M | win.rs:5456-5474 | P(URL 열기) |
| CMD-164 | 380 + i | `CMD_CLOUD_COPYURL_BASE` | 연결 i 서비스 URL 복사 | M | win.rs:5475-5488 | P(클립보드) |
| CMD-165 | 420 + i | `CMD_CLOUD_DISC_BASE` | 연결 i 해제(API = 토큰 폐기) | M | win.rs:5489-5515 | P(비밀 저장소) |
| CMD-166 | 460 + i | `CMD_CLOUD_ADD_BASE` | 감지 후보 i 를 동기화 폴더 연결로 추가 | M · C(내 PC 배경 메뉴) | win.rs:5527-5549 | P(동기화 클라이언트 감지) |
| CMD-167 | 492 + i | `CMD_CLOUD_OAUTH_BASE` | OAuth 서비스 i 직접 연결 시작 | M | win.rs:5516-5526 | P(브라우저·루프백) |

`run_command` 공통: 진입 시 예약된 느린 재클릭 리네임 폐기(win.rs:5109). 대부분의 arm은 말미에서 무효화 flush + 타이틀/상태 갱신(win.rs:5552-5554). 대역 상한 `CLOUD_MAX = 32`(win.rs:278).

### 2-2. 컨텍스트 메뉴 · 팝업 지역 ID

| ID | 값 | 상수 | 동작 | 처리 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-168 | 0x8000 | `CTX_DELETE_PERMANENT` | 완전 삭제 | win.rs:3125 (정의 win.rs:2804) | N |
| CMD-169 | 0x8001 | `CTX_PASTE_INTO` | 폴더에 붙여넣기 | win.rs:3126-3137 (정의 win.rs:2805) | P |
| CMD-170 | 0x8002 | `CTX_UNDO` | 배경 메뉴 실행 취소 | win.rs:2908 (정의 win.rs:2806) | N |
| CMD-171 | 0x8003 | `CTX_REDO` | 배경 메뉴 다시 실행 | win.rs:2909 (정의 win.rs:2807) | N |
| CMD-172 | 0x8004 | `CTX_PASTE_BG` | 배경 메뉴 붙여넣기 | win.rs:2897-2907 (정의 win.rs:2808) | P |
| CMD-173 | 0x8005 | `CTX_COPY_PATH` | 경로 복사 | win.rs:3105-3114 (정의 win.rs:2810) | P(경로 표기·줄바꿈) |
| CMD-174 | 0x8006 | `CTX_COPY_NAME` | 이름 복사 | win.rs:3115-3124 (정의 win.rs:2812) | N |
| CMD-175 | 1 ~ 0x6FFF | `ID_SHELL_FIRST` ~ `ID_SHELL_LAST` | 셸 `IContextMenu` 명령 대역 | shellmenu.rs:37-38, shellmenu.rs:679-704 | W |
| CMD-176 | 0x7000 ~ 0x7FFF | `ID_NEW_FIRST` ~ `ID_NEW_LAST` | 셸 "새로 만들기" 확장(CLSID_NewMenu) 대역 | shellmenu.rs:40-41, shellmenu.rs:664-677 | W |
| CMD-177 | 0x8000 ~ | `ID_CUSTOM_FIRST` | 고유 병합 항목 대역 시작 | shellmenu.rs:43, shellmenu.rs:661-663 | N |
| CMD-178 | 1 | 탭 메뉴 `CMD_LOCK` | 탭 잠금 토글 | win.rs:7178 (정의 win.rs:7120) | N |
| CMD-179 | 2 | 탭 메뉴 `CMD_DUP` | 탭 복제 | win.rs:7180-7183 (정의 win.rs:7121) | N |
| CMD-180 | 3 | 탭 메뉴 `CMD_CLOSE` | 탭 닫기 | win.rs:7189 (정의 win.rs:7122) | N |
| CMD-181 | 4 | 탭 메뉴 `CMD_PIN` | 탭 고정 토글 | win.rs:7179 (정의 win.rs:7123) | N |
| CMD-182 | 5 | 탭 메뉴 `CMD_NEW` | 새 탭 | win.rs:7184-7188 (정의 win.rs:7124) | N |
| CMD-183 | 1 | 편집 팝업 `ID_UNDO` | `ClipAct::Undo` | win.rs:7578 (정의 win.rs:7501) | N |
| CMD-184 | 2 | 편집 팝업 `ID_CUT` | `ClipAct::Cut` | win.rs:7579 (정의 win.rs:7502) | P |
| CMD-185 | 3 | 편집 팝업 `ID_COPY` | `ClipAct::Copy` | win.rs:7580 (정의 win.rs:7503) | P |
| CMD-186 | 4 | 편집 팝업 `ID_PASTE` | `ClipAct::Paste` | win.rs:7581 (정의 win.rs:7504) | P |
| CMD-187 | 5 | 편집 팝업 `ID_DELETE` | `ClipAct::Delete` | win.rs:7582 (정의 win.rs:7505) | N |
| CMD-188 | 6 | 편집 팝업 `ID_SELECT_ALL` | `ClipAct::SelectAll` | win.rs:7583 (정의 win.rs:7506) | N |
| CMD-189 | 1 | 도구모음 팝업 #1 | 도구 모음 순서 편집 창 | win.rs:6292-6294 | A |
| CMD-190 | 2 | 도구모음 팝업 #2 | 설정 창 | win.rs:6295-6297 | A |
| CMD-191 | 1 | 컬럼 헤더 팝업 #1 | 파일 컬럼 편집 창 | win.rs:6298-6300 | A |
| CMD-192 | 1 | `BTN_BACK` | 패널 뒤로 | panel.rs:1535 (정의 panel.rs:100) | N |
| CMD-193 | 2 | `BTN_FORWARD` | 패널 앞으로 | panel.rs:1536 (정의 panel.rs:101) | N |
| CMD-194 | 3 | `BTN_UP` | 패널 상위 | panel.rs:1537 (정의 panel.rs:102) | N |
| CMD-195 | 4 | `BTN_HOME` | 패널 홈(내 PC) | panel.rs:1538 (정의 panel.rs:104) | N("내 PC" 가상 루트의 타 OS 대응은 P) |
| CMD-196 | 1 + i | 내 PC 클라우드 메뉴 | 후보 i 연결 → `CMD_CLOUD_ADD_BASE + i` | win.rs:3242-3247 | N |
| CMD-197 | 1 | 미리보기 창 `ID_COPY` | 선택 텍스트 rich 복사 | previewwnd.rs:372-377 (정의 previewwnd.rs:341) | P |
| CMD-198 | 2 | 미리보기 창 `ID_SELECT_ALL` | 전체 선택 | previewwnd.rs:378-381 (정의 previewwnd.rs:342) | N |

### 2-3. 지연 실행 메시지(명령의 2단계 — 모달 창을 State 차용 밖에서 열기 위한 게시)

| ID | 값 | 상수 | 동작 | 처리 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-199 | 0x8005 | `WM_APP_PREFS` | `open_prefs` | win.rs:8984-8987 (정의 win.rs:138) | P(타입 있는 사용자 이벤트로 교체) |
| CMD-200 | 0x8008 | `WM_APP_BULK` | `open_bulk_rename` | win.rs:9039-9042 (정의 win.rs:148) | P |
| CMD-201 | 0x8009 | `WM_APP_CTLDEMO` | ctl 갤러리 표시(개발 전용) | win.rs:9043-9049 (정의 win.rs:150) | X |
| CMD-202 | 0x800A | `WM_APP_EDIT_TOOLBAR` | `open_order_editor(ORDER_FIELD_TOOLBAR)` | win.rs:8988-8991 (정의 win.rs:141) | P |
| CMD-203 | 0x800B | `WM_APP_EDIT_COLS` | `open_order_editor(ORDER_FIELD_COLS)` | win.rs:8992-8995 (정의 win.rs:142) | P |
| CMD-204 | 0x800C | `WM_APP_ABOUT` | `about::show` | win.rs:9050-9057 (정의 win.rs:152) | P |
| CMD-205 | (enum) | `ClipAct::{Undo, Cut, Copy, Paste, Delete, SelectAll}` | 편집 동작 공용 디스패치 `do_clip`: ① 경로바 편집 → ② 인라인 이름변경 → ③ 도크 터미널(키 포커스 · 잘라내기 = 복사) → ④ 도크 텍스트 선택(복사만) → ⑤ 파일 목록 | win.rs:7210-7218, win.rs:7246-7385 | N(클립보드 호출부만 P) |

### 2-4. 일괄 이름변경 창 컨트롤 ID (bulkrename.rs:49-99 · 통지 처리 `br_proc` bulkrename.rs:1812-2005)

| ID | 값 | 상수 | 역할 | 근거 |
|---|---|---|---|---|
| CMD-206 | 1 | `ID_KIND` | 카드 동작 종류 콤보 | bulkrename.rs:50 |
| CMD-207 | 7 | `ID_SCOPE` | 적용 범위 콤보(Apply to) | bulkrename.rs:52 |
| CMD-208 | 2 | `ID_ADD` | 카드 [+] = 아래에 새 카드 | bulkrename.rs:60, bulkrename.rs:1866-1879 |
| CMD-209 | 5 | `ID_DEL` | 카드 [−] = 이 카드 삭제(마지막 1장은 불가) | bulkrename.rs:61, bulkrename.rs:1881-1893 |
| CMD-210 | 10 | `ID_FIND` | 치환: 찾을 문자열 | bulkrename.rs:63 |
| CMD-211 | 11 | `ID_WITH` | 치환: 바꿀 문자열 | bulkrename.rs:64 |
| CMD-212 | 12 | `ID_MC` | 치환: 대소문자 일치 | bulkrename.rs:65 |
| CMD-213 | 14 | `ID_RX_MODE` | 치환 모드(모든/첫/마지막/전체) | bulkrename.rs:67 |
| CMD-214 | 13 | `ID_RXF_MC` | 정규식 카드: 대소문자 일치 | bulkrename.rs:69 |
| CMD-215 | 15 | `ID_RXF_FIND` | 정규식 카드: 패턴 | bulkrename.rs:70 |
| CMD-216 | 16 | `ID_RXF_WITH` | 정규식 카드: 치환 문자열 | bulkrename.rs:71 |
| CMD-217 | 96 | `ID_DT_HELP` | 날짜 포맷 도움말 [?] → `MessageBoxW` 안내 | bulkrename.rs:73, bulkrename.rs:1855-1865 |
| CMD-218 | 20 + 0..3 | `ID_CASE_BASE` | 대소문자 변환(upper/lower/title/sentence) | bulkrename.rs:74 |
| CMD-219 | 30 | `ID_INS_TEXT` | 삽입: 텍스트 | bulkrename.rs:75 |
| CMD-220 | 33 | `ID_INS_OFF` | 삽입: 위치 오프셋(스핀) | bulkrename.rs:76 |
| CMD-221 | 34 | `ID_INS_DIR` | 삽입: 앞/뒤(세그먼트) | bulkrename.rs:77 |
| CMD-222 | 40 | `ID_NUM_START` | 번호: 시작값 | bulkrename.rs:78 |
| CMD-223 | 41 | `ID_NUM_STEP` | 번호: 증가값 | bulkrename.rs:79 |
| CMD-224 | 42 | `ID_NUM_PAD` | 번호: 자릿수 | bulkrename.rs:80 |
| CMD-225 | 45 | `ID_NUM_OFF` | 번호: 위치(스핀) | bulkrename.rs:81 |
| CMD-226 | 46 | `ID_NUM_DIR` | 번호: 앞/뒤(세그먼트) | bulkrename.rs:82 |
| CMD-227 | 47 | `ID_NUM_WPRE` | 번호: 감싸기 Prefix | bulkrename.rs:83 |
| CMD-228 | 48 | `ID_NUM_WSUF` | 번호: 감싸기 Suffix | bulkrename.rs:84 |
| CMD-229 | 90 | `ID_DT_KIND` | 날짜: 수정/생성 콤보 | bulkrename.rs:86 |
| CMD-230 | 91 | `ID_DT_FMT` | 날짜: 포맷 | bulkrename.rs:87 |
| CMD-231 | 92 | `ID_DT_OFF` | 날짜: 위치 | bulkrename.rs:88 |
| CMD-232 | 93 | `ID_DT_DIR` | 날짜: 앞/뒤 | bulkrename.rs:89 |
| CMD-233 | 94 | `ID_DT_PRE` | 날짜: Prefix | bulkrename.rs:90 |
| CMD-234 | 95 | `ID_DT_SUF` | 날짜: Suffix | bulkrename.rs:91 |
| CMD-235 | 82 | `ID_COUNT` | 변경 건수 라벨 | bulkrename.rs:93 |
| CMD-236 | 84 | `ID_PREV` | 미리보기 그리드 — 적용 열 체크 토글(행별 제외 · 헤더 = 전체 토글) · 헤더 정렬 변경 | bulkrename.rs:95, bulkrename.rs:1939-1966 |
| CMD-237 | 71 | `ID_PRESET_MENU` | 프리셋 `…` 메뉴 버튼(CMD-102~104) | bulkrename.rs:97, bulkrename.rs:1894-1938 |
| CMD-238 | 80 | `ID_APPLY` | [이름 변경] 확정 — 이름이 바뀌고 제외되지 않은 행만 반환 | bulkrename.rs:98, bulkrename.rs:1967-1994 |
| CMD-239 | 81 | `ID_CANCEL` | [취소] | bulkrename.rs:99, bulkrename.rs:1995-1997 |

### 2-5. 그 밖의 하위 창 컨트롤 ID

| ID | 값 | 상수 | 역할 | 근거 |
|---|---|---|---|---|
| CMD-240 | 1002 | 설정 `ID_SEARCH` | 검색 상자(입력 변경 = 트리 필터·우측 재구성) | prefs.rs:847, prefs.rs:2353-2354 |
| CMD-241 | 1997 | 설정 `ID_MODBAR` | "수정됨" 세로 accent 바(표시 전용 · 공유 id) | prefs.rs:849 |
| CMD-242 | 1998 | 설정 `ID_DESC` | 설명 문장(표시 전용 · 공유 id) | prefs.rs:851 |
| CMD-243 | 1100 | 설정 `ID_TREE` | 사이드바 트리(선택 = 그룹 펼침 토글·페이지 전환) | prefs.rs:852, prefs.rs:2395 |
| CMD-244 | 1200 + field | 설정 `ID_FIELD_BASE` | 필드별 체크/편집 컨트롤(클릭 즉시 수확 + 적용). 순서 편집 창 열기 버튼 포함 | prefs.rs:853, prefs.rs:2391 |
| CMD-245 | 1400 + n | 설정 `ID_OPT_BASE` | 라디오 옵션 순번 | prefs.rs:854 |
| CMD-246 | 1600 + ti | 설정 `ID_NAV_BASE` | 그룹 페이지의 하위 메뉴 링크(드릴다운) | prefs.rs:856, prefs.rs:2415-2419 |
| CMD-247 | 2100 + pi | 설정 `ID_PLUGIN_BASE` | 플러그인 사용 여부 체크(동적 목록) | prefs.rs:873, prefs.rs:1833 |
| CMD-248 | 1 | 순서 편집 `ID_TREE` | 순서 트리(체크 토글·드래그 이동 통지) | ordereditor.rs:31, ordereditor.rs:360-361 |
| CMD-249 | 2 | 순서 편집 `ID_UP` | ▲ 선택 블록 위로(선택 전 비활성) | ordereditor.rs:32, ordereditor.rs:358 |
| CMD-250 | 3 | 순서 편집 `ID_DOWN` | ▼ 선택 블록 아래로(선택 전 비활성) | ordereditor.rs:33, ordereditor.rs:359 |
| CMD-251 | 1 | 암호 입력 `ID_EDIT` | 암호 입력 필드 | pwprompt.rs:33 |
| CMD-252 | 2 | 암호 입력 `ID_SHOW` | 암호 표시 체크 | pwprompt.rs:34 |
| CMD-253 | 3 | 암호 입력 `ID_OK` | 확인 | pwprompt.rs:35 |
| CMD-254 | 4 | 암호 입력 `ID_CANCEL` | 취소 | pwprompt.rs:36 |
| CMD-255 | 1 | About `ID_OK` | 닫기 | about.rs:38, about.rs:144-148 |
| CMD-256 | 1 | 압축 미리보기 `ID_GRID` | 압축 항목 그리드 | archivewnd.rs:37 |
| CMD-257 | 1 | 진행 창 `ID_CANCEL` | 전송 취소 버튼 | dialog.rs:354, dialog.rs:490-491 |
| CMD-258 | 100~900 | ctl 갤러리 15종(`ID_SCOPE` · `ID_MODE` · `ID_FIND` · `ID_DIR` · `ID_OFF` · `ID_OPKIND` · `ID_CASE` · `ID_ADD` · `ID_REMOVE` · `ID_MORE` · `ID_CANCEL` · `ID_RENAME` · `ID_DISABLED` · `ID_ROW_BASE` · `ID_STATUS`) | 개발 검증 전용 창 | nexa-dir2/crates/nexa-app/src/ctldemo.rs:29-50 |
| CMD-259 | 0 / 1 / 2 | 접근성 `SEL_SINGLE` / `SEL_ADD` / `SEL_REMOVE`(`WM_APP_UIA_SELECT`) | 스크린리더가 요청하는 프로그램적 행 선택(단일/추가/해제) | nexa-dir2/crates/nexa-app/src/uia.rs:43-46, win.rs:9291-9312 |

---

## 3. 단축키 표

### 3-1. 문맥 ① — 경로바 편집 중 (`pathbar.is_editing()` · win.rs:8706-8739 · 문자 입력 win.rs:9383-9387)

이 문맥에서는 처리 후 항상 return 하므로 **전역 단축키(F5 · Ctrl+T 등)가 동작하지 않는다**.

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-260 | Enter | 경로바 편집 | 입력 제출 → 이동(환경변수 `%VAR%` · `$env:VAR` 확장, `shell:` 특수 폴더 해석) | win.rs:8707-8709, panel.rs:1518-1532 | P(경로 문법) |
| CMD-261 | Esc | 경로바 편집 | 자동완성 팝업이 열려 있으면 **팝업만 닫기**, 아니면 편집 취소 | win.rs:8710-8716 | N |
| CMD-262 | ↑ / ↓ | 경로바 편집 | 자동완성 제안 이동(선택 항목을 필드에 미리 채움 · 첫 항목에서 ↑ = 원래 입력 복원) | win.rs:8717-8720, gui/widgets/pathbar.rs:224-254 | N |
| CMD-263 | Ctrl+C | 경로바 편집 | 선택 텍스트 복사 | win.rs:8721-8724, win.rs:7261-7266 | P |
| CMD-264 | Ctrl+X | 경로바 편집 | 선택 텍스트 잘라내기 | win.rs:7254-7260 | P |
| CMD-265 | Ctrl+V | 경로바 편집 | 붙여넣기(첫 줄만 · 제어 문자 제거) | win.rs:7267-7273 | P |
| CMD-266 | Ctrl+Z | 경로바 편집 | 필드 실행 취소(단일 단계) | win.rs:7253 | N |
| CMD-267 | ← / → (Shift = 선택 확장) | 경로바 편집 | 캐럿 이동. 선택이 있으면 Shift 없이 누를 때 선택 가장자리로 접힘 | win.rs:8725-8727, gui/edit.rs:186-203 | N |
| CMD-268 | Home / End (Shift = 선택 확장) | 경로바 편집 | 처음/끝으로 | gui/edit.rs:204-205 | N |
| CMD-269 | Delete | 경로바 편집 | 앞으로 삭제(선택 있으면 선택 삭제) + 자동완성 갱신 | win.rs:8728-8730, gui/edit.rs:210-215 | N |
| CMD-270 | Ctrl+A | 경로바 편집 | 전체 선택 | win.rs:7644, gui/edit.rs:206-209 | N |
| CMD-271 | 문자 · Backspace | 경로바 편집 | 문자 삽입/뒤로 삭제 + 자동완성 갱신(하위 폴더 최대 20개 제안) | win.rs:9383-9387, win.rs:7199-7206 | N |

### 3-2. 문맥 ② — 도크 터미널 키 포커스 (`term_focus` · win.rs:8743-8771 · 문자 win.rs:9342-9381)

터미널이 포커스를 가지면 `WM_KEYDOWN` 이 모두 여기서 return 한다 → **F5 · F6 · Ctrl+T · Tab(패널 전환) 등 전역 단축키가 동작하지 않는다**. 단 `WM_SYSKEYDOWN` 경로(CMD-325~329)는 터미널 포커스를 보지 않아 **Alt+방향키 · Shift+F10 은 파일 패널에 그대로 작용**한다(win.rs:8905-8937 — 포커스 검사 없음).

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-272 | ↑ ↓ → ← | 터미널 | `ESC[A` / `ESC[B` / `ESC[C` / `ESC[D` 전송 | win.rs:6134-6137 | N |
| CMD-273 | Home / End | 터미널 | `ESC[H` / `ESC[F` | win.rs:6138-6139 | N |
| CMD-274 | Delete | 터미널 | `ESC[3~` | win.rs:6140 | N |
| CMD-275 | PgUp / PgDn | 터미널 | `ESC[5~` / `ESC[6~` | win.rs:6141-6142 | N |
| CMD-276 | 아무 키 | 터미널(셸 종료 상태) | 터미널 재시작(다음 그리기에서 새 PTY) | win.rs:8747-8751, win.rs:9348-9351 | P |
| CMD-277 | 문자 · Enter · Tab · Esc · Ctrl+문자 | 터미널 | `WM_CHAR` 코드 그대로 셸 stdin 으로(UTF-8). 입력 시 스크롤백 보기 해제 + 선택 해제 | win.rs:9362-9371 | P(winit 은 문자와 제어 키가 분리됨) |
| CMD-278 | Backspace / Ctrl+Backspace | 터미널 | Backspace = `0x7F`(1글자) · Ctrl+Backspace(`0x7F` 수신) = `0x08`(단어 삭제) 교차 매핑 | win.rs:9364-9367 | P(ConPTY 규약 — 타 OS PTY 는 재검증) |
| CMD-279 | Ctrl+C | 터미널 | **선택이 있으면 복사**(평문 + HTML + RTF) 후 선택 해제, 없으면 `0x03` 인터럽트 전송 | win.rs:9354-9358 | P |
| CMD-280 | Ctrl+V | 터미널 | 붙여넣기(CRLF/LF → CR) | win.rs:9359-9361 | P |
| CMD-281 | 모든 키 | 터미널 포커스인데 PTY 미기동(재기동 대기) | 키를 삼키고 다시 그리기만(목록으로 누수 금지) | win.rs:8758-8762, win.rs:9345 | N |

터미널 포커스 판정 `route_key_with_term`(win.rs:6096-6113): 포커스 없음 = 목록 · 도크 숨김/종류 전환 = 포커스 해제 후 목록 · PTY 있음 = 터미널 · 없음 = 대기. **F1~F12 · Insert · Shift+Tab 등은 VT 시퀀스로 전달되지 않는다**(`term_key_seq` 에 9개 키만 정의 — win.rs:6132-6145).

### 3-3. 문맥 ③ — 인라인 이름변경 중 (`rows().is_renaming()` · win.rs:8772-8792)

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-282 | Enter | 이름변경 | 확정 → `apply_rename`(실패는 상태 줄 안내) | win.rs:8774-8781 | N |
| CMD-283 | Esc | 이름변경 | 취소 | win.rs:8782-8783 | N |
| CMD-284 | Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z | 이름변경 | 필드 복사 · 잘라내기 · 붙여넣기(첫 줄) · 실행 취소 | win.rs:8784-8786, win.rs:7288-7316 | P |
| CMD-285 | ← → Home End(Shift = 선택) · Delete · Ctrl+A | 이름변경 | 캐럿 이동 · 선택 · 앞으로 삭제 · 전체 선택 | win.rs:8787-8788, win.rs:7636-7647 | N |
| CMD-286 | 문자 · Backspace | 이름변경 | 버퍼 편집(스페이스도 문자로 입력) | gui/widgets/rows.rs:1733-1736, win.rs:9393-9395 | N |

### 3-4. 문맥 ④ — 파일 목록/전역 (위 문맥이 모두 아닐 때 · win.rs:8793-8901)

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-287 | Esc | 탭 드래그 중 | 탭 드래그 취소 — 잡은 탭을 원위치로(패널 간 미리 보기 이동 포함) | win.rs:8793-8796, win.rs:5962-5992 | N |
| CMD-288 | Esc | 컬럼 헤더 드래그 중 | 컬럼 재배열 취소(원래 순서 복원) | win.rs:8797-8804 | N |
| CMD-289 | Esc | 전송 진행 중 | 전송 취소 플래그 설정 | win.rs:8805-8810 | N |
| CMD-290 | Esc | 메뉴바 드롭다운 열림 | 드롭다운 닫기 | win.rs:8811-8812 | N |
| CMD-291 | F5 | 목록 | 새로 고침(`CMD_REFRESH`) | win.rs:8813-8815 | N |
| CMD-292 | F6 | 목록 | 테마 순환: 다크 → 라이트 → 시스템 → 다크 | win.rs:8816-8824 | N |
| CMD-293 | F3 | 목록 | 독립 미리보기 창(모달). **단일 선택 파일**일 때만. 압축 파일이면 압축 그리드 창 | win.rs:8825-8833, win.rs:2425-2472 | A |
| CMD-294 | Shift+F3 | 목록 | 개발용 스크롤 벤치(200프레임) | win.rs:8826-8827, win.rs:6054-6077 | X |
| CMD-295 | Tab | 목록 | 패널 전환(좌↔우). 싱글 패널이면 좌 고정 | win.rs:8834-8840, win.rs:6031-6032 | N |
| CMD-296 | Ctrl+Tab | 목록 | 다음 탭(순환). **이전 탭(Ctrl+Shift+Tab)은 없음** — Shift 를 눌러도 다음 탭 | win.rs:8835-8836, panel.rs:751-756 | N |
| CMD-297 | Ctrl+T | 목록 | 새 탭 | win.rs:8841-8842 | N |
| CMD-298 | Ctrl+W | 목록 | 활성 탭 닫기 | win.rs:8843-8845 | N |
| CMD-299 | Ctrl+A | 목록 | 전체 선택(현재 가시 노드 전부) | win.rs:8846-8847, win.rs:7349-7354 | N |
| CMD-300 | Ctrl+C | 목록 | 도크 정보/미리보기에 텍스트 선택이 있으면 그것을 복사, 없으면 선택 파일을 클립보드(복사)로. 선택 없으면 클립보드 유지 | win.rs:8848-8851, win.rs:7332-7366 | P |
| CMD-301 | Ctrl+X | 목록 | 선택 파일 잘라내기(잘라낸 행은 흐리게 표시) | win.rs:8852-8853, win.rs:9691-9700 | P |
| CMD-302 | Ctrl+V | 목록 | 붙여넣기 — 대상: 선택 1개가 폴더면 그 안 · 파일이면 그 파일의 폴더 · 그 외 현재 폴더. 잘라내기는 1회성. 실경로가 없으면 가상 파일 폴백 | win.rs:8854-8857, win.rs:7367-7382, win.rs:2118-2134 | P |
| CMD-303 | Ctrl+Z | 목록 | 파일 작업 실행 취소 | win.rs:8858-8860 | N |
| CMD-304 | Ctrl+Shift+Z | 목록 | 다시 실행(메뉴에는 표기 없음) | win.rs:8858-8859 | N |
| CMD-305 | Ctrl+Y | 목록 | 다시 실행 | win.rs:8861-8863 | N |
| CMD-306 | Enter | 목록 | 캐럿 행 활성화: 폴더 = 진입 · 파일 = 연결 프로그램 실행(전송 중 대상은 차단 + 경고음 · 클라우드 파일은 내려받은 뒤 열기) | win.rs:8864-8869, win.rs:7059-7078 | P(파일 열기) |
| CMD-307 | F2 | 목록 | 캐럿 행 인라인 이름변경 | win.rs:8870-8872, win.rs:4005 | N |
| CMD-308 | Delete | 목록 | 휴지통으로 삭제(워커 · undo 가능 · 잠긴 항목은 대화상자) | win.rs:8873-8875, win.rs:3748-3750 | P(휴지통) |
| CMD-309 | Shift+Delete | 목록 | 완전 삭제(확인 대화상자 · 기본 = 아니요) | win.rs:8873-8875, win.rs:3732-3764 | N |
| CMD-310 | Apps(메뉴) 키 | 목록 | 캐럿 행 위치에 행 컨텍스트 메뉴 | win.rs:8876-8878 | P |
| CMD-311 | Ctrl+Shift+N | 목록 | 새 폴더 | win.rs:8879-8881 | N |
| CMD-312 | Ctrl+Shift+R | 목록 | 일괄 이름변경 | win.rs:8882-8884 | N |
| CMD-313 | Ctrl+H | 목록 | 숨김 파일 표시 토글 | win.rs:8885-8887 | N |
| CMD-314 | Ctrl+. | 목록 | 점 파일 표시 토글 | win.rs:8888-8890 | N |
| CMD-315 | Ctrl+, | 목록 | 설정 창 | win.rs:8891-8893 | N |
| CMD-316 | Ctrl+` | 목록 | 하단 도크 토글 | win.rs:8894-8896 | N |
| CMD-317 | ↑ / ↓ (Shift = 범위 선택 · Ctrl = 캐럿만 이동) | 목록 | 캐럿 이동 + 단일 선택. 자동 반복이 이어지면 고속 스크롤 배수 적용. 타일 보기 = ±열 수 | gui/widgets/rows.rs:1804-1860, gui/widgets/rows.rs:1095-1108 | N |
| CMD-318 | PgUp / PgDn (Shift · Ctrl 동일 규칙) | 목록 | 한 페이지 이동 | gui/widgets/rows.rs:1853-1854, gui/widgets/rows.rs:1829-1830 | N |
| CMD-319 | Home / End (Shift · Ctrl 동일 규칙) | 목록 | 처음/끝 | gui/widgets/rows.rs:1855-1856 | N |
| CMD-320 | → | 목록(트리 보기) | 접힌 폴더 = 펼침 · 이미 펼침 = 첫 자식으로. 플랫 = 무동작 · 타일 = 다음 항목 | gui/widgets/rows.rs:1862-1881, gui/widgets/rows.rs:1828 | N |
| CMD-321 | ← | 목록(트리 보기) | 펼친 폴더 = 접힘 · 그 외 = 부모 행으로. 플랫 = 무동작 · 타일 = 이전 항목 | gui/widgets/rows.rs:1883-1893, gui/widgets/rows.rs:1827 | N |
| CMD-322 | Space · Ctrl+Space | 목록 | 캐럿 행 선택 토글. 단 타입어헤드 접두사 입력 중 + "공백 포함" 옵션이면 문자로 처리 | gui/widgets/rows.rs:1895-1902, win.rs:9391-9395 | N |
| CMD-323 | 인쇄 가능 문자(Ctrl 안 누름) | 목록 | 타입어헤드 — 접두사 일치 행으로 캐럿 이동(옵션: 특수문자 포함 · 타임아웃) | win.rs:9388-9406, gui/widgets/rows.rs:1905-1924 | N |
| CMD-324 | Backspace | 목록 | 타입어헤드 접두사 한 글자 축소(옵션이 꺼져 있으면 무시). **상위 폴더 이동이 아님** | gui/widgets/rows.rs:1906-1913 | N |

### 3-5. Alt 조합 · 시스템 키 (`WM_SYSKEYDOWN` · win.rs:8905-8939)

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-325 | Shift+F10 | 전역(포커스 무관) | 캐럿 행 위치에 행 컨텍스트 메뉴 | win.rs:8910-8912 | P |
| CMD-326 | Alt+← | 전역 | 활성 패널 뒤로 | win.rs:8913-8915 | N |
| CMD-327 | Alt+→ | 전역 | 활성 패널 앞으로 | win.rs:8916-8918 | N |
| CMD-328 | Alt+↑ | 전역 | 상위 폴더(떠난 폴더를 선택 · 배치 = 설정 `nav_up_align`) | win.rs:8919-8921, panel.rs:985-1009 | N |
| CMD-329 | Alt+↓ | 전역 | 캐럿 행 활성화(더블클릭과 동등) | win.rs:8922-8929 | N |
| CMD-330 | Alt+F4 · Alt+Space 등 | 전역 | OS 기본 처리(`DefWindowProcW`) — 앱 코드 없음 | win.rs:8938 | P |

### 3-6. 하위 창 · 공용 컨트롤

| ID | 키 | 문맥 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-331 | Esc | 독립 미리보기 창 | 창 닫기 | previewwnd.rs:835-837 | N |
| CMD-332 | Ctrl+C | 독립 미리보기 창 | 선택 텍스트 rich 복사(평문 + 모노 RTF). **Ctrl+A 는 없음**(전체 선택은 우클릭 메뉴로만) | previewwnd.rs:838-843 | P |
| CMD-333 | ↑ / ↓ | 독립 미리보기 창 | 1줄 스크롤 | previewwnd.rs:844-845 | N |
| CMD-334 | ← / → | 독립 미리보기 창 | 가로 24px 스크롤 | previewwnd.rs:846-847 | N |
| CMD-335 | PgUp / PgDn | 독립 미리보기 창 | 한 화면 스크롤 | previewwnd.rs:848-849 | N |
| CMD-336 | Home / End | 독립 미리보기 창 | Home = 맨 위·맨 왼쪽 · End = 맨 아래 | previewwnd.rs:850-851 | N |
| CMD-337 | Esc | 압축 미리보기 창 | 창 닫기 | archivewnd.rs:286-289 | N |
| CMD-338 | Ctrl+C | 압축 미리보기 창 | 선택 행 복사(그리드 키는 CMD-355) | archivewnd.rs:290-293 | P |
| CMD-339 | Enter | 암호 입력 창 | 확인(입력값 회수 후 닫기) | pwprompt.rs:224-227 | N |
| CMD-340 | Esc | 암호 입력 창 | 취소 | pwprompt.rs:228-231 | N |
| CMD-341 | Enter | 설정 창(편집 컨트롤 포커스) | 편집 중 값 **즉시 적용**. 글꼴 상자 드롭다운이 열려 있으면 목록 확정이 우선. 설정 창에 **Esc 닫기 처리는 없음**(Grep 0건) · Tab 이동도 없음(`IsDialogMessageW` 미사용) | prefs.rs:3032-3060 | A |
| CMD-342 | Esc | 순서 편집 창 | 드래그 중이면 드래그 취소, 아니면 창 닫기 | ordereditor.rs:373-383 | N |
| CMD-343 | ↑ / ↓ | 순서 편집 창 | 선택 이동 | ordereditor.rs:386-401 | N |
| CMD-344 | Shift+↑ / ↓ | 순서 편집 창 | 같은 부모 형제 범위로 선택 확장 | ordereditor.rs:397-398 | N |
| CMD-345 | Ctrl+↑ / ↓ | 순서 편집 창 | 선택 블록 순서 이동(▲▼ 버튼과 동일) | ordereditor.rs:390-394 | N |
| CMD-346 | Space | 순서 편집 창 | 체크(표시 여부) 토글 | ordereditor.rs:404-409 | N |
| CMD-347 | Tab / Shift+Tab | 일괄 이름변경 창 · 프리셋 이름 입력 · 프리셋 관리 | 생성 순서대로 포커스 이동(`IsDialogMessageW`). Enter/Esc 의 기본 버튼 동작은 대화상자 관리자 몫 — **추정**(명시 코드 없음) | bulkrename.rs:2236-2240, bulkrename.rs:1611-1616, bulkrename.rs:1802-1806 | A |
| CMD-348 | Space / Enter | 버튼 컨트롤 | 클릭 | ctl/button.rs:197-198 | A |
| CMD-349 | Space | 체크박스 컨트롤 | 토글 | ctl/checkbox.rs:163-164 | A |
| CMD-350 | ↓(열기) · ↑/↓(이동) · Enter(확정) · Esc(닫기) | 콤보박스 컨트롤 | 팝업 조작 | ctl/combobox.rs:280-300 | A |
| CMD-351 | ← / → | 세그먼트 컨트롤 | 선택 이동 | ctl/segmented.rs:208-213 | A |
| CMD-352 | ↑ / ↓ | 스핀 컨트롤 | 값 증감 | ctl/spin.rs:187-192 | A |
| CMD-353 | ↓(열기) · ↑/↓ · Enter · Esc | 메뉴 버튼 컨트롤 | 팝업 조작(구분선 건너뜀) | ctl/menubutton.rs:253-287 | A |
| CMD-354 | ↓(열기) · ↑/↓ · PgUp/PgDn · Enter · Esc | 글꼴 상자 컨트롤 | 드롭다운 목록 조작 · 입력 중 접두 매칭 | ctl/fontbox.rs:381-416 | A |
| CMD-355 | ↑ ↓ PgUp PgDn Home End(Shift = 범위 · Ctrl = 포커스만) · Space(단일 선택) · Ctrl+Space(토글) · Ctrl+A(전체) | 그리드 컨트롤(압축 창 · 일괄 이름변경 미리보기 · 프리셋 관리) | 행 선택 조작 | ctl/grid.rs:749-803 | A |
| CMD-356 | Esc | 파일 드래그 중(OLE) | 드래그 취소 | dnd.rs:251-253 | P |

---

## 4. 마우스 제스처 표

수식키 해석: 좌클릭의 Shift/Ctrl 은 `wParam` 의 `MK_SHIFT`/`MK_CONTROL`(win.rs:7965-7966), 우클릭·스플리터는 `GetKeyState`(win.rs:8185, win.rs:6151). **가운데 버튼(휠 클릭)과 Ctrl+휠은 어디에도 구현되어 있지 않다**(`WM_MBUTTON*` Grep 0건).

### 4-1. 파일 목록

| ID | 제스처 | 대상 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-400 | 좌클릭 | 미선택 행 | 단일 선택 + 캐럿. 기선택 행을 누르면 **선택 유지**, 드래그 없이 떼는 순간 단일 선택으로 붕괴 | gui/widgets/rows.rs:1960-1975, gui/widgets/rows.rs:2096-2103 | N |
| CMD-401 | Shift+좌클릭 | 행 | 앵커부터 범위 선택 | gui/widgets/rows.rs:1967-1968 | N |
| CMD-402 | Ctrl+좌클릭 | 행 | 비연속 토글 | gui/widgets/rows.rs:1969-1970 | P(macOS = Cmd) |
| CMD-403 | 좌클릭 | 행의 펼침 삼각형 | 인라인 펼침/접힘(선택과 분리) | gui/widgets/rows.rs:1953-1958 | N |
| CMD-404 | 좌버튼 드래그 | 미선택 행 또는 빈 본문에서 시작 | 러버밴드 다중 선택(4px 미만은 무시). 타일 보기 = 사각형과 교차하는 타일 | gui/widgets/rows.rs:1976-1999, gui/widgets/rows.rs:2032-2077 | N |
| CMD-405 | 좌클릭 | 빈 본문 | 선택 해제 | gui/widgets/rows.rs:1989-1998 | N |
| CMD-406 | 좌버튼 드래그 | **기선택** 행에서 시작 | 파일 드래그 발신(OLE). 임계 = 시스템 드래그 거리(최소 4px). 전송 중인 대상은 제외. 시작되면 느린 재클릭 리네임 취소 | win.rs:8151-8153, win.rs:8338-8378 | P(DnD) |
| CMD-407 | 느린 재클릭(1000ms 이상 간격) | 기선택 행(Shift/Ctrl 없음 · 리네임 중 아님) | 더블클릭 시간만큼 지연 후 인라인 이름변경 진입. 그 사이 더블클릭·키 입력·드래그·우클릭이면 취소. 예약 당시 (패널, 경로)가 현재 캐럿 행과 같을 때만 발동 | win.rs:8132-8150, win.rs:8573-8581, win.rs:9583-9602 | P(더블클릭 시간 조회) |
| CMD-408 | 더블클릭 | 행(삼각형 제외) | 활성화: 폴더 진입 / 파일 열기 | win.rs:8675-8691 | P(파일 열기) |
| CMD-409 | 더블클릭 | 이름변경 필드 안 · 경로바 편집 필드 안 | 필드 전체 선택(열기 아님) | win.rs:8628-8647 | N |
| CMD-410 | 더블클릭 | 컬럼 경계 | 그 컬럼 폭을 내용에 맞춤(최대 폭 = 설정 `col_autofit_max`) | win.rs:8667-8674 | N |
| CMD-411 | 좌클릭 | 컬럼 헤더 라벨 | 정렬 3상태 순환: 오름 → 내림 → 없음(열거 순서) | gui/widgets/rows.rs:2090-2094, gui/widgets/rows.rs:1343-1347 | N |
| CMD-412 | Shift+좌클릭 | 컬럼 헤더 라벨 | 다중열 정렬: 추가(오름) → 내림 → 제거 | gui/widgets/rows.rs:1332-1341 | N |
| CMD-413 | 좌버튼 드래그 | 컬럼 헤더 라벨 | 컬럼 재배열(임계 5px · 라이브 미리보기 · 뗌 = 확정 · Esc = 취소) | gui/widgets/rows.rs:1940-1950, gui/widgets/rows.rs:2013-2031 | N |
| CMD-414 | 좌버튼 드래그 | 컬럼 헤더 경계 | 그 컬럼 폭 조절(최소 폭 하한 · 이웃 불변). 동기화가 켜져 있으면 반대 패널에도 반영 | gui/widgets/rows.rs:1932-1938, gui/widgets/rows.rs:2002-2012, win.rs:8426 | N |
| CMD-415 | 우클릭(누름) | 행 | 미선택 행 = 단독 선택 · 선택된 행 = 선택 유지 + 캐럿 이동. 동시에 컨텍스트 메뉴 선행 구축 | gui/widgets/rows.rs:2108-2117, win.rs:8213 | N |
| CMD-416 | 우클릭(뗌) | 행 | 행 컨텍스트 메뉴(§1-5). 짝이 되는 누름이 없던 뗌은 무시 | win.rs:8237-8241, win.rs:8271-8278 | P |
| CMD-417 | Shift+우클릭 | 행 · 빈 본문 | 확장 동사가 포함된 셸 메뉴 | win.rs:3019, win.rs:2875 | W |
| CMD-418 | 우클릭 | 빈 본문 | 선택 해제 + 배경 메뉴(§1-6). 내 PC 에서는 클라우드 메뉴(§1-7) | gui/widgets/rows.rs:2118-2122, win.rs:8279 | P |
| CMD-419 | 우클릭 | 컬럼 헤더 | 그 패널 활성화 + "파일 컬럼..." 팝업 | win.rs:8171-8176 | A |
| CMD-420 | 좌클릭 / 드래그 | 오버레이 스크롤바 | 썸 드래그 = 비례 스크롤 · 트랙 클릭 = 한 페이지 이동. 다른 처리보다 우선 | gui/widgets/rows.rs:1715-1728, gui/widgets/overlaybar.rs:201-238 | A |
| CMD-421 | 마우스 X버튼 1 / 2 | 창 어디서나 | 활성 패널 뒤로 / 앞으로 | win.rs:8592-8604 | P(winit Back/Forward 버튼) |

### 4-2. 휠

| ID | 제스처 | 대상 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-422 | 휠 | 파일 목록 | **마우스 아래 패널**을 세로 스크롤(활성 패널이 아님). 노치 = 시스템 줄 수 × 고속 스크롤 배수 · 정밀 터치패드 = 픽셀 | win.rs:7858-7859, win.rs:7920-7927, gui/widgets/rows.rs:1769-1790 | P(휠 줄 수 조회 · 델타 단위) |
| CMD-423 | Shift+휠 | 파일 목록 | 가로 스크롤 | win.rs:7920-7922, gui/widgets/rows.rs:1791-1803 | P(macOS 는 OS 가 이미 가로로 변환) |
| CMD-424 | 틸트(가로) 휠 | 파일 목록 | 가로 스크롤 | win.rs:7931-7956 | N |
| CMD-425 | 휠 | 도크 정보/미리보기 내용 | 내용 세로 스크롤(Shift = 가로 · 틸트 = 가로). 이미지 미리보기는 무동작 | win.rs:7909-7919, win.rs:7946-7953, gui/widgets/dock.rs:756-794 | N |
| CMD-426 | 휠 | 도크 터미널 | 스크롤백 보기(줄 단위 + 고속 배수 · 터치패드 = 픽셀) | win.rs:7872-7903 | N |
| CMD-427 | 휠 | 도크 터미널(TUI 마우스 모드 · SGR 1006) | 휠 버튼 64/65 를 앱에 전달(로컬 스크롤 억제) | win.rs:7874-7881 | N |
| CMD-428 | Shift+휠 · 틸트 휠 | 도크 터미널(줄바꿈 꺼짐) | 터미널 가로 스크롤(노치 = 4열) | win.rs:7863-7871, win.rs:7936-7944 | N |
| CMD-429 | Shift+휠 | 도크 터미널(줄바꿈 **켜짐**) | 터미널이 아니라 **그 패널의 파일 목록이 가로 스크롤**된다(앞 분기 둘 다 불일치 → 목록 폴백. 주석 "Shift = 항상 로컬"과 실제 동작이 다름) | win.rs:7863, win.rs:7872, win.rs:7920-7926 | N(의도 확인 필요) |
| CMD-430 | 휠 / Shift+휠 / 틸트 휠 | 독립 미리보기 창 | 세로 / 가로 / 가로 스크롤 | previewwnd.rs:727-769 | N |
| CMD-431 | 휠 / Shift+휠 | 그리드 컨트롤 · 순서 트리 · 일괄 이름변경 카드 영역 · 설정 창 | 세로 / 가로(그리드만) 스크롤 | ctl/grid.rs:570-600, ctl/ordertree.rs:558, bulkrename.rs:1817-1828, prefs.rs:2570, prefs.rs:2693 | A |

### 4-3. 탭 바 · 경로바 · 네비 바 · 도구 모음 · 메뉴바

| ID | 제스처 | 대상 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-432 | 좌클릭 | 탭 본체 | 그 탭으로 전환 | gui/widgets/tabbar.rs:220-232 | N |
| CMD-433 | 좌클릭 | 탭 × 존 | 탭 닫기(마지막·잠긴 탭은 전환으로 처리) | gui/widgets/tabbar.rs:222-227 | N |
| CMD-434 | 좌클릭 | [+] | 새 탭 | gui/widgets/tabbar.rs:233-236 | N |
| CMD-435 | 좌버튼 드래그 | 탭 본체 | 같은 패널 안 재정렬(임계 8px · 대상 탭 가로 중간점을 넘을 때 스냅 · 여러 줄 탭 포함) | gui/widgets/tabbar.rs:244-272 | N |
| CMD-436 | 좌버튼 드래그 | 탭 → 반대 패널 **탭 바** 위 | 즉시 교차 이동(미리 보기) + 드래그 이양. 뗌 = 확정(보기 옵션 값 채택) · Esc = 원위치 | win.rs:8436-8465, win.rs:8519-8529 | N |
| CMD-437 | 좌버튼 드래그 후 뗌 | 탭 → 반대 패널 **본문** 위 | 뗄 때 반대 패널 끝에 삽입(미리 보기 없음) | win.rs:8530-8547 | N |
| CMD-438 | 더블클릭 | 탭 본체 | 설정 `tab_dblclick`: `close`(기본) / `pin` / `lock` | win.rs:8648-8658, config.rs:291, config.rs:702 | N |
| CMD-439 | 더블클릭 | 탭 바 빈 공간 | 새 탭 | win.rs:8659-8666 | N |
| CMD-440 | 우클릭 | 탭 | 탭 메뉴(§1-4) | gui/widgets/tabbar.rs:238-243, win.rs:8215-8220 | A |
| CMD-441 | 좌클릭 | 경로바 세그먼트 | 그 경로로 이동(마지막 = 현재 세그먼트는 비활성 · hover 강조 없음) | gui/widgets/pathbar.rs:434-439 | N |
| CMD-442 | 우클릭 | 경로바(비편집) | 편집 모드 진입(전체 선택 상태). 이 클릭에서는 편집 메뉴를 띄우지 않음 | gui/widgets/pathbar.rs:441-445, win.rs:8206-8209 | N |
| CMD-443 | 좌클릭 / 드래그 | 경로바 편집 필드 · 이름변경 필드 | 클릭 = 캐럿 배치 · 드래그 = 텍스트 선택 | gui/widgets/pathbar.rs:426-433, gui/widgets/pathbar.rs:447-452, gui/widgets/rows.rs:1738-1762 | N |
| CMD-444 | 좌클릭 | 경로바 편집 중 필드 밖 | 편집 취소(포커스 아웃). 이름변경 중 필드 밖 클릭 = 이름변경 취소 후 정상 클릭 처리 | win.rs:8016-8019, gui/widgets/rows.rs:1746 | N |
| CMD-445 | 좌클릭 | 경로 자동완성 제안 | 그 폴더로 즉시 이동 | win.rs:8001-8007, gui/widgets/pathbar.rs:257-273 | N |
| CMD-446 | 우클릭 | 경로바 편집 필드 · 이름변경 필드 · 도크 텍스트 · 터미널 | 편집 메뉴(§1-8). 터미널이면 키 포커스도 이동하고 진행 중 편집을 정리 | win.rs:8250-8270 | A |
| CMD-447 | 좌클릭 | 네비 버튼 · 도구 모음 버튼 · 런처 버튼 | 명령 실행(비활성 버튼은 클릭·hover 무시) | gui/widgets/chrome.rs:218-241, win.rs:7981-7996 | A |
| CMD-448 | hover 500ms | 도구 모음 버튼 | 툴팁 표시(250ms 틱 × 2). 클릭·이탈 시 즉시 제거 | win.rs:86-90, win.rs:5843-5930, win.rs:7962 | A |
| CMD-449 | 우클릭 | 도구 모음 빈 영역 | "도구 모음 순서... / 설정..." 팝업 | win.rs:8165-8170 | A |
| CMD-450 | 좌클릭 / hover | 메뉴바 | 제목 클릭 = 토글 · 열린 채 다른 제목 hover = 전환 · 하위 메뉴 항목 hover/클릭 = 플라이아웃 · 외부 클릭 = 닫기 | gui/widgets/menubar.rs:229-302 | A |

### 4-4. 스플리터 · 도크 · 터미널 · DnD 수신 · 기타

| ID | 제스처 | 대상 | 동작 | 위치 | 분류 |
|---|---|---|---|---|---|
| CMD-451 | 좌버튼 드래그 | 파일 좌/우 스플리터(히트 반폭 3px · 듀얼일 때만) | 비율 조절(0.1~0.9). 창 50% 와 도크 스플리터 위치에 20px 자석 스냅 · **Alt 유지 = 스냅 해제** | win.rs:8036-8042, win.rs:8404-8413, win.rs:6149-6164 | P(macOS = Option) |
| CMD-452 | 좌버튼 드래그 | 도크 밴드 좌/우 스플리터 · 도크 상단 가로 분리선 | 도크 좌우 비율(0.15~0.85 · 같은 스냅) / 도크 높이 비율(양 패널 공통) | win.rs:8020-8035, win.rs:8383-8403 | N |
| CMD-453 | hover | 스플리터 · 도크 상단 · 컬럼 경계 | 커서 모양 ↔ / ↕ | win.rs:7815-7854 | P(커서 API) |
| CMD-454 | 좌클릭 / 드래그 / Shift+클릭 | 도크 터미널 그리드 | 클릭 = 키 포커스 + 캐럿 깜빡임 시작. 드래그 = 텍스트 선택(그리드 밖 위/아래 유지 시 60ms 간격 자동 스크롤). TUI 마우스 모드면 누름·이동·뗌을 앱에 전달하고 **Shift 를 누르면 로컬 선택**. 우클릭도 TUI 모드에서는 앱에 전달(Shift = 로컬 메뉴) | win.rs:8104-8131, win.rs:8301-8336, win.rs:8185-8199, win.rs:6203-6221 | N |
| CMD-455 | 좌버튼 드래그 / 클릭 | 도크 정보/미리보기 텍스트 | 문자 단위 드래그 선택(가장자리 자동 스크롤) · 이동 없는 클릭 = 선택 해제. 도크 밖 클릭·반대 패널 클릭도 선택 해제 | gui/widgets/dock.rs:671-679, gui/widgets/dock.rs:702-754, panel.rs:1466-1468, win.rs:8074 | N |
| CMD-456 | 파일 드롭(외부·내부) | 파일 목록 | 대상 = 폴더 행이면 그 폴더, 그 외 = 그 패널 현재 폴더. 연산: **Ctrl = 복사 · Shift = 이동 · 기본 = 같은 볼륨이면 이동, 다르면 복사**. 자기 자신/하위로는 거부. 가상 파일(메일 첨부 등)은 항상 복사. 드래그 중 비활성 탭 위에 머물면(`dnd_hover_ms`) 그 탭으로 전환 · 접힌 폴더 위에 머물면 펼침 · 본문 상하단 가장자리 = 자동 스크롤. 좌클릭/우클릭으로 패널 활성화(공유 도크 클릭은 활성 유지) · 공유 도크 더블클릭 = 무동작 | dnd.rs:191-238, win.rs:3252-3268, win.rs:3345-3410, win.rs:8048-8051, win.rs:8615-8618 | P(DnD · 볼륨 판정 · 수식키) |

하위 창의 마우스(요약 — 필드 단위는 각 담당 문서): 그리드 컨트롤 = 클릭 단일 선택 · Shift 범위 · Ctrl 토글 · 헤더 클릭 정렬(Shift = 다중) · 헤더 경계 드래그 폭 조절 · 체크/− 마크 클릭 · 빈 영역 클릭 선택 해제(ctl/grid.rs:665-743). 순서 트리 = 클릭 단일 선택 · Shift 형제 범위 · 체크 상자 클릭 토글 · 셰브론 클릭 접기 · 선택 블록 드래그 이동(같은 부모 안에서만 · 자동 스크롤 · Esc 취소)(ctl/ordertree.rs:7-21, ctl/ordertree.rs:640-697). 독립 미리보기 창 = 좌버튼 드래그 선택(경계 밖 50ms 연속 스크롤) · 우클릭 메뉴(previewwnd.rs:771-826). About 창 = 링크 위 손 커서 · 링크 좌클릭 = 기본 브라우저로 URL 열기(about.rs:126-134, about.rs:150-158, about.rs:164-175).

---

## 5. macOS 관례 대응안 (Linux 는 Windows 프리셋을 기본으로 따름)

기준: nexa-sql 의 단축키 맵 구조를 그대로 가져온다 — 명령 표 한 곳에 `win` / `mac` / `linux` 기본 코드를 두고, 설정 `key.<명령 id>` 로 재정의, `key.preset = auto|windows|macos|linux`(nexa-sql/crates/nexa-sql/src/keymap.rs:1-58). nexa-ui 입력 이벤트는 `ctrl` 대신 **`primary`(macOS = ⌘ · 그 외 = Ctrl)** 한 개만 전달한다(nexa-ui/crates/nexa-ctl/src/event.rs:57-66). 따라서 dir3 는 "명령 id(문자열) ↔ 키 코드" 표를 먼저 만들고, §3 의 if 사슬을 그 표 조회로 바꾼다.

| ID | Windows(현재) | macOS 제안 | 대상 CMD | 비고 |
|---|---|---|---|---|
| CMD-480 | Ctrl+T · Ctrl+W · Ctrl+A · Ctrl+C · Ctrl+X · Ctrl+V · Ctrl+Z · Ctrl+, · Ctrl+Shift+N · Ctrl+Shift+R | ⌘T · ⌘W · ⌘A · ⌘C · ⌘X · ⌘V · ⌘Z · ⌘, · ⇧⌘N · ⇧⌘R | CMD-297~303, 311, 312, 315 | 주 수식키 일괄 치환. 편집 필드 문맥(CMD-263~270, 284, 285)도 같은 규칙 |
| CMD-481 | Ctrl+Y / Ctrl+Shift+Z (다시 실행) | ⇧⌘Z 를 기본, ⌘Y 는 보조 | CMD-304, 305 | 메뉴 표기도 OS 별 기본 코드에서 생성(현재는 "Ctrl+Y" 고정 문자열) |
| CMD-482 | Ctrl+X → Ctrl+V (파일 잘라내기 후 붙여넣기) | ⌘C 후 ⌥⌘V("여기로 이동")를 **추가** 지원, ⌘X 는 유지 | CMD-301, 302 | Finder 는 파일 잘라내기가 없다. dir2 동작(잘라낸 행 흐림)은 유지하되 Finder 식 이동 붙여넣기를 병행 |
| CMD-483 | Delete = 휴지통 · Shift+Delete = 완전 삭제 | ⌘⌫ = 휴지통 · ⌥⌘⌫ = 완전 삭제. `fn+⌫`(앞으로 삭제 키)도 휴지통으로 허용 | CMD-308, 309 | macOS 노트북에는 Delete 키가 없다 |
| CMD-484 | F2 = 이름변경 · Enter = 열기 | F2 유지 + Return = 이름변경 은 **옵션**(기본은 dir2 와 같게 Return = 열기) · ⌘O / ⌘↓ = 열기 | CMD-306, 307, 329 | Finder 관례(Return = 이름변경)와 dir2 관례가 충돌 → 설정 항목으로 결정 필요 |
| CMD-485 | Alt+← / Alt+→ / Alt+↑ / Alt+↓ | ⌘[ / ⌘] / ⌘↑ / ⌘↓ | CMD-326~329 | macOS 에서 ⌥← → 는 편집 필드의 단어 이동이라 내비에 쓰지 않는다 |
| CMD-486 | Ctrl+H(숨김) · Ctrl+.(점 파일) | ⇧⌘. 로 통합 토글(두 값을 함께) | CMD-313, 314 | ⌘H = 앱 숨기기 · ⌘. = 취소라 그대로 못 쓴다. macOS/Linux 는 "숨김 속성"과 "점 파일"이 사실상 같은 개념 → 설정 구조 결정 필요 |
| CMD-487 | Ctrl+Tab(다음 탭) | ⌃Tab 유지 + ⇧⌘] / ⇧⌘[ (다음/이전) | CMD-296 | 이전 탭 명령(⌃⇧Tab)을 **신규 추가** 권장 — dir2 에는 없음 |
| CMD-488 | Tab(패널 전환) | Tab 유지 | CMD-295 | 그대로 |
| CMD-489 | Ctrl+`(도크) | ⌃` 유지(⌘` 는 OS 의 창 순환) | CMD-316 | 주 수식키 치환 **예외** |
| CMD-490 | F5(새로 고침) | ⌘R 기본 + F5 보조 | CMD-291 | macOS 의 F 키는 기본이 미디어 키 |
| CMD-491 | F3(미리보기 창) · F6(테마 순환) | Space 는 선택 토글과 충돌하므로 ⌘Y(훑어보기 관례)를 미리보기에, F6 은 유지하되 메뉴로도 접근 | CMD-292, 293 | 테마 메뉴 3항목 모두에 "F6" 표기 → 순환 명령 하나에만 표기하도록 정리 권장 |
| CMD-492 | Apps 키 · Shift+F10(키보드 컨텍스트 메뉴) | ⌃Return 등 대체 키 지정(Apps 키 없음) + Shift+F10 유지 | CMD-310, 325 | 접근성 필수 — 대체 키는 설정 가능하게 |
| CMD-493 | Ctrl+클릭(비연속 선택) | ⌘+클릭 = 비연속 선택 · ⌃+클릭 = 우클릭(컨텍스트 메뉴)으로 번역 | CMD-402, 415, 416 | winit 은 ⌃+클릭을 좌클릭 + Control 로 전달 → 호스트가 우클릭으로 변환해야 한다 |
| CMD-494 | 드롭: Ctrl = 복사 · Shift = 이동 | ⌥ = 복사 · ⌘ = 이동(Finder 관례). 기본 규칙(같은 볼륨 = 이동)은 동일 | CMD-456 | Linux 는 Ctrl = 복사 · Shift = 이동 그대로 |
| CMD-495 | Alt + 스플리터 드래그(스냅 해제) | ⌥ + 드래그 | CMD-451 | 단순 치환 |
| CMD-496 | 파일 ▸ 종료 · 파일 ▸ 설정 · 도움말 ▸ 정보 | ⌘Q · ⌘, · "Nexa Dir 정보" — 창 안 메뉴바(자체 그리기)에 그대로 두되 단축키만 OS 관례로 | CMD-005, 006, 042 | OS 메뉴 막대를 쓰지 않는 기조(전부 직접 그림)를 유지할 때의 안. ⌘Q 는 창 닫기 저장 경로(CMD-122)와 같아야 한다 |
| CMD-497 | 터미널 Ctrl+C(선택 있으면 복사 · 없으면 인터럽트) · Ctrl+V | macOS: ⌘C / ⌘V = 복사/붙여넣기, ⌃C = **항상** 인터럽트. Linux: Ctrl+Shift+C / Ctrl+Shift+V = 복사/붙여넣기, Ctrl+C = 항상 인터럽트(옵션으로 dir2 방식 허용) | CMD-279, 280 | 터미널 안에서는 Control 을 주 수식키로 치환하면 안 된다(셸 제어 문자). Backspace 교차 매핑(CMD-278)은 ConPTY 전용 규약이라 유닉스 PTY 에서는 `0x7F` 그대로 |
| CMD-498 | 마우스 X버튼 1/2(뒤로/앞으로) | 동일 지원 + 트랙패드 두 손가락 좌우 쓸기는 **후속**(winit 지원 범위 확인 후) | CMD-421 | macOS 마우스는 X버튼이 드물다 |
| CMD-499 | Shift+휠 = 가로 스크롤 · 정밀 터치패드 = 노치 미만 델타를 픽셀로 | OS 가 이미 Shift+휠을 가로 델타로 바꿔 주므로 **이중 변환 금지**. 픽셀 델타는 그대로 픽셀 스크롤, 관성 구간에는 고속 스크롤 배수를 적용하지 않음 | CMD-422~428 | 휠 줄 수(Windows 시스템 설정 조회 — win.rs:6327-6344)는 타 OS 에서 고정 기본값 3 또는 설정값 |

---

## 6. 부록 — OS 분기점 · nexa-ui 공백 · 위험

### 6-1. 이 범위에서 확인된 OS 분기점

| 분기점 | Windows(현재) | macOS · Linux 에서 필요한 것 | 근거 |
|---|---|---|---|
| 행/배경 컨텍스트 메뉴 | 셸 `IContextMenu` 호스팅(설치 프로그램 확장 포함) + 동사 가로채기 | OS 셸 메뉴 호스팅 수단이 없음 → nexa-ui 컨텍스트 메뉴로 **앱 고유 메뉴를 전부 직접 구성**(열기 · 연결 프로그램 · 잘라내기/복사/붙여넣기 · 이름 바꾸기 · 삭제 · 새로 만들기 · 속성/정보 가져오기 등). Windows 도 같은 메뉴를 기본으로 하고 셸 메뉴는 "더 보기"로 붙이는 안 검토 | shellmenu.rs:1-11, win.rs:2817-3140 |
| 새로 만들기 하위 메뉴 | 셸 ShellNew 템플릿 | 고정 목록(폴더 · 빈 텍스트 파일) 또는 Linux `~/Templates` | shellmenu.rs:757-815 |
| 팝업 메뉴 전부 | `TrackPopupMenuEx`(모달 · 동기 반환) | nexa-ui 컨텍스트 메뉴(비모달 · 이벤트 수거) — 호출부가 "표시 후 결과 수거" 2단계로 바뀐다 | win.rs:7166, win.rs:7568, win.rs:6282, win.rs:3233, previewwnd.rs:365, prefs.rs:2311 |
| 키 입력 모델 | `WM_KEYDOWN`(가상 키) + `WM_CHAR`(문자 · 제어 문자 포함) + `WM_SYSKEYDOWN`(Alt · F10) | winit 은 키 이벤트 하나에 논리 키 · 텍스트 · 수식키가 함께 온다 → 터미널 제어 문자(Ctrl+문자 → 0x01~0x1A) 생성과 Alt 조합을 호스트가 직접 만들어야 한다 | win.rs:8694, win.rs:8905, win.rs:9331 |
| 더블클릭 · 느린 재클릭 | OS 가 `WM_LBUTTONDBLCLK` 를 주고, 지연은 `GetDoubleClickTime()` | winit 에는 더블클릭 이벤트가 없음 → 시간·거리로 직접 판정, 간격은 설정값 | win.rs:8605, win.rs:8580 |
| 파일 열기 · URL 열기 | `ShellExecuteW("open")` + 포그라운드 권한 양도 | macOS `open` · Linux `xdg-open`(권한 양도 개념 없음) | win.rs:7097-7110, win.rs:5460-5471 |
| 휴지통 · 완전 삭제 확인 | `SHFileOperationW` · `MessageBoxW` | OS 별 휴지통 API + nexa-ui 대화상자 | win.rs:3182-3200, win.rs:3738 |
| 클립보드(파일 목록 · 잘라내기 표시 · rich 텍스트) | CF_HDROP · 클립보드 변경 통지 · HTML/RTF | OS 별 파일 URL 목록 형식 · "잘라내기" 표식의 OS 별 표현 · 변경 통지 유무 | win.rs:7355-7382, win.rs:9691-9700 |
| DnD | OLE `DoDragDrop`/`IDropTarget` · `MK_CONTROL`/`MK_SHIFT` | OS 별 드래그 세션 + 수식키 관례(CMD-494) | dnd.rs:191-263, win.rs:8359-8377 |
| 런처 · 터미널 셸 | pwsh → powershell → cmd | 로그인 셸(`$SHELL`) · 런처 시드도 OS 별 | nexa-dir2/crates/nexa-app/src/conpty.rs:281-300, nexa-dir2/crates/nexa-app/src/launcher.rs:47-72 |
| 커서 모양 · 캐럿 깜빡임 주기 · 휠 줄 수 | `LoadCursorW` · `GetCaretBlinkTime` · `SPI_GETWHEELSCROLLLINES` | winit 커서 아이콘 · 고정 기본값(530ms · 3줄) 또는 설정 | win.rs:7846-7849, win.rs:6190-6197, win.rs:6327-6344 |
| "내 PC" 홈 · 경로 문법 | 가상 루트 `::PC::` · 드라이브 문자 · `%VAR%` · `shell:` | macOS = 볼륨 목록(`/Volumes`) · Linux = 마운트 목록 · `~` 와 `$VAR` 확장 | panel.rs:1011-1023, panel.rs:1526-1531, gui/widgets/pathbar.rs:25-52 |

### 6-2. nexa-ui 에 없거나 보강이 필요한 것(이 범위에서 확인한 것만)

| 항목 | 현재 nexa-ui | 필요 |
|---|---|---|
| 메뉴바 항목의 체크/라디오 표시 · 오른쪽 단축키 문구 | `MenuEntry` = `Item` / `Emph` / `Disabled` / `Separator` / `Sub` 이고 항목 데이터(`ComboItem`)에 체크·단축키 필드가 없다(nexa-ui/crates/nexa-ctl/src/controls/pulldown.rs:25-36, nexa-ui/crates/nexa-ctl/src/controls/combo.rs:44-53) | dir2 메뉴는 체크/라디오 17종(+ 동적 언어 목록) + 단축키 표기 17건 → 메뉴바 항목에 체크 상태와 단축키 칸 추가(컨텍스트 메뉴 `CtxItem` 에는 이미 있음 — nexa-ui/crates/nexa-ctl/src/controls/ctxmenu.rs:101-131) |
| 도구 모음 버튼의 토글(켜짐) 표시 | `ToolItem` 에 켜짐 상태 필드 없음 · 색조(`ToolTone`)만 있음(nexa-ui/crates/nexa-ctl/src/controls/toolbar.rs:66-122) | 토글 버튼 11종(CMD-044~054) → 켜짐 배경 표시 추가 또는 색조로 대체할지 결정 |
| 입력 이벤트의 버튼·수식키 구분 | 좌/우 누름과 `shift` · `primary` 만 있다. 가운데 버튼 · X버튼 · 더블클릭 · Alt 가 없다(nexa-ui/crates/nexa-ctl/src/event.rs:47-115) | 호스트(dir3 플랫폼 계층)가 X버튼 · 더블클릭 · Alt 스냅 해제 · ⌃+클릭을 직접 처리. 탭 가운데 클릭 닫기는 nexa-ui 탭 바가 별도 호출을 제공(nexa-ui/crates/nexa-ctl/src/controls/tabbar.rs:466) — dir2 에는 없던 기능이라 추가 여부 결정 |
| 경로바(브레드크럼 + 편집 + 자동완성 팝업) | `controls/` 에 해당 파일 없음(목록 확인: button · carousel · checkbox · colorpanel · colorpick · combo · ctxmenu · editmenu · flash · glyphs · icondrop · listedit · pairs · posdrop · posgrid · pulldown · radio · scroll · splitter · switch · tabbar · textbox · timeout_button · toolbar · tooldock · tree) | 신규 추가(CMD-260~271, 441~445) |
| 정보/미리보기/터미널 도크(종류 스트립 + 텍스트 선택 + ↗ 버튼) | 해당 컨트롤 없음(`tooldock` 은 다른 용도 — 추정) | 신규 추가(CMD-105~109, 425, 455) |
| 순서 편집 트리 · 세그먼트 · 스핀 · 검색 상자 · 메뉴 버튼 · 글꼴 상자 | `controls/` 에 같은 이름 파일 없음(다른 이름으로 대응 컨트롤이 있을 수 있음 — **추정**) | 설정 창·일괄 이름변경 담당 문서에서 확정 |

### 6-3. 위험 · 결정 필요

1. **단축키가 한곳에 정의되어 있지 않다.** 메뉴 표기 문자열과 실제 키 처리가 따로라서 이미 불일치가 있다 — 다시 실행은 Ctrl+Shift+Z 도 되지만 메뉴에는 Ctrl+Y 만, 테마 3항목 모두 "F6" 표기, Ctrl+Tab 은 Shift 를 눌러도 다음 탭. dir3 는 명령 표 하나에서 메뉴 표기와 키 처리를 함께 만들어야 회귀를 막는다.
2. **터미널 포커스 중 전역 단축키가 죽는다**(F5 · Ctrl+T · Tab · Ctrl+` 포함 — 도크를 키보드로 닫을 수 없다). 반면 Alt+방향키 · Shift+F10 은 터미널 포커스 중에도 파일 패널에 작용한다. 그대로 계승할지 정리할지 결정 필요.
3. **메뉴 항목 비활성 표시가 없다.** dir2 는 눌러 본 뒤 상태 줄로 알린다(탭 닫기 · 정보 패널 모드 · 실행 취소 · 일괄 이름변경 · 링크 추가). nexa-ui 메뉴는 비활성을 지원하므로 조건을 메뉴 구성 시점으로 옮길 수 있으나, "기능 그대로" 원칙과의 선택이 필요하다.
4. **컨텍스트 메뉴의 대부분이 OS 셸이 주는 항목이다.** macOS · Linux 에서는 열기 · 연결 프로그램 · 속성 등을 전부 직접 만들어야 하며, 항목 목록 자체가 새 설계 대상이다(dir2 고유 항목은 7개뿐).
5. **명령 ID 가 숫자 대역 방식**(200+i · 300+i …)이다. nexa-ui 는 문자열 id 라 `cloud.goto:3` 같은 매핑 규칙을 먼저 정해야 한다.
6. **죽은 명령**: `CMD_NAV_BACK` / `CMD_NAV_FORWARD` / `CMD_NAV_UP` 은 어디에서도 발화하지 않는다. `CMD_CTLDEMO` · Shift+F3 벤치는 개발 전용. 이식 제외 여부 확정 필요.
7. **Shift+휠의 터미널 동작 불일치**(CMD-429): 줄바꿈이 켜진 터미널 위 Shift+휠이 파일 목록을 가로 스크롤한다. 의도인지 확인 필요.
8. **가운데 클릭 · Ctrl+휠 · 이전 탭 · 미리보기 창 Ctrl+A · 메뉴바 키보드 접근**은 dir2 에 없다. dir3 에서 추가하면 "기능 유지" 범위를 넘으므로 별도 결정 사항으로 둔다.
9. 설정 창은 Esc 로 닫히지 않고 Tab 이동도 없다(CMD-341). nexa-ui 대화상자 관례와 다를 수 있다.
10. 일괄 이름변경 창의 Enter/Esc 동작, `copyaspath` 동사가 나오는 조건, nexa-ui 에 다른 이름으로 대응 컨트롤이 있는지(순서 트리 · 세그먼트 · 스핀 등)는 코드로 확정하지 못해 "추정"으로 남겼다.
