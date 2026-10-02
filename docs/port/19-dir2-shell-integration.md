# 19 · nexa-dir2 인벤토리 — OS 셸 통합 (컨텍스트 메뉴 · 클립보드 · DnD · 휴지통 · 감시 · 파일 속성 · 접근성 · 비밀)

> 성격: **이해(인벤토리) 단계 산출물** — 읽기 전용 조사 결과. 이 문서의 `SHELL-NNN` ID가 이후 구현·교차 검증의 체크리스트다.
> 조사 기준일 2026-10-03 · 원본 = nexa-dir2 `0.22.0`(Windows 전용) · 대상 = nexa-dir3(Windows·macOS·Linux).
>
> **경로 약칭**(모든 근거는 `저장소/경로:줄`):
> `app/` = `nexa-dir2/crates/nexa-app/src/` · `core/` = `nexa-dir2/crates/nexa-core/src/` · `ex/` = `nexa-dir2/crates/nexa-app/examples/` ·
> `ops/` = `nexa-dir2/crates/nexa-ops/src/` · `ui-fs/` = `nexa-ui/crates/nexa-fs/src/` · `ui-ctl/` = `nexa-ui/crates/nexa-ctl/src/` ·
> `sql/` = `nexa-sql/crates/nexa-sql/src/`.
>
> **"추정" 표기**: 저장소 코드로 확인하지 못한 것(특히 §4의 macOS·Linux API·규격은 저장소 밖 지식이라 **구현 시 실측 필요**)에는 "추정"을 붙였다.

---

## 0. 범위 — 읽은 파일과 줄 수

### 0-1. 담당 범위(전부 끝까지 읽음)

| 파일 | 줄 | 내용 |
| --- | ---: | --- |
| `app/shellmenu.rs` | 897 | 클래식 `IContextMenu` 호스팅(구축·표시·동사 가로채기·New 서브메뉴·메시지 포워딩·계측) |
| `app/menuthread.rs` | 353 | 전용 메뉴 STA 스레드(선행 구축·표시·결과 통지) |
| `app/shellnotify.rs` | 93 | `SHChangeNotifyRegister` 셸 변경 통지 구독 |
| `app/clipboard.rs` | 1,387 | 파일 목록(CF_HDROP)·잘라내기 표시·가상 파일(FileGroupDescriptorW)·텍스트/RTF/HTML |
| `app/dnd.rs` | 632 | OLE DnD 수신(`IDropTarget`)·발신(`DoDragDrop`)·지연 렌더링 소스 스테이징 |
| `app/recycle.rs` | 224 | 휴지통 복원(삭제 undo — 셸 `undelete` 동사) |
| `app/fileinfo.rs` | 496 | 파일 기본 정보·디스크 할당 크기·속성 시스템 상세·상세 워커 |
| `app/uia.rs` | 391 | UI Automation 서버측 프로바이더(리스트/행) |
| `app/watcher.rs` | 235 | `ReadDirectoryChangesW` 폴더 감시 |
| `app/fsprobe.rs` | 257 | 폴더 변경 저비용 프로브(서명 비교) — **`std::fs`만 사용(전 플랫폼)** |
| `app/secret.rs` | 255 | 클라우드 refresh 토큰 DPAPI 보관 |
| `core/lib.rs` | 35 | `CORE_VERSION`·`FileKind` |
| `core/secret.rs` | 164 | 세션 한정 비밀 타입 `Secret`(소거·비노출) |
| `ex/ctxmenu_handlers.rs` | 149 | 셸 확장 핸들러별 프로파일러(진단 예제) |
| `ex/ctxmenu_probe.rs` | 188 | 셸 메뉴 격리 재현기·단계 타이밍(진단 예제) |
| `ex/preview_image.rs` | 203 | WIC 이미지 미리보기 독립 예제 |
| `nexa-dir2/docs/08-adr-0003-shell-context-menu.md` | 44 | ADR-0003(필독) |

> 주의: 작업 지시의 `crates/nexa-app/src/examples/*`는 **존재하지 않는다**. 실제 위치는 `nexa-dir2/crates/nexa-app/examples/`(3개 파일).

### 0-2. 진입점 확인을 위해 부분 열람한 파일(범위 밖 — 호출부만)

| 파일 | 열람 구간(줄) | 목적 |
| --- | --- | --- |
| `app/win.rs`(9,951) | 96–184 · 1738–1800 · 2036–2345 · 2476–2520 · 2736–3710 · 3716–3985 · 4122–4165 · 7076–7115 · 7208–7400 · 7726–7800 · 8158–8380 · 8868–8952 · 9140–9325 · 9455–9495 · 9608–9625 · 9688–9712 | 셸 통합 모듈의 호출부·타이머·WM_APP 메시지·단축키 |
| `app/config.rs` | 99–105 · 182–183 · 269–271 · 313 · 362–372 · 654–695 · 985–1015 · 1074–1092 | 설정 키·`CTXMENU_BLOCKS`·`data_dir`·원자 저장 |
| `app/main.rs` | 1–103 | 모듈별 `cfg(windows)` 게이팅 |
| `ops/lib.rs` | 45–100 · 398 | `same_volume`·`is_same_or_sub`·`unique_dest` 등 |
| `nexa-dir2/crates/nexa-app/lang/ko.lang` | `ctx.*`·`del.*`·`info.*`·`undo.*` 키 | 문구 자원 |
| `nexa-dir2/docs/23-cross-platform-feasibility.md` | 249(전체) | 기존 크로스플랫폼 검토 |
| `nexa-dir2/docs/audit/20261002-ultracode/ctxmenu-latency.md` | 79(전체) | 우클릭 지연 실측·메뉴 스레드 설계 근거 |

### 0-3. 매핑 대상(nexa-ui · nexa-sql)

| 파일 | 줄 | 확인 결과 요약 |
| --- | ---: | --- |
| `ui-fs/shell.rs` | 1,029 | OS 아이콘/종류 이름(**Windows만 구현**, 그 외 `None`)·`IconService`·`resolve_alias`·`reveal_in_file_manager`(3-OS) |
| `ui-fs/watch.rs` | 325 | **파일 단위** 서명 감시 `StatWatch`(폴더 감시 아님) |
| `ui-fs/lister.rs` | 284 | 백그라운드 폴더 열거 `ListHandle`(배치·프로브·취소) |
| `ui-fs/lib.rs` | 957 | 개요만 Grep(`Entry`·`places`·`drives`·`naming`·`path`·`local_time` 등) |
| `ui-ctl/controls/ctxmenu.rs` | 2,145 | 자체 그림 `ContextMenu`/`CtxItem`(아이콘·단축키·하위 메뉴·스크롤) — 95–334·520–562 열람 + 공개 API Grep |
| `ui-ctl/controls/editmenu.rs` | 256 | 편집 우클릭 메뉴 `EditMenu`(공개 API Grep) |
| `sql/clipboard.rs` | 299 | **텍스트·HTML만**(파일 목록 없음). Win=user32 직접 · mac=`pbcopy`/`osascript` · Linux=X11 직접→CLI 폴백 |
| `sql/clipboard_x11.rs` | 388 | X11 `CLIPBOARD` selection 직접 구현(텍스트 타깃·INCR) |
| `sql/drop_win.rs` | 416 | **파일 DnD가 아니다** — DB 객체 DROP 확인 모달 창(`TimeoutButton` 2단 확인) |
| `sql/app/drop.rs` | 501 | **파일 DnD가 아니다** — DB 객체 삭제 흐름(백업 후 DROP) |
| `sql/app/event_loop.rs` | 598–603 | 파일 드롭 수신 = winit `WindowEvent::DroppedFile`(경로만 · 좌표/수정키 없음) → `open_file` |

---

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지(타 OS는 대체·비활성).

### 1-A. 셸 컨텍스트 메뉴

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-001 | 행 우클릭 → 탐색기 "더 많은 옵션"과 같은 셸 메뉴(7-Zip·Git·보내기·연결 프로그램·속성) | 경로→PIDL(`SHParseDisplayName`, 실패 항목은 제외)→공통 부모 `IShellFolder`(`SHBindToParent`, 첫 폴더만 유지)→`GetUIObjectOf::<IContextMenu>`→`QueryContextMenu(hmenu,0,1,0x6FFF,flags)`→`TrackPopupMenuEx(TPM_RETURNCMD\|TPM_RIGHTBUTTON)`. 빈 경로·전 항목 파싱 실패 = `Cancelled`. 표시 전 `SetForegroundWindow`, 종료 후 `PostMessage(WM_NULL)` | `app/shellmenu.rs:271`(show) · `:328`(prepare_items) · `:626`(track) · 호출 `app/win.rs:3037` | `IContextMenu`·`IShellFolder`·HMENU·`TrackPopupMenuEx`·COM STA | **P+A**(메뉴 그리기=A · 항목 공급=P) | 없음(표시는 실기) |
| SHELL-002 | 다중 선택 우클릭 — 교차 폴더 선택은 클릭(캐럿) 항목의 폴더 기준으로 축소 | 대상 = 선택(없으면 캐럿) 중 `parent == 캐럿 항목의 parent`만. 캐럿 없음/부모 없음 = 축소 없이 전체. `GetUIObjectOf`가 단일 부모만 표현하기 때문(ADR-0003 §다중 선택) | `app/win.rs:2781`(context_targets) · `:2739`(keyboard_targets) | — | **N** | 없음 |
| SHELL-003 | 키보드로 메뉴 열기 — Apps 키 / Shift+F10 | 캐럿 행 앵커(`row_anchor`)를 화면 좌표로 변환해 그 위치에 표시(`at=Some`). 마우스는 커서 위치(`at=None`→`GetCursorPos`). 앵커 없으면 표시 안 함 | `app/win.rs:8876`(VK_APPS) · `:8910`(Shift+F10, WM_SYSKEYDOWN) · `:2942-2956` | `ClientToScreen`·`GetCursorPos` | **A** | 없음 |
| SHELL-004 | Shift+우클릭 = 확장 동사 | `GetKeyState(VK_SHIFT)<0` → `CMF_EXTENDEDVERBS`, 아니면 `CMF_NORMAL`. 선행 구축분 재사용 판정에도 포함(`same_menu`) | `app/shellmenu.rs:515-519` · `app/win.rs:3019` | `CMF_EXTENDEDVERBS` | **P**(Win 전용 개념 · 타 OS=자체 "확장 항목" 표시 여부) | `app/menuthread.rs:338`(same_menu) |
| SHELL-005 | 삭제·이름 바꾸기·복사·잘라내기는 **앱 경로로 실행**(undo 기록·인라인 리네임·교차 폴더 선택 지원) | 선택한 셸 항목의 canonical verb(`GetCommandString(GCS_VERBW)`)가 `intercept` 목록(행 메뉴 = `delete`,`rename`,`copy`,`cut` / 배경 메뉴 = `paste`)과 대소문자 무시 일치하면 셸 실행 대신 `Outcome::Verb`. verb 조회 실패한 확장은 가로채기 없이 셸 실행. delete=`do_delete(permanent=Shift 열림 여부)` · rename=인라인 리네임 · copy/cut=**축소 전 전체 선택**(`req.full`)을 클립보드에 | `app/shellmenu.rs:683-689` · `:843`(get_verb) · `app/win.rs:3020-3023` · `:3086-3104` | `IContextMenu::GetCommandString` | **P**(타 OS는 자체 메뉴라 가로채기 불필요 — 처음부터 앱 명령) | 없음 |
| SHELL-006 | "경로 복사"는 셸 항목 자리에 그대로 두되 앱이 실행(교차 폴더 전체 선택 복사) | `hide=[("copyaspath", CTX_COPY_PATH, tr("ctx.copyPath"))]` — 셸 대역 항목 중 verb 일치 항목의 **ID만 고유 ID로 교체 + 라벨을 앱 언어로**(`SetMenuItemInfoW(MIIM_ID\|MIIM_STRING)`), 위치 유지. 선택 시 `Outcome::Custom(id)` | `app/shellmenu.rs:527-558` · `app/win.rs:3024` | `GetMenuItemID`·`SetMenuItemInfoW` | **P** | `app/menuthread.rs:338`(hide 비교) |
| SHELL-007 | 앱 고유 항목 병합 — 완전 삭제 / 이름 복사 / 폴더에 붙여넣기 | 고유 ID 0x8000+. `after_id` 지정 항목은 그 ID 항목 **바로 아래** 삽입(이름 복사 → 경로 복사 아래), 앵커 부재·미지정은 **구분자 + 하단 섹션**. `enabled=false`는 `MF_GRAYED`. "폴더에 붙여넣기"는 단일 선택이 폴더이고 클립보드에 파일이 있을 때만 **추가**(없으면 항목 자체 없음) | `app/shellmenu.rs:560-600` · `app/win.rs:2957-3014` · ID `app/win.rs:2804-2812` | `InsertMenuW`·`AppendMenuW` | **A** | `app/menuthread.rs:338` |
| SHELL-008 | 항목 메뉴 하단 "새로 만들기 ▸" 서브메뉴(폴더·바로가기·txt·docx… ShellNew 템플릿 전체) + 생성 직후 인라인 이름 바꾸기 | 단일 선택일 때만. 대상 폴더 = 폴더 항목=자신 / 파일 항목=부모. `CoCreateInstance(CLSID_NewMenu {D969A300-E7FF-11D0-A93B-00A0C90F2719})`→`IShellExtInit::Initialize(pidl)`→구분자 + `QueryContextMenu(pos, 0x7000, 0x7FFF)`→최상위 라벨만 앱 언어로 교체(서브 항목은 OS 로케일). 실패는 조용히 생략(구분자 회수). 선택 시 New 확장 ICM으로 invoke → 폴더 이름 전후 diff로 **정확히 1개** 신규일 때 `Outcome::Created(path)`(20ms×10 재시도), 아니면 `Shell` | `app/shellmenu.rs:757`(attach_new_menu) · `:664-678` · `:816`(dir_names) · `:824`(detect_created) · `app/win.rs:2976-2989` · `:4127`(focus_created_and_rename) | `CLSID_NewMenu`·`IShellExtInit` | **P**(Win=ShellNew · mac/Linux=자체 템플릿 목록) | 없음 |
| SHELL-009 | 빈 본문 우클릭 = 폴더 **배경** 메뉴(보기·새로 만들기·붙여넣기·속성 등) + 고유 붙여넣기/실행 취소/다시 실행 | `SHGetDesktopFolder`→`BindToObject(pidl)`→`CreateViewObject::<IContextMenu>`. 고유 항목: `paste`(클립보드에 파일 있을 때만 활성)·`undo`(라벨 = "실행 취소: {설명}" 또는 기본, `can_undo`로 활성)·`redo` 동일. `paste` 동사 가로채기 → OS 클립보드 읽어 전송 엔진 합류(Move면 클립보드 비움), 실경로 없으면 가상 파일 폴백. 셸 실행 후 폴더 diff(20ms×2)로 1개 신규면 `Created`. **UI 스레드 동기 경로**(메뉴 스레드 미사용) | `app/shellmenu.rs:401`(show_background) · `:692-703` · `app/win.rs:2817-2912` | `IShellFolder::CreateViewObject` | **P+A** | 없음 |
| SHELL-010 | 내 PC(가상 루트 `::PC::`) 빈 영역 우클릭 = 클라우드 연결 메뉴 | 실경로가 없어 셸 배경 메뉴 불가 → 자체 팝업: 후보(`"{0} 링크"`, 선택=연결) → 연결됨(체크+회색) → 둘 다 없으면 안내 1줄(회색). 최대 `CLOUD_MAX`개. 선택 → `CMD_CLOUD_ADD_BASE+i` | `app/win.rs:2821-2826` · `:3205-3249` | `CreatePopupMenu`·`TrackPopupMenuEx` | **A** | 없음 |
| SHELL-011 | 셸 확장의 동적 서브메뉴·아이콘이 정상 표시됨(보내기·연결 프로그램 등) | wndproc이 `WM_INITMENUPOPUP/DRAWITEM/MEASUREITEM/MENUCHAR`를 활성 핸들러에 포워딩. **선별 라우팅**: INITMENUPOPUP = 열리는 HMENU를 소유한 핸들러 / DRAW·MEASURE = itemID 대역 소유 핸들러 / 미매치 = 주 메뉴 핸들러(`submenu==0`) / MENUCHAR만 전 핸들러 순회(첫 비0 응답). ICM3 우선(`HandleMenuMsg2`), 없으면 ICM2. `ACTIVE`는 thread_local(메뉴를 표시하는 스레드 전용) | `app/shellmenu.rs:209`(forward_menu_msg) · `:190-205` · `app/win.rs:8286-8295` · `app/menuthread.rs:258-267` | `IContextMenu2/3::HandleMenuMsg(2)` | **W** | `app/shellmenu.rs:874`·`:892` |
| SHELL-012 | 셸 명령 실행 후 목록 자동 갱신 + 연 프로그램이 앞으로 옴 | `InvokeCommand(CMINVOKECOMMANDINFOEX{fMask=CMIC_MASK_UNICODE(0x4000)\|CMIC_MASK_PTINVOKE(0x20000000), lpVerb=MAKEINTRESOURCE(offset), nShow=SW_SHOWNORMAL, ptInvoke})`. 직전 `AllowSetForegroundWindow(ASFW_ANY)`. 성공=`Outcome::Shell`→`reload_both` / 실패=`Cancelled`(확장 실패 격리) | `app/shellmenu.rs:732`(invoke) · `app/win.rs:7091` · `:3083` | `InvokeCommand`·`AllowSetForegroundWindow` | **W** | 없음 |
| SHELL-013 | (내부) 명령 ID 대역 분리 | 셸 `1..=0x6FFF` · New 확장 `0x7000..=0x7FFF` · 고유 `0x8000+`(`ID_CUSTOM_FIRST`). 고유 ID: DELETE_PERMANENT=+0 · PASTE_INTO=+1 · UNDO=+2 · REDO=+3 · PASTE_BG=+4 · COPY_PATH=+5 · COPY_NAME=+6 | `app/shellmenu.rs:37-43` · `app/win.rs:2804-2812` | — | **N**(자체 메뉴에서는 문자열 id로 대체 가능) | — |
| SHELL-014 | 우클릭해도 앱이 멈추지 않음 — 전용 메뉴 스레드 | 스레드 `nexa-ctxmenu`(STA, 프로세스 수명 데몬)가 숨은 소유자 창(`NexaDirMenuHost`, `WS_EX_TOOLWINDOW\|WS_POPUP`, 0×0)을 갖고 구축→표시→포워딩→명령 실행 전 생애 담당(`IContextMenu`는 프록시 미등록이라 아파트 밖으로 못 넘김). 표시 중 `AttachThreadInput(메뉴, UI)` + `SetForegroundWindow(주 창)`, 종료 후 `SetFocus(주 창)`+분리. 결과 = `PostMessage(주 창, WM_APP_CTXMENU_RESULT, gen, Box<(MenuReq, Outcome)>)`, **세대 일치 시에만 반영**. 기동 실패 = `None` → 동기 폴백 | `app/menuthread.rs:93`(spawn) · `:162`(create_host_window) · `:193`(host_proc) · `:290`(show_attached) · `app/win.rs:7684` · `:9144-9160` | `AttachThreadInput`·`CreateWindowExW`·메시지 루프 | **W**(타 OS는 메뉴 구축이 가벼워 불필요 — 단 "연결 프로그램 목록" 조회는 워커화) | 없음 |
| SHELL-015 | 우클릭 메뉴가 즉시 뜸 — 선행 구축 | ① 선택이 300ms(`CTX_PREBUILD_MS`) 머물면 타이머가 `prepare` ② 우클릭 **누름** 즉시 `prepare` ③ 메뉴 닫힌 직후 같은 선택으로 재구축. 준비분은 1개만 보관. 표시 요청 시 `same_menu`(targets·extended·New 대상 dir·custom(id/label/enabled)·hide 동일)이면 재사용, 아니면 그때 구축. 대상 없음 → `invalidate`. `ctx_showing` 중엔 무장·구축 안 함 | `app/win.rs:3144`(arm_ctx_prebuild) · `:3163`(prebuild_ctx_menu) · `:8213`(누름) · `:9486`(타이머) · `:9158`(닫힌 뒤) · `app/menuthread.rs:54`(same_menu) · `:137-159` | `SetTimer`·`PostMessage` | **W** | `app/menuthread.rs:338` |
| SHELL-016 | (내부) 메뉴 스레드 부재 시 동기 폴백 | `shellmenu::show`를 UI 스레드에서 직접(모달 펌프). 모달 진입 전 State 참조 종료 규약 | `app/win.rs:3053-3071` | — | **W** | 없음 |
| SHELL-017 | 우클릭 판정 순서·억제 규칙 | **누름**: 도구 모음 빈 영역→막대 팝업 / 컬럼 헤더→컬럼 팝업 / 지연 리네임 취소 / TUI 마우스 모드 터미널(Shift 아닐 때)→앱 전달 / 패널 활성화+선택 규약 반영(`RightDown`) / 탭 메뉴. **뗌**: TUI 릴리스면 메뉴 억제 → 누름 짝 없으면(`rbutton_down_seen=false`) 무시 → 편집 진입 클릭이면 무시 → 텍스트 편집 대상(경로바·리네임·도크 텍스트·터미널)이면 편집 팝업 → 활성 패널의 행 위=행 메뉴 / 본문 빈 곳=배경 메뉴 | `app/win.rs:8161-8222` · `:8223-8282` | — | **N**(로직) + **A**(팝업) | 없음(실기) |
| SHELL-018 | (진단) 우클릭 지연 단계 계측 | `NEXA_CTX_TIMING=1`일 때만. 단계: `parse+bind`·`GetUIObjectOf`·`QueryContextMenu`·`verbs`·`custom+NewMenu`·`pre-track`·`TrackPopupMenuEx(user)` + 포워딩 누적(건수·ms·첫 포워딩 시각)을 `%TEMP%\nexa-ctxmenu-timing.log`에 한 줄 append. 꺼져 있으면 분기 1개 | `app/shellmenu.rs:109-188` | 환경 변수·임시 파일 | **N**(단계 이름만 OS별) | 없음 |
| SHELL-019 | 고유 메뉴 항목의 순서·표시를 설정에서 편집 | 설정 `ctx_menu_order`(문법 `블록:vis[자식:vis,…]\|…`). 블록 `row`=[`new`,`deletePermanent`,`copyName`,`pasteInto`] · `bg`=[`paste`,`undo`,`redo`]. 블록 vis=0이면 그 그룹 고유 항목 전부 제외. `new`는 표시 여부만(위치는 하단 고정), `copyName`은 after_id가 순서보다 우선 | `app/config.rs:1078-1082` · `:183` · `:313` · `:693-695` · `app/win.rs:2839-2845` · `:2960-2966` | — | **N**(설정 구조는 nexa-sql 방식으로 이관) | `app/config.rs:1294`·`:1478`(왕복) |
| SHELL-020 | 고유 항목 동작 — 경로 복사 / 이름 복사 / 완전 삭제 / 폴더에 붙여넣기 | 경로 복사 = **표시 순서**의 전체 선택(교차 폴더 포함)을 `\r\n` 구분 텍스트로. 이름 복사 = 파일/폴더 이름만 `\r\n`. 완전 삭제 = `do_delete(permanent=true)`(확인창). 폴더에 붙여넣기 = `paste_dir`로 전송(Move면 클립보드 비움 · 실경로 없으면 가상 파일 폴백) | `app/win.rs:3105-3137` · `:2759`(display_order_targets) | 클립보드 | **N**(줄 구분자는 OS별 결정 필요 — §4-9) | 없음 |
| SHELL-021 | (내부) 메뉴 닫힘 직후 포워딩 해제 | `TrackPopupMenuEx` 반환 직후 `ACTIVE` 비움 — 이후 `InvokeCommand`가 띄우는 모달 UI의 **남의 HMENU** 메시지가 주 메뉴 핸들러로 흘러가는 것 방지(G8-20). 끝에서 한 번 더 비움(멱등) | `app/shellmenu.rs:654-660` · `:707` | — | **W** | `app/shellmenu.rs:874` |
| SHELL-022 | 파일 실행(더블클릭·Enter·Alt+↓) = 기본 연결 프로그램 | `ShellExecuteW(hwnd,"open",path,SW_SHOWNORMAL)` + 직전 포그라운드 양도 | `app/win.rs:7097-7110`(shell_open) · `:7091` | `ShellExecuteW` | **P** | 없음 |

### 1-B. 폴더 자동 갱신(감시·셸 통지·프로브)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-030 | 폴더가 밖에서 바뀌면 목록 자동 갱신 — 폴더 감시 | 폴더 1개당 스레드 1개. `CreateFileW(FILE_LIST_DIRECTORY, 전체 공유, BACKUP_SEMANTICS\|OVERLAPPED)` + `ReadDirectoryChangesW`(**비재귀**, 필터 = FILE_NAME\|DIR_NAME\|SIZE\|LAST_WRITE\|ATTRIBUTES, 버퍼 8KiB). 개별 엔트리는 해석하지 않고 `PostMessage(msg, panel, gen)`만. 시작 실패(권한·소실) = `None`(F5 폴백). drop = `SetEvent(stop)`(논블로킹), 핸들 정리는 스레드. 스레드엔 **복제 핸들** 전달 | `app/watcher.rs:61`(start) · `:201`(Drop) | `ReadDirectoryChangesW`·`WaitForMultipleObjects`·`CancelIoEx` | **P** | `app/watcher.rs:219` |
| SHELL-031 | 펼친 하위 폴더의 변경도 자동 반영 | `sync_watchers`: 패널별 원하는 목록 = 루트 + 가시 펼침 폴더(`watch_dirs(64)`, `WATCH_CAP=64`). diff 재구독(유지분 그대로·이탈분 drop·신규분 start). `is_alive()==false`인 죽은 watcher 솎아 재구독. watcher마다 세대(`watch_gen += 1`), 통지는 `watchers[panel].any(gen 일치)`일 때만 유효. `update_status` 길목 + `TIMER_FSPOLL` 틱에서 호출 | `app/win.rs:3552-3591` · `:3545` · `:8941-8950` | — | **N**(로직) | `app/watcher.rs:219`(죽음 관측) |
| SHELL-032 | 변경 폭주에도 화면이 멈추지 않고 1초 안에 갱신 | 디바운스 300ms(`WATCH_DEBOUNCE_MS`) — 첫 통지 시각 기록, 이후 통지는 첫 통지로부터 1,000ms(`WATCH_MAX_MS`) 이내일 때만 타이머 재무장. 만료 시: 인라인 리네임·경로바 편집·전송·휴지통 삭제 중이면 300ms 재무장(연기), 아니면 `reopen_filtered`(펼침·선택·캐럿·스크롤 보존) + 프로브 기준선 재수립 | `app/win.rs:3597`(arm_watch_debounce) · `:9457-9484` · `app/fsprobe.rs:115` · 상수 `app/win.rs:107-112` | `SetTimer` | **N** | `app/fsprobe.rs:250` |
| SHELL-033 | 대량 변경(버퍼 오버플로) 후에도 감시 지속 | `GetOverlappedResult` 실패가 `ERROR_NOTIFY_ENUM_DIR`이면 통지 1회 후 **계속**(전체 재열거 신호). 그 외 오류 = 스레드 종료 + `alive=false` | `app/watcher.rs:164-178` · `:185` | `ERROR_NOTIFY_ENUM_DIR` | **P** | 없음 |
| SHELL-034 | 클라우드 동기화(OneDrive 플레이스홀더 생성 등 FS가 침묵하는 변경)도 즉시 반영 | 패널 활성 탭 루트를 **재귀** 구독: `SHChangeNotifyRegister(hwnd, SHCNRF_ShellLevel\|SHCNRF_NewDelivery, SHCNE_DISKEVENTS, msg, 1, {pidl, fRecursive=true})`. 수신 시 `Lock/Unlock`으로 페이로드만 반납하고 디바운스 합류. 가상 루트·클라우드 센티널은 미등록. 루트 변경 시 재등록, drop=Deregister | `app/shellnotify.rs:45`(register) · `:88`(release_payload) · `app/win.rs:3577-3589` · `:9185-9191` | `SHChangeNotifyRegister` | **W**(타 OS는 FSEvents/inotify + 프로브로 대체) | 없음 |
| SHELL-035 | 통지가 안 오는 드라이브(클라우드 가상·네트워크)도 갱신 — 폴더 서명 프로브 | `DirSig{dir_mtime, scanned, count, fold}`. 1단계 = 폴더 mtime(O(1)). 2단계 = 항목 수 + 항목별 `(size, mtime_ns)` 혼합(`entry_mix`)의 순서 무관 합(`wrapping_add`). 항목 > `SCAN_CAP=4096`이면 2단계 포기(`scanned=false`). 폴더 아님/접근 불가 = `None`(가상 루트 포함, 네트워크 호출 없음). `changed`: mtime 다름 or scanned 다름 or (scanned && count/fold 다름) | `app/fsprobe.rs:64`(probe) · `:101`(changed) · `:56`(entry_mix) | `std::fs`만 | **N** | `app/fsprobe.rs:132`·`:144`·`:159`·`:176`·`:190`(win)·`:243` |
| SHELL-036 | 주기 점검 — 활성 3초 / 비활성(보임) 30초 / 최소화 시 정지 | `TIMER_FSPOLL`: 틱마다 `sync_watchers` + `poll_fs_probe`. 프로브 대상 = 보이는 패널(`bounds.w>0`)의 루트 + **뷰포트의 폴더들**(`viewport_dirs`, 접힌 폴더 포함). 처음 보는 폴더 = 기준선만. 화면을 떠난 폴더는 맵에서 소거. 경로가 바뀐 직후 = 기준선만. 서명은 비교 즉시 갱신. 재로드 후 `refresh_probe_baseline` | `app/win.rs:3626`(poll_fs_probe) · `:3675` · `:9611-9617` · 상수 `:120`·`:128`·`:132` · `:7739-7740` | `SetTimer` | **N** | (서명은 SHELL-035) |
| SHELL-037 | 다른 앱에서 돌아오면 목록이 최신 | `WM_ACTIVATEAPP(활성)` → `reload_both` + 폴링 3s. 비활성 → 30s. 최소화(`SIZE_MINIMIZED`) → 폴링·자니터 정지, 복원(트림 플래그) 시 갱신 | `app/win.rs:7755-7768` · `:3695` · `:7788-7795` | `WM_ACTIVATEAPP`·`WM_SIZE` | **N**(이벤트 원천만 winit로) | 없음 |
| SHELL-038 | USB 삽입·제거 시 내 PC 뷰 갱신 | `WM_DEVICECHANGE`(`DBT_DEVICEARRIVAL 0x8000`/`DBT_DEVICEREMOVECOMPLETE 0x8004`) → 가상 루트를 보이는 패널만 디바운스 재로드 | `app/win.rs:7772-7786` | `WM_DEVICECHANGE` | **P** | 없음 |

### 1-C. 클립보드

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-040 | Ctrl+C / Ctrl+X — 선택 파일을 OS 클립보드에(탐색기와 양방향) | `EmptyClipboard` → `CF_HDROP`(DROPFILES 헤더 + wide 경로 이중 NUL, `fWide=true`) + 등록 포맷 `"Preferred DropEffect"` DWORD(COPY=1/MOVE=2). 효과 포맷 게시 실패해도 파일 목록은 유효(복사 취급). 선택 없으면 클립보드 유지. **OS 클립보드가 단일 출처**(내부 클립보드 없음) | `app/clipboard.rs:200`(write_file_list) · `:167`(hglobal_file_list) · `app/win.rs:7354-7365` · `:2102` | `OpenClipboard`·`SetClipboardData`·`RegisterClipboardFormatW` | **P** | `app/clipboard.rs:1363`(#[ignore] 실 클립보드 왕복) |
| SHELL-041 | Ctrl+V — 파일 붙여넣기(복사/이동 자동 판정) | `GetClipboardData(CF_HDROP)` → `DragQueryFileW` 루프(개별 실패 격리). `Preferred DropEffect & MOVE` → 이동, 없음/실패 → 복사. 이동이면 붙여넣기 시작 전 클립보드 비움(잘라내기 1회성). 전송 엔진(`start_transfer` — 진행·취소·undo) 합류. CF_HDROP 없고 가상 파일 있으면 SHELL-046 | `app/clipboard.rs:251`(read_file_list) · `:229` · `:1009`(clear) · `app/win.rs:7366-7381` | `GetClipboardData`·`DragQueryFileW` | **P** | `app/clipboard.rs:1363` |
| SHELL-042 | 붙여넣기 대상 폴더 규칙 | 선택 1개가 폴더 = 그 폴더 안 / 파일 = 그 파일의 부모 폴더(펼친 하위 폴더 안 파일이면 그 하위 폴더) / 없음·다중 = 현재 폴더 | `app/win.rs:2118-2132`(paste_dest) | — | **N** | 없음 |
| SHELL-043 | 붙여넣기 메뉴 활성 판정 | `has_files()` = `CF_HDROP` 또는 `FileGroupDescriptorW` 존재(열지 않고 `IsClipboardFormatAvailable`). `has_text()` = `CF_UNICODETEXT` | `app/clipboard.rs:93` · `:99` · `:985` | `IsClipboardFormatAvailable` | **P** | 없음 |
| SHELL-044 | 잘라낸 항목이 흐리게 표시됨(탐색기에서 잘라낸 것도) | `WM_CLIPBOARDUPDATE`마다 `sync_cut_marks`: 클립보드가 Move 파일 목록이면 그 경로 집합, 아니면 빈 집합(thread_local `CUT_MARKS`). 바뀌었을 때만 양 패널 목록 무효화. 시작 시 1회 동기. 페인트는 `has_cut_marks()` 선판정 후 `is_cut_marked(path)` | `app/clipboard.rs:122` · `:143` · `:148` · `app/win.rs:1793-1794` · `:9691-9700` · `:9704` | `AddClipboardFormatListener`·`WM_CLIPBOARDUPDATE` | **P**(통지) + **A**(행 흐림 그리기) | 없음 |
| SHELL-045 | 클립보드 관리자(Win+V·Ditto)와 경합해도 복사/붙여넣기가 무시되지 않음 | `OpenClipboard` 최대 5회·10ms 간격 재시도(첫 성공은 대기 0, 마지막 실패 뒤 대기 없음). 읽기·쓰기·비우기 공통. drop 가드로 `CloseClipboard` 보장 | `app/clipboard.rs:57-89` | `OpenClipboard` | **W**(X11은 selection 소유 모델 — 타임아웃 규약으로 대응) | `app/clipboard.rs:1275` · `:1307`(#[ignore]) |
| SHELL-046 | RDP·Outlook 첨부·압축 폴더에서 복사한 "가상 파일" 붙여넣기 | `OleGetClipboard`→`IDataObject`. `FileGroupDescriptorW`(HGLOBAL) 파싱: `cItems`가 실제보다 커도 담긴 만큼만, 이름 `sanitize_rel`(빈 이름·`:` 포함·`.`/`..`·중간 빈 조각 기각, 말미 구분자 허용, `\`·`/` 수용), `FD_ATTRIBUTES(0x4)`+DIRECTORY=폴더, `FD_FILESIZE(0x40)`=크기 힌트. `plan_dests`: 최상위 이름 충돌은 `" (2)"`(`nexa_ops::unique_dest`), 하위 항목은 같은 루트 매핑 추종(자식이 부모보다 먼저 와도 성립). `FileContents`(lindex=인덱스)는 `IStream` 또는 `HGLOBAL` 수용(HGLOBAL은 크기 힌트만큼만) | `app/clipboard.rs:290`(sanitize_rel) · `:310`(parse_group_descriptor) · `:396`(plan_dests) · `:427`(extract_virtual_from) · `:449`(save_contents) | `OleGetClipboard`·`IDataObject`·`FILEDESCRIPTORW`·`IStream` | **W**(mac=file promise / Linux=XDS — §4-2·4-3) | `app/clipboard.rs:1050` · `:1177` |
| SHELL-047 | 가상 파일 붙여넣기 중 진행 창·취소 가능(UI 안 멈춤) | UI 스레드: 디스크립터 파싱·대상 계획·항목별 `GetData`(스트림 확보) → `IStream`은 `CoMarshalInterThreadInterfaceInStream`으로 패킷화(`VSource::Marshaled`), HGLOBAL은 즉시 복사(`VSource::Bytes`). 워커(MTA): 256KiB 청크, 청크마다 취소 확인·진행 가산, 크기 미상은 EOF까지·힌트 도달 시 중단, 실패/취소 파편 삭제, 취소여도 마샬 패킷은 1회 소비. 세그먼트 상태 Pending→Active→Done/Failed/Skipped. 완료 = `PostMessage(done_msg, 전건 성공, 0)` → `WM_APP_VPASTE`: 진행 창 마감·undo 기록·재로드 | `app/clipboard.rs:525`(plan_clipboard_paste) · `:533` · `:561`(acquire_contents) · `:603`(run_virtual_paste) · `:656` · `:744` · `app/win.rs:2141-2183` · `:9194-9211` | COM 마샬링·`IStream::Read` | **W** | `app/clipboard.rs:1223` |
| SHELL-048 | 가상 파일 — 동기 폴백·클라우드 대상 | 진행 슬롯 사용 중(`cloud_shared`/`vpaste_roots` 존재) = UI 스레드 동기 추출(64KiB, 대기 커서). 클라우드 대상 = `%TEMP%\NexaDir\vpaste-<pid>-<seq>`에 동기 추출 후 업로드 전송(Copy)으로 연계 | `app/win.rs:2143-2167` · `:2186-2194` · `app/clipboard.rs:359` · `:793` | 임시 폴더 | **W** | (SHELL-046 테스트) |
| SHELL-049 | 가상 파일 붙여넣기/드롭도 Ctrl+Z로 되돌림 | 생성된 **최상위** 경로 중 존재하는 것만 `VPasteOp` 기록: undo = 생성물 휴지통 삭제 · redo = 셸 undelete 복원. 설명 = `op.copyCount` | `app/win.rs:2197-2207` · `:3455-3491` | 휴지통 | **P** | 없음 |
| SHELL-050 | 텍스트 복사/붙여넣기(경로바·이름 바꾸기·터미널) | 쓰기 `CF_UNICODETEXT`(NUL 종단) · 읽기 `CF_UNICODETEXT`(시스템이 CF_TEXT 자동 변환) | `app/clipboard.rs:823`(write_text) · `:991`(read_text) | user32 | **P**(nexa-sql `clipboard.rs` 재사용 가능) | 없음 |
| SHELL-051 | 서식 있는 복사 — 미리보기(RTF 모노스페이스) / 터미널(HTML+RTF) | `write_text_rich`: 평문 + `"Rich Text Format"`(Consolas 지정 `{\rtf1\ansi\deff0{\fonttbl{\f0\fmodern Consolas;}}\f0\fs18 …}`, 비ASCII=`\uN?` 부호 16비트·서로게이트 쌍, 줄=`\r\n`→`\par `). `write_text_html_rtf`: 평문 + `"HTML Format"`(CF_HTML로 이미 래핑된 문자열) + RTF. RTF/HTML 게시 실패해도 평문은 유효 | `app/clipboard.rs:852` · `:923` · `:899`(put_registered) · `:961`(to_rtf_mono) | 등록 클립보드 포맷 | **P** | 없음 |
| SHELL-052 | Ctrl+C/X/V/Z·편집 메뉴가 포커스 문맥에 맞게 동작 | 디스패치 순서: ①경로바 편집 ②인라인 리네임 ③도크 터미널(키 포커스) ④도크 Info/Preview 텍스트 선택(복사만) ⑤파일 목록. 터미널이 대상이 되는 순간 경로바 편집·리네임 취소(`cancel_text_edits`) | `app/win.rs:7246`(do_clip) · `:7210`(ClipAct) · `:7239`(cancel_text_edits) | — | **N** | 없음 |

### 1-D. 드래그 앤 드롭

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-060 | 탐색기 등에서 파일을 끌어다 놓기(수신) | `OleInitialize` 후 `RegisterDragDrop(hwnd, DropTarget)`(등록 실패는 stderr 로그 후 계속). `DragEnter`: `GetData(CF_HDROP)` 경로 캐시, 없으면 `FileGroupDescriptorW` 광고 여부로 가상 소스 판정, 둘 다 없으면 `DROPEFFECT_NONE`. 해제 = `WM_NCDESTROY`에서 `RevokeDragDrop` | `app/dnd.rs:391-435` · `:163`(hdrop_paths) · `app/win.rs:1743` · `:1771-1784` · `:9705` | `IDropTarget`·`RegisterDragDrop` | **P** | `app/dnd.rs:514` |
| SHELL-061 | 드롭 위치에 따라 대상 폴더 결정 | 화면 좌표 → 클라이언트 → `panel_at(x)`. 폴더 행 위 = 그 폴더 / 파일 행·빈 본문 = 그 패널의 현재 폴더 / 패널 밖 = `None`(수신 거부) | `app/win.rs:3252-3268`(drop_dest_at) | `ScreenToClient` | **N** | 없음 |
| SHELL-062 | 복사/이동 자동 판정 + 수정키 강제 | Ctrl=복사 · Shift=이동 · 기본 = 첫 소스와 대상이 같은 볼륨이면 이동 / 다르면 복사. 소스 중 하나라도 대상과 같거나 대상의 상위면 🚫(`DROPEFFECT_NONE`). 가상 소스 = 항상 복사(볼륨·수정키·자기/하위 검사 비대상) | `app/dnd.rs:192`(op_for) · `:208`(resolve) · `ops/lib.rs:79`(same_volume) · `:64`(is_same_or_sub) | `MK_CONTROL`·`MK_SHIFT` | **N**(판정) + **P**(수정키 관례 — mac은 Option/Cmd) | 없음 |
| SHELL-063 | 이동 드롭 시 원본이 사라지지 않음(전송 엔진이 이동 수행) | Drop에서 효과가 MOVE면 소스에 `DROPEFFECT_NONE` 반환(최적화 이동 규약 — MOVE를 돌려주면 소스가 원본을 삭제해 비동기 전송과 경쟁). COPY는 그대로 | `app/dnd.rs:466-475` | OLE 규약 | **P** | 없음 |
| SHELL-064 | 7-Zip·압축 폴더에서 끌어온 파일이 유실되지 않음 | Drop 시 `GetData(CF_HDROP)` **재조회**(이때 실제 추출). `is_delayed` = DragEnter 광고가 비어 있지 않고 재조회와 집합이 다름. 지연 소스면 Drop 반환 전 `steal_volatile`: **`%TEMP%` 아래 항목만** `%TEMP%\NexaDir\dnd-<pid>-<seq>\<i>\<name>`으로 같은 볼륨 rename(실패 항목은 원경로 유지), 확보분이 있으면 연산을 Move로 강제(스테이징 자연 소거). 재조회로 실경로가 오면 가상 소스 플래그 해제. 드롭 거부 시 `restore_volatile`로 원위치 | `app/dnd.rs:437-504` · `:81`(is_delayed) · `:87`(under_dir) · `:111`(steal_volatile) · `:148`(restore_volatile) | `GetData` 지연 렌더링 | **P**(판정 로직은 N — Linux file-roller XDS·mac file promise에도 같은 "반환 전 확보" 원칙) | `app/dnd.rs:533` · `:544` · `:557` · `:580` · `:600` |
| SHELL-065 | Outlook 첨부·zip 내부·MTP 항목 끌어다 놓기(가상 파일 드롭) | CF_HDROP 없이 `FileGroupDescriptorW`만 → Drop **반환 전 동기 추출**(대기 커서). 클라우드 대상은 무동작. 생성물은 undo 기록 + 재로드 | `app/dnd.rs:495-501` · `app/win.rs:3294-3313` · `app/clipboard.rs:342` | `IDataObject` | **W** | `app/clipboard.rs:1177` · `app/dnd.rs:525` |
| SHELL-066 | 드래그 중 — 본문 상/하단 자동 스크롤, 비활성 탭에 머물면 탭 전환, 접힌 폴더에 머물면 펼침 | DragEnter/DragOver마다 `track` → `TIMER_DND`(100ms) 폴링 무장(정지 커서에서도 판정). ①엣지 = 1행 스크롤(폴링 반복 = 연속) ②후보 우선순위: 비활성 탭 > 접힌 폴더 행. 같은 후보에 `dnd_hover_ms`(기본 3000, 200~10000) 이상 머물면 발동 후 대기 리셋. 폴더는 발동 시 행-경로 재검증. 이탈/드롭 = 타이머·대기 해제 | `app/win.rs:3326`(dnd_track) · `:3336` · `:3345`(dnd_track_update) · `:3395` · `:3317`(DndHover) · 상수 `:100-101` | `SetTimer` | **N**(로직) + **A**(목록·탭 컨트롤 API) | 없음 |
| SHELL-067 | 앱에서 탐색기/다른 앱으로 끌어다 놓기(발신) | 왼쪽 누름 후 `SM_CXDRAG/SM_CYDRAG`(최소 4px) 초과 이동 시 시작. 대상 = 선택(없으면 캐럿), 전송 중인 대상은 제외. 캡처 해제 후 `DoDragDrop(FileListDataObject, DropSource, COPY\|MOVE)`. 데이터 객체는 **CF_HDROP 직접 제공**(교차 폴더 선택 지원). Esc=취소 · 왼쪽 버튼 해제=드롭 · 기본 커서. MOVE가 돌아와도 원본 삭제 안 함. 종료 후 양 패널 `abort_press`, 드롭 성공 시에만 `reload_both` | `app/dnd.rs:372`(begin_drag) · `:243-263` · `:269-359` · `app/win.rs:8340-8377` | `DoDragDrop`·`IDropSource`·`IDataObject` | **P** | `app/dnd.rs:514` |
| SHELL-068 | 전송 진행 중 드롭은 거부되고 안내됨 | `handle_external_drop`: `st.transfer.is_some()`이면 제목줄 `ops.busy` + `false` 반환 → dnd.rs가 확보 파일 원위치 | `app/win.rs:3273-3288` · `app/dnd.rs:491-494` | — | **N** | `app/dnd.rs:600`(복귀) |
| SHELL-069 | (내부) DnD 스테이징 폴더 정리 | 전송 결과 중 `<tmp>\NexaDir\dnd-*` 출신(`is_dnd_staging`)을 분리(`split_staged`), 비워진 슬롯 `dnd-N\i`→`dnd-N` 순 `remove_dir`(비었을 때만 성공) | `app/win.rs:2211` · `:2222` · `:2235` | `std::fs` | **N** | `app/win.rs:9873-9876` |

### 1-E. 삭제·휴지통

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-070 | Del = 휴지통으로(확인 없음, 행은 즉시 사라짐) | 대상 = 선택(없으면 캐럿). 동시 1잡(`pending_delete`). 워커 스레드(STA)에서 `SHFileOperationW(FO_DELETE, FOF_ALLOWUNDO\|FOF_NOCONFIRMATION\|FOF_SILENT)` 배치(이중 NUL 목록) → `WM_APP_DELETE`. 워커 시작 직후 양 패널 `hide_paths`(낙관적 숨김 — FS 무변) | `app/win.rs:3716`(do_delete) · `:3772`(start_recycle_delete) · `:3821-3839` · `:3182`(delete_to_recycle_bin) · 키 `:8873` | `SHFileOperationW` | **P** | `app/recycle.rs:205`(#[ignore] 왕복) |
| SHELL-071 | 다른 프로그램이 쓰는 파일은 삭제 전에 한 번에 물어봄 | 사전 프로브: `CreateFileW(DELETE 권한, 전체 공유, BACKUP_SEMANTICS)` → `ERROR_SHARING_VIOLATION`만 "잠김"(폴더는 자신만, 기타 오류는 사후 판정). 잠긴 항목이 있으면 모달 1회: [건너뛰고 삭제(N개)](남은 것이 있을 때만) / [다시 시도] / [취소]. 목록은 최대 10개 + "…외 N개" | `app/win.rs:3849`(probe_locked) · `:3776-3811` · `:3884`(name_list) | `CreateFileW` 공유 위반 | **P**(잠금 개념이 OS별 상이) + **A**(모달) | 없음 |
| SHELL-072 | 삭제 실패 항목을 정확히 알려주고 재시도 가능 | 완료 시 배치 반환값 대신 **사후 diff**: 원위치에 남아 있으면 실패. 성공분만 undo 기록(`DeleteBatchOp`, 설명 `del.recycleOp`). 재로드 후 실패 행 선택 강조 → 모달 [다시 시도]/[닫기]. 다시 시도 = 실패분만 재진입 | `app/win.rs:3926-3975`(on_delete_message) · `:3898`(select_paths) | — | **N** + **A**(모달) | 없음 |
| SHELL-073 | 삭제 실행 취소(Ctrl+Z) = 휴지통에서 원위치 복원 / 다시 실행 = 재삭제 | undo: 휴지통 셸 폴더(`CSIDL_BITBUCKET`) 열거, 상세 컬럼 0(이름)+1(원래 위치)로 `"{위치}\{이름}"` 조합 → 대소문자 무시 정확 일치, 실패 시 **확장자 숨김 폴백**(같은 폴더 + stem 일치). 경로당 최초 일치 1건. 일치 항목들에 `undelete` 동사. 복원 수 < 요청 수 = `OpError::Failed(n)`. redo: 존재하는 것만 다시 휴지통. `STRRET`은 직접 파싱(WSTR/CSTR/OFFSET) | `app/recycle.rs:32` · `:42` · `:133` · `:147` · `app/win.rs:3414-3450` · `:3514`(do_undo_redo) | `IShellFolder2::GetDetailsOf`·`IEnumIDList`·`InvokeCommand("undelete")` | **P** | `app/recycle.rs:205`(#[ignore]) |
| SHELL-074 | Shift+Del = 완전 삭제(확인창, 기본 = 아니오) | `MessageBoxW(MB_YESNO\|MB_ICONWARNING\|MB_DEFBUTTON2)` 제목 `del.title`·본문 `del.confirm`({0}개). 항목별 `nexa_ops::delete_permanent`(개별 격리), 결과 노트 `del.done` + 실패 수. undo 기록 없음 | `app/win.rs:3731-3764` · `ops/lib.rs:398` | `MessageBoxW` | **A**(확인 대화상자) + **N**(삭제) | 없음 |
| SHELL-075 | 클라우드 항목 삭제는 서비스 API로 | 첫 대상이 클라우드 경로면 같은 연결의 항목만 `WriteOp::Delete` 워커로(로컬 undo 불가) — **클라우드 인벤토리 소관**, 여기는 분기점만 | `app/win.rs:3721-3731` | — | **N** | — |

### 1-F. 파일 정보(도크 Info)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-080 | 단일 선택 시 기본 정보 8줄(이름·종류·경로·크기·디스크 할당 크기·만든/수정한/액세스한 날짜) | `std::fs::metadata` 기반 `Basic`. 폴더는 크기 줄 없음. 메타데이터 불가(클라우드 가상 등) = 트리 값으로 크기·수정일 폴백. 다중 선택 = "{0}개 선택" 1줄. 클라우드는 라벨 경로 표시 | `app/fileinfo.rs:48`(basic) · `app/win.rs:2266-2330`(dock_info) | `std::fs` + (Win) 파일 속성 | **N**(종류 이름은 P) | `app/fileinfo.rs:455` |
| SHELL-081 | 디스크 할당 크기 | Win: `GetCompressedFileSizeW`(압축/스파스 반영) → 클러스터(`GetDiskFreeSpaceW` 루트 질의: 섹터/클러스터×바이트/섹터) 올림. UNC(`\\`) 경로·폴더·플레이스홀더 = 생략. 클러스터 0/미상 = 점유 그대로, 0바이트 = 0. 비Windows 현재 `None` | `app/fileinfo.rs:83` · `:131` · `:137`(round_up_cluster) | `GetCompressedFileSizeW`·`GetDiskFreeSpaceW` | **P** | `app/fileinfo.rs:431` |
| SHELL-082 | 탐색기식 크기 표기 "220 KB (225,871 B)" | 1024 미만 = "512 B". 단위 KB~PB. 유효 3자리 **버림**(<10 = 소수 2자리, <100 = 1자리, 그 외 정수). 천 단위 콤마 | `app/fileinfo.rs:145` · `:168` | — | **N** | `app/fileinfo.rs:378` · `:388` |
| SHELL-083 | 형식별 상세(탐색기 "자세히" 탭 — PPT·Excel·MP4·사진의 제목·만든 이·재생 시간·해상도 등) | `IShellItem2` + `System.PropList.FullDetails` 문자열 파싱(`prop:` 접두·항목 앞 비영문 플래그 제거) → 속성별 `IPropertyDescription::GetDisplayName`(라벨)·`FormatForDisplay`(OS 로캘 서식). `System.PropGroup.*` = 그룹 머리글(뒤따르는 값 있는 속성이 있을 때만 출력). 기본 8줄과 겹치는 10개(`BASIC_DUPES`)·빈 값 제외. 표시 = 빈 줄 + `[그룹]` + `라벨: 값`. 비Windows 현재 빈 벡터 | `app/fileinfo.rs:212`(details) · `:182`(parse_proplist) · `:195` · `app/win.rs:2330-2345` | Windows 속성 시스템 | **P** | `app/fileinfo.rs:440` · `:475`(win) |
| SHELL-084 | 선택을 빠르게 옮겨도 상세 조회가 쌓이지 않음 | 스레드 `nexa-fileinfo` 1개(STA 평생 유지). **레인 2개(패널별) × 최신 요청 1칸**(덮어쓰기). 결과 = `notify(lane, gen, path, lines)` → `WM_APP_INFO_DETAILS`, 그 레인의 최신 요청과 세대·경로 일치 시에만 반영. 같은 경로 미도착 요청이 있으면 재요청 안 함. 핸들러 panic은 `catch_unwind`로 격리. 조회 중엔 `info.loadingDetails` 한 줄(추정 — 문구 키 존재 `ko.lang:191`, 표시 코드 구간 미열람) | `app/fileinfo.rs:310-366` · `app/win.rs:7689-7693` · `:2476-2515` · `:9163-9183` | 스레드·Condvar | **N**(COM 초기화만 cfg) | `app/fileinfo.rs:396` |
| SHELL-085 | 클라우드 온라인 전용 파일은 상세 조회로 다운로드를 유발하지 않음 | `FILE_ATTRIBUTE_OFFLINE(0x1000)` 또는 `RECALL_ON_DATA_ACCESS(0x400000)` = `placeholder` → 디스크 할당 크기·상세 생략. `wants_details` = 폴더 아님 && 플레이스홀더 아님 | `app/fileinfo.rs:38-39` · `:51-59` · `:369` | 파일 속성 비트 | **P** | `app/fileinfo.rs:414` |

### 1-G. 접근성

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-090 | 스크린 리더가 파일 목록과 행 이름을 읽음 | `WM_GETOBJECT`(lparam=`UiaRootObjectId -25`) → 활성 패널 **가시 행 불변 스냅샷**(`Snap{name=패널 경로, rect, first_row, rows[name, selected, focused, rect]}`)을 담은 List 프로바이더 반환. 행 = ListItem(Name=파일명, HasKeyboardFocus, IsKeyboardFocusable). 탐색 FirstChild/LastChild/Next/Previous/Parent. `ElementProviderFromPoint`는 y만으로 행 판정. 행 런타임 id = 전역 행 인덱스. 헤더 높이는 `row_h` 근사 | `app/uia.rs:141-237` · `:245-312` · `:358` · `app/win.rs:2038-2068` · `:9314-9321` | UI Automation | **P** | 없음 |
| SHELL-091 | 캐럿 이동·목록 변경이 스크린 리더에 통지됨 | `uia_notify`: (활성 패널, 캐럿) 변화 → `AutomationFocusChanged` / (활성 패널, 경로, 행 수) 변화 → `StructureChanged(ChildrenInvalidated)`. `UiaClientsAreListening()`이 거짓이면 스냅샷도 안 만듦(비용 0). 같은 행 수의 개명은 미포착(α 한계) | `app/win.rs:2074-2098` · `app/uia.rs:353` · `:369` · `:383` | `UiaRaiseAutomationEvent` | **P** | 없음 |
| SHELL-092 | 보조 기술이 행을 선택/선택 해제할 수 있음 | `ISelectionItemProvider`: Select/AddToSelection/RemoveFromSelection → `PostMessage(WM_APP_UIA_SELECT=0x8007, 전역 행, SEL_SINGLE 0/ADD 1/REMOVE 2)`. UI 스레드가 `select_program`으로 범위 검사 후 반영(ADD는 미선택일 때만, REMOVE는 선택일 때만 토글) | `app/uia.rs:314-350` · `app/win.rs:9291-9311` | UIA 패턴 | **P** | 없음 |

### 1-H. 비밀(토큰·암호)·코어 공용

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-095 | 클라우드 로그인 유지(재시작 후 재로그인 불필요) + USB로 옮기면 토큰이 평문으로 남지 않음 | refresh 토큰을 DPAPI(`CryptProtectData`, 현재 사용자+머신 바인딩)로 암호화 → **소문자 hex 1줄**로 `data\secrets\cloud<N>.tok`. 저장은 원자적(`config::save` = temp+`sync_all`+rename, 임시명에 pid). 로드 실패(부재·타 PC·손상) = `None`→재로그인. `decode_hex`: 앞뒤 공백 허용, 비ASCII(BOM·멀티바이트)·홀수 길이·빈 문자열·비hex 거부. 연결 해제 = 파일 삭제, 목록 축소 = `clear_from(idx)`로 꼬리 슬롯(상한 32) 정리. 비Windows 현재 `protect/unprotect = None`(클라우드 인증 미지원) | `app/secret.rs:24` · `:29` · `:34` · `:39` · `:76` · `:88-130` · `app/config.rs:994` · 호출 `app/win.rs:634` · `:4168` · `:5501` · `:5790` | DPAPI(crypt32) | **P** | `app/secret.rs:174`(win) · `:205`(win) · `:220` · `:228` · `:248` |
| SHELL-096 | 압축 파일 암호는 기록·노출되지 않음 | `Secret`: `Display`/직렬화 없음, `Debug`=`Secret(***)`(길이도 숨김), 접근은 `expose`/`expose_str`뿐, Drop에서 `write_volatile` 0 덮기 + `compiler_fence`. `take_from_string`(용량 선확보 후 복사 + 원본 소거)·`take_from_u16`(NUL까지·경유 버퍼 소거). `Clone`도 Drop 소거. 헬퍼 `zeroize_bytes/u16/string`. 외부 crate 0 | `core/secret.rs:23` · `:31` · `:39` · `:46-93` · `:104` · `:111` · 사용처 `app/pwprompt.rs:258` · `app/preview/archive.rs:14` · `app/preview/wasm.rs:265` | 없음(표준 라이브러리) | **N** | `core/secret.rs:122` · `:130` · `:143` · `:151` |
| SHELL-097 | (내부) 코어 공용 타입 | `CORE_VERSION`(패키지 버전 — 플러그인 호환성 점검용)·`FileKind{File, Dir, Symlink}` | `core/lib.rs:9` · `:13` | 없음 | **N** | `core/lib.rs:24` · `:31` |

### 1-I. 예제(진단 도구)

| ID | 기능 | 동작 상세 | 진입점 | 의존 | 분류 | 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| SHELL-098 | 셸 확장 핸들러별 프로파일러 | 인수 = 경로 + CLSID 목록. CLSID마다 `CoCreateInstance`→`IShellExtInit::Initialize(부모 pidl, IDataObject)`→`QueryContextMenu` 단계별 ms 출력. 비Windows = "Windows 전용" 출력 | `ex/ctxmenu_handlers.rs:76` | COM | **W**(Windows 전용 진단 도구로 유지) | — |
| SHELL-099 | 셸 메뉴 격리 재현기 | 인수 = 경로 [반복수=2] [QueryContextMenu 플래그 16진]. parse·bind·GetUIObjectOf·Query·verb 조회·NewMenu 병합·같은 ICM 재질의 시간 출력(콜드 vs 웜). 종료 코드로 확장 즉사 판정 | `ex/ctxmenu_probe.rs:160` | COM | **W** | — |
| SHELL-100 | 이미지 미리보기 독립 예제 | WIC 디코드→Fant 스케일(확대 없음)→32bppBGRA→`StretchDIBits`. 본편 `dw.rs::image_scaled`와 같은 파이프라인 | `ex/preview_image.rs:161` | WIC·GDI | **W→대체**(dir3는 nexa-gfx 디코더 사용 — 예제는 재작성 또는 폐기, 미리보기 인벤토리 소관) | — |

**기능 수: 75개**(SHELL-001~022 · 030~038 · 040~052 · 060~069 · 070~075 · 080~085 · 090~092 · 095~097 · 098~100).

---

## 2. 화면·컨트롤 배치

이 범위는 대부분 비시각 모듈이다. 화면에 드러나는 것은 **팝업 메뉴 5종 · 모달 3종 · 진행 창 재사용 · 도크 Info 텍스트 · 행 흐림 표시**다.

### 2-1. 행 컨텍스트 메뉴(위→아래 구성)

```
┌──────────────────────────────────────────┐
│ [셸 항목 — OS/확장이 공급, 순서 그대로]        │  ID 1..0x6FFF
│   열기 / 편집 / 연결 프로그램 ▸ / 7-Zip ▸ / …  │
│   잘라내기 · 복사            ← 선택 시 앱이 가로챔(copy/cut)
│   삭제 · 이름 바꾸기         ← 선택 시 앱이 가로챔(delete/rename)
│   경로 복사                 ← 제자리 대체(copyaspath → CTX_COPY_PATH, 라벨 = 앱 언어)
│   이름 복사                 ← 고유(after_id = 경로 복사 바로 아래)
│   속성                      ← 셸 실행
├──────────────────────────────────────────┤  (하단 고유 항목이 있을 때만 구분자)
│ 완전 삭제                   ← CTX_DELETE_PERMANENT
│ 폴더에 붙여넣기              ← 단일 폴더 선택 + 클립보드에 파일 있을 때만 존재
├──────────────────────────────────────────┤  (New 병합 성공 시에만 구분자)
│ 새로 만들기 ▸               ← 단일 선택일 때만. ID 0x7000..0x7FFF (서브메뉴는 열 때 채움)
└──────────────────────────────────────────┘
```

- 하단 고유 항목의 순서 = 설정 `ctx_menu_order`의 `row` 블록 순서(기본 `new, deletePermanent, copyName, pasteInto` — `new`는 위치 고정, `copyName`은 앵커 우선). 근거 `app/win.rs:2957-3014` · `app/shellmenu.rs:560-616`.
- "경로 복사" 셸 항목은 Windows에서 **Shift+우클릭(확장 동사)일 때만** 셸이 제공한다(추정 — Windows 11 클래식 메뉴 동작. 코드상으로는 verb `copyaspath`가 메뉴에 있을 때만 대체되고, 없으면 "이름 복사"는 하단 고유 섹션으로 폴백한다: `app/shellmenu.rs:573-587`).
- 표시 위치: 마우스 = 커서 위치 · 키보드(Apps/Shift+F10) = 캐럿 행 앵커(`row_anchor`). 정렬 = 기본(`TPM_RETURNCMD|TPM_RIGHTBUTTON`만 — 좌상단 기준, OS가 화면 밖 보정).
- 문구 키: `ctx.new`="새로 만들기" · `ctx.deletePermanent`="완전 삭제" · `ctx.pasteInto`="폴더에 붙여넣기" · `ctx.copyPath`="경로 복사" · `ctx.copyName`="이름 복사"(`nexa-dir2/crates/nexa-app/lang/ko.lang:161-163`·`:215`·`:236`).

### 2-2. 배경(빈 영역) 컨텍스트 메뉴

```
[셸 배경 항목 — 보기 ▸ / 정렬 ▸ / 새로 고침 / 붙여넣기 / 새로 만들기 ▸ / 속성 …]   (paste 동사는 앱이 가로챔)
───────────────
붙여넣기                 ← CTX_PASTE_BG (클립보드에 파일 없으면 회색)
실행 취소: {설명}         ← CTX_UNDO (없으면 "실행 취소" 기본 라벨 + 회색)
다시 실행: {설명}         ← CTX_REDO
```

- 순서 = `ctx_menu_order`의 `bg` 블록(기본 `paste, undo, redo`). 문구 `ctx.paste`·`ctx.undoOf`·`ctx.redoOf`·`menu.edit.undo`·`menu.edit.redo`(`ko.lang:164`·`:193-194`). 근거 `app/win.rs:2828-2876`.
- 조건: 활성 패널의 본문 영역이면서 행이 아닌 곳(`rows.in_body && !row_at`). 비활성 패널 우클릭은 누름 단계에서 활성화된다(`app/win.rs:8200-8204`).

### 2-3. 내 PC 배경 메뉴(클라우드 연결)

`{라벨} 링크`(후보, 선택 가능) × n → `{라벨}`(연결됨, 체크 + 회색) × n → 둘 다 없으면 `감지된 클라우드 동기화 클라이언트가 없습니다`(회색). 정렬 `TPM_LEFTALIGN|TPM_TOPALIGN`, 커서 위치. 근거 `app/win.rs:3205-3249` · `ko.lang:51-52`.

### 2-4. 모달 대화상자

| 대화상자 | 제목 | 본문 | 버튼(왼→오, id) | 기본/취소 | 근거 |
| --- | --- | --- | --- | --- | --- |
| 완전 삭제 확인 | `del.title`="삭제" | `del.confirm`="{0}개 항목을 영구 삭제할까요?" + 경고 아이콘 | 예 / 아니오(OS 표준) | **기본 = 아니오**(`MB_DEFBUTTON2`) · 예일 때만 진행 | `app/win.rs:3731-3746` |
| 잠긴 항목 | `del.lockedTitle`="삭제할 수 없는 항목" | `del.lockedMsg`="다음 {0}개 항목이 다른 프로그램에서 사용 중입니다:" + 파일명 목록(최대 10 + `del.listMore`="…외 {0}개") | [건너뛰고 삭제({n}개)] id=1(남은 항목 있을 때만) · [다시 시도] id=2 · [취소] id=3 | 닫힘/기타 = 취소(아무것도 안 지움) | `app/win.rs:3776-3811` |
| 삭제 실패 | `del.failTitle`="삭제 실패" | `del.failMsg`="다음 {0}개 항목을 삭제하지 못했습니다 (사용 중이거나 접근할 수 없음):" + 목록 | [다시 시도] id=1 · [닫기] id=2 | 다시 시도 = 실패분만 재진입 | `app/win.rs:3954-3974` |

- 잠긴 항목·삭제 실패 모달은 `crate::dialog::show_buttons(hwnd, title, msg, &[DlgButton{id,label}], &dlg_font)`(자체 그림 대화상자 — **크기·여백 상수는 `app/dialog.rs` 소관, 본 범위 밖**).
- 가상 파일 붙여넣기 진행 창 = `crate::dialog::Progress::open(hwnd, tr("ops.progressTitle"), tr("ops.progressLabel"), &dlg_font)`(전송 진행 창 재사용, `transfer_close_ms > 0`일 때만 생성) — `app/win.rs:2173-2181`.

### 2-5. 도크 Info 텍스트 레이아웃(단일 선택)

```
이름: {name}
종류: {kind 컬럼 값}
경로: {path 또는 클라우드 라벨 경로}
크기: 220 KB (225,871 B)            ← 파일만
디스크 할당 크기: 224 KB (229,376 B)  ← 파일·비UNC·비플레이스홀더만
만든 날짜: {datetime}
수정한 날짜: {datetime}
액세스한 날짜: {datetime}
                                     ← 빈 줄
[{그룹 표시 이름}]                     ← 예: [설명] [원본] [미디어] [비디오]
{속성 라벨}: {OS 서식 값}
…
```

문구 키 `info.selected`/`name`/`kind`/`path`/`size`/`sizeOnDisk`/`created`/`modified`/`accessed`/`loadingDetails`/`currentFolder`(`ko.lang:182-192`). 근거 `app/win.rs:2266-2345`.

### 2-6. 단축키·입력(본 범위)

| 입력 | 동작 | 근거 |
| --- | --- | --- |
| 우클릭(행) / 우클릭(빈 본문) | 행 메뉴 / 배경 메뉴 | `app/win.rs:8278-8279` |
| Shift+우클릭 | 확장 동사 메뉴 | `app/win.rs:3019` · `:2875` |
| Apps 키 · Shift+F10 | 캐럿 행 메뉴(키보드 위치) | `app/win.rs:8876` · `:8910` |
| Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z | 문맥별 복사/잘라내기/붙여넣기/실행 취소 | `app/win.rs:7221-7229` · `:7246` |
| Del / Shift+Del | 휴지통 / 완전 삭제 | `app/win.rs:8873` |
| 드래그(왼쪽 누름 + `SM_CXDRAG`/`SM_CYDRAG`, 최소 4px) | 발신 DnD 시작 | `app/win.rs:8343-8347` |
| 드래그 중 Ctrl / Shift / Esc | 복사 강제 / 이동 강제 / 취소 | `app/dnd.rs:193-197` · `:251` |
| F5 | 수동 새로 고침(감시 실패 폴백) | `app/watcher.rs:5` |

### 2-7. 타이머·시간 상수

| 상수 | 값 | 용도 | 근거 |
| --- | ---: | --- | --- |
| `TIMER_WATCH_BASE`(+패널) / `WATCH_DEBOUNCE_MS` | 10,11 / 300ms | 재로드 디바운스 | `app/win.rs:97` · `:107` |
| `WATCH_MAX_MS` | 1,000ms | 디바운스 연장 상한 | `app/win.rs:112` |
| `WATCH_CAP` | 64 | 패널당 감시 폴더 상한 | `app/win.rs:3545` |
| `TIMER_DND` / `DND_TICK_MS` | 12 / 100ms | 드래그 추적 폴링 | `app/win.rs:100-101` |
| `dnd_hover_ms`(설정) | 3000(200~10000) | 호버 발동 대기 | `app/config.rs:271` · `:662-664` |
| `TIMER_FSPOLL` / `FSPOLL_MS` / `FSPOLL_IDLE_MS` | 14 / 3,000 / 30,000ms | 프로브 폴링(활성/비활성) | `app/win.rs:120` · `:128` · `:132` |
| `TIMER_CTX_PREBUILD` / `CTX_PREBUILD_MS` | 16 / 300ms | 메뉴 선행 구축 | `app/win.rs:125-126` |
| `SCAN_CAP` | 4,096 | 프로브 2단계 열거 상한 | `app/fsprobe.rs:37` |
| `OPEN_ATTEMPTS` / `OPEN_RETRY_DELAY` | 5 / 10ms | 클립보드 열기 재시도 | `app/clipboard.rs:57-58` |
| 생성 감지 재시도 | 20ms × 10(New) / × 2(배경) | 새로 만들기 diff | `app/shellmenu.rs:672` · `:698` |
| 가상 파일 버퍼 | 256KiB(워커) / 64KiB(동기) | 스트림 추출 | `app/clipboard.rs:756` · `:798` |
| 감시 버퍼 | 8KiB | RDCW | `app/watcher.rs:127` |
| `MAX_SLOTS` | 32 | 토큰 슬롯 상한 | `app/secret.rs:66` |

---

## 3. nexa-ui 매핑

`ui-ctl/` 실재 여부는 `nexa-ui/crates/nexa-ctl/src/controls/mod.rs:16-65`와 각 파일 Grep으로 확인했다.

### 3-1. 컨트롤·그리기

| dir2 요소 | nexa-ui 대응 | 상태 | 필요한 추가/변경 |
| --- | --- | --- | --- |
| 네이티브 HMENU 팝업(`TrackPopupMenuEx`) — 행/배경/내 PC/탭/막대/편집 메뉴 | `nexa_ctl::ContextMenu` + `CtxItem`(`ui-ctl/controls/ctxmenu.rs:101`·`:272`) — 아이콘(`MenuIcon::from_rgba/from_alpha` `:72`·`:87`)·단축키 문구·하위 메뉴(`CtxItem::submenu` `:173`)·체크(`with_checked`/`with_mark`)·비활성(`maybe`)·구분선·행 수 상한+스크롤(`set_max_rows` `:385`)·대상 회피 배치(`open_beside` `:563`)·`take_picked()`가 **문자열 id** 반환(`:1134`) | **있음** | ① **지연 채움 하위 메뉴** 없음(children이 정적) → "연결 프로그램 ▸"·"새로 만들기 ▸"·(Windows) 셸 확장 서브메뉴용으로 `children` 플레이스홀더 + "펼침 요청" 신호(예: `take_expand_request() -> Option<id>` + `set_children(id, Vec<CtxItem>)`) 추가 필요 ② 팝업이 **호스트 창 표면 안으로 접힘**(`open_at`의 `host` — `:526-558`): 네이티브 메뉴와 달리 창 밖으로 못 나간다 → 작은 창에서 긴 셸 메뉴는 `set_max_rows`로 스크롤(결정 필요: 별도 팝업 창 호스팅 여부) ③ 항목별 **임의 비트맵 아이콘**(셸 확장 아이콘은 컬러 RGBA) — `from_rgba` 존재 확인, 틴트 없이 원색 출력되는지는 추정(미확인) |
| 텍스트 편집 우클릭 메뉴 | `nexa_ctl::EditMenu`/`EditMenuAction`/`EditMenuCaps`(`ui-ctl/controls/editmenu.rs:42-70`) | **있음** | 없음(클립보드는 호스트 몫 — 같은 설계) |
| `MessageBoxW`(완전 삭제 확인) · `dialog::show_buttons`(잠김/실패 모달) | 범용 메시지/버튼 대화상자 **없음**. `nexa-dlg`는 `FilePicker`뿐(`nexa-ui/crates/nexa-dlg/src/lib.rs:29-154`). nexa-sql은 앱 안에서 창별로 직접 구현(`sql/drop_win.rs:49` — `Button`·`TimeoutButton` 조합) | **없음 → 추가** | `nexa-dlg`에 `MessageDialog`(제목·본문 여러 줄·버튼 목록 `(id, label, tone)`·기본 버튼·Esc=취소·본문 가운데 줄임 `ellipsize_middle`) 추가. `sql/drop_win.rs`의 레이아웃(패딩 16·버튼 높이 = 글자 높이+14·내용에 맞춘 창 높이 재조정 `:362-373`)을 일반화하면 된다. 완전 삭제는 `TimeoutButton` 2단 확인으로 강화할지 결정 필요(dir2는 기본=아니오 1단) |
| `dialog::Progress`(전송 진행 창 — 세그먼트·취소) | **없음**(nexa-ctl에 진행 막대 컨트롤 미확인 — Grep 결과 `progress` 전용 컨트롤 없음) | **없음 → 추가**(추정: 전송/대화상자 인벤토리에서 정의) | 세그먼트 진행 막대 컨트롤 + 진행 창 |
| 잘라내기 대기 행 **흐림 표시** | 목록 컨트롤(파일 목록 — 다른 인벤토리 소관)에 행 상태 플래그 필요 | **없음 → 추가** | 행 그리기에 `dim: bool`(아이콘·텍스트 알파) — `is_cut_marked(path)` 판정을 RowSource가 노출 |
| DnD 중 목록 피드백 — 엣지 스크롤(`drag_scroll_edge`)·접힌 폴더 판정(`is_collapsed_dir`)·호버 펼침(`hover_expand`)·`row_at`·탭 `tab_index_at`·눌림 취소(`abort_press`) | `nexa_ctl::InputEvent`에 드래그 변형 **없음**(`ui-ctl/event.rs:49-110` — Wheel/Key/Char/Mouse*/RightDown만). `Tree`(`ui-ctl/controls/tree.rs`)·`TabBar`에 해당 API 유무 미확인 | **없음 → 추가** | 파일 목록/탭 컨트롤에 호스트 주도 API 6종(위 이름 그대로)을 추가하거나 `InputEvent::DragOver{x,y}`/`DragLeave`/`Drop` 변형 추가. **드롭 대상 행 강조 표시**는 dir2에 없음(커서 효과만) — 그대로 유지 |
| 대기 커서(`IDC_WAIT`) | winit `Window::set_cursor(CursorIcon::Wait)`(추정 — nexa-ui 범위 밖, 호스트 몫) | 호스트 | 없음 |
| UIA 프로바이더 | nexa-ui에 접근성 계층 **없음**(`nexa-ctl`·`nexa-sys`·`nexa-dlg`에서 `accessib\|a11y\|uia\|AT-SPI\|NSAccessibility` Grep 0건) | **없음 → 추가** | §4-7. 스냅샷 모델(`Snap`/`RowSnap`)을 중립 타입으로 `nexa-sys`(또는 신규 `nexa-a11y`)에 두고 OS별 백엔드 |

### 3-2. 파일시스템·셸 서비스(nexa-fs)

| dir2 요소 | nexa-fs 대응 | 상태 | 필요한 추가/변경 |
| --- | --- | --- | --- |
| 폴더 열거(워커·취소) | `ListHandle::start/probe/try_recv`(`ui-fs/lister.rs:54`·`:165`·`:192`) | **있음** | 없음 |
| `fsprobe`(폴더 서명) | 없음. `watch.rs`의 `FileSig`/`StatWatch`는 **열린 파일 1개의 내용 변경** 감시(`ui-fs/watch.rs:16`·`:134`) — 용도가 다르다 | **없음 → 추가** | `app/fsprobe.rs`를 **그대로** `nexa-fs::dirprobe`로 이관(`std::fs`만 사용 — 테스트 7개 동반). `content_hash`(FNV-1a, `ui-fs/watch.rs:46`)는 재사용 가능 |
| 폴더 감시(RDCW) | 없음(`inotify\|FSEvent\|ReadDirectoryChanges\|kqueue` Grep 0건 — nexa-ui·nexa-sql 전체) | **없음 → 추가** | `nexa-fs::dirwatch` — 공통 인터페이스 `DirWatcher::start(path, wake: Box<dyn Fn()+Send>) -> Option<Self>` + `is_alive()` + Drop=논블로킹 중지. 백엔드 3종(§4-5) |
| 셸 아이콘·종류 이름 | `IconService::global().icon(&IconKey, large)`/`kind_name`(`ui-fs/shell.rs:262`·`:278`) — 워커·LRU 512·`version()` 폴링. **Windows만 실구현**, 그 외 `imp::SUPPORTED=false` → `Ready(None)`(`:741`) | **부분** | macOS `NSWorkspace iconForFile:`·Linux 아이콘 테마 구현 추가(아이콘 인벤토리 소관 — 여기서는 "종류 이름"이 Info 2번째 줄에 쓰인다는 의존만 기록) |
| 파일 관리자에서 보기 | `reveal_in_file_manager`(`ui-fs/shell.rs:856`) — Win `explorer /select,` · mac `open -R` · Linux `xdg-open <부모>` | **있음** | Linux는 `org.freedesktop.FileManager1.ShowItems`로 개선 여지(§4-1) |
| `shell:` 별칭·특수 폴더 | `resolve_alias`(`ui-fs/shell.rs:333`)·`places()`/`drives()`(`ui-fs/lib.rs:377`·`:416`) | **있음** | 없음 |
| 파일 실행(`ShellExecuteW open`) | nexa-fs에 없음. nexa-sql 앱 안에 `open_external`(`sql/main.rs:1191-1202` — Win `cmd /C start "" path` · mac `open` · Linux `xdg-open`) | **없음 → 추가** | `nexa-fs::shell::open_default(path)`로 승격. Windows는 `cmd /C start` 대신 `ShellExecuteW`(콘솔 깜빡임·특수문자 인용 문제 회피 + 포그라운드 양도) 권장 |
| 휴지통 삭제/복원 | 없음(`trash\|recycle\|SHFileOperation` Grep — 관련 구현 0건) | **없음 → 추가** | `nexa-fs::trash` — `trash(paths) -> Vec<TrashResult{orig, token}>` · `restore(token\|orig)`(§4-4) |
| 파일 상세 속성 | 없음 | **없음 → 추가** | `nexa-fs::fileinfo`(`Basic`·`DetailLine`·`DetailWorker`는 중립 그대로, `details()`만 OS별) |
| 볼륨 판정 `same_volume` | `ops/lib.rs:79` — 비Windows는 **항상 같은 볼륨**(루트 `/`) | **결함** | unix는 `MetadataExt::dev()` 비교로 교체(§6-L14) |

### 3-3. nexa-sql이 이미 제공하는 것(재사용/확장 대상)

| nexa-sql 구현 | 제공 범위 | dir3에서 부족한 것 |
| --- | --- | --- |
| `sql/clipboard.rs:15-29` `read_text`/`write_text`/`write_rich(text, html)` | 텍스트·HTML. Win=user32 FFI 직접(5×10ms 재시도 `:79-88`) · mac=`pbpaste`/`pbcopy`/`osascript «class HTML»` · Linux=X11 직접 → `wl-paste`/`xclip`/`xsel` 폴백 | **파일 목록·잘라내기 플래그·RTF·형식 존재 판정·변경 통지 전부 없음**. `cf_html`(`:33`) 재사용 가능 |
| `sql/clipboard_x11.rs` | `CLIPBOARD` 소유 스레드(1×1 InputOnly 창)·`TARGETS`/`UTF8_STRING`/`STRING`/`text/plain;charset=utf-8` 응답·INCR 송수신(`CHUNK=128KiB`)·읽기 타임아웃 1.2s | 타깃이 텍스트뿐 → `text/uri-list`·`x-special/gnome-copied-files`·`application/x-kde-cutselection`·`text/html`·`text/rtf` 다중 타깃으로 일반화 + XFixes 소유자 변경 통지 필요. 구조(소유 스레드·`MINE` 캐시·`serve`)는 그대로 확장 가능 |
| winit `WindowEvent::DroppedFile`(`sql/app/event_loop.rs:598`) | 드롭된 **경로만**(항목당 이벤트 1개) | **드롭 좌표·수정키·DragOver·효과 반환·발신·가상 파일 없음** → dir2 DnD(SHELL-060~067)를 표현할 수 없다. OS별 직접 구현 필요(§4-3) |
| `sql/drop_win.rs`·`sql/app/drop.rs` | (이름과 달리) DB 객체 DROP 확인 모달 | DnD와 무관. **모달 대화상자·2단 확인 버튼 패턴**의 참고 구현으로만 사용 |
| `nexa-sql/crates/nsql-vault/src/devkey.rs:1-9` | 기기 키: Win=DPAPI 봉투(`NSDK`+ver) · mac/Linux=**평문 32B 0600 파일** | 토큰 보관(SHELL-095)의 크로스플랫폼 선례 — §4-8 |
| `nexa-sql/crates/nexa-sql/Cargo.toml:44-67` | winit 0.30·softbuffer 0.4 · Linux `x11rb 0.13`(X11/XWayland 백엔드 기본) · macOS `objc2 0.5`/`objc2-app-kit 0.2`/`objc2-foundation 0.2` | dir3 OS 분기 구현이 쓸 수 있는 **이미 승인된 의존**. NSPasteboard·NSWorkspace·NSFileManager 사용 시 objc2-app-kit/foundation feature 추가 필요 |

---

## 4. OS 분기점

표기: 🟦 Windows 현 구현 → 🍎 macOS / 🐧 Linux 대응. macOS·Linux 칸의 API·규격은 **저장소 밖 지식(추정) — 구현 시 실측으로 확정**한다.

### 4-1. 셸 컨텍스트 메뉴(SHELL-001~022) — OS 차이 최대

**공통 구조(권장)**: 메뉴는 3-OS 모두 `nexa_ctl::ContextMenu`로 **직접 그린다**. 메뉴 내용은 `MenuModel = 앱 고정 항목(공통) + OS 공급 항목(분기)`으로 합성한다. dir2에서 "셸이 제공 → 앱이 가로챔"이던 항목(열기·잘라내기·복사·삭제·이름 바꾸기·경로 복사·붙여넣기·속성·새로 만들기)은 **dir3에서 앱 고정 항목으로 승격**하면 3-OS 화면이 같아진다.

```
trait ShellMenuProvider {            // nexa-dir3 앱 계층(또는 nexa-fs::shellmenu)
    fn items(&self, targets: &[PathBuf], extended: bool) -> Vec<ShellItem>;   // 워커 스레드에서 호출
    fn expand(&self, id: &ShellItemId) -> Vec<ShellItem>;                     // 지연 서브메뉴
    fn invoke(&self, id: &ShellItemId, targets: &[PathBuf]) -> Outcome;       // Shell / Created(path) / Cancelled
}
```

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 셸 확장 항목(7-Zip·Git·보내기·공유 등) | **선택지 W-a**: 현행 유지 — 네이티브 HMENU `TrackPopupMenuEx`(OS가 그림 = "직접 그림" 기조의 예외). **선택지 W-b(권장)**: HMENU를 화면에 띄우지 않고 구축(`QueryContextMenu`까지 현행 `prepare_items` 그대로) → `GetMenuItemCount`/`GetMenuItemInfoW(MIIM_STRING\|MIIM_ID\|MIIM_STATE\|MIIM_SUBMENU\|MIIM_BITMAP\|MIIM_FTYPE)`로 항목을 추출해 `CtxItem`으로 변환·자체 그림 → 선택된 id로 `InvokeCommand`. 서브메뉴는 펼칠 때 `HandleMenuMsg(WM_INITMENUPOPUP, hSub, …)`를 직접 호출해 채운 뒤 추출. `hbmpItem`은 `GetDIBits`로 RGBA 변환(`ui-fs/shell.rs:578-611`의 `dib` 재사용). **owner-draw 항목(`MFT_OWNERDRAW`)은 문자열이 없다** → 메모리 DC에 `WM_MEASUREITEM`/`WM_DRAWITEM`을 보내 비트맵으로 받거나, 추출 불가 항목이 있으면 "Windows 메뉴 열기…" 항목으로 W-a 폴백. **메뉴 스레드(STA)·선행 구축·세대 가드는 그대로 유지**(추출도 그 스레드에서) | **등가물 없음**. Finder Sync 확장(Dropbox 등)의 메뉴는 타 앱이 호출 불가 → **기능 상실 수용**. 서비스 메뉴(`NSApp.servicesMenu`)는 네이티브 `NSMenu` 전제라 직접 그림과 충돌 → 1차 제외(후속: `NSSharingService sharingServicesForItems:`로 공유 대상만 목록화) | **표준 없음**. 선택 구현(2차): KDE ServiceMenus(`~/.local/share/kio/servicemenus/*.desktop`·`/usr/share/kio/servicemenus/` — `Type=Service`, `MimeType=`, `Actions=`, `[Desktop Action X] Name/Icon/Exec`) · Nautilus 스크립트(`~/.local/share/nautilus/scripts/` — 실행 시 `NAUTILUS_SCRIPT_SELECTED_FILE_PATHS` 환경 변수) · Nemo 액션(`~/.local/share/nemo/actions/*.nemo_action`) · Thunar 사용자 액션(`~/.config/Thunar/uca.xml`). 1차는 미제공 |
| 열기(기본 프로그램) | `ShellExecuteW("open")` + `AllowSetForegroundWindow` | `NSWorkspace openURL:` 또는 `open <path>` | `xdg-open <path>`(폴백 `gio open`) |
| 연결 프로그램 ▸(지연 서브메뉴) | W-b에서 셸의 "연결 프로그램" 서브메뉴를 추출. 또는 `SHAssocEnumHandlers(ext, ASSOC_FILTER_RECOMMENDED)`→`IAssocHandler::GetUIName/GetIconLocation/Invoke`로 직접 구성 + "다른 앱 선택…" = `SHOpenWithDialog` | `NSWorkspace URLsForApplicationsToOpenURL:`(macOS 12+) / 구형 `LSCopyApplicationURLsForURL(url, kLSRolesAll)` · 기본 앱 `URLForApplicationToOpenURL:` · 실행 `openURLs:withApplicationAtURL:configuration:completionHandler:` 또는 `open -a <App> <path>` · 앱 이름/아이콘 = 번들 `CFBundleDisplayName` + `iconForFile:` | MIME 판정(`xdg-mime query filetype <path>` 또는 `/usr/share/mime/globs2` 직접 파싱) → 후보 = `mimeapps.list`(`$XDG_CONFIG_HOME/mimeapps.list` → `$XDG_CONFIG_DIRS` → `$XDG_DATA_DIRS/applications/mimeapps.list`의 `[Default Applications]`/`[Added Associations]`/`[Removed Associations]`) + `mimeinfo.cache` → `.desktop` 파싱(`Name[로캘]`, `Icon`, `Exec` 필드 코드 `%f %F %u %U %i %c %k`, `NoDisplay`, `Terminal`, `TryExec`) → `Exec` 치환 후 spawn(`Terminal=true`면 터미널 래핑) |
| 파일 관리자에서 보기 | `explorer /select,"<path>"`(`ui-fs/shell.rs:864`) | `NSWorkspace activateFileViewerSelectingURLs:` / `open -R`(현 구현) | D-Bus `org.freedesktop.FileManager1.ShowItems(as uris, s startup_id)`(세션 버스, 경로 `/org/freedesktop/FileManager1`) → 실패 시 `xdg-open <부모>`(현 구현) |
| 속성 | 셸 `properties` 동사(W-b 추출 항목) 또는 `SHObjectProperties(hwnd, SHOP_FILEPATH, path, NULL)` | "정보 가져오기": `osascript -e 'tell application "Finder" to open information window of (POSIX file "…" as alias)'`(최초 1회 자동화 권한 프롬프트) — 또는 앱 도크 Info로 대체 | `org.freedesktop.FileManager1.ShowItemProperties(as, s)` → 미지원 시 앱 도크 Info로 대체 |
| 훑어보기/미리보기 | 없음(dir2 자체 미리보기 도크) | `qlmanage -p <path>`(별도 프로세스 Quick Look) — 선택 항목. `QLPreviewPanel`은 `NSResponder` 체인 참여가 필요해 winit 창과 결합 난이도 高 → 1차 제외 | 없음(`sushi`는 GNOME 전용 D-Bus `org.gnome.NautilusPreviewer` — 제외) |
| 새로 만들기 ▸ | `CLSID_NewMenu` 호스팅(현행, W-b로 항목 추출) 또는 레지스트리 `HKCR\<.ext>\ShellNew`(`NullFile`/`FileName`/`Data`) 직접 열거 | 고정 목록: 폴더 · 빈 텍스트 파일(+ 사용자 템플릿 폴더 — dir3 자체 `data/templates/`) | 폴더 · 빈 파일 + **XDG 템플릿 폴더**(`xdg-user-dir TEMPLATES` 또는 `~/.config/user-dirs.dirs`의 `XDG_TEMPLATES_DIR`) 안 파일을 템플릿으로 복사 |
| 생성 감지 → 인라인 리네임 | 폴더 전후 diff(정확히 1개 신규) — 현행 | 앱이 직접 생성하므로 **경로를 이미 안다** → diff 불필요(`Outcome::Created` 직행) | 동일 |
| 확장 동사(Shift) | `CMF_EXTENDEDVERBS` | 해당 없음 — 앱 "고급" 항목(터미널에서 열기·경로 복사 등) 표시 토글로 재해석 | 동일 |
| 동사 가로채기·제자리 대체 | W-b에서는 추출 단계에서 verb(`GetCommandString(GCS_VERBW)`)가 `delete/rename/copy/cut/paste/copyaspath`인 항목을 **앱 고정 항목으로 치환(중복 제거)** | 불필요 | 불필요 |
| 메뉴 메시지 포워딩·AttachThreadInput·포그라운드 양도 | W-a에서만 필요. W-b에서는 `InvokeCommand` 전 `AllowSetForegroundWindow`만 유지 | 불필요 | 불필요 |
| 터미널에서 열기(권장 추가 — 사용자 요청의 터미널 분기와 일관) | `pwsh`(없으면 `powershell`) | 기본 셸(`$SHELL`) — 도크 터미널 또는 `open -a Terminal <dir>` | `$SHELL` — 도크 터미널 또는 `x-terminal-emulator`/`xdg-terminal-exec` |

**결정 필요(사용자 확인 항목)**: ① Windows 셸 확장 항목을 W-a(네이티브 메뉴 예외) / W-b(추출해 자체 그림) 중 무엇으로. 사용자 기조("모두 직접 개발")에는 W-b가 맞지만 owner-draw 확장의 추출 품질은 실측 전까지 미확정. ② macOS·Linux에서 서드파티 확장 메뉴 부재를 수용하는지(`nexa-dir2/docs/23-cross-platform-feasibility.md:122`·`:200`이 이미 "기능 상실 — 패리티 정책 필요"로 기록).

### 4-2. 클립보드(SHELL-040~052)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 파일 목록 쓰기 | `CF_HDROP` + `Preferred DropEffect`(현행) | `NSPasteboard generalPasteboard` → `clearContents` → `writeObjects:@[NSURL…]`(타입 `public.file-url`; 구형 호환 `NSFilenamesPboardType`) | X11 `CLIPBOARD` 소유 + 타깃 다중 응답: `text/uri-list`(RFC 2483 — `file:///퍼센트-인코딩 경로`, 줄 구분 `\r\n`) · `x-special/gnome-copied-files`(GNOME/Nautilus·Nemo·Caja·Thunar — 본문 `copy\nfile:///a\nfile:///b`, 끝 개행 없음) · `text/plain;charset=utf-8`·`UTF8_STRING`(경로 줄 목록) |
| 잘라내기 표식 | `Preferred DropEffect = MOVE(2)` | **OS에 개념 없음**(Finder는 복사 후 ⌥⌘V로 이동). 앱 전용 타입(예: `com.bimatrix.nexadir.cut`)을 같이 게시 + 게시 시점 `changeCount` 기억 → 붙여넣기 때 타입 존재 && changeCount 일치면 이동. **Finder에서 잘라낸 것은 감지 불가·Finder는 우리 cut을 복사로 취급** | `x-special/gnome-copied-files` 첫 줄 `cut` + KDE용 `application/x-kde-cutselection`(값 `"1"`) 동시 게시. 읽을 때 둘 중 하나라도 있으면 이동 |
| 파일 목록 읽기 | `GetClipboardData(CF_HDROP)` | `readObjectsForClasses:@[NSURL] options:@{NSPasteboardURLReadingFileURLsOnly:@YES}` → `path`. **파일 참조 URL(`file:///.file/id=…`)은 `filePathURL`로 해석** | `TARGETS` 조회 → `x-special/gnome-copied-files` 우선(첫 줄 copy/cut) → 없으면 `text/uri-list`(+`x-kde-cutselection`). `file://` URI 디코딩(호스트 부분 무시/`localhost`) |
| 형식 존재 판정(열지 않고) | `IsClipboardFormatAvailable` | `availableTypeFromArray:` / `canReadObjectForClasses:options:` | `TARGETS` 왕복(소유자 응답 필요 → **타임아웃 필수** — `sql/clipboard_x11.rs:33`의 1.2s 선례. 메뉴 열 때마다 블로킹하지 않도록 XFixes 통지 시점에 캐시) |
| 변경 통지(잘라내기 흐림 동기) | `AddClipboardFormatListener` → `WM_CLIPBOARDUPDATE` | 통지 API 없음 → `changeCount` **폴링**(창 활성 시 + 250~500ms 타이머, 비활성 시 정지) | XFixes `XFixesSelectSelectionInput(SetSelectionOwnerNotifyMask)` → `XFixesSelectionNotify`(x11rb `xfixes` feature 추가 필요). Wayland 네이티브는 포커스 창만 data offer 수신 |
| 가상 파일(FileGroupDescriptorW) | 현행 유지(**W**) | 클립보드에는 사실상 없음. file promise는 DnD 전용(§4-3) | 없음 |
| 텍스트 | `CF_UNICODETEXT` | `NSPasteboardTypeString`(`public.utf8-plain-text`) — 현 nexa-sql은 `pbcopy`/`pbpaste` CLI | `UTF8_STRING`·`text/plain;charset=utf-8`(`sql/clipboard_x11.rs` 그대로) |
| RTF·HTML | `"Rich Text Format"`·`"HTML Format"`(CF_HTML 헤더 래핑) | `public.rtf`·`public.html`(헤더 래핑 **없이** 원문 HTML) — 다중 타입 동시 게시(`setData:forType:` 여러 번). 현 nexa-sql은 `osascript`(HTML+문자열만) | 타깃 `text/rtf`(또는 `application/rtf`)·`text/html`(원문, UTF-8) 추가 응답 |
| 열기 경합 | `OpenClipboard` 재시도 5×10ms | 해당 없음 | 소유자 무응답 = 읽기 타임아웃 |
| Wayland | — | — | winit이 `wl_data_device`를 노출하지 않음 → nexa-sql 방침대로 **X11(XWayland) 백엔드 기본**(`nexa-sql/crates/nexa-sql/Cargo.toml:50-51`). XWayland↔Wayland 클립보드 브리지는 컴포지터가 수행(파일 URI 타깃 전달 여부는 컴포지터별 — 실측 필요). 폴백 CLI: `wl-copy --type text/uri-list`·`wl-paste --list-types` |
| RTF 모노스페이스 글꼴 | `Consolas` | `Menlo` | `monospace`(추정 — 터미널/폰트 인벤토리와 맞춤) |

**권장 구조**: `sql/clipboard.rs`를 nexa-ui 쪽(`nexa-sys::clipboard` 등)으로 승격하면서 다중 표현 API로 확장 — `write(&[Repr])` / `read(kind)` / `has(kind)` / `change_token()` (`Repr = Text | Html | Rtf | Files{paths, cut}`). macOS는 CLI(`pbcopy`) 대신 `objc2-app-kit` `NSPasteboard` 직접(파일 URL은 CLI로 불가).

### 4-3. 드래그 앤 드롭(SHELL-060~069)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 수신 등록 | `OleInitialize` + `RegisterDragDrop`(현행). **winit 충돌 주의**: winit이 자체 `IDropTarget`을 등록해 `DroppedFile`을 내므로 창 생성 시 `WindowAttributesExtWindows::with_drag_and_drop(false)`로 끄고 자체 등록(같은 HWND에 이중 등록 불가 — `DRAGDROP_E_ALREADYREGISTERED`) | winit의 `NSView`가 파일 URL 드래그를 이미 등록(→`DroppedFile`, 좌표·operation 미노출). 자체 처리하려면 winit 뷰 클래스에 `NSDraggingDestination` 메서드(`draggingEntered:`/`draggingUpdated:`/`draggingExited:`/`performDragOperation:`)를 objc2 `class_addMethod`/교체로 주입하거나, 투명 서브뷰를 얹어 `registerForDraggedTypes:@[NSPasteboardTypeFileURL]` | winit X11 백엔드가 XDND 수신 구현(→`DroppedFile`만). 자체 처리: 창에 `XdndAware`(v5) 속성 + ClientMessage `XdndEnter/XdndPosition/XdndLeave/XdndDrop` 처리 + `XdndStatus`(수락·액션)·`XdndFinished` 응답, 데이터 = `XdndSelection`을 `text/uri-list`로 `ConvertSelection`. winit 이벤트 루프의 X 이벤트 훅(`EventLoopBuilderExtX11`/`msg_hook` 계열) 필요 — **가용 훅 실측 필요(추정)** |
| 좌표·대상 판정 | `POINTL`(화면) → `ScreenToClient` | `[sender draggingLocation]`(창 좌표, y 뒤집기) | `XdndPosition`의 루트 좌표 → `TranslateCoordinates` |
| 수정키 → 연산 | Ctrl=복사 · Shift=이동 · 기본=같은 볼륨 이동/다른 볼륨 복사 | 관례: ⌥(Option)=복사 · ⌘(Command)=이동 · 기본 = 같은 볼륨 이동/다른 볼륨 복사. `draggingSourceOperationMask`가 소스 허용 범위 | `XdndPosition`의 요청 액션(`XdndActionCopy/Move/Link/Ask`) + 수정키(Ctrl=복사 · Shift=이동 — GTK/Qt 관례) |
| 효과 반환(최적화 이동) | 이동은 `DROPEFFECT_NONE` 반환 | `performDragOperation:`가 YES + 반환 operation = `NSDragOperationGeneric`/`Copy`(Finder는 Move 반환 시 원본 삭제 안 함 — 타깃이 수행. 추정) | `XdndFinished`의 액션을 `XdndActionCopy`로(소스가 Move 후 삭제하지 않게) 또는 수락 플래그만 — file-roller/Nautilus 동작 실측 필요 |
| 지연 렌더링 소스 | Drop 시 CF_HDROP 재조회 + `%TEMP%` 스테이징 확보 | **file promise**: `NSFilePromiseReceiver receivePromisedFilesAtDestination:options:operationQueue:reader:`(소스가 대상 폴더에 직접 기록 — 완료 콜백에서 재로드) | **XDS**(X Direct Save, 타깃 `XdndDirectSave0`): 수신 측이 속성에 저장할 `file://` URI를 써 주면 소스(file-roller·Ark 등)가 직접 추출. 미구현 시 압축 관리자 드래그는 수신 불가 |
| 가상 파일 드롭 | FileGroupDescriptorW 동기 추출(현행) | 위 file promise로 흡수 | 위 XDS로 흡수 |
| 발신 | `DoDragDrop` + CF_HDROP `IDataObject`(현행) | `[view beginDraggingSessionWithItems:event:source:]` — `NSDraggingItem`(pasteboardWriter = `NSURL`) × n, `NSDraggingSource`의 `draggingSession:sourceOperationMaskForDraggingContext:` = Copy\|Move, 드래그 이미지 = 항목 아이콘 | XDND 소스 직접 구현: `XdndSelection` 소유 + 포인터 그랩 + 커서 아래 창의 `XdndAware`/`XdndProxy` 조회 + Enter/Position/Drop 송신 + `XdndTypeList`(`text/uri-list`). 분량 大 — **2차 슬라이스 권장** |
| 드래그 시작 임계 | `SM_CXDRAG`/`SM_CYDRAG`(≥4px) | 고정 4pt(시스템 값 없음 — HIG 관례. 추정) | `gtk-dnd-drag-threshold`(기본 8px) 근사 → 공통 4~8px 상수 × scale |
| 드래그 중 폴링(정지 커서 판정) | `SetTimer(TIMER_DND, 100ms)` | `draggingUpdated:`가 주기적으로 오지만(추정) 타이머 병행 | `XdndPosition`은 이동 시에만 → 타이머 병행 필수 |
| 임시 폴더 판정(`under_dir`) | `%TEMP%`, 대소문자 무시 | `$TMPDIR`(`/var/folders/…/T/`) — **심볼릭 링크 정규화 필요**(`/var`→`/private/var`), 대소문자는 볼륨 설정 따름 | `$TMPDIR` 또는 `/tmp` + `~/.cache`(file-roller는 `~/.cache/.fr-XXXXXX` 사용 — 추정), 대소문자 구분 |

**1차 범위 제안**: 3-OS 공통 = winit `DroppedFile` 기반 **단순 수신**(대상 = 드롭 시점 커서 아래 폴더 — winit가 드롭 직후 `CursorMoved`를 주는지 OS별 실측 필요) → 2차 = Windows OLE 완전 이식(SHELL-060~067 전부, 코드 대부분 재사용) → 3차 = macOS NSDragging → 4차 = Linux XDND 자체 구현. 단계별 패리티 표를 문서화한다.

### 4-4. 휴지통(SHELL-070~074·049)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 휴지통으로 | `SHFileOperationW(FO_DELETE, FOF_ALLOWUNDO\|NOCONFIRMATION\|SILENT)`(현행 — 워커 STA). 대안 `IFileOperation` | `-[NSFileManager trashItemAtURL:resultingItemURL:error:]` — **`resultingItemURL`(휴지통 안 실제 경로)을 반환**. 볼륨별 휴지통(`~/.Trash`, `/Volumes/X/.Trashes/<uid>/`)은 OS가 처리 | **FreeDesktop Trash 규격 1.0** 직접 구현: 홈 휴지통 `$XDG_DATA_HOME/Trash`(기본 `~/.local/share/Trash`)의 `files/<이름>` + `info/<이름>.trashinfo`(`[Trash Info]\nPath=<URL 이스케이프 절대 경로>\nDeletionDate=YYYY-MM-DDThh:mm:ss`). **info 파일을 `O_EXCL`로 먼저 만들어 이름 확정**(충돌 시 `이름.2` 식 재시도) 후 rename. 다른 마운트는 `$topdir/.Trash/$uid`(디렉터리·sticky 비트·심볼릭 링크 아님 검사) → 불가 시 `$topdir/.Trash-$uid` 생성(이때 `Path=`는 topdir 상대 경로). 둘 다 불가 = 실패 보고(홈 휴지통으로의 교차 장치 복사는 하지 않음). 대안 CLI `gio trash <path>` |
| 복원(undo) | 휴지통 폴더 열거 + `undelete` 동사(현행) | 공개 "되돌려 놓기" API 없음 → **기록해 둔 `resultingItemURL`을 원경로로 `moveItemAtURL:`**(원경로 점유 시 `OpError::NameExists`). 세션을 넘는 undo는 없으므로 충분 | 삭제 시 기록한 (`files/` 경로, `info/` 경로) → 원경로로 rename + trashinfo 삭제. 기록 유실 시 `info/*.trashinfo`의 `Path=`로 역탐색(dir2의 "원래 위치+이름" 매칭 대응) |
| undo 기록 구조 | `DeleteBatchOp{paths}`(원경로만 — 복원 시 재탐색) | `DeleteBatchOp{items: Vec<(orig, trashed)>}`로 **확장 필요** | 동일 확장 |
| 잠금 사전 프로브 | `CreateFileW(DELETE)` 공유 위반 | 해당 개념 약함(unlink는 열린 파일도 가능) → 프로브 생략, 권한(`access(parent, W_OK)`·sticky·`uchg` 플래그)만 사전 점검(선택) | 동일(프로브 생략). 사후 diff 백스톱은 **3-OS 공통 유지** |
| 낙관적 숨김·워커·사후 diff·실패 모달 | 현행 | 공통 유지(N) | 공통 유지(N) |
| 완전 삭제 | `nexa_ops::delete_permanent` | 동일(N) | 동일(N) |
| 네트워크/미지원 볼륨 | 셸이 "영구 삭제?" 확인을 띄울 수 있으나 `FOF_NOCONFIRMATION`으로 **조용히 영구 삭제될 수 있음**(추정 — 현행 위험) | `trashItemAtURL`이 실패(휴지통 없는 볼륨) → 실패 모달에 "완전 삭제" 선택지 제공 검토 | 규격상 실패 → 동일 |

### 4-5. 파일 감시·셸 통지·프로브(SHELL-030~038)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 폴더 감시 | RDCW OVERLAPPED, 폴더당 스레드 1(현행) | **FSEvents**: `FSEventStreamCreate(paths, kFSEventStreamEventIdSinceNow, latency≈0.3s, kFSEventStreamCreateFlagFileEvents\|NoDefer\|WatchRoot)` + 전용 dispatch queue. 재귀가 기본이라 **패널 루트 1개 스트림**으로 가시 펼침 폴더까지 커버 가능(이벤트 경로가 감시 집합의 직계 자식인지로 필터) → `WATCH_CAP` 불필요. 대안 kqueue `EVFILT_VNODE`(폴더 fd별, 항목 추가/삭제만 — 내용 변경 미통지)는 비권장 | **inotify**: `inotify_init1(IN_NONBLOCK\|IN_CLOEXEC)` 1개 fd + 폴더별 `inotify_add_watch(IN_CREATE\|IN_DELETE\|IN_MOVED_FROM\|IN_MOVED_TO\|IN_CLOSE_WRITE\|IN_MODIFY\|IN_ATTRIB\|IN_DELETE_SELF\|IN_MOVE_SELF\|IN_ONLYDIR)` — **비재귀(dir2 규약과 동형)**, 스레드 1개가 전 패널 처리(wd→(panel, gen) 맵). 중지 = `eventfd`/self-pipe + `poll` |
| 버퍼 오버플로 복구 | `ERROR_NOTIFY_ENUM_DIR` → 통지 후 계속 | `kFSEventStreamEventFlagMustScanSubDirs`/`UserDropped`/`KernelDropped` → 통지 후 계속 | `IN_Q_OVERFLOW` → 전 패널 통지 후 계속 |
| 죽음 관측(`is_alive`) | 스레드 종료 시 `alive=false` | `kFSEventStreamEventFlagRootChanged`(루트 소실/이동) → `alive=false` | `IN_IGNORED`/`IN_DELETE_SELF`/`IN_MOVE_SELF` → 그 wd `alive=false` |
| 상한 | 폴더 64/패널 | 스트림 수 소수 | `/proc/sys/fs/inotify/max_user_watches`(기본 8192~수십만) — `ENOSPC` 시 그 폴더만 비감시(F5·프로브 폴백) |
| 셸 수준 통지(클라우드 플레이스홀더) | `SHChangeNotifyRegister`(현행) | 불필요(추정) — iCloud Drive·File Provider 변경은 FSEvents로 옴 | 등가물 없음 → 프로브가 유일 보험(FUSE·rclone·gvfs·NFS/CIFS는 원격 변경을 inotify로 안 줌) |
| 프로브(`fsprobe`) | 현행 | **그대로(N)** — APFS 폴더 mtime 갱신 의미 동일(추정) | **그대로(N)** |
| 활성/비활성/최소화 | `WM_ACTIVATEAPP`·`WM_SIZE(SIZE_MINIMIZED)` | winit `WindowEvent::Focused(bool)`·`Occluded(bool)`(3-OS 공통) | 동일 |
| 볼륨 도착/제거 | `WM_DEVICECHANGE` | `NSWorkspaceDidMountNotification`/`DidUnmountNotification`(`[NSWorkspace sharedWorkspace].notificationCenter`) 또는 `/Volumes` FSEvents | `/proc/self/mountinfo`를 `poll(POLLPRI\|POLLERR)` 또는 `/run/media/$USER`·`/media` inotify · UDisks2 D-Bus는 선택 |
| UI 스레드 통지 | `PostMessageW(hwnd, msg, panel, gen)` | `EventLoopProxy<UserEvent>::send_event(FsChange{panel, gen})`(winit — 3-OS 공통) | 동일 |

### 4-6. 파일 속성(SHELL-080~085)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 기본 시각·크기 | `std::fs::metadata`(현행) | 동일. `created()` 지원(APFS birthtime) | 동일. `created()`는 `statx` btime(커널 4.11+·glibc 2.28+·ext4/btrfs/xfs) — 미지원 FS는 `Err` → "만든 날짜" 줄 생략(현 코드가 `Option`이라 자연 처리) |
| 디스크 할당 크기 | `GetCompressedFileSizeW` + 클러스터 올림 | `MetadataExt::blocks() * 512`(`st_blocks` — 이미 할당 단위 반영, 올림 불필요) | 동일 |
| 플레이스홀더 판정 | `OFFLINE`/`RECALL_ON_DATA_ACCESS` | iCloud dataless 파일: `st_flags & SF_DATALESS(0x40000000)`(`std::os::macos::fs::MetadataExt::st_flags`) 또는 `NSURLUbiquitousItemDownloadingStatusKey != Current` · 구형 `.이름.icloud` 스텁 | 표준 없음 → `false`(rclone/gvfs 마운트는 판별 불가 — 상세 조회가 다운로드를 유발할 수 있음을 수용 or 마운트 FS 타입 `fuse.*`이면 상세 생략) |
| 종류 이름(Info 2번째 줄) | `SHGetFileInfoW(SHGFI_TYPENAME)`(`ui-fs/shell.rs:712`) | `NSURLLocalizedTypeDescriptionKey` | shared-mime-info `/usr/share/mime/<type>.xml`의 `<comment xml:lang="ko">` |
| 형식별 상세 | 속성 시스템 `System.PropList.FullDetails`(현행) | Spotlight 메타데이터: `MDItemCreateWithURL` → `MDItemCopyAttributeNames`/`MDItemCopyAttribute`(`kMDItemTitle`·`kMDItemAuthors`·`kMDItemPixelWidth/Height`·`kMDItemDurationSeconds`·`kMDItemCodecs`…) + 라벨 `MDSchemaCopyDisplayNameForAttribute`(로캘). 간이 대안 `mdls -plist - <path>`. 그룹 머리글은 자체 분류표로(일반/미디어/이미지/문서) | **표준 속성 시스템 없음** → 1차 = 상세 없음(빈 벡터 — 현행 비Windows 동작 `app/fileinfo.rs:302`). 2차 = 자체 경량 추출(이미지 크기는 nexa-gfx 디코더 헤더, 그 외는 WASM 미리보기 플러그인의 메타 출력과 통합 검토). 외부 도구(`exiftool`·`mediainfo`) 의존은 비권장 |
| 워커 COM 초기화 | STA | 불필요(스레드에 `NSAutoreleasePool`/`autoreleasepool` 필요) | 불필요 |

### 4-7. 접근성(SHELL-090~092)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 프로바이더 | UIA `IRawElementProviderSimple/Fragment/FragmentRoot`(현행). **winit 창에서는 `WM_GETOBJECT`를 직접 받을 수 없음** → 창 서브클래싱(`SetWindowSubclass`/`SetWindowLongPtrW(GWLP_WNDPROC)`) 필요 | `NSAccessibility` 프로토콜: winit `NSView`에 `accessibilityChildren`/`accessibilityRole`(List/Row)·`accessibilityFrame`·`accessibilityLabel` 주입 + `NSAccessibilityPostNotification(el, NSAccessibilityFocusedUIElementChangedNotification / …RowCountChangedNotification)` | AT-SPI2(D-Bus `org.a11y.Bus` → `org.a11y.atspi.Accessible`/`Component`/`Selection` 인터페이스 직접 구현) — 분량 大 |
| 권장 | **AccessKit**(`accesskit` + `accesskit_winit`, MIT/Apache-2.0) 채택 검토 — `TreeUpdate` 스냅샷 푸시 모델이 dir2의 "불변 스냅샷" 설계(`app/uia.rs:4-7`)와 동형이고 3-OS 백엔드(UIA·NSAccessibility·AT-SPI)를 한 번에 해결. 외부 crate 최소화 기조(nexa-sql DR-3 예외 원장)와의 정합은 **결정 필요**. 직접 구현을 고수하면 1차 = Windows만 이식(서브클래싱) + mac/Linux 미지원 명시 |
| 선택 요청 | `PostMessage(WM_APP_UIA_SELECT)` | `EventLoopProxy` 사용자 이벤트 | 동일 |

### 4-8. 비밀 보관(SHELL-095~096)

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| refresh 토큰 | DPAPI → hex 1줄 `data/secrets/cloud<N>.tok`(현행) | (a) Keychain: `SecItemAdd`/`SecItemCopyMatching`(`kSecClassGenericPassword`, service=`NexaDir`, account=`cloud<N>`) 또는 CLI `security add-generic-password -U -s NexaDir -a cloud0 -w <tok>` / `security find-generic-password -s NexaDir -a cloud0 -w` (b) **nexa-sql 선례**: 기기 키(32B, 0600 평문 파일) + 봉투 암호화(`nexa-sql/crates/nsql-vault/src/devkey.rs:6`) | (a) Secret Service(D-Bus `org.freedesktop.secrets`; CLI `secret-tool store --label=NexaDir service NexaDir account cloud0` / `secret-tool lookup …`) — 데스크톱 키링 부재 환경 多 (b) nexa-sql 선례(0600 기기 키) |
| 권장 | 설정 구조를 nexa-sql에서 차용하는 사용자 지시에 맞춰 **nsql-vault 방식(기기 키 + 봉투)** 으로 통일하고 토큰 파일 형식(hex 1줄·원자 저장·슬롯 32·꼬리 정리)은 dir2 그대로. "USB로 옮기면 평문 토큰이 남지 않는다"(`app/secret.rs:4-6`)는 Windows에서만 성립 — mac/Linux는 기기 키 파일이 함께 복사되면 풀린다(정직한 한계로 문서화) |
| `Secret` 타입 | 그대로(N) | 그대로(N) | 그대로(N) — `take_from_u16`은 Win32 편집 컨트롤 경유용이라 dir3(nexa-ctl `TextBox` = `String`)에서는 `take_from_string` 경로만 쓰임 |

### 4-9. 기타 분기

| 항목 | 🟦 Windows | 🍎 macOS | 🐧 Linux |
| --- | --- | --- | --- |
| 경로/이름 복사 줄 구분자(SHELL-020) | `\r\n`(현행) | `\n` | `\n` |
| 경로 비교(복원 매칭·`under_dir`·`is_same_or_sub`) | ASCII 대소문자 무시(현행) | 볼륨 따라(APFS 기본 비구분) — 정규화(NFD/NFC) 주의 | 대소문자 **구분** — `eq_ignore_ascii_case` 사용처 전부 재검토(`app/recycle.rs:87`·`:97` · `app/dnd.rs:93` · `ops/lib.rs:60-74`) |
| 가상 파일 이름 검증(`sanitize_rel`) | `:` 거부(드라이브) | `:` 는 HFS 구분자 — 거부 유지 무해 | `:` 허용 문자지만 소스가 Windows 규약이므로 유지(Windows 전용 경로라 무관) |
| 포그라운드 양도 | `AllowSetForegroundWindow(ASFW_ANY)` | 불필요(`open`/NSWorkspace가 활성화) | X11: `_NET_ACTIVE_WINDOW`/startup-notification — `xdg-open`에 `XDG_ACTIVATION_TOKEN`(Wayland) 전달 검토(추정) |
| 스레드 모델 | UI=STA(OLE) · 메뉴=STA · 삭제 워커=STA · 가상 붙여넣기 워커=MTA · 상세 워커=STA | AppKit 호출은 **메인 스레드 한정**(NSPasteboard·드래그·NSWorkspace 일부) — 워커에서 호출 금지, `autoreleasepool` 필수 | X 연결은 스레드별 별도 연결(nexa-sql 선례 `sql/clipboard_x11.rs:107-112`) |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 설정 키(영속)

| 키 | 형식·기본값·범위 | 용도 | 근거 |
| --- | --- | --- | --- |
| `ctx_menu_order` | `row:1[new:1,deletePermanent:1,copyName:1,pasteInto:1]\|bg:1[paste:1,undo:1,redo:1]`(기본 = `default_order(CTXMENU_BLOCKS)`; 파싱 시 정규화 재직렬화) | 고유 메뉴 항목 순서·표시 | `app/config.rs:183` · `:313` · `:490` · `:693-695` · `:1078-1082` |
| `dnd_hover_ms` | 정수 3000, clamp 200~10000 | DnD 호버 발동 대기 | `app/config.rs:105` · `:271` · `:461` · `:662-664` |
| `transfer_close_ms` | 정수 2000, clamp 0~10000(0 = 진행 창 미표시) · 구 키 `transfer_close_secs` 수용(×1000) | 진행 창 닫힘 지연 — 가상 붙여넣기 진행 창 생성 조건에도 사용 | `app/config.rs:103` · `:270` · `:657-670` |
| `term_copy_format` | `text`\|`html`\|`rtf`\|`both`(기본 `text`) | 터미널 복사 형식(`write_text_html_rtf` 인자 결정) | `app/config.rs:99` · `:269` · `:654-655` |

dir3에서는 nexa-sql 설정 레지스트리(`nsql-settings`) 문법으로 이관(키 이름·기본값·범위 유지). 이관 규칙 자체는 설정 인벤토리 소관.

### 5-2. 파일·임시 자원

| 경로 | 형식 | 수명 | 근거 |
| --- | --- | --- | --- |
| `<data>/secrets/cloud<N>.tok`(N=0..31) | DPAPI blob의 소문자 hex 1줄(ASCII) | 연결 해제 시 삭제 | `app/secret.rs:14-21` · `:45-51` |
| `<data>` | exe 옆 `data/`(쓰기 불가면 `%LOCALAPPDATA%\NexaDir\data`) — 프로세스당 1회 판정 | — | `app/config.rs:360-372` |
| `<name>.<pid>.tmp` | 원자 저장 임시 파일(성공 시 rename, 실패 시 삭제) | 순간 | `app/config.rs:994-1011` |
| `%TEMP%\NexaDir\dnd-<pid>-<seq>\<i>\<name>` | DnD 지연 소스 확보 스테이징 | 전송(Move)이 비우면 슬롯 `remove_dir` | `app/dnd.rs:115-119` · `:131` · `app/win.rs:2235` |
| `%TEMP%\NexaDir\vpaste-<pid>-<seq>` | 가상 파일 → 클라우드 업로드 스테이징 | OS 임시 정리 | `app/win.rs:2186-2194` |
| `%TEMP%\nexa-ctxmenu-timing.log` | 계측 줄 append | `NEXA_CTX_TIMING=1`일 때만 | `app/shellmenu.rs:178` |

### 5-3. 메모리 상태(State 필드 — `app/win.rs`)

| 필드 | 의미 | 근거 |
| --- | --- | --- |
| `menu_thread: Option<MenuThread>` · `ctx_gen: u64` · `ctx_prepared_key: Option<Vec<PathBuf>>` · `ctx_showing: bool` | 메뉴 스레드 핸들·세대·선행 구축 대상·표시 중 플래그 | `app/win.rs:920-926` |
| `rbutton_down_seen: bool` · `rclick_began_edit: bool` | 우클릭 누름-뗌 짝 · 편집 진입 클릭 억제 | `app/win.rs:1058` · `:8209` |
| `watchers: [Vec<DirWatcher>; 2]` · `watch_gen` · `watch_since: [u64; 2]` | 패널별 감시·세대·디바운스 첫 통지 시각 | `app/win.rs:1027-1030` |
| `probe: [Option<(PathBuf, DirSig)>; 2]` · `sub_probe: [HashMap<PathBuf, DirSig>; 2]` | 루트/뷰포트 폴더 서명 기준선 | `app/win.rs:1034` · `:1042` |
| `shell_watch: [Option<ShellWatch>; 2]` | 셸 변경 구독 | `app/win.rs:1037` |
| `pending_delete: Option<Vec<PathBuf>>` | 진행 중 휴지통 삭제(동시 1잡) | `app/win.rs:1006-1007` |
| `vpaste_roots: Option<Vec<PathBuf>>` · `cloud_shared` · `cloud_progress` | 가상 붙여넣기 잡(진행 슬롯 공용) | `app/win.rs:2156` · `:2171-2181` |
| `dnd_hover: Option<(DndHover, since_ms)>` · `dnd_hover_ms` · `drag_press` | DnD 호버 대기·드래그 시작 후보 | `app/win.rs:1013-1014` · `:3317-3322` · `:8340` |
| `info_worker` · `info_gen` · `info_req: [Option<(u64, PathBuf)>; 2]` · `info_details: [Option<(PathBuf, Vec<DetailLine>)>; 2]` | 상세 워커·요청·결과 | `app/win.rs:913-917` |
| `uia_caret` · `uia_struct` | UIA 이벤트 발행용 직전 서명 | `app/win.rs:2075-2088` |
| thread_local `CUT_MARKS: HashSet<PathBuf>` | 잘라내기 표시 집합(UI 스레드) | `app/clipboard.rs:113-117` |
| thread_local `ACTIVE: Vec<MenuHost>` | 표시 중 메뉴 핸들러(메뉴를 띄운 스레드) | `app/shellmenu.rs:102-104` |
| thread_local `TS: Option<ThreadState>`(메뉴 스레드) | 주 창 hwnd/tid·결과 메시지·준비분 `(gen, MenuReq, Prepared)` 1개 | `app/menuthread.rs:74-83` |

### 5-4. 스레드

| 스레드 | 수 | 아파트 | 역할 | UI로의 통지 |
| --- | --- | --- | --- | --- |
| UI(주) | 1 | STA(`OleInitialize`) | 창·DnD 콜백·클립보드·배경 메뉴(동기)·`OleGetClipboard` | — |
| `nexa-ctxmenu` | 1(데몬) | STA | 행 메뉴 구축·표시·명령 실행 | `WM_APP_CTXMENU_RESULT(wparam=gen, lparam=Box<(MenuReq, Outcome)>)` |
| watcher | 감시 폴더당 1(≤64×2) | — | RDCW 대기 | `WM_APP_FSCHANGE(wparam=panel, lparam=gen)` |
| `nexa-fileinfo` | 1 | STA | 속성 시스템 조회 | `WM_APP_INFO_DETAILS(wparam=lane, lparam=Box<(gen, path, lines)>)` |
| 휴지통 삭제 워커 | 잡당 1 | STA | `SHFileOperationW` | `WM_APP_DELETE(wparam=ok)` — `post_final_notify`(유실 방지 재시도) |
| 가상 붙여넣기 워커 | 잡당 1 | MTA | 스트림 → 파일 | `WM_APP_VPASTE(wparam=전건 성공)` |
| UIA 콜백 | OS 임의 | MTA | 스냅샷 읽기만 | `WM_APP_UIA_SELECT(wparam=전역 행, lparam=모드)` |

### 5-5. 메시지 번호(주 창)

| 메시지 | 값 | 근거 |
| --- | --- | --- |
| `WM_APP_FSCHANGE` | 0x8002 | `app/win.rs:96` |
| `WM_APP_UIA_SELECT` | 0x8007 | `app/uia.rs:43` |
| `WM_APP_DELETE` | 0x800D | `app/win.rs:155` |
| `WM_APP_VPASTE` | 0x8013 | `app/win.rs:175` |
| `WM_APP_SHCHANGE_BASE`(+패널 0/1) | 0x8014, 0x8015 | `app/win.rs:178` |
| `WM_APP_INFO_DETAILS` | 0x8016 | `app/win.rs:181` |
| `WM_APP_CTXMENU_RESULT` | 0x8017 | `app/win.rs:184` |
| (메뉴 스레드 내부) `WM_MT_PREPARE/SHOW/INVALIDATE` | `WM_APP+0x40/0x41/0x42` | `app/menuthread.rs:31-33` |

dir3 대응: Win32 메시지 번호는 전부 **winit `EventLoopProxy<UserEvent>`의 enum 변형**으로 치환(`FsChange{panel, gen}`·`DeleteDone`·`VPasteDone{ok}`·`InfoDetails{lane, gen, path, lines}`·`CtxMenuResult{gen, req, outcome}`·`A11ySelect{row, mode}`·`ClipboardChanged`). Box 포인터 전달·`post_final_notify` 재시도는 채널 소유권 이동으로 대체되어 불필요.

### 5-6. 핵심 흐름 3종

**(1) 행 우클릭(메뉴 스레드 경로)**
`WM_RBUTTONDOWN`(선택 규약 반영 + `prebuild_ctx_menu`) → `WM_RBUTTONUP`(짝 검사·편집 메뉴 우선) → `show_row_context_menu` → `build_row_menu_req`(State 읽기만) → `ctx_gen += 1`·`MenuThread::show(gen, req, at)`·`ctx_showing = true` → [메뉴 스레드] 준비분 `same_menu`면 재사용/아니면 구축 → `AttachThreadInput` + `track`(모달) → 선택 분기(고유/New/가로채기/셸 invoke) → `PostMessage(RESULT)` → [UI] `ctx_showing = false` · 세대 일치 시 `apply_row_menu_outcome` + `update_status` → `prebuild_ctx_menu`(재구축).

**(2) 외부 변경 → 목록 갱신**
(watcher 스레드 RDCW 완료 | 셸 통지 메시지 | FSPOLL 프로브 불일치 | WM_DEVICECHANGE) → `arm_watch_debounce(panel)`(첫 통지 시각 기록 · 1s 상한 내에서만 300ms 재무장) → `TIMER_WATCH_BASE+panel` 만료 → 편집/전송/삭제 중이면 300ms 연기, 아니면 `reopen_filtered` + `refresh_probe_baseline` + `update_status`(→ `sync_watchers`·`arm_ctx_prebuild`).

**(3) 휴지통 삭제**
Del → `do_delete(false)` → `start_recycle_delete`(동시 1잡 검사 → 잠금 프로브 루프 + 모달) → `pending_delete = Some` → 워커 spawn(STA, `SHFileOperationW`) + 낙관적 `hide_paths` → `WM_APP_DELETE` → `on_delete_message`: 존재 여부로 성공/실패 분할 → 성공분 `DeleteBatchOp` push → `reload_both` → 실패분 선택 강조 + 모달(다시 시도 = `start_recycle_delete(failed)`).

---

## 6. 이식 시 주의 — 회귀 방지에 필요한 실측 교훈·결함 수정 이력

| # | 교훈(무엇이 깨졌었나 → 어떻게 고쳤나) | 근거 | dir3 적용 |
| --- | --- | --- | --- |
| L01 | **`IContextMenu`는 프록시 미등록 — 만든 스레드가 표시·명령 실행까지** 맡아야 한다. 워커에서 만들고 UI에서 표시 불가. 같은 ICM 재질의도 확장이 매번 다시 실행(0.8~1.2s)되어 캐시 효과 없음 | `nexa-dir2/docs/audit/20261002-ultracode/ctxmenu-latency.md:60-64` · `app/menuthread.rs:3-5` | W-b(추출)도 **메뉴 스레드에서** 추출·invoke. 추출 결과(`Vec<CtxItem>`)만 Send |
| L02 | 우클릭 지연의 95%는 `QueryContextMenu`(셸 확장 19개 0.6~1.4s). 미리 만든 메뉴 표시는 수 ms | `…/ctxmenu-latency.md:46` · `:64` | 선행 구축(300ms 머무름·누름 즉시·닫힌 뒤 재구축) 유지. 회귀 지표 = 표시 ≤ 50ms(`…:73`) |
| L03 | 다중 ICM 호스팅 시 **전 핸들러 브로드캐스트 금지** — `CNewMenu`가 주 메뉴의 `WM_INITMENUPOPUP`에 반응해 템플릿을 주 메뉴에 평탄 삽입 | `app/shellmenu.rs:86-89` · `:232-233` | W-a 유지 시 선별 라우팅 그대로. W-b에서는 서브메뉴 채움을 해당 핸들러에만 직접 호출 |
| L04 | 메뉴가 닫힌 뒤에도 포워딩이 살아 있으면 `InvokeCommand`가 띄운 모달의 **남의 HMENU**가 주 핸들러로 흘러감(G8-20) | `app/shellmenu.rs:654-660` | W-a 유지 시 동일. 테스트 `forward_stops_once_active_cleared` 이식 |
| L05 | 모달 메뉴 펌프 동안 wndproc 재진입 — **State 가변 참조를 모달 진입 전에 끝낸다**(요청 데이터 추출 → 표시 → 재획득) | `app/shellmenu.rs:267-269` · `app/win.rs:2827` · `:3053` | 자체 그림 메뉴는 모달이 아니므로 구조적으로 해소. 단 Windows `InvokeCommand`·`DoDragDrop`은 여전히 모달 — 같은 규약 유지 |
| L06 | 세대 번호로 낡은 결과 무시(메뉴·watcher·상세 워커 공통) — 선택이 바뀐 뒤 늦게 닫힌 메뉴/낡은 스레드 통지/지난 상세 | `app/menuthread.rs:15-16` · `app/win.rs:8945` · `:9171-9173` | 사용자 이벤트에 `gen` 필드 유지 |
| L07 | 짝 없는 `WM_RBUTTONUP`(누름을 모달·다른 창이 흡수)에서 메뉴를 열면 선택 규약이 반영 안 된 낡은 선택에 메뉴가 뜸(G3-15) | `app/win.rs:8237-8241` | winit `MouseInput{Right, Pressed/Released}` 짝 검사 유지 |
| L08 | 비 OVERLAPPED `ReadDirectoryChangesW`는 **drop의 `CloseHandle`이 블로킹**(조용한 OneDrive 폴더에서 수십 초 UI 프리즈) → OVERLAPPED + 중지 이벤트, drop=SetEvent만 | `app/watcher.rs:7-12` | 3-OS 공통 계약: **감시 중지는 논블로킹**(inotify=eventfd, FSEvents=`FSEventStreamStop` 비동기 큐) |
| L09 | 스레드에 원본 핸들을 넘기면 drop이 닫은 뒤 **핸들 값 재활용**으로 무관 객체를 기다림 → 복제 핸들 전달 | `app/watcher.rs:99-100` | fd 공유 시 `dup`/`Arc<OwnedFd>` |
| L10 | 죽은 watcher가 "감시 중"으로 남아 그 폴더가 영구 무갱신 → `alive` 플래그 + 주기 재구독(유휴 중에도 `TIMER_FSPOLL`이 수행) | `app/watcher.rs:42-46` · `app/win.rs:3557-3560` · `:9612-9615` | `is_alive()`를 공통 인터페이스에 포함. 테스트 `watcher_death_is_observable_when_dir_removed` 3-OS 이식 |
| L11 | `ERROR_NOTIFY_ENUM_DIR`에서 break하면 폭주 한 번에 감시가 죽음 → 통지 후 계속 | `app/watcher.rs:165-176` | inotify `IN_Q_OVERFLOW`·FSEvents `MustScanSubDirs` 동일 처리 |
| L12 | 디바운스 타이머 무한 연기(동기화 폭주 동안 화면 정지) → 첫 통지 기준 1s 상한 | `app/win.rs:108-112` · `app/fsprobe.rs:111-117` | 그대로(순수 함수 `debounce_should_extend` 이식) |
| L13 | OneDrive 플레이스홀더 생성은 **부모 mtime도 RDCW 통지도 안 남김** → 서명에 count/fold 포함 + 셸 통지 구독 + 뷰포트 폴더 프로브. 종전 `bytes 합 + 최신 mtime`은 같은 크기 덮어쓰기 미검출 → 항목별 접기 | `app/shellnotify.rs:4-10` · `app/fsprobe.rs:47-51` · `app/win.rs:3645-3650` | `fsprobe` 그대로 + 테스트 6종 이식 |
| L14 | 프로브 기준선을 다음 틱에 세우면 열거~틱 사이(0~3s)의 외부 변경이 **영영 삼켜짐** → 재로드 직후 즉시 기준선 | `app/win.rs:3672-3674` | `refresh_probe_baseline`을 모든 재로드 끝에 |
| L15 | 재로드를 편집·전송·휴지통 삭제 중에 하면 편집 행 인덱스가 흔들리거나 낙관적 숨김 행이 조기 원복 → 연기 재무장 | `app/win.rs:9458-9468` | 그대로 |
| L16 | `WM_CLIPBOARDUPDATE` 직후 `OpenClipboard`는 클립보드 관리자와 경합 — 실패를 "파일 없음"으로 오판하면 잘라내기 흐림이 지워지고 Ctrl+V가 무시됨 → 5×10ms 재시도 | `app/clipboard.rs:52-58` | Windows 유지. X11은 소유자 무응답 타임아웃을 "변경 없음"으로 처리(캐시 유지) |
| L17 | `SHCreateDataObject`(PIDL 기반)는 CF_HDROP을 렌더링하지 않아 탐색기·자기 수신부가 드롭 거부 → **CF_HDROP 직접 제공 IDataObject** | `app/dnd.rs:265-268` | Windows 발신은 `FileListDataObject` 그대로 |
| L18 | 이동 드롭에 `DROPEFFECT_MOVE`를 돌려주면 소스(탐색기)가 원본을 삭제해 비동기 전송과 경쟁(교차 볼륨 이동 중 원본 소실) → NONE 반환 | `app/dnd.rs:466-475` | mac/Linux 수신도 "타깃이 이동 수행, 소스에는 복사/일반으로 보고" 원칙 |
| L19 | 7-Zip은 `DoDragDrop` 반환 직후 임시 폴더를 삭제(4개 중 2~3개만 생존) → **Drop 반환 전** 같은 볼륨 rename으로 확보 | `app/dnd.rs:100-104` | mac file promise·Linux XDS에서도 "반환 전 확보/완료 콜백 후 전송" 순서 보장 |
| L20 | DragEnter 광고가 비었는데(일시 실패·가상 겸용) Drop 재조회가 실경로면 **지연 소스로 오판 → 사용자 실파일을 %TEMP%로 rename**하던 결함(G8-03) → 광고가 비면 지연 아님 + `%TEMP%` 밖은 절대 확보 안 함 + Drop 시 가상 플래그 재판정 | `app/dnd.rs:77-83` · `:105-108` · `:459-462` | 테스트 `is_delayed_*`·`steal_volatile_skips_paths_outside_temp_dir` 이식. macOS는 `$TMPDIR` 심볼릭 링크 정규화 후 비교 |
| L21 | 전송 중 드롭을 조용히 버리면 확보한 스테이징이 고립 → 훅이 수락 여부 반환 + 거부 시 원위치 복귀 | `app/dnd.rs:38-40` · `:146-147` | 그대로 |
| L22 | `DoDragDrop`이 버튼 해제를 소비해 위젯에 MouseUp이 안 옴 → 낡은 press 보류가 다음 클릭에서 **다른 파일을 단일 선택** → 종료 후 `abort_press` | `app/win.rs:8362-8369` | mac `beginDraggingSession`·XDND 그랩도 동일(드래그 종료 시 목록 press 상태 강제 소거) |
| L23 | 드래그 취소 시 양 패널 재열거 생략(불필요 비용) — 드롭 성공일 때만 재로드 | `app/win.rs:8370-8375` | 그대로 |
| L24 | `SHFileOperationW`는 소파일 몇 개에도 3~4초 → UI 스레드 금지(워커) + 낙관적 숨김. 배치 반환값은 항목별 판정 불가 → 사후 diff | `app/win.rs:3815-3817` · `:3835-3836` · `:3922-3930` | 3-OS 워커화·사후 diff 공통 |
| L25 | 삭제 완료 통지 유실 = `pending_delete` 영구 고착(자동 갱신 정지·이후 삭제 차단) → 재시도 통지(`post_final_notify`) | `app/win.rs:3832-3833` | 채널 기반이면 유실 없음 — 워커 panic 시에도 완료 이벤트가 가도록 Drop 가드 |
| L26 | 휴지통 복원 매칭: 탐색기 "확장자 숨김" 설정이면 이름 컬럼에 확장자가 없다 → stem 일치 폴백. `SHELLDETAILS`는 packed — 필드 참조 금지(복사 후 파싱) | `app/recycle.rs:88-100` · `:141-143` | Windows 유지. mac/Linux는 삭제 시 받은 경로를 기록해 매칭 자체를 없앰 |
| L27 | 연결 프로그램이 이미 떠 있으면(Office DDE) 창이 뒤에 숨음 → 실행 직전 `AllowSetForegroundWindow(ASFW_ANY)` | `app/win.rs:7079-7094` | Windows 유지(`ShellExecuteW`·`InvokeCommand` 직전) |
| L28 | 속성 핸들러가 파일을 열면 클라우드 플레이스홀더가 **다운로드됨** → 플레이스홀더 상세 생략. 핸들러는 수십~수백 ms → 워커 + 레인별 최신 1칸. 핸들러 panic이 워커를 죽이면 이후 상세가 영영 안 옴 → `catch_unwind` | `app/fileinfo.rs:10-13` · `:306-309` · `:350-352` | 공통 유지 |
| L29 | UNC 경로의 볼륨 조회는 네트워크 왕복 → UI 스레드 `basic()`에서 디스크 할당 크기 생략 | `app/fileinfo.rs:81` · `:88-90` | unix는 `st_blocks`라 왕복 없음. 단 `metadata()` 자체가 네트워크 마운트에서 막힐 수 있음 — `basic()`도 워커로 옮길지 검토 |
| L30 | UIA 콜백은 임의 스레드 — 창 상태 직접 접근 금지, 불변 스냅샷만. 무클라이언트 시 스냅샷 생성도 생략 | `app/uia.rs:4-7` · `app/win.rs:2089-2091` | 스냅샷 모델 유지 |
| L31 | 토큰 파일: 비원자 저장은 크래시 시 0바이트/반쪽 hex → 재로그인 강요(G10-17). hex 디코드가 멀티바이트 경계에서 panic(G10-04) | `app/secret.rs:43-44` · `:73-75` | `decode_hex` 검증·원자 저장 유지. 테스트 4종 이식 |
| L32 | `Secret` 소거 테스트: drop 후 원주소 재읽기는 UB — macOS·Linux 할당자가 free 리스트 메타데이터를 써서 **실제로 실패**(Windows만 우연 통과) → 살아 있는 버퍼에서 `zeroize()` 확인 | `core/secret.rs:87-92` · `:155-158` | 현 테스트 그대로 이식(이미 3-OS 안전) |
| L33 | 가상 파일 이름은 외부 앱 제공 — 절대 경로·드라이브·`..` 탈출 차단. `cItems`가 실제보다 큰 손상 디스크립터 방어. EOF를 안 주는 소스는 크기 힌트로 중단 | `app/clipboard.rs:288-306` · `:308-309` · `:785-787` | Windows 유지. mac/Linux의 URI 목록 파싱에도 같은 검증(`file://` 외 스킴 기각·상대 경로 기각) |
| L34 | `OleGetClipboard`의 `IDataObject`는 STA 구속 — 워커에서 못 씀 → 스트림만 마샬링. 취소여도 마샬 패킷은 정확히 1회 소비(누수 방지) | `app/clipboard.rs:487-493` · `:497-498` | Windows 유지 |
| L35 | **`same_volume`이 비Windows에서 항상 true**(루트 `/` 비교) → mac/Linux에서 다른 볼륨으로의 드롭이 "이동"으로 판정되어 원본이 지워진다 | `ops/lib.rs:77-91` | **필수 수정**: unix `st_dev` 비교. 회귀 테스트 추가 |
| L36 | winit은 창 스레드에서 OLE를 초기화하고 자체 `IDropTarget`을 등록 — 자체 DnD와 충돌 | (저장소 밖 지식 — 추정) | `with_drag_and_drop(false)` 후 자체 등록. COM 아파트 모드(STA) 일치 확인 |
| L37 | nexa-ctl `ContextMenu`는 호스트 창 안으로 접힘 — 창이 작으면 긴 메뉴가 잘림/스크롤 | `ui-ctl/controls/ctxmenu.rs:526-558` | `set_max_rows` 기본값 설정 또는 별도 팝업 창 결정 |
| L38 | Linux 외부 프로그램 의존 클립보드는 패키지 없으면 기능 소실 — X 프로토콜 직접 참여가 정답 | `sql/clipboard_x11.rs:1-7` | 파일 클립보드도 CLI(`xclip`) 아닌 x11rb 직접 |
| L39 | 아이콘 워커가 `CoUninitialize` 없이 종료하면 워커 수명마다 핸들·GDI 누수 | `ui-fs/shell.rs:187-190` · `:794-795` | 메뉴 스레드·상세 워커·삭제 워커의 COM 초기화/해제 짝 유지(dir2 `details()`는 짝 맞춤: `app/fileinfo.rs:242`·`:294-296`) |

---

## 7. 회귀 테스트 후보

자동화 표기: **U**=순수 단위(3-OS CI) · **I**=통합(임시 폴더·실 FS, 3-OS CI) · **W**=Windows CI 전용 · **M**=수동/`#[ignore]`(실 클립보드·실 휴지통·GUI 세션 필요) · **S**=nexa-sql식 자체 시험 덤프(`*.dump:` 명령)로 자동화 가능.

| # | 시나리오 | 대상 ID | 자동화 | 비고(기존 테스트 이식 여부) |
| --- | --- | --- | --- | --- |
| T01 | 메뉴 모델 합성: 단일 파일/단일 폴더/다중/교차 폴더 선택 × 클립보드 유무 × Shift → 항목 id 목록·순서·활성 상태가 기대와 일치(완전 삭제·이름 복사 위치·폴더에 붙여넣기 조건·새로 만들기 단일 선택 한정) | 002·004·007·008·019·020 | **U** + **S** | 신규(메뉴 모델을 순수 함수로 분리해야 가능) |
| T02 | `ctx_menu_order` 파싱·왕복·그룹 숨김 = 고유 항목 전부 제외 | 019 | **U** | `app/config.rs:1294`·`:1478` 이식 |
| T03 | `same_menu` 재사용 판정(대상·Shift·고유 항목 상태·New 대상) | 015 | **U** | `app/menuthread.rs:338` 이식(W) |
| T04 | 메뉴 닫힌 뒤 포워딩 중단·비메뉴 메시지 통과 | 011·021 | **W** | `app/shellmenu.rs:874`·`:892` 이식(W-a 유지 시) |
| T05 | (Windows) 임시 폴더의 txt 파일로 `prepare_items` → 추출 항목에 verb `open`·`properties`·`delete` 존재, 크래시 없음 | 001·005 | **W**(CI 러너 셸 환경 의존 — 불안정하면 M) | 신규. `ex/ctxmenu_probe.rs`를 스모크로 활용 |
| T06 | 새로 만들기 생성 감지: 폴더 전후 diff — 1개 신규 = Created / 2개 이상 = None / 0개 = 재시도 후 None | 008·009 | **I** | 신규(`detect_created`가 순수 `std::fs` — 3-OS) |
| T07 | 컨텍스트 대상 축소: 캐럿 부모 기준 필터 | 002 | **U** | 신규 |
| T08 | 우클릭 누름/뗌 짝 규약 — 짝 없는 뗌은 메뉴 없음 | 017 | **S** | 신규 |
| T09 | 폴더 서명: 파일 추가 / 내용만 변경 / 같은 크기 덮어쓰기 / 무변경 안정 / 부모 mtime 불변인 자식 추가 / 비폴더 None | 035 | **I** | `app/fsprobe.rs:132`~`:246` 6종 이식(mtime 강제 설정 헬퍼는 unix `utimensat`/`filetime` 대체 필요) |
| T10 | 디바운스 연장 상한(1s) | 032 | **U** | `app/fsprobe.rs:250` 이식 |
| T11 | 감시: 임시 폴더에 파일 생성 → N ms 내 통지 1회 이상 / 폴더 삭제 → `is_alive()==false` 5s 내 / drop이 100ms 내 반환(논블로킹) | 030·033 | **I**(3-OS 백엔드별) | `app/watcher.rs:219` 이식 + 통지·논블로킹 신규 |
| T12 | `sync_watchers` diff: 유지분 재시작 없음·이탈분 중지·죽은 항목 재구독·상한 64 | 031 | **U**(가짜 watcher 주입) | 신규 |
| T13 | 낡은 세대 통지 무시 | 031 | **U** | 신규 |
| T14 | 가상 파일 디스크립터 파싱 + 경로 탈출 차단 + 손상 방어 | 046 | **W** | `app/clipboard.rs:1050` 이식 |
| T15 | 가짜 `IDataObject` → 동기 추출(충돌 " (2)"·하위 폴더·크기 미상 스트림 100KB) | 046·065 | **W** | `app/clipboard.rs:1177` 이식 |
| T16 | 가상 붙여넣기 워커 왕복(300KB 다중 청크·진행 바이트·세그먼트 상태) | 047 | **W** | `app/clipboard.rs:1223` 이식 |
| T17 | 클립보드 열기 재시도 의미(첫 성공·n번째·상한) | 045 | **U** | `app/clipboard.rs:1275` 이식 |
| T18 | 실 클립보드 파일 목록 왕복(복사/잘라내기 판정·비ASCII·비우기) | 040·041·043 | **M**(3-OS 각각) | `app/clipboard.rs:1363` 이식 + mac/Linux판 신규 |
| T19 | 파일 클립보드 **직렬화 순수 함수**: CF_HDROP 바이트 / `text/uri-list` / `x-special/gnome-copied-files`(copy·cut) 인코딩·디코딩 왕복(공백·한글·`#`·`%` 포함 경로) | 040·041 | **U** | 신규(OS 호출과 분리해야 3-OS CI 가능) |
| T20 | 잘라내기 표시 집합: Move 목록 → 집합 = 경로들 / Copy·텍스트·비움 → 빈 집합 / 변경 여부 반환 | 044 | **U**(읽기 함수 주입) | 신규 |
| T21 | 붙여넣기 대상 규칙(폴더 1개·파일 1개·다중·없음) | 042 | **U** | 신규 |
| T22 | RTF 모노 변환: `\`·`{`·`}` 이스케이프·비ASCII `\uN?`(음수 포함)·서로게이트 쌍·줄 → `\par` | 051 | **U** | 신규(`to_rtf_mono`에 기존 테스트 없음) |
| T23 | DnD 연산 판정: Ctrl/Shift/같은 볼륨/다른 볼륨/자기·하위 금지/가상 = 항상 복사 | 062 | **U** | 신규(`op_for`·`resolve`를 순수 함수로) |
| T24 | `same_volume` — unix에서 서로 다른 `st_dev`면 false | 062 | **I**(mac/Linux — tmpfs 등 마운트 필요 시 조건부) | 신규(§6-L35 결함 방지) |
| T25 | 지연 소스 판정·집합 비교·임시 폴더 접두(구성요소 단위·대소문자) | 064 | **U** | `app/dnd.rs:533`·`:544`·`:557` 이식(대소문자는 OS별 기대값) |
| T26 | 스테이징 확보: TEMP 안 = rename + 원위치 소멸 / TEMP 밖 = 미확보 / 거부 시 원복·스테이징 비움 | 064·068 | **I** | `app/dnd.rs:580`·`:600` 이식 |
| T27 | 스테이징 판정·분리·슬롯 정리 | 069 | **U/I** | `app/win.rs:9873-9876` 이식 |
| T28 | 발신 데이터 객체 CF_HDROP 왕복 | 067 | **W** | `app/dnd.rs:514` 이식 |
| T29 | DnD 호버: 같은 후보 `dnd_hover_ms` 경과 → 발동 1회 후 리셋 / 후보 변경 → 대기 재시작 / 탭 우선 | 066 | **U**(시계 주입) | 신규 |
| T30 | 휴지통 왕복: 삭제 → 원위치 비움 → 복원 → 내용 동일 | 070·073 | **M**(Win/mac) · **I**(Linux — `XDG_DATA_HOME`을 임시 폴더로 바꿔 격리하면 CI 가능) | `app/recycle.rs:205` 이식 + Linux판 신규 |
| T31 | (Linux) trashinfo 형식: `Path=` URL 이스케이프·`DeletionDate` 형식·이름 충돌 시 접미·다른 마운트 topdir 선택 | 070 | **U/I** | 신규 |
| T32 | 삭제 사후 diff: 성공분만 undo 기록·실패분 선택·재시도 대상 = 실패분 | 072 | **U**(삭제 함수 주입) + **S** | 신규 |
| T33 | 삭제 동시 1잡 — 진행 중 두 번째 요청 무시 | 070 | **S** | 신규 |
| T34 | 완전 삭제 확인 기본 버튼 = 취소 측 | 074 | **S** | 신규 |
| T35 | 크기 표기·천 단위·클러스터 올림·proplist 파싱 | 081~083 | **U** | `app/fileinfo.rs:378`·`:388`·`:431`·`:440` 이식 |
| T36 | `basic()` 파일/폴더/부재 + (unix) `size_on_disk = blocks*512 ≥ 0` | 080·081 | **I** | `app/fileinfo.rs:455` 이식 |
| T37 | 상세 워커: 최신 요청 전달·레인 독립·panic 격리 | 084 | **I** | `app/fileinfo.rs:396` 이식 |
| T38 | `wants_details` — 폴더·플레이스홀더 제외 | 085 | **U** | `app/fileinfo.rs:414` 이식 |
| T39 | 상세 출력 형식: 그룹 머리글은 속성 앞에만·연속 그룹 없음·마지막이 그룹 아님·기본 중복 제외 | 083 | **W** | `app/fileinfo.rs:475` 이식 |
| T40 | 접근성 스냅샷: 가시 행 수·rect·focused/selected 반영, 선택 요청 범위 밖 인덱스 무시 | 090~092 | **U**(스냅샷 생성 순수화) | 신규 |
| T41 | 토큰 왕복·덮어쓰기·제거·꼬리 정리·손상 hex·비ASCII | 095 | **I**(보호 백엔드별) | `app/secret.rs:174`~`:254` 5종 이식 |
| T42 | `Secret` Debug 비노출·원본 소거·u16 경유·clone 독립 | 096 | **U** | `core/secret.rs:122`~`:163` 4종 이식 |
| T43 | 우클릭 표시 지연 ≤ 50ms(선행 구축 적중 시) | 014·015 | **M**(`nexa-dir2/scripts/ctxmenu-timing.ps1` 재사용) | 성능 회귀 지표 |
| T44 | 탐색기 ↔ 앱 복사/잘라내기/붙여넣기·드래그 양방향(실기) | 040·041·060·067 | **M** | 3-OS 각각의 파일 관리자(탐색기·Finder·Nautilus/Dolphin)로 체크리스트화 |
| T45 | 7-Zip(Win)·file-roller(Linux)·Archive Utility(mac) 드래그 추출 전량 생존 | 064 | **M** | L19 재현 |
| T46 | OneDrive/iCloud/Google Drive 폴더에서 원격 생성분 자동 반영(≤ 30s) | 034~036 | **M** | L13 재현 |

**핵심 기능 즉시 점검(헬스 체크) 후보** — 앱 기동 시 또는 진단 명령으로 1회 실행해 실패를 바로 드러내는 자기 점검(수 ms, 부작용 없음):
1. 임시 폴더에 파일 생성 → 감시 통지 수신 여부(감시 백엔드 생존).
2. `fsprobe` 서명 변화 감지(프로브 보험 생존).
3. 클립보드 텍스트 자기 왕복(쓰기 → 읽기 일치 — 사용자 클립보드를 덮으므로 진단 명령에서만).
4. 휴지통 가용성(Windows: 드라이브 휴지통 조회 / macOS: `~/.Trash` 접근 / Linux: `Trash/files`·`Trash/info` 생성 가능).
5. 토큰 보호 왕복(`protect`→`unprotect` 임시 값).
6. 메뉴 공급자 스모크(임시 파일 1개로 항목 ≥ 1개 — Windows는 메뉴 스레드 생존 포함).
7. 기본 연결 프로그램 조회 경로 가용성(`xdg-open`/`open` 존재, Windows는 항상 참).
