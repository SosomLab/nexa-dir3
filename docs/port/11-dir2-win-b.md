# 11. nexa-dir2 인벤토리 — `win.rs` B구간(3250~6550줄)

> 접두사 **WINB-NNN**. 이 문서의 ID는 이후 구현·교차 검증의 체크리스트다.
> 줄 표기 약속: 별도 저장소 표기가 없는 `win.rs:줄`은 모두 `nexa-dir2/crates/nexa-app/src/win.rs:줄`이다.
> "추정"이라고 적은 항목은 이 구간 밖 소스를 열어 확인하지 않은 것이다.

## 0. 범위 — 읽은 파일과 줄 수

| 대상 | 읽은 범위 | 비고 |
|---|---|---|
| `nexa-dir2/crates/nexa-app/src/win.rs` | **3250~6557줄 전부**(약 3,300줄, 구간별 연속 읽기) | 담당 구간. 끝(6550)은 `apply_prefs`(6475~) 중간 — 6558줄 이후는 C구간 문서 소관 |
| 같은 파일 | 40~285줄(상수·타이머·`WM_APP_*`·`CMD_*`), 3180~3249줄(`delete_to_recycle_bin`), 7019~7063줄(`plock`·`post_final_notify`), 9700~9951줄(단위 테스트) | 담당 구간이 참조하는 정의 확인용 |
| 같은 파일 | 전체 `fn` 목록·`const` 목록(Grep) | 구조 파악(함수 약 230개 중 본 구간 105개) |
| `nexa-dir2/crates/nexa-app/src/dialog.rs` | 170~200, 600~640, 720~770줄 + 공개 시그니처 Grep | `show_buttons`·`Progress` 계약 확인 |
| `nexa-dir2/crates/nexa-app/src/{tip,fsprobe,recycle,watcher,shellnotify,secret,launcher,ordereditor,bulkrename}.rs` | 공개 시그니처 Grep만 | 본문은 다른 인벤토리 소관 |
| `nexa-dir2/crates/nexa-gui/src/widgets/{rows,chrome,menubar}.rs` | 공개 `fn` Grep만 | dir2 위젯 API 이름 확인 |
| `nexa-ui/crates/nexa-ctl/src/**` | `pub struct/enum/trait` 전수 Grep, `controls/{pulldown,toolbar,tabbar,splitter,ctxmenu,timeout_button,flash,tree}.rs`의 `pub fn` Grep, `pulldown.rs:1-160`·`toolbar.rs:20-130`·`combo.rs:40-115` 읽기 | §3 매핑의 실재 확인 |
| `nexa-ui/crates/{nexa-dlg,nexa-fs,nexa-sys,nexa-conf}/src` | 공개 API Grep | 대화상자·휴지통·클립보드 부재 확인 |
| `nexa-sql/crates/nexa-sql/src` | 파일 목록, `clipboard.rs`·`winhost.rs`·`toast.rs`·`imestate.rs` 시그니처 Grep, `Cargo.toml` 의존성 | 기준 구조(winit 0.30 + softbuffer 0.4 + `EventLoopProxy` `Wake`) 확인 |

본 구간의 성격: **창 프로시저(`wndproc`, C구간)가 호출하는 "동작 본체" 함수 모음**이다. DnD 훅, 실행 취소, 폴더 감시·프로브, 삭제, 이름 바꾸기, 새로 만들기, 클라우드 전송, 로컬 전송 엔진 UI측, IME, 페인트, 상태바 동기 길목, 명령 디스패처(`run_command`), 테마·언어, OAuth, 툴팁, 탭 교차 이동, 터미널 입력 보조, 우클릭 팝업, 설정·일괄 이름변경 창 진입, 설정 적용(전반부)이 들어 있다.

## 1. 기능 목록

이식 분류: **N**=플랫폼 중립(거의 그대로) / **A**=nexa-ui 컨트롤·그리기로 교체 / **P**=OS별 구현 분기 필요 / **W**=Windows 전용 유지(타 OS는 대체·비활성). 둘 이상이면 병기.

### 1.1 끌어서 놓기(DnD) 수신 훅 — `dnd.rs`가 호출

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-001 | 드롭 대상 폴더 결정 | 화면 좌표→클라이언트 변환 → `panel_at(x)` → 행이 **폴더면 그 폴더**, 파일 행·빈 본문이면 **패널 현재 폴더**. 패널 밖이면 `None`(드롭 거부) | `win.rs:3252` `drop_dest_at` | `ScreenToClient` | P(좌표계)·N(판정) | 없음 |
| WINB-002 | 외부(탐색기 등) 파일 드롭 확정 | 전송 중이면 **시작하지 않고 `false` 반환** + 제목줄 `ops.busy`(dnd.rs가 확보 파일을 원위치로 되돌림). 아니면 `start_transfer` 합류(진행·취소·undo·재로드) 후 `true` | `win.rs:3273` `handle_external_drop` | 없음(상위 OLE) | P | 없음 |
| WINB-003 | 가상 파일 드롭(Outlook 첨부·zip 내부·MTP) | CF_HDROP 없는 소스의 스트림 추출. **Drop 반환 전 동기 실행**(소스 생존 보장)이라 대용량은 창 멈춤(알려진 한계). 대상이 클라우드면 무동작. 대기 커서 표시. 생성물은 `record_vpaste_undo`로 Ctrl+Z 대상 + `reload_both` | `win.rs:3294` `handle_virtual_drop` | `IDataObject`, `LoadCursorW(IDC_WAIT)`, `SetCursor`, `clipboard::extract_virtual_from` | W(타 OS는 파일 프라미스 등 대체 — §4) | 없음 |
| WINB-004 | 드래그 추적 시작/갱신 | DragEnter/DragOver마다 호출. 클라이언트 좌표 변환 후 **`TIMER_DND` 100ms 폴링 무장**(정지 커서에서 OLE DragOver가 멎어도 대기 경과·연속 스크롤 판정) → `dnd_track_update` | `win.rs:3326` `dnd_track`, 상수 `win.rs:100-101` | `ScreenToClient`, `SetTimer` | P | 없음 |
| WINB-005 | 드래그 이탈/종료 | `TIMER_DND` 해제 + `dnd_hover = None` | `win.rs:3336` `dnd_leave` | `KillTimer` | P | 없음 |
| WINB-006 | 드래그 중 엣지 자동 스크롤 | 커서 아래 패널 본문 상/하단 엣지면 **1행 스크롤**, 폴링 반복으로 연속 스크롤 | `win.rs:3345-3350` (`rows_mut().drag_scroll_edge`) | 없음 | A(목록 위젯 API) | 없음 |
| WINB-007 | 드래그 호버 대기 판정 | 후보 우선순위 **비활성 탭 > 접힌 폴더 행**. 같은 후보 위에 `dnd_hover_ms`(설정, 음수는 0 처리) 이상 머물면 발동 후 대기 리셋. 후보가 바뀌면 그 시각부터 새로 대기, 후보 없으면 해제. 발동 시 `update_title("")`+`update_status`(경로 변경 길목) | `win.rs:3351-3391`, `enum DndHover` `win.rs:3317` | 없음 | N | 없음 |
| WINB-008 | 호버 발동 | `Tab(패널,탭)`=그 탭으로 전환. `Folder(패널,행,경로)`=행의 현재 경로가 예약 경로와 **같을 때만** 펼침(스크롤·재정렬로 밀렸으면 무시 → 다음 폴링이 다시 대기) | `win.rs:3395` `dnd_hover_fire` | 없음 | N | 없음 |

### 1.2 실행 취소·다시 실행

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-009 | 휴지통 삭제의 undo/redo 단위 | `DeleteBatchOp{paths,description}`. undo=`recycle::restore_by_original_paths`(복원 수 < 대상 수면 `OpError::Failed(부족분)`). redo=아직 존재하는 경로만 다시 휴지통(없으면 Ok) | `win.rs:3414-3450` | 셸 undelete(`recycle.rs:32`), `SHFileOperationW` | P | 없음 |
| WINB-010 | 가상 붙여넣기·DnD 스테이징 생성물의 undo 단위 | `VPasteOp` = DeleteBatchOp의 **역방향**: undo=생성물 휴지통 삭제, redo=휴지통 복원 | `win.rs:3455-3491` | 위와 동일 | P | 없음 |
| WINB-011 | 단일 경로 휴지통 삭제 주입 함수 | `CreateOp`·`CopyBatchOp`의 undo용. 실패 문구 `"휴지통 삭제 실패"`는 **하드코딩 한국어**(i18n 아님) | `win.rs:3494` `recycle_delete_one` | `delete_to_recycle_bin`(`win.rs:3182`) | P | 없음 |
| WINB-012 | 오류 사유 문구화 | `OpError::Failed(n)`→`history.failedItems`, `MissingSource(name)`→`history.missingSource`, `NameExists(name)`→`history.nameExists` | `win.rs:3503` `op_error_text` | 없음 | N | 없음 |
| WINB-013 | Ctrl+Z / Ctrl+Y / 편집 메뉴 | 스택 비었으면 재로드 없이 제목 노트 `undo.none`/`redo.none`. 성공=`undo.done`/`redo.done {설명}`, 실패=`undo.fail`/`redo.fail {설명}{사유}`. **실패해도 항상 `reload_both`**(일부 수행 가능). 실패 연산은 스택에서 소실(무결성 우선) | `win.rs:3514` `do_undo_redo` | 없음 | N | 없음(코어 `nexa-ops::history` 테스트는 추정) |

### 1.3 폴더 감시·프로브·재로드

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-014 | 패널별 폴더 watcher 동기화 | 대상 = 현재 폴더 + **가시 펼침 폴더**, 패널당 상한 `WATCH_CAP=64`(초과분 비감시=F5 폴백). **diff 재구독**: 유지분 그대로, 이탈분 drop, 신규만 시작. **죽은 watcher(`is_alive()==false`)도 솎아 재구독**. 신규마다 `watch_gen += 1`(낡은 스레드 통지 무시). 시작 실패(권한·가상 루트)는 그 폴더만 비감시 | `win.rs:3545-3572` `sync_watchers` | `watcher::DirWatcher::start`(ReadDirectoryChangesW 추정) → `WM_APP_FSCHANGE` | P | 없음 |
| WINB-015 | 셸 변경 통지 구독 | 패널 활성 탭 루트가 바뀌면 재등록(재귀). 가상 루트·클라우드 센티널은 해제 유지. 메시지 `WM_APP_SHCHANGE_BASE + 패널` | `win.rs:3573-3590` | `shellnotify::ShellWatch::register`(SHChangeNotifyRegister 추정) | W(타 OS 대체 §4) | 없음 |
| WINB-016 | 재로드 디바운스 무장(상한 있음) | 첫 무장 시 `watch_since[panel]=now` + `TIMER_WATCH_BASE+panel` 300ms. 이후 통지는 `fsprobe::debounce_should_extend(since, now, WATCH_MAX_MS=1000)`일 때만 재무장(연장). 상한 초과면 재무장 안 함 → 걸린 타이머가 만료(폭주 중에도 갱신). 만료 처리가 `watch_since=0`(C구간) | `win.rs:3597` `arm_watch_debounce`, 상수 `win.rs:97,107,112` | `SetTimer` | P(타이머)·N(판정) | `fsprobe.rs:115` 테스트 존재 추정 |
| WINB-017 | 루트 폴더 프로브 폴링 | 패널 폭 ≤0(싱글의 숨은 쪽)은 건너뜀. `fsprobe::probe(root)` 서명이 직전과 **같은 경로에서 달라졌을 때만** 발화. 경로가 바뀐 직후는 기준선만. **서명은 비교 즉시 갱신**(재로드가 미뤄져도 매 틱 재무장 금지). 가상 루트는 `probe=None` | `win.rs:3626-3644` `poll_fs_probe` | 파일시스템 stat·열거(중립) | N | 없음 |
| WINB-018 | 뷰포트 폴더 서명 점검 | `viewport_dirs(false)`(접힌 폴더 포함) 각각을 프로브. 처음 보는 폴더=기준선만, 화면을 떠난 폴더는 맵에서 소거, 소실·비폴더는 비교 제외. 하나라도 변하면 `arm_watch_debounce` | `win.rs:3645-3669` | 중립 | N | 없음 |
| WINB-019 | 프로브 기준선 재수립(발화 없음) | 방금 끝난 열거 결과를 기준선으로. 보이는 패널만 | `win.rs:3675` `refresh_probe_baseline` | 중립 | N | 없음 |
| WINB-020 | 화면 복귀 시 갱신 | 앱 활성화·최소화 복원 공용: `reload_both("")` + `TIMER_FSPOLL` 3000ms 재무장 | `win.rs:3695` `refresh_on_return`, 상수 `win.rs:120,128,132` | `SetTimer` | P(타이머)·N | 없음 |
| WINB-021 | 양쪽 패널 재로드 | 두 패널 `reopen_filtered(nav_ctx)` → `refresh_probe_baseline` → flush → `update_title(note)` → `update_status` | `win.rs:3703` `reload_both` | 없음 | N | 없음 |

### 1.4 삭제

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-022 | 클라우드 항목 삭제 | 대상 첫 항목이 클라우드면 **같은 연결 인덱스의 항목만** `WriteOp::Delete`로 모아 `start_cloud_write(…,"cloud.deleting")`. 서비스 휴지통 이동, 로컬 undo 불가 | `win.rs:3721-3731` (`do_delete`) | 없음 | N | 없음 |
| WINB-023 | Shift+Del 완전 삭제 | 확인창 필수: 제목 `del.title`, 본문 `del.confirm {개수}`, **예/아니오 + 경고 아이콘 + 기본 버튼=아니오**. 예가 아니면 중단. 항목별 `nexa_ops::delete_permanent`(개별 격리 — 실패해도 계속). 노트 `del.done {del.kindPermanent}{성공수}` + 실패 있으면 `ops.errors {n}`. **undo 기록 없음**(설계상) | `win.rs:3716-3765` `do_delete` | `MessageBoxW(MB_YESNO+MB_ICONWARNING+MB_DEFBUTTON2)` | A(확인창)·N(삭제) | 없음 |
| WINB-024 | Del 휴지통 삭제 — 잠금 사전 프로브 모달 | 확인 없음. 진행 중 삭제가 있으면(`pending_delete`) 무시. 루프: `probe_locked` → 잠긴 것 있으면 모달(제목 `del.lockedTitle`, 본문 `del.lockedMsg {잠긴수}{이름목록}`), 버튼 ①`del.skipLocked {나머지수}`(나머지>0일 때만) ②`del.retry` ③`del.cancel`. ①=잠긴 것 제외 후 진행, ②=재프로브, 그 외/닫힘=**아무것도 안 지움** | `win.rs:3772-3814` `start_recycle_delete` | `dialog::show_buttons` | A(모달)·P(잠금 판정) | 없음 |
| WINB-025 | 휴지통 삭제 워커 + 낙관적 숨김 | `pending_delete=Some(targets)` → 스레드에서 COM STA 초기화 후 배치 삭제 → `post_final_notify(WM_APP_DELETE, ok, 0)`. UI는 즉시 양 패널 `hide_paths` + 제목 `del.progress` + `update_status` | `win.rs:3815-3843` | `CoInitializeEx`/`CoUninitialize`, `SHFileOperationW(FO_DELETE+FOF_ALLOWUNDO+FOF_NOCONFIRMATION+FOF_SILENT)` | P | 없음 |
| WINB-026 | 삭제 가능 사전 프로브 | `DELETE` 권한 + 전체 공유로 열어 `ERROR_SHARING_VIOLATION`이면 잠김. 폴더는 자기 자신만(하위 재귀 없음). 부재·권한 오류=잠김 아님 | `win.rs:3849` `probe_locked` | `CreateFileW(DELETE, FILE_SHARE_*, FILE_FLAG_BACKUP_SEMANTICS)` | W(Unix는 잠금 개념 없음 — 빈 결과) | 없음 |
| WINB-027 | 모달용 파일명 목록 | 최대 10줄 + 초과 시 `del.listMore {나머지}` 1줄, 줄바꿈 연결 | `win.rs:3883` `name_list` | 없음 | N | 없음(순수 함수 — 테스트 후보) |
| WINB-028 | 실패 항목 선택 강조 | 재로드 후 활성 패널에서 실패 경로 행을 첫 항목 `Single`, 이후 `Toggle`로 선택. 행 미발견 무시 | `win.rs:3898` `select_paths` | 없음 | A(목록 API) | 없음 |
| WINB-029 | 삭제 완료 처리(사후 diff 백스톱) | 워커 반환값은 무시. **원 위치 잔존 여부**로 성공/실패 분할. 성공분만 `DeleteBatchOp` 기록(`del.recycleOp {n}`). 노트 `del.done {del.kindRecycle}{n}` + 실패 시 `del.partialFail {n}`. `reload_both`+`update_status`. 실패가 있으면 선택 강조 후 모달(제목 `del.failTitle`, 본문 `del.failMsg {n}{목록}`, 버튼 `del.retry`/`del.close`) → 재시도=실패분만 `start_recycle_delete` 재진입 | `win.rs:3926` `on_delete_message` | `dialog::show_buttons` | A·N | 없음 |

### 1.5 이름 바꾸기·새로 만들기

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-030 | 느린 재클릭 리네임 만료 판정 | 예약 `(패널, 경로)`가 **현재** 활성 패널·캐럿 행 경로와 일치할 때만 발화. 비교는 끝 구분자(`\`,`/`) 제거 + ASCII 대소문자 무시 | `win.rs:3983` `rename_timer_should_fire` | 없음(경로 규약은 §4-23) | N(대소문자 정책은 P) | `win.rs:9729` |
| WINB-031 | 예약 리네임 폐기 | 새 클릭·우클릭·키·명령은 모두 폐기 사유. `pending_rename` 비우고 `TIMER_RENAME` 해제 | `win.rs:3999` `cancel_pending_rename`, 상수 `win.rs:51,77` | `KillTimer` | P(타이머) | 없음 |
| WINB-032 | F2 인라인 이름 바꾸기 시작 | 활성 패널 캐럿 행의 표시 텍스트로 `begin_rename`. 캐럿 없으면 무동작 | `win.rs:4005` `begin_rename_caret` | 없음 | A | 없음 |
| WINB-033 | 이름 바꾸기 확정 | 클라우드=`WriteOp::Rename`(`cloud.renaming`). 로컬: **`.lnk`는 확장자 숨김** — 입력에 `.lnk`가 없으면 복원. 성공+경로 변경 시 펼침 집합 접두사 치환(`rename_expanded`) + `RenameOp` undo 기록(설명 `rename.done {옛이름}{새이름}`). 동일 이름은 기록 없음. 실패=`rename.fail {오류}`. 항상 `reload_both` | `win.rs:4019` `apply_rename` | 없음(`.lnk` 규칙은 Windows 관례) | N(`.lnk` 분기만 W) | 없음 |
| WINB-034 | 새 폴더/새 파일 | 클라우드 경로: 폴더만 지원(파일은 `cloud.err.foldersOnly`), `WriteOp::NewFolder{name=new.folderBase}`(`cloud.creating`). 로컬: `create_new_dir(dir, new.folderBase)` / `create_new_file(dir, "{new.fileBase}.txt")`. 실패=`new.fail {오류}`. 성공=`CreateOp` undo 기록(undo=휴지통 삭제, redo=재생성: 폴더 `create_dir`, 파일 `create_new`) 후 `focus_created_and_rename` | `win.rs:4068` `create_new` | 휴지통(undo) | N·P(undo) | 없음 |
| WINB-035 | 생성 직후 캐럿 이동 + 리네임 진입 | `reload_both` → 경로로 행 탐색. 못 찾으면 부모 행을 `hover_expand`(접힘일 때만) 후 재탐색. 찾으면 `begin_rename(row, leaf_name)`. 미발견은 조용히 생략. 셸 "새로 만들기" 결과(`shellmenu::Outcome::Created`)와 공용 | `win.rs:4127` `focus_created_and_rename` | 없음 | A | 없음 |

### 1.6 클라우드 전송

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-036 | 워커용 연결 정보 스냅숏 | API 연결만. `client_id`/`client_secret`은 설정 우선 → 내장 기본값. refresh 토큰은 `secret::load_token(idx)`(없으면 `None`) | `win.rs:4161` `cloud_conn_info` | DPAPI(`secret.rs`) | P(토큰 보관) | 없음 |
| WINB-037 | 클라우드 임시 폴더 | `temp_dir()/NexaDir/cloud` | `win.rs:4182` `cloud_temp_dir` | 없음 | N | 없음 |
| WINB-038 | 클라우드 → 로컬 다운로드 | 같은 연결의 항목만(혼합은 첫 연결). 이름 빈 항목 제외. 폴더 여부·크기는 열린 트리에서 읽음. 토큰 없으면 `cloud.err.noToken` 후 **`true` 반환**(호출자 폴백 금지). 제목 `cloud.downloading {n}`. **대상 폴더 지정 + `transfer_close_ms>0`일 때만 진행 창**(임시 폴더 다운로드=더블클릭 열기는 창 없음). `dest_dir=None`이면 받은 뒤 연결 프로그램으로 열기 | `win.rs:4190` `start_cloud_download` | `dialog::Progress::open` | A(진행 창)·N | 없음 |
| WINB-039 | 클라우드 진행 갱신 + 취소 폴링 | `WM_APP_CLOUD_PROGRESS`·`TIMER_CLOUD_POLL`(200ms) 공용. 세그먼트 스냅숏, `cur`=Active 항목 번호(없으면 Pending 아닌 수). 창 취소 → `shared.cancel=true`. 제목 `ops.progress {pct}`(총량 0이면 0) | `win.rs:4255` `on_cloud_progress`, 상수 `win.rs:105-106` | 없음 | A·N | 없음 |
| WINB-040 | 클라우드 전송 개시/마감 | 개시: `cloud_shared` 걸고 폴링 타이머 켬. 마감: 해제 + 타이머 끔 + 진행 창을 `set_done(note, max(transfer_close_ms,1))`로 바꿔 `transfer_close` 슬롯에 넣고 `TIMER_PROG_CLOSE`를 `ms+500`으로 무장 | `win.rs:4290-4310` | `SetTimer`/`KillTimer` | P(타이머)·A | 없음 |
| WINB-041 | 트리에서 (폴더 여부, 크기) 조회 | 양 패널의 가시 행을 선형 탐색(추가 네트워크 요청 없음) | `win.rs:4314` `cloud_row_info` | 없음 | N | 없음 |
| WINB-042 | 클라우드 쓰기 공용 시작 | 업로드·삭제·이름 변경·새 폴더. 토큰 없으면 안내 후 `true`. 제목 `{busy_key} {n}`. **바이트를 옮기는 작업(Upload/UploadTree)에만 진행 창** | `win.rs:4326` `start_cloud_write` | 없음 | A·N | 없음 |

### 1.7 로컬 전송(복사·이동) 엔진의 UI측

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-043 | 전송 시작 가드 | 원본 비었으면 무동작. 전송 중이면 `ops.busy` 안내(조용한 무시 금지). **동시 1잡** | `win.rs:4364-4377` `start_transfer` | 없음 | N | 없음 |
| WINB-044 | 대상이 클라우드일 때 경로 | ① 원본도 클라우드·**같은 연결**=서버 사이드 `MoveWithin`/`CopyWithin` ② **계정 간**=`start_cross_copy`(임시 폴더 경유, 원본은 첫 연결 기준, 진행 창은 `transfer_close_ms>0`일 때, 제목 `cloud.copying {n}`) ③ 로컬→클라우드=폴더 `UploadTree`/파일 `Upload`(그 외 종류 제외). ops가 비면 `cloud.err.filesOnly`. busy 키: 원본 클라우드 `cloud.copying`, 아니면 `cloud.uploading` | `win.rs:4378-4497` | 없음 | N | 없음 |
| WINB-045 | 원본만 클라우드 | 복사·이동 무관하게 **다운로드**(원본 삭제 없음) | `win.rs:4498-4503` | 없음 | N | 없음 |
| WINB-046 | 충돌 확인 4버튼 모달 | 워커 스레드에서 모달 표시(자체 메시지 루프, UI 스레드는 계속 펌프). 제목 `ops.overwriteTitle`, 본문 `ops.overwrite`의 `{0}`=충돌 파일 leaf. 버튼 ①`ops.yes`=이 파일만 덮어쓰기(다음 충돌 재질문) ②`ops.yesAll`=이후 무확인 ③`ops.skip`=이 파일만 건너뜀 ④`ops.cancel`/닫힘=**전체 취소**(cancel 플래그) + Skip. i18n 문구는 UI 스레드에서 선확정 | `win.rs:4518-4577` | `dialog::show_buttons`(워커 스레드) | A·P(스레드 모델 — §6-3) | 없음 |
| WINB-047 | 전송 이벤트 → 공유 상태 → 통지 | `Plan{sizes,total}`=세그먼트 목록 생성+게시. `ItemStart`=`in_flight` 기록·`item_base` 저장·Active(게시 안 함). `Bytes`=done/total 저장·항목 done=`done-item_base`(size 상한)·게시(4MB 청크 단위). `ItemEnd`=Done(=size 채움)/Skipped/Failed·`in_flight=None`·게시. 종료 시 `outcome` 저장 후 **`post_final_notify(WM_APP_TRANSFER, gen, 1)`** | `win.rs:4578-4636` | `PostMessageW` | P(통지 채널)·N | 없음 |
| WINB-048 | 진행 창 열기 | `transfer_close_ms>0`이면 비모달 진행 창(제목 `ops.progressTitle`, 라벨 `ops.progressLabel`), 0이면 창 없이 제목줄 %만. `TransferJob{shared,gen,op,progress,item_count}` 저장, 제목 `ops.progress 0` | `win.rs:4637-4657` | `dialog::Progress::open` | A | 없음 |
| WINB-049 | 진행 통지 처리 | 세대 불일치(낡은 워커)는 무시. pct=`done*100/total`(total 0이면 **100**). 진행 창 갱신(`count`=세그먼트 수 또는 `item_count`), 취소 폴링 → cancel 플래그. 제목 `ops.progress {pct}` | `win.rs:4660-4703` `on_transfer_message` | 없음 | A·N | 없음 |
| WINB-050 | 완료 — 진행 창 카운트다운 닫기 | 마지막 스냅숏 반영 후 `set_done(ops.doneClosing, max(ms,1))`, `transfer_close`에 넣고 `TIMER_PROG_CLOSE` `ms+500`(백스톱) | `win.rs:4704-4728` | `SetTimer` | A·P | 없음 |
| WINB-051 | 완료 — undo 기록 | 수행된 `(원본,대상)` 쌍만(취소돼도 기록). **DnD 스테이징 출신**(`%TEMP%/NexaDir/dnd-*`)은 `VPasteOp`(설명 `op.moveCount`)로 기록 + 빈 슬롯 폴더 정리. 일반 쌍은 Move=`MoveBatchOp`(`op.moveCount {n}`), Copy=`CopyBatchOp`(`op.copyCount {n}`, undo 삭제=휴지통) | `win.rs:4729-4760` | 휴지통 | N·P | `win.rs:9852`, `win.rs:9892` |
| WINB-052 | 완료 — 재로드·결과 노트 | 양 패널 `reopen_filtered`. 노트 `ops.done {n}` · `ops.skipped {n}` · `ops.errors {n}` · `ops.canceled`를 ` · `로 연결. `update_status` | `win.rs:4761-4779` | 없음 | N | 없음 |

### 1.8 IME·페인트·상태바 길목

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-053 | IME 조합 창을 편집 캐럿 옆에 배치 | 대상 우선순위: 편집 중 경로바(활성 패널 → 반대 패널) → 활성 패널 인라인 리네임 필드. 위치 x=`min(field.x+pad+캐럿앞텍스트폭, field.right-pad)`, y=`field.y+2`. 대상 없으면 무동작(IME 기본 위치) | `win.rs:4785` `position_ime` | `ImmGetContext`/`ImmSetCompositionWindow(CFS_POINT)`/`ImmReleaseContext` | P | 없음 |
| WINB-054 | 페인트 — 패널 | 좌 패널, 우 패널(폭>0일 때만). **싱글 정보 모드**면 좌 도크를 우 패널 뒤에 재도장 | `win.rs:4820-4840` `paint` | `BeginPaint`, `DwBackend` | A·P(표면) | 없음 |
| WINB-055 | 페인트 — 도크 터미널 | 도크 표시 + 높이>0 + 종류=2(터미널)인 패널에 셀 그리드 직접 렌더. 캐럿 깜빡임은 키 포커스일 때만(비포커스 상시 표시). 팔레트=`nexa_term::resolve_scheme(term_theme, dark, light, is_dark)`. **PTY 기동 실패 + 터미널 포커스면 포커스 해제**(키 삼킴 방지). `view_frac>0`(픽셀 스크롤)이면 종류 스트립 재도장 | `win.rs:4841-4885` | `term_paint`(ConPty 기동 포함) | A·P(PTY) | 없음 |
| WINB-056 | 페인트 — 스플리터 3종 | 파일 좌/우 스플리터(**폭>0일 때만** — 싱글 패널은 음수), 도크 좌/우 스플리터(동일 가드), 파일↔도크 가로 분리선. 상세 수치는 §2-2 | `win.rs:4886-4925` | `fill_rect` | A | 없음 |
| WINB-057 | 페인트 — 크롬 순서 | 도구 모음 → 런처 바(높이>0) → 상태바 → **메뉴바 마지막**(드롭다운 오버레이가 위) → 백버퍼 전송 | `win.rs:4926-4933` | `BitBlt` | A·P | 없음 |
| WINB-058 | 페인트 — 후처리 | 통계 누적·첫 렌더 시각 기록. 탭 바 줄 수 변화 감지 시 재레이아웃(1프레임 지연 수렴). 아이콘 로딩 대기 있으면 `TIMER_ICONS` 무장. 첫 프레임과 20프레임마다 제목 갱신 | `win.rs:4935-4955` | `SetTimer`, `EndPaint` | N·P | 없음 |
| WINB-059 | 상태 동기 — 보기 옵션 미러 | 활성 탭 값(숨김·닷파일·폴더 우선)을 `State` 미러로 복사. 각 패널 활성 탭이 stale이면 재열람 | `win.rs:4958-4976` `update_status` | 없음 | N | 없음 |
| WINB-060 | 상태바 문구 | 좌: `[{panel.left 또는 panel.right}] {status.itemCount n}` + 선택>0이면 ` · {status.selectedCount n}` + ` · {status.tab 현재/전체}`. 우: `{status.filters 숨김표시,닷파일표시}` · `{status.avg 평균µs}` | `win.rs:4977-5010` | 없음 | A(상태바) | 없음 |
| WINB-061 | 상태 동기 — 부수 길목 | 도크 정보 갱신 → 스크린리더 통지(`uia_notify`) → `sync_watchers` → **경로가 바뀐 보이는 패널만** 프로브 기준선 즉시 수립(가상 루트·클라우드 제외) + `sub_probe` 비움 → 클라우드 배지 동기 | `win.rs:5011-5038` | UIA(W) | N·W | 없음 |
| WINB-062 | 상태 동기 — 메뉴·도구 모음 체크 | 보기 모드 라디오 3종(트리/일반/타일)을 활성 탭 기준으로 메뉴·도구 모음 동기. 숨김·닷파일(메뉴+도구 모음), 폴더 우선(도구 모음만) 체크 동기 | `win.rs:5039-5071` | 없음 | A | 없음 |
| WINB-063 | 상태 동기 — 세션 저장·메뉴 선행 구축 | 어느 패널이든 세션 dirty면 `TIMER_SESSION_SAVE` 1000ms 재무장(코얼레싱). `arm_ctx_prebuild`(셸 메뉴 300ms 선행 구축) | `win.rs:5072-5082`, 상수 `win.rs:85,91,125-126` | `SetTimer`, 셸 메뉴 스레드 | P·W | 없음 |
| WINB-064 | 입력 핸들러 공용 꼬리 | flush → `update_title("")` → `update_status`. 선택·경로를 바꿀 수 있는 **모든** 입력 경로가 지나야 한다 | `win.rs:5090` `finish_input` | 없음 | N | 없음 |
| WINB-065 | 선택 시그니처 | 패널별 `(선택 수, 캐럿)` — MouseUp이 선택을 바꿨을 때만 동기 길목을 지나게 하는 비교값 | `win.rs:5099` `selection_sig` | 없음 | N | 없음(순수 — 테스트 후보) |

### 1.9 명령 디스패처 `run_command`(메뉴·도구 모음 공용)

공통: 진입 시 `cancel_pending_rename`(`win.rs:5109`), 말미 flush → `update_title("")` → `update_status`(`win.rs:5552-5554`). 일부 분기는 결과 노트를 보존하려고 **조기 return**한다(아래 표기).

| ID | 명령(id) | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-066 | 디스패처 골격 | 위 공통 꼬리. 알 수 없는 id는 무동작 | `win.rs:5108-5112, 5550-5555` | 없음 | N | 없음 |
| WINB-067 | 새 탭(1)·탭 닫기(2) | 활성 패널 `new_tab` / 활성 탭 `close_tab` | `win.rs:5113-5117` | 없음 | N | 없음 |
| WINB-068 | 끝내기(3) | **창 닫기와 같은 경로**(`WM_CLOSE` 게시) — 설정·세션 저장을 건너뛰지 않게 | `win.rs:5120-5122` | `PostMessageW(WM_CLOSE)` | P | 없음 |
| WINB-069 | 숨김(10)·닷파일(11)·폴더 우선(67) 토글 | 값의 원천은 탭. `view_scope`: `"tab"`=활성 탭, `"panel"`=활성 패널 전 탭, `"global"`=양 패널 전 탭. 숨김·닷은 대상 패널 활성 탭 즉시 재열람(비활성 탭은 stale→전환 시 수렴), 폴더 우선은 메모리 재정렬. 메뉴·도구 모음 체크 즉시 동기 + `persist_settings` | `win.rs:5127-5164` | 없음 | N·A | 없음 |
| WINB-070 | 항상 맨 위(68) | 토글 → 즉시 적용 → 메뉴·도구 모음 체크 → 영속 | `win.rs:5166-5174` | `apply_always_on_top`(SetWindowPos 추정, `win.rs:333`) | P | 없음 |
| WINB-071 | 새 폴더(4)·새 파일(5) | `create_new`(WINB-034) | `win.rs:5175-5177` | — | N | 없음 |
| WINB-072 | 보기 모드 트리(13)·일반(14)·타일(15) | **탭별** 적용(활성 패널 활성 탭). `view_mode` 설정에 마지막 선택 저장(새 세션 기본). 즉시 설정 파일 저장 + 전체 무효화 | `win.rs:5178-5192` | `InvalidateRect` | N·A | 없음 |
| WINB-073 | 컬럼 너비 동기화(63) | 토글·체크 동기. 켜질 때 활성 패널 `columns_snapshot`을 반대 패널에 `apply_columns`(폭뿐 아니라 **구성까지**). 즉시 저장 | `win.rs:5193-5209` | 없음 | N·A | 없음 |
| WINB-074 | 패널 싱글(16)·듀얼(17)·토글(64) | 싱글=우 패널 숨김(**상태 보존**). 싱글 진입 시 활성=좌 강제. 메뉴 라디오 동기, 도구 모음 **재구성**(`build_toolbar` 11인자), 정보 라디오는 효과 기준(싱글 패널이면 싱글 표시·선호값 보존), 재레이아웃, 도크 정보 갱신, 저장, 전체 무효화, `update_status` | `win.rs:5210-5258` | 없음 | N·A | 없음 |
| WINB-075 | 정보(도크) 싱글(18)·듀얼(19)·토글(65) | **싱글 패널에서는 변경 불가** → 제목 `status.infoLocked`. 그 외 위와 같은 재구성 절차 | `win.rs:5259-5303` | 없음 | N·A | 없음 |
| WINB-076 | 하단 도크 토글(8, Ctrl+`) | 양 패널 동시 표시/숨김. 메뉴 체크·도구 모음 재구성·재레이아웃·도크 정보·영속 | `win.rs:5304-5330` | 없음 | N·A | 없음 |
| WINB-077 | 퀵 런처 바 토글(9) | `launcher_visible` 반전, 메뉴 체크, 재레이아웃, 영속 | `win.rs:5331-5339` | 없음 | N·A | 없음 |
| WINB-078 | 퀵 런처 항목 실행(200+i) | 구분선이면 return. `launcher::launch(hwnd, item, 활성 패널 현재 폴더)`(`%path%` 치환). 성공 `launcher.ran {라벨}` / 실패 `launcher.failed {라벨}` | `win.rs:5340-5356` | `launcher.rs:146`(프로세스 실행) | P | 없음 |
| WINB-079 | 설정(60)·일괄 이름변경(61)·ctl 갤러리(62)·정보(66) | 모달은 `State` 차용과 분리하려고 **메시지 게시로 지연 실행**: `WM_APP_PREFS`·`WM_APP_BULK`·`WM_APP_CTLDEMO`·`WM_APP_ABOUT`. 62는 임시 개발용(이식 제외 후보) | `win.rs:5357-5372` | `PostMessageW` | P(지연 실행 방식) | 없음 |
| WINB-080 | 실행 취소(6)·다시 실행(7) | 경로바 편집 중이거나 리네임 중의 실행 취소는 **필드 내용 복귀**(`do_clip(Undo)`). 그 외 `do_undo_redo`. **조기 return**(노트 보존) | `win.rs:5373-5384` | 없음 | N | 없음 |
| WINB-081 | 잘라내기(69)·복사(70)·붙여넣기(71)·모두 선택(72) | 단축키와 같은 경로 `do_clip`(포커스 문맥 디스패치 — C구간). 조기 return | `win.rs:5385-5395` | 클립보드 | P | 없음 |
| WINB-082 | 새로 고침(12, F5) | 가상 루트(내 PC)면 클라우드 캐시 전량 무효화, 클라우드 경로면 그 연결 캐시 무효화 후 `reopen_filtered` | `win.rs:5396-5405` | 없음 | N | 없음 |
| WINB-083 | 뒤로(20)·앞으로(21)·위로(22) | 활성 패널 `nav_back`/`nav_forward`/`nav_up` | `win.rs:5406-5408` | 없음 | N | 없음 |
| WINB-084 | 테마 시스템(30)·라이트(31)·다크(32) | `theme_mode` 설정 → `apply_theme` → 영속 | `win.rs:5409-5417` | — | N·P | 없음 |
| WINB-085 | 언어 시스템(40)·언어 n(41+i) | `lang_setting="system"` 또는 발견 목록의 코드 → `apply_lang` → 영속 | `win.rs:5418-5427` | — | N·P | 없음 |
| WINB-086 | 클라우드 "연결 추가하기"(후보 0개, 299) | 제목 `cloud.addNone` 안내만. 조기 return | `win.rs:5429-5434` | 없음 | N | 없음 |
| WINB-087 | 클라우드 바로 가기(300+i) | API 연결=센티널 `nexa_vfs::cloud_root(i)`로 진입, 동기화 폴더=로컬 경로. 폴더 소실이면 `cloud.gone {라벨}` 후 조기 return | `win.rs:5435-5455` | 없음 | N | 없음 |
| WINB-088 | 클라우드 온라인 보기(340+i) | `cloud::web_url(kind)`를 기본 브라우저로. 전경 양도 허용 선행 | `win.rs:5456-5474` | `ShellExecuteW("open")`, `allow_foreground_handoff` | P | 없음 |
| WINB-089 | 클라우드 URL 복사(380+i) | URL을 클립보드에 쓰고 `cloud.urlCopied`. 조기 return | `win.rs:5475-5488` | `clipboard::write_text` | P | 없음 |
| WINB-090 | 클라우드 연결 해제(420+i) | 목록에서 제거. API 연결이면 `secret::clear_from(i)`(인덱스가 당겨지므로 꼬리 재배치) + 캐시 전량 무효화. `apply_cloud_change`. 노트 API=`cloud.removed`, 동기화 폴더=`cloud.unlinked`. 조기 return | `win.rs:5489-5515` | 토큰 보관소 | N·P | 없음 |
| WINB-091 | Connect Cloud(492+서비스 인덱스) | `start_cloud_oauth`. 조기 return | `win.rs:5516-5526` | — | N·P | 없음 |
| WINB-092 | 클라우드 연결 추가(460+i) | 메뉴 구성 시점 후보 스냅숏(`cloud_cands`) 인덱스 기준. 상한 `CLOUD_MAX=32`. 라벨의 `|`는 `/`로 치환. `account` 빈 문자열(동기화 폴더 연결). `cloud.linked {라벨}`. 조기 return | `win.rs:5527-5549` | 없음 | N | 없음 |

### 1.10 테마·언어·클라우드 인증

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-093 | 테마 적용 | `resolve_theme(mode)` → 메뉴 라디오 3종 동기 → 제목 표시줄 다크 여부 적용 → 전체 무효화 | `win.rs:5558` `apply_theme` | `apply_titlebar_theme`(DWM 추정, `win.rs:349`), `os_uses_light_theme`(`win.rs:289`) | P | 없음 |
| WINB-094 | 언어 전환(재시작 없음) | 언어 발견 → 코드 해석(`resolve_code(설정, 시스템 UI 언어, 목록)`) → 테이블 활성화 → 클라우드 후보 스냅숏 갱신 → 메뉴 재구성(`build_menus` 14인자) → 도구 모음 재구성(툴팁 재주입) → 양 패널 `set_metrics(panel_metrics(dpi), columns(dpi))`. **알려진 한계: 컬럼 폭이 기본값으로 재설정** | `win.rs:5573` `apply_lang` | `system_ui_lang`(`win.rs:321`) | N·P(시스템 언어) | 없음 |
| WINB-095 | OAuth 직접 연결 시작 | client_id 비면 모달(제목 `cloud.connect`, 본문 `cloud.err.noClientIdMsg {표시명}{kind}`, 버튼 `del.close`) 후 중단. `AuthSession::begin` 실패=제목에 오류 키. 인증 URL 안내 모달(본문 `cloud.auth.prompt {표시명}{URL}`): ①`cloud.auth.openBrowser` ②`cloud.copyUrl` ③`ops.cancel`. 3 또는 닫힘(0)=취소(세션 drop으로 리스너 닫힘). 2=URL 클립보드 복사, 1=기본 브라우저. 제목 `cloud.auth.waiting`. 워커가 **300초** 리디렉션 대기→토큰 교환→`CloudAuthResult`를 Box로 `WM_APP_CLOUD_AUTH`에 실어 종결 통지 | `win.rs:5623-5736` `start_cloud_oauth` | `ShellExecuteW`, 클립보드, `Box::into_raw` 전달 | A(모달)·P(브라우저·클립보드·통지) | 없음 |
| WINB-096 | OAuth 완료 처리 | 실패=제목+**모달**(본문 `{표시명} — {오류문구}` + 상세). 연결 수 ≥32면 `cloud.err.full`. 라벨=`{표시명} – {계정 또는 cloud.account.unknown}`(`|`→`/`). **같은 종류+API 연결이고 계정이 같거나 비어 있던 기존 행은 갱신**(유령 행 방지), 없으면 추가(`path=""`). 토큰 저장 실패=`cloud.err.tokenSave`(연결은 유지). `apply_cloud_change` 후 `cloud.connected {라벨}` | `win.rs:5739` `on_cloud_auth` | `secret::save_token`(DPAPI) | N·P | 없음 |
| WINB-097 | 클라우드 연결 변경 공용 마감 | 영속 → vfs 루트 동기(`sync_cloud_roots`) → 후보 재계산 → 메뉴 재구성 → 가상 루트(내 PC)를 보는 패널 활성 탭 재열람 → 전체 무효화 | `win.rs:5809` `apply_cloud_change` | 없음 | N·A | 없음 |

### 1.11 툴팁·탭 교차 이동·포커스·마우스 과도 상태

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-098 | 도구 모음 툴팁 무장/해제 | 마우스 이동 시 hover 버튼이 **새 버튼이면** 기존 해제 후 `(id, 0틱)` 무장 + `TIMER_TIP` 250ms. hover 없음이면 해제. 같은 버튼 유지 중엔 무동작 | `win.rs:5843-5867` `tip_cancel`·`tip_on_mousemove`, 상수 `win.rs:88-90` | `SetTimer`/`KillTimer`, `tip::hide` | A | 없음 |
| WINB-099 | 툴팁 표시 틱 | 무장 id와 현재 hover id 불일치·hover 없음·**커서가 버튼 화면 사각형 밖**이면 해제. 2틱(500ms) 경과 시 버튼 **하단 +2px, 좌측 정렬**로 표시. 색: 글자 `theme.text`, 배경 `theme.chrome_bg`, 테두리 `theme.border`. 글꼴=대화상자 글꼴 | `win.rs:5871` `tip_tick` | `GetCursorPos`, `ClientToScreen`, `tip::show`(별도 팝업 창) | A | 없음 |
| WINB-100 | 패널 간 탭 이동 | `CrossMove{src,from,dst,at,adopt}`. 마지막 탭은 이동 불가(`detach_tab`이 거부 → `false`). `adopt && view_scope != "tab"`이면 대상 패널 보기 값 채택(커밋 시에만 — 미리 보기 이동은 값 무변). 붙인 뒤 재레이아웃(탭 줄 수 변동), 대상 패널 활성화, `update_status` | `win.rs:5918-5958` `cross_move_tab` | 없음 | N·A | 없음 |
| WINB-101 | 탭 드래그 ESC 취소 | `tab_drag_undo=(원 패널, 원 인덱스)` 스냅숏으로 복귀. 임계 미달(드래그 시작 전)이면 프레스만 취소. 같은 패널=`move_tab`, 반대 패널에 미리 보기로 가 있었으면 detach→원위치 attach→재레이아웃→원 패널 활성화. 반환=ESC 소비 여부 | `win.rs:5962` `cancel_tab_drag` | 없음 | N·A | 없음 |
| WINB-102 | 캡처 상실 시 마우스 과도 상태 일괄 정리 | 스플리터 3종 드래그 플래그, 터미널 선택 드래그(+`TIMER_TERM_SEL` 해제, 선택은 유지), DnD 발신 후보(`drag_press`), `rename_on_up`, TUI 버튼(`term_mouse_btn`), 탭 드래그(ESC 경로와 동일) 정리. 스플리터가 걸려 있었으면 전체 무효화 | `win.rs:6002` `reset_mouse_transients` | `WM_CAPTURECHANGED` 전용, `KillTimer` | P(이벤트 대응) | 없음 |
| WINB-103 | TUI 마우스 버튼 유지 판정 | SGR 버튼 코드 0=좌→`MK_LBUTTON`, 2=우→`MK_RBUTTON`, 그 외 false | `win.rs:6023` `tui_btn_held` | `MK_*` 비트 | N(입력 표현만 P) | `win.rs:9768` |
| WINB-104 | 활성 패널 전환 | 싱글 패널이면 항상 0. 바뀌었을 때만 포커스 시각 동기 + 제목 갱신 | `win.rs:6031` `set_active` | 없음 | N | 없음 |
| WINB-105 | 포커스 시각 동기 | 패널 강조=`active==i && 터미널 포커스 없음`. 도크 스트립 강조=`term_focus==Some(i)`. 즉 **터미널 포커스 중에는 패널 전부 비활성 표시** | `win.rs:6046` `sync_focus_visuals` | 없음 | A | 없음 |
| WINB-106 | F3 스크롤 벤치 | Home → 통계 리셋 → 200프레임(`BENCH_FRAMES`) 휠 -120 주입 + 즉시 그리기 → 제목 `" · 벤치 완료"`(**하드코딩 한국어**) | `win.rs:6054` `bench` | `UpdateWindow` | P(동기 페인트)·N | 없음 |

### 1.12 터미널 입력 보조·스플리터 스냅·휠

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-107 | 키 라우팅 판정 | `KeyRoute`: 포커스 없음=`List` / 도크 숨김 또는 종류≠2=`ClearFocus` / PTY 있음=`Term` / PTY 없음=`TermPending`(**키 삼킴** — 목록으로 누수 금지) | `win.rs:6083-6129` `route_key_with_term`·`term_key_route` | 없음 | N | `win.rs:9916` |
| WINB-108 | 터미널 비문자 키 → VT 시퀀스 | ↑`ESC[A` ↓`ESC[B` →`ESC[C` ←`ESC[D` Home`ESC[H` End`ESC[F` Delete`ESC[3~` PageUp`ESC[5~` PageDown`ESC[6~`. 나머지는 None(문자·Enter·Backspace·Ctrl+문자는 문자 입력 경로) | `win.rs:6132` `term_key_seq` | `VK_*` 코드 | N(키 표현만 P) | 없음(순수 — 테스트 후보) |
| WINB-109 | 스플리터 자석 스냅 | **Alt 누름=스냅 없음**. 임계 `SNAP_PX(20)*dpi/96`. 창 폭 50%에 근접하면 중앙, 도크 표시 중이고 반대편 구분선(`other`)에 근접하면 그 위치 | `win.rs:6149` `snap_split_x`, 상수 `win.rs:55` | `GetKeyState(VK_MENU)` | N(수식키 조회만 P) | 없음(순수화 후 테스트 후보) |
| WINB-110 | 텍스트 도크 히트 | 도크 표시·종류≠터미널·높이>0·내용 영역 포함이면 그 패널 인덱스(휠·가로 휠 라우팅) | `win.rs:6168` `dock_text_at` | 없음 | N | 없음 |
| WINB-111 | 휠 대상 패널 | 휠 좌표는 **화면 좌표** → 클라이언트 변환 → 마우스 아래 패널(없으면 활성 패널 폴백) | `win.rs:6180` `wheel_target` | `ScreenToClient` | P(좌표계) | 없음 |
| WINB-112 | 캐럿 깜빡임 주기 | 시스템 값, 0 또는 `u32::MAX`(깜빡임 끔)면 530ms | `win.rs:6190` `caret_blink_ms` | `GetCaretBlinkTime` | P | 없음 |
| WINB-113 | 터미널 마우스 이벤트 전달(TUI) | 마우스 모드 켜짐 + **SGR(1006) 인코딩일 때만**. 그리드 밖이면 false. 열=`(x-rc.x-2)/cw+1`, 행=`(y-rc.y-1)/ch+1`. `ESC[<{btn};{col};{row}M`(누름)/`m`(뗌). 전송했으면 true(호스트가 로컬 선택·스크롤 억제) | `win.rs:6203` `term_send_mouse` | 없음 | N | 없음 |
| WINB-114 | 터미널 선택 드래그 확장 | 그리드 위로 벗어나면 1줄 위 스크롤, 아래면 1줄 아래. 좌표를 그리드 안으로 클램프한 셀로 끝점 갱신 | `win.rs:6225` `term_drag_extend` | 없음 | N | 없음 |
| WINB-115 | 터미널 그리드 히트 | 도크 표시 + 종류=2 + 터미널 존재 + 그리드 사각형 포함 | `win.rs:6242` `term_hit` | 없음 | N | 없음 |
| WINB-116 | 도크 영역만 무효화 | 터미널 스크롤·선택 갱신용 부분 무효화 | `win.rs:6251` `invalidate_dock` | `InvalidateRect(rect)` | P(부분 무효화) | 없음 |

### 1.13 우클릭 팝업·편집 창·설정

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINB-117 | 도구 모음 빈 영역·컬럼 헤더 우클릭 팝업 | 도구 모음: ①`{pref.toolbarOrder}...` ②`menu.file.prefs`. 컬럼 헤더: ①`{pref.colLayout}...`. 커서 위치에 표시. 선택 결과는 메시지 게시로 지연 실행(`WM_APP_EDIT_TOOLBAR`·`WM_APP_PREFS`·`WM_APP_EDIT_COLS`) | `win.rs:6264` `show_bar_popup` | `CreatePopupMenu`·`AppendMenuW`·`TrackPopupMenuEx(TPM_RETURNCMD+TPM_RIGHTBUTTON)`·`SetForegroundWindow`·`GetCursorPos` | A | 없음 |
| WINB-118 | 순서 편집 창 열기 | `field`=`ORDER_FIELD_TOOLBAR(1)`→`toolbar_order` + `prefs::toolbar_editor_spec()` / `ORDER_FIELD_COLS(2)`→활성 패널 `panel_col_layout` + `prefs::col_editor_spec()`. 대화상자 글꼴 생성 후 `ordereditor::show`, 닫히면 글꼴 해제 | `win.rs:6306` `open_order_editor` | `HFONT`, `DeleteObject` | A | 없음 |
| WINB-119 | 휠 줄 수 동기 | 시스템 휠 스크롤 줄 수(기본 3, 페이지 스크롤 값은 -1로 전달 추정)를 `nexa_gui::set_wheel_lines`에 주입. 실패 시 종전 값 | `win.rs:6327` `sync_wheel_lines` | `SystemParametersInfoW(SPI_GETWHEELSCROLLLINES)` | P | 없음 |
| WINB-120 | 설정 창 열기(Ctrl+,) | 현재 값 스냅숏 `PrefValues`(필드 55개 — §5-1) 구성 → `prefs::show`(모달, State 참조 차단) → 닫힐 때 최종 값으로 `apply_prefs` 1회 더(멱등). 변경은 창이 열린 동안 `WM_APP_PREFS_APPLY`로 실시간 반영 | `win.rs:6349` `open_prefs` | 모달 창 | A | 없음 |
| WINB-121 | 일괄 이름변경(Ctrl+Shift+R) | 대상=`keyboard_targets`(각각 폴더 여부 포함). 비면 `bulk.noSelection`. `bulkrename::show(hwnd, targets, 글꼴, tz_offset_min)` 모달 → 확정 쌍을 순차 `nexa_ops::rename`(실패 개별 격리). 성공분: 펼침 집합 접두사 치환 + **`MoveBatchOp` 1건**(Ctrl+Z로 배치 전체 되돌림). 노트 `bulk.done {n}` + 실패 시 `bulk.fail {n}`. `reload_both` | `win.rs:6424` `open_bulk_rename` | 모달 창, 시간대 | A·N | 없음 |
| WINB-122 | 설정 적용 — 테마·언어 | **변경됐을 때만** 기존 명령 경로 재사용(`run_command(CMD_THEME_*)`·`CMD_LANG_*`). 각 단계마다 `state_of` 재획득(재진입 규약) | `win.rs:6475-6493` `apply_prefs` | — | N | 없음 |
| WINB-123 | 설정 적용 — 글꼴 | 터미널 글꼴·크기 변경 → 그리기 백엔드 재생성(`st.dw=None`). 기본·상태바·목록 글꼴(크기 8~32 클램프) 변경 → 백엔드 재생성. 우클릭 메뉴 글꼴은 **저장만**(OS 메뉴라 미적용 — dir3는 자체 그리기 메뉴이므로 적용 대상이 됨) | `win.rs:6494-6522` | DirectWrite 백엔드 | A·P | 없음 |
| WINB-124 | 설정 적용 — 목록 장식·대화상자 글꼴 | 폴더 굵게·헤더 굵게/이탤릭 변경 시 양 패널 전 탭 즉시(`set_font_decor`). `dlg_font{family,size_pt}`는 항상 대입 | `win.rs:6523-6539` | 없음 | A | 없음 |
| WINB-125 | 설정 적용 — 숨김·닷파일 일괄 | 설정 창 체크박스는 **범위와 무관하게 양 패널 전 탭 일괄** 기입(`set_view_filters(true, …)`) 후 활성 탭 재열람. 메뉴·도구 모음 체크 동기는 무효화 수집기 공유. (6558줄 이후 나머지 설정 항목은 C구간 문서) | `win.rs:6540-6557` | 없음 | N·A | 없음 |

**합계 125개 항목.**

## 2. 화면·컨트롤 배치

본 구간은 배치 계산(`layout`, A구간)이 아니라 **그리기 순서·스플리터·팝업·대화상자 호출 규격**을 담는다.

### 2-1. 주 창 그리기 순서(Z 순서) — `win.rs:4820-4933`

1. 좌 패널 전체(`panels[0].paint`)
2. 우 패널(`bounds().w > 0`일 때만)
3. 싱글 정보 모드일 때 좌 도크 재도장(전폭 공유 도크 — 우 목록의 마지막 부분 행이 침범하는 것 덮기)
4. 도크 터미널 셀 그리드(패널별, 종류=터미널) + 픽셀 스크롤 중이면 종류 스트립 재도장
5. 파일 좌/우 스플리터
6. 도크 좌/우 스플리터 + 파일↔도크 가로 분리선
7. 도구 모음 → 퀵 런처 바(높이>0) → 상태바 → **메뉴바(최상위 — 드롭다운 오버레이)**
8. 백버퍼를 화면에 전송

### 2-2. 스플리터 수치 — `win.rs:53,55,205-206,4886-4925`

| 요소 | 위치·크기 | 색 |
|---|---|---|
| 파일 좌/우 스플리터 | x=`panels[0].bounds().right()`, 폭=`panels[1].bounds().x - 위 x`, y·높이=좌 패널 bounds. **폭>0일 때만 그림** | 평상시 `theme.border`, 드래그 중 `theme.accent` |
| 도크 좌/우 스플리터 | x=좌 도크 `right()`, 폭=우 도크 x − 위 x, y·높이=좌 도크 bounds. 폭>0일 때만 | 평상시 `theme.border`, 드래그 중 `theme.accent` |
| 파일↔도크 가로 분리선 | x=0, 폭=**클라이언트 전폭**, y=`도크 y − gap`, 높이=`gap`. `gap = max(2, SPLIT_TH(3) × dpi / 96)` | 평상시 `theme.text_dim`(다크에서 식별), 높이 드래그 중 `theme.accent` |
| 공통 두께 | `SPLIT_TH = 3`px(@96dpi) — 세 구분선 통일 | — |
| 히트 존 반폭 | `SPLIT_HALF = 3` | — |
| 패널 최소 폭 | `MIN_PANEL = 200`(논리 px) | — |
| 자석 스냅 | `SNAP_PX = 20`px(@96dpi): 창 50%, 반대편 구분선. **Alt=해제** | — |

### 2-3. 상태바 — `win.rs:4977-5010`

- 좌측 문구: `[좌 또는 우] 항목 N개 · 선택 M개(선택 있을 때만) · 탭 i/n`
- 우측 문구: `필터(숨김 표시/숨김 · 닷파일 표시/숨김) · 평균 페인트 시간(µs)`
- 좌·우 2구역 구성(`statusbar.set_text(left, right, inv)`), 구역 폭·여백은 `nexa-gui/src/widgets/chrome.rs:384` 소관(본 구간 밖).

### 2-4. 도구 모음 툴팁 — `win.rs:5843-5908`

- 지연: 250ms 틱 × 2틱 = 500ms.
- 위치: 버튼 사각형의 **좌측 x, 하단 y + 2px**(화면 좌표).
- 색: 글자 `theme.text` / 배경 `theme.chrome_bg` / 테두리 `theme.border`. 글꼴=대화상자 글꼴(`dlg_font`).
- 해제 조건: hover 버튼 변경, hover 없음, 커서가 버튼 사각형 밖(창 밖 포함), 클릭(C구간 호출).

### 2-5. 팝업 메뉴(커서 위치) — `win.rs:6264-6303`

| 위치 | 항목(순서) | 결과 |
|---|---|---|
| 도구 모음 빈 영역 우클릭 | 1. `도구 모음 순서...`(`pref.toolbarOrder` + "...") 2. `설정`(`menu.file.prefs`) | 1→순서 편집 창, 2→설정 창 |
| 컬럼 헤더 우클릭 | 1. `컬럼 구성...`(`pref.colLayout` + "...") | 컬럼 편집 창 |

### 2-6. 버튼 모달 대화상자(공용 `dialog::show_buttons`) 호출 목록

`show_buttons(owner, title, message, buttons[{id,label}], font)` → 누른 버튼 id, 닫힘=0. 첫 버튼=기본 강조. 대화상자 최소 클라이언트 폭 380, 버튼 폭=`max(56, 글자폭+20)`, 버튼 높이=`줄높이+10`(`nexa-dir2/crates/nexa-app/src/dialog.rs:181-200`).

| 용도 | 제목 키 | 본문 키 | 버튼(id 순) | 근거 |
|---|---|---|---|---|
| 삭제 — 잠긴 항목 | `del.lockedTitle` | `del.lockedMsg {잠긴수}{목록}` | 1 `del.skipLocked {나머지}`(나머지>0만) / 2 `del.retry` / 3 `del.cancel` | `win.rs:3783-3810` |
| 삭제 — 실패 통지 | `del.failTitle` | `del.failMsg {실패수}{목록}` | 1 `del.retry` / 2 `del.close` | `win.rs:3958-3975` |
| 전송 — 덮어쓰기 확인 | `ops.overwriteTitle` | `ops.overwrite`(`{0}`=파일명) | 1 `ops.yes` / 2 `ops.yesAll` / 3 `ops.skip` / 4 `ops.cancel` | `win.rs:4545-4576` |
| OAuth — client_id 미설정 | `cloud.connect` | `cloud.err.noClientIdMsg` | 1 `del.close` | `win.rs:5649-5655` |
| OAuth — 인증 URL 안내 | `cloud.connect` | `cloud.auth.prompt {표시명}{URL}` | 1 `cloud.auth.openBrowser` / 2 `cloud.copyUrl` / 3 `ops.cancel` | `win.rs:5666-5682` |
| OAuth — 실패 통지 | `cloud.connect` | `{표시명} — {오류}` + 상세 | 1 `del.close` | `win.rs:5749-5757` |
| 완전 삭제 확인(**OS 기본 메시지 상자**) | `del.title` | `del.confirm {개수}` | 예 / 아니오(**기본=아니오**), 경고 아이콘 | `win.rs:3733-3746` |

목록 표기(`name_list`): 최대 10줄 + `외 N개`(`del.listMore`) — 창이 화면을 넘지 않게(`win.rs:3883`).

### 2-7. 전송 진행 창(비모달) — `win.rs:4231-4241, 4637-4648, 4704-4728`

- 제목 `ops.progressTitle`, 라벨 `ops.progressLabel`(로컬) 또는 `cloud.progressLabel`(클라우드).
- 구성(본문은 `dialog.rs:609-770`): 라벨, **항목별 세그먼트 바**(`SegItem{size,done,status}` — 상태 Pending/Active/Done/Skipped/Failed), 바이트 진행(done/total), 파일 `cur/count`, [취소] 버튼.
- 완료 시: 라벨을 `ops.doneClosing`(또는 클라우드 결과 노트)으로 교체, [취소] → **[닫기 (N)] 카운트다운**(N=올림 초, 첫 틱=ms 나머지, 이후 1초 주기), 0이 되거나 클릭 시 닫힘. 진행 값은 실제 바이트 유지(취소·실패는 부분 진행 그대로).
- `transfer_close_ms = 0`이면 창을 띄우지 않고 제목줄 %만.

### 2-8. 단축키(본 구간이 처리 본체인 것)

| 키 | 동작 | 근거 |
|---|---|---|
| Del | 휴지통 삭제(확인 없음) | `win.rs:3714-3716` |
| Shift+Del | 완전 삭제(확인, 기본=취소) | 동일 |
| F2 | 캐럿 행 이름 바꾸기 | `win.rs:3978, 4005` |
| Ctrl+Z / Ctrl+Y | 실행 취소 / 다시 실행 | `win.rs:3512-3514` |
| F3 | 200프레임 스크롤 벤치(개발용) | `win.rs:6053` |
| F5 | 새로 고침(클라우드는 캐시 무효화) | `win.rs:5396-5397` |
| Ctrl+` | 하단 도크 토글 | `win.rs:5305` |
| Ctrl+, | 설정 창 | `win.rs:240, 6346` |
| Ctrl+Shift+R | 일괄 이름변경 | `win.rs:242` |
| Esc(탭 드래그 중) | 탭 드래그 취소·원위치 복귀 | `win.rs:5960-5962` |
| Alt(스플리터 드래그 중) | 자석 스냅 해제 | `win.rs:6148-6153` |
| 터미널 포커스 중 ↑↓←→ Home End Del PgUp PgDn | VT 시퀀스 전송 | `win.rs:6132-6145` |

키→명령 연결 자체(가속기 테이블)는 C구간(`wndproc`) 소관이다.

### 2-9. 명령 id 표(메뉴·도구 모음 공용) — `win.rs:209-278`

1 새 탭 · 2 탭 닫기 · 3 끝내기 · 4 새 폴더 · 5 새 파일 · 6 실행 취소 · 7 다시 실행 · 8 도크 토글 · 9 런처 바 토글 · 10 숨김 · 11 닷파일 · 12 새로 고침 · 13 트리 · 14 일반 · 15 타일 · 16 패널 싱글 · 17 패널 듀얼 · 18 정보 싱글 · 19 정보 듀얼 · 20 뒤로 · 21 앞으로 · 22 위로 · 30~32 테마(시스템/라이트/다크) · 40 언어 시스템 · 41+i 언어 · 60 설정 · 61 일괄 이름변경 · 62 ctl 갤러리(임시) · 63 컬럼 너비 동기화 · 64 패널 토글 · 65 정보 토글 · 66 정보(About) · 67 폴더 우선 · 68 항상 맨 위 · 69~72 잘라내기/복사/붙여넣기/모두 선택 · 200+i 런처 항목(상한 32) · 299 클라우드 추가(후보 없음) · 300+i 바로 가기 · 340+i 온라인 보기 · 380+i URL 복사 · 420+i 연결 해제 · 460+i 연결 추가 · 492+s OAuth 연결. 각 클라우드 대역 상한 `CLOUD_MAX=32`.

## 3. nexa-ui 매핑

실재 확인은 `nexa-ui/crates/nexa-ctl/src`를 Grep한 결과다.

| dir2 요소(본 구간 사용 호출) | nexa-ui 대응 | 상태 | 필요한 추가 API 요약 |
|---|---|---|---|
| 메뉴바 `st.menubar.set_menus / set_checked(id, on) / paint` (`nexa-gui/src/widgets/menubar.rs:121,130`) | `nexa_ctl::controls::pulldown::MenuBar`(`pulldown.rs:84`) — `set_menus`(132), `take_picked`(152, **문자열 값**), `is_open`(147) | **부분** | `MenuEntry`(`pulldown.rs:25-36`)에 **체크·라디오 표시와 단축키 열이 없다**(Item/Emph/Disabled/Separator/Sub만). → `MenuBar::set_checked(value, bool)` + 체크/라디오 글리프 열 + 단축키 텍스트 열 추가. 명령 id는 `u32`→문자열 값으로 변환 계층 필요 |
| 도구 모음 `st.toolbar.set_buttons / set_checked / hover_tip / paint` (`chrome.rs:133,159,190`) | `nexa_ctl::controls::toolbar::Toolbar`(`toolbar.rs:230`) — `take_clicked`(298), `set_item_tone`(445), `set_item_enabled`(489), `set_item_tip`(481), `set_item_icon`(423), `paint_tooltip`(506) | **부분** | **체크(눌림) 상태 없음** — `ToolTone::Accent`로 색만 표현 가능(`toolbar.rs:68-80`). dir2의 "켜짐=accent 아이콘" 의미와는 맞지만 눌림 배경이 필요하면 `set_item_checked` 추가. 전체 교체용 `set_items(Vec<ToolItem>)` 유무는 미확인(추정: `new`로 재생성) |
| 툴팁 팝업 창 `tip::show/hide`(`tip.rs:47,105`) + 타이머 판정 | `nexa_ctl::draw::draw_tooltip / draw_tooltip_in`(`draw.rs:227,240`), `Toolbar::paint_tooltip`(506), `set_tooltip_above`(276) | **있음** | 별도 OS 창이 아니라 **창 안에 그림** → 창 밖으로 못 나감(가로 클램프). 500ms 지연은 `tokens::HoverIntent`(`tokens.rs:718`)로 대체 가능(추정) |
| 상태바 `st.statusbar.set_text(left, right)`(`chrome.rs:384`) | 없음(Grep `StatusBar` 0건) | **없음 — 추가 필요** | `StatusBar { set_text(left, right), preferred_height, paint }` — 좌·우 2구역, 가운데 말줄임(`draw::ellipsize_middle` 재사용) |
| 탭 바 `tabbar.tab_index_at / pressed_tab / dragging / cancel_drag / take_lines_changed` | `nexa_ctl::controls::tabbar::TabBar`(`tabbar.rs:111`) — `tab_index_at`(446), `pressed_tab`(488), `dragging`(477), `cancel_drag`(482), `begin_drag`(493), `set_multiline`(362), `take_lines_changed`(429) | **있음** | 패널 간 이동(detach/attach)은 앱(Panel) 계층 그대로 |
| 파일 목록 `rows().row_at / is_collapsed_dir / hover_expand / drag_scroll_edge / begin_rename / rename_edit_info / select_program / caret` + `Panel::hide_paths` (`nexa-gui/src/widgets/rows.rs:1188,1228,1236,1211,441,554,582,905`) | `nexa_ctl::controls::tree::TreeGrid`(`tree.rs:675`) — 공개 API는 `set_marked_paths`, `set_column_width`, `clear_selection`, `has_selection` 등뿐 | **없음(대부분) — 추가 필요** | dir2 `RowsView` 상당의 **파일 목록 위젯을 nexa-ui에 추가**: 가상 행 소스, `row_at(x,y)`, 접힘 폴더 판정·호버 펼침, 드래그 엣지 스크롤, 인라인 리네임(편집 정보 = IME 위치용), 프로그램 선택(Single/Toggle), 컬럼 스냅숏/적용, 보기 모드 3종(트리/일반/타일), 글꼴 장식. `nexa_ctl::edit::EditState`(`edit.rs:34`)·`typeahead::TypeAhead`(`typeahead.rs:33`)·`controls::scroll::{FastScroll, SpeedHud, ScrollBars}`는 재사용 가능 |
| 경로바 `pathbar.edit_info / is_editing` | 없음(`PathBar`·`Breadcrumb` Grep 0건) | **없음 — 추가 필요** | `PathBar`(브레드크럼 + 편집 모드 + 제안 목록). `TextBox`(`textbox.rs:350`) 기반 조립 가능 |
| 스플리터(직접 `fill_rect` + 드래그 플래그) | `nexa_ctl::controls::splitter::Splitter`(`splitter.rs:40`) — `SplitAxis`, `SplitEvent`, `is_dragging`, `is_hover`, `paint` | **있음** | 자석 스냅(50%·반대편·Alt 해제)은 호스트가 `SplitEvent` 값에 적용(추정: Splitter에 스냅 없음). 두께 3px·색 규칙(§2-2) 맞춤 확인 필요 |
| 버튼 모달 `dialog::show_buttons` + `MessageBoxW` | 없음 — `nexa-dlg`는 `FilePicker`만(`nexa-dlg/src/lib.rs:154`) | **없음 — 추가 필요** | `nexa-dlg`에 `MessageDialog { title, message, buttons: Vec<(id,label)>, default, escape_id }` 추가. 버튼은 `controls::button::Button`(`button.rs:90`), 본문 줄바꿈 측정. **동기 반환이 아니라 결과 이벤트(`take_result`) 방식**이어야 한다(§6-3) |
| 진행 창 `dialog::Progress`(`open/update/cancelled/set_done`) | 없음(`Progress`·`SegBar` Grep 0건). 단 `controls::timeout_button::TimeoutButton`(`timeout_button.rs:33`)은 **있음** — [닫기 (N)] 카운트다운에 대응 | **없음(바) / 있음(카운트다운 버튼)** | `SegmentedProgress { set_items(Vec<SegItem>), set_bytes(done,total), set_file(cur,count), set_label }` 추가 + 진행 창 조립(`nexa-dlg` 또는 dir3 앱) |
| 네이티브 팝업 메뉴(`CreatePopupMenu`/`TrackPopupMenuEx`) | `nexa_ctl::controls::ctxmenu::ContextMenu`(`ctxmenu.rs:272`) — `CtxItem::item`(137), `open_at`(528), `take_picked`(1134), `with_checked`(259), `with_shortcut`(233), `submenu`(173) | **있음** | 창 밖으로 나가는 팝업이 필요하면 별도 창 호스팅 검토(nexa-sql `winhost.rs:35` `open_window` 방식) |
| 그리기 `DwCtx`(DirectWrite) — `fill_rect`, `text_width` | `nexa_ctl::draw::DrawCtx`(`draw.rs:31`) — `fill_rect`(58), `text_width`(68), `fill_rect_alpha`(181); 래스터 `raster::RasterCtx`(`raster.rs:60`), 글꼴 `FontSet`/`theme::FontPrefs`(`theme.rs:354`) | **있음** | 글꼴 슬롯(기본·상태바·목록·터미널·메뉴) ↔ `FontSlot`(`draw.rs:16`) 대응표 필요 |
| 무효화 수집 `Invalidations` + `flush_invalidations` | `nexa_ctl::widget::Invalidations`(`widget.rs:14`) | **있음** | 호스트는 winit `request_redraw`로 수렴 |
| 터미널 셀 그리드 렌더(`term_paint`) | 없음 | **없음 — 추가 필요** | `TermView`(셀 그리드·선택·스크롤백·캐럿·팔레트) — `nexa-term`(중립 코어)을 그리는 위젯. nexa-ui에 추가하거나 dir3 앱에 둔다(기조상 nexa-ui 추가 권장) |
| 순서 편집 창(`ordereditor` + `ctl/ordertree.rs`) | `controls::listedit::ListEditor`(`listedit.rs:37`), `tree::TreeView`(`tree.rs:493`) | **부분(추정)** | 그룹·구분선 포함 순서 편집 기능 대조 필요(다른 인벤토리 소관) |
| 설정 창·일괄 이름변경 창 | nexa-sql `prefs_win.rs` 구조(설정 레지스트리 `nsql-settings`) | 기준 차용 | 본 구간은 진입·적용만 — 값 55개(§5-1)를 레지스트리 키로 이관 |
| IME 조합 위치 | winit `Window::set_ime_cursor_area` + `nexa_ctl::hangul::Composer`(`hangul.rs:117`) | **있음(호스트)** | 목록 위젯·경로바가 편집 캐럿 사각형을 돌려주는 API 필요 |

## 4. OS 분기점

| # | 기능 | Windows(현 구현) | macOS | Linux |
|---|---|---|---|---|
| 1 | 휴지통 삭제(WINB-009·011·025) | `SHFileOperationW(FO_DELETE+FOF_ALLOWUNDO)` 배치, 워커 스레드, COM STA 초기화(`win.rs:3182, 3821-3834`) | `NSFileManager trashItemAtURL:resultingItemURL:error:`(항목별, 결과 URL 보관) | FreeDesktop Trash 규격: `$XDG_DATA_HOME/Trash/{files,info}` + `.trashinfo`(`Path=`, `DeletionDate=`), 다른 볼륨은 `$topdir/.Trash-$uid`. 대안 `gio trash` |
| 2 | 휴지통 복원(undo) | 셸 undelete(`recycle.rs:32`) | 공개 "되돌리기" API 없음 → 삭제 시 받은 결과 URL에서 원위치로 이동 | `.trashinfo`의 `Path=`로 `rename` + info 삭제 |
| 3 | 삭제 잠금 사전 프로브(WINB-026) | `CreateFileW(DELETE)` 공유 위반 | 잠금 개념 없음 → **빈 결과**(사후 diff 백스톱이 판정) | 동일(빈 결과) |
| 4 | 폴더 감시(WINB-014) | 폴더별 비재귀 핸들 + 스레드 → `WM_APP_FSCHANGE` | FSEvents(파일 이벤트 플래그) 또는 폴더별 kqueue `EVFILT_VNODE` | inotify(폴더별 `IN_CREATE+IN_DELETE+IN_MOVED_FROM+IN_MOVED_TO+IN_MODIFY+IN_ATTRIB`). 공통 보험=프로브 폴링(WINB-017·018, 중립). nexa-ui `nexa-fs/src/watch.rs:134` `StatWatch`는 stat 폴링형 |
| 5 | 셸 변경 통지(WINB-015) | `SHChangeNotifyRegister`(재귀) | 없음 — FSEvents가 File Provider(iCloud·OneDrive) 변경을 전달(추정) | 없음 — 프로브 폴링에 의존 |
| 6 | 타이머(`SetTimer`/`KillTimer` 전반) | 창 타이머 id 1~16 | winit `ControlFlow::WaitUntil` + 앱 내 타이머 표 | 동일 |
| 7 | 워커→UI 통지(`PostMessageW`, `post_final_notify`) | 메시지 + Box 원시 포인터 | `std::sync::mpsc` 채널 + `EventLoopProxy::send_event(Wake)`(nexa-sql 방식 — `app/connwin.rs:92` 등) | 동일 |
| 8 | 지연 실행(모달 재진입 규약 `WM_APP_PREFS` 등) | 메시지 게시 | 앱 내 대기 작업 큐(다음 루프 반복에서 실행) | 동일 |
| 9 | IME 조합 위치(WINB-053) | `ImmSetCompositionWindow(CFS_POINT)` | winit `set_ime_cursor_area`(내부적으로 `firstRectForCharacterRange`) | winit `set_ime_cursor_area`(X11 XIM / Wayland text-input) |
| 10 | 표면·페인트(WINB-054~058) | `BeginPaint`/DirectWrite 백버퍼/`BitBlt` | softbuffer + `nexa-gfx`(`coretext.rs`) | softbuffer + `nexa-gfx`(`text.rs`) |
| 11 | 항상 맨 위(WINB-070) | `SetWindowPos(HWND_TOPMOST)`(추정) | winit `set_window_level(AlwaysOnTop)` | 동일(Wayland는 미지원 가능 — 비활성 표시) |
| 12 | 제목 표시줄 테마(WINB-093) | DWM 속성(추정) | winit `set_theme` | winit `set_theme`(환경 의존) |
| 13 | 시스템 테마·언어 | 레지스트리 / UI 언어 API | winit `Window::theme()`·`ThemeChanged` / `NSLocale preferredLanguages` | winit `theme()` / `LC_ALL`·`LC_MESSAGES`·`LANG` |
| 14 | URL·브라우저 열기(WINB-088·095) | `ShellExecuteW("open")` + `AllowSetForegroundWindow` | `open <url>` | `xdg-open <url>` |
| 15 | 클립보드 텍스트(WINB-089·095) | `clipboard::write_text(hwnd, …)` | NSPasteboard(nexa-sql `clipboard.rs:15-27` 방식) | X11 자체 구현(nexa-sql `clipboard_x11.rs:89`) + Wayland 대응 |
| 16 | 토큰 보관(WINB-036·090·096) | DPAPI(`secret.rs:24-39`), **인덱스 기반** | Keychain(`SecItemAdd`/`SecItemCopyMatching`) | Secret Service(D-Bus), 없으면 0600 파일 폴백. nexa-sql `nsql-vault` 참고(추정) |
| 17 | 퀵 런처 실행(WINB-078) | `launcher::launch`(셸 실행) | `std::process::Command` + `open -a`, 기본 항목 시드 교체(Terminal 등) | `Command` + `xdg-open`, 터미널 에뮬레이터 탐색 |
| 18 | 도크 터미널 PTY(WINB-055) | ConPty + pwsh | `posix_openpt`/`forkpty` + `$SHELL`(없으면 `/bin/zsh`) 로그인 셸 | `forkpty` + `$SHELL`(없으면 `/bin/sh`) |
| 19 | 캐럿 깜빡임 주기(WINB-112) | `GetCaretBlinkTime` | 고정 530ms(시스템 기본값 조회는 선택) | 고정 530ms(또는 `gsettings cursor-blink-time`/2) |
| 20 | 마우스 캡처 상실(WINB-102) | `WM_CAPTURECHANGED` | winit `Focused(false)`·`CursorLeft`·버튼 해제 미수신 감시 | 동일 |
| 21 | 수식키·버튼 상태(WINB-103·109) | `MK_*` 비트, `GetKeyState(VK_MENU)` | winit `ModifiersState`(Alt=Option), 버튼 상태 자체 추적 | 동일 |
| 22 | 휠 줄 수(WINB-119) | `SPI_GETWHEELSCROLLLINES` | `MouseScrollDelta::PixelDelta` 직접 사용(트랙패드), 줄 단위는 3 | `LineDelta`, 기본 3 |
| 23 | 경로 비교(WINB-030) | ASCII 대소문자 무시 + `\`·`/` 끝 구분자 무시 | 기본 볼륨은 대소문자 무시(유지) | **대소문자 구분** — 플랫폼별 비교 함수로 분리 |
| 24 | 파일 DnD 수신·발신(WINB-001~008) | OLE `IDropTarget`/`IDropSource`(`dnd.rs`) | winit는 `DroppedFile`/`HoveredFile`만 제공(좌표·발신 부족) → `NSDraggingDestination`·`NSDraggingSource`(objc2) 직접 구현 | X11 XDND / Wayland `wl_data_device` 직접 구현. 1차는 winit 수신만으로 축소 가능 |
| 25 | 가상 파일 드롭(WINB-003) | `IDataObject` 스트림 추출 | `NSFilePromiseReceiver`(선택 구현) | 해당 없음(비활성) |
| 26 | 대기 커서 | `SetCursor(IDC_WAIT)` | winit `set_cursor(CursorIcon::Wait)` | 동일 |
| 27 | 스크린리더 통지(WINB-061) | UIA | 비활성(후속 NSAccessibility) | 비활성(후속 AT-SPI) |
| 28 | 셸 컨텍스트 메뉴 선행 구축(WINB-063) | 전용 메뉴 스레드 | 해당 없음 — 자체 메뉴(서비스·"다음으로 열기"는 별도 조회) | 해당 없음 — 자체 메뉴(`.desktop` MIME 연결 조회) |
| 29 | `.lnk` 확장자 숨김(WINB-033) | 적용 | 미적용(선택: 확장자 숨김 속성 존중) | 미적용 |
| 30 | 시간대(WINB-121) | `tz_offset_min`(`win.rs:1369`) | `nexa-fs` `local_time`(`nexa-fs/src/lib.rs:758`) | 동일 |
| 31 | 종료 경로(WINB-068) | `WM_CLOSE` 게시 | 창 닫기 요청과 **같은 종료 함수** 호출(저장 포함) | 동일 |
| 32 | 부분 무효화·동기 페인트(WINB-106·116) | `InvalidateRect(rect)`·`UpdateWindow` | `request_redraw`(전체) — 벤치는 루프 내 직접 렌더 호출로 대체 | 동일 |
| 33 | 클라우드 동기화 폴더 후보(WINB-092·094) | `cloud_candidates`(`win.rs:541`) — Windows 경로 규칙(추정) | `~/Library/CloudStorage/*`, `~/Dropbox` 등 | `~/Dropbox`, rclone 마운트 등 |
| 34 | 가상 루트 "내 PC"(WINB-082·097) | 드라이브 목록 | `/Volumes` + 홈·즐겨찾기(`nexa-fs` `drives`·`places` — `lib.rs:377,416`) | 마운트 지점(`/proc/mounts`, `/run/media/$USER`) |

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 설정 키(본 구간이 읽고 쓰는 것)

저장 경로: `config::save(&config::data_dir(), SETTINGS_FILE, &settings.serialize())`(`win.rs:5190`) 또는 `persist_settings(st)`(`win.rs:6942`). 세션은 `SESSION_FILE`, 디바운스 1초(`win.rs:5072-5079`).

| 범주 | 값(문자열 값은 허용 집합) | 근거 |
|---|---|---|
| 보기 옵션 | `show_hidden`, `show_dotfiles`, `sort_folders_first`, `view_scope`(`"tab"`/`"panel"`/`"global"`) | `win.rs:5127-5164` |
| 창·배치 | `always_on_top`, `view_mode`(`"tree"`/`"flat"`/`"tiles"`), `panel_mode`(`"single"`/`"dual"`), `info_mode`(`"single"`/`"dual"`), `col_width_sync`, 도크 표시, `launcher_visible` | `win.rs:5166-5339` |
| 테마·언어 | `theme_mode`(`"system"`/`"light"`/`"dark"`), `lang_setting`(`"system"` 또는 언어 코드) | `win.rs:5409-5427` |
| 클라우드 | `cloud_conns[]{kind,label,path,account}`(상한 32, 라벨의 `|`는 `/`로 치환 — 직렬화 구분자 충돌 방지 추정), 서비스별 `client_id`/`client_secret` | `win.rs:5532-5537, 5640-5646, 5780-5789` |
| 전송·DnD | `transfer_close_ms`(0=진행 창 없음), `dnd_hover_ms` | `win.rs:4234, 3377` |
| 설정 창 스냅숏 `PrefValues`(55개 필드) | `theme, lang, langs, term_font, term_font_size, term_wrap, term_cols, term_theme, term_theme_dark, term_theme_light, term_copy_format, col_autofit_max, toolbar_order, ctx_menu_order, col_layout, dlg_font, dlg_font_size, base_font, base_font_size, ctx_font, ctx_font_size, status_font, status_font_size, list_font, list_font_size, list_folder_bold, header_bold, header_italic, show_hidden, show_dotfiles, dock, sort_folders_first, view_scope, hide_empty_glyph, sort_case_sensitive, nav_up_align, tab_dblclick, plugins_disabled, typeahead_scope, typeahead_reset_ms, typeahead_pos, typeahead_special, typeahead_space, typeahead_backspace, fast_scroll, fast_scroll_step, fast_scroll_max, fast_scroll_window_ms, fast_scroll_hud, fast_scroll_hud_pos, fast_scroll_hud_hold_ms, fast_scroll_hud_fade_ms, fast_scroll_grid_extra, transfer_close_ms, dnd_hover_ms` | `win.rs:6352-6407` |
| 토큰 | 설정 파일이 아니라 `secret::{save_token, load_token, clear_from}(idx)` — **연결 인덱스 기반**(연결 삭제 시 꼬리 재배치) | `win.rs:4168, 5501, 5790` |
| 임시 파일 | `temp_dir()/NexaDir/cloud`(클라우드 열기), `temp_dir()/NexaDir/dnd-*`(DnD 스테이징) | `win.rs:4182, 4732-4735` |

dir3 이관 방침: 위 키를 nexa-sql `nsql-settings` 레지스트리 형식으로 옮기되, 키 이름·허용 값·기본값은 dir2 것을 유지한다(값 의미 변경 금지).

### 5-2. 런타임 상태(`State` 필드 중 본 구간 사용분) — `win.rs:874-1069`

`transfer: Option<TransferJob>`, `transfer_gen`, `transfer_close`(완료 후 진행 창 보관), `cloud_progress`, `cloud_shared`, `pending_delete`, `pending_rename: Option<(패널, 경로)>`, `dnd_hover: Option<(DndHover, 시작ms)>`, `drag_press`, `rename_on_up`, `watchers: [Vec<DirWatcher>; 2]`, `watch_gen`, `watch_since: [u64; 2]`, `shell_watch: [Option<ShellWatch>; 2]`, `probe: [Option<(경로, 서명)>; 2]`, `sub_probe: [HashMap<경로, 서명>; 2]`, `tip_win`, `tip_armed: Option<(버튼 id, 틱)>`, `tab_drag_undo: Option<(패널, 인덱스)>`, `split_drag`, `dock_split_drag`, `dock_drag`, `term_focus`, `term_drag`, `term_mouse_btn: Option<(패널, 버튼)>`, `term_caret_on`, `history: OperationHistory`.

### 5-3. 스레드와 통지

| 워커 | 시작 | 공유 상태 | 통지 | 수신 처리 |
|---|---|---|---|---|
| 로컬 전송 | `win.rs:4527` | `TransferShared{cancel: AtomicBool, done_bytes/total_bytes: AtomicU64, outcome, items, in_flight: Mutex}`(`win.rs:4507-4514`) | `WM_APP_TRANSFER` wparam=세대, lparam 0=진행(단발)/1=완료(**재시도 게시**) | `on_transfer_message`(`win.rs:4660`) — 세대 불일치 무시 |
| 휴지통 삭제 | `win.rs:3821` | `pending_delete`(UI 소유) | `WM_APP_DELETE` wparam=성공 여부(재시도 게시) | `on_delete_message`(`win.rs:3926`) |
| OAuth | `win.rs:5703` | 없음 | `WM_APP_CLOUD_AUTH` lparam=`Box<CloudAuthResult>`(재시도 게시) | `on_cloud_auth`(`win.rs:5739`) |
| 클라우드 목록·다운로드·쓰기 | `cloudfs` 모듈 | `TransferShared`(진행 창 공유) | `WM_APP_CLOUD_LIST/DOWNLOAD/WRITE`(Box, 재시도) + `WM_APP_CLOUD_PROGRESS`(단발) | C구간 + `on_cloud_progress`(`win.rs:4255`) |
| 폴더 watcher | `win.rs:3567` | 없음 | `WM_APP_FSCHANGE` wparam=패널, lparam=세대 | C구간 → `arm_watch_debounce` |
| 셸 변경 | `win.rs:3581` | 없음 | `WM_APP_SHCHANGE_BASE + 패널` | C구간 |

- **종결 통지 유실 방지**: `post_final_notify`는 실패 시 100ms 간격 50회(5초) 재시도(`win.rs:7048`). 진행 통지는 유실돼도 다음 것이 덮으므로 단발.
- **공유 Mutex는 `plock`**(poison 무시 — `win.rs:7037`).
- **세대 가드**: `transfer_gen`, `watch_gen`, 터미널 `term_gen` — 낡은 워커 통지 무시.

### 5-4. 타이머 표(본 구간이 무장·해제하는 것)

| 타이머 | 주기 | 무장 | 용도 |
|---|---|---|---|
| `TIMER_ICONS`(2) | `icons::shell::TICK_MS` | `win.rs:4950` | 아이콘 로딩 큐 |
| `TIMER_RENAME`(4) | 더블클릭 시간 | 해제 `win.rs:4001` | 느린 재클릭 리네임 |
| `TIMER_TERM_SEL`(5) | 60ms | 해제 `win.rs:6008` | 터미널 선택 엣지 스크롤 |
| `TIMER_PROG_CLOSE`(7) | `transfer_close_ms + 500` | `win.rs:4303, 4727` | 진행 창 해제 백스톱 |
| `TIMER_SESSION_SAVE`(8) | 1000ms | `win.rs:5073` | 세션 저장 디바운스 |
| `TIMER_TIP`(9) | 250ms(2틱 표시) | `win.rs:5858` | 툴팁 |
| `TIMER_WATCH_BASE+패널`(10,11) | 300ms(연장 상한 1000ms) | `win.rs:3601, 3610` | 재로드 디바운스 |
| `TIMER_DND`(12) | 100ms | `win.rs:3329` | 드래그 추적 폴링 |
| `TIMER_CLOUD_POLL`(13) | 200ms | `win.rs:4292` | 클라우드 취소 폴링 |
| `TIMER_FSPOLL`(14) | 활성 3000ms / 비활성 30000ms / 최소화 시 정지 | `win.rs:3697` | 프로브 폴링 |

### 5-5. 동기 길목(반드시 유지할 호출 사슬)

입력 처리 → `finish_input`(flush → `update_title("")` → `update_status`) → `update_status` 내부: 보기 값 미러 → stale 탭 재열람 → 상태바 문구 → `update_dock_info` → `uia_notify` → `sync_watchers` → 프로브 기준선 → 클라우드 배지 → 메뉴·도구 모음 체크 → 세션 저장 디바운스 → 컨텍스트 메뉴 선행 구축. 파일 변경 작업 → `reload_both`(재열람 → 기준선 재수립 → 제목 → `update_status`).

## 6. 이식 시 주의 — 실측 교훈·결함 수정 이력

1. **모든 입력 경로는 `finish_input` 한 곳을 지난다**(`win.rs:5084-5094`). 경로별로 손으로 쓴 꼬리가 `update_status`를 빠뜨려 상태바·도크 정보가 이전 폴더를 보이고 watcher·기준선이 낡았던 결함이 있었다.
2. **종료 명령은 창 닫기와 같은 경로**(`win.rs:5118-5122`). 메시지 루프만 끝내면 설정·세션 저장이 건너뛰어진다.
3. **모달 구조가 달라진다(가장 큰 구조 위험)**. dir2는 `show_buttons`가 자체 메시지 루프로 **동기 반환**하고, 덮어쓰기 확인은 **워커 스레드에서** 띄운다(`win.rs:4539-4576`). winit에는 중첩 루프가 없고 창은 이벤트 루프 스레드에서만 만든다. → ⓐ 삭제 잠금 프로브 루프(`win.rs:3777-3811`), 삭제 실패 재시도, OAuth 안내는 **상태 기계(대화상자 결과 이벤트로 이어 가기)**로 재작성 ⓑ 덮어쓰기 확인은 워커가 질문을 채널로 보내고 **응답 채널에서 대기**. "모달은 `State` 차용 밖에서"라는 재진입 규약(`win.rs:5358, 6442`)은 이 구조에서 자연히 해소되지만, 모달 중 `State`가 바뀔 수 있다는 전제(`state_of` 재획득 — `win.rs:6486, 6494`)는 유지한다.
4. **종결 통지는 절대 잃지 않는다**(`win.rs:7041-7056`). 유실 시 `pending_delete`·`st.transfer`가 영구 고착돼 자동 갱신 정지 + 이후 삭제·전송 전부 차단. 채널 기반에서도 "워커 종료 = 결과 반드시 전달"을 보장(패닉 시 drop 가드로 통지).
5. **휴지통 삭제는 워커에서**(`win.rs:3815-3817`). 셸 API가 소파일 몇 개에도 3~4초 걸려 UI가 멈췄다. 행은 **낙관적 숨김**으로 즉시 제거, 실패분은 완료 재로드가 원복.
6. **삭제 성공 판정은 사후 diff**(`win.rs:3922-3932`). 배치 반환값(1비트)은 항목별 판정 불가. 원 위치 잔존 여부로 가른다.
7. **실패는 조용히 무시하지 않는다**: 전송 중 재시작 시 `ops.busy` 안내(`win.rs:4375`), 외부 드롭은 수락 여부 반환(`win.rs:3271-3288`), OAuth 실패는 제목줄이 아니라 모달(`win.rs:5742-5743`).
8. **디바운스 연장 상한**(`win.rs:108-112, 3593-3617`). 같은 타이머를 재무장하면 만료가 밀려, 동기화 클라이언트가 변경을 쏟는 동안 목록이 멈췄다. 첫 통지로부터 1초를 넘기면 연장하지 않는다.
9. **죽은 watcher 솎기**(`win.rs:3557-3560`). 경로만 비교하면 죽은 스레드가 "감시 중"으로 남아 그 폴더가 영구 무갱신.
10. **프로브 서명은 비교 즉시 갱신 + 열거 직후 기준선 즉시 수립**(`win.rs:3623-3625, 3672-3674, 5014-5031`). 기준선을 다음 틱에 세우면 열거~틱 사이(0~3초)의 외부 변경이 영영 삼켜진다. 재로드 후 기준선 재수립은 여분 재로드도 막는다.
11. **OneDrive 플레이스홀더 생성은 부모 mtime도 디렉터리 변경 통지도 바꾸지 않는다**(`win.rs:3645-3650`). 새 자식은 열거만이 본다 → 뷰포트 폴더 전체에 열거 기반 서명.
12. **싱글 패널에서 스플리터 폭이 음수**(`win.rs:4886-4893, 4909`). 음수 폭 사각형을 그대로 채우면 정규화돼 패널 전체를 덮어칠했다(공백 화면). **폭>0일 때만 그린다** — softbuffer 래스터에서도 동일 가드 필수.
13. **터미널 포커스인데 PTY가 없으면 키를 삼킨다**(`win.rs:6079-6082`). 목록으로 내려가면 Delete=휴지통·Enter=실행이 보이지 않는 캐럿에 적용된다. PTY 기동 실패 시에는 포커스를 목록으로 돌린다(`win.rs:4875-4879`).
14. **캡처 상실 시 과도 상태 일괄 정리**(`win.rs:5994-6017`). 버튼 뗌이 오지 않으면 터미널 선택 타이머·DnD 발신 후보·TUI 버튼·탭 드래그·리네임 예약이 잔존했다. winit에서는 포커스 상실·커서 이탈에 같은 정리를 건다.
15. **TUI 버튼 유지는 버튼별로 판정**(`win.rs:6019-6029`). 우버튼을 보지 않아 창의 모든 마우스 이동이 조기 반환으로 죽었다.
16. **예약 리네임은 명령·클릭·키에서 폐기**(`win.rs:3997-4003, 5109`), 만료 시 **현재** 패널·캐럿 경로 재검증(`win.rs:3979-3995`). 메뉴 아래에 편집 필드가 열리던 결함.
17. **탭 드래그 미리 보기 이동은 보기 값을 건드리지 않는다**(`adopt`는 커밋에만 — `win.rs:5914-5915`). ESC 복귀가 무손실이어야 한다.
18. **진행 창 정직 표기**(`dialog.rs:751-753`, `win.rs:4708`). 완료 시 진행 값을 강제로 채우지 않는다. 바이트 없는 작업(삭제·이름 변경·서버 사이드 복사)은 진행 창을 띄우지 않는다(`win.rs:4342-4349`).
19. **클라우드 취소는 통지와 무관하게 주기 폴링**(`win.rs:102-106`). 청크 없는 단일 요청 구간에서 [취소]가 먹지 않았다.
20. **DnD 스테이징 출신 전송의 undo는 "원위치 복귀"가 아니라 "생성물 휴지통 삭제"**(`win.rs:4732-4744`). 원위치가 임시 폴더라 복귀하면 파일이 숨는다.
21. **컬럼 너비 동기화는 폭이 아니라 구성째 적용**(`win.rs:5202-5205`). 구성이 다르면 서로 다른 컬럼에 폭이 실린다.
22. **무효화 수집기를 버리지 않는다**(`win.rs:6546-6548`). 버려진 수집기가 체크 표시 재도장을 유실시켰다.
23. **언어 전환 시 컬럼 폭이 기본값으로 돌아가는 한계**(`win.rs:5572`)는 dir3에서 고칠 수 있으면 고친다(결함 계승 금지 후보 — 결정 필요).
24. **하드코딩 한국어 2건**: `"휴지통 삭제 실패"`(`win.rs:3498`), `" · 벤치 완료"`(`win.rs:6075`) — i18n 키로 승격.
25. **OAuth 재연결 시 빈 계정 행 흡수**(`win.rs:5770-5778`). 안 하면 유령 행이 남는다. 연결 삭제 시 토큰은 인덱스가 당겨지므로 꼬리 재배치(`win.rs:5499-5504`) + 캐시 전량 무효화.

## 7. 회귀 테스트 후보

자동화: **U**=단위(순수 함수) / **I**=통합(임시 폴더·헤드리스 상태 기계) / **M**=수동 또는 UI 구동 필요.

| # | 시나리오 | 대상 ID | 자동화 |
|---|---|---|---|
| T1 | 리네임 만료 판정 표(패널·경로·대소문자·끝 구분자·예약 없음·캐럿 없음) — dir2 테스트 이식 + Linux 대소문자 구분 사례 추가 | WINB-030 | U(기존 `win.rs:9729`) |
| T2 | TUI 버튼 유지 판정 6조합 | WINB-103 | U(기존 `win.rs:9768`) |
| T3 | 키 라우팅 MC/DC(포커스·도크 표시·종류·PTY) | WINB-107 | U(기존 `win.rs:9916`) |
| T4 | 스테이징 쌍 분리·빈 슬롯 정리 | WINB-051 | U/I(기존 `win.rs:9852, 9892`) |
| T5 | `name_list`: 10개 이하·11개 이상(`외 N개`)·빈 목록 | WINB-027 | U |
| T6 | `term_key_seq`: 9개 키 → VT 시퀀스, 미지 키=None | WINB-108 | U |
| T7 | 스냅 계산(순수화): 중앙 근접·반대편 근접·Alt 해제·도크 숨김 시 반대편 무시·DPI 배율 | WINB-109 | U |
| T8 | 디바운스 연장 상한: 첫 무장·상한 이내 연장·상한 초과 미연장 | WINB-016 | U |
| T9 | 프로브: 경로 변경 직후 미발화·같은 경로 변경 발화·서명 즉시 갱신(연속 틱 1회만)·뷰포트 폴더 첫 관측 미발화 | WINB-017·018·019 | I(임시 폴더) |
| T10 | DnD 호버 대기: 같은 후보 유지 시 `dnd_hover_ms` 경과 발동·후보 변경 시 리셋·탭이 폴더보다 우선·행 밀림 시 무시(시계 주입) | WINB-007·008 | U(시계 주입 후) |
| T11 | 전송 이벤트 → 세그먼트 상태: Plan/ItemStart/Bytes/ItemEnd 순서에 따른 done·status, `cur/count` 계산, total=0일 때 pct | WINB-047·049 | U |
| T12 | 전송 완료 노트 조립(완료·건너뜀·오류·취소 조합) | WINB-052 | U |
| T13 | 충돌 결정: 1=이 파일만(재질문)·2=이후 무확인·3=건너뜀·4/닫힘=전체 취소 | WINB-046 | U(결정 함수 분리 후) |
| T14 | 삭제 사후 diff: 일부 잔존 시 성공분만 undo 기록·실패 목록 산출 | WINB-029 | I |
| T15 | 휴지통 왕복: 삭제 → undo 복원 → redo 재삭제(OS별 백엔드 각각) | WINB-009·010·025 | I(OS별 CI) |
| T16 | 새 폴더/파일 생성 → undo(휴지통) → redo(재생성), 이름 충돌 시 번호 부여 | WINB-034 | I |
| T17 | 이름 바꾸기: 성공 undo 기록·동일 이름 무기록·`.lnk` 복원(Windows 한정)·펼침 집합 접두사 치환 | WINB-033 | I |
| T18 | 일괄 이름변경: 일부 실패 시 성공분만 `MoveBatchOp` 1건, undo로 배치 전체 복귀 | WINB-121 | I |
| T19 | 보기 옵션 범위 전파: `tab`/`panel`/`global`별 대상 탭 집합과 stale 수렴 | WINB-069·059 | I(헤드리스 패널) |
| T20 | 패널/정보 모드 규칙: 싱글 패널 진입 시 활성=좌, 싱글 패널에서 정보 모드 변경 거부, 복귀 시 탭 상태 보존 | WINB-074·075 | I |
| T21 | 탭 교차 이동: 마지막 탭 거부·미리 보기 후 ESC 무손실 복귀·커밋 시 값 채택(범위≠tab) | WINB-100·101 | I |
| T22 | 설정 적용 멱등성: 같은 `PrefValues` 두 번 적용 시 재생성·재열람 0회 | WINB-122~125 | I |
| T23 | 클라우드 라우팅: 로컬→클라우드(파일=Upload/폴더=UploadTree)·같은 연결(Move/CopyWithin)·계정 간(교차 복사)·클라우드→로컬(다운로드) 분기 | WINB-044·045 | U(분기 함수 분리 후) |
| T24 | OAuth 완료 병합: 같은 종류+빈 계정 행 흡수·상한 32·라벨의 `|` 치환 | WINB-096 | U |
| T25 | 연결 해제: 인덱스 당김 후 토큰 재배치·캐시 무효화 | WINB-090 | I(보관소 모의) |
| T26 | 종결 통지 보장: 워커 패닉·취소 시에도 `transfer`/`pending_delete` 해제 | WINB-025·047 | I |
| T27 | 페인트 스모크(골든 이미지): 듀얼·싱글 패널·싱글 정보·도크 표시 조합에서 스플리터 사각형과 음수 폭 가드 | WINB-054~057 | I(오프스크린 래스터) |
| T28 | 상태바 문구 조립(선택 0/다수, 좌/우 패널, 필터 조합) | WINB-060 | U |
| T29 | 툴팁: 500ms 지연 표시·버튼 변경 시 재무장·이탈 해제 | WINB-098·099 | U(시계 주입) / M |
| T30 | IME 조합 위치: 경로바 편집·인라인 리네임에서 캐럿 사각형 산출 | WINB-053 | U(사각형 산출) / M(실제 IME) |
| T31 | 터미널 마우스 SGR 인코딩: 셀 좌표 계산·그리드 밖 미전송·비SGR 미전송 | WINB-113 | U |
| T32 | 캡처 상실 정리: 각 과도 상태가 켜진 채 포커스 상실 → 전부 해제 | WINB-102 | I |
| T33 | 외부 파일 드롭: 폴더 행=그 폴더·파일 행/빈 본문=현재 폴더·전송 중 거부 | WINB-001·002 | U(대상 결정) / M(실제 DnD) |
| T34 | 삭제 잠금 모달 흐름(Windows): 건너뛰기·다시 시도·취소 | WINB-024·026 | M(Windows) |
| T35 | 종료 명령이 설정·세션 저장을 수행 | WINB-068 | I |
