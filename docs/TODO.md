# TODO — 순차 백로그

> 목표순. 열 = ID · 우선(P0~P2) · 규모(소/중/대) · 의존 · 상태(☐/🚧/✅). 번호는 한 줄로 늘어난다(최댓값 + 1). 이식 원장 ID([port/00](port/00-index.md))를 "원장" 열에.

## M0 골격

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-01 | 규칙 문서(CLAUDE · 10 · 15 · 16 · 18 · 01 · README) + 현황 문서 4층 | P0 | 중 | — | PROC · CI | ✅ 10-03 |
| T-02 | 워크스페이스(`Cargo.toml` · toolchain · `.cargo` · lints · 프로필) + `nexa-dir` bin 뼈대(`--version` `--smoke` `--selfcheck` 틀) | P0 | 소 | T-01 | SKEL-401~405 | ✅ 10-03 |
| T-03 | CI 3-OS(`ci.yml` · 형제 체크아웃 · fmt/clippy/test/smoke/selfcheck) | P0 | 소 | T-02 | CI-113 | ✅ 10-03 |
| T-04 | 이식 원장 색인 `port/00-index.md` + 검증 매트릭스 틀 `port/90` | P0 | 소 | — | CI-116 | ✅ 10-03 |
| T-05 | `scripts/check-all.sh`(형제 → dir3 · 게이트 4단계 · summary) | P1 | 소 | T-02 | CI-115 | ✅ 10-03 journal §47 |
| T-06 | `ndir-check` 시나리오 러너(`.scn` · 격리 홈 · 샘플 트리 · 검사식) | P1 | 중 | M3 | CI-108 | ✅ 10-03(journal §17 · 시나리오 5 · CI Windows 단계 · Linux xvfb/macOS는 후속) |
| T-70 | M6 파일 작업 배선(복사/잘라내기/붙여넣기 · 전송 작업 스레드 · undo/redo · 새 폴더/새 파일 · 인라인 이름 바꾸기) | P0 | 중 | T-50 | OPS-001~039 | ✅ 10-03 journal §28·§29(잔여: 확인/진행 창 T-29 · 일괄 이름 변경 창 · 영구 삭제) |
| T-07 | CI에 `wasm32` 플러그인 빌드 검증 + Windows 임포트 화이트리스트·용량 측정 | P1 | 소 | M5 | CI-113 | 🚧 10-03 journal §26(wasm32 빌드·로드 검증 ✅ · 임포트 화이트리스트·용량 측정 = M7) |
| T-08 | 성능 스크립트(`perf-*` · 기동 · 대량 폴더 · 누수) | P2 | 중 | M3 | CI-118 | 🚧 10-03 journal §70(`scripts/perf-baseline.sh` 1차 = 기동 `--smoke` 중앙값 · 자가 점검 그룹별 ms · exe 크기 ✅ · 대량 폴더 1만/10만(헤드리스 측정 경로 필요) · RSS/누수 · fps ☐) |

## M1 기반 크레이트

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-10 | `ndir-core` · `ndir-vfs` · `ndir-tree` 이식(dir2 테스트 그대로 · cfg 4곳 3-OS 처리) | P0 | 중 | T-02 | OPS | ✅ 10-03 |
| T-11 | `ndir-ops`(전송·히스토리·일괄 이름 변경) · `ndir-term`(VT) 이식 | P0 | 중 | T-10 | OPS · TERM | ✅ 10-03 |
| T-12 | `ndir-i18n` — `.lang` 3종 임베드 · 파서 · `@fallback` · 사용자 오버레이 · 키 파리티 빌드 검사 · OS 언어 감지(syslang) | P0 | 중 | — | KEY · EXT-2xx | ✅ 10-03 |
| T-13 | `ndir-settings` 엔진(kind·store·tables·migrate·json + 무결성 시험 · perf는 Q-7로 보류) | P0 | 중 | T-12 | SET-001~044 · 120~137 | ✅ 10-03 |
| T-14 | 레지스트리 표(dir2 70키 → nexa-sql 규칙 키 · `CATEGORY_TREE` dir2 페이지 순 · `DEPENDS` · `HIDDEN` · `OS_DEFAULTS` · nexa-sql 공통 키) — 86키 | P0 | 중 | T-13 | PREFS-101~170 · KEY §5 | ✅ 10-03 |
| T-15 | dir2 `settings.cfg` 가져오기(`import_dir2` 순수 함수 + 전수 시험) ✅ · `session.cfg` → `session.conf` Store는 M3 T-45로 | P1 | 소 | T-14 | PREFS-040~057 | ✅ 10-03(세션은 T-45) |
| T-16 | `ndir-license`(nsql-license 복제 · Product `nexa-dir` · Feature 0 · 시험 8건 + 타 제품 거부 · 자가 점검 license 5항목) | P0 | 소 | T-13 | LIC-151~157 · 164 | ✅ 10-03 |
| T-17 | 명령 표 `commands.rs`(48 명령 · 라벨 키 · OS별 기본 키 · repeatable) + 키맵 엔진 `keymap.rs` + `key.<id>` 전수 등재 시험 | P0 | 중 | T-14 | CMD · SET-090~097 · 130 | ✅ 10-03 |

## M2 nexa-ui 보강(형제 저장소 · 각각 nexa-ui 커밋/push)

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-20 | `InputEvent` 확장(DoubleClick · MiddleDown · XButton · 휠 줄 수 전역) — 추가만 · nexa-sql 빌드 확인 | P0 | 소 | — | UIK-211 | ✅ 10-03(nexa-ui 103차) |
| T-21 | `RecordCtx` 공용 기록기(nexa-ctl 공개) | P0 | 소 | — | CI-104 | ✅ 10-03(103차) |
| T-22 | MenuBar 확장(단축키 열 · 체크/라디오 · set_checked/enabled/shortcut · open_menu_index) | P0 | 중 | T-20 | UIK-205 · GUI-060~066 | ✅ 10-03(104차) |
| T-23 | Toolbar `checked` ✅ · TabBar 아이콘·툴팁·MiddleDown ✅ · 툴바 오버플로·press 전환 옵션은 보류(dir2 배치상 불필요 시 생략) | P1 | 소 | — | UIK-206·207 | ✅ 10-03(104차) |
| T-24 | StatusBar 컨트롤 | P0 | 소 | — | UIK-203 · GUI-080 | ✅ 10-03(104차) |
| T-25 | `nexa-grid` VirtualRows + Column + RowSource + 인라인 이름 바꾸기 + 보기 모드 + 픽셀/고속 스크롤(dir2 rows·columns·typeahead·fastscroll 이식) | P0 | 대 | T-20·21 | UIK-201·216·217 · PANEL · GUI | ✅ G-1 10-03(nexa-ui 105차 · 시험 53) · G-2(`write_cell`·`icon()`·타입어헤드/고속 스크롤 통일)는 M3 소비 시 |
| T-26 | PathBar(브레드크럼·편집·자동완성 팝업) | P0 | 중 | — | UIK-202 · GUI-090~101 | ✅ 10-03(nexa-ui 106차 `nexa-explorer::PathBar` · dir2 시험 7) |
| T-27 | InfoDock(종류 스트립 · 텍스트/이미지 · 오버레이 바 · 터미널 슬롯) | P0 | 중 | T-25 | UIK-204 · GUI-110~ | ✅ 10-03(106차 `InfoDock`·`OverlayBars` · dir2 시험 18) |
| T-28 | Tooltip 관리자 · Overlay z 스택 | P1 | 소 | — | UIK-208·209 | ☐ |
| T-29 | nexa-dlg: Dialog 프레임 · MessageBox(버튼 N) · Prompt · Progress 창 · 폴더 선택 | P0 | 중 | — | UIK-212·215 · DLG | 🚧 10-03 journal §30(A: 확인/4버튼/마스킹 입력 창 + 영구 삭제·충돌 배선 ✅ · B: 폴더 찾아보기 ✅ journal §36 `file_win` · 진행 창 ✅ journal §46) |
| T-30 | dir2 전용 소형 컨트롤 대응(fontbox · spin · segmented · ordertree · groupcard · searchbox · iconbutton · menubutton) — 기존 nexa-ctl 대체 또는 추가 · 툴바/런처 아이콘 | P1 | 중 | — | DLG-0xx · GUI-07x | ✅ 10-03(A 툴바 SVG journal §39 · B 런처 exe 아이콘 journal §52 · ordertree = order_win §50 · 나머지 소형 컨트롤 = nexa-ctl 기존으로 대체) |
| T-31 | DrawCtx `push_clip/pop_clip` · 터미널 셀 텍스트 · italic · 테마 토큰(tab_bar_bg·header_bg·dock_bg·status_bar_bg) · ICO/SVG 디코더 | P1 | 중 | — | UIC-310~317 · RENDER | 🚧 10-03(SVG 래스터 + `draw_image_hint` ✅ nexa-ui 108차 journal §38 · 클립 스택 ✅ 112차 §67 · italic/장식 설정/터미널 굵은 셀 ✅ 113차 §68 · 테마 토큰 = nexa-grid 대체 종결 · ICO ☐) |
| T-32 | FolderTree(지연 로딩) · Toast 승격 · FilterBox | P1 | 중 | — | UIK-210·213·214 | ✅ 10-03 journal §69(Toast → nexa-ui 114차 `controls::toast` · FolderTree/FilterBox = dir2에 없음 → 범위 밖) |

## M3 앱 골격

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-40 | 호스트 껍질 복사(present·winhost·wingeom·winfocus·theme·icon·input·clipboard·toast) + `App`/`Focus`/`layout` + 이벤트 루프 + 깨움·타이머 표 | P0 | 대 | M1·M2 | SKEL-001~167 | ✅ 10-03(journal §12 · 창 실증 · 깨움 표는 T-46) |
| T-41 | AppCore/Shell 분리 + FakePlatform + `layout.dump` 골든 1장 | P0 | 중 | T-40 | CI-102·105 | ✅ 10-03(journal §13 · 최소 분리 · 골든 1장 · 시험 5 · FakePlatform은 T-50) |
| T-42 | 메뉴바(5 메뉴 · 체크 상태 동기) · 툴바 13버튼 · 런처 · 명령 디스패치 `menu_action` | P0 | 중 | T-22·23 | WINA · CMD | 🚧 10-03(메뉴 5·툴바 13 글리프·`command` 한 길·런처 바 ✅ journal §35 · SVG/exe 아이콘 = T-30 · 툴바 그룹 도크(이동 · 배치 저장) + 툴바/런처 크기·간격 설정 ✅ §74 · 잔여: 떼어 낸 그룹 플로팅 창 · 배치 초기화 명령) |
| T-43 | 탭바(패널별) · 경로바 · 듀얼 패널 파일 목록 · 스플리터 · 상태바 · 폴더 트리 · 포커스 순환 · 선택 모델 · 정렬 · 열 | P0 | 대 | T-25·26 | PANEL · WINA~C | ✅ 10-03 3차(journal §14·§19 — 패널 구조 · 탭별 히스토리 · 스플리터 · 탭 잠금/고정/복제/메뉴 · 패널 간 이동 · 열 폭 기억/동기 ✅ · 폴더 트리 = T-32 · 내 PC 용량 열 ✅ 10-03 journal §22 · 네비 버튼 dir2 MDL2 글리프 + 쉐브론 두부 수정 ✅ §72 · 행 셸 아이콘 GAP-003 + 아이콘 계층 ✅ §73 · 감시 재열람 선택·펼침·정렬 복원 GAP-005 ✅ §73 · 잔여: FilePicker 최근/숨김 주입 GAP-004) |
| T-44 | 설정 창(prefs_win 복사 → dir2 페이지 구성 WIDGETS 힌트) · 단축키 창 · 키맵 · `apply_setting` 조각 + 적용 누락 감시 시험 | P0 | 대 | T-14·17 | SET-060~097 · PREFS-3xx | ✅ 10-03(journal §18 · JSON 편집·폴더 찾아보기·순서 편집 창은 잔여) |
| T-45 | 세션 복원(탭·경로·열·스플리터) · 창 기하 기억 · 테마/언어 즉시 전환 · 고속 스크롤 설정 연동 | P1 | 중 | T-43 | PREFS-2xx | ✅ 10-03(journal §15 · dir2 session.cfg 형식 · 디바운스 1 s/5 s · 창 기하·테마·언어는 T-40 — 펼침 집합·고속 스크롤 연동은 T-44/T-32로) |
| T-46 | 기동 명령(`NDIR_STARTUP_CMD`) · 덤프 어휘 · `assert` · `quit` · 패닉 훅·crash 기록 | P0 | 중 | T-41 | CI-106·107·112 | ✅ 10-03(journal §16 · `@ready/@idle/@after` · `assert` 종료 코드 3 · 덤프 6 · crash.rs) |

## M4 플랫폼 층

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-50 | 포트 trait 8종 + Fake + ADR-0001 | P0 | 중 | T-41 | DR-5 · CI-103 | ✅ 10-03(journal §20 · 9종(+Disk) · 폴링 감시·외부 열기·Windows 용량 · selfcheck shell/open/fs) |
| T-51 | Windows: 셸 메뉴(IContextMenu + 메뉴 스레드) · 휴지통 · CF_HDROP 클립보드 · OLE DnD · 폴더 감시 · 셸 통지 · ShellExecute · ConPTY | P0 | 대 | T-50 | SHELL · TERM | 🚧 10-03 journal §21·§33(A 휴지통·클립보드·용량 ✅ · B-1 셸 컨텍스트 메뉴 ✅ · B-2a 배경 셸 메뉴 + 생성 감지 ✅ journal §41 · B-2b 폴더 감시 ReadDirectoryChangesW ✅ journal §42 · B-2c 휴지통 복원 ✅ journal §43 · B-2d 새로 만들기 ▸ 템플릿 ✅ journal §53 · DnD 수신 1차(winit) ✅ journal §61 · 메뉴 스레드+선행 구축+아이콘 ✅ 10-03 §71(SHELL-014~016 · 011 hbmpItem) · B-2 잔여: OLE DnD 완전(발신·자동 스크롤)·SHCNE 통지·메뉴 owner-draw 아이콘·Shift 확장 동사) |
| T-52 | macOS: 자체 메뉴 + 연결 프로그램 · trashItem · NSPasteboard 파일 URL · NSDragging · FSEvents · open · forkpty/$SHELL | P0 | 대 | T-50 | SHELL §4 | 🚧 10-03(open/open -R · $SHELL · statvfs · ~/.Trash 이동 · forkpty(unixpty) · kqueue 감시 journal §54 · NSPasteboard 파일 클립보드 journal §55 · trashItem 휴지통+복원 journal §57 ✅ / NSDragging ☐) |
| T-53 | Linux: 자체 메뉴 + xdg/MimeApps · freedesktop Trash · gnome-copied-files/text/uri-list · XDND · inotify · xdg-open · openpty/$SHELL | P0 | 대 | T-50 | SHELL §4 | 🚧 10-03(xdg-open · $SHELL · statvfs · freedesktop Trash(복원 포함) · openpty(unixpty) · inotify 감시 journal §54 · XDG 템플릿 §53 · uri-list 파일 클립보드 journal §56 ✅ / XDND ☐) |
| T-54 | `--selfcheck` 실제 항목(fs·trash·shell·pty·ctxmenu·clipboard·open) · Help ▸ 자가 점검 창 | P0 | 중 | T-51~53 | CI-110·111 | 🚧 10-03 journal §27(창 ✅ · 실제 항목 fs/trash/shell/open/plugin/license ✅ · ctxmenu ✅ journal §41 · pty/clipboard/dnd = 해당 슬라이스에서) |

## M5 도크·터미널·미리보기·플러그인

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-60 | 도크(정보 8줄+형식별 상세 · 미리보기 텍스트/이미지 · 스크롤·선택·복사) | P0 | 중 | T-27 | WINA · GUI-11x | 🚧 10-03 journal §23(배치·정보 기본 8줄·텍스트 미리보기·스트립 ✅ · 이미지 그리기 T-31 · 형식별 상세 T-5x · 터미널 T-61) |
| T-61 | 터미널 뷰(셀 격자·선택·캐럿·스크롤백·테마 15종·복사 서식 HTML/RTF·cwd 동기·키 라우팅) | P0 | 대 | T-31 · T-51~53 | TERM | 🚧 10-03 journal §24·§34(A ✅ · B 고정 열/가로 스크롤·HTML 복사·TUI 마우스·글꼴 크기 ✅ · 잔여: 픽셀 스크롤·고속 스크롤·RTF) |
| T-62 | 플러그인 런타임 이식(dir2 ABI · 탐색 경로 3단 · 내장 폴백 · 격리 시험) + 동봉 `.wasm` 2종 + F3 창 + 압축 미리보기 그리드·암호 | P0 | 대 | T-60 | PLUG · EXT-441~445 | 🚧 10-03 journal §25·§31(A 런타임·시임·동봉 ✅ · B F3 창·암호 입력 ✅ · C-1 압축 그리드 창 ✅ journal §37 · C-2 SVG 래스터·인라인 이미지 ✅ journal §38 · C-3 드래그 선택 ✅ journal §44) |
| T-63 | 플러그인 설정 페이지 · 매니저(sha256·설치) · `plugins/sdk` · `plugin-build.{ps1,sh}` | P1 | 중 | T-62 | EXT-414~418 | ✅ 10-03(A: sdk·빌드 스크립트·CI journal §26 · B: 체크박스 페이지 journal §51 · 매니저 1차 설치/삭제 journal §59 — 2단계 원격 저장소 보류) |

## M6 파일 작업

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-70 | 전송(복사/이동 · 진행 창 · 취소 · 충돌 · 스테이징 · 덮어쓰기 확인 상태 기계) · 삭제(휴지통·영구·잠금 프로브) · 새 폴더/파일 · 실행 취소 | P0 | 대 | T-29 · T-50 | OPS · WINB | 🚧 10-03(전송·충돌·undo·휴지통 undo·진행 창 ✅ journal §28·§43·§46 · 잠금 프로브 WINB-024 ☐) |
| T-71 | 인라인 이름 바꾸기 · 일괄 이름 변경 창(규칙·프리셋·미리보기) · 순서 편집기(툴바/메뉴) | P0 | 대 | T-25·30 | DLG · OPS | ✅ 10-03(인라인 §29 · 일괄 창 journal §48 · 순서 편집기 journal §50 — 잔여: 우클릭 진입·접기·프리셋 관리 팝업·TZ) |
| T-72 | 클라우드(Q-6 결정 뒤): OAuth 루프백·토큰 봉투·가상 FS — 범위 = CLOUD-001~099 + WINB-036~042 · O-001~020 + SHELL-034 SHCNE(OneDrive 플레이스홀더 감지 · 10-05 §14 이관) | P2 | 대 | — | CLOUD · WINB-036~042 · O-001~020 | ⏸ 대기(사용자 10-05 — 목록 등재 · 착수 보류 · 10-05 §9) |

## M7 라이선스·배포

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-80 | 라이선스 창 · Help ▸ 라이선스… · About 상태 줄 · i18n(ja 신규) · 기동 명령 `license.*` | P0 | 중 | T-16 · T-40 | LIC-158~164 | ✅ 10-03 journal §36(상태줄 배지 ⓒ 보류 · CLI = GUI만) |
| T-81 | nexa-license 발급기 보강(`mail_text` 제품 분기 · `--id-prefix` 기본 · E2E `nexa-dir`) → push + 태그 | P1 | 소 | — | LIC-165~170 | ✅ 10-03 journal §40(태그 `nexa-dir3/t81-2026-10-03`) |
| T-82 | 패키징 3-OS(rc·MSI·포터블 zip·pkg·dmg·deb·rpm) · `release.yml` · 동봉 플러그인 · 라이선스 고지 | P0 | 대 | M5 | CI-114 · PROC | 🚧 10-03(스크립트·release.yml·고지·플러그인 ✅ journal §49 · 태그 릴리스 실행 ☐ · ci 예산 단계 ☐) |

## M8 교차 검증

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-90 | 검증 매트릭스 전수(원장 ID ↔ 구현 ↔ 시험) · 누락 보충 · 의도된 차이 등재 | P0 | 대 | M3~M7 | CI-116 | 🚧 10-03(1차 감사 스크립트 + 91 전수 journal §62 · 2차 원장 기반 시험 + 집단 행 journal §63 · 3차 CMD 대조 journal §64 · 4차 묶음 행 44 journal §65 · 5차 잔여 접두 28 journal §66 = **원장 전 ID 덮음** · 세부 ✅화 = T-91과 함께) |
| T-91 | dir2 대조 실기 QA 표 · 성능 기준선 · 누락 문서(port/52 nexa-dlg·fs · 98 릴리스 대조 · 99 커버리지) | P1 | 중 | — | — | ✅ 10-03 journal §70(문서 52·92·98·99 + 생성 스크립트 `qa-checklist.py`·`coverage-files.py` + `perf-baseline.sh` 1차 기준선 · check-all 단계 2 · 실기 QA 수행 = 사용자 · 성능 잔여 = T-08) |

## dir3 신규 기능(dir2에 없음 · 사용자 10-03 · 원장 = [22](22-dir3-features.md))

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-92 | 로그 창 — `ndir-log`(nsql-log 복제) · `log_win.rs` · 개발자 모드 상세 로그 · 계측 지점 · 상태줄 ms 표기를 로그로 이전 | P1 | 중 | — | NEW-001 · DR-15 | ☐ — 우선순위 상향 근거: 사용자 10-04 "로그 확인 방법" 질문(지금은 앱 안 로그 창 없음 · 추적 env(`NDIR_TERM_TRACE` 등) + crash 파일뿐 · §28) · T-140(클릭 느림) 계측도 이 창에 연결 |
| T-93 | (사용자 10-03 재요청: "상태바에 메모리 · 라이선스 · CPU · Memory · Disk 정보가 안 나옴") 메모리 모니터 — `memstat`/`mem_win`/`memtrim`/`app/memory.rs` 복제 + dir3 카테고리 | P1 | 중 | T-92 | NEW-002 · DR-15 | 🚧 1차 10-04 §11 · 2차 일부 §14(3a6fd52 · 아이콘 캐시 · 터미널 버퍼) · 3차 §25(42ccb5a · 누적 정보 · 기능별 묶음 6 · 영역 13 · ▲/▼ · 힙 정리 · 미리보기/플러그인 · 실행 취소 ✅ · nexa-ui 146) · [힙 정리] "정리 중…" 잠금 + 결과 줄 10-05 §1(6c42c90) · 남은 것 = 셸 자식 프로세스 메모리 · 다른 보조 창 표면 · 도크 이미지 · 창을 여는 기동 명령/덤프(T4) · 화면 판정 |
| T-94 | 상태줄 구성 — 우측 칸 [탭 n/m][CPU][메모리][디스크 I/O][라이선스](프로세스 부하 · 기본 표시) · nexa-ctl `StatusBar` 클릭 가능한 세그먼트(nexa-ui 추가) · `statusbar.layout` 순서 편집기 · 부하 수집 `platform/` 포트 | P1 | 중 | T-93 | NEW-003 · DR-15 · DR-23 | 🚧 1차 10-04 §2(fbb6b62 · 칸 · 부하 3-OS · 라이선스 클릭 · 순서 편집기/메모리 클릭 = 후속) · §29 디스크 · 네트워크 칸 삼각형 표식(고정 자리) + 속도 단계 깜빡임 0~9(14c6866 · nexa-ui 147 · 깜빡임 다시 그리기 CPU 관찰 필요) · 10-05 §1 CPU/메모리 사용률 단계 색(`load_level` · nexa-ui 149 `tint`) · D/N 두 줄 1 px 간격 가운데(nexa-ui 148 · "/" 삐침 화면 판정)(6c42c90) · §3 M 칸 폭 견본 128.0GB · 숫자/단위 빈칸 제거(1e4520b · e7cd2b4 · CPU 100.0% 기준 유지) |
| T-101 | 터미널 도크 = Windows Terminal 설정 따르기(글꼴 목록 · 크기 · 색 구성표 · 커서 · 여백 · 시작 명령) + SGR 3 이탤릭 | P1 | 중 | — | NEW-012 · DR-21 | 🚧 1차(글꼴 목록 · 크기 · 이탤릭 · 10-03 §75~§80 · Linux §98) · 10-05 §4 따르기 끔 크기 = Linux 경로와 같은 계산(716f0b3) · §5 `term.font_face` 쉼표 목록 = 폴백 체인(결함 수정) · 글꼴 설정 즉시 적용(87b4740) · §7 글꼴 이름 = 실제 이름 탐색(nexa-ui 152 · 굴림체 · ⚠ 기본값 Consolas가 D2Coding 폴백이던 결함 해소) · 남은 것 = 색 구성표 · 커서 · 여백 · 시작 명령(T-107) |
| T-102 | 창 투명도 `window.transparency`(3-OS 포트 · 보조 창 · 성능 향상 모드면 0) | P2 | 소 | — | NEW-013 · DR-22 | ☐ |
| T-95 | (사용자 10-03 재요청: "상태바를 각 탭(패널) 하단에") 탭 상태바 + `DirInfoProvider` + 폴더 상태(`FolderStats` · 1번 칸) + Git 공급자(2번 칸 · 1차 읽기 전용) | P1 | 중 | T-94 | NEW-004 · NEW-005 · DR-16 | 🚧 1차 10-04 §2 · 2차 §14(3a6fd52 · Git 앞섬/뒤짐/변경 수 · porcelain v2 워커) · 남은 것 = HEAD/index 감시 · 플라이아웃 |
| T-100 | 값 추출 1차(DR-19 · [23](23-settings-extraction.md) §2 1차 — 감시 · 아이콘 폴링/캐시 · 스크롤백 · 프레임 · 토스트 · 미리보기 줄 + 범위 불일치 2 정렬) | P1 | 중 | — | DR-19 | ☐ |
| T-99 | 성능 향상 모드 — `perf.boost` + `BOOST` 표(읽을 때 덮어쓰기) · 설정 창 잠금 표시 · 덮을 동작의 설정 키 먼저 신설(아이콘 · 폴링 · 애니메이션 · 스크롤백 …) | P2 | 중 | T-93 | NEW-008 · DR-18 | 🚧 1차 10-04 §23(fad2ca8 · `perf.boost` = 시스템 상태 모니터링 칸 제거 · 조회 중지 · DEPENDS) · 2차 = BOOST 표 · 아이콘/애니메이션/폴링/스크롤백 · 설정 창 ⚡ 표시(T-130과 묶음) |

## 다음 세션(10-05 마감 · Windows PC 세션 · 10-05 사용자 결정 = T-148 그 밖 6건 진행 · T-147 DnD 완성(P0) 등재 · 클라우드 T-72/T-129 대기 · 순서 = T-148 → 이번 세션 후속 T-143 · T-93 남은 것 · T-144 · T-145(결정) · T-146 · T-92 → T-134 남은 것 → T-95 3차 → macOS 디스크 칸(IOKit · Mac 실기) → T-113 → T-131 → T-130 · T-129 → T-128 → T-117 → T-114 → 남은 T-103~T-108 — 완료 항목은 상태 칸 ✅)

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-103 | Linux 첫 실기 확인(사용자) — 쉐브론 크기·모양(✅ 선 쉐브론 §98) · 경로 바 `/` 표시·세그먼트 클릭(nexa-ui 122) · 내 PC 목록(§95 NEW-015 · 용량 열) · Alt+←/→/↑ · Enter · Alt+↓(§93) → 결과로 후속 결정 | P0 | 소 | — | NEW-015 · GAP-008 | 🚧 10-03(쉐브론 ✅ · 나머지 확인 대기) |
| T-104 | Linux 아이콘 — 런처 바 아이콘(지금 글자 "VS" · "Te") · 파일 행 아이콘(지금 기본 도형) — freedesktop 아이콘 테마 조회(nexa-fs) | P1 | 중 | T-103 | UIX-033 | ✅ 10-03 §97(파일 행 = 아이콘 테마) · §110(런처 바 = .desktop/테마 앱 아이콘 · nexa-ui 128) |
| T-105 | 경로 바 IME 조합 창 위치(`edit_info` 연결) | P1 | 소 | — | GAP-014 | ✅ 10-05 §11(2cc7c11 · 화면 판정 대기) |
| T-106 | 하네스: 기동 명령 `ui.key`를 `App::key_chord` 경로로(타이핑·메뉴 통과 규칙까지) · 경로 제안 팝업 덤프 어휘 | P1 | 소 | — | CI-106 | ☐ |
| T-107 | 터미널 2차 — 셸 인자 · 에이전트 env(`NO_COLOR` · `CLAUDE_*`) 차단 · `term.color` vs 사용자 NO_COLOR(결정 대기) · WT 2차(색 구성표 · 커서/여백) | P1 | 중 | — | NEW-012 · DR-21 | ☐ — 10-05 §4: 글꼴 크기 계산 통일(따르기 끔 = Linux 기준 · 716f0b3 · 결정 해소) |
| T-108 | 툴바 배치 초기화 명령 · 플로팅 창 | P2 | 소 | — | NEW-010 | ☐ |
| T-109 | 벡터 쉐브론(글꼴 글리프 대신 그리기 — nexa-grid 추가 · OS 간 모양 통일) | P2 | 소 | T-103 | — | ✅ 10-03 §98(nexa-ui 123 `set_marker_vector` · 아이콘 글꼴 없는 OS만 · 사용자 "해결") |
| T-110 | 툴바 "항상 위" 버튼을 새로고침 그룹으로(새로고침 다음) — 사용자 요청 | P1 | 소 | — | NEW-010 | ✅ 10-03 §101(8773ef1 · dir2 기본 순서와 의도된 차이 · T4 기대값 c1dc5a3) |
| T-111 | 숨김 파일 · 점 파일 · 폴더 우선 토글을 **탭별 상태**로(dir2에 사용자가 구현한 개념 — 개발 세션 조사) | P1 | 중 | — | PANEL-045 · WINB-069 · PREFS-131 | ✅ 10-03 §102(53c45b5 · 설정 = 새 탭 기본값 · 보호 파일 = 전역 · `list.view_scope` 기본 tab · 화면 확인 대기) |
| T-112 | Linux 우클릭 메뉴 통합 1차 — MimeApps 기본 앱 / 다른 앱으로 열기 · 압축 · 속성 · `ctxmenu.layout` 순서/표시 연결 | P1 | 중 | — | SHELL · NEW-025 | ✅ 1차 10-03 §117(e13e7b2 · 실행 실기 대기) · 2차 = T-131 |
| T-113 | `scripts/install-dev-desktop.sh` — 개발 빌드용 사용자 영역 `.desktop` + hicolor 아이콘 설치(Wayland GNOME 톱니바퀴 아이콘 해소 · §100) | P2 | 소 | — | — | ☐ |
| T-115 | 스플리터 3종(좌우 패널 · 상/하단 사이 · 하단 좌/우) — 동작 방식 · 두께 · 서서히 밝아지는 표시(dir2 대조 구현) — 사용자 요청 | P1 | 중 | — | WINB-056 · WINC-094~096 · NEW-017 | ✅ 10-03 §103(13e19b0 · nexa-ui 126 · 사용자 결정 4건 = 개발 세션 기본안 · 화면 확인 대기) |
| T-116 | 시간 설정 단위 — nexa-sql과 같은 개념: 10초 이상 설정은 초 단위 · 10초 미만은 ms 단위(키 · 라벨 · 이주) — 사용자 요청 | P1 | 소 | — | DR-3 · DR-19 | ✅ 10-03 §105(7a13e4d · 시간 키 7개 이미 ms · 바꿀 키 없음 · 감시 시험 `time_keys_follow_unit_rule`) |
| T-118 | 보기 옵션 관리 방법 4택(전체 · 좌/우 패널 · 탭 · 폴더) · 기본 = 폴더(같은 폴더면 좌우 공통 · 폴더별 기억 · 세션 `dirview`) · `ui.theme` 기본 system — 사용자 요청 | P1 | 중 | T-111 | NEW-018 · PREFS-131 | ✅ 10-03 §104(273b462 · 화면 확인 대기) |
| T-119 | 툴바 대소문자 구분 정렬 토글(폴더 우선 옆 · 탭 보기 옵션 4번째 · 보기 관리 방법만큼 적용 · 설정 = 새 탭 기본값) — 사용자 요청 | P1 | 소 | T-118 | NEW-019 | ✅ 10-03 §105(523857f · 화면 확인 대기) |
| T-120 | **설정 상하/종속 관계 UX** — 상위를 끄면 하위 Disable · 앞 값에 따라 연관 설정 불가/범위 제약 · 강제/제약으로 조정된 값은 사용자 값을 숨겨 유지하고 제약이 풀리면 복귀(성능 향상 모드처럼 직접 설정한 경우 제외) — 사용자 요청 · 사전 분석 중 | P1 | 중 | — | SET-016 · NEW-023 · NEW-008 | ✅ 1단계 10-03 §115(d49644c · 전이 잠금 · DEPENDS 19 · 이유 덧줄 · 초기화 막기 · 강제 값 표시 · 화면 확인 대기) · 2단계 = T-130 |
| T-121 | (첫 단계: 탭 바 배율 결함 `panel.rs:501` 장치 px → 논리 px) 탭 구성 **Multi-line 기본** · Single-line은 옵션 · 싱글일 때 좌우 이동 버튼 배치 3택(① 우측 끝 = 기본 ② 좌측 끝 ③ 이전 = 좌측 끝 · 다음 = 우측 끝) — 사용자 요청 | P1 | 중 | — | UIK-207 · WINB(탭) · NEW-020 | ✅ 10-03 §111(3af767e · nexa-ui 129 · 배율 결함 수정 · 화면 확인 대기) |
| T-122 | 탭을 좌우 패널 간 **드래그로** 이동(지금은 탭 메뉴 "반대 패널로 이동"만 · dir2 71baf67 `cross_move_tab` · 미리 보기 이동 · 값 채택) — 사용자 요청 | P1 | 중 | — | WINB-100 · WINC-098 · WINC-101 | ✅ 10-03 §113(3f30fa1 · 놓을 때 이동 · 미리 보기 이동은 의도적으로 안 함 · 화면 확인 대기) |
| T-123 | 파일 그리드 컬럼 이동 시 탭 이동처럼 **옮겨갈 위치 표식** — 사용자 요청 | P2 | 소 | — | PANEL(열) · GAP-016 · NEW-022 | ✅ 10-03 §114(0110f82 · nexa-ui 131 · GAP-016 함께 · 화면 확인 대기) |
| T-124 | 컬럼 너비 동기화를 끈 채 쓰다 다시 켰을 때의 동작 점검 · 결함이면 수정(설정 창에서 켜도 즉시 맞춤 · 열 key 기준 복사 · 같은 패널 전 탭) — 사용자 요청 | P1 | 소 | — | `list.col_width_sync` | ✅ 10-03 §109(6032bc0 · 열 key별 · 설정 창 즉시 · 같은 패널 전 탭 · 시험 `col_width_sync_matches_by_column_key`) |
| T-125 | GAP-015 연결 안 된 설정 키 정리 — 연결하거나(ui.text_* · typeahead.* 등) 설정 창에서 빼기(INTERNAL) · 사용자 결정 필요 | P1 | 중 | — | GAP-015 | ☐ |
| T-126 | 파일 그리드 **기본 열 = 이름 · 상태 · 크기 · 수정한 날짜**(확장자 · 종류 숨김) · **"상태" 열 신설** = 클라우드에 연결된/네트워크 파일 상태(탐색기 OneDrive "상태" 열 대응) — 사용자 요청 · 사전 분석 완료(§112) · 사용자 결정 5건 대기(동기화 중 상태 · Linux 표시 · 상태 열 정렬 · 내 PC 상태 열 · 플레이스홀더 링크 오판 범위) | P1 | 중 | — | PANEL-131 · CLOUD-096 · NEW-024 · GAP-018 | ✅ 10-03 §116(6965a76 · nexa-ui 132 · 권장안 5건 = 사용자 확인 대기 · Windows OneDrive 실기 필요) |
| T-127 | **Shift 다중 정렬 머리** — 인디케이터는 머리 칸 가장 우측 · 다중 정렬 순번은 인디케이터 우측 · Shift+클릭 반복 = 오름차순 → 내림차순 → 없음(nexa-sql 결과 그리드 참고) — 사용자 요청 | P1 | 중 | — | PANEL-077 · 078 · NEW-021 · GAP-017 | ✅ 10-03 §112(8ee6d05 · nexa-ui 130 · Shift 순환은 종전부터 같음 · GAP-017 함께 · 화면 확인 대기) |
| T-129 | 클라우드 플레이스홀더 정리 — ~~GAP-018 링크 오판~~(✅ §116) · GAP-019 SHELL-085 방어(미리보기 · 상세 · 할당 크기 생략 — ndir-vfs `cloud_status` 재사용) · CLOUD-096 후속(동기화 중 · pin/unpin 일괄 · cldapi) | P1 | 중 | — | GAP-019 · SHELL-085 · CLOUD-096 | 🚧(GAP-018 ✅) · 남은 것(GAP-019 다운로드 방지 · CLOUD-096) = ⏸ 대기(사용자 10-05 · 클라우드 묶음 T-72와 함께 · 10-05 §9) |
| T-130 | 설정 종속 2단계 — 범위 결합(Constraint 표 · 다른 키에 묶인 min/max · effective만 클램프 · "허용 범위 a..b" 덧줄) · S3(`scroll.fast_max` > 1 · Gt) · D2 강제(단일 패널 → 정보 단일) · TX1~3(GAP-015 연결 뒤) · 성능 향상 모드를 FORCES 출처로(NEW-008 · T-99) | P2 | 중 | T-120 | SET-016 · NEW-023 · NEW-008 | ☐ |
| T-131 | Linux 우클릭 2차 — 항목별 순서/표시 편집(`ctxmenu.layout`에 xdg 항목 · 사용자 "윈도우처럼 메뉴 추가/변경/순서") · 전자메일 · 다른 위치로 이동/복사 · 항목 아이콘 · 노틸러스 스크립트 · 배경 메뉴(터미널에서 열기 등) · selfcheck ctxmenu 항목 · T4 ctx-menu.scn Linux 분기 | P1 | 중 | T-112 | SHELL · NEW-025 · T-117 | ☐ |
| T-132 | 열 경계 **더블클릭 자동 맞춤** · 최대 폭(`list.col_autofit_max` · GAP-015 연결) — dir2 이식(사용자 "nexa-dir2에 반영되어 있으니 동일하게") | P1 | 소 | — | WINC-029 · WINC-110 · PREFS-135 · GAP-015 | ✅ 10-03 §118(8a58ecd · 열 폭 40 축소 결함 함께 수정 · 화면 확인 대기) |
| T-133 | 툴바 · 빠른 실행(런처) 영역 **우클릭 메뉴** — 설정 바로가기 + 추가/편집(순서 편집기 · 런처 항목 편집) — 사용자 요청 | P1 | 소 | — | CMD-097~099 · TERM-120 | ✅ 10-04 §13(a29094b · 툴바 = 도구 모음 순서 · 설정 · 빠른 실행 = 편집/삭제/추가/구분선/숨기기 · NEW-028 · 버튼 위 우클릭도 메뉴 = dir2와 의도된 차이) |
| T-134 | **i18n 정리** — 사용자에게 보이는 하드코딩 문자열 → 키 · lang 파일 누락/영어 잔존/같은 뜻 중복 키 통합 · 언어 전환 때 안 바뀌는 곳(사용자 "파일 그리드도 다국어 변경이 안 된다") — 사용자 요청 · 협업 세션 조사 → 개발 세션 반영 | P1 | 중 | — | KEY · EXT-2xx | 🚧 c ✅ · a/b 일부 ✅ 10-04 §13(9763858 · a29094b) · 결정 4건 ✅ §15(dfcd2f5 · 대체 언어 = 시스템 → 영어) · 남은 것 = 자가 점검 창 · 라이선스 문구 · 허용값 · unsupported · Command Prompt 비교 · 중복 키 통합 · PositionDropdown · 조사 10-04 §1(언어 전환 미반영 10 · 하드코딩 · 중복 키 → 개발 세션 c → a → b 순 반영 · 사용자 결정 4건: 크기 단위 · ja OK · 런처 시드 라벨 · @fallback) · 열 제목 = 1d8ef8d ✅ |
| T-128 | 정렬 후속(T-127에서 뺀 것) — ▲/▼ `fill_triangle` 도형(nexa-sql 모양 · 화면 판정 뒤) · 화살표–순번 gap · 다중 정렬 열 Shift 없는 클릭 = nexa-sql식 리셋(옵트인 · 사용자 결정) · 내 PC 전체 크기/여유 공간 열 `sortable=false` · 정렬 상태 상속/영속(nexa-grid `set_sort` 시드 API) · "정렬 없음" = 이름순 vs dir2 열거 순서 확인 | P2 | 소 | T-127 | PANEL-065 · 077 · 078 | ☐ |
| T-135 | ⚠ macOS 휴지통 — **운영 결함**: `SystemTrash::trash_one`이 trashItemAtURL 성공 + resultingItemURL nil이면 실패 판정 → HomeTrash 폴백이 이미 없는 원본 rename → ENOENT(성공했는데 실패 토스트 · undo 기록 없음 · 10-04 §2 원인 규명) · 시험 `system_trash_round_trip` 흔들림 2회 · 실제 시스템 휴지통 사용(규약 위반 소지) → 판정 순수 함수 + MC/DC · 폴백은 원본이 남아 있을 때만 · 시험은 #[ignore]/env 가드/자가 점검 opt-in | P1 | 소 | — | SHELL-049 · T-52 · CI | ✅ 10-04 §3(e73e06a · `trash_outcome` · 시험 = CI 전용 · CI 성공) |
| T-136 | ⚠ T4 `delete-confirm` 흔들림 조사 — 10-04 2회(1회째 exit 0 · 300 ms 뒤 dlg.dump 타이밍 · 2회째 **exit 101 패닉**) · 단독 25 + 부하 20회 재현 실패 · 다음 실패 때 `tests/out/delete-confirm/home/crash/` 보존 → 패닉 위치 · 고정 대기(@after:300) → 조건 대기(@idle) 검토 | P1 | 소 | — | CI-108 · T-29 | ☐ |
| T-137 | T4 반복 실행 화면 격리 — 재현 루프의 시나리오 창이 사용자 데스크톱을 가리고 결함으로 오인(10-04 §21) → xvfb(`xvfb-run` · X11 백엔드 강제) 또는 반복 전 사용자 알림 · 이 PC에 xvfb 없음(설치 = 사용자 결정) · CI ubuntu xvfb(T-117)와 함께 | P2 | 소 | — | CI-108 · T-117 | ☐ |
| T-138 | **Command Palette** — Ctrl(macOS Cmd)+Shift+P · 명령 검색 · 실행(참고 = nexa-sql `palette.rs` 약 700줄 · 명령 표 `commands.rs` 라벨/단축키 · nexa-ui 승격 권장) — 사용자 요청 10-04 · 검토 완료 · **사용자 진행 승인 대기** | P1 | 중 | — | CMD · NEW | ☐ |
| T-139 | gate.sh 판정 개선 — 형제 저장소(nexa-ui · nexa-license)는 `crates/` 등 코드 변경만 full 사유로(지금은 HEAD가 바뀌면 문서 커밋에도 full · 10-04 §22) | P2 | 소 | — | DR-26 | ☐ — 별건 ✅ 10-05 §2(094997f): 시험 판정 `^error` 줄 오판(런처 시험 자식의 'version' 오류) → cargo 종료 코드 + 실패 수 |
| T-143 | 메뉴 바(풀다운) ↑/↓ 순환 이동이 `menu.wrap_around`를 따르는지 확인 · 필요하면 연결(10-05 §2 — 이번엔 ContextMenu만) | P2 | 소 | — | NEW-030 | ☐ |
| T-140 | 사용자 보고 "**파일 클릭이 느리다**"(10-04 Windows) — 당시 CPU 100 % = 개발 세션 백그라운드 빌드(nexa-sql 전체 시험) · 클릭 경로에 무거운 동기 작업 못 찾음 · 계측 미실시 → 부하 없는 상태 재확인(사용자) · 재현되면 클릭 → 그리기 구간 계측(T-92 로그 계측 지점과 함께) | P1 | 소 | — | — | ⏸ 보류(사용자 10-05 §10 — 증상이 심해지면 다시 요청) |
| T-141 | 사용자 보고 "**프로그램 아이콘이 기본 아이콘으로 보인다**"(10-04 Windows) — exe 리소스 아이콘 · `icon.rs` 정상 확인 · 어디서 보이는지(작업 표시줄 · Alt+Tab · 탐색기) **사용자 답 대기** | P1 | 소 | — | — | ⏸ 보류(사용자 10-05 §10 "확인" · 재보고 때 재개) — 참고: 빠른 실행 아이콘의 바로 가기 화살표(10-05 §6 · 57943de ✅)는 별건 |
| T-142 | nexa-ui nexa-fs `service_never_blocks_and_settles` 흔들림(shell.rs:945 `elapsed < 5 ms` · 전역 초기화 포함 · CPU 100 %에서 1회 실패 · 평시 13/13 통과 10-04 §24) → 측정 전 `IconService::global()` 선호출 또는 한도 완화(nexa-ui · 개발 세션 판단) | P2 | 소 | — | CI | ☐ |
| T-144 | 상태줄 D/N 깜빡임이 **창 전체 다시 그리기**를 부름(단계 9 ≈ 초당 16회 · 10-04 §29) → 상태줄 칸만 부분 무효화 검토 · 디버그 빌드 CPU 측정(단계별) | P2 | 소 | — | NEW-003 · DR-23 | ☐ |
| T-145 | 앱 메모리 칸 · D/N 줄 표기도 M 칸처럼 숫자/단위 빈칸 없이 통일할지(10-05 §3에서 M 칸만 변경) — **사용자 결정** | P2 | 소 | — | NEW-003 | ☐ |
| T-147 | **드래그 앤 드롭 완성**(사용자 10-05 요구 확정 — 보내기 · 받기 모두 · VMware 등 가상 머신과 주고받기 · Windows Terminal과 주고받기(경로 텍스트/파일 드롭)) — Windows OLE DnD 발신(`IDropSource` · `IDataObject` CF_HDROP + FileGroupDescriptor/FileContents) · 수신 보강(가상 파일 · 텍스트 드롭) · 드래그 중 자동 스크롤 · 최적화 이동(같은 볼륨 = 이동 · 전송 엔진) · macOS NSDragging · Linux XDND · 1차 = winit 수신만(10-03 §61) | P0 | 대 | — | SHELL-046~049 · SHELL-063~069 · T-51/52/53 DnD 잔여 | ☐ |
| T-148 | **"그 밖" 6건 묶음**(사용자 10-05 진행 · 개발 세션 착수 — ②⑤ 먼저): ① 셸 변경 통지(SHCNE · T-51 B-2 잔여) ② Shift+우클릭 확장 동사(SHELL-004) ③ 삭제 전 잠금 확인(WINB-024 · T-70 잔여) ④ 터미널 RTF 복사(T-61 잔여) ⑤ 경로 바 IME 조합 창 위치(GAP-014 · T-105) ⑥ ICO 디코더(UIC-317 · T-31 잔여) — 사전 분석 10-05 §9(① = dir2처럼 드라이브 목록 `WM_DEVICECHANGE`도 — dir3 0건 · ④ = `ndir-term::to_rtf` 이미 이식 · 호출/클립보드만 · ⑥ = dir2에 디코더 없음 · nexa-gfx 신규) | P1 | 중 | — | SHELL-004 · WINB-024 · GAP-014 · UIC-317 | ✅ 6/6 10-05 §14 — ① 장치 변경 = ba98ee9(SHCNE는 클라우드와 함께 대기 → T-72로 이관) · ② 6ff7410 · ④ 6ff7410(Windows · macOS/Linux RTF 후속) · ③ e66a973 · ⑤ 2cc7c11(§11) · ⑥ ICO = nexa-ui 153 · 5e150a5(§13) ✅ |
| T-149 | **dir2 대비 누락 · 결함 정리**(10-05 §12 매트릭스 대조 · 클라우드 제외 · 체감 순): ⚠ Shift+F10/Apps 명령 ID 불일치(`list.context_menu` 분기 없음) · ⚠ 명령행 경로 인자 미동작(KEY-301 · PANEL-023) · 탭 더블클릭/빈 곳 새 탭(CMD-438/439) · 느린 재클릭 이름 바꾸기(WINC-082) · 끝 말줄임(N-02) · 배경 탭 stale(PANEL-015/016) · 펼친 폴더 감시(WINB-014/018) · 삭제 실패 재시도 모달(CMD-112) · 머리글 우클릭 열 배치(WINB-117) · 우클릭 편집 메뉴 3곳 · 다중 부모 우클릭 축소(WINA-084) · 재로드 미루기(WINC-151) · 글꼴 입력 상자 · 메뉴 니모닉 · owner-draw 아이콘 · 도크 상세 · `shell:` 별칭 · 하위 항목(목록 = journal 10-05 §12) | P1 | 대 | — | 각 ID | 🚧 10-05 §13 — 결함 2건 ✅(Shift+F10 명령 ID · 명령행 경로 인자 · 5e150a5) · 장치 변경 ✅(§14 ba98ee9) · 탭 더블클릭 ✅(§15 3587451) · 배경 탭 stale ✅(§16 126c4ad) · 삭제 실패 재시도 ✅(§17 1233db2) · 머리글 우클릭 ✅(§18 dca2406) · 개발 세션 순서 = ① 셸 통지/장치 변경 → 탭 더블클릭 → 배경 탭 stale → 삭제 실패 재시도 → 머리글 우클릭 → 다중 부모 축소 → 재로드 미루기 → shell: 별칭 → 느린 재클릭 → 끝 말줄임 → 편집 메뉴 3곳 → T-150 → T-147 |
| T-150 | **WINA/B/C 시험 보강**(사용자 방침 "가능하면 자동 시험" · 구현됐고 시험 없는 것 19건 — 목록 = journal 10-05 §12) | P1 | 중 | — | WINA · WINB · WINC | ☐ |
| T-146 | nexa-sql에서 nexa-ui 152(글꼴 실제 이름 탐색) 실기 확인 — 글꼴 이름 설정이 파일명과 다른 경우(Consolas 등) 표시 글꼴이 바뀌는지(CONSUMER-CHANGES 152 행 · 시험 751 ✓ · 실기 미검증) | P2 | 소 | — | — | ☐ |
| T-117 | T4 시나리오 Linux 실행 — Windows 전제 3개 분기(`ctx-menu` 셸 배경 항목 · `launcher` · `selfcheck-win`) · CI ubuntu xvfb 단계(T-06 후속) | P1 | 소 | — | CI-108 | ☐ |
| T-114 | macOS 실기 맞춤(사용자 "동일한 과정을 맥에서도") — 이미 적용: 점 파일 = 숨김 · 선 쉐브론 · 터미널 폴백 em · 칸 폭 반올림 / 남음: 행 아이콘(NSWorkspace iconForFile) · 시스템 터미널 글꼴(Terminal.app 프로필) · 우클릭 통합 | P1 | 중 | T-112 | NEW-012 · NEW-016 | ☐ |

그다음 = 위 "dir3 신규 기능" 표 순서: T-92 로그 창 → T-93 메모리 모니터 → T-94 상태줄 구성 → T-95 탭 상태바 → T-102 투명도 → T-99 성능 향상 모드 → T-96 전송 UI → T-97·98 대량 전송 엔진.

## M9 고성능 전송(FastCopy/robocopy류 내장 · 원장 = [22](22-dir3-features.md))

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-96 | 전송 진행 UI 개편 — 카드형 진행 창 · 전체/현재 파일 · 충돌 인라인 · 상태바 진행 칸 · 완료 토스트 → undo | P1 | 중 | T-94 | NEW-006 · DR-17 | 🚧 일부 10-04: [닫기 (N)] 기본 버튼 색 · 충돌 인라인(진행 창 안 질문) §21 · 첫 질문도 인라인 §22 · 질문 중 앞 항목 결과 즉시 반영 §26(9137085) · 취소 = 손대지 못한 항목도 건너뜀 §27(89df13a) · 결과 안내에 전체 대상 수 §28(daa499d) · 남은 것 = 카드형 · 전체/현재 파일 · 상태바 진행 칸 · 완료 토스트 → undo |
| T-97 | 대량 전송 엔진 설계 문서(제공 형태 = rlib `ndir-xfer` 정적 내장 · bin `nexa-copy` · 선택적 cdylib · 직렬화 가능한 JobSpec/Progress/Report · 전략 계층 · 버퍼/비버퍼드 · 병렬 · 검증 · 미러/차등 · 재개 · 필터 · 속성 보존 포트 · 속도 제한 · 작업 큐 · dry-run 보고서) + 성능 기준선 확장 계획 | P1 | 중 | T-08 | NEW-007 · DR-17 | ☐ |
| T-98 | 대량 전송 엔진 1차 구현(T-97 설계 기준) | P1 | 대 | T-97 | NEW-007 · DR-17 | ☐ |
