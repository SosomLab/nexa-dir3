# port/98 · dir2 릴리스 기능(로드맵 M0~M5 · `0.22.0`) ↔ dir3 대조

> 작성 2026-10-03(T-91). dir2에는 CHANGELOG가 없고 릴리스 범위는 `nexa-dir2/docs/02-roadmap.md`(M0~M5 원안)와 README "현재 상태"(포스트 M5 · `0.22.0` · exe 4.04 MB · 임포트 OS 인박스 21종)로 정의된다.
> 이 표는 **로드맵 글머리 1개 = 행 1개**로 dir3 어디에 있는지(이식 원장 접두 · TODO 항목 · 매트릭스 상태)를 적는다. 세부 ID 단위는 [90](90-verification-matrix.md)이 원천이고, 여기는 **릴리스 관점의 빠짐 점검**이다.
> 상태: ✅ dir3에 있음(시험 포함) · 🚧 부분 · ⚠ 의도된 차이(DR) · ☐ 없음.

## M0 — 기반·게이트

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| 워크스페이스 · 코어 3크레이트(core/vfs/tree) rlib 이식 · 테스트 green | `ndir-core`·`ndir-vfs`·`ndir-tree`(dir2 테스트 그대로 · DR-11) | T-10 ✅ · 99 GAP-001(아카이브 6종 포함) | ✅ |
| Win32 창 + GDI 렌더 스파이크 | winit + softbuffer + nexa-gfx CPU 래스터(3-OS) | DR-1 · T-40 ✅ | ⚠ DR-1(대체) |
| CI(core/windows) | CI 3-OS 매트릭스 + plugins 잡 + budget 단계 | T-03 ✅ · T-07 🚧(wasm32 빌드·로드 ✅ · 플러그인 임포트 화이트리스트·용량 = M7) | ✅ |
| 게이트: 유휴 RSS · exe 크기 · 임포트 테이블(dir2 실측 `0.22.0`: exe 4.04 MB · 임포트 인박스 21종 · RSS 16.86 MB) | exe ≤ 10 MB(`ci.yml` budget 단계 · 실측 10-03 release 5.31 MB = 5,307,392 B) · 임포트 = OS 인박스만(`check-imports.ps1` · 같은 budget 단계) · **RSS 미측정** | `ci.yml:62` · T-08 🚧(RSS) · perf-baseline ④ | 🚧 RSS |

## M1 — 뷰어 코어

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| `nexa-gui` 분리(위젯·무효화·입력 라우팅) · DirectWrite 렌더 | nexa-ui(nexa-ctl/grid/explorer) · ab_glyph 래스터 + 광학 보정 | DR-2 · T-25~27 ✅ · RENDER 행 ⚠ DR-1 | ⚠ DR-1/2 |
| 가상화 파일 리스트 · 컬럼 5 · 헤더 정렬(3상태·다중열) | nexa-grid `VirtualRows` · `Column` | PANEL 130/138 ✅ · T-25 ✅ · T-43 ✅ | ✅ |
| 셸 아이콘(LRU) | nexa-fs `IconService`는 런처·템플릿에만 쓰이고 **패널 행 아이콘은 미구현**(`filelist.rs:276` `TreeSource`에 `icon` 구현 없음 · [52](52-nexa-ui-dlg-fs-status.md) UIX-033) | 개발 세션 확인 대기 | 🚧 |
| 인라인 폴더 확장 + 교차폴더 다중 선택 · 키보드 네비 전체 · 타입어헤드 · 러버밴드 | nexa-grid Tree 모드 · 선택 모델 · `typeahead` · 밴드 | PANEL/CMD 행 · T-43 ✅ | ✅ |
| 숨김/점 파일 토글 · 네비게이션(뒤로/앞으로/위로 · 히스토리) | `list.show_hidden/show_dotfiles` · `nav` | T-43 ✅ · KEY ✅ | ✅ |
| 게이트: 100k 첫 렌더 <150 ms(dir2 실측 115 ms) · 스크롤 60 fps · RSS | **미측정** — `scripts/perf-baseline.sh`는 기동(`--smoke`)·자가 점검 그룹·exe 크기만 잰다. 대량 폴더 목록·첫 렌더·fps는 헤드리스 측정 경로가 없다 | T-08 🚧 · journal 10-03 §70 수치 | ☐ |

## M2 — 셸 골격·상주

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| 계층 경로 바(세그먼트 · 편집 · 자동완성) | nexa-explorer `PathBar` | T-26 ✅ · GUI-110~151 행 | ✅ |
| 듀얼 패널 + 패널별 탭(드래그 · 잠금/고정) · 스플리터 | `panel.rs` · 탭 컨텍스트 메뉴 · 세션 | T-43 ✅ · T-45 ✅ | ✅ |
| 메뉴 바 · 도구 모음 · 상태바 · 테마(라이트/다크/시스템) | nexa-ctl MenuBar/Toolbar/StatusBar · `theme::resolve` · 툴바 SVG 아이콘 | T-22~24 ✅ · T-42 🚧(툴바 오버플로 등) · T-30 A ✅ | 🚧 |
| 설정/세션 영속(`data\`) · i18n(en/ko) | `ndir-settings`(nexa-sql 구조 · DR-3) · `ndir-i18n`(en/ko/ja) · 포터블 `data/`(DR-9) | T-12~13 ✅ · T-44~45 ✅ · SET/KEY ✅ | ✅ |
| IME(한글) 1차 · UIA 접근성 1차 · 유휴 트림 | IME = winit `Ime` 사건(3-OS) · **UIA 없음**(매트릭스 SHELL-090~100 ⚠ "접근성 = nexa-ui 범위 밖(후속 결정)") · 유휴 = `WaitUntil` | SKEL-061~085 행 🚧(IME 실기 ko/ja) · SHELL-090~100 ⚠ | 🚧 UIA ☐ |
| 게이트: 상주(탭 4 · 듀얼) RSS ≤ 30 MB | 미측정 | T-08 🚧(RSS 미측정) | ☐ |

## M3 — 파일 조작

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| 전송 엔진(복사/이동 · 진행률 · 충돌 · 취소) · 삭제(휴지통/완전) · 이름 변경 · 새로 만들기 · Undo/Redo | `ndir-ops` 이식 + `app/ops.rs` · 진행 창(`SegProgress`) · 휴지통 3-OS · 새로 만들기(템플릿 SHELL-008) · undo 스택 | T-11 ✅ · T-70 🚧(잔여 확인) · T-71 ✅ · OPS 85/162 ✅ | 🚧 |
| 셸 컨텍스트 메뉴(IContextMenu + 고유 병합) · 클립보드 상호운용 · OLE DnD | `platform/winshell.rs`(COM · 메뉴 스레드) · 파일 클립보드 3-OS · DnD 1차(수신 · winit) — **발신/OLE 완전 · 자동 스크롤 = DnD 2차** | T-51~53 🚧 · SHELL 38/75 ✅ | 🚧 |
| watcher(무간섭 갱신) | `platform/{winwatch,linuxwatch,macwatch}.rs` + `reopen_filtered` 규약 | T-52/53 감시 ✅ · PANEL-036 ✅ | ✅ |

## M4 — 하단 패널

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| 정보 뷰 · 내장 미리보기(텍스트/이미지) | nexa-explorer `InfoDock` + `dockinfo.rs`(8줄 · 형식별) · 텍스트/이미지(`draw_image_hint`) | T-27 ✅ · T-60 🚧(형식별 상세) | 🚧 |
| `nexa-term`: ConPTY + VT 스크린 | `ndir-term`(VT 이식) · `platform/winpty.rs`(ConPTY) · `unixpty.rs`(forkpty) · `termview.rs`(셀 격자 · 굵게 · 선택 · 복사 서식) | T-11 ✅ · T-61 🚧 · TERM 73/90 ✅ | 🚧 |

## M5 — 마감·릴리스 · 포스트 M5

| dir2 릴리스 기능 | dir3 | 근거 | 상태 |
| --- | --- | --- | --- |
| 퀵 런처 · 일괄 이름변경 | `launcher.rs`(+ exe 아이콘 T-30 B) · 일괄 이름 변경 창 | T-30 ✅ · T-71 ✅ | ✅ |
| 접근성/IME 마감 | IME ✅(winit) · 접근성 ☐(위 M2) | — | 🚧 |
| 릴리스 파이프라인(단일 exe 첨부) · 예산 게이트 · 서명 | `release.yml`(태그 → 3-OS 설치 스모크 → 초안) · MSI/zip/pkg/dmg/deb/rpm · 서명 미결(사용자) | T-82 🚧 · 23 PROC 행 | 🚧 |
| 후속: 검색 · 아카이브 · 클라우드 · 플러그인 | 아카이브 그리드 창 ✅(PLUG-070~077) · 플러그인 ABI 바이트 호환 ✅(DR-7) · **클라우드 ☐(DR 후보 · CLOUD 99 ID 미이식)** · 검색 = dir2에도 없음 | T-62~63 ✅/🚧 · T-72 ☐ | 🚧 CLOUD |
| 포스트 M5 UX(0.7~0.22): 순서 편집기 · 템플릿 새로 만들기 · 잘라낸 항목 흐림 · 세션 코얼레싱 · 터미널 설정 · 핀 그룹 · 폴더 우선 | 순서 편집기 ✅(T-71) · 템플릿 ✅(SHELL-008) · 흐림 ✅(SHELL-044) · 세션 자동 저장 ✅(T-45) · 터미널 wrap/cols ✅ · 핀/고정 ✅ · 폴더 우선 ✅ | 매트릭스 해당 행 | ✅ |

## 요약(릴리스 관점 빠짐)

1. **없음(☐)**: 클라우드(CLOUD 99 ID · DR 후보 — 사용자 결정) · UIA/접근성 · 성능 게이트 측정(RSS · 100k 첫 렌더 · fps — T-08).
2. **부분(🚧)**: 패널 행 셸 아이콘(52 UIX-033) · DnD 2차(발신 · OLE · 자동 스크롤) · 툴바 오버플로(T-42) · 도크 형식별 상세(T-60) · 터미널 잔여(T-61) · 전송 잔여 확인(T-70) · 패키징 서명/채널 확정(T-82).
3. 나머지 로드맵 항목은 dir3에 있고 시험이 붙어 있다(매트릭스 10-03 §70 감사: ✅ 2,357 / 4,289 · 🚧 1,024 · ⚠ 766 · 미착수 0 — 원장 증가분 23 = 52의 UIX).
