# CLAUDE.md — Nexa Dir 3 프로젝트 컨텍스트 (이식용 메모리)

> 이 파일은 **다른 PC에서 clone 시 즉시 컨텍스트를 공유**하기 위한 휴대용 프로젝트 메모리다.
> **먼저 읽기:** [docs/STATUS.md](docs/STATUS.md)(현황) → [docs/10-decision-record.md](docs/10-decision-record.md)(결정) → [docs/15-dev-methodology.md](docs/15-dev-methodology.md)(개발 규칙).

## 1. 이 프로젝트는

**Nexa Dir 3**(제품명 그대로 **Nexa Dir** · 제품 id `nexa-dir`) = [nexa-dir2](https://github.com/SosomLab/nexa-dir2)(Windows 전용 · Win32 + 자체 그리기 · 올 러스트 · `0.22.0`)의 **크로스플랫폼(Windows · macOS · Linux) 재구현**.

- **기능·기술구조·컨트롤 배치·자원(아이콘·언어 파일·샘플·플러그인)은 nexa-dir2를 그대로 유지**한다. 바뀌는 것은 ① 창·입력 호스트(Win32 → winit + softbuffer) ② 컨트롤 구현(dir2 자체 ctl/위젯 → **형제 저장소 [nexa-ui](../nexa-ui) 컨트롤**, 없으면 nexa-ui에 추가) ③ 설정 구조(**nexa-sql `nsql-settings` 방식** 차용) ④ 라이선스(**nexa-license** 기반 · 정책은 dir2 계승) ⑤ OS 의존 기능의 `platform/` 분기.
- OS 기본 컨트롤은 쓰지 않는다 — **전부 직접 그린다**(OS 간 차이 최소화 기조 · nexa-ui 계열 공통).
- 원천(SSOT): 기능 = nexa-dir2 코드·문서(로컬 `../nexa-dir2`) · UI/기술 골격 = nexa-sql(`../nexa-sql`) · 컨트롤 = nexa-ui · 라이선스 = nexa-license. 조사 결과는 **[docs/port/](docs/port/00-index.md)** 이식 원장(접두 ID = 교차 검증 체크리스트).
- 조직: **SosomLab** · 개발자: Sangyong Bae · kiros33@gmail.com · 라이선스: **PolyForm Noncommercial 1.0.0**(개인·비상업 무료 · 상업 유료).
- 현 단계: **M7 배포 · M8 교차 검증 진행**(11-05 맥 전체 점검 마감 · main에 GAP-049 맥 복사 무한 재시도 수정 · v0.30.2 = 사용자 결정 대기). 공개 최신 = **v0.30.1**(11-04 패치 · 태그 62c25d3 · GAP-042 Windows RDP 교착 · GAP-043~048 nexa-copy 옵션 · 완전 삭제 끝까지 · 설정 재기록 · 피커 붙여넣기 · release run 41830986654 · Release 본문에 변경 내역 수동 추가(T-206 = 자동화) · choco 0.30.1 Submitted · **winget은 0.29.0에 머묾**(#468006 0.28.1 Portable Defender 오탐 정체 · 재스캔 요청 코멘트 · T-207)). 그 전 = v0.30.0(dff5251) · v0.29.0(75ff1ef) · v0.28.1(68d926b). 현황 · 다음 일 = [docs/STATUS.md](docs/STATUS.md)(원본) · 지난 회차 = [docs/archive/](docs/archive/).
- **알려진 상류 결함 우회(GAP-042 · DR-38)**: winit 0.30.13 Windows `TaskbarCreated` 교착(RDP 재접속·explorer 재시작 때 창 영구 "응답 없음")은 앱 쪽 서브클래스 선처리로 막는다(`platform/wintaskbar.rs` · 창 생성 직후 `guard_taskbar_created` · 정적 가드). **다른 프로젝트(nexa-sql · beep · clip)에 "dir3의 winit 결함 해결 반영" 요청이 오면 [docs/27](docs/27-winit-taskbar-deadlock.md) §4 절차로 이식**한다. winit 0.31 이후로 올리면 제거.

## 2. 확정 결정 (요약 — 전문은 [docs/10](docs/10-decision-record.md))

| # | 결정 |
| --- | --- |
| DR-1 | **올 러스트 · 3-OS 단일 코드** — 창/입력 = winit + softbuffer(nexa-sql 조합) · 그리기 = nexa-gfx CPU 래스터 · 컨트롤 = nexa-ctl. 관리 런타임·UI 프레임워크·OS 네이티브 컨트롤 금지 |
| DR-2 | **컨트롤은 전부 nexa-ui** — dir2 `ctl/`·`nexa-gui/widgets`는 nexa-ui로 승격해 쓴다. 앱 크레이트에 범용 컨트롤을 두지 않는다. nexa-ui 공개 API는 **추가만**(기존 소비자 nexa-sql 빌드 유지) |
| DR-3 | **설정 = nexa-sql 구조 차용**(`REGISTRY` 단일 원천 · `settings.conf` 변경분만 · 곁 표 · 설정 창 자동 생성) · 키 이름은 nexa-sql 규칙 · **값·기본값·페이지 구성은 dir2 계승** · dir2 `settings.cfg` 1회 가져오기 |
| DR-4 | **라이선스 = nexa-license(Ed25519) · 정책 = dir2 계승**(전 기능 무료 · 상업 사용은 라이선스 필요 · 게이트 0) · 제품 id `nexa-dir` · 파일 `nexa-dir.license` |
| DR-5 | **OS 분기는 `platform/` 한 층**(포트 trait + OS별 구현 + 시험용 가짜) — 셸 컨텍스트 메뉴 · 휴지통 · 파일 클립보드 · DnD · 폴더 감시 · PTY/셸(Windows `pwsh` → `powershell` · macOS/Linux `$SHELL` → `/bin/sh`) · 열기 · 기기 ID. 호출부는 OS를 모른다 |
| DR-6 | **크레이트 접두 `ndir-*`**(nexa-ui의 `nexa-*`와 구분 · nexa-sql `nsql-*` 선례) · 앱 bin = `nexa-dir` · 버전은 dir2 뒤를 잇는 **`0.23.0`**(제품 연속 · `max_major 0` 라이선스 호환) |
| DR-7 | **플러그인 ABI는 dir2와 바이트 호환**(`nx_meta`/`nx_preview`/`nx_archive` · import 7종 · 동봉 `markdown.wasm`·`archive.wasm` 무수정 로드) · 탐색 경로·매니페스트·해시·3-OS 빌드는 nexa-sql 확장 구조 차용 |
| DR-8 | 외부 crate 기본 0 지향 — 허용 목록 = winit · softbuffer · wasmi · ab_glyph(nexa-gfx) · ed25519-dalek(nexa-license) · regex-lite(ops) · OS 바인딩(windows · objc2 · x11rb). 추가는 docs/10 §3 원장에 건별 기록 |
| DR-9 | **배포 = 포터블 우선 계승**: 설정 폴더 = `NDIR_HOME` → exe 옆 `data/`(이미 있고 쓰기 가능 · 교체형 설치 자리 아님) → `user_config_dir("nexa-dir")` · 채널은 M7에서 확정(Windows 포터블 zip + MSI · macOS pkg/dmg · Linux deb/rpm) |
| DR-10 | **회귀 하네스 7층**(T0 정적 → T1 단위 → T2 헤드리스 컨트롤 → T3 헤드리스 앱 시나리오 → T4 프로세스 E2E → T5 `--selfcheck` → T6 성능) · 앱은 `AppCore`(창 없음)와 `Shell`(winit)로 나눠 T3가 `cargo test`에서 돈다 · **검증 매트릭스**(이식 원장 ID ↔ 구현 ↔ 시험) = [docs/port/90](docs/port/90-verification-matrix.md) |
| DR-11 | dir2의 순수 로직 테스트(코어·ops·tree·term·vfs · 약 425개)는 **그대로 이식**해 패리티 증거로 삼는다 |
| DR-12 | **push 규칙(사용자 10-03)**: 슬라이스/마일스톤 완료 + 게이트 green이면 main push. 형제 저장소 의존 변경은 nexa-ui → nexa-license → dir3 순. 버전 태그는 별도 승인. 복원 지점 = `baseline/pre-nexa-dir3-2026-10-03`(nexa-ui·nexa-license) |

## 3. 아키텍처 요약 ([docs/01](docs/01-architecture.md))

- 크레이트: `ndir-core` · `ndir-vfs` · `ndir-tree` · `ndir-ops` · `ndir-term`(dir2 rlib 이식 · 플랫폼 중립) · `ndir-i18n`(dir2 `.lang` 자원 + nexa-sql 카탈로그 방식) · `ndir-settings`(nexa-sql 엔진 복사 + dir2 키 표) · `ndir-license`(nsql-license 복제) · `nexa-dir`(bin · winit 호스트 · `app/` 조각 · `platform/` · `preview/` 플러그인 런타임) · 형제 = `nexa-ui/{gfx,ctl,conf,font,fs,dlg,sys}` · `nexa-license`.
- 렌더링: 창 1개 + `RedrawRequested`에서만 그림 · 유휴 = `WaitUntil` · 하위 기능은 `tick(now) -> bool` + `next_wake()` 두 모양만(자체 타이머·스레드 금지).
- 스레딩: UI 스레드 1 + 워커 · 통지 = 공유 상태(원자값 · `Mutex`) 또는 `mpsc` 큐를 `event_loop.rs` 유휴 틱(`WaitUntil` · `*_tick`)이 폴링(`EventLoopProxy` 깨움은 쓰지 않음 · docs/01 §3 · T-173) · 세대 번호 가드(dir2 A-1 계승) · 종결 통지만 재시도.

## 4. 개발 환경 ([docs/18](docs/18-build-and-test.md))

- **이 PC(Windows) = 개발·실행·QA** · 교차 타깃 검사는 `scripts/check-3os.sh`(fmt + 호스트·나머지 두 OS clippy `-D warnings`) · CI(3-OS 매트릭스)가 실행 신뢰 원천.
- 형제 저장소 `../nexa-ui` · `../nexa-license`를 **먼저** pull(path 의존). 새 PC면 세 저장소 `cargo test --workspace`.
- 앱 실기 시험은 **`NDIR_HOME` 격리** + `NDIR_STARTUP_CMD`로 몰고 덤프 파일로 판정(입력 주입·포커스 탈취 금지).

## 5. 작업 규약 (전문 = [docs/15](docs/15-dev-methodology.md) · [docs/16](docs/16-doc-git-conventions.md))

- **답은 예외 없이 한글로.** 코드 식별자·명령·경로·오류 원문만 그대로.
- **확인 · 결정 질문은 Action Item 1개씩 세분화**(사용자 11-01 · 전문 = docs/15 §1): 묶어 묻지 않는다 · 항목마다 배경 · 선택지와 영향 · 권고와 이유를 상세히 · 질문 1개 = 결정 1개(여러 결정 = 여러 질문 · 의존 순서 명시).
- **패키지 게시물과 프로그램 내부 정보는 영어로**(사용자 10-10): 패키지 메타데이터(deb/rpm/MSI/pkg · winget/choco/homebrew 매니페스트 · `.desktop` · `Cargo.toml` description) · 버전/정보(about) 문구 · 동봉 문서(README · LICENSE · NOTICE · 릴리스 노트/CHANGELOG) · 게시 채널의 설명·요약은 **영어**가 원본이다. 한글·일본어는 `Comment[ko]`처럼 현지화 항목으로만 덧붙인다(원본을 대체하지 않는다). UI 문자열은 종전대로 i18n 키(en 원본 · ko/ja 번역). 저장소 안의 개발 문서(docs/ · journal · 커밋 본문)는 한글 그대로.
- **수직 슬라이스 · 단위 = 커밋 1개 · main 항상 green · Conventional Commits**(scope 어휘 = docs/16 §3) · `git add <파일>`만(`-A`·`.` 금지) · 이식 커밋은 **출처 경로 + 이식 원장 ID** 명기.
- **push 전 게이트 = `bash scripts/gate.sh`(단계형 · 사용자 10-04)** — 모든 수정에 전수를 돌리지 않는다. **quick**(fmt + 호스트 clippy — 바뀐 diff에 OS 조건부 `cfg(unix|windows|target_os)`가 있으면 3-OS clippy로 격상(10-18) + 바뀐 크레이트 시험 + `--smoke`)이 기본이고, **full**(3-OS clippy + 전체 시험 + `--smoke` + `--selfcheck --ci` + T4 시나리오)은 ① 이 PC에 전수 기록이 없거나 ② 마지막 전수가 24시간(`NDIR_GATE_FULL_HOURS`)보다 오래됐거나 ③ 핵심 경로(코어 rlib · `platform/` · 설정 엔진 · 라이선스 · 빌드/CI/스크립트 · 시나리오) 또는 형제 저장소가 바뀌었을 때만 자동으로 돈다. **배포 · 태그 · 마일스톤 마감 전에는 `gate.sh full`을 한 번 더.** 빨강이면 push하지 않는다. push 뒤 CI(3-OS 전수) 결과 확인 — quick으로 push했으면 CI가 전수 역할이다.
- **세션 분담 · 세마포어(사용자 11-05 · 11-06 · PC 전역 `~/.claude/CLAUDE.md`)**: 개발(Fable) = 설계 결정 · 새 기능/다층 변경 · 코어/플랫폼/동시성 · 원인 분석 · 릴리스만 / 협업(Opus · Agent 병렬) = 상태 점검 · 기록 · gate/CI/실기 실행 · 간단 수정(≤30줄). 파일마다 기본 소유자 · 남의 파일은 `[SEM 요청]` → `[SEM 승인]` → `[SEM 해제] <해시>` · 협업도 자기 영역은 commit+push. dir3 소유표·예외 = [docs/15 §5](docs/15-dev-methodology.md) · 전역 사본 = `../nexa-sql/docs/claude-global-rules.md`(새 PC = `install-global-rules.sh`).
- **기록 = 한 트랜잭션**: 커밋 → `docs/journal/YYYY-MM-DD.md` → DEVLOG → STATUS → MILESTONES/TODO → (브랜치면) BRANCHES.
- **디스크 예산(사용자 11-02)**: 개발 산출물(저장소·워크트리 `target/` · 벤치 자료 · 게이트 로그 · 편집기 색인)이 수십 GB를 차지하지 않게 관리한다. ① 회차 마감 때 `bash scripts/disk-check.sh`(크기 보고 · 예산 기본 30 GB `NDIR_DISK_BUDGET_GB` · `--trim` = 워크트리 target · `target/perf/bench-*` · tmp · 게이트 로그 · incremental · 교차 타깃 폴더 삭제 · 측정은 느리니 백그라운드) ② 워크트리(`_collab/*` · `*-wt-*`)에서는 **빌드하지 않는다**(target 중복) · 끝난 워크트리는 `git worktree remove` ③ 실기·벤치 자료는 끝나면 스스로 지운다(bench-copy · perf-baseline · T4 샌드박스) ④ Sublime은 저장소의 `nexa-dir3.sublime-project`로 열어 `target/`을 색인에서 제외(전역 `index_exclude_patterns`도 같게 · `*.sublime-workspace`는 커밋하지 않음) ⑤ 측정·기록 = 협업 세션 · 삭제 실행 = 개발 세션(사용자 확인) ⑥ **프로젝트 밖 임시 폴더/파일(사용자 11-03)**: 시험·스크립트·세션이 OS Temp에 만드는 것은 **끝나면 스스로 지우고**, 프로젝트 밖에 둘 때는 반드시 이 프로젝트 접두사(`ndir-*` · `nexa_*` · `nexa-preview*` · `xfer-home-*`)를 쓴다 — 남은 것(패닉한 시험 · 끊긴 세션)은 `--trim` ⑦이 **나이 가드**(고정물 24 h `NDIR_TMP_AGE_H` · 세션 스크래치 `Temp/claude/<프로젝트>/<세션>` 7일)로 거둔다 · 새 접두사를 쓰면 disk-check의 목록에 함께 올린다. 게이트의 여유 5 GB 하한(`NDIR_GATE_MIN_FREE_GB`)은 그대로.
- **기능 설계 전 이식 원장(docs/port) · dir2 원본 코드 먼저 확인**(재발명 금지). 원장에 없는 기능을 발견하면 원장에 `GAP-NNN`으로 먼저 등재.
- **컨트롤 추가는 nexa-ui에**(DR-2): **① 착수 전 최신화**(`git -C ../nexa-ui fetch` → ff-pull · 상류에 같은 기능이 이미 있는지 확인 · nexa-license도 동일) → ② 추가 → nexa-ui 테스트 + nexa-sql 빌드 확인 → **③ 기능 단위 커밋 1개 + 즉시 push**(여러 기능을 모아 두지 않는다 — 다른 PC·세션이 같은 저장소를 쓰며 바꾸고 있을 수 있다 · 사용자 10-10) → ④ dir3에서 사용. nexa-ui 커밋 본문에 `영향: nexa-sql | dir3`.
- **OS 분기는 `platform/`에만**(DR-5) · 분기 판정이 2조건 이상이면 순수 함수 + MC/DC 시험 · 다른 OS = `None`/`Err(Unsupported)`로 돌려주고 호출부가 안내.
- **자식 프로세스는 콘솔 창을 띄우지 않는다**(사용자 11-02): 앱이 내부적으로 띄우는 명령줄 도구(git · 열기/보기 · 미리보기 · 동기화 등)는 `platform::quiet_command`(Windows `CREATE_NO_WINDOW`)로만 만든다 — `std::process::Command::new` 직접 호출 금지. 예외 = 런처(사용자가 고른 프로그램 · 창이 의도) · PTY(ConPTY · 창 없음) · `ShellExecuteW` 열기 · 콘솔 보조 `ndir`. 정적 가드 = nexa-dir 시험 `command_spawn_sites_are_allowlisted`(파일별 호출 수 표 · 새 호출은 표를 고치며 사유를 적는다). 자동 실행(기동 · 셸 탐지 · 상태줄)도 같은 규칙.
- **사용자 문자열은 전부 i18n 키**(dir2 `.lang` 자원) · **설정 키는 `ndir-settings::REGISTRY`에만**(라벨·설명 = i18n 키) · 구현 상수는 설정 키로(자주 안 바꾸면 `HIDDEN`).
- **새 기능 = 시험 1개 이상**(단위 또는 T3 시나리오) + 검증 매트릭스 행 갱신. 실기만 가능한 것은 "실기 필요" 사유를 매트릭스에.
- 시험은 실제 설정 폴더·사용자 파일·클립보드·휴지통을 건드리지 않는다(임시 샌드박스 · 가짜 플랫폼 · opt-in 자가 점검만).
- `.claude/settings.json`은 덮어쓰기 금지, 병합만.

## 6. 새 세션 오리엔테이션

1. 이 CLAUDE.md + [docs/STATUS.md](docs/STATUS.md) → 2. [DEVLOG](docs/DEVLOG.md) 최상단 + 최신 journal → 3. 할 일 = [docs/TODO.md](docs/TODO.md)(M 순서) · 이식 원장 색인 = [docs/port/00-index.md](docs/port/00-index.md).
2. 형제 저장소 `../nexa-ui` · `../nexa-license` · 원본 `../nexa-dir2` · 기준 `../nexa-sql`이 나란히 있어야 한다(path 의존 · 원본 대조).
