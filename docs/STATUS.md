# STATUS — 현황 한 장

> 최신 위. 상세는 [journal](journal/), 요약은 [DEVLOG](DEVLOG.md), 목표 대비는 [MILESTONES](MILESTONES.md) · [TODO](TODO.md).

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
