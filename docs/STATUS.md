# STATUS — 현황 한 장

> 최신 위. 상세는 [journal](journal/), 요약은 [DEVLOG](DEVLOG.md), 목표 대비는 [MILESTONES](MILESTONES.md) · [TODO](TODO.md).

## 10-03 67차 — 우클릭 가속(dir2 X-61) + 셸 메뉴 아이콘 칸

- **한 일**: 전용 메뉴 STA 스레드 + 선행 구축(300 ms · 배경 · 감시 무효화) + 즉시 열림/나중 채움 + `invoke_async` + `hbmpItem` 아이콘(사용자 피드백 2건) · 시험 +4(167) · 매트릭스 SHELL-014~016 ✅ · 011 🚧 · GAP-003(패널 행 셸 아이콘)·004(FilePicker 주입) 등재.
- **지금 상태**: 우클릭 = 준비된 대상이면 µs 단위로 즉시 · 아니면 자체 항목 먼저. 다음 = 네비 버튼 dir2 구성/MDL2 글리프 · 두부 방지 · 쉐브론 글리프(사용자 스크린샷) → GAP-003 → GAP-004.
- **걸린 것**: 실기 확인(우클릭 체감 속도 · 아이콘 칸 — 사용자) · owner-draw 메뉴 아이콘 미수집 · 실기 QA 수행(92) · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §71](journal/2026-10-03.md)

---

## 10-03 66차 — T-91 누락 문서 4 · 실기 QA 표 · 성능 기준선 1차

- **한 일**: port/52·92·98·99 + 생성 스크립트 2(`qa-checklist.py`·`coverage-files.py` · check-all 단계) · `perf-baseline.sh` 버그 3 수정 · 기준선(기동 2,935 ms · ctxmenu 1,884 ms · exe 5.06 MiB) · 매트릭스 CI-118 🚧 + UIX 3행 · 감사 원장 4,289 덮음 100 %.
- **지금 상태**: T-91 ✅ · T-08 🚧(대량 폴더·RSS·fps 미측정). 개발 세션 = 우클릭 메뉴 아이콘 칸 · 우클릭 지연(X-61 메뉴 STA 스레드 + 선행 구축). 다음 = 빈칸 2 확인(FilePicker 최근/숨김 주입 · 패널 행 셸 아이콘) · CLOUD 결정 · SHCNE · DnD 2차 · ICO.
- **걸린 것**: 실기 QA 수행(92 · 사용자) · 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §70](journal/2026-10-03.md)

---

## 10-03 65차 — T-32 Toast 승격 · FolderTree/FilterBox 범위 밖

- **한 일**: 앱 `toast.rs` 삭제 → nexa-ui 114차 `controls::toast`(UIK-213 ✅) · UIK-210/214는 dir2에 실체 없음 확인(⚠ 사유).
- **지금 상태**: M3 컨트롤 묶음(T-30~32) 종결 — 남은 건 ICO 디코더(UIC-317 · P2). 다음 = T-91(QA 표·성능 기준선·누락 문서) · CLOUD 결정 · SHCNE · DnD 2차.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §69](journal/2026-10-03.md)

---

## 10-03 64차 — T-31 B(italic 합집합 · 글꼴 장식 설정 · 터미널 굵은 셀)

- **한 일**: nexa-ui 113차 `DrawCtx::select_font_styled` + `RecordCtx.fonts` · dir3 `apply_font_decor`(dir2 X-12 세 설정이 비로소 적용) · 터미널 SGR 1 굵게 그리기 · T3 `font_decor_settings_reach_grid_font_selection` · 매트릭스 UIC-311/313 ✅ · 315 ⚠(PeerList 매핑 수용 — nexa-sql `FontPrefs` 리터럴 때문에 슬롯 추가 불가).
- **지금 상태**: T-31 잔여 = ICO 디코더(UIC-317)뿐(테마 토큰은 nexa-grid 대체로 종결). 다음 = T-91 · CLOUD 결정 · SHCNE · DnD 2차 · T-32.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §68](journal/2026-10-03.md)

---

## 10-03 63차 — T-31 클립 스택(nexa-ui 112차 소비)

- **한 일**: nexa-ui `RasterCtx` 클립 스택 실제 구현(91214a5 · 모든 그리기 어휘 · 시험 +2) · nexa-grid `paint_grid` = `push_clip(bounds)` · dir3 T3 `panel_grid_pushes_its_bounds_as_clip` · 매트릭스 UIC-310/N-001 ✅.
- **지금 상태**: 셀이 패널 경계를 넘치는 실제 렌더 결함 해소(호스트 열 폭 맞춤은 사용성 때문에 유지 · T-43). T-31 잔여 = 터미널 셀 텍스트·italic·List 슬롯·테마 토큰·ICO. 다음 = T-91 · CLOUD 결정 · SHCNE · DnD 2차 · T-32.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §67](journal/2026-10-03.md)

---

## 10-03 62차 — T-90 5차: 원장 전 ID 덮음(묶음 행 28)

- **한 일**: RENDER/B/N/L/O/OS/RT/T/CI/SET/SHELL/소형 묶음 행 · 감사 재생성.
- **지금 상태**: 매트릭스가 원장 4,266 ID 전부를 가리킨다(상태별 수치 = 90 집계). T-90 = 전수 달성 · 이후 🚧→✅ 세분화는 T-91과 함께. 다음 = T-91 QA 표/성능 기준선 · CLOUD 결정 · SHCNE · DnD 2차 · T-31/T-32.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §66](journal/2026-10-03.md)

## 10-03 61차 — T-90 4차: 접두별 묶음 행 44

- **한 일**: PROC/WINA/WINB/WINC/PANEL/OPS/TERM/PLUG/PREFS/DLG/GUI/LIC/EXT/SKEL/UIC/UIK/CLOUD 묶음 행 · 감사 재생성.
- **지금 상태**: 덮음 3,677/4,266(86 %) · ✅ 2,136. 남은 미착수 589 = 렌더링(RENDER/B)·OS 분기(OS/L/N/O)·위험(RT/T) 원장 + 세부 ID. 남은 미착수 = 세부 ID(회귀 후보·메시지 단위) → T-91 QA 표와 함께. 다음 = T-91 · CLOUD 결정(DR) · SHCNE · DnD 2차.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정 · CLOUD 이식 여부 결정.

→ [journal/2026-10-03 §65](journal/2026-10-03.md)

## 10-03 60차 — T-90 3차: CMD 원장 대조 + 묶음 행(덮음 37 %)

- **한 일**: `dir2_catalog_menu_commands_map_to_dir3_ids` · CMD 묶음 행 9 · 감사 재생성.
- **지금 상태**: 덮음 1,603/4,266. 다음 = PREFS/SET/SKEL/UIC/WINB/WINC 묶음 행 · CLOUD/PROC 사유 행 · T-91.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §64](journal/2026-10-03.md)

## 10-03 59차 — T-90 2차: 원장 기반 전수 시험 + 집단 행(덮음 29 %)

- **한 일**: `dir2_catalog_i18n_keys_present_in_all_langs` · `dir2_catalog_settings_keys_are_mapped` · 매트릭스 집단 행 4 · 감사 재생성.
- **지금 상태**: 덮음 1,250/4,266. 다음 = CMD·PREFS·SET·SKEL 묶음 행 · 미이식 사유 행 · T-91 QA 표.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §63](journal/2026-10-03.md)

## 10-03 58차 — T-90 1차: 매트릭스 전수 감사 스크립트 + 미착수 전수(91)

- **한 일**: `scripts/matrix-audit.py` · docs/port/90 집계 생성물화 · docs/port/91 생성(미착수 3,574) · check-all 단계.
- **지금 상태**: 덮음 692/4,266(16 %). T-90 2차 = 집단 행(KEY·CMD·PREFS·SET)으로 실제 시험 범위를 ID에 연결 · 미이식(CLOUD·PROC) 사유 행. 다음 = T-90 2차 · T-91 QA 표 · SHCNE · DnD 2차.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §62](journal/2026-10-03.md)

## 10-03 57차 — DnD 1차: 외부 끌어다 놓기 수신(3-OS 공통)

- **한 일**: `app/dnd.rs` · 이벤트 루프 3 arm + 틱 flush · i18n 2키 · 시험 +1(163).
- **지금 상태**: DnD = 수신 1차 ✅ · 발신/OLE 완전/자동 스크롤 = 2차. 다음 = T-90/91(매트릭스 전수 · QA 표) · SHCNE.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 드롭 좌표 실기 확인 · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §61](journal/2026-10-03.md)

## 10-03 56차 — SHELL-044: 잘라낸 항목 흐림(3-OS)

- **한 일**: `TreeSource.cut_marks`/`is_ghosted` · `Panel::set_cut_marks` · `App::sync_cut_marks`(4 시점) · 시험 +1(162).
- **지금 상태**: T-51 B-2 잔여 = OLE DnD · SHCNE. 다음 = T-90/91(매트릭스 전수 · QA 표) · DnD 1차(winit DroppedFile 수신).
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §60](journal/2026-10-03.md)

## 10-03 55차 — T-63 매니저 1차: 플러그인 설치/삭제(무재시작)

- **한 일**: `app/plugins.rs` · `preview` 캐시 무효화/사용자 폴더/검증 · 설정 창 [설치…]/[삭제] · `FilePurpose::Plugin` · i18n 8키 · 시험 +1(161).
- **지금 상태**: T-63 = A·B·매니저 1차 ✅(2단계 원격 저장소는 보류). 다음 = T-90/91(매트릭스 전수 · QA 표) · DnD(NSDragging/XDND) · SHELL-044.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §59](journal/2026-10-03.md)

## 10-03 54차 — T-52: macOS 시스템 휴지통 + 복원

- **한 일**: `SystemTrash`(trashItemAtURL · moveItem 복원 · 폴백) · Cargo objc2-foundation NSFileManager/NSError · 시험 +1(macOS CI).
- **지금 상태**: 휴지통 복원 3-OS(Windows undelete · macOS trashItem · Linux trashinfo) ✅. T-52/53 잔여 = DnD(NSDragging · XDND) · SHELL-044. 다음 = T-90/91 · T-63 매니저.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §57](journal/2026-10-03.md)

## 10-03 53차 — T-53: Linux 파일 클립보드(X11 uri-list) → 파일 클립보드 3-OS 완료

- **한 일**: `clipboard_x11.rs` Payload/파일 타깃/`read_files` · `X11Files` · `Platform::native` Linux 교체 · 시험 +1(ubuntu CI).
- **지금 상태**: 파일 클립보드 = Windows · macOS · Linux ✅. T-52/53 잔여 = DnD(NSDragging · XDND) · mac trashItem · SHELL-044 흐림 통지. 다음 = T-90/91(매트릭스 전수 · QA 표) · T-63 매니저.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §56](journal/2026-10-03.md)

## 10-03 52차 — T-52: macOS 파일 클립보드(NSPasteboard)

- **한 일**: `macclip.rs` · Cargo objc2 기능 · `Platform::native` macOS 클립보드 교체 · 시험 +1(macOS CI).
- **지금 상태**: 파일 클립보드 = Windows(CF_HDROP) · macOS(NSPasteboard) ✅ · Linux(uri-list) ☐. 다음 = Linux uri-list(clipboard_x11 다중 타깃) · T-90/91 · T-63 매니저.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §55](journal/2026-10-03.md)

## 10-03 51차 — T-52/53: 폴더 감시 네이티브(inotify · kqueue)

- **한 일**: `linuxwatch.rs` · `macwatch.rs` · `Platform::native` 분기 · OS별 시험 +2(CI 러너가 실행).
- **지금 상태**: 폴더 감시 3-OS 네이티브 완료. T-52/53 잔여 = 파일 클립보드(NSPasteboard · uri-list) · DnD(NSDragging · XDND) · mac trashItem. 다음 = T-90/91 · T-63 매니저 · 클립보드.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §54](journal/2026-10-03.md)

## 10-03 50차 — SHELL-008: 행 메뉴 "새로 만들기 ▸"(3-OS 템플릿 포트)

- **한 일**: `platform::Templates`(+`wintemplates.rs` 레지스트리 · `UserTemplates` · XDG) · `create_new_at`/`NewKind` · 행 메뉴 서브메뉴 · 덤프 자식 표기 · 시험 +3(160).
- **지금 상태**: T-51 B-2 잔여 = OLE DnD · SHCNE 통지. 다음 = T-52/53(mac/linux 잔여) · T-90/91 · T-63 매니저.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §53](journal/2026-10-03.md)

## 10-03 49차 — T-30 B: 퀵 런처 exe 셸 아이콘

- **한 일**: `app/launcher_icons.rs` · `launcher::exe_path` · `make_launcherbar`(16px · 패딩 2) · 이벤트 루프 폴링 · 덤프 · 시험 +2(157).
- **지금 상태**: T-30 완료 판정(A 툴바 SVG · B 런처 아이콘 · 소형 컨트롤은 기존 nexa-ctl로 대체 완료). 다음 = ShellNew 행 메뉴 · T-52/53 · T-90/91 · T-63 매니저.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §52](journal/2026-10-03.md)

## 10-03 48차 — T-63 B: 플러그인 설정 페이지 체크박스

- **한 일**: `prefs_win.rs` 플러그인 블록(동적 Checkbox 묶음 · 실시간 `plugins.disabled`) · `set_plugins` · 덤프 · 시험 +1(155) · `prefs-open.scn` 확장.
- **지금 상태**: T-63 잔여 = 매니저(EXT-415). 다음 = ShellNew 행 메뉴 · T-52/53 · T-90/91 · T-30 B.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §51](journal/2026-10-03.md)

## 10-03 47차 — T-71 완료: 순서/표시 편집 창(툴바 · 컬럼 · 컨텍스트 메뉴 공통)

- **한 일**: `order.rs` · `order_win.rs` · `app/order.rs` · 설정 창 [편집…] 3필드 · `toolbar.layout`/`ctxmenu.layout`/`list.col_layout` 실제 반영 · 세션 `cols` · 시험 +6(154) · `order-editor.scn`.
- **지금 상태**: T-71 ✅(잔여 = 툴바/헤더 우클릭 진입 · 접기). 다음 = ShellNew 행 메뉴 · T-63 B · T-52/53 · T-90/91.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기 · 릴리스 태그는 사용자 결정.

→ [journal/2026-10-03 §50](journal/2026-10-03.md)

## 10-03 46차 — T-82: 패키징 3-OS(nexa-sql packaging 이식 · release.yml · 동봉 플러그인)

- **한 일**: `packaging/` 트리 전체 + `scripts/third-party-notices.*` · `check-imports.ps1` + `release.yml` + 브랜딩 PNG · §3-4 결정(MSI+zip · 예산 10 MB(실측 5.2) · plugins/ · MIME) · 로컬 MSI/zip 빌드.
- **지금 상태**: 릴리스 태그는 아직 안 찍음(사용자 결정) · deb/rpm/pkg/dmg는 CI에서 첫 실행 때 검증. 다음 = ShellNew 행 메뉴 · T-63 B · T-52/53 · T-71 순서 편집기 · T-90/91.
- **걸린 것**: 실기 캡처 비교 전(사용자) · 글꼴/전각 피드백 재확인 대기.

→ [journal/2026-10-03 §49](journal/2026-10-03.md)

## 10-03 45차 — T-71: 일괄 이름 변경 창(카드 파이프라인 · 미리보기 · 프리셋)

- **한 일**: `bulk_win.rs` · `app/bulk.rs` · `edit.bulk_rename` · 시험 +2(148).
- **지금 상태**: T-71 잔여 = 순서 편집기(툴바/메뉴 DLG-069~073) · 프리셋 관리 팝업 · 날짜 TZ. 다음 = T-82 패키징 · ShellNew · T-63 B · T-52/53.
- **걸린 것**: 실기 캡처 비교 전(사용자).

→ [journal/2026-10-03 §48](journal/2026-10-03.md)

## 10-03 44차 — T-05: `check-all.sh` 전체 게이트

- **한 일**: `scripts/check-all.{sh,ps1}` · summary.txt · 문서 규약 갱신.
- **지금 상태**: push 전 = `check-all.sh`(또는 `--quick` + 시나리오). 다음 = T-71 일괄 이름 변경 창(dir2 bulkrename 2,252줄 · 코어 `ndir-ops::batch_rename` 이미 이식) · T-82 · ShellNew.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §47](journal/2026-10-03.md)

## 10-03 43차 — T-70: 전송 진행 창(세그먼트 바 · 취소 · 닫기 카운트다운)

- **한 일**: nexa-ui `SegProgress` · `progress_win.rs` · 워커 항목 진행 · 시험 +2(146).
- **지금 상태**: T-29 완료 · T-70 잔여 = 삭제 잠금 프로브(WINB-024). 다음 = T-71 일괄 이름 변경 창 · T-82 패키징 · ShellNew.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §46](journal/2026-10-03.md)

## 10-03 42차 — 사용자 피드백(글꼴 크기 em 변환 · 고정폭 글꼴) · T-62 C-3 드래그 선택

- **한 일**: nexa-ui 110차 `em_to_px` · dir3 `app/fonts.rs`(font_px/font_prefs/mono) · 미리보기 창 드래그 선택 · 시험 +2(144).
- **지금 상태**: T-62 완료(A·B·C). 사용자 재확인 대기 = 크기(12 em → 16 px) · 터미널 셀 폭(Consolas). 다음 = T-82 패키징 · ShellNew · T-52/53 포트 잔여.
- **걸린 것**: Info 도크의 "전각" 보고는 Mono 슬롯 미사용이라 원인이 다를 수 있음 — 수정 뒤 캡처로 재확인.

→ [journal/2026-10-03 §44·§45](journal/2026-10-03.md)

## 10-03 41차 — T-51 B-2c: 휴지통 복원(삭제 undo · Windows/Linux)

- **한 일**: `Trash::restore` 포트 · winrecycle · Linux .trashinfo · `TrashOp` · selfcheck 왕복 · 시험 +3(142).
- **지금 상태**: T-51 B-2 잔여 = 행 메뉴 ShellNew · OLE DnD · SHCNE 통지. 다음 = T-62 C-3(도크/창 드래그 문자 선택) · T-82 패키징 · ShellNew.
- **걸린 것**: macOS 휴지통 복원은 메타데이터가 없어 미지원(undo 실패 안내) — T-52 `trashItem` 되돌리기와 함께.

→ [journal/2026-10-03 §43](journal/2026-10-03.md)

## 10-03 40차 — T-51 B-2b: Windows 폴더 감시(ReadDirectoryChangesW) + 폴링 폴백

- **한 일**: `winwatch.rs` · 포트 간격 메서드 · selfcheck 감시 항목 · 시험 +2(139).
- **지금 상태**: T-51 B-2 잔여 = 행 메뉴 ShellNew · OLE DnD · 휴지통 복원 · 셸 통지(SHCNE). T-52/53 = FSEvents/inotify 같은 틀로. 다음 = T-62 C-3 · T-82 · ShellNew.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §42](journal/2026-10-03.md)

## 10-03 39차 — T-51 B-2a: Windows 배경 셸 메뉴 · 생성 감지 · selfcheck ctxmenu

- **한 일**: 포트 계약 2 메서드(기본 구현) · winshell 배경 메뉴 · ctxmenu 합류/실행 · fake · selfcheck · 시험 +2(137).
- **지금 상태**: T-51 B-2 잔여 = 행 메뉴 새로 만들기 ▸(ShellNew) · OLE DnD · ReadDirectoryChangesW · 휴지통 복원. 다음 = ShellNew 행 메뉴 또는 T-62 C-3 또는 T-82.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §41](journal/2026-10-03.md)

## 10-03 38차 — T-81: nexa-license 발급기 제품 분기(NDL · GUI 안내 · E2E)

- **한 일**: nexa-license `presets`/`main`/E2E/문서(태그 `nexa-dir3/t81-2026-10-03`) · dir3 LIC-163 확인.
- **지금 상태**: M7 잔여 = T-82 패키징 · 실기 발급(루트 키). 다음 = T-32 폴더 트리 · T-51 B-2 · T-62 C-3 · T-82.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §40](journal/2026-10-03.md)

## 10-03 37차 — T-30 A: 툴바 SVG 아이콘(dir2 자산 · 마스크 틴트 · HiDPI 재렌더)

- **한 일**: `assets/toolbar` 이식 · `icons.rs` · `build_toolbar` 마스크 · 배율 변경 재구성 · 시험 +3(135).
- **지금 상태**: M3 잔여 = T-30 B(런처 exe 아이콘 · 소형 컨트롤) · T-31(클립 스택 등) · T-32(폴더 트리). 다음 = T-32 또는 T-81/82(M7) 또는 T-51 B-2.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §39](journal/2026-10-03.md)

## 10-03 36차 — T-62 C-2: SVG 래스터(nexa-ui 108차) · 인라인 이미지 · Mermaid E2E

- **한 일**: nexa-gfx `svg`(파서+래스터) · nexa-ctl `draw_image_hint`+캐시 · dir3 `render_svg_impl` 3-OS · preview_win/InfoDock 인라인 이미지 · `tree: text` · 시험 +1(132) · 시나리오 19.
- **지금 상태**: T-62 잔여 = C-3 드래그 문자 선택. T-30(툴바 SVG 아이콘)은 이제 래스터가 있어 착수 가능. 다음 = T-30 또는 T-32 폴더 트리 또는 T-81/82.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §38](journal/2026-10-03.md)

## 10-03 35차 — T-62 C-1: 압축 그리드 창(nexa-grid) · Linux CI 런처 시드 수정

- **한 일**: `archive_win.rs` + 미리보기 흐름 배선 · `archive.dump` · 시험 +4(131) · 시나리오 18 · 런처 시드 시험 3-OS 안정화.
- **지금 상태**: T-62 잔여 = C-2(SVG 래스터 · 인라인 이미지 · 드래그 선택). 다음 = T-62 C-2 또는 T-32 폴더 트리 또는 T-81/82(M7).
- **걸린 것**: 없음.

→ [journal/2026-10-03 §37](journal/2026-10-03.md)

## 10-03 34차 — T-80: 라이선스 창 · About 대화상자 · 파일 창(FilePicker 호스트)

- **한 일**: `license_win.rs` + `app/license.rs` + `file_win.rs` · Help ▸ 라이선스… · About [라이선스…] · `license.install/dump` 기동 명령 · i18n 3언어 · 시험 +3(127) · 시나리오 17.
- **지금 상태**: M7 잔여 = T-81(발급기 보강 `mail_text` 제품 분기 · `--id-prefix` · E2E) · T-82(패키징 3-OS) · LICENSE 파일(LIC-163). M3 잔여 = T-30 · T-31 · T-32. 다음 = T-32 폴더 트리 또는 T-62 C 압축 그리드 또는 T-81/82.
- **걸린 것**: 상태줄 라이선스 배지(LIC-158 ⓒ)는 dir2 배치 유지 지시로 보류 · 유효 라이선스 설치 경로는 발급 PC 루트 키(`ROOT_KEYS`)가 비어 있어 실기로는 `Invalid(NoRootKey)`까지만 확인(단위 시험은 ndir-license에서 주입 키로 통과).

→ [journal/2026-10-03 §36](journal/2026-10-03.md)

## 10-03 33차 — T-42: 퀵 런처 바(3-OS 시드 · 24 밴드 · launch:<i>)

- **한 일**: `launcher.rs` + App 배선(배치 밴드 · 입력 영역 · 명령 · 설정 즉시 반영 · 덤프) · 픽스처 런처 끔(골든 안정) · 시험 +5(124) · 시나리오 16.
- **지금 상태**: M3 잔여 = T-30(SVG 툴바/런처 아이콘·소형 컨트롤) · T-31 · T-32(폴더 트리) · T-44 잔여 — 다음 = T-32 폴더 트리(nexa-ui FolderTree) 또는 T-62 C 압축 그리드 또는 M7(라이선스 창 · 배포 패키징).
- **걸린 것**: 런처 exe 아이콘 없음(라벨 버튼) — T-30 아이콘 추출(Windows SHGetFileInfo · mac NSWorkspace · Linux .desktop) 뒤.

→ [journal/2026-10-03 §35](journal/2026-10-03.md)

## 10-03 32차 — T-61 B: 터미널 글꼴 크기 · 고정 열/가로 스크롤 · HTML 복사 · TUI 마우스 모드

- **한 일**: `TermStyle` 도입(`term.font_size/wrap/cols`) · 가로 오프셋 `view_x` + 가로 휠 · `term.copy_format` html/both = CF_HTML 동시 게시 · DECSET 마우스 모드 SGR 좌표 전달(Shift = 로컬 선택) · 설정 즉시 반영 · 시험 +1(119).
- **지금 상태**: M5 🚧(T-61 잔여 = 픽셀 스크롤·고속 스크롤·RTF · T-62 C · T-63 B) · M4 🚧(T-51 B-2 · T-52/53) · M6 🚧 — 다음 = M3 잔여(T-42 런처 바 · T-30 SVG 툴바 아이콘 · T-32 폴더 트리) 또는 T-62 C 압축 그리드 또는 M7 라이선스 창/배포.
- **걸린 것**: RTF 클립보드 미지원(평문 폴백) · 비Windows 리치 클립보드 없음(T-52/53).

→ [journal/2026-10-03 §34](journal/2026-10-03.md)

## 10-03 31차 — T-51 B-1: Windows 셸 컨텍스트 메뉴(IContextMenu 포트 · 행 메뉴 합류)

- **한 일**: `platform/winshell.rs`(PIDL → IShellFolder → IContextMenu → HMENU 열거 → 항목 트리 · verb · InvokeCommand · `windows` 0.62) · 포트 계약 확장 · 행 메뉴 = 셸 항목 + 앱 고유(가로채기·중복 금지) · 실기 시험 1 · core 시험 보강(118).
- **지금 상태**: M4 🚧(T-51 B-2 = 배경 셸 메뉴 · ShellNew · OLE DnD · ReadDirectoryChangesW · 휴지통 복원 · T-52/53) · M5 🚧 · M6 🚧 — 다음 = T-61 B(터미널 가로 스크롤·HTML/RTF 복사·TUI 마우스) 또는 T-62 C 또는 M3 잔여(T-42 런처 · T-30 SVG 아이콘 · T-32 폴더 트리).
- **걸린 것**: 셸 `InvokeCommand`는 UI 스레드 동기(모달 대화상자를 띄우는 확장은 그동안 앱이 멈춤 — dir2는 전용 스레드) · 서브메뉴 아이콘 없음.

→ [journal/2026-10-03 §33](journal/2026-10-03.md)

## 10-03 30차 — 파일 행 · 배경 컨텍스트 메뉴(앱 고유 항목) + Shift+F10

- **한 일**: `app/ctxmenu.rs`(행 15항목 · 배경 8항목 · 활성/비활성 규칙 · 고유 항목 실행) · 패널 우클릭 보고 · `cmd.contextMenu` · 기동 명령 `ctx.pick` · 덤프 `ctx` · 시험 +1(117) · 시나리오 15.
- **지금 상태**: M4 🚧(T-51 B = 셸 메뉴 합류 · OLE DnD · ReadDirectoryChangesW · 휴지통 복원 · T-52/53) · M5 🚧(T-62 C · T-61 B · T-63 B) · M6 🚧(일괄 이름 변경 창 · 진행 창) — 다음 = T-51 B 셸 컨텍스트 메뉴(IContextMenu → `ContextMenuProvider` 포트 · 행 메뉴 상단 합류) 또는 T-61 B.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §32](journal/2026-10-03.md)

## 10-03 29차 — T-62 B: 독립 미리보기 창(F3 · ↗) · 압축 암호 입력 흐름

- **한 일**: `preview_win.rs`(스타일드 라인 렌더 · 스크롤 · 복사 · 창 없이 덤프) · `app/previewcmd.rs`(F3/↗ → 시임 → 창/안내/압축 요약 · 암호 대화상자 재시도 루프 + 세션 기억) · View 메뉴 `view.preview_window` · 시험 +2(116) · 시나리오 14.
- **지금 상태**: M5 🚧(T-62 C = 압축 그리드 창 · SVG 래스터 · 인라인 이미지 · T-63 B) · M6 🚧(일괄 이름 변경 창 · 진행 창) — 다음 = T-51 B(Windows 셸 메뉴 IContextMenu · OLE DnD · ReadDirectoryChangesW · 휴지통 복원) 또는 T-61 B 또는 T-62 C.
- **걸린 것**: 압축 목록은 창에 요약 텍스트로(그리드 컨트롤 = nexa-ui 추가 필요 — port/20 §3 DataGrid).

→ [journal/2026-10-03 §31](journal/2026-10-03.md)

## 10-03 28차 — T-29 A: 대화상자 창(영구 삭제 확인 · 전송 충돌 4버튼 · 암호 입력 준비)

- **한 일**: `dlg_win.rs` + `app/dialogs.rs`(모달 보조 창 · 요청 큐 · `DlgReply` 분기 · 창 없이도 결정) · `edit.delete_permanent` 확인 · 충돌 = 작업 스레드 채널 질문 → 4버튼 회신(OPS-004 ⚠ 해소) · 기동 명령 `dlg.*` · 시험 +2(114) · 시나리오 13.
- **지금 상태**: M6 🚧(남은 것 = 진행 창(선택) · 일괄 이름 변경 창) — 다음 = T-62 B(F3 독립 미리보기 창 · 압축 그리드 창 · 암호 입력 — 대화상자 재사용) 또는 T-61 B 또는 T-51 B(셸 메뉴 · DnD · ReadDirectoryChangesW · 휴지통 복원).
- **걸린 것**: 진행 창 없음(상태줄 % + 취소는 `ops.cancel` 기동 명령뿐 — 메뉴/단축키 배선은 진행 창과 함께).

→ [journal/2026-10-03 §30](journal/2026-10-03.md)

## 10-03 27차 — M6 B: 새 폴더/새 파일 · 인라인 이름 바꾸기(F2) · 편집 필드 명령

- **한 일**: `create_new`(unique 이름 · CreateOp · 생성 행 선택 + 즉시 이름 바꾸기) · `begin_rename`/`apply_rename`(nexa-grid 인라인 편집 · RenameOp · 상태줄) · 편집 필드 안 Edit 명령 라우팅 · 기동 명령 `ui.type`/`ui.press` · 시험 +1(112) · 시나리오 12.
- **지금 상태**: M6 🚧(A·B ✅ — 남은 것 = 확인/진행 창(T-29) · 일괄 이름 변경 창 · 영구 삭제) · 다음 = T-29 nexa-dlg(확인/입력 대화상자 — 충돌 4버튼 · 영구 삭제 · 암호 창이 전부 이 위에) 또는 T-62 B · T-61 B · T-51 B.
- **걸린 것**: 대화상자 부재가 M6 잔여·T-62 B(암호)·T-29 전부의 공통 선행 — nexa-dlg를 다음 슬라이스로.

→ [journal/2026-10-03 §29](journal/2026-10-03.md)

## 10-03 26차 — M6 A: 복사/잘라내기/붙여넣기 · 전송 엔진 작업 스레드 · undo/redo

- **한 일**: `app/ops.rs`(클립보드 2단 · `paste_dest` · 작업 스레드 전송 + 틱 진행 · 완료 재열람/토스트/히스토리 · undo/redo · `ops.busy`) · `Platform.trash = Rc`(삭제 주입 공유) · i18n `clip.*` 3언어 · 덤프 `ops` · `ops.cancel` · 시험 +1(111) · 시나리오 11.
- **지금 상태**: M6 🚧(A ✅) — 다음 = M6 B(새 폴더/새 파일 · 인라인 이름 바꾸기 · 영구 삭제 · 일괄 이름 변경 창 · 충돌/진행 창은 T-29 뒤) 또는 T-62 B(F3 창 · 압축 그리드) 또는 T-51 B(셸 메뉴 · DnD · 감시).
- **걸린 것**: 충돌 확인 창 없음(건너뜀 정책 ⚠ OPS-004) · 진행 창 없음(상태줄 %만) — nexa-dlg(T-29) 뒤.

→ [journal/2026-10-03 §28](journal/2026-10-03.md)

## 10-03 25차 — T-54: Help ▸ 자가 점검 창(백그라운드 점검 · 표 · 복사)

- **한 일**: `check_win.rs`(작업 스레드 `selfcheck::run` + mpsc 수거 · 판정 색 표 · 요약 · 다시 점검/F5 · 복사 · Esc) · `help.selfcheck` 명령/키/메뉴/i18n 3언어 · 덤프 `check` · 시험 +3(110) · 시나리오 10 · CI `plugins` 잡 실행 비트(+x) 수정.
- **지금 상태**: M4 🚧(T-51 B·52·53 잔여) · M5 🚧 — 다음 = T-62 B(F3 독립 미리보기 창 · 압축 그리드 · 암호 창 · SVG 래스터) 또는 T-61 B(터미널 가로 스크롤·HTML/RTF 복사) 또는 M6 파일 작업(edit.cut/copy/paste · ndir-ops 배선).
- **걸린 것**: selfcheck 그룹 중 pty/ctxmenu/clipboard/dnd/preview/archive/cloud/window은 아직 "(not implemented)" SKIP — 각 기능 슬라이스가 채운다.

→ [journal/2026-10-03 §27](journal/2026-10-03.md)

## 10-03 24차 — T-63 A + T-07: 플러그인 빌드 스크립트 3-OS · CI wasm32 검증 · 설정 플러그인 목록

- **한 일**: `plugins/sdk/plugins.list` 단일 출처 + `scripts/plugin-build.{sh,ps1}`(옵션 동형) · CI `plugins` 잡(빌드 → `NDIR_PLUGINS_DIR` 자가 점검 로드 검증) · 설정 창 `plugins.disabled` 설명 줄에 로드 목록·오류 · 로컬 실기 PASS(2종 빌드 · selfcheck 3 PASS).
- **지금 상태**: M5 🚧 — 다음 = T-62 B(F3 독립 미리보기 창 · 압축 그리드 · 암호 창 · SVG 래스터) 또는 T-54(Help ▸ 자가 점검 창) 또는 T-61 B(터미널 가로 스크롤·HTML/RTF 복사).
- **걸린 것**: 플러그인 체크박스 묶음 UI(T-63 B) · 매니저(sha256·설치·index.json)는 T-63 B.

→ [journal/2026-10-03 §26](journal/2026-10-03.md)

## 10-03 23차 — M5 T-62 A: WASM 플러그인 런타임 · 미리보기 시임 · 동봉 플러그인 2종

- **한 일**: dir2 `preview/` 이식(`mod/wasm/archive/sample_tests` — 격리 수치·브레이커·ABI v1/v2·암호 슬롯 그대로) · 탐색 경로 3단 + `NDIR_PLUGINS_DIR` · 로드 오류 표면화 · 도크 미리보기가 시임을 소비(태그 벗기기·압축 요약·공급자 id) · 동봉 `plugins/markdown.wasm`·`archive.wasm`(dir2 dist 무수정) + `plugins/sdk/` 소스 · 자가 점검 `plugin` 3항목 · 시험 +18(107) · 시나리오 9.
- **지금 상태**: M5 🚧 — 다음 = T-62 B(F3 독립 미리보기 창 `LineView` · 압축 그리드 창 · 암호 창 · SVG CPU 래스터 · 인라인 이미지) 또는 T-63(설정 플러그인 페이지 · 매니저 · `plugin-build.{ps1,sh}`) 또는 T-54 자가 점검 창.
- **걸린 것**: Mermaid flowchart는 `render_svg` 미구현으로 아트/원문 폴백(이미지 마커 단언은 B에서 강화) · 비Windows 구형 zip 이름 디코더 없음(CP437 폴백).

→ [journal/2026-10-03 §25](journal/2026-10-03.md)

## 10-03 22차 — M5 T-61 A: 도크 터미널(Pty 3-OS · VT 셀 렌더 · 키/마우스 · cd 동기)

- **한 일**: Pty 포트 구현(Windows ConPTY `winpty.rs` · Unix `unixpty.rs` forkpty) + `termview.rs`(Utf8Chunker · pump · 셀 격자 렌더 · 선택/스크롤백) + 호스트 배선(`app/term.rs` — 지연 시작 · 30 ms 폴링 · 포커스 · 키 라우팅 · → cd · Edit 메뉴) + 기동 명령 `dock.kind/term.focus/term.send/term.dump`. ConPTY 실기 적발 2(파이프 std 핸들 누수 → `STARTF_USESTDHANDLES` · 종료 flush → read가 ClosePseudoConsole). 시험 +7(89) · 시나리오 8.
- **지금 상태**: M5 🚧 — 다음 = T-61 B(고정 열·가로 스크롤 · 트랙패드 픽셀 · 고속 스크롤 · HTML/RTF 복사 · TUI 마우스 모드 · 터미널 글꼴 크기) 또는 T-62 플러그인 런타임(WASM) → T-54 자가 점검 창.
- **걸린 것**: nexa-ctl DrawCtx에 클립 스택이 없어 셀 클립은 호출마다 rect(T-31). macOS/Linux pty는 CI 실기 시험으로만 검증(로컬 Windows).

→ [journal/2026-10-03 §24](journal/2026-10-03.md)

## 10-03 21차 — M5 착수: T-60 하단 도크(정보 · 미리보기 · 터미널 스트립)

- **한 일**: nexa-explorer `InfoDock` 2개를 App에 통합 — dir2 전폭 밴드 배치(`dock.visible` · `layout.dock_height_pct/dock_split_pct/info_mode`) · 정보 8줄(std 메타) · 미리보기 텍스트(64 KiB · 바이너리/빈 파일 판정 · 이미지는 경로만) · 활성 패널 원천 · `Area::Dock` 라우팅 · `dock.dump` · 시험 +4(82) · T4 시나리오 7.
- **지금 상태**: M4 🚧 · **M5 🚧** — 다음 = T-61 터미널 뷰(Pty 포트 ConPTY/forkpty · ndir-term VtScreen 셀 렌더 · cwd 동기 · → 버튼) → T-62 플러그인 런타임. 이미지 미리보기 그리기는 T-31 `draw_image`(nexa-grid Adapt) 뒤.
- **걸린 것**: 디스크 할당 크기·형식별 상세는 Windows 속성 시스템(T-5x) ⚠.

→ [journal/2026-10-03 §23](journal/2026-10-03.md)

## 10-03 20차 — PANEL-044 내 PC 전용 열 + Windows 클립보드 실기 교훈

- **한 일**: 내 PC(가상 최상위) = 드라이브 열(이름·종류·전체 크기·여유 공간 · 진입/이탈 시점만 교체 · 이름 폭 상속 · 타일 "X 중 Y 사용 가능" + 용량 바)을 Disk 포트로 채움(한 번만) · 시험 +3(78). Windows 파일 클립보드 opt-in 왕복 시험이 적발한 실기 교훈 2(DropEffect 선독 · 쓰기 직후 재렌더 재시도) 수정.
- **지금 상태**: M4 🚧 — 다음 = T-51 B(IContextMenu 셸 메뉴 · OLE DnD · ReadDirectoryChangesW) 또는 M5 터미널(ConPTY/forkpty · nexa-term) → T-54 자가 점검 창. 비Windows 내 PC 마운트 열거(X-17 β)는 T-53.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §22](journal/2026-10-03.md)

## 10-03 19차 — M4 T-51 A: 휴지통 3-OS · CF_HDROP · 용량 3-OS · `edit.delete`

- **한 일**: 휴지통(Windows SHFileOperationW ALLOWUNDO · Linux freedesktop Trash 규격 · macOS ~/.Trash) · Windows 파일 클립보드(CF_HDROP + Preferred DropEffect) · 드라이브 용량 Linux/mac(`statvfs` 수동 extern · CI 3-OS가 레이아웃 검증) · `edit.delete` = Trash 포트 → 재열람 + 토스트 · selfcheck `trash`(비CI). 시험 +4(75 · 1 ignored).
- **지금 상태**: M4 🚧 — T-50 ✅ · T-51/52/53 부분. 다음 = 내 PC 드라이브 열(PANEL-044 · Disk 포트 소비) → T-51 B(IContextMenu · OLE DnD · ReadDirectoryChangesW) 또는 M5 터미널(ConPTY/forkpty) → T-54 자가 점검 창.
- **걸린 것**: 삭제 확인창 없음(interim · 휴지통이라 되돌릴 수 있음 · T-29 nexa-dlg 뒤) · 실제 클립보드/휴지통 시험은 `--ignored`.

→ [journal/2026-10-03 §21](journal/2026-10-03.md)

## 10-03 18차 — M4 착수: T-50 플랫폼 포트 + 가짜 + ADR-0001

- **한 일**: `platform/{mod,fake,windows,macos,linux}.rs` — 포트 9종(DR-5 8 + `Disk`) · `Platform::native()`(셸 탐지 · 열기/보기 · Windows 용량 · 폴링 감시) · `Platform::fake()`(기록·주입) · `App.platform` 배선(파일 열기 = Opener · 1 s 폴링 자동 재열람) · `--selfcheck` shell/open/fs 실제 항목 · [ADR-0001](adr/0001-platform-ports.md). 시험 +5.
- **지금 상태**: M3 🚧(T-42 런처·SVG 잔여) · **M4 🚧** — 다음 = T-51 Windows(셸 메뉴 IContextMenu · 휴지통 · CF_HDROP · OLE DnD · ConPTY · ReadDirectoryChangesW) → T-52 macOS → T-53 Linux → T-54 자가 점검 나머지.
- **걸린 것**: Unix 드라이브 용량(statvfs 수동 extern 구조체 레이아웃) = T-52/53에서.

→ [journal/2026-10-03 §20](journal/2026-10-03.md)

## 10-03 17차 — M3 T-43 2차: 탭 메뉴 · 패널 간 이동 · 열 폭 동기

- **한 일**: `Tab{locked, pinned}` · 잠금/고정/복제/분리·부착(dir2 PANEL-016~022) · 탭 우클릭 메뉴(nexa-ctl ContextMenu · 잠금·고정·복제·새 탭·다른 패널로·닫기) · 열 폭 동기(`list.col_width_sync`) · 세션 잠금/고정 저장·복원 · i18n +1 · 시험 +2.
- **지금 상태**: M3 🚧 — T-40·41·43·44·45·46·06 ✅ · 남은 M3 = T-42 런처 바·SVG 아이콘(T-30과 함께) · T-44 잔여(JSON 편집 · 폴더 찾아보기 T-29). **M4 플랫폼 층 착수 가능**(T-50 포트 trait + Fake → 셸/PTY/휴지통/클립보드/드라이브 용량).
- **걸린 것**: 내 PC 용량 열(PANEL-044) = M4 용량 조회 뒤 ⚠ · 탭 드래그 간 이동은 메뉴로 대체(잔여).

→ [journal/2026-10-03 §19](journal/2026-10-03.md)

## 10-03 16차 — M3 T-44 설정 창 · 단축키 창

- **한 일**: `prefs_win.rs`(nexa-sql 복사 · 검색/트리/카드/종속 잠금/고급/복사/기하 기억 · 라벨은 i18n 키) · `keys_win.rs`(캡처·충돌) · `app/windows.rs`(열기 펌프 · 사건 분배 · 메인 창 id 가드) · `app/settings.rs` `apply_setting` + **적용 누락 감시 시험**(레지스트리 전 키) · `copybtn.rs` · i18n +19×3 · 시나리오 `prefs-open`. 시험 +5.
- **지금 상태**: M3 🚧 — T-40·41·44·45·46·06 ✅ · T-42·43 부분. 남은 M3 = T-43 잔여(탭 잠금/고정/메뉴 · 열 폭 동기 · 폴더 트리 T-32) · T-42 런처 · T-44 잔여(JSON 편집 · 폴더 찾아보기 T-29). 다음 = T-43 잔여 → M4 플랫폼 층(T-50 포트 trait + Fake).
- **걸린 것**: 설정 창 한글 IME는 nexa-sql 경로 그대로(Windows 실기 확인 필요 · 하네스로는 못 본다).

→ [journal/2026-10-03 §18](journal/2026-10-03.md)

## 10-03 15차 — T-06 `ndir-check` T4 러너 + 시나리오 5

- **한 일**: `crates/ndir-check`(의존 0): `.scn` → 격리 홈·샘플 트리 → 앱 실행 → 종료 코드·패닉·검사식 → 표·`summary.txt`. 시나리오 5(배치 · 위로 선택/히스토리 · 탭/패널 · 세션 저장 · 단언 실패 음성) 전부 PASS(각 0.2~0.3 s). CI에 Windows `scenarios` 단계. 상대 경로 함정(앱 cwd = 트리) 적발·수정.
- **지금 상태**: M3 🚧 — T-40·41·45·46·06 ✅ · T-42·43 부분. **회귀 하네스 T0~T5가 전부 섰다**(T6 성능만 남음). 다음 = T-44 설정 창/단축키 창 → T-43 잔여 → T-05 check-all.
- **걸린 것**: Linux xvfb·macOS 러너 단계(후속 · docs/18 §4).

→ [journal/2026-10-03 §17](journal/2026-10-03.md)

## 10-03 14차 — M3 T-46 기동 명령 어휘 + 패닉 훅

- **한 일**: `NDIR_STARTUP_CMD` 확장(`@ready`·`@idle`·`@after` · `quit[:코드]` · `assert.<대상>:<식>` 실패 = 종료 코드 3 · `ui.click:@영역` · `ui.key`) · 덤프 6종(`panel`·`list`·`tabs`·`status`·`menu`·`layout`) · `crash.rs` 패닉 훅(crash-<unix>.txt · 마지막 명령 id · 다음 기동 안내) · 종료 코드가 프로세스로. 창 실증(exit 5 · assert 실패 exit 3). 시험 59.
- **지금 상태**: M3 🚧 — T-40·41·45·46 ✅ · T-42·43 부분. 다음 = T-44 설정 창/단축키 창(nexa-sql prefs_win·keys_win 복사 · 보조 창 호스트 winhost · `apply_setting`) → T-43 잔여 → T-06 `ndir-check` 러너(이제 `@ready…assert…quit`로 판정 가능).
- **걸린 것**: 없음.

→ [journal/2026-10-03 §16](journal/2026-10-03.md)

## 10-03 13차 — M3 T-45 세션 복원·저장

- **한 일**: `session.rs`(dir2 `session.cfg` 형식 그대로 · `session.conf` + 레거시 읽기 · 원자적 저장 · 미사용 키 보존) · `Panel::restore`(실패 탭 건너뜀 · 보기 모드 · 열 폭) · `App::new(…, start, session)`(실행 인자 우선) · `app/sessions.rs`(더러움 수거 → `SaveScheduler` 1 s/5 s → 틱 저장 · 종료 flush). 시험 +3 · 창 2회 실행으로 저장→복원 실증.
- **지금 상태**: M3 🚧 — T-40·41·45 ✅ · T-42·43 부분. 다음 = T-44 설정 창/단축키 창(nexa-sql prefs_win·keys_win 복사 · `apply_setting`) → T-43 잔여(탭 잠금/고정/메뉴 · 열 폭 동기 · 폴더 트리) → T-46 덤프 어휘.
- **걸린 것**: 없음(펼침 집합 `exp`·잠금/고정 복원은 T-43 잔여와 함께).

→ [journal/2026-10-03 §15](journal/2026-10-03.md)

## 10-03 12차 — M3 T-43 1차: dir2 패널 구조 + 스플리터

- **한 일**: `panel.rs`(dir2 `panel.rs` 핵심 이식 — 패널 = 탭 바 + [홈][←][→][↑] + 경로 바 + 목록 · 탭별 히스토리 · 위로 = 떠난 폴더 자동 선택 · 홈 = 내 PC · 무간섭 재열람) · `nav.rs`(dir2 그대로) · 창 배치 dir2 수치(툴바 28 · 상태 22 · 스플리터 3 · 최소 200) · 스플리터 드래그 = 비율 설정 + 50 % 스냅(Alt 해제) · 열 5(name·ext·size·modified·kind). 골든 재생성 · 시험 52 green.
- **지금 상태**: M3 🚧 — T-40·41 ✅ · T-42·43 부분. 다음 = T-43 잔여(탭 잠금/고정/복제/메뉴 · 패널 간 탭 이동 · 열 폭 기억/동기 · 내 PC 열) → T-44 설정/단축키 창 → T-45 세션(`panel.session()` 준비됨) → T-46.
- **걸린 것**: 패치 스크립트 사고로 `filelist.rs`가 비워졌다 → git 복원(메모리·docs/15 §3-1 등재). 셀 클립(T-31)은 그대로 ⚠.

→ [journal/2026-10-03 §14](journal/2026-10-03.md)

## 10-03 11차 — M3 T-41 창 없는 AppCore + 골든

- **한 일**: `viewport`/`scale` 주입(`layout_for`) · `paint_into(&mut dyn DrawCtx)` · 창 없이도 `layout()` — `cargo test`가 App을 만들어 골든(`tests/golden/layout-1200x800.txt`)·RecordCtx(표면 밖 0)·입력 시나리오·기동 명령 어휘를 0.1 s에 돈다(시험 5 · nexa-dir 43). 적발 3 수정(종류 열 넘침 → 이름 열 흡수 ⚠T-31 · 창 없는 layout 조기 반환 · CI mac/linux 플랫폼 단언).
- **지금 상태**: M3 🚧 — T-40·41 ✅ · T-42/43 부분. 다음 = T-43 잔여(패널별 탭·스플리터 드래그·폴더 트리·열 폭 기억) → T-44 설정/단축키 창 → T-45 세션 → T-46 덤프 어휘(`assert`·`@ready`·패닉 훅).
- **걸린 것**: 직전 CI(51f8c38) mac/linux 빨강 = `tab_title("C:/")` 단언 — 이번 커밋에 수정 포함. Q-8 그대로.

→ [journal/2026-10-03 §13](journal/2026-10-03.md)

## 10-03 10차 — M3 T-40 앱 골격(창이 뜬다)

- **한 일**: nexa-sql 호스트 껍질 복사(`NDIR_*`) · dir2 아이콘 자원 · `App`/`Focus`/`layout`/`ApplicationHandler` · `filelist::TreeSource`(ndir-tree → nexa-grid) · 메뉴 5/툴바 13/명령 한 길 `command(id)` · 키맵 연결 · `NDIR_STARTUP_CMD`(`layout.dump`·`ui.click`·`@after`·`app.exit`). Windows 실증: 격리 홈·비활성 창으로 덤프 → 메뉴 클릭 → F6 → 더블클릭 진입 → 종료 1.5 s · 설정 저장 확인. 게이트 전부 green.
- **지금 상태**: M3 🚧 — T-40 ✅ · T-42/T-43 부분. 다음 = T-41(AppCore/Shell + FakePlatform + 골든) → T-43 잔여(패널별 탭·스플리터 드래그·폴더 트리·열 폭 기억) → T-44 설정/단축키 창 → T-45 세션 → T-46 덤프 어휘.
- **걸린 것**: Q-8(macOS present 설정 노출 — 기본 softbuffer로 진행). 컬럼 폭이 `layout()`마다 초기화됨(T-43에서 기억).

→ [journal/2026-10-03 §12](journal/2026-10-03.md)

## 10-03 9차 — M2 T-26·27 `nexa-explorer`(nexa-ui 106차)

- **한 일**: dir2 PathBar·InfoDock·OverlayBars를 nexa-ui `nexa-explorer` 크레이트로 이식(dir2 시험 25) · nexa-grid DrawCtx 어휘 보강. push 완료 · nexa-sql 빌드 유지.
- **지금 상태**: M2 🚧 — 남은 것 = T-28 Tooltip/Overlay → T-29 nexa-dlg 대화상자 → T-30 소형 컨트롤 → T-31 DrawCtx 보강 → T-32 FolderTree. 핵심 창 골격 컨트롤은 전부 준비됨 → M3 착수 가능.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §11](journal/2026-10-03.md)

## 10-03 8차 — M2 T-25 `nexa-grid`(nexa-ui 105차)

- **한 일**: dir2 가상 행 그리드 엔진 전체를 nexa-ui `nexa-grid` 크레이트로 이식(dir2 테스트 52 green) + `draw::Adapt` + nexa-ctl `Invalidations` 틱 요청. push 완료.
- **지금 상태**: M2 🚧 — 남은 것 = T-26 PathBar → T-27 InfoDock → T-28 Tooltip/Overlay → T-29 nexa-dlg 대화상자 → T-30~32. 그 뒤 M3 앱 골격.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §10](journal/2026-10-03.md)

## 10-03 7차 — M2 착수: nexa-ui 103·104차(T-20~T-24)

- **한 일**(형제 저장소 nexa-ui · push 완료): `InputEvent` 3변형 + 휠 줄 수 · `RecordCtx` · MenuBar 체크/라디오/단축키 열/활성/프로그램 열기 · Toolbar 토글 · TabBar 아이콘/툴팁/가운데 클릭 · **StatusBar 신규**. nexa-ctl 시험 391 · nexa-sql 빌드 유지.
- **지금 상태**: M2 🚧 — 다음 = T-25 `nexa-grid`(dir2 rows/columns/typeahead/fastscroll 이식 · 가장 큰 덩어리) → T-26 PathBar → T-27 InfoDock → T-28·29.
- **걸린 것**: 없음.

→ [journal/2026-10-03 §9](journal/2026-10-03.md) · nexa-ui [journal](../../nexa-ui/docs/journal/2026-10-03.md)

## 10-03 6차 — M1 T-17 명령 표·키맵 → **M1 완료**

- **한 일**: `ndir-settings::commands`(48 명령 · dir2 단축키 + macOS 대응안) · `keymap`(nexa-sql 엔진) · 레지스트리 `key.*` 49 · 시험 7(명령↔키 1:1 · 충돌 0 · dir2 31건 · macOS 13건). 전체 테스트 196.
- **지금 상태**: M1 ✅ → **M2 nexa-ui 보강 착수**(형제 저장소 · T-20 InputEvent → T-21 RecordCtx → T-22 MenuBar → T-24 StatusBar → T-25 nexa-grid …). 각 컨트롤은 nexa-ui 커밋/push 뒤 dir3에서 소비.
- **걸린 것**: 의도된 차이 3(단축키 페이지 신설 · `tab.prev` · F6 단일 표기) — 매트릭스 ⚠ 등재.

→ [journal/2026-10-03 §8](journal/2026-10-03.md)

## 10-03 5차 — M1 T-16 `ndir-license`

- **한 일**: nsql-license 1:1 복제(제품 `nexa-dir` · `NDIR_BUILD_DATE` · Feature 변형 0 = dir2 게이트 0 정책) · 시험 8(타 제품 거부 포함) · 자가 점검 license 5항목 PASS.
- **지금 상태**: M1 남은 것 = T-17 명령 표(`commands.rs`). 그 뒤 M2(nexa-ui 보강 — 형제 저장소 작업).
- **걸린 것**: Q-1(Feature/Tier/Kind 구성)은 기본값(게이트 0)으로 진행.

→ [journal/2026-10-03 §7](journal/2026-10-03.md)

## 10-03 4차 — M1 T-13~15 `ndir-settings`

- **한 일**: nexa-sql 설정 엔진 복사 + dir2 86키 레지스트리(dir2 페이지 순·기본값) + 곁 표(OS_DEFAULTS·DEPENDS·HIDDEN·ADVANCED) + JSON + dir2 가져오기(`import_dir2`) + 자가 점검 `config` 3항목. 시험 +17.
- **지금 상태**: 다음 = T-16 `ndir-license` → T-17 명령 표 → M2(nexa-ui 보강).
- **걸린 것**: 열린 결정 Q-7(성능 거버너 `perf.*` 도입 여부 — dir2에 없음 · 보류).

→ [journal/2026-10-03 §6](journal/2026-10-03.md)

## 10-03 3차 — M1 T-12 `ndir-i18n`

- **한 일**: dir2 언어 자원 3종 임베드 + 빌드 시 파리티 검사(결함 6건 실제 적발 → 수정) + 3-OS OS 언어 감지 + 전역 표(워커 스레드 OK) · 자가 점검 `resources` 실제 항목. 테스트 **166 green** · 3-OS ✓.
- **지금 상태**: 다음 = T-13/14 `ndir-settings`(nexa-sql 엔진 복사 + dir2 키 표) → T-16 `ndir-license` → T-17 명령 표.
- **걸린 것**: 없음(OS 종속 문구 15건은 기능 이식 때).

→ [journal/2026-10-03 §5](journal/2026-10-03.md)

## 10-03 2차 — M1 착수: 코어 5크레이트 이식

- **한 일**: `ndir-core/vfs/tree/ops/term` = dir2 복사 + 크레이트 이름 치환 + lint 적응(Debug 16 · unwrap 6). 테스트 **156 green** · `check-3os.sh` ✓ · CI 1차 커밋 3-OS success.
- **지금 상태**: M1 🚧 — 다음 = T-12(`ndir-i18n`: dir2 `.lang` 3종 + 키 검사) → T-13/14(`ndir-settings`) → T-16(`ndir-license`) → T-17(명령 표).
- **걸린 것**: 없음.

→ [journal/2026-10-03 §4](journal/2026-10-03.md)

## 10-03 1차 — M0 골격 착수

- **한 일**: 형제 저장소 최신화(nexa-ui `df75f5a` ff · nexa-license `4f02524`) + 복원 태그 `baseline/pre-nexa-dir3-2026-10-03` push · nexa-ui 3-OS 검사 통과 · **이식 원장 23문서**(`docs/port/10~51` · ultracode 조사 19건 — 7건은 세션 한도로 중단 뒤 재개 중 사용자 지시로 중지, 산출물은 존재) · 규칙 문서(CLAUDE · 01 · 10 · 15 · 16 · 18) · 현황 4층 · 워크스페이스 + `nexa-dir` bin 뼈대(`--version`·`--smoke`·`--selfcheck`) · CI 3-OS.
- **지금 상태**: M0 🚧 → 다음 = T-05(check-all) → M1 T-10(코어 이식).
- **걸린 것**: 열린 결정 Q-1~Q-6([10 §4](10-decision-record.md)) — 기본값으로 진행 중. 누락 조사 = port/52(nexa-dlg·fs 카탈로그) · 98·99(대조) → T-91.
- **게이트**: 로컬 fmt/clippy/test/smoke/selfcheck → CI 결과는 journal에.

→ [journal/2026-10-03](journal/2026-10-03.md)
