# 15 · 개발 방법론 · 개발 기준 (프로젝트 관리 기준 · 코드 규칙 · OS 분기 · 컨트롤 규칙)

> 이 저장소의 **개발 규칙 SSOT**. 문서·git 규약은 [16](16-doc-git-conventions.md), 빌드·테스트·점검 절차는 [18](18-build-and-test.md), 결정은 [10](10-decision-record.md). 출처 = nexa-dir2 `docs/15` + nexa-sql `docs/61`(핵심 설계·작업 규칙)을 dir3 사정에 맞게 합침(2026-10-03).

## 1. 프로젝트 관리 기준

| 항목 | 기준 |
| --- | --- |
| 목표 | nexa-dir2 `0.22.0`의 **기능·배치·자원을 그대로** 3-OS에서 — "차이"는 의도된 것만([port/90](port/90-verification-matrix.md)에 "의도된 차이"로 등재) |
| 단위 | **수직 슬라이스** = 사용자가 관찰할 수 있는 얇은 끝단(예: "탭 바가 그려지고 클릭으로 전환"). 슬라이스 = 커밋 1개(+ 기록 1 트랜잭션) |
| 순서 | [MILESTONES](MILESTONES.md) M0 → M8 · 각 M 안은 [TODO](TODO.md)의 T-번호 순. 의존 역전 금지(예: 컨트롤이 없으면 먼저 nexa-ui에) |
| 원천 | 기능 = **이식 원장**([port/00](port/00-index.md) · 접두 ID) → dir2 코드 → dir2 문서. 골격 = nexa-sql. 원장에 없는 것을 발견하면 **먼저 원장에 `GAP-NNN`** 등재 후 구현 |
| 완료 정의(DoD) | ① 코드 ② 시험(단위/T3 ≥ 1 · 실기만 가능하면 사유) ③ 검증 매트릭스 행 ④ 게이트 green ⑤ 기록(journal → DEVLOG → STATUS → MILESTONES/TODO) |
| 교차 검증 | 마일스톤 끝마다 **원장 ID 전수 대조**(구현 위치·시험·상태) — 빈칸 = 누락. 결과는 journal + 매트릭스 |
| 자동 진행 | 사용자 개입 최소화 — 묻지 않고 진행: 저장소 내부 파일 · 격리 홈 · 스크래치 · `target/` · 내가 띄운 프로세스 · cargo/git/gh/스크립트 실행. **먼저 묻는 것**: 프로젝트 밖 사용자 자원 · 되돌릴 수 없는 git 작업 · 버전 태그 · 공개 릴리스 · 남이 띄운 프로세스 |
| 보고 | 한글 · 슬라이스마다 중간 보고(완료 + 남은 목록) · 지시 충돌은 "충돌·조정" 절로(나중 지시 우선) |

## 2. 코드 기준

### 2-1. 크레이트 경계(단방향)

```
ndir-core ← ndir-vfs ← ndir-tree          (dir2 이식 · 의존 0 · 3-OS)
ndir-ops · ndir-term                       (dir2 이식 · 의존 0 · 3-OS)
ndir-i18n(의존 0) ← ndir-settings(nexa-conf · nexa-sys) ← ndir-license(nexa-license)
nexa-dir(bin) ← 위 전부 + nexa-ui 7 + winit/softbuffer/wasmi
```

- 라이브러리 크레이트는 **winit·창·OS API를 모른다**(ndir-settings의 `config_dir`만 OS 폴더 규칙 — nexa-conf에 위임).
- 앱 크레이트 = `main.rs`(모듈 선언 · `App` · `Focus` · `layout` · `main`) / `app/*.rs`(`impl App` 조각 — nexa-sql 3층) / `platform/`(DR-5) / `preview/`(플러그인 런타임) / 보조 창 `*_win.rs` / 호스트 껍질(`present` · `winhost` · `wingeom` · `winfocus` · `theme` · `icon` · `input` · `clipboard`).
- **`AppCore` / `Shell` 분리**(DR-10): 상태·라우팅·그리기·명령은 창 없이 동작(`AppCore::new(FakePlatform)` + `route(InputEvent)` + `paint(&mut dyn DrawCtx)`), winit 창·present·OS 사건 변환만 `Shell`.

### 2-2. 컨트롤 규칙(DR-2)

1. 범용 UI는 **nexa-ui에만**. 앱에는 "조립"만(배치·라벨·명령 연결).
2. 없는 컨트롤은 nexa-ui에 추가: `nexa-ctl/src/controls/<이름>.rs` · 위젯 계약(`Widget`/`Control`) · 토큰 사용 · `geom::place_popup` 팝업 규칙 · 헤드리스 시험(`RecordCtx`) · `controls/mod.rs` 등재 · nexa-ui `docs/` 기록. 절차 = [port/50 §5](port/50-ui-core.md).
3. nexa-ui 공개 API는 **추가만**. 바꿔야 하면 nexa-sql도 같은 묶음에서 고치고 커밋 본문에 `영향: nexa-sql | dir3`.
4. 순서 = nexa-ui 커밋 → nexa-ui 테스트 + **nexa-sql 빌드**(`cargo check -p nexa-sql`) → nexa-ui push → dir3 사용.
5. dir2 위젯 이식 시 **배치 수치·동작은 그대로**(DrawCtx 어휘만 어댑트 — 대조표 [port/17](port/17-dir2-rendering-text.md)·[port/14 §3](port/14-dir2-gui-widgets.md)).
6. 포커스 링 ≤ 1 · 마우스는 커서 아래 컨트롤에만 · 팝업은 표면 밖으로 나가지 않음 · 바깥 클릭은 닫고 흘림 · hover는 `IntentFade`(nexa-sql 규칙 계승 — [port/40 §4](port/40-sql-app-skeleton.md)).

### 2-3. OS 분기 규칙(DR-5)

1. `#[cfg(target_os)]`는 **`platform/` 모듈 안에서만**. 그 밖의 분기는 `cfg!(target_os)` 상수 판정(설정 OS 기본값 · 주 조합키)으로 한정하고 `OS_DEFAULTS` 표·`primary_mod()` 같은 한 자리에 모은다.
2. 포트 trait마다 `fake.rs` 구현(기록·주입) — T3 시나리오는 가짜로, 실제 구현은 `--selfcheck`로.
3. 분기 조건 ≥ 2이면 순수 함수로 뽑고 **MC/DC 시험**(조건별 독립 영향 쌍).
4. 미지원은 **조용히 넘기지 않는다**: `Err(Unsupported("…"))` → 상태줄/토스트 한 번 + 로그. 메뉴 항목은 비활성(숨기지 않음 — 배치 유지).
5. 교차 clippy(`check-3os.sh`)를 push 전에 — `cfg` 모듈 경로 실수는 다른 OS에서만 드러난다. `allow(dead_code)` 무조건 금지 · OS별은 `cfg_attr` + 이유.
6. OS별 차이 원장 = [port/19](port/19-dir2-shell-integration.md)(셸·클립보드·DnD·휴지통·감시) · [port/18](port/18-dir2-terminal.md)(PTY·셸) · [port/40 §3](port/40-sql-app-skeleton.md)(창·입력·IME·클립보드) — 새 분기를 만들면 해당 문서 표에 한 줄.

### 2-4. 설정 · i18n 규칙(DR-3 · DR-14)

- 설정 키는 `ndir-settings::REGISTRY`에만 · 라벨/설명 = i18n 키 · 종류(`SettingKind`)가 컨트롤과 검증을 겸함 · 종속·숨김·고급은 곁 표 · 새 키 = 레지스트리 1줄 + i18n 3언어 + `apply_setting` 분기(+ "재시작 필요" 목록) — **적용 누락 감시 시험**이 잡는다.
- 시간 단위: 기본값 ≤ 10 s = `_ms` · 초과 = `_secs`. 크기 `_mb/_kb` · 비율 `_pct` · 색 `_color`.
- 사용자 문자열 리터럴 금지 — `t("key")`. 세 언어 파일 키 파리티는 빌드 검사.
- 구현 상수는 설정 키로(자주 안 바꾸면 `HIDDEN`).

### 2-5. 스레딩 · 통지 · 재그리기(nexa-sql 규약 계승)

- 그리기는 `RedrawRequested`에서만 · 유휴 = `WaitUntil(next)` · 하위 기능은 `tick(now) -> bool` + `next_wake()`만.
- 배경 작업 = `mpsc` + 깨움 클로저 · 진행률 = 원자값 · 취소 = `Arc<AtomicBool>` · 물음(덮어쓰기 확인 등) = 전용 답 채널 + 깃발(모달 중첩 루프 없음 — dir2 `show_buttons` 동기 반환은 상태 기계로 재작성 · WINB 위험 1).
- 세대 번호 가드(전송·감시·터미널·상세·메뉴) · **종결 통지만 재시도**(유실 시 상태 고착 — dir2 교훈).
- 모든 입력 경로는 `finish_input → update_status` 한 길목(dir2 WINB 교훈).

### 2-6. 성능 · 자원

- 지연 로딩이 기본(보이는 것만 · 캐시는 즉시 배정 · 미리 읽기는 유휴 + 상한 + 스위치).
- 부하원(스레드·소켓·디스크·매 프레임·캐시)은 설정 키로 끄거나 상한 — 등재 표는 [18 §6](18-build-and-test.md).
- 측정은 Release · 수치는 journal.

### 2-7. 안전

- 파일 작업은 dir2 `ndir-ops` 규칙 그대로(스테이징 → 덮어쓰기 확인 → 원본 보존 · 실행 취소 기록).
- 시험은 임시 샌드박스 안에서만 · 휴지통/클립보드는 가짜 포트 + opt-in 자가 점검.
- 비밀(토큰·암호)은 꺼내는 즉시 0으로(`take_secret_text` · nexa-ctl 규약) · 로그/덤프에 쓰지 않음.

## 3. 이식 절차(한 기능)

1. 원장에서 ID를 고른다(예 `PANEL-031`) → dir2 원본 코드를 **읽는다**(요약 믿지 않음).
2. 필요한 컨트롤이 nexa-ui에 있는지 확인(없으면 §2-2 절차 먼저).
3. 플랫폼 중립 로직 → 라이브러리 크레이트 또는 `app/`; OS 의존 → `platform/` 포트.
4. dir2 테스트가 있으면 **그대로 이식**(DR-11) + 새 시험(단위/T3).
5. 커밋(`출처: nexa-dir2/crates/…:줄` + 원장 ID) → 매트릭스 행 → 기록.

## 3-1. Windows 개발 PC 함정(실측)

- **Git Bash heredoc이 백슬래시·따옴표를 망가뜨린다**(`\\n` → 실제 개행): 인라인 `python3 - <<EOF`·`perl -pi` 코드 패치는 "치환 2건"이라 보고하면서 파일은 그대로인 유령 현상이 난다(10-03 `.lang` 결함 수정에서 3회 헛돎). → 패치는 **스크립트 파일**(스크래치패드 `.py`)로 쓰고 바이너리 모드로 읽고 쓴 뒤 `grep -c`/`od -c`로 검증.
- 실행 중인 exe가 링크를 막는다 → 내가 띄운 `target/` 아래 `nexa-dir.exe`는 종료하고 진행.
- Git Bash에서 `target\x` 같은 백슬래시 경로 인자는 `targetx`가 된다 → 슬래시로.
- `autocrlf`: `.gitattributes`가 텍스트를 LF로 고정 · `.sh`는 체크아웃도 LF.

## 4. 금지 목록

- `git add -A`/`.` · 자동 태그 push · 강제 push · 사용자 자원 수정.
- 앱 크레이트 안 범용 컨트롤 · `platform/` 밖 `#[cfg(target_os)]` · 문자열 리터럴 UI 문구 · 레지스트리 밖 설정 키 · 자체 타이머/스레드 틱 · 모달 중첩 루프 · 입력 주입(SendKeys 등) · 사용자 클립보드 덮어쓰기 시험 · 픽셀 골든 시험(글꼴 OS 차이).
