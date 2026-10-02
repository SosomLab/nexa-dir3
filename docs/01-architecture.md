# 01 · 아키텍처

> 결정은 [10](10-decision-record.md), 규칙은 [15](15-dev-methodology.md). 기준 골격의 상세 근거 = [port/40](port/40-sql-app-skeleton.md)(nexa-sql) · 원본 구조 = [port/22](port/22-dir2-ops-core.md)·[port/10~12](port/00-index.md)(dir2).

## 1. 저장소 · 크레이트

```
nexa-dir3/
├─ Cargo.toml                 워크스페이스(버전 자리 하나 · lints · 프로필)
├─ crates/
│  ├─ ndir-core/              dir2 nexa-core 이식 — 공용 타입·secret(0 채우기)            [3-OS]
│  ├─ ndir-vfs/               dir2 nexa-vfs — 가상 최상위·압축 스트림(zip/7z/rar/cab/tar)  [3-OS · cfg 4곳]
│  ├─ ndir-tree/              dir2 nexa-tree — 인라인 트리 평면 스트림·교차 선택          [3-OS]
│  ├─ ndir-ops/               dir2 nexa-ops — 전송(진행·취소·충돌)·히스토리·일괄 이름 변경 [3-OS · regex-lite]
│  ├─ ndir-term/              dir2 nexa-term — VT 파서·화면 버퍼                          [3-OS]
│  ├─ ndir-i18n/              dir2 lang/*.lang 임베드 + 키 검사 빌드 스크립트 + OS 언어   [의존 0]
│  ├─ ndir-settings/          nsql-settings 엔진 복사(kind·registry·tree·tables·migrate·store·json·perf) + dir2 키 표
│  ├─ ndir-license/           nsql-license 복제(Product nexa-dir · Feature 0)
│  └─ nexa-dir/               (bin) 앱
│     ├─ src/main.rs          모듈 선언 · App · Focus · layout · main(인자: --smoke --selfcheck --version)
│     ├─ src/app/*.rs         impl App 조각(event_loop·input·paint·windows·menus·toolbar·settings·events·startup_cmd·license·panels·fileops·dock·terminal·preview·clipboard·dnd)
│     ├─ src/core/            AppCore(창 없음 — 상태·라우팅·명령·그리기) ↔ Shell(winit·present)
│     ├─ src/platform/        mod(포트 trait) · windows.rs · macos.rs · linux.rs · fake.rs
│     ├─ src/preview/         플러그인 런타임(dir2 ABI) · archive · 내장 폴백
│     ├─ src/plugins/         매니저·sha256(nexa-sql 차용)
│     ├─ src/*_win.rs         보조 창(prefs·keys·about·license·bulkrename·archive·preview·pwprompt)
│     ├─ src/{present,winhost,wingeom,winfocus,theme,icon,input,clipboard,toast}.rs  호스트 껍질(nexa-sql 복사)
│     ├─ assets/              dir2 assets 그대로(ico·png·toolbar svg·rename png)
│     └─ lang/                dir2 en/ko/ja.lang 그대로
├─ plugins/                   동봉 .wasm(dir2 dist 바이너리 그대로) · sdk/(게스트 SDK + 샘플 소스)
├─ tests/scenarios/*.scn      T4 시나리오 · tests/golden/*.layout  T3 골든
├─ scripts/                   check-3os.sh · check-all · plugin-build.{ps1,sh} · perf-* · capture-*
├─ packaging/                 (M7) windows(rc·msi) · macos(pkg·dmg) · linux(deb·rpm)
└─ docs/                      4층 + port/ 이식 원장
형제: ../nexa-ui(gfx·ctl·conf·font·fs·dlg·sys) · ../nexa-license
```

## 2. 층

| 층 | 내용 | OS 의존 |
| --- | --- | --- |
| 호스트(Shell) | winit 창(메인 + 보조 창 · 모달 = 소유·transient) · softbuffer present · 입력 변환(키·휠·IME) · DPI · 클립보드 텍스트 · Dock/창 아이콘 | winit 추상 + `#[cfg]` 소량(nexa-sql 복사) |
| AppCore | `App` 상태 · `Focus` · `layout()` · `route()` · `paint()` · `menu_action(id)` · 타이머 표 · 워커 수거 | 없음(FakePlatform으로 시험) |
| 영역(위젯 조립) | 메뉴바 · 툴바 · 런처 · 탭바 · 경로바 · 파일 목록(×2 패널 + 스플리터) · 폴더 트리 · 도크(정보/미리보기/터미널) · 상태바 — 전부 nexa-ctl 컨트롤 | 없음 |
| 플랫폼 포트 | 셸 메뉴 · 휴지통 · 파일 클립보드 · DnD 발신 · 폴더 감시 · 열기 · PTY/셸 · 기기 ID | `platform/{windows,macos,linux}` |
| 도메인 | vfs · tree · ops · term · settings · i18n · license · 플러그인 런타임 | 없음 |

## 3. 데이터 흐름(요약)

- 입력: winit 사건 → `Shell`이 `InputEvent`로 변환 → `AppCore::route` → 포커스 영역 → 컨트롤 `on_event` → `take_action` → `menu_action(id)` → **`finish_input → update_status`** 한 길목.
- 그리기: `RedrawRequested` → `AppCore::paint(RasterCtx)` 본문 층 → 팝업 층 → present.
- 배경: 워커(열거·전송·감시·PTY 읽기·플러그인) → `mpsc` + 깨움 → `user_event`에서 `try_recv` 루프 → 세대 가드 → 상태 반영.
- 설정: `REGISTRY` → `Settings`(변경분) → `apply_setting(key)` 조각 → 컨트롤 주입. 세션은 `session.conf`(디바운스 저장).

## 4. 명령 어휘

dir2의 u32 명령 ID(메뉴·툴바·런처·클라우드 대역)는 **문자열 id 표**(`app/commands.rs` · `Command { id, label_key, win, mac, linux, repeatable }`)로 통일 — 메뉴바·툴바·컨텍스트 메뉴·단축키(keymap)·기동 명령·하네스가 같은 어휘를 쓴다. 원장 = [port/30](port/30-catalog-commands-shortcuts.md).

## 5. 예산(측정 — 게이트는 Q-3)

exe 크기 · 유휴 RSS · 100k 항목 첫 렌더 · 입력 지연. dir2 실측(4.04 MB · 16.9 MB · 115 ms)을 기준선으로 journal에 비교.
