# nexa-dir2 인벤토리 — win.rs 앞 1/3(1~3300줄) + main.rs (WINA)

> 목적: nexa-dir2(Windows 전용)의 메인 창 뼈대 — 진입점·상수 테이블·메뉴/도구 모음 구성·상태 구조체·기동 순서·레이아웃·타이틀·도크 내용·터미널 그리기·컨텍스트 메뉴·DnD 훅 — 를 nexa-dir3(크로스플랫폼)로 옮길 때의 체크리스트.
> 이 문서의 `WINA-NNN` ID는 구현·교차 검증에서 그대로 쓴다.

## 0. 범위 — 읽은 파일과 줄 수

| 파일 | 전체 줄 수 | 읽은 구간 | 비고 |
|---|---|---|---|
| `nexa-dir2/crates/nexa-app/src/main.rs` | 123 | 1~123(전부) | 진입점·모듈 게이팅 |
| `nexa-dir2/crates/nexa-app/src/win.rs` | **9951**(작업 지시서의 9703과 다름 — 실측값) | 1~3427 | 담당 = 1~3300. 3300줄이 `handle_virtual_drop` 중간이라 DnD 호버 묶음 끝(3410)까지 읽어 WINA-091·092에 포함(다음 구간 문서와 겹침 — 중복 허용) |
| `nexa-dir2/crates/nexa-app/src/win.rs` | — | 8822~8835(F3 키 분기 1곳만 확인) | 모듈 머리말 주석과 실제 동작 불일치 확인용 |

표기 약속: 이 문서에서 `win.rs:N` = `nexa-dir2/crates/nexa-app/src/win.rs:N`, `main.rs:N` = `nexa-dir2/crates/nexa-app/src/main.rs:N`. 그 밖의 파일은 `저장소/경로:줄` 전체 표기.

구조 파악: win.rs 전체의 `fn/const/struct/enum/impl` 목록을 Grep으로 뽑아 확인(1~9951). 담당 구간 밖에 있어 **이 문서가 다루지 않는** 핵심: `paint`(win.rs:4820) · `update_status`(4958) · `run_command`(5108) · `apply_prefs`(6475) · `current_settings`(6947) · `wndproc`(7664 — WM_NCCREATE 포함 전 메시지) · 단위 테스트 모듈(9717~). 이 구간의 함수가 그쪽을 호출하는 지점은 본문에 "(구간 밖)"으로 표시했다.

nexa-ui 확인 방법: `nexa-ui/crates/nexa-ctl/src` 의 `pub struct/enum/fn` Grep + `lib.rs` 재수출 목록(`nexa-ui/crates/nexa-ctl/src/lib.rs:47-80`) 열람. nexa-sql은 대응 사례 확인용으로 일부만 열람(테마·언어·창 레벨·아이콘·메모리 회수).

---

## 1. 기능 목록

이식 분류: **N** = 플랫폼 중립(거의 그대로) / **A** = nexa-ui 컨트롤·그리기로 교체 / **P** = OS별 구현 분기 필요 / **W** = Windows 전용 유지(타 OS는 대체·비활성)

### 1-1. 진입점 · 상수 테이블

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점 | Win32/OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-001 | 앱 실행 | 릴리스 빌드는 콘솔 창 없음(`windows_subsystem = "windows"`, 디버그는 콘솔 유지). `win::run()` 이 Err면 stderr에 `nexa-app 실행 실패: {e}` 후 종료 코드 1 | main.rs:4, main.rs:109-115 | PE 서브시스템 | P | 없음 |
| WINA-002 | 비Windows 스텁 | Windows가 아니면 "Windows 전용 UI" 안내 1줄만 출력하고 종료 | main.rs:117-123 | — | dir3에서 삭제(전 OS 동일 `run`) | 없음 |
| WINA-003 | 모듈 게이팅 지도 | `#[cfg(windows)]` 전용 27개: about, dw, fontchain, archivewnd, bulkrename, clipboard, conpty, cloudfs, ctl, ctldemo, dialog, dnd, icon, launcher, ordereditor, prefs, previewwnd, pwprompt, recycle, shellmenu, menuthread, shellnotify, shellpath, tip, uia, watcher, win. 전 플랫폼 컴파일(순수 로직) 14개: config, i18n, cloud, oauth, secret, fsprobe, fileinfo, icons, nav, panel, pathinput, preview, source, svg | main.rs:6-107 | — | 이식 범위 기준표(전용 27개가 P/A/W 대상) | — |
| WINA-004 | 타이머 16종 | §5-3 표 참조(ID·주기·용도). 유휴 시 스스로 꺼지는 규율(자니터·위젯 틱·폴링) | win.rs:66-132 | `SetTimer`/`KillTimer` | P(이벤트 루프 타이머로 교체) | 없음 |
| WINA-005 | 워커→UI 통지 23종 | §5-4 표 참조. 종결 통지는 재시도 게시(`post_final_notify`, 구간 밖 win.rs:7048), 진행 통지는 단발(유실 허용) | win.rs:71-203 | `PostMessageW`, `WM_APP+n`, Box 원시 포인터 전달 | P(타입 있는 사용자 이벤트로 교체) | 없음 |
| WINA-006 | 명령 ID 체계 | 메뉴·도구 모음 공용 u32. 고정 1~72, 런처 200+idx(상한 32), 클라우드 대역 299/300/340/380/420/460/492(각 32 상한 `CLOUD_MAX`). §2-3 표 참조 | win.rs:208-278 | — | N(nexa-ui는 문자열 id → 매핑 계층 필요) | 없음 |
| WINA-007 | 마우스 수식키·좌표 해석 | `MK_LBUTTON/RBUTTON/SHIFT/CONTROL` 비트, lparam 하위/상위 16비트를 부호 확장해 (x, y) | win.rs:57-61, win.rs:1806-1811 | `WM_MOUSE*` wParam/lParam | P(winit 이벤트로 대체) | 없음 |
| WINA-008 | 조작 임계 상수 | 느린 재클릭 리네임 최소 간격 1000ms · 스플리터 두께 3px · 자석 스냅 20px(Alt=해제) · 패널 최소 폭 200px · 스플리터 히트 반폭 3px · 벤치 200프레임 | win.rs:50-64, win.rs:204-206 | — | N | 없음 |

### 1-2. 테마 · 언어 · 창 속성

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-009 | 테마 모드 3종 | `System/Light/Dark`. 문자열 `"system"/"light"/"dark"`. **알 수 없는 값은 Dark**(`from_str`의 `_`) | win.rs:280-286, win.rs:360-375 | — | N | 없음 |
| WINA-010 | OS 라이트/다크 판정 | HKCU `Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` 의 `AppsUseLightTheme`(DWORD). 0이 아니면 라이트. **조회 실패 = 라이트** | win.rs:288-303 | `RegGetValueW` | P | 없음 |
| WINA-011 | 실효 테마 결정 | Light/Dark는 고정, System은 WINA-010 결과 추종 | win.rs:305-318 | — | N | 없음 |
| WINA-012 | OS UI 언어 조회 | BCP-47 로캘명(예 `ko-KR`), 실패 시 `"en"`. i18n "system" 해석에 1차 서브태그만 사용 | win.rs:320-330 | `GetUserDefaultLocaleName` | P | 없음 |
| WINA-013 | 항상 맨 위에 표시 | 토글 시 TOPMOST 밴드 전환 — 활성화·위치·크기는 불변 | win.rs:332-346 | `SetWindowPos(HWND_TOPMOST/NOTOPMOST, NOMOVE+NOSIZE+NOACTIVATE)` | P | 없음 |
| WINA-014 | 타이틀바 다크 모드 | 본문 테마가 다크면 제목 표시줄도 다크 | win.rs:348-358 | `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` | P | 없음 |

### 1-3. 메뉴 · 도구 모음 · 런처 구성

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-015 | 메뉴 바 5개 | 파일 / 편집 / 보기 / 클라우드 / 도움말. 라벨은 i18n — 언어 전환 시 전체 재구성(`set_menus`). 항목 상세는 §2-3 | win.rs:377-484 | — | A | 없음 |
| WINA-016 | 파일 메뉴 | 새 탭(Ctrl+T) · 탭 닫기(Ctrl+W) · ─ · 새 폴더(Ctrl+Shift+N) · 새 파일 · ─ · 설정(Ctrl+,) · ─ · 종료 | win.rs:437-450 | — | A | 없음 |
| WINA-017 | 편집 메뉴 | 실행 취소(Ctrl+Z) · 다시 실행(Ctrl+Y) · ─ · 잘라내기/복사/붙여넣기(Ctrl+X/C/V) · ─ · 모두 선택(Ctrl+A) · ─ · 일괄 이름변경(Ctrl+Shift+R). **비활성 표시 없음** — 할 것이 없으면 상태바로 알림 | win.rs:451-467 | — | A | 없음 |
| WINA-018 | 보기 메뉴 | 보기 모드 라디오 3(트리/일반/타일) · 패널 듀얼/싱글 · 정보 듀얼/싱글 · 컬럼 너비 동기화 · 숨김(Ctrl+H) · Dot(Ctrl+.) · 도크(Ctrl+`) · 런처 · 항상 맨 위 · 새로고침(F5) · 테마 라디오 3(모두 F6 표기 = 순환) · 언어 라디오. **정보 라디오의 체크는 효과 기준**(싱글 패널이면 싱글 정보 고정 — `info_single_eff`) | win.rs:396-435 | — | A | 없음 |
| WINA-019 | 언어 라디오 동적 목록 | "시스템"(40) + 발견된 언어 각각(41+idx). 목록 = 데이터 폴더에서 발견한 (코드, 표기) | win.rs:429-435, win.rs:237-239 | — | A | 없음 |
| WINA-020 | 클라우드 메뉴 | 연결마다 하위 메뉴(바로 가기 / 온라인 보기 / URL 복사 / ─ / 연결 해제[API 연결] 또는 링크 해제[동기화 폴더]). 그 아래 "연결 추가하기"(감지 후보 하위 — 후보 0이면 단독 항목 299, 클릭 시 상태바 안내). ─ 후 "Connect Cloud"(서비스별 OAuth 하위, 492+idx). 연결·후보 각 32개 상한 | win.rs:486-538 | — | A | 없음 |
| WINA-021 | 도움말 메뉴 | About 1개(홈페이지 항목은 미완) | win.rs:478-482 | — | A | 없음 |
| WINA-022 | 미연결 클라우드 후보 | `cloud::detect()` 결과 중 이미 연결된 경로를 제외 | win.rs:540-550 | (cloud.rs 내부 — 구간 밖) | N(감지 자체는 P) | 없음 |
| WINA-023 | 클라우드 → 내 PC 루트 동기 | 동기화 폴더 연결은 **실존 폴더만** 노출(잔재 숨김). API 연결은 `::CLOUD:<idx>::` 센티널 루트 | win.rs:552-573 | — | N | 없음 |
| WINA-024 | 패널이 클라우드를 보는가 | 루트가 가상 최상위(내 PC)거나 해당 연결의 클라우드 경로면 참 | win.rs:575-586 | — | N | 없음 |
| WINA-025 | 클라우드 로딩 배지 | 두 패널 각각: 내 PC면 어느 연결이든 진행 중일 때, 클라우드 경로면 그 연결이 진행 중일 때 "불러오는 중" 배지. 상태 변화 지점에서만 호출 | win.rs:588-605 | — | N | 없음 |
| WINA-026 | 클라우드 열거 콜백 | 기동 1회 등록. 캐시 적중이면 즉시 반환. 미적중: 토큰 없으면 빈 목록, 있으면 워커 기동 후 **"불러오는 중" 플레이스홀더 행 1개** 반환(대상 = 현재 폴더 자신 → 실수로 열어도 무해). 연결 정보 없으면 빈 목록 | win.rs:607-656 | `state_of`(창 userdata) 접근 | N(상태 접근 방식만 교체) | 없음 |
| WINA-027 | 보기 토글 툴팁 문구 | `"라벨 — 적용 범위"`. 범위 = global/panel/tab(기본 panel) → 설정 창 옵션 키 재사용 | win.rs:658-670 | — | N | 없음 |
| WINA-028 | 도구 모음 구성 | 블록 5개·버튼 13개를 설정 `toolbar_order` 순서/표시대로 배치. 블록 사이 구분선, 숨긴 블록·모두 숨긴 블록은 구분선까지 제거. 전 버튼 = 임베드 SVG 아이콘(`emb:*`) + 미로드 폴백 글리프 + 툴팁 + 토글 상태. 활성 규칙: `info` = 듀얼 패널 **그리고** 도크 켜짐일 때만, `colsync` = 듀얼 패널일 때만. 상세 §2-4 | win.rs:672-792 | — | A | 없음(순서 파서는 config.rs 쪽) |
| WINA-029 | 퀵 런처 바 구성 | 항목마다 exe 셸 아이콘(스몰 16px) 정사각 버튼, `-` 항목은 구분선. 아이콘 실패 시 라벨 앞 2자 폴백(위젯 쪽 동작 — 추정). 명령 = 200+idx | win.rs:794-813 | 셸 아이콘(icons 모듈) | A + P(아이콘 취득) | 없음 |

### 1-4. 상태 구조체 · 순수 판정

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-030 | 단조 시각 | 프로세스 기동 기준 ms(`OnceLock<Instant>`) | win.rs:815-820 | — | N | 없음 |
| WINA-031 | 페인트 통계 | 누적 µs·프레임 수·기동→첫 페인트 ms. 벤치 시 리셋(첫 렌더 값은 보존) | win.rs:822-847 | — | N | 없음 |
| WINA-032 | 창 상태 `State` | 필드 약 110개 — §5-1에 묶음별 정리 | win.rs:849-1070 | `HWND`(tip_win), `DwBackend`, `ShellIcons` 등 | N + A | — |
| WINA-033 | 터미널 상태 `TermState` | PTY + VT 화면 + 스크롤백 보기 오프셋 + 선택(앵커, 끝) + 셀 그리드 캐시 + 가로 오프셋 + 휠 누적 4종 + 부분 행/열 픽셀 오프셋 | win.rs:1072-1121 | `conpty::ConPty` | P(PTY) + N(나머지) | 없음 |
| WINA-034 | 터미널 보기 이동 | 행 단위(`scroll_view`, 양수=위·부분 오프셋 0으로 스냅) · 세로 픽셀(`scroll_view_px`) · 가로 픽셀(`scroll_view_x_px`). 범위는 스크롤백 수/열 수로 클램프, 변화 있을 때만 true | win.rs:1123-1138, win.rs:1166-1194 | — | N | 없음 |
| WINA-035 | 터미널 선택·히트 | `sel_norm` = 정렬된 범위(앵커=끝이면 None). `cell_at` = 좌표 → (절대 라인, 열), 부분 오프셋 반영, 범위 밖은 가장자리 클램프 | win.rs:1140-1164 | — | N | 없음 |
| WINA-036 | 전송 공유 상태 | 워커와 UI가 공유: 취소 플래그 · 완료/전체 바이트 · 결과 · 항목별 세그먼트 진행 · 현재 쓰는 대상 경로(완료 전 실행/이동 차단). 워커는 `State` 접근 금지 | win.rs:1197-1234 | — | N | 없음 |
| WINA-037 | 활성 패널·네비 문맥 | `nav_ctx`(숨김·dot·tz·빈 폴더 글리프 억제) · `active_panel` · `panel_at(x)`(스플리터 존 = None) | win.rs:1236-1279 | — | N | 없음 |
| WINA-038 | 포인터 히트 존 | `Panel(i)` / `SharedDock` / `Split`. 도크 밴드 안은 도크 분할 기준, 밖은 파일 스플리터 기준. **싱글 정보의 공유 도크를 눌러도 활성 패널 유지** | win.rs:1281-1342 | — | N | `hit_zone_table` win.rs:9786 |
| WINA-039 | 패널 지표 | 96dpi 기준 행 20(최소 14) · 좌우 여백 6 · 들여쓰기 16 · 탭 22 · 경로 바 24, DPI 비례 | win.rs:1344-1354 | — | N | 없음 |
| WINA-040 | 기본 5컬럼 | 이름 340 · 확장자 64 · 크기 96(우측 정렬) · 수정한 날짜 140 · 종류 110(px @96dpi), 제목 = i18n `col.*` | win.rs:1356-1366 | — | N(컬럼 타입은 A) | 없음 |
| WINA-041 | 로컬 타임존 오프셋 | 분 단위(UTC 동쪽 양수), 표준/일광 절약 반영. 기동 시 1회 조회해 고정 | win.rs:1368-1379 | `GetTimeZoneInformation` | P | 없음 |
| WINA-042 | 시작 경로 | argv[1] → `%USERPROFILE%` → `C:\` | win.rs:1381-1388 | 환경 변수·드라이브 문자 | P | 없음 |
| WINA-043 | 시작 트리 열기 폴백 | 열기 실패 시 전 드라이브·홈 순회(`panel::open_any_root`). 전부 실패하면 panic | win.rs:1390-1397 | — | N(순회 대상은 P) | 없음 |

### 1-5. 기동 순서(`run`)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-044 | 크래시 로그 | panic 후크가 메시지·위치를 `data\crash.txt`에 기록(릴리스는 `panic=abort`라 유일한 단서) | win.rs:1404-1413 | — | N | 없음 |
| WINA-045 | 설정·세션 로드 | `data\settings.cfg`, `data\session.cfg`. 없으면 기본값. 구 `.txt`는 1회성 마이그레이션 | win.rs:1414-1419 | — | N(파일 구조는 nexa-sql 방식으로 교체) | (config.rs 테스트) |
| WINA-046 | 고속 스크롤·휠 줄 수 | 전역 고속 스크롤 설정 2종 주입 + 시스템 "한 번에 스크롤할 줄 수" 동기 | win.rs:1421-1425 | `sync_wheel_lines`(구간 밖 win.rs:6327) | N + P | 없음 |
| WINA-047 | i18n 활성화 | 언어 발견 → 설정/OS 언어로 코드 결정 → 로드. **패널·메뉴 생성 전에** 수행(컬럼 제목·라벨이 `tr()` 경유) | win.rs:1426-1429 | WINA-012 | N | (i18n.rs 테스트) |
| WINA-048 | 압축 이름 디코더 주입 | 구형 zip의 CP949 한글 이름용(UTF-8 플래그 없는 항목만) | win.rs:1430-1432 | (preview 내부 — 추정: Windows 코드페이지 API) | P | 없음 |
| WINA-049 | 클라우드 루트 선동기 | **패널 복원보다 먼저** — 세션에 내 PC 탭이 있으면 이 목록으로 열거되므로 | win.rs:1442-1446 | — | N | 없음 |
| WINA-050 | 패널 생성 | argv 경로가 있으면 좌·우 모두 그 경로, 활성 = 좌. 없으면 세션 복원(탭·활성 탭·펼침·활성 패널) | win.rs:1447-1475 | — | N | (panel.rs 테스트) |
| WINA-051 | 탭 잠금·고정 복원 | 세션의 locked/pinned | win.rs:1477-1484 | — | N | 없음 |
| WINA-052 | 도크 복원 | 비율(`dock_ratio`)은 항상, 표시(`dock`)는 켜져 있을 때만 양 패널에 적용 | win.rs:1485-1494 | — | N | 없음 |
| WINA-053 | 정렬·보기·타입어헤드 적용 | 폴더 우선 끔/대소문자 구분/Alt+↑ 배치/폰트 장식(기본이 아닐 때만) · 탭별 보기 모드(세션 우선, 없으면 설정) · 탭별 보기 플래그 · 타입어헤드 옵션(리셋 ms 최소 1, HUD 위치 0~8)은 항상 | win.rs:1495-1546 | — | N | 없음 |
| WINA-054 | 런처 시드 | 키 부재(첫 실행)면 시드, 시드 버전이 낮으면 신규 시드만 1회 추가 | win.rs:1547-1556 | (launcher.rs — VS Code·pwsh·cmd) | P(시드 목록) | 없음 |
| WINA-055 | `State` 초기화 | 설정/세션 값 복사 + 런타임 필드 초기값(DPI 96, split 등). 컬럼 폭·레이아웃은 보류 슬롯에 담아 DPI 반영 뒤 적용 | win.rs:1557-1722 | — | N | 없음 |
| WINA-056 | DPI 인식 | 모니터별 DPI 인식 V2. 실제 DPI는 창 생성 시 반영(구간 밖) | win.rs:1725, win.rs:1439 | `SetProcessDpiAwarenessContext` | P | 없음 |
| WINA-057 | 메인 창 생성 | 클래스 `NexaDirMain`(더블클릭 통지, 화살표 커서, 32px 아이콘), 제목 `Nexa Dir`, 일반 겹침 창, 위치 OS 기본, **크기 1400×800**, 작은 아이콘 16px 별도 설정. 이 구간에 창 위치·크기 복원 로직은 없음 | win.rs:1727-1768 | `RegisterClassW`, `CreateWindowExW`, `WM_SETICON`, `LoadCursorW` | P | 없음 |
| WINA-058 | DnD 수신 등록 | OLE 초기화 후 드롭 대상 등록. 훅 5개: 대상 폴더 판정 · 드롭 확정 · 추적 · 이탈 · 가상 파일 드롭. 등록 실패는 로그만 남기고 계속 | win.rs:1742-1743, win.rs:1770-1784 | `OleInitialize`, `RegisterDragDrop`, `IDropTarget` | P | 없음 |
| WINA-059 | 항상 위 복원 | 설정이 켜져 있으면 기동 시 적용 | win.rs:1786-1789 | WINA-013 | P | 없음 |
| WINA-060 | 잘라내기 흐림 동기 | 클립보드 변경 구독 + 시작 시 1회 동기(앱 시작 전에 탐색기에서 잘라낸 것도 흐림) | win.rs:1791-1794 | `AddClipboardFormatListener`, `WM_CLIPBOARDUPDATE` | P | 없음 |
| WINA-061 | 메시지 루프 | 표준 Get/Translate/Dispatch | win.rs:1796-1800 | `GetMessageW` 등 | P | 없음 |

### 1-6. 레이아웃 · 타이틀 · 자원 관리

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-062 | 창 상태 접근·클라이언트 영역 | 창 userdata의 `Box<State>` 포인터, 클라이언트 rect | win.rs:1813-1822 | `GetWindowLongPtrW(GWLP_USERDATA)`, `GetClientRect` | P | 없음 |
| WINA-063 | 무효화 반영 | 위젯이 모은 rect마다 부분 무효화. 틱 요청이 있으면 40ms 위젯 틱 타이머 무장 | win.rs:1824-1837 | `InvalidateRect`, `SetTimer` | P | 없음 |
| WINA-064 | 스플리터 위치 | 폭 × 비율, 패널 최소 폭 200px로 클램프(창이 좁으면 절반 기준) | win.rs:1839-1845 | — | N | 없음 |
| WINA-065 | 전체 레이아웃 | 메뉴 22 / 도구 모음 28 / 런처 24(숨김·실행 항목 0이면 0) / 패널 영역 / 도크 밴드 / 상태바 22. 싱글 패널·싱글 정보 변형 포함. 상세 §2-1·2-2 | win.rs:1847-1934 | — | N | 없음 |
| WINA-066 | 타이틀 문자열 | `Nexa Dir — [좌/우] 경로 [항목 수 · 선택 수 · 탭 n/m · 평균 페인트 µs · 첫 렌더 ms] 메모`. 내 PC·클라우드는 사람이 읽는 라벨(센티널 노출 금지) | win.rs:1936-1982 | `SetWindowTextW` | N + P(설정 API) | 없음 |
| WINA-067 | 그리기 백엔드 확보 | 없으면 생성(폰트 3슬롯 + 터미널 폰트), 크기가 다르면 리사이즈. 실패는 로그 | win.rs:1984-2009 | DirectWrite/GDI(`dw.rs`) | A | 없음 |
| WINA-068 | 유휴 트림 | 마지막 입력 후 60초 경과 + 미트림이면: 백버퍼·레이아웃 캐시·아이콘 캐시 해제 + 작업집합 반납. 화면 무효화 없음(다음 페인트에서 재적재). 입력 오면 플래그 해제·자니터(10초) 재가동 | win.rs:2011-2034 | `SetProcessWorkingSetSize` | N(판정·캐시 해제) + P(메모리 반납) | `idle_trim_threshold_and_once` win.rs:9941 |
| WINA-069 | 접근성(UIA) | 활성 패널 가시 행을 화면 좌표 스냅샷으로 제공. 캐럿·구조(패널, 경로, 행 수) 변화 시 이벤트 — 클라이언트가 붙어 있을 때만, 중복 억제 | win.rs:2036-2099 | UI Automation, `ClientToScreen` | W | 없음 |

### 1-7. 클립보드 · 붙여넣기 · 스테이징

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-070 | 선택 → 클립보드 항목 | 활성 패널 선택 경로 + 연산(복사/이동). 선택 없으면 None(클립보드 유지) | win.rs:2101-2113 | — | N | 없음 |
| WINA-071 | 붙여넣기 대상 폴더 | 선택 1개가 폴더면 그 안, 파일이면 **그 파일의 부모 폴더**, 그 외(없음·다중)는 현재 폴더 | win.rs:2115-2133 | — | N | 없음 |
| WINA-072 | 가상 파일 붙여넣기 | 실경로 없는 클립보드(RDP·메일 첨부·압축 폴더)용 폴백. ① 클라우드 대상 = 임시 폴더로 동기 추출 후 업로드 전송 ② 진행 창 슬롯 사용 중 = 대상에 동기 추출 + undo 기록 + 재로드 ③ 그 외 = 계획 수립 → 진행 창(`transfer_close_ms > 0`일 때만) → 워커 추출, 완료 통지 `WM_APP_VPASTE` | win.rs:2135-2183 | OLE 클립보드 스트림(`clipboard.rs`), `SetCursor(IDC_WAIT)` | W(가상 파일 형식) + P(대체) | 없음 |
| WINA-073 | 가상 붙여넣기 스테이징 폴더 | `<temp>/NexaDir/vpaste-<pid>-<seq>` | win.rs:2185-2194 | — | N | 없음 |
| WINA-074 | 붙여넣기 생성물 undo | 실제 존재하는 최상위만 기록, 전량 실패면 무기록. 설명 = "n개 복사" | win.rs:2196-2207 | — | N | 없음 |
| WINA-075 | DnD 스테이징 판정·정리 | `<temp>/NexaDir/dnd-*/…` 아래만 스테이징으로 판정. 전송 결과를 (일반, 스테이징 출신)으로 분리. 빈 슬롯 폴더만 `dnd-N/i` → `dnd-N` 순으로 삭제(잔존 항목 있으면 그대로) | win.rs:2209-2244 | — | N | `split_staged_only_dnd_staging_sources` win.rs:9852 · `cleanup_staging_slots_removes_only_empty_dirs` win.rs:9892 |

### 1-8. 도크(정보 · 미리보기 · 터미널)

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-076 | 도크 대상 키 | 단일 선택 = 그 경로, 다중 = `*n`, 없음 = 현재 폴더. 같은 대상의 갱신은 스크롤 유지 | win.rs:2246-2259 | — | N | 없음 |
| WINA-077 | 도크 정보 | 다중 선택 = "n개 선택" 1줄. 단일 = 이름·종류·경로·크기·디스크 할당 크기·만든/수정한/액세스한 날짜(값 있는 것만) + 형식별 상세(워커 결과가 이 경로 것일 때만, 아니면 "불러오는 중"). 메타 불가(클라우드 등)는 트리 값으로 폴백. 선택 없음 = "현재 폴더: …". 클라우드는 라벨 경로 | win.rs:2261-2364 | 속성 시스템(`fileinfo.rs` 워커) | N(기본 줄) + P(상세·디스크 할당 크기) | 없음 |
| WINA-078 | 도크 미리보기 내용 | 단일 선택 파일만(그 외 "미리보기 없음"). 공급자 결정 = 설정 오버라이드 > 확장자 선언 > 텍스트 폴백. 줄 문서는 종류 태그를 벗겨 평문으로(hr = `─`×40, 인용 = `│ ` 접두). 이미지는 경로 반환. 압축은 요약 + 앞 60개 항목 | win.rs:2366-2420 | (이미지 디코드는 도크/백엔드 쪽) | N | 없음 |
| WINA-079 | 독립 미리보기 창 | **F3**(Shift+F3 = 벤치 — win.rs:8825-8832) 또는 도크 ↗. 단일 선택 파일만. 줄 문서 → 콘솔 폰트 그리드 모달 창, 이미지 → 안내 1줄, 압축 → 그리드 창(이미 읽은 목록 재사용) | win.rs:2422-2472 | `previewwnd`/`archivewnd`(별도 HWND), GDI 폰트 | A(별도 창) | 없음 |
| WINA-080 | 도크 내용 갱신 | 표시 중인 도크만. 종류 탭 3개(정보/미리보기/터미널). 싱글 정보면 우 도크 건너뜀·내용 원천 = 활성 패널. 정보 종류는 상세 요청(같은 경로 미도착 요청이 있으면 재요청 안 함, 세대 번호 증가). 미리보기 종류일 때만 ↗ 오버레이 | win.rs:2474-2529 | — | N + A | 없음 |
| WINA-081 | 터미널 그리기 | 지연 시작(첫 그리기에서 PTY 기동) · 크기 동기 · 셀 그리드 · 선택 반전 · faint · 전각 · 세로바 캐럿 · 종료 안내 · 고속 스크롤 배지. 시작 실패 시 안내 문구를 그리고 false 반환(호출자가 키 포커스 해제). 상세 §2-6 | win.rs:2531-2736 | `ConPty::start/resize`, DirectWrite 셀 텍스트 | P(PTY) + A(그리기) | 없음 |

### 1-9. 조작 대상 · 컨텍스트 메뉴 · 휴지통

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-082 | 키보드 조작 대상 | 선택이 있으면 선택, 없으면 캐럿 행 | win.rs:2738-2755 | — | N | 없음 |
| WINA-083 | 표시 순서 대상 | 가시 행을 위에서부터 훑어 선택 항목 수집(선택 삽입 순서가 아님). 비면 WINA-082로 폴백 | win.rs:2757-2777 | — | N | 없음 |
| WINA-084 | 우클릭 대상 | WINA-082 결과를 **캐럿 항목의 부모 폴더와 같은 것만**으로 축소 | win.rs:2779-2801 | (셸 메뉴가 단일 부모만 표현) | N(축소 규칙 유지 여부는 결정 필요) | 없음 |
| WINA-085 | 고유 메뉴 항목 ID | 완전 삭제 · 폴더에 붙여넣기 · 실행 취소 · 다시 실행 · 배경 붙여넣기 · 경로 복사 · 이름 복사(`ID_CUSTOM_FIRST`+0~6) | win.rs:2803-2812 | — | N | 없음 |
| WINA-086 | 빈 영역 컨텍스트 메뉴 | 내 PC 배경이면 WINA-092로 위임. 그 외: 폴더 배경 셸 메뉴 + 고유 항목(붙여넣기[클립보드에 파일 있을 때만 활성] · 실행 취소 · 다시 실행 — 순서/표시 = 설정 `ctx_menu_order`의 `bg` 블록, 라벨에 대상 설명 포함). Shift = 확장 메뉴. 결과: 셸 실행 → 양쪽 재로드 / 새로 만들기 → 캐럿 이동 + 인라인 리네임 / 붙여넣기(셸 동사 가로채기·고유 항목 동일 경로) → 전송 엔진, 없으면 가상 붙여넣기 / undo·redo | win.rs:2814-2912 | `shellmenu::show_background`(IContextMenu), `GetKeyState(VK_SHIFT)` | P | 없음 |
| WINA-087 | 행 메뉴 요청 구축 | 대상(같은 부모) · 전체 선택(표시 순서) · Shift 확장 · 가로챌 동사(delete/rename/copy/cut) · 제자리 대체(셸 "경로 복사" → 고유 항목) · 고유 항목(`row` 블록 순서/표시: 새로 만들기[단일 선택만 — 파일=부모, 폴더=자신] · 완전 삭제 · 이름 복사[경로 복사 바로 아래 고정] · 폴더에 붙여넣기[단일 폴더 + 클립보드에 파일]). 키보드 호출이면 캐럿 행 앵커의 화면 좌표. 상태를 읽기만 하고 즉시 반환 | win.rs:2914-3032 | `ClientToScreen`, `GetKeyState` | N(요청 데이터) + P(표시) | 없음 |
| WINA-088 | 행 메뉴 표시 | 이미 떠 있으면 무시. 메뉴 스레드가 있으면 세대 +1 후 표시 요청만 보내고 반환(결과는 `WM_APP_CTXMENU_RESULT`). 없으면 동기 폴백 | win.rs:3034-3072 | `menuthread`, `shellmenu::show` | P | 없음 |
| WINA-089 | 행 메뉴 결과 반영 | 셸 실행 → 재로드 / 새로 만들기 → 리네임 진입 / delete → 앱 삭제 경로(Shift로 연 메뉴면 완전 삭제) / rename → 인라인 리네임 / copy·cut → **교차 폴더 전체 선택**을 앱 클립보드로 / 경로 복사·이름 복사 → CRLF 구분 텍스트 / 완전 삭제 / 폴더에 붙여넣기(이동이면 클립보드 비움) | win.rs:3074-3140 | 클립보드 쓰기 | N(분기 로직) | 없음 |
| WINA-090 | 메뉴 선행 구축 | 선택이 바뀌면 300ms 뒤 메뉴 스레드에 미리 구축. 대상이 같으면 재요청 없음, 대상이 비면 준비분 무효화, 표시 중이면 건너뜀 | win.rs:3142-3178 | `SetTimer`, `menuthread` | W(셸 메뉴 구축 지연 대응 — 자체 메뉴면 불필요) | 없음 |
| WINA-091 | 휴지통 삭제(배치) | 확인창·UI 없이 휴지통으로. 성공 = 반환 0 **그리고** 중단 없음 | win.rs:3180-3200 | `SHFileOperationW(FO_DELETE, FOF_ALLOWUNDO)` | P | 없음 |
| WINA-092 | 내 PC 클라우드 메뉴 | 감지 후보(활성 — "…에 연결") → 연결된 항목(체크·회색) → 둘 다 없으면 안내 1줄(회색). 선택 시 "연결 추가" 명령과 같은 경로. 후보 스냅숏을 상태에 고정해 인덱스 어긋남 방지 | win.rs:3202-3249 | `CreatePopupMenu`, `TrackPopupMenuEx`, `GetCursorPos` | A | 없음 |

### 1-10. 드래그 앤 드롭 훅

| ID | 기능 | 동작 상세 | 진입점 | OS 의존 API | 분류 | 기존 테스트 |
|---|---|---|---|---|---|---|
| WINA-093 | 드롭 대상 폴더 판정 | 폴더 행 위 = 그 폴더, 파일 행·빈 본문 = 그 패널의 현재 폴더, 스플리터 존 = 없음 | win.rs:3251-3268 | `ScreenToClient` | N | 없음 |
| WINA-094 | 외부 드롭 확정 | 전송이 이미 진행 중이면 **거부(false)** + 제목줄 "작업 중" 안내(발신 쪽이 확보 파일을 되돌림). 아니면 전송 엔진 합류 후 true | win.rs:3270-3288 | `IDropTarget::Drop` | N(판정) + P(수신) | 없음 |
| WINA-095 | 가상 파일 드롭 | 실경로 없는 소스의 스트림 추출. 클라우드 대상은 무동작. 드롭 반환 전 동기 실행(대용량이면 창 멈춤 — 알려진 한계). 생성물은 undo 기록 + 재로드 | win.rs:3290-3313 | `IDataObject`(FILEDESCRIPTOR/FILECONTENTS), `SetCursor` | W + P(대체) | 없음 |
| WINA-096 | 드래그 중 호버·엣지 스크롤 | 100ms 폴링 타이머. ① 커서 아래 패널 본문 상/하단 엣지 = 1행 자동 스크롤 ② 비활성 탭(우선) 또는 접힌 폴더 행 위에 `dnd_hover_ms`(기본 3000) 이상 머물면 탭 전환/폴더 펼침. 대상이 바뀌면 대기 리셋. 발동 시 행-경로 재검증(밀렸으면 무시). 이탈 시 타이머·대기 해제 | win.rs:3315-3410 | `SetTimer(TIMER_DND)`, `ScreenToClient` | N(판정) + P(추적 입력) | 없음 |

**기능 수: 96개(WINA-001 ~ WINA-096).**

---

## 2. 화면·컨트롤 배치

모든 px 값은 96dpi 기준 논리값이며 `s(v) = v × dpi / 96`(정수 나눗셈)으로 환산한다(win.rs:1850).

### 2-1. 메인 창 세로 구성

창: 초기 1400×800, 제목 `Nexa Dir`(win.rs:1748-1753). 위에서 아래로:

| 순서 | 영역 | 높이 | rect | 근거 |
|---|---|---|---|---|
| 1 | 메뉴 바 | 22 | `(0, 0, W, min(22, H))` | win.rs:1858-1860 |
| 2 | 도구 모음 | 28(아이콘 20) | `(0, 22, W, 28)` | win.rs:1851-1862 |
| 3 | 퀵 런처 바 | 24(아이콘 16) / 0 | `(0, 50, W, h)` — 숨김이거나 실행 항목(구분선 제외)이 0이면 h=0 | win.rs:1863-1871 |
| 4 | 패널 영역(좌 ║ 우) | 나머지 − 도크 밴드 | top = 22+28+런처 | win.rs:1872, win.rs:1891-1902 |
| 5 | 도크 밴드(전폭) | 영역 × `dock_ratio`, 범위 `[min(행높이×3, 영역/2), 영역/2]`. 도크 꺼짐 = 0 | 패널 영역 바로 아래 | win.rs:1885-1890 |
| 6 | 상태바 | 22 | `(0, bottom, W, H−bottom)`, `bottom = max(H−22, top)` | win.rs:1873-1875 |

도구 모음 28·런처 24 분리는 여러 차례 조정 끝에 확정된 값이다(이력: 24→26→24→32→28/24). 아이콘 20·16은 래스터 버킷과 일치해 늘림 없이 선명 — 값을 바꾸지 말 것(win.rs:1851-1857).

### 2-2. 가로 분할

- 스플리터 두께 `gap = max(s(3), 2)`, `g2 = gap / 2`(win.rs:1879-1880). 파일 좌/우 · 도크 좌/우 · 가로 분리선 **전부 같은 두께**.
- 스플리터 x: `sx = clamp(W × split, min(s(200), W/2), max(W − s(200), W/2))`(win.rs:1840-1845).
- 듀얼 패널: 좌 `(0, top, sx − g2, ph)` / 우 `(sx − g2 + gap, top, W − sx + g2 − gap, ph)`(win.rs:1897-1901).
- 싱글 패널: 좌 = 전폭, 우 = 빈 rect(상태·탭은 보존)(win.rs:1892-1895).
- 도크 밴드: 가로 분리선 두께 gap → 내용은 `dock_y = band_y + gap`, `dock_h = band_h − gap`. 도크 분할 x `dsx = clamp(W × dock_split, W/8, W×7/8)` — **파일 스플리터와 독립**(win.rs:1903-1908).
  - 듀얼 정보: 좌 도크 `(0, dock_y, dsx − g2, dock_h)` / 우 도크 `(dsx − g2 + gap, dock_y, W − dsx + g2 − gap, dock_h)`(win.rs:1916-1928).
  - 싱글 정보: 좌 도크 = 전폭 1개(내용 = 활성 패널 추종), 우 도크 = 빈 rect(win.rs:1909-1915).
- 패널 내부 지표: 행 20(최소 14) · 좌우 여백 6 · 들여쓰기 16 · 탭 줄 22 · 경로 바 24(win.rs:1345-1354). 패널 내부 배치(탭 줄/경로 바/헤더/목록)는 `panel.rs` 담당 문서 참조.
- 기본 컬럼 폭: 이름 340 / 확장자 64 / 크기 96(우측 정렬) / 수정한 날짜 140 / 종류 110(win.rs:1357-1366).

### 2-3. 메뉴 항목 전체(순서 그대로)

`✓` = 체크/라디오 표시 대상. 단축키 열은 메뉴에 **표기되는 문자열**이며 실제 키 처리는 `wndproc`(구간 밖 — win.rs:8813~8910 부근)에 있다.

**파일**(win.rs:437-450)

| 순서 | 항목(i18n 키) | 명령 | 단축키 표기 |
|---|---|---|---|
| 1 | `menu.file.newTab` | 1 | Ctrl+T |
| 2 | `menu.file.closeTab` | 2 | Ctrl+W |
| ─ | | | |
| 3 | `menu.file.newFolder` | 4 | Ctrl+Shift+N |
| 4 | `menu.file.newFile` | 5 | |
| ─ | | | |
| 5 | `menu.file.prefs` | 60 | Ctrl+, |
| ─ | | | |
| 6 | `menu.file.exit` | 3 | |

**편집**(win.rs:451-467)

| 순서 | 항목 | 명령 | 단축키 표기 |
|---|---|---|---|
| 1 | `menu.edit.undo` | 6 | Ctrl+Z |
| 2 | `menu.edit.redo` | 7 | Ctrl+Y |
| ─ | | | |
| 3 | `menu.edit.cut` | 69 | Ctrl+X |
| 4 | `menu.edit.copy` | 70 | Ctrl+C |
| 5 | `menu.edit.paste` | 71 | Ctrl+V |
| ─ | | | |
| 6 | `menu.edit.selectAll` | 72 | Ctrl+A |
| ─ | | | |
| 7 | `menu.edit.bulkRename` | 61 | Ctrl+Shift+R |

**보기**(win.rs:396-435)

| 순서 | 항목 | 명령 | 단축키 표기 | ✓ 조건 |
|---|---|---|---|---|
| 1 | `menu.view.modeTree` | 13 | | 보기 모드 = tree |
| 2 | `menu.view.modeFlat` | 14 | | = flat |
| 3 | `menu.view.modeTiles` | 15 | | = tiles |
| ─ | | | | |
| 4 | `menu.view.panelDual` | 17 | | 패널 모드 = dual |
| 5 | `menu.view.panelSingle` | 16 | | = single |
| 6 | `menu.view.infoDual` | 19 | | 효과상 듀얼 정보 |
| 7 | `menu.view.infoSingle` | 18 | | 효과상 싱글 정보(싱글 패널이면 항상) |
| 8 | `menu.view.colWidthSync` | 63 | | 켜짐 |
| ─ | | | | |
| 9 | `menu.view.hidden` | 10 | Ctrl+H | 켜짐 |
| 10 | `menu.view.dot` | 11 | Ctrl+. | 켜짐 |
| 11 | `menu.view.dock` | 8 | Ctrl+` | 켜짐 |
| 12 | `menu.view.launcher` | 9 | | 켜짐 |
| 13 | `menu.view.alwaysOnTop` | 68 | | 켜짐 |
| ─ | | | | |
| 14 | `menu.view.refresh` | 12 | F5 | |
| ─ | | | | |
| 15~17 | `menu.view.theme.system/light/dark` | 30/31/32 | F6(셋 다) | 현재 모드 |
| ─ | | | | |
| 18 | `menu.view.lang.system` | 40 | | 설정 = system |
| 19~ | 발견된 언어 표기 | 41+idx | | 설정 = 그 코드 |

**클라우드**(win.rs:489-538): `[연결 라벨 ▸ (cloud.goto 300+i / cloud.web 340+i / cloud.copyUrl 380+i / ─ / cloud.disconnect 또는 cloud.unlink 420+i)]…` → (연결이 있으면 ─) → `cloud.add ▸ (후보 라벨 460+i)…`(후보 0이면 단독 항목 299) → ─ → `cloud.connect ▸ (서비스 표시명 492+i)…`.

**도움말**(win.rs:478-482): `menu.help.about`(66).

메뉴에 노출되지 않는 명령: 22(위로)·20/21(이전/다음)은 패널별 네비 바 전담, 62(컨트롤 갤러리)는 개발용 주입 전용, 64/65/67은 도구 모음 전용(win.rs:231-257).

이 구간에서 확인되는 그 밖의 키: F3 = 독립 미리보기, Shift+F3 = 벤치(win.rs:8825-8832). F2 · Apps · Shift+F10 · Tab 등은 다음 구간 문서에서 확정한다.

### 2-4. 도구 모음 버튼(기본 순서)

기본 순서 = `refresh | panel[toggle, dock, info, colsync, ontop] | view[tree, flat, tiles] | show[hidden, dot, foldersfirst] | settings`(`nexa-dir2/crates/nexa-app/src/config.rs:1036-1044`). 블록 사이에 구분선.

| 블록 | 항목 | 명령 | 아이콘 키 | 폴백 글리프 | 툴팁 | 토글 | 활성 조건 |
|---|---|---|---|---|---|---|---|
| refresh | — | 12 | `emb:refresh` | ⟳ | `menu.view.refresh` | — | 항상 |
| panel | toggle | 64 | `emb:panel-toggle` | ▌▐ | 듀얼이면 `panelDual`, 아니면 `panelSingle` | 듀얼일 때 켜짐 | 항상 |
| panel | dock | 8 | `emb:dock` | ▂ | `menu.view.dock` | 도크 켜짐 | 항상 |
| panel | info | 65 | `emb:info-toggle` | ⓘ | 듀얼 정보면 `infoDual`, 아니면 `infoSingle` | 듀얼 정보 | **듀얼 패널 그리고 도크 켜짐** |
| panel | colsync | 63 | `emb:colsync` | ⇔ | `menu.view.colWidthSync` | 켜짐 | **듀얼 패널** |
| panel | ontop | 68 | `emb:always-on-top` | 📌 | `menu.view.alwaysOnTop` | 켜짐 | 항상 |
| view | tree / flat / tiles | 13 / 14 / 15 | `emb:view-tree` / `view-flat` / `view-tiles` | ├─ / ☰ / ▦ | 각 `modeTree/Flat/Tiles` | 현재 모드 | 항상 |
| show | hidden | 10 | `emb:hidden` | 👁 | `menu.view.hidden — 범위` | 켜짐 | 항상 |
| show | dot | 11 | `emb:dotfiles` | … | `menu.view.dot — 범위` | 켜짐 | 항상 |
| show | foldersfirst | 67 | `emb:folders-first` | ▲ | `pref.sortFoldersFirst — 범위` | 켜짐 | 항상 |
| settings | — | 60 | `emb:settings` | U+E713 | `menu.file.prefs`에서 끝의 `.`/`…` 제거 | — | 항상 |

근거: win.rs:691-760. 숨김 규칙: 블록 숨김 = 통째 제외, 항목 표시 해제 = 그 버튼만 제외, 블록의 버튼이 하나도 안 남으면 구분선도 제거(win.rs:762-790).

퀵 런처 바: 항목 순서대로 정사각 아이콘 버튼, `-` 항목 = 구분선(win.rs:800-813).

### 2-5. 컨텍스트 메뉴 고유 항목

- 행 메뉴(`row` 블록 기본 순서): 새로 만들기 ▸ / 완전 삭제 / 이름 복사 / 폴더에 붙여넣기(`nexa-dir2/crates/nexa-app/src/config.rs:1078-1082`). 이름 복사는 설정 순서와 무관하게 "경로 복사" 바로 아래(win.rs:2996-3003).
- 배경 메뉴(`bg` 블록): 붙여넣기 / 실행 취소(대상 설명 포함) / 다시 실행(win.rs:2846-2874).
- 내 PC 배경: WINA-092 참조.
- 셸이 주는 항목 가운데 앱이 가로채는 동사: 행 = delete·rename·copy·cut(win.rs:3020-3023), 배경 = paste(win.rs:2880).

### 2-6. 터미널 그리기 수치(win.rs:2550-2735)

- 셀 폭 = 터미널 폰트 실측, 셀 높이 = `max(12, font_px × 4/3 × dpi/96)`.
- 내용 여백: 좌 2px, 상 1px. 가시 열 = `(w − 4) / 셀폭`, 행 = `(h − 2) / 셀높이`. 열·행이 2 미만이면 그리지 않음.
- 줄 바꿈 켬 = 가시 열 수, 끔 = 설정 열 수(80~1000 클램프) + 가로 스크롤.
- 같은 (전경, 배경) 연속 구간 단위로 배경을 채우고, **문자는 셀마다 개별 배치**(전각은 2셀 클립).
- 선택 = 전경/배경 반전. 단 밝은 팔레트의 기본색 셀은 accent 배경 + 팔레트 배경색 글자.
- faint = 전경을 배경 쪽으로 절반 블렌드.
- 캐럿 = 세로바(폭 `max(1, dpi/96)`, 높이 셀높이−2, 색 = 팔레트 기본 전경). 스크롤백을 보는 중이거나 깜빡임 오프 프레임이면 생략.
- 종료 시 = 맨 아래 줄에 `term.exited`(accent 색). 시작 실패 = 첫 줄에 `term.fail`(흐린 색).
- 마지막에 고속 스크롤 ×N 배지.

### 2-7. 타이틀 형식

`Nexa Dir — [{좌|우}] {경로 표시} [{항목 수}{ · 선택 수} · {탭 n/m} · {평균 µs}{ · 첫 렌더 ms}]{메모}`(win.rs:1964-1979). 경로 표시: 내 PC = `nav.mypc`, 클라우드 = 연결 라벨 경로, 그 외 = 실제 경로.

---

## 3. nexa-ui 매핑

확인 방법: `nexa-ui/crates/nexa-ctl/src` Grep. "없음"은 해당 이름·기능이 Grep에 잡히지 않았다는 뜻이다.

| dir2(이 구간에서 쓰는 것) | nexa-ui 대응 | 판정 | 필요한 추가/주의 |
|---|---|---|---|
| `nexa_gui::widgets::MenuBar` + `Menu` + `MenuItem`(u32 id · 단축키 문자열 · `.checked()` · `.with_submenu()` · `set_menus` · `set_checked` · `take_command`) | `nexa_ctl::MenuBar` + `MenuDef` + `MenuEntry`(`Item/Emph/Disabled/Separator/Sub`) — `nexa-ui/crates/nexa-ctl/src/controls/pulldown.rs:25-152` | **부분** | `MenuEntry`의 항목은 `ComboItem`(값·라벨·아이콘뿐 — `controls/combo.rs:44-53`)이라 **체크/라디오 표시와 단축키 열이 없다**(pulldown.rs에 shortcut·checked·mark 0건). 추가 필요: 체크 표시 + 오른쪽 정렬 단축키 문구. `CtxItem`에는 이미 `shortcut/checked/mark/children`이 있으므로(`controls/ctxmenu.rs:101-131`) 드롭다운을 그쪽 모델로 맞추는 방법도 있다. id는 문자열 → 명령 번호 매핑 계층 필요. 하위 메뉴는 1단계만 지원(dir2 클라우드 메뉴도 1단계라 충분) |
| `nexa_gui::widgets::Toolbar` + `ToolButton`(u32 id · 글리프 · `.with_icon(key, hint)` · `.with_tip()` · `.toggled()` · `.enable()` · `sep()` · `set_buttons` · `hover_tip`) | `nexa_ctl::Toolbar` + `ToolItem` + `ToolIcon`(`Glyph/Image/Mask`) — `controls/toolbar.rs:24-230`, `tip()`·`disabled()`·`separator()`·`set_item_enabled`·`set_icon_size`·`paint_tooltip` | **부분** | **토글(켜짐) 상태가 없다**(toolbar.rs에 toggled 0건). `ToolTone::Accent` + `set_item_tone`으로 "켜짐 = accent 아이콘"은 표현 가능하나, 켜짐 배경까지 필요한지는 `chrome.rs` 담당 문서와 맞춰 결정. `ToolIcon::Mask`의 알파가 `&'static [u8]`라 **실행 중 래스터한 SVG 마스크를 못 넣는다** → 소유형 마스크 변형 추가 또는 빌드 시 생성. 바 높이 28/24 · 아이콘 20/16을 재현하려면 `set_icon_size` + `set_padding` 조합 확인 필요 |
| 도구 모음 순서/표시 편집(`toolbar_order` 문자열) | `nexa_ctl::ToolDock` + `ToolGroup` + `DockLayout::parse/serialize` — `controls/tooldock.rs:35-171` | 참고 | 직렬화 문법이 다르다. dir2 문법(`블록:vis[자식:vis,…]|…`)과 설정 호환을 유지하려면 dir2 파서를 그대로 옮기고 `Toolbar`에 결과만 넣는다 |
| `nexa_gui::widgets::StatusBar`(`new` · `set_text`) | **없음**(nexa-ui 전체에서 statusbar 0건) | **추가 필요** | `StatusBar` 컨트롤: 구획 텍스트 설정, 변경 시에만 무효화, `FontSlot::Status` 사용, 높이 22 |
| 수동 스플리터(비율 · 두께 3 · 히트 반폭 3 · 스냅 20) | `nexa_ctl::controls::splitter::Splitter`(`SplitAxis` · `SplitEvent` · `tick` · `paint`) — `controls/splitter.rs:18-136` | 있음 | 자석 스냅(창 50%·반대편 구분선, Alt 해제)과 최소 폭 클램프는 호스트 로직으로 유지. 파일/도크/가로 3개 인스턴스 |
| 툴팁 팝업 창(`tip.rs` — 별도 HWND, 250ms 틱 2회 후 표시) | `nexa_ctl::draw::draw_tooltip` / `draw_tooltip_in` — `draw.rs:227-240`, `Toolbar::paint_tooltip` | 있음(창 안 오버레이) | 창 밖으로는 못 나간다 → 창 가장자리 클램프로 대체. 지연 표시(500ms)는 호스트 타이머 |
| 네이티브 팝업 메뉴(`TrackPopupMenuEx` — 내 PC 클라우드 메뉴, 탭 메뉴, 편집 팝업) | `nexa_ctl::controls::ctxmenu::ContextMenu` + `CtxItem`(활성/비활성 · `with_checked` · `with_mark` · `submenu` · `with_shortcut`) — `controls/ctxmenu.rs:101-272` | 있음 | 모달 펌프가 없어진다 → "스냅숏 → 표시 → 결과 반영" 구조는 유지하되 결과가 이벤트 루프 다음 턴에 온다 |
| 셸 컨텍스트 메뉴(`shellmenu` + `menuthread`) | `ContextMenu`로 자체 구성 | **OS 분기**(§4) | 고유 항목·가로채기 결과 처리(WINA-089)는 그대로 재사용 |
| `nexa_gui::Column`(`new(key, title, width)` · `right_aligned`) | `nexa_ctl::GridColumn` / `TreeGrid` — `controls/tree.rs:645-675` | 있음(상세는 rows 담당 문서) | 컬럼 키 5종·기본 폭 유지 |
| `nexa_gui::Invalidations`(`push` · `drain` · **`request_tick`/`take_tick`**) | `nexa_ctl::Invalidations`(`push` · `is_empty` · `drain`) — `widget.rs:14-41` | **부분** | 틱 요청이 없다. nexa-ctl은 컨트롤별 `tick(now_ms) -> bool` 방식(`Splitter::tick`, `ScrollBars::tick`) → 호스트가 애니메이션 중인 컨트롤을 물어 타이머를 돌리는 방식으로 바꾼다 |
| `nexa_gui::WheelAccum` | `nexa_ctl::WheelAccum` — `event.rs:120-126` | 있음 | 전역 "휠 줄 수"(`set_wheel_lines`)는 nexa-ctl에 없음 → 호스트가 보관해 `add(delta, lines)`에 넘긴다 |
| `nexa_gui::fastscroll::{FastScroll, set_fast_scroll, FastScroller, set_fast_scroll_grid}` | `nexa_ctl::{FastScroll, set_fast_scroll, ScrollAccel, SpeedHud}` — `controls/scroll.rs:56-320` | **부분** | `FastScroller`(휠+키 가속과 배지를 묶은 래퍼)와 그리드 전용 설정(`set_fast_scroll_grid`)이 없음 → 추가하거나 `ScrollAccel + SpeedHud` 조합 + `ScrollBars::set_fast_override`로 구성 |
| `nexa_gui::Theme`(`dark()`/`light()` · `text_dim` · `panel_bg` · `accent` · `is_dark`) | `nexa_ctl::Theme` — `theme.rs:14-56`(같은 이름 필드 존재) | 있음 | 그대로 |
| `DwCtx`(DirectWrite 백버퍼: `fill_rect` · `term_cell_w` · `term_text` · `push_clip`/`pop_clip`) | `nexa_ctl::DrawCtx`(`fill_rect` · `text_opaque` · `text` · `text_width` · `FontSlot::Mono`) + `RasterCtx` — `draw.rs:16-218`, `raster.rs` | **부분** | `push_clip`/`pop_clip`이 없다(텍스트는 호출마다 clip rect를 받음) — 터미널 부분 행 스크롤에서 위로 번지는 글리프 처리 방식 확인 필요. 터미널 전용 폰트·크기 슬롯(`Mono`는 크기를 Base와 공유)과 셀 폭 조회 API 필요 |
| 도크 위젯(`dock.set_kinds` · `set_content(key, lines)` · `set_image` · `set_popout` · `active_kind`) | **없음**(`ToolDock`은 도구 모음 도킹이라 다른 것) | **추가 필요** | 종류 탭 + 텍스트 줄 뷰(스크롤·선택·키 유지) + 이미지 + ↗ 오버레이. 상세 API는 `dock.rs` 담당 문서 |
| 터미널 그리드 그리기(`term_paint`) | **없음**(nexa-ui에 VT/터미널 0건) | **추가 필요** | `TermView`: 셀 그리드·선택·캐럿·부분 스크롤. VT 파서(`nexa-term`)는 중립이라 그대로 사용 |
| 셸 아이콘 캐시(`icons::shell::ShellIcons` — 런처·파일 아이콘) | `nexa_fs::shell::{IconService, icon_for_path, icon_for_kind}` — `nexa-ui/crates/nexa-fs/src/shell.rs:35-108` | 있음 | 런처 exe 아이콘 = `icon_for_path(path, false)` → `ToolIcon::Image` |
| 진행 창·확인창·설정 창·미리보기 창(별도 HWND) | nexa-sql 방식의 별도 창(`*_win.rs`) + nexa-ctl 컨트롤 | 각 담당 문서 | 이 구간은 호출만 한다 |
| UIA 프로바이더(`uia.rs`) | 없음 | W | §4 |

---

## 4. OS 분기점

| # | 항목 | Windows 현 구현 | macOS | Linux | 관련 ID |
|---|---|---|---|---|---|
| 1 | 실행 형태 | `windows_subsystem="windows"` | `.app` 번들 | 일반 실행 파일 + `.desktop` | WINA-001 |
| 2 | 이벤트 루프·창 | `RegisterClassW`/`CreateWindowExW`/`GetMessageW`, 창 userdata에 `Box<State>` | winit + softbuffer(nexa-sql 기준). 상태는 앱 구조체가 소유 | 동일 | WINA-057·061·062 |
| 3 | 더블클릭 | `CS_DBLCLKS` + `GetDoubleClickTime` | winit에 더블클릭 이벤트 없음 → 자체 판정. 간격 = `NSEvent.doubleClickInterval` | 자체 판정. 기본 400ms(데스크톱 설정 조회는 선택) | WINA-057, 타이머 4 |
| 4 | DPI | `SetProcessDpiAwarenessContext` + dpi/96 정수 환산 | winit `scale_factor` + `ScaleFactorChanged`(`nexa-sql/crates/nexa-sql/src/app/event_loop.rs:668`) | 동일 | WINA-056 |
| 5 | 타이머 | `SetTimer`/`KillTimer` 16종 | 이벤트 루프 `WaitUntil` 기반 자체 타이머 테이블 | 동일 | WINA-004 |
| 6 | 워커→UI 통지 | `PostMessageW(WM_APP+n)` + Box 원시 포인터 | winit `EventLoopProxy`에 타입 있는 enum 전달(원시 포인터 제거) | 동일 | WINA-005 |
| 7 | OS 테마 판정 | 레지스트리 `AppsUseLightTheme` | `defaults read -g AppleInterfaceStyle`, 변경 = winit `ThemeChanged` | `gsettings get org.gnome.desktop.interface color-scheme` | WINA-010 — nexa-sql 구현 재사용: `nexa-sql/crates/nexa-sql/src/theme.rs:7-44` |
| 8 | 타이틀바 테마 | DWM 속성 | winit `Window::set_theme`(`nexa-sql/crates/nexa-sql/src/app/settings.rs:957`) | 동일(지원 범위는 창 관리자 의존) | WINA-014 |
| 9 | OS 언어 | `GetUserDefaultLocaleName` | `CFLocaleCopyPreferredLanguages` | `LANGUAGE` → `LC_ALL` → `LC_MESSAGES` → `LANG` | WINA-012 — `nexa-sql/crates/nsql-i18n/src/syslang.rs:7-10` |
| 10 | 항상 맨 위 | `SetWindowPos(HWND_TOPMOST)` | winit `WindowLevel::AlwaysOnTop`(`nexa-sql/crates/nexa-sql/src/app/windows.rs:141`) | 동일(Wayland는 미지원 가능 — 추정) | WINA-013 |
| 11 | 창 아이콘 | 클래스 아이콘 32 + `WM_SETICON` 16 | Dock 아이콘(`nexa-sql/crates/nexa-sql/src/icon.rs:188`) | winit 창 아이콘(`icon.rs:158`) | WINA-057 |
| 12 | 창 제목 | `SetWindowTextW` | winit `set_title` | 동일 | WINA-066 |
| 13 | 타임존 | `GetTimeZoneInformation` | `nexa_fs::local_time`(`nexa-ui/crates/nexa-fs/src/lib.rs:758`) 또는 `localtime_r`의 `tm_gmtoff` | 동일 | WINA-041 |
| 14 | 시작 경로·루트 | `%USERPROFILE%` → `C:\`, 드라이브 순회 | `nexa_fs::home_dir`(`lib.rs:368`) → `/`, `/Volumes` | 홈 → `/`, 마운트 지점 | WINA-042·043 |
| 15 | 데이터 폴더 | exe 옆 `data\`, 쓰기 불가면 `%LOCALAPPDATA%\NexaDir\data`(`nexa-dir2/crates/nexa-app/src/config.rs:362`) | `nexa_conf::user_config_dir`(`nexa-ui/crates/nexa-conf/src/lib.rs:314`) — nexa-sql 설정 구조 차용 | 동일 | WINA-044·045 |
| 16 | 메모리 반납 | `SetProcessWorkingSetSize(-1,-1)` | `malloc_zone_pressure_relief` | `malloc_trim(0)` | WINA-068 — **기준 충돌**: nexa-sql은 작업집합 트림을 일부러 하지 않고 힙 정리만 한다(`nexa-sql/crates/nexa-sql/src/memtrim.rs:5-7`). 캐시 해제(백버퍼·아이콘)는 공통 유지, 반납 방식은 결정 필요 |
| 17 | 휠 줄 수 | 시스템 설정 조회 | 고정 3 + winit 픽셀 델타(트랙패드) | 고정 3 | WINA-046 |
| 18 | 캐럿 깜빡임 | `GetCaretBlinkTime`(구간 밖) | 기본 530ms 고정(추정) | 고정 또는 `gsettings cursor-blink-time` | 타이머 6 |
| 19 | 대기 커서 | `SetCursor(IDC_WAIT)` | winit `set_cursor(Wait)` | 동일 | WINA-072·095 |
| 20 | 수식키 상태 | `GetKeyState(VK_SHIFT)` 즉시 조회 | winit `ModifiersChanged`로 추적한 상태 사용 | 동일 | WINA-086·087 |
| 21 | 터미널 PTY | ConPTY + pwsh | `openpty`/`forkpty` + `$SHELL`(없으면 `/bin/zsh`), 크기 = `TIOCSWINSZ`, `TERM=xterm-256color` | 동일(없으면 `/bin/sh`) | WINA-033·081 |
| 22 | 셸 컨텍스트 메뉴 | `IContextMenu` 호스팅(행·배경) + 전용 메뉴 스레드 선행 구축 | 공개 API 없음 → 자체 메뉴: 열기 / 연결 프로그램(`NSWorkspace`) / Finder에서 보기 / 정보 가져오기 / 훑어보기 + 앱 고유 항목 | 자체 메뉴: 열기(`xdg-open`) / 연결 프로그램(`.desktop` + `mimeinfo.cache`) / 파일 관리자에서 보기·속성(`org.freedesktop.FileManager1`의 `ShowItems`/`ShowItemProperties`) / 새로 만들기(`XDG_TEMPLATES_DIR`) | WINA-086~090 |
| 23 | 새로 만들기 하위 메뉴 | 셸 New 확장 호스팅 + 생성 감지 | 자체 구성(새 폴더·빈 파일) | 템플릿 폴더 | WINA-086·087 |
| 24 | 휴지통 | `SHFileOperationW`, 복원 = 셸 undelete | `NSFileManager trashItemAtURL:resultingItemURL:`. 복원 API 없음 → 반환 URL을 기억해 되옮김 | FreeDesktop Trash 규격(`$XDG_DATA_HOME/Trash/{files,info}` + `.trashinfo`, 다른 볼륨은 `$topdir/.Trash-$uid`). 복원 = `.trashinfo`의 `Path=` | WINA-091 |
| 25 | 파일 클립보드 | `CF_HDROP` + Preferred DropEffect, 변경 구독 `AddClipboardFormatListener` | `NSPasteboard` 파일 URL. 잘라내기는 OS 개념 없음 → 앱 내부 플래그. 변경 감지 = `changeCount` 폴링 | X11 `x-special/gnome-copied-files`(`copy`/`cut` + URI 목록) · `text/uri-list` · KDE `application/x-kde-cutselection`. 변경 감지 = XFixes(X11) | WINA-060·070·089 — nexa-sql에 텍스트 클립보드 구현 있음(`nexa-sql/crates/nexa-sql/src/clipboard.rs`, `clipboard_x11.rs`), 파일 목록 형식은 신규 |
| 26 | 가상 파일(스트림) | FILEDESCRIPTOR/FILECONTENTS(클립보드·드롭) | 파일 프라미스(`NSFilePromiseReceiver`) — 1차 범위 밖 권장 | 대응 규격 없음 → 비활성 | WINA-072·095 |
| 27 | DnD 수신·발신 | OLE `IDropTarget`/`IDropSource` | winit은 `DroppedFile`/`HoveredFile`만 제공(드래그 중 좌표·발신 없음 — `nexa-sql/crates/nexa-sql/src/app/event_loop.rs:598`) → `NSDraggingDestination`/`NSDraggingSource` 직접 구현 | XDND(X11) / `wl_data_device`(Wayland) 직접 구현 | WINA-058·093~096 |
| 28 | 접근성 | UI Automation | `NSAccessibility` — 후속 | AT-SPI2 — 후속 | WINA-069 |
| 29 | 압축 이름 디코더 | CP949 등 레거시 코드페이지(추정: OS API) | 자체 디코드 테이블 필요 | 동일 | WINA-048 |
| 30 | 런처 시드 | VS Code · pwsh · cmd | 터미널.app · VS Code 등 OS별 목록 | 터미널 에뮬레이터 등 | WINA-054 |
| 31 | 경로 텍스트 줄바꿈 | 경로/이름 복사 구분자 `\r\n` | `\n` | `\n` | WINA-089 |
| 32 | 임시 폴더 | `std::env::temp_dir()/NexaDir/…` | 동일(중립) | 동일 | WINA-073·075 |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. `State` 필드 묶음(win.rs:850-1070)

| 묶음 | 필드 | 영속 여부 |
|---|---|---|
| 크롬 위젯 | `menubar` · `toolbar` · `launcherbar` · `statusbar` | 아니오 |
| 런처 | `launcher_visible` · `launcher_items` | 설정 |
| 보기 모드 | `view_mode` · `panel_mode` · `info_mode`(선호값 — 효과는 싱글 패널이면 싱글 고정) | 설정 |
| 클라우드 | `cloud_conns` · `cloud_cands`(메뉴 구성 시점 스냅숏) · `cloud_client_ids` · `cloud_client_secrets` · `cloud_progress` · `cloud_shared` · `vpaste_roots` | 앞 4개 중 conns·ids·secrets는 설정 |
| 폰트 | `base_font(+size)` · `ctx_font(+size)` · `status_font(+size)` · `list_font(+size)` · `list_folder_bold` · `header_bold` · `header_italic` · `dlg_font` · `term_font(+size)` | 설정 |
| 컬럼 | `col_width_sync` · `col_autofit_max` · `pending_colw[2]` · `pending_cols[2]`(세션 복원값을 DPI 반영 뒤 적용하려는 보류 슬롯) | 설정 / 세션 |
| 패널 | `panels[2]` · `active` · `split` · `split_drag` | split = 설정, 탭 등은 세션 |
| 테마·그리기 | `theme` · `theme_mode` · `dpi` · `dw` · `icons` · `stats` · `tz` | theme_mode = 설정 |
| 도크 정보 워커 | `info_worker` · `info_details[2]` · `info_req[2]` · `info_gen` | 아니오 |
| 메뉴 스레드 | `menu_thread` · `ctx_gen` · `ctx_prepared_key` · `ctx_showing` | 아니오 |
| 보기 옵션 미러 | `show_hidden` · `show_dotfiles` · `sort_folders_first`(값의 원본은 **탭** — 여기는 활성 탭 사본) · `view_scope` · `hide_empty_glyph` · `always_on_top` · `sort_case_sensitive` · `nav_up_align` · `tab_dblclick` | 설정 |
| 미리보기·플러그인 | `preview_map`(`ext:id` 목록) · `plugins_disabled`(`id` 목록) | 설정 |
| 타입어헤드 | `ta_scope` · `ta_reset_ms` · `ta_pos` · `ta_special` · `ta_space` · `ta_backspace` | 설정 |
| 고속 스크롤 | `fast` · `fast_grid_extra` | 설정 |
| 언어 | `lang_setting` · `langs` | lang = 설정 |
| 터미널 | `term_wrap` · `term_cols` · `term_theme` · `term_theme_dark` · `term_theme_light` · `term_copy_format` · `terms[2]` · `term_gen` · `term_focus` · `term_drag` · `term_mouse_btn` · `term_caret_on` | 앞 6개 설정 |
| 순서 문자열 | `toolbar_order` · `ctx_menu_order` | 설정 |
| 툴팁 | `tip_win` · `tip_armed` | 아니오 |
| 상주 관리 | `last_activity_ms` · `trimmed` | 아니오 |
| 접근성 | `uia_caret` · `uia_struct` | 아니오 |
| 전송·삭제 | `transfer` · `transfer_close` · `transfer_gen` · `transfer_close_ms`(0 = 진행 창 미표시) · `pending_delete` | close_ms = 설정 |
| 마우스 과도 상태 | `drag_press` · `dnd_hover` · `dnd_hover_ms` · `slow_click` · `rename_on_up` · `pending_rename` · `tab_drag_undo` · `rbutton_down_seen` · `rclick_began_edit` | hover_ms = 설정 |
| 폴더 감시 | `watchers[2]` · `watch_gen` · `watch_since[2]` · `probe[2]` · `shell_watch[2]` · `sub_probe[2]` | 아니오 |
| 도크 드래그 | `dock_drag` · `dock_split_drag` · `dock_split` | dock_split = 설정 |
| 이력 | `history`(undo/redo — 세션 한정) | 아니오 |

### 5-2. 이 구간이 읽는 설정·세션 필드

- 파일: `data\settings.cfg` · `data\session.cfg`(구 `.txt`에서 1회 마이그레이션) · `data\crash.txt`(`nexa-dir2/crates/nexa-app/src/config.rs:1016-1024`, win.rs:1411-1419). 키 이름·직렬화 형식은 `config.rs` 담당 문서가 원본이다.
- `Settings` 필드(win.rs:1420-1712에서 참조): theme · lang · show_hidden · show_dotfiles · hide_empty_glyph · cloud_conns · cloud_client_ids · cloud_client_secrets · dock · dock_ratio · dock_split · split · sort_folders_first · sort_case_sensitive · nav_up_align · list_folder_bold · header_bold · header_italic · view_mode · panel_mode · info_mode · view_scope · col_width_sync · col_autofit_max · typeahead_scope · typeahead_reset_ms · typeahead_special · typeahead_space · typeahead_backspace · typeahead_pos · launcher · launcher_items · launcher_seed · always_on_top · toolbar_order · ctx_menu_order · base_font(+size) · ctx_font(+size) · status_font(+size) · list_font(+size) · dlg_font(+size) · term_font(+size) · term_wrap · term_cols · term_theme · term_theme_dark · term_theme_light · term_copy_format · tab_dblclick · preview_map · plugins_disabled · fast_scroll() · fast_scroll_grid() · fast_scroll_grid_extra · transfer_close_ms · dnd_hover_ms.
- `Session` 필드: `active_panel`, 패널별 `tabs` · `active` · `expanded` · `locked` · `pinned` · `modes` · `views` · `col_widths` · `col_layout`(win.rs:1455-1484, win.rs:1526-1531, win.rs:1621-1682).
- 순서 문자열 문법: `블록:vis[자식:vis,…]|블록:vis|…`(`nexa-dir2/crates/nexa-app/src/config.rs:1084-1092`).

이식 방침: 설정 **구조·저장 방식**은 nexa-sql(레지스트리형 설정, `nexa_conf::Store`·`SaveScheduler` — `nexa-ui/crates/nexa-conf/src/lib.rs:174-272`)을 따르고, 위 **필드 목록과 기본값**은 dir2를 계승한다.

### 5-3. 타이머(win.rs:66-132)

| ID | 이름 | 주기 | 용도 |
|---|---|---|---|
| 1 | TYPEAHEAD | 250ms | 타입어헤드 버퍼 타임아웃 점검 |
| 2 | ICONS | `icons::shell::TICK_MS` | 셸 아이콘 로딩 큐(속도 제한) |
| 3 | JANITOR | 10초 | 유휴 60초 트림 점검. 트림 후 스스로 꺼짐 |
| 4 | RENAME | 더블클릭 시간 | 느린 재클릭 리네임 지연(두 번째 클릭이 오면 취소) |
| 5 | TERM_SEL | 60ms | 터미널 선택 엣지 자동 스크롤 |
| 6 | TERM_CARET | 캐럿 깜빡임 시간 | 터미널 캐럿 토글 |
| 7 | PROG_CLOSE | `transfer_close_ms` | 전송 완료 진행 창 자동 닫기 |
| 8 | SESSION_SAVE | 1초 디바운스 | 세션 자동 저장 |
| 9 | TIP | 250ms × 2틱 | 도구 모음 툴팁 표시·이탈 감지 |
| 10, 11 | WATCH_BASE+패널 | 300ms 디바운스, **연장 상한 1초** | 폴더 변경 통지 코얼레싱 |
| 12 | DND | 100ms | 드래그 중 엣지 스크롤·호버 대기 |
| 13 | CLOUD_POLL | 200ms | 클라우드 전송 취소 버튼 폴링 |
| 14 | FSPOLL | 활성 3초 / 비활성(보이는 창) 30초 / 최소화 시 정지 | 변경 통지가 오지 않는 경로의 프로브 |
| 15 | WIDGET_TICK | 40ms | 위젯 틱 요청(스크롤바 페이드 등). 요청 없으면 해제 |
| 16 | CTX_PREBUILD | 300ms | 컨텍스트 메뉴 선행 구축 |

### 5-4. 워커→UI 통지(win.rs:71-184)

| 값 | 이름 | 내용 |
|---|---|---|
| 0x8001 | TRANSFER | 전송 잡: w=세대, l=0 진행 / 1 완료 |
| 0x8002 | FSCHANGE | 폴더 변경: w=패널, l=세대 |
| 0x8003 | TERM | 터미널 출력/종료: w=패널(종료 플래그 포함), l=세대 |
| 0x8004 | ICON | 파일별 아이콘 결과(Box) |
| 0x8005 | PREFS | 설정 창 열기 지연 실행 |
| 0x8006 / 0x8007 | (prefs 적용 / UIA 선택 — 다른 모듈 정의) | win.rs:147 주석 |
| 0x8008 | BULK | 일괄 이름변경 창 지연 실행 |
| 0x8009 | CTLDEMO | 컨트롤 갤러리(개발용) |
| 0x800A / 0x800B | EDIT_TOOLBAR / EDIT_COLS | 순서 편집 창 지연 실행 |
| 0x800C | ABOUT | About 창 지연 실행 |
| 0x800D | DELETE | 휴지통 삭제 워커 완료: w=성공 여부 |
| 0x800E | CLOUD_AUTH | OAuth 워커 완료(Box) |
| 0x800F | CLOUD_LIST | 클라우드 목록 적재 완료(Box) |
| 0x8010 | CLOUD_DOWNLOAD | 다운로드 완료(Box) |
| 0x8011 | CLOUD_WRITE | 쓰기 완료(Box) |
| 0x8012 | CLOUD_PROGRESS | 진행 틱(페이로드 없음, 단발) |
| 0x8013 | VPASTE | 가상 붙여넣기 완료: w=전건 성공 여부 |
| 0x8014 / 0x8015 | SHCHANGE_BASE+패널 | 셸 변경 통지 |
| 0x8016 | INFO_DETAILS | 파일 상세 워커 완료: w=패널, l=Box(세대, 경로, 줄) |
| 0x8017 | CTXMENU_RESULT | 메뉴 스레드 결과: w=세대, l=Box(요청, 결과) |

### 5-5. 스레딩 규약

- UI 스레드만 `State`를 만진다. 워커는 공유 원자값/뮤텍스(`TransferShared`)와 통지로만 소통(win.rs:1197).
- **세대 번호 가드**: 전송(`transfer_gen`) · 폴더 감시(`watch_gen`) · 터미널(`term_gen`) · 상세 정보(`info_gen`) · 컨텍스트 메뉴(`ctx_gen`) — 낡은 워커의 늦은 통지는 버린다.
- **동시 1개 규약**: 전송 잡 1개, 휴지통 삭제 1잡, 클라우드 전송과 가상 붙여넣기는 진행 창·취소 슬롯을 공유(win.rs:877-879, win.rs:999-1008).
- **모달 재진입 규약**: 모달 창·팝업은 `State` 참조가 끝난 뒤에 연다 — 지연 실행 통지(PREFS·BULK·ABOUT·EDIT_*)와 "스냅숏 추출 → 표시 → 상태 재획득 후 반영" 2단계(win.rs:137-152, win.rs:2827, win.rs:3053, win.rs:3210).
- 기동 순서 의존: i18n → 클라우드 루트 → 패널 복원 → 메뉴/도구 모음 구성 → 창 생성 → (창 생성 통지에서 DPI 반영·보류 컬럼 적용 — 구간 밖) → DnD 등록 → 항상 위 → 클립보드 구독(win.rs:1426-1794).

---

## 6. 이식 시 주의 — 회귀 방지에 필요한 실측 교훈

| # | 교훈 | 근거 |
|---|---|---|
| 1 | 디바운스 타이머를 같은 ID로 재무장하면 만료가 계속 밀린다 — 동기화 클라이언트가 변경을 쏟는 동안 목록이 멈췄다. **첫 통지 후 1초를 넘기면 더 연장하지 않는다** | win.rs:108-112, win.rs:1029-1031 |
| 2 | 폴더 프로브 폴링은 창이 보이는 한 비활성이어도 유지(감속), 최소화 때만 정지 | win.rs:113-132 |
| 3 | 플레이스홀더 파일 생성은 부모 수정 시각도 변경 통지도 바꾸지 않는다 → 보이는 폴더는 **열거 서명** 비교가 필요 | win.rs:1038-1042 |
| 4 | 취소 버튼 판정을 진행 통지 안에만 두면 통지가 멎는 구간에서 취소가 안 먹는다 → 통지와 무관하게 200ms 폴링 | win.rs:102-106 |
| 5 | 종결 통지는 재시도 게시, 진행 통지는 단발(유실돼도 다음 틱이 덮음) | win.rs:165, win.rs:196-198 |
| 6 | 휴지통 삭제를 UI 스레드에서 하면 3~4초 멈춘다 → 워커 | win.rs:153-155 |
| 7 | 릴리스가 `panic=abort`면 어느 스레드든 무통보로 죽는다 → 후크로 `crash.txt` 기록 | win.rs:1404-1407 |
| 8 | 클라우드 루트 동기는 패널 복원보다 먼저 — 아니면 기동 직후 클라우드 행이 빠지고 F5를 눌러야 나온다 | win.rs:1443-1446 |
| 9 | 클라우드 재로드·배지 판정은 내 PC(가상 최상위)도 포함해야 한다 — 빼면 "불러오는 중"이 남는다 | win.rs:577-579 |
| 10 | 클라우드 로딩 중 빈 목록은 동작 여부를 알 수 없다 → 플레이스홀더 행 1개, 대상 = 현재 폴더 자신 | win.rs:642-644 |
| 11 | 시작 폴백을 `C:\` 고정으로 두면 C:가 없거나 잠긴 시스템에서 기동이 중단된다 | win.rs:1393-1395 |
| 12 | 정보 라디오 체크와 도구 모음 정보 버튼은 **효과 기준**(싱글 패널 = 싱글 정보 고정) | win.rs:402-403, win.rs:1572, win.rs:1590 |
| 13 | 숨김·dot·폴더 우선 값의 원본은 탭이다. `State`의 값은 활성 탭 사본 | win.rs:927-932 |
| 14 | 세션 컬럼 폭·레이아웃은 DPI 지표 반영(기본 폭 리셋) **뒤에** 적용해야 덮이지 않는다 | win.rs:894-896, win.rs:985-986 |
| 15 | 싱글 정보의 공유 도크를 눌렀을 때 활성 패널이 뒤집히면 방금 보던 미리보기가 바뀐다 → `SharedDock` 존은 활성 유지 | win.rs:1281-1284 |
| 16 | 리네임 지연 타이머 만료 시 **현재 활성 패널·캐럿 행이 예약 대상과 같은지** 대조 — 아니면 다른 행에 편집 필드가 뜬다 | win.rs:1019-1023 |
| 17 | 우클릭은 누름과 뗌이 짝일 때만 메뉴를 연다(다른 창이 누름을 삼키고 뗌만 오는 경우 방지) | win.rs:1058-1061 |
| 18 | 경로 바 첫 우클릭은 편집 진입만 — 같은 클릭의 뗌에서 메뉴를 띄우지 않는다 | win.rs:1062-1065 |
| 19 | 트랙패드는 노치(120) 미만 델타를 잘게 보낸다 — 정수 나눗셈으로 버리면 무반응, 델타마다 1회 처리하면 과속 → 분수 누적 | win.rs:1085-1089 |
| 20 | 터미널 문자는 셀 x에 개별 배치 — 연속 구간 단위로 그리면 폴백 글꼴 전진폭 때문에 열이 밀린다 | win.rs:2656-2658 |
| 21 | 밝은 팔레트에서 기본색 셀을 단순 반전하면 검은 블록이 된다 → accent 블록 | win.rs:2615-2617 |
| 22 | 터미널 시작 실패 시 키 포커스를 해제하지 않으면 키 입력이 영구히 삼켜진다 | win.rs:2587 |
| 23 | 줄 바꿈 전환·리사이즈 뒤 가로 오프셋을 다시 클램프 | win.rs:2597 |
| 24 | 팔레트 해석은 그리기 시점 — 테마 전환 즉시 스크롤백까지 새 색 | win.rs:2598-2599 |
| 25 | 도크 내용 키 = 종류 + 대상 — 같은 대상 갱신은 스크롤 유지 | win.rs:2516 |
| 26 | 선택 경로 목록은 **삽입 순서**다. 경로 복사·이름 복사는 화면 순서로 다시 모은다 | win.rs:2757-2758 |
| 27 | 복사/잘라내기는 교차 폴더 전체 선택을 대상으로(우클릭 대상은 같은 부모로 축소되지만 클립보드는 전체) | win.rs:3096-3103 |
| 28 | 잘라내기 붙여넣기는 1회성 — 이동이면 붙여넣은 뒤 클립보드를 비운다 | win.rs:2889-2891, win.rs:3129-3131 |
| 29 | 배경 메뉴에 고유 붙여넣기를 둔다(셸이 paste 동사를 안 내는 환경) | win.rs:2852-2853 |
| 30 | 전송 중 외부 드롭은 조용히 버리지 말고 거부 + 안내(발신 쪽이 파일을 되돌릴 수 있게) | win.rs:3271-3272 |
| 31 | 드래그 중 커서가 멈추면 OS의 추적 콜백도 멎는다 → 타이머 폴링으로 호버 대기·연속 스크롤 판정 | win.rs:98-99 |
| 32 | 호버 발동 전에 행-경로 일치 재검증(스크롤·재정렬로 행이 밀림) | win.rs:3393-3394 |
| 33 | 스테이징 정리는 `dnd-*` 아래만, 빈 폴더만 | win.rs:2209-2210, win.rs:2233-2234 |
| 34 | 메뉴 표시와 실행 사이에 감지 후보가 바뀔 수 있다 → 표시 시점 스냅숏으로 인덱스 해석 | win.rs:866-868, win.rs:3213 |
| 35 | 도구 모음 28 / 런처 24, 아이콘 20 / 16은 사용자 확정값 | win.rs:1851-1857, win.rs:797-799 |
| 36 | 접근성 이벤트는 (패널, 캐럿)·(패널, 경로, 행 수) 서명으로 중복 억제, 클라이언트가 붙었을 때만 | win.rs:2074-2099 |
| 37 | win.rs 머리말의 "F3 = 스크롤 벤치"는 낡은 주석 — 실제는 F3 = 미리보기 창, Shift+F3 = 벤치 | win.rs:4, win.rs:8825-8832 |
| 38 | 알 수 없는 테마 문자열 = Dark, OS 테마 조회 실패 = Light(서로 다른 기본값) | win.rs:372, win.rs:302 |

---

## 7. 회귀 테스트 후보

자동화: **단위** = 순수 함수 단위 테스트 / **헤드리스** = 창 없이 상태+가짜 그리기 문맥으로 검증 / **수동** = 실제 OS 상호작용 필요.

| # | 시나리오 | 관련 ID | 자동화 |
|---|---|---|---|
| T1 | 히트 존 표: 듀얼/싱글 정보 × 도크 표시/숨김 × 좌·우·스플리터 좌표 | WINA-038 | 단위(기존 `hit_zone_table` 이식) |
| T2 | 유휴 트림 판정: 59.9초 false / 60초 true / 이미 트림 false | WINA-068 | 단위(기존 테스트 이식) |
| T3 | 스테이징 분리·정리: `dnd-*`만 분리, 빈 폴더만 삭제 | WINA-075 | 단위(기존 2건 이식) |
| T4 | 레이아웃 수치: 1400×800 @96dpi에서 메뉴 22 / 도구 28 / 런처 24·0 / 상태 22, 스플리터 gap 3, 듀얼·싱글 패널, 듀얼·싱글 정보, 도크 밴드 클램프 | WINA-064·065 | 헤드리스(레이아웃을 순수 함수로 분리) |
| T5 | 레이아웃 배율: 144dpi·192dpi에서 같은 식, 창이 400px보다 좁을 때 스플리터 클램프 | WINA-064·065 | 헤드리스 |
| T6 | 메뉴 구성 스냅숏: 5개 메뉴의 항목 순서·명령·단축키 표기·체크 조건(싱글 패널이면 정보 싱글 체크) | WINA-015~021 | 단위(구성 함수는 순수) |
| T7 | 클라우드 메뉴: 연결 0/1/32/33개, 후보 0개(단독 항목 299), API 연결과 폴더 연결의 해제 라벨 | WINA-020 | 단위 |
| T8 | 도구 모음 구성: 기본 순서 13버튼, 블록 숨김·항목 숨김·전부 숨김 시 구분선 제거, `info`/`colsync` 활성 조건 | WINA-028 | 단위 |
| T9 | 런처 바: 구분선 항목, 명령 = 200+idx, 실행 항목 0이면 높이 0 | WINA-029·065 | 단위 |
| T10 | 테마 모드 왕복·알 수 없는 값 = Dark | WINA-009 | 단위 |
| T11 | 터미널 보기 이동: 행/픽셀/가로 스크롤 클램프, 부분 오프셋 스냅, 변화 없으면 false | WINA-034 | 단위 |
| T12 | 터미널 선택·히트: 역방향 드래그 정규화, 범위 밖 좌표 클램프, 부분 오프셋 반영 | WINA-035 | 단위 |
| T13 | 터미널 그리기: 가짜 그리기 문맥으로 셀 개별 배치·전각 2셀·선택 반전·밝은 팔레트 accent·캐럿 생략 조건 검증 | WINA-081 | 헤드리스 |
| T14 | 붙여넣기 대상 폴더: 폴더 1개 / 파일 1개(부모) / 다중 / 없음 | WINA-071 | 헤드리스(임시 폴더) |
| T15 | 조작 대상: 선택 없음 = 캐럿, 표시 순서 수집, 우클릭 대상의 같은 부모 축소 | WINA-082~084 | 헤드리스 |
| T16 | 도크 정보 줄: 다중 선택 / 단일 파일 8줄 순서 / 메타 불가 폴백 / 선택 없음 / 상세 미도착 시 "불러오는 중" | WINA-077 | 헤드리스 |
| T17 | 미리보기 평문 변환: 태그 제거, hr, 인용 접두, 폴더·다중 선택 = "없음" | WINA-078 | 단위 |
| T18 | 도크 갱신: 싱글 정보에서 내용 원천 = 활성 패널, 같은 경로 상세 중복 요청 없음, 세대 증가 | WINA-080 | 헤드리스 |
| T19 | 행 메뉴 요청: 고유 항목 순서·표시(설정 문자열), 새로 만들기는 단일 선택만, 폴더에 붙여넣기 조건, 이름 복사 위치 | WINA-087 | 헤드리스 |
| T20 | 메뉴 결과 반영: 경로/이름 복사 텍스트(표시 순서·줄 구분자), 이동 붙여넣기 후 클립보드 비움 | WINA-089 | 헤드리스(클립보드 가짜 구현) |
| T21 | 드롭 대상 판정: 폴더 행 / 파일 행 / 빈 본문 / 스플리터 | WINA-093 | 헤드리스 |
| T22 | 전송 중 외부 드롭 거부 + 안내 | WINA-094 | 헤드리스 |
| T23 | DnD 호버: 대상 유지 시 대기 경과 후 발동, 대상 변경 시 리셋, 행-경로 불일치 시 무시, 탭 우선 | WINA-096 | 헤드리스(시각 주입) |
| T24 | 기동 순서: 세션에 내 PC 탭 + 클라우드 연결이 있을 때 첫 목록에 클라우드 행 존재 | WINA-049·050 | 헤드리스 |
| T25 | argv 경로 지정 시 세션 무시·좌우 동일 경로·활성 = 좌 | WINA-050 | 헤드리스 |
| T26 | 크래시 로그: panic 시 데이터 폴더에 파일 생성 | WINA-044 | 단위(자식 프로세스) |
| T27 | 타이틀 문자열: 내 PC·클라우드 라벨, 선택 0이면 선택 수 생략 | WINA-066 | 단위 |
| T28 | 클라우드 열거 콜백: 캐시 적중 / 토큰 없음 = 빈 목록 / 미적중 = 플레이스홀더 1행 | WINA-026 | 헤드리스(워커 가짜 구현) |
| T29 | 휴지통 삭제·복원 왕복(OS별) | WINA-091 | 수동 + OS별 통합 테스트 |
| T30 | 터미널 기동: OS 기본 셸이 뜨고 크기 변경이 반영 | WINA-081 | OS별 통합 테스트 |
| T31 | 항상 맨 위·타이틀바 테마·OS 테마 추종·OS 언어 추종 | WINA-010~014 | 수동(판정 함수의 문자열 파싱은 단위) |
| T32 | 외부 앱 ↔ 앱 드래그 앤 드롭, 파일 클립보드 상호운용(복사/잘라내기) | WINA-058·060·093~096 | 수동 |
