# port/00 · 이식 원장 색인

> `docs/port/`는 **nexa-dir2 → nexa-dir3 이식의 원장**이다. 2026-10-03 조사(읽기 전용 · 에이전트 19건)로 작성됐고, 각 문서의 **접두 ID**가 구현·교차 검증의 체크리스트가 된다([90 검증 매트릭스](90-verification-matrix.md)). 근거 표기는 `저장소/경로:줄`, 확인 못 한 것은 "추정".
> 번호 공간은 `docs/NN`과 별개 — 색인·링크에서는 `port/NN`으로 부른다.

## 1. 문서 표

| 문서 | 접두 | 범위 | 항목 수 | 핵심 |
| --- | --- | --- | --- | --- |
| [10 dir2 win.rs A](10-dir2-win-a.md) | WINA | `win.rs` 1~3410 + `main.rs` — 상수·테마·메뉴/툴바/런처 구성·State·기동·레이아웃·도크 내용·터미널 그리기·컨텍스트 메뉴 요청·휴지통·DnD 훅 | 96 | 레이아웃 고정값(메뉴 22/툴바 28/런처 24/상태 22 · 초기 1400×800) · 기동 순서 의존 · 통지 세대 가드 |
| [11 dir2 win.rs B](11-dir2-win-b.md) | WINB | 3250~6557 — DnD 수신·실행 취소·감시/프로브·삭제·이름/새로 만들기·전송·IME·페인트·`finish_input`·`run_command`(60종)·OAuth·툴팁·탭 교차·설정 적용 전반 | 125 | 모달 동기 반환 → 상태 기계 재작성 필요 · 종결 통지 재시도 · 3중 자동 갱신 |
| [12 dir2 win.rs C](12-dir2-win-c.md) | WINC | 6470~9951 — `apply_prefs` 25 · 헬퍼 31 · `wndproc` 111(마우스·키·타이머 16·통지 20+) · 테스트 7 | 174 | 좌클릭 라우팅 8단계 · 단축키 전수 · 설정 61필드 클램프 · OS 분기 35 |
| [13 dir2 패널·파일 목록](13-dir2-panel-filelist.md) | PANEL | `panel.rs` `nav.rs` `source.rs` `rows.rs` `columns.rs` `typeahead.rs` `nexa-tree` | — | 선택 모델·정렬·열·인라인 이름 변경·타입어헤드·픽셀/고속 스크롤·호버 펼침 |
| [14 dir2 nexa-gui 위젯](14-dir2-gui-widgets.md) | GUI | dock·menubar·tabbar·pathbar·chrome·overlaybar·fastscroll·edit·event·geom + pathinput·shellpath·tip | — | nexa-ctl 대응 API 차이 대조 · DrawCtx 어휘 어댑터 |
| [15 dir2 설정·구성](15-dir2-prefs-config.md) | PREFS | `prefs.rs`(설정 창 전 페이지) `config.rs` `i18n.rs` — 키 표(타입·기본값·범위·UI·즉시 반영) · 세션 | — | dir3 제안 키 PREFS-101~170 · 페이지 배치 3xx |
| [16 dir2 앱 컨트롤·대화상자](16-dir2-app-controls-dialogs.md) | DLG | `ctl/` 16종 · ctldemo · dialog · pwprompt · about · ordereditor · bulkrename(UI) | — | 컨트롤별 nexa-ui 대응·없는 기능 |
| [17 dir2 그리기·텍스트](17-dir2-rendering-text.md) | RENDER | dw(DirectWrite)·fontchain·gdipctx·style·draw·theme·icons·svg·build.rs ↔ nexa-gfx/ctl/font | — | DrawCtx 메서드 대조표 · 테마 토큰 대조 · 아이콘 자원·DPI |
| [18 dir2 터미널](18-dir2-terminal.md) | TERM | nexa-term · conpty · launcher · 터미널 관련 코드 · 테마 15종 · 복사 서식 | — | POSIX PTY 이식 방안 · 기본 셸 |
| [19 dir2 셸 통합](19-dir2-shell-integration.md) | SHELL | shellmenu·menuthread·shellnotify·clipboard·dnd·recycle·fileinfo·uia·watcher·fsprobe·secret·core | — | OS별 대응 규격(NSPasteboard·FSEvents·trash · gnome-copied-files·XDND·inotify·freedesktop) · nexa-fs 보유분 |
| [20 dir2 미리보기·플러그인](20-dir2-preview-plugins.md) | PLUG | preview/(mod·wasm·archive) · previewwnd · archivewnd · vfs/archive · samples · build-plugins | — | ABI v1/v2 · 격리 수치 · 탐색 경로 · WIC → nexa-gfx |
| [21 dir2 클라우드](21-dir2-cloud.md) | CLOUD | cloud·cloudfs·oauth · ADR-0006 | — | OAuth 루프백·토큰 보관(DPAPI → Keychain/Secret Service) · HTTP 스택 선택지 |
| [22 dir2 코어·작업 엔진](22-dir2-ops-core.md) | OPS | nexa-ops(전송·히스토리·일괄 이름 변경) · vfs · tree · core · cfg 전수 | — | 플랫폼 중립도 · 스텁 목록 · 테스트 수 |
| [23 dir2 규약·라이선스·배포](23-dir2-process-license-dist.md) | PROC | CLAUDE·15·16·18·21·23·29 · CI · scripts · 설치본 · 패키징 · about | — | 계승 규칙 · 라이선스 정책 P1~P8 · QA 하네스 · docs/23 크로스플랫폼 결론 |
| [30 명령·단축키 카탈로그](30-catalog-commands-shortcuts.md) | CMD | 메뉴 트리 · 명령 ID 전수 · 단축키(문맥별) · 마우스 제스처 · macOS 수정키 대응안 | — | dir3 명령 표 `commands.rs`의 원천 |
| [31 설정·i18n 카탈로그](31-catalog-settings-i18n.md) | KEY | `settings.cfg` 69키 · `session.cfg` · i18n 키 전수(3언어) · 영속 파일 · 환경/인자/레지스트리 · nexa-sql 방식 대응표 | — | 코드 미참조 키 61 · 플랫폼 종속 문구 · 대응표 §5-3 |
| [40 nexa-sql 앱 골격](40-sql-app-skeleton.md) | SKEL | main·app/*·winhost·present·wingeom·winfocus·theme·icon·toast·clipboard(+x11)·ime·input·worker·about_win | 291 | 골격 요소 167 · OS 분기 34 · 함정 50 · dir3 제안 40(모듈 구성·체크리스트·하네스 훅) |
| [41 nexa-sql 설정](41-sql-settings.md) | SET | nsql-settings 전체 · nexa-conf · prefs_win · app/settings · keymap · keys_win | 137 | 복사/표 교체/개작 구분 · 결함 7(SET-130~137) · dir3 크레이트 구성·구현 순서 |
| [42 라이선스](42-sql-license.md) | LIC | nexa-license(lib·tool) · nsql-license · license_win · app/license · dir2 정책 | ~197 | API · 앱 층 · dir2 정책 P1~P8 · 통합 계획 · 발급기 변경 필요분 · OS 분기 · 위험 |
| [43 i18n·확장](43-sql-i18n-extensions.md) | EXT | nsql-i18n · extensions/(manager·wasm·sha256) · ext_panel · SDK · ext-build | — | dir3 i18n 구성안 · 런타임 비교 · ABI 유지 체크리스트 · 플러그인 계획 P1~P5 |
| [44 CI·테스트·문서 운영](44-sql-ci-test-docs.md) | CI | 문서·git 규칙 · 3-OS CI · 패키징 · 하네스 층 · 스크립트 66 | 120 | dir3 하네스 7층 · `--selfcheck` 항목 · 검증 매트릭스 제안 |
| [50 nexa-ui 기반 API](50-ui-core.md) | UIC | nexa-ctl 기반(draw·event·geom·widget·raster·theme·tokens·shape·scroll·splitter) · gfx · sys · conf | ~323 | 위젯 계약 · 새 컨트롤 추가 절차 · 갭 UIC-310~323 |
| [51 nexa-ui 컨트롤 API](51-ui-controls.md) | UIK | controls 21종 + nexa-sql 앱 내 범용 UI | ~222 | 없는 컨트롤 22(UIK-201~222 · 권장 API · 구현 순서) |
| [90 검증 매트릭스](90-verification-matrix.md) | — | 원장 ID ↔ 구현 ↔ 시험 | — | 마일스톤마다 갱신 |

**T-91(10-03)로 작성**: [52](52-nexa-ui-dlg-fs-status.md)(nexa-ctl TextBox·nexa-grid edit · nexa-dlg FilePicker · nexa-fs OS 분기 현황) · [92](92-qa-checklist.md)(dir2 대조 실기 QA 표 — 생성물 `scripts/qa-checklist.py`) · [98](98-dir2-release-parity.md)(dir2 릴리스 기능(로드맵 M0~M5) ↔ dir3 대조) · [99](99-coverage-gaps.md)(dir2 파일 커버리지 · GAP — 생성물 `scripts/coverage-files.py`).

## 2. 교차 검증에 쓰는 법

1. 기능 하나를 구현하기 전 원장에서 ID를 고른다 → 해당 문서의 "동작 상세·진입점·이식 분류(N/A/P/W)·OS 분기·주의"를 읽는다 → dir2 원본 코드를 **직접** 연다.
2. 구현 커밋 제목/본문에 ID를 적는다 → [90](90-verification-matrix.md)에 행(구현 위치 · 시험 층 · 시험 이름 · 상태).
3. 마일스톤 끝: 매트릭스에서 해당 M의 ID를 전수 대조 → 빈칸·🚧을 journal "교차 검증" 절에 센다 → 누락은 TODO.
4. 원장에 없는 dir2 기능을 발견하면 [99](99-coverage-gaps.md)(없으면 생성)에 `GAP-NNN`으로 먼저 등재.
5. 의도된 차이(예: 라이선스 메뉴 항목 추가 · 셸 메뉴 자체 구현)는 매트릭스 상태 "의도된 차이"로 — 근거 DR 번호 필수.

## 3. 이식 분류 범례

| 분류 | 뜻 |
| --- | --- |
| N | 플랫폼 중립 — 거의 그대로 이식 |
| A | nexa-ui 컨트롤·그리기로 교체(배치·동작은 유지) |
| P | OS별 구현 분기 필요(`platform/`) |
| W | Windows 전용 유지 — 타 OS는 대체·비활성(사유 안내) |
