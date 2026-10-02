# 10 · 결정 기록 (Decision Record)

> 확정 사항의 단일 원천. 변경은 과거 행을 지우지 않고 **새 행으로 정정**(규칙 = [16 §2-5](16-doc-git-conventions.md)). 큰 결정(대안 비교)은 ADR 문서로 두고 여기 §2에 색인한다.
> 번호는 한 줄로 늘어난다(DR-n). 조사·이식 항목의 접두 ID(`SET-`·`LIC-`…)는 [docs/port](port/00-index.md) 소관.

## 1. DR 표

| # | 일자 | 결정 | 근거 · 출처 |
| --- | --- | --- | --- |
| DR-1 | 10-03 | **올 러스트 · 3-OS 단일 코드.** 창/입력 = `winit 0.30` + `softbuffer 0.4`(Linux Wayland/X11 dlopen) · 그리기 = `nexa-gfx` CPU 래스터 · 컨트롤 = `nexa-ctl`. 관리 런타임·UI 프레임워크·OS 네이티브 컨트롤 금지(macOS 메뉴바도 창 안 자체 그리기 — nexa-sql과 같음) | 사용자 지시 "OS 간 차이 최소화 · 전부 직접 개발" · nexa-sql 골격([port/40](port/40-sql-app-skeleton.md) SKEL-001~) · dir2 DR-1 계승 |
| DR-2 | 10-03 | **컨트롤은 전부 nexa-ui.** dir2 앱 안 컨트롤(`ctl/` 16종)·`nexa-gui/widgets`(rows·dock·pathbar·menubar·tabbar·chrome·overlaybar)는 nexa-ui로 승격하거나 기존 nexa-ctl 컨트롤로 대체. 앱 크레이트에 범용 컨트롤 금지. nexa-ui 공개 API는 **추가만**(nexa-sql 빌드 유지 · 바꿔야 하면 nexa-sql도 같은 묶음에서) | 사용자 지시 · [port/51 §4](port/51-ui-controls.md)(UIK-201~222 추가 후보) · nexa-ui DR-2(앱 도메인 의존 0) |
| DR-3 | 10-03 | **설정 구조 = nexa-sql `nsql-settings` 차용.** `REGISTRY: &[Entry]` 단일 원천 · `settings.conf`(nexa-conf · 변경분만 · 원자 저장) · 곁 표(`OS_DEFAULTS`·`DEPENDS`·`HIDDEN`·`ADVANCED`·`INFO_KEYS`·`RENAMED`·`RESCALED`·`OLD_DEFAULTS`) · 설정 창은 레지스트리에서 생성(검색·트리·카드·Advanced·JSON). **키 이름 = nexa-sql 규칙(`<접두>.<이름>`) · 값·기본값·페이지 구성 = dir2 계승**(예 `ui.theme = dark`). 세션 상태는 `session.conf`로 분리(`Store` + `SaveScheduler`). dir2 `settings.cfg`는 첫 실행에 1회 가져오기(`migrate::import_dir2`) | 사용자 지시 "설정 구조는 nexa-sql 차용" · [port/41 §5](port/41-sql-settings.md) SET-001~137 · [port/31 §5](port/31-catalog-settings-i18n.md) 대응표 |
| DR-4 | 10-03 | **라이선스 = nexa-license(Ed25519 · `nxl1`) · 정책 = dir2 계승.** 전 기능 무료 · 상업 사용은 라이선스 필요(P1~P8) · 기능 게이트 0(`license.gates` HIDDEN 자리만) · 제품 id **`nexa-dir`**(dir2와 같은 제품) · 파일 `nexa-dir.license` · 폴더 `<설정 폴더>/license` → 기기 공용 폴더 · `ndir-license` = `nsql-license` 복제. dir2 X-15의 P-256/CNG 계획은 **폐기**(크로스플랫폼 불가). 진입점 = Help ▸ 라이선스… + About 상태 줄(상태줄 배지는 보류 — dir2 배치 유지) | 사용자 지시 · [port/42 §4·§5](port/42-sql-license.md) LIC-151~197 · dir2 `LICENSE.md` |
| DR-5 | 10-03 | **OS 분기는 `nexa-dir/src/platform/` 한 층.** 포트(trait) = `Shell`(기본 셸 탐지) · `Pty` · `ContextMenuProvider` · `Trash` · `FileClipboard` · `DragSource` · `Watcher` · `Opener` · `MachineId`(nexa-license 내장). 운영 = `windows.rs`/`macos.rs`/`linux.rs` · 시험 = `fake.rs`. 다른 OS·미지원 = `None`/`Err(Unsupported)` → 호출부가 안내. 기본 셸: Windows `pwsh` → `powershell` → `cmd` · macOS/Linux `$SHELL` → `/bin/zsh`(mac) → `/bin/sh` | 사용자 지시(터미널 셸 OS 분기) · [port/40 §5-7](port/40-sql-app-skeleton.md) SKEL-425~432 · [port/44](port/44-sql-ci-test-docs.md) CI-103 |
| DR-6 | 10-03 | **크레이트 접두 `ndir-*`** · 앱 bin **`nexa-dir`**(exe 이름 = 제품명) · **버전 `0.23.0`**부터(dir2 `0.22.0`의 다음 · 제품 연속 · 발급 라이선스 `max_major 0` 호환). 버전 자리 = 루트 `Cargo.toml` 하나 | nexa-ui `nexa-*`와 이름 충돌 회피 · nexa-sql `nsql-*` 선례 · [port/42](port/42-sql-license.md) LIC-152·177 |
| DR-7 | 10-03 | **플러그인 ABI = dir2 바이트 호환**(export `memory`·`nx_meta`·`nx_preview`·`nx_archive` · import `env` 7종 · 버퍼 규약 · 격리 수치). 동봉 `markdown.wasm`·`archive.wasm`은 **무수정 로드**가 회귀 기준. 탐색 경로(관리 설치본 → 드롭인 → 동봉)·매니페스트·sha256·3-OS 빌드 스크립트는 nexa-sql 확장 구조 차용. 용어는 dir2대로 **`plugins/`** | 사용자 지시 "Plugin 크로스플랫폼 변환" · [port/43 §4](port/43-sql-i18n-extensions.md) EXT-401~456 · [port/20](port/20-dir2-preview-plugins.md) |
| DR-8 | 10-03 | **외부 crate 기본 0 지향.** 허용 원장(§3): winit · softbuffer · wasmi · regex-lite(ops · dir2 계승) · 형제 경로 의존(nexa-ui 7 · nexa-license) · OS 바인딩(`windows`/`windows-core` · `objc2*` · `x11rb`). 추가는 §3에 건별 기록 후 | dir2 DR-8 · nexa-ui DR-3 · nexa-sql DR-3 |
| DR-9 | 10-03 | **설정 폴더 = 포터블 우선 계승.** `NDIR_HOME` → exe 옆 `data/`(이미 존재 + 쓰기 가능 + `is_replaced_on_upgrade` 아님) → `nexa_conf::user_config_dir("nexa-dir")`. 배포 채널은 M7에서 확정(후보: Windows 포터블 zip + MSI(+ winget/choco 자산 승계 여부) · macOS pkg/dmg · Linux deb/rpm) | dir2 DR-3(포터블) · [port/41 §5-4 결정 1](port/41-sql-settings.md) · [port/44 §3-4](port/44-sql-ci-test-docs.md) |
| DR-10 | 10-03 | **회귀 하네스 7층 + 자가 점검.** T0 정적(fmt·clippy·3-OS) → T1 단위 → T2 헤드리스 컨트롤(`RecordCtx`) → T3 헤드리스 앱 시나리오(`AppCore` + `FakePlatform` + 레이아웃 덤프 골든) → T4 프로세스 E2E(`NDIR_HOME` + `NDIR_STARTUP_CMD` + `*.dump`) → T5 `--selfcheck`(doctor · `--ci` 부분집합 · Help ▸ 자가 점검 창과 같은 함수) → T6 성능. **검증 매트릭스** [port/90](port/90-verification-matrix.md) = 이식 원장 ID ↔ 구현 ↔ 시험. 패닉 훅 → `<HOME>/crash/<시각>.txt`(dir2 계승) | 사용자 지시 "회귀 테스트 하네스 · 핵심 기능 오류 즉시 확인" · [port/44 §5](port/44-sql-ci-test-docs.md) CI-101~120 |
| DR-11 | 10-03 | **dir2 순수 로직 테스트 그대로 이식**(코어·vfs·tree·ops·term · 약 425개)이 패리티의 1차 증거. 이식으로 기대값이 달라지는 곳은 "dir2 기대값 = dir3 기대값" 고정물로 남긴다 | [port/22](port/22-dir2-ops-core.md) OPS · CI-117 |
| DR-12 | 10-03 | **push 규칙.** 사용자가 "중간중간 commit과 push" 명시 → 슬라이스/마일스톤 완료 + 게이트 green이면 main push(자동). 형제 의존 변경은 nexa-ui → nexa-license → dir3 순. 버전 태그 push는 별도 승인. 복원 지점 태그 `baseline/pre-nexa-dir3-2026-10-03`을 nexa-ui(`c5a9667`)·nexa-license(`f1e282a`)에 생성·push 완료(10-03) | 사용자 지시 · [16 §4](16-doc-git-conventions.md) |
| DR-13 | 10-03 | **다중 에이전트(ultracode)는 조사 단계에만 사용**(10-03 인벤토리 19건). 이후 구현은 단일 세션 순차 진행 — 사용자 "사용량이 너무 많다"(10-03) | 사용자 지시 |
| DR-14 | 10-03 | **i18n = dir2 `.lang` 자원 유지**(en·ko·ja · `@fallback` · 사용자 오버레이 `<HOME>/lang/*.lang`) + nexa-sql 방식의 **컴파일 타임 키 검사**(빌드 스크립트가 세 파일의 키 파리티·자리표시자 검사 → 불일치 = 빌드 실패). 설정 라벨은 i18n **키 문자열**(`Entry.label: &'static str`) — `Msg` enum 생성 단계는 두지 않는다 | [port/41 §5-4 결정 3](port/41-sql-settings.md) · [port/43 §2](port/43-sql-i18n-extensions.md) · [port/31 §2](port/31-catalog-settings-i18n.md) |

## 2. ADR 색인

| ADR | 주제 | 상태 |
| --- | --- | --- |
| (예약) 20-adr-0001-platform-ports | OS 분기 포트 설계(셸 메뉴·휴지통·DnD·PTY) — M4 착수 시 작성 | 📐 |
| (예약) 21-adr-0002-distribution | 3-OS 배포 채널 — M7 착수 시 작성 | 📐 |

## 3. 외부 crate 원장

| crate | 용도 | 라이선스 | 등재 | 비고 |
| --- | --- | --- | --- | --- |
| `winit 0.30` | 창·입력 | Apache-2.0 | DR-1 | nexa-sql·clip·beep과 같은 판 |
| `softbuffer 0.4` | CPU 픽셀 제출 | MIT/Apache-2.0 | DR-1 | Wayland dlopen |
| `wasmi 1.1` | 플러그인 런타임 | MIT/Apache-2.0 | DR-7 | dir2·nexa-sql 동일 판 |
| `wat 1`(dev) | 시험 전용 — WAT 텍스트 → .wasm 조립(격리·브레이커·ABI v2 시험) | Apache-2.0 WITH LLVM-exception | DR-7 | dir2 dev-dependency 동일 · 배포 바이너리에 안 들어간다 |
| `regex-lite 0.1` | 일괄 이름 변경 정규식 | MIT/Apache-2.0 | DR-8 | dir2 `nexa-ops` 계승 |
| `windows`/`windows-core 0.62` | Win32 바인딩(platform/windows만) | MIT/Apache-2.0 | DR-8 | 기능 플래그는 필요한 것만 |
| `objc2` · `objc2-app-kit` · `objc2-foundation` | macOS(platform/macos · Dock 아이콘) | MIT | DR-8 | winit과 같은 판 |
| `x11rb 0.13` | Linux 모달 transient 속성 | MIT/Apache-2.0 | DR-8 | nexa-sql 선례 |
| (dev) `wat` | WASM 격리 시험 조립 | MIT/Apache-2.0 | DR-11 | dir2 dev-dep 계승 |
| 형제 경로 | `nexa-gfx` `nexa-ctl` `nexa-conf` `nexa-font` `nexa-fs` `nexa-dlg` `nexa-sys` · `nexa-license` | PolyForm NC | DR-1·4 | `../nexa-ui` · `../nexa-license` |

## 4. 열린 결정(사용자 결정 대기 — 기본값으로 진행 중)

| # | 쟁점 | 지금 적용 중인 기본값 | 근거 |
| --- | --- | --- | --- |
| Q-1 | 라이선스 `Feature`·`Tier`·`Kind` 구성 | Feature 0개(게이트 없음) · Tier/Kind는 nexa-license 공통 그대로 | dir2에 유료 기능 정의 없음(LIC-155) |
| Q-2 | Windows 배포 채널(포터블 exe/zip·Inno·choco·winget 승계 vs MSI) | 포터블 zip + MSI 둘 다 생성 · winget/choco 제출은 사용자 결정 | [port/44 §3-4](port/44-sql-ci-test-docs.md) |
| Q-3 | 예산 게이트 수치(dir2 exe ≤10 MB · RSS ≤30 MB) | 측정만 하고 게이트는 M7에서 | winit·softbuffer·wasmi 증가분 실측 뒤 |
| Q-4 | macOS/Linux 셸 컨텍스트 메뉴 범위 | 자체 메뉴(열기·연결 프로그램·파일 관리자에서 보기·속성) + "OS 메뉴 열기" 없음 | [port/19](port/19-dir2-shell-integration.md) |
| Q-5 | `kind=org` 라이선스 단독 설치 허용 | nexa-sql과 같이 허용(경고 없음) | LIC-172 |
| Q-6 | 클라우드(OAuth) 기능의 3-OS HTTP 스택 | M6에서 결정(OS 네이티브 API별 vs 최소 crate) | [port/21](port/21-dir2-cloud.md) |
| Q-7 | 성능 거버너(nexa-sql `perf.mode`/`perf.boost` · `nexa-sys` 신호) 도입 여부 | 도입하지 않음(dir2에 대응 기능 없음 · `ndir-settings`에 `perf.rs` 없음) — 부하원이 늘면 재검토 | [port/41](port/41-sql-settings.md) SET-051~053 |
| Q-8 | macOS 화면 제출 경로(nexa-sql `gfx.mac_present` = softbuffer/iosurface) 설정 노출 여부 | 노출하지 않음(`present::set_mode` 미호출 = softbuffer 기본 · dir2에 대응 키 없음) — 맥 실기 계측 뒤 재검토 | [port/40](port/40-sql-app-skeleton.md) §1-10 |
