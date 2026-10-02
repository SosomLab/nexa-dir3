# TODO — 순차 백로그

> 목표순. 열 = ID · 우선(P0~P2) · 규모(소/중/대) · 의존 · 상태(☐/🚧/✅). 번호는 한 줄로 늘어난다(최댓값 + 1). 이식 원장 ID([port/00](port/00-index.md))를 "원장" 열에.

## M0 골격

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-01 | 규칙 문서(CLAUDE · 10 · 15 · 16 · 18 · 01 · README) + 현황 문서 4층 | P0 | 중 | — | PROC · CI | ✅ 10-03 |
| T-02 | 워크스페이스(`Cargo.toml` · toolchain · `.cargo` · lints · 프로필) + `nexa-dir` bin 뼈대(`--version` `--smoke` `--selfcheck` 틀) | P0 | 소 | T-01 | SKEL-401~405 | ✅ 10-03 |
| T-03 | CI 3-OS(`ci.yml` · 형제 체크아웃 · fmt/clippy/test/smoke/selfcheck) | P0 | 소 | T-02 | CI-113 | ✅ 10-03 |
| T-04 | 이식 원장 색인 `port/00-index.md` + 검증 매트릭스 틀 `port/90` | P0 | 소 | — | CI-116 | ✅ 10-03 |
| T-05 | `scripts/check-all.sh`(형제 → dir3 · 게이트 4단계 · summary) | P1 | 소 | T-02 | CI-115 | ☐ |
| T-06 | `ndir-check` 시나리오 러너(`.scn` · 격리 홈 · 샘플 트리 · 검사식) | P1 | 중 | M3 | CI-108 | ☐ |
| T-07 | CI에 `wasm32` 플러그인 빌드 검증 + Windows 임포트 화이트리스트·용량 측정 | P1 | 소 | M5 | CI-113 | ☐ |
| T-08 | 성능 스크립트(`perf-*` · 기동 · 대량 폴더 · 누수) | P2 | 중 | M3 | CI-118 | ☐ |

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
| T-29 | nexa-dlg: Dialog 프레임 · MessageBox(버튼 N) · Prompt · Progress 창 · 폴더 선택 | P0 | 중 | — | UIK-212·215 · DLG | ☐ |
| T-30 | dir2 전용 소형 컨트롤 대응(fontbox · spin · segmented · ordertree · groupcard · searchbox · iconbutton · menubutton) — 기존 nexa-ctl 대체 또는 추가 | P1 | 중 | — | DLG-0xx | ☐ |
| T-31 | DrawCtx `push_clip/pop_clip` · 터미널 셀 텍스트 · italic · 테마 토큰(tab_bar_bg·header_bg·dock_bg·status_bar_bg) · ICO/SVG 디코더 | P1 | 중 | — | UIC-310~317 · RENDER | ☐ |
| T-32 | FolderTree(지연 로딩) · Toast 승격 · FilterBox | P1 | 중 | — | UIK-210·213·214 | ☐ |

## M3 앱 골격

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-40 | 호스트 껍질 복사(present·winhost·wingeom·winfocus·theme·icon·input·clipboard·toast) + `App`/`Focus`/`layout` + 이벤트 루프 + 깨움·타이머 표 | P0 | 대 | M1·M2 | SKEL-001~167 | ✅ 10-03(journal §12 · 창 실증 · 깨움 표는 T-46) |
| T-41 | AppCore/Shell 분리 + FakePlatform + `layout.dump` 골든 1장 | P0 | 중 | T-40 | CI-102·105 | ✅ 10-03(journal §13 · 최소 분리 · 골든 1장 · 시험 5 · FakePlatform은 T-50) |
| T-42 | 메뉴바(5 메뉴 · 체크 상태 동기) · 툴바 13버튼 · 런처 · 명령 디스패치 `menu_action` | P0 | 중 | T-22·23 | WINA · CMD | 🚧 10-03(메뉴 5·툴바 13 글리프·`command` 한 길 ✅ · 런처·SVG 아이콘 ☐) |
| T-43 | 탭바(패널별) · 경로바 · 듀얼 패널 파일 목록 · 스플리터 · 상태바 · 폴더 트리 · 포커스 순환 · 선택 모델 · 정렬 · 열 | P0 | 대 | T-25·26 | PANEL · WINA~C | 🚧 10-03 2차(dir2 패널 구조 `panel.rs` — 패널별 탭·네비 4버튼·경로바·목록 · 탭별 히스토리 · 스플리터 드래그/스냅 ✅ · 탭 잠금/고정/복제/메뉴 · 패널 간 탭 이동 · 폴더 트리 · 열 폭 기억/동기 · 내 PC 열 ☐) |
| T-44 | 설정 창(prefs_win 복사 → dir2 페이지 구성 WIDGETS 힌트) · 단축키 창 · 키맵 · `apply_setting` 조각 + 적용 누락 감시 시험 | P0 | 대 | T-14·17 | SET-060~097 · PREFS-3xx | ☐ |
| T-45 | 세션 복원(탭·경로·열·스플리터) · 창 기하 기억 · 테마/언어 즉시 전환 · 고속 스크롤 설정 연동 | P1 | 중 | T-43 | PREFS-2xx | ✅ 10-03(journal §15 · dir2 session.cfg 형식 · 디바운스 1 s/5 s · 창 기하·테마·언어는 T-40 — 펼침 집합·고속 스크롤 연동은 T-44/T-32로) |
| T-46 | 기동 명령(`NDIR_STARTUP_CMD`) · 덤프 어휘 · `assert` · `quit` · 패닉 훅·crash 기록 | P0 | 중 | T-41 | CI-106·107·112 | ✅ 10-03(journal §16 · `@ready/@idle/@after` · `assert` 종료 코드 3 · 덤프 6 · crash.rs) |

## M4 플랫폼 층

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-50 | 포트 trait 8종 + Fake + ADR-0001 | P0 | 중 | T-41 | DR-5 · CI-103 | ☐ |
| T-51 | Windows: 셸 메뉴(IContextMenu + 메뉴 스레드) · 휴지통 · CF_HDROP 클립보드 · OLE DnD · 폴더 감시 · 셸 통지 · ShellExecute · ConPTY | P0 | 대 | T-50 | SHELL · TERM | ☐ |
| T-52 | macOS: 자체 메뉴 + 연결 프로그램 · trashItem · NSPasteboard 파일 URL · NSDragging · FSEvents · open · forkpty/$SHELL | P0 | 대 | T-50 | SHELL §4 | ☐ |
| T-53 | Linux: 자체 메뉴 + xdg/MimeApps · freedesktop Trash · gnome-copied-files/text/uri-list · XDND · inotify · xdg-open · openpty/$SHELL | P0 | 대 | T-50 | SHELL §4 | ☐ |
| T-54 | `--selfcheck` 실제 항목(fs·trash·shell·pty·ctxmenu·clipboard·open) · Help ▸ 자가 점검 창 | P0 | 중 | T-51~53 | CI-110·111 | ☐ |

## M5 도크·터미널·미리보기·플러그인

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-60 | 도크(정보 8줄+형식별 상세 · 미리보기 텍스트/이미지 · 스크롤·선택·복사) | P0 | 중 | T-27 | WINA · GUI-11x | ☐ |
| T-61 | 터미널 뷰(셀 격자·선택·캐럿·스크롤백·테마 15종·복사 서식 HTML/RTF·cwd 동기·키 라우팅) | P0 | 대 | T-31 · T-51~53 | TERM | ☐ |
| T-62 | 플러그인 런타임 이식(dir2 ABI · 탐색 경로 3단 · 내장 폴백 · 격리 시험) + 동봉 `.wasm` 2종 + F3 창 + 압축 미리보기 그리드·암호 | P0 | 대 | T-60 | PLUG · EXT-441~445 | ☐ |
| T-63 | 플러그인 설정 페이지 · 매니저(sha256·설치) · `plugins/sdk` · `plugin-build.{ps1,sh}` | P1 | 중 | T-62 | EXT-414~418 | ☐ |

## M6 파일 작업

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-70 | 전송(복사/이동 · 진행 창 · 취소 · 충돌 · 스테이징 · 덮어쓰기 확인 상태 기계) · 삭제(휴지통·영구·잠금 프로브) · 새 폴더/파일 · 실행 취소 | P0 | 대 | T-29 · T-50 | OPS · WINB | ☐ |
| T-71 | 인라인 이름 바꾸기 · 일괄 이름 변경 창(규칙·프리셋·미리보기) · 순서 편집기(툴바/메뉴) | P0 | 대 | T-25·30 | DLG · OPS | ☐ |
| T-72 | 클라우드(Q-6 결정 뒤): OAuth 루프백·토큰 봉투·가상 FS | P2 | 대 | — | CLOUD | ☐ |

## M7 라이선스·배포

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-80 | 라이선스 창 · Help ▸ 라이선스… · About 상태 줄 · i18n(ja 신규) · 기동 명령 `license.*` | P0 | 중 | T-16 · T-40 | LIC-158~164 | ☐ |
| T-81 | nexa-license 발급기 보강(`mail_text` 제품 분기 · `--id-prefix` 기본 · E2E `nexa-dir`) → push + 태그 | P1 | 소 | — | LIC-165~170 | ☐ |
| T-82 | 패키징 3-OS(rc·MSI·포터블 zip·pkg·dmg·deb·rpm) · `release.yml` · 동봉 플러그인 · 라이선스 고지 | P0 | 대 | M5 | CI-114 · PROC | ☐ |

## M8 교차 검증

| ID | 할 일 | 우선 | 규모 | 의존 | 원장 | 상태 |
| --- | --- | --- | --- | --- | --- | --- |
| T-90 | 검증 매트릭스 전수(원장 ID ↔ 구현 ↔ 시험) · 누락 보충 · 의도된 차이 등재 | P0 | 대 | M3~M7 | CI-116 | ☐ |
| T-91 | dir2 대조 실기 QA 표 · 성능 기준선 · 누락 문서(port/52 nexa-dlg·fs · 98 릴리스 대조 · 99 커버리지) | P1 | 중 | — | — | ☐ |
