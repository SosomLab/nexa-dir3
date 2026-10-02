# 23 · nexa-dir2 인벤토리 — 프로젝트 규약 · 라이선스 정책 · 빌드/테스트/점검 하네스 · 배포 · 크로스플랫폼 검토 결론

> 성격: **이해(인벤토리) 단계 산출물** — 읽기 전용 조사 결과. 이 문서의 `PROC-NNN` ID가 이후 구현·교차 검증의 체크리스트다.
> 조사 기준일 2026-10-03 · 원본 = nexa-dir2 `0.22.0`(Windows 전용) · 대상 = nexa-dir3(Windows·macOS·Linux).
>
> **경로 약칭**(모든 근거는 `저장소/경로:줄`): `dir2/` = `nexa-dir2/` · `sql/` = `nexa-sql/` · `ui/` = `nexa-ui/` · `lic/` = `nexa-license/`.
> 긴 경로의 줄임: `sql/docs/33…` = `nexa-sql/docs/33-distribution-and-packaging.md` · `dir2/…/about.rs`·`win.rs`·`config.rs` = `nexa-dir2/crates/nexa-app/src/` 아래 · `plan.md` = `nexa-dir2/docs/audit/20261002-ultracode/plan.md`.
>
> **"추정" 표기**: 저장소의 코드·문서로 확인하지 못한 것에는 "추정"을 붙였다. 특히 §4의 macOS·Linux 패키징/서명 규격 가운데 nexa-sql 저장소에 선례가 없는 항목은 저장소 밖 지식이라 구현 시 실측이 필요하다.
>
> **ID 구간**: 001~034 프로젝트 관리·개발·문서·커밋 규칙 / 040~057 라이선스 정책 / 060~081 빌드·테스트·점검·UI QA / 090~109 배포·패키징 / 120~130 크로스플랫폼 검토(docs/23) 결론.

---

## 0. 범위 — 읽은 파일과 줄 수

### 0-1. 담당 범위(전부 끝까지 읽음)

| 파일 | 줄 | 내용 |
| --- | ---: | --- |
| `dir2/CLAUDE.md` | 67 | 이식용 프로젝트 메모리(정체성·DR 요약·작업 규약·다음 단계) |
| `dir2/README.md` | 59 | 설계 원칙·설치·동봉 플러그인·라이선스 한 줄 |
| `dir2/LICENSE.md` | 139 | PolyForm Noncommercial 1.0.0 영문 정본 + 상단 고지 |
| `dir2/LICENSE.ko.md` | 119 | 비공식 한글 번역 |
| `dir2/docs/15-dev-methodology.md` | 40 | 개발 방법론 |
| `dir2/docs/16-doc-git-conventions.md` | 134 | 문서·커밋/푸시 규약(이식용 표준) |
| `dir2/docs/18-build-and-test.md` | 153 | 빌드·테스트 SSOT |
| `dir2/docs/21-distribution.md` | 429 | 배포 설계·채널 상태 |
| `dir2/docs/23-cross-platform-feasibility.md` | 249 | macOS·Linux 확장 타당성 검토 |
| `dir2/docs/29-audit-checklist.md` | 266 | 정규 점검 규격 |
| `dir2/docs/10-decision-record.md` | 54 | DR·ADR 색인·crate 원장 |
| `dir2/docs/12-packaging-single-exe.md` | 115 | 패키징·서명 조사·Defender 오탐 |
| `dir2/docs/STATUS.md` | 242 | 현황(루트가 아니라 `docs/` 아래에 있다) |
| `dir2/docs/TODO.md` | 143 | 백로그(전 구간 읽음 — 열린 항목은 §1 PROC-022 참조) |
| `dir2/.github/workflows/ci.yml` | 50 | CI |
| `dir2/.github/workflows/release.yml` | 193 | 릴리스 파이프라인 |
| `dir2/.github/workflows/resubmit-chocolatey.yml` | 96 | choco 재제출 |
| `dir2/scripts/audit.ps1` | 162 | 정규 점검 하네스 |
| `dir2/scripts/budget-b3.ps1` | 39 | 임포트 DLL 화이트리스트 게이트 |
| `dir2/scripts/build-plugins.ps1` | 64 | 동봉 플러그인 빌드 |
| `dir2/scripts/ui-qa.ps1` | 173 | UI 자동 조작·캡처 하네스 |
| `dir2/scripts/ctxmenu-timing.ps1` | 78 | 우클릭 메뉴 지연 측정 |
| `dir2/installer/nexa.iss` | 83 | Inno Setup 스크립트 |
| `dir2/.cargo/config.toml` | 21 | 링크 플래그 |
| `dir2/rust-toolchain.toml` | 3 | 툴체인 고정 |
| `dir2/crates/nexa-app/src/about.rs` | 394 | About 창(라이선스 표시) |

### 0-2. 범위 보강을 위해 읽은 것

| 파일 | 줄 | 읽은 범위 · 이유 |
| --- | ---: | --- |
| `dir2/Cargo.toml` | 46 | 전부 — 버전·license 필드·lint·release 프로파일 |
| `dir2/crates/nexa-app/Cargo.toml` · `build.rs` | 28 · 162 | 전부 — 의존 crate·VERSIONINFO·매니페스트 |
| `dir2/packaging/**`(구조 + 핵심 파일) | — | `chocolatey/*/nuspec`·`tools/*.ps1`·`pack-and-push.ps1`(86)·`winget/0.22.0/*`·`winget/portable/0.22.0/*`·`branding/README.md`(119)·`av-false-positive.md`(76) 전부 |
| `dir2/docs/11-dev-environment.md` · `05-requirements.md` | 39 · 53 | 전부 — 개발 모델·예산(NFR)·제약 |
| `dir2/docs/audit/20261002-ultracode/plan.md` · `baseline.md` | 732 · 19 | plan §2-B 머리·§2-E·§3-0·§4·§6·§8, baseline 전부 — 품질 게이트·회귀 시나리오 |
| `dir2/crates/nexa-app/lang/{en,ko,ja}.lang` | — | `about.*`·`menu.help*`·`@` 머리 키만(Grep) |
| `dir2/crates/nexa-app/src/win.rs` | — | About 진입점 152·255·481·5369~5371·9050~9054, panic 후크 1406~1411만 |
| `dir2/crates/nexa-app/src/config.rs` | — | `data_dir`/`choose_data_dir` 358~400만 |
| `dir2/.claude/settings.json` · `.gitignore` | — | 전부 |
| `sql/docs/13-licensing.md`(21) · `33-distribution-and-packaging.md`(129) · `scripts/check-3os.sh`(46) · `scripts/third-party-notices.sh`(13) · `.github/workflows/ci.yml`(54) | — | 전부 — 기준 저장소의 대응 규칙 확인 |
| `sql/crates/nsql-license/src/lib.rs` · `sql/crates/nexa-sql/src/about_win.rs` · `license_win.rs` | 824 · 326 · 565 | 머리 부분만(구조 파악) — 상세는 nexa-sql/nexa-license 인벤토리 문서 몫 |
| `lic/README.md`(76) · `crates/nexa-license/src/types.rs`(1~60) · `crates/nexa-license-tool/src/presets.rs`(51) | — | 제품 id·프리셋 확인 |
| `ui/crates/nexa-ctl/src/controls/*.rs` · `nexa-dlg` · `nexa-conf` · `nexa-fs/shell.rs` | — | 공개 타입 Grep(§3 존재 확인용) |

### 0-3. 확인하지 못한 것

- `dir2/docs/13-licensing.md`: `dir2/LICENSE.md:9`·`dir2/LICENSE.ko.md:9`·`dir2/docs/12-packaging-single-exe.md:43`이 참조하지만 **nexa-dir2 저장소에 없다**(`docs/` 목록에 13번 부재). `sql/docs/13-licensing.md:3`에 따르면 원본은 `../nexa-dir/docs/13-licensing.md`(+ `17-licensing-activation.md`)인데, 로컬에 `D:/Projects/kiros33/nexa-dir` 저장소가 없어 읽지 못했다. → 가격·에디션·체험 기간 같은 **판매 정책 세부는 dir2 저장소로는 확인 불가**(§1 PROC-055).
- `dir2/docs/journal/*`(29개)·`DEVLOG.md`(367)·`BRANCHES.md`(306)·`MILESTONES.md`(127)·`docs/wiki/*`: 형식(머리 규약)만 확인, 본문은 범위 밖.

---

## 1. 기능 목록 — 규칙·정책 항목

이식 분류: **N** = 플랫폼 중립(거의 그대로 계승) · **A** = nexa-ui 컨트롤·그리기로 교체 · **P** = OS별 구현 분기 필요 · **W** = Windows 전용 유지(타 OS는 대체·비활성).

### 1-A. 프로젝트 관리·개발·문서·커밋 규칙 (PROC-001~034)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점(경로:줄) | Win32/OS 의존 | 이식 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-001 | 문서 4층 체계 | 진입(`CLAUDE.md`) / 현황(`STATUS`·`MILESTONES`·`TODO`) / 경과(`DEVLOG`·`journal/YYYY-MM-DD`·`BRANCHES`) / 지식(`NN-주제.md`·ADR) + 색인(`docs/README.md`). 시간축·목적축·상태축을 분리하고 같은 사실은 링크로 잇는다 | `dir2/docs/16-doc-git-conventions.md:46-66` | 없음 | N | — |
| PROC-002 | 문서 작성 필수 규칙 8 | ① journal이 원본·나머지는 요약+링크 ② 진행 기록 전부 시간 역순(같은 날은 "N차") ③ 한 작업 = 한 트랜잭션 갱신 ④ SSOT 지정 + 절차를 바꾼 커밋에서 동시 갱신 ⑤ 결정은 DR 표/ADR, 과거는 지우지 않고 "정정" 명기 ⑥ "왜 + 실측값" ⑦ 기존 자산 우선·출처 경로 명기 ⑧ 문서 번호 불변·신규는 뒤에 append·폐기는 상단에 "폐기 — 대체: NN" | `dir2/docs/16-doc-git-conventions.md:68-77` | 없음 | N | — |
| PROC-003 | 한 작업 = 한 트랜잭션 갱신 | 코드 커밋 → journal 상세 → DEVLOG 한 줄 → MILESTONES/TODO 상태 → (브랜치 작업이면) BRANCHES 표 | `dir2/docs/16-doc-git-conventions.md:19-20,72` · `dir2/CLAUDE.md:44` | 없음 | N | — |
| PROC-004 | SSOT 문서 지정 | 문서·git 규약 = docs/16 · 빌드/테스트 = docs/18(절차 변경 시 같은 커밋에서 갱신) · 정규 점검 = docs/29 · 배포 설계 = docs/21 · 결정 = docs/10 | `dir2/CLAUDE.md:45,48,49` · `dir2/docs/18-build-and-test.md:3` | 없음 | N | — |
| PROC-005 | 결정 기록(DR 표 + ADR) | 확정은 DR-n 표, 대안 비교가 필요한 큰 결정은 ADR 문서. ADR 색인 6건(0001 스택 · 0002 텍스트 렌더 · 0003 셸 컨텍스트 메뉴 · 0004 플러그인 시임 · 0005 wasmi · 0006 클라우드 OAuth) | `dir2/docs/10-decision-record.md:8-28` | 없음 | N(내용은 dir3에서 개정 — §6-1) | — |
| PROC-006 | 문서 번호 불변 | `NN-` 재번호 금지 · 신규는 뒤에 append | `dir2/docs/16-doc-git-conventions.md:25,77` | 없음 | N | — |
| PROC-007 | 커밋 형식 | Conventional Commits `type(scope): 제목`. type = `feat` `fix` `refactor` `docs` `chore` `ci` `test` `release` `merge`. scope 예 = `core/tree` `gui/list` `app/win` `ops` `shell` `term` `pkg` `dist` | `dir2/docs/16-doc-git-conventions.md:79-87` · `dir2/docs/15-dev-methodology.md:16` | 없음 | N(scope 목록은 dir3 크레이트 구성에 맞춰 재정의) | — |
| PROC-008 | 단위 = 커밋 1개 | 수직 슬라이스(관찰·테스트 가능한 얇은 끝단) · 작게·순차로 · 초안 먼저, 확장은 별도 커밋 · 테스트 동반 | `dir2/docs/15-dev-methodology.md:5-9` · `dir2/docs/16-doc-git-conventions.md:84` | 없음 | N | — |
| PROC-009 | 커밋 제목 맥락 태그·본문 | 제목에 `(사용자 요청)` `(사용자 QA)` `(사용자 확정)` 또는 백로그 ID(`X-32`·`M0-4`). 본문에 근거·출처 경로·실측 수치(테스트 수·산출물 크기) | `dir2/docs/16-doc-git-conventions.md:85-86,89-95` | 없음 | N | — |
| PROC-010 | main 항상 green | 테스트·린트 통과 전 병합 금지 | `dir2/docs/16-doc-git-conventions.md:32,87` · `dir2/CLAUDE.md:42` | 없음 | N | CI(`dir2/.github/workflows/ci.yml:10-50`) |
| PROC-011 | 브랜치 규칙 | 큰 단위 = 브랜치(`feat/…` `fix/…` `refactor/…` `docs/…`), 세부 기능 = 커밋. 병합·green 확인 후 로컬 브랜치 삭제, 이력은 `BRANCHES.md` 표(브랜치·생성·병합 커밋·삭제·커밋수·요약·상세) + journal | `dir2/docs/16-doc-git-conventions.md:97-100` · `dir2/docs/BRANCHES.md:1-12` | 없음 | N | — |
| PROC-012 | push 규칙 | **push는 사용자가 명시적으로 요청할 때만**, 자동 push 금지 | `dir2/docs/16-doc-git-conventions.md:37,101` · `dir2/CLAUDE.md:43` | 없음 | N — **dir3는 조정 필요**(§6-1 ①: 이번 사용자 요청이 "중간중간 commit과 push"를 명시) | — |
| PROC-013 | 버전 태그 push는 별도 승인 | 태그 = 공개 릴리스 생성 행위(release.yml이 태그로 발화)이므로 main push와 분리해 다시 확인 | `dir2/docs/16-doc-git-conventions.md:38,102` · `dir2/.github/workflows/release.yml:5-9` | 없음 | N | — |
| PROC-014 | 파괴적 작업 사전 확인 | 파일·브랜치 삭제, reset/revert, force push, 덮어쓰기는 실행 전 확인 | `dir2/docs/16-doc-git-conventions.md:39,103` | 없음 | N | `.claude/settings.json` `ask` 목록(`rm`·`git reset --hard`·`git clean`·`git push --force/-f`·`Remove-Item`) |
| PROC-015 | 일상 작업은 자동 진행 | 사용자 개입 최소화. 상태 기록 md 갱신은 묻지 않고 진행 | `dir2/docs/16-doc-git-conventions.md:40,104` | 없음 | N | — |
| PROC-016 | 권한 설정 파일 병합만 | `.claude/settings.json`은 덮어쓰기 금지·병합만(세션 승인 항목 유실 방지). 현 내용 = `defaultMode: acceptEdits` + allow(git·gh·cargo·rustup·pwsh·python 등) + ask(파괴적 명령) | `dir2/docs/16-doc-git-conventions.md:41,105` · `dir2/CLAUDE.md:47` · `dir2/.claude/settings.json` | 없음 | N(allow 목록에 mac/linux 셸 명령 추가 필요 — `sql/CLAUDE.md:4`의 "OS별 추가분 제안 파일" 규칙 참고) | — |
| PROC-017 | 기존 자산 우선(재발명 금지) | 기능 설계 전 원본 문서·코드 먼저 확인. 이식 커밋 본문에 원본 경로 명기 | `dir2/CLAUDE.md:46` · `dir2/docs/15-dev-methodology.md:14` | 없음 | N — dir3에서 "원본" = nexa-dir2, "기준" = nexa-sql/nexa-ui | — |
| PROC-018 | 스파이크 우선 | 큰 미지수는 버릴 수 있는 실험으로 먼저 검증(M0 렌더 스파이크가 대표) | `dir2/docs/15-dev-methodology.md:9` | 없음 | N | — |
| PROC-019 | 예산 게이트(DR-2) | B1 유휴 RSS ≤ 30MB · B2 exe ≤ 10MB · B3 임포트 = OS 인박스 DLL만. 초과 상태로 main 병합 금지. B4 콜드 스타트 < 200ms · B5 100k 첫 렌더 < 150ms·60fps · B6 백그라운드 CPU ~0%는 목표 | `dir2/docs/10-decision-record.md:11` · `dir2/docs/05-requirements.md:24-35` | B3 = PE 임포트 테이블 | P — 플랫폼별 재정의 필요(PROC-126) | CI B2·B3(`ci.yml:37-46`) · `audit.ps1` B-1/B-2/B-3 |
| PROC-020 | 릴리스 절차 | ① `release: X.Y.Z 승격(요약) — 채널` 커밋으로 버전 동기 ② 승인 후 태그 push → 파이프라인 ③ `docs: X.Y.Z 배포 결과 동기 — 자산·채널 상태` ④ 외부 심사 채널 대기 중이면 보류 방침 문서화 | `dir2/docs/16-doc-git-conventions.md:107-112` | 없음 | N | — |
| PROC-021 | 새 세션 오리엔테이션 | `CLAUDE.md` + `docs/STATUS.md` → DEVLOG 최상단 + 최신 journal → `docs/TODO.md` | `dir2/CLAUDE.md:51-53` | 없음 | N | — |
| PROC-022 | TODO 백로그 형식 | 표 열 = ID · 항목 · 우선(P0~P3) · 규모(소 = 반나절/1커밋 · 중 = 1~2일/1~3슬라이스 · 대 = 3일+/ADR 동반) · 의존 · 상태(☐ 대기 · 🚧 진행 · ✅ 완료(커밋) · ⏸ 보류 · 📐 검토 완료 · 🔶 부분). 새 항목은 §7에 append-only. **0.22.0 시점 열린 항목**: X-1 · X-2(잔여) · X-11 · X-13(1/2) · X-14 · X-15 · X-16(잔여) · X-23(β) · X-24 · X-25(2차) · X-26(①②) · X-33 · X-37(배포 선행 조건) · X-40(실검증) · X-47(2/2) · X-52~X-57 · X-64(배치 5~21) | `dir2/docs/TODO.md:1-5,74-143` | 없음 | N | — |
| PROC-023 | STATUS "한 장" 규약 | 최신 차수 + 직전 N차 스택 + 이전 이력은 하루 한 줄 색인(원문은 journal). 예산 실측 표·마일스톤·다음 단계 포함 | `dir2/docs/STATUS.md:1-8,91-93,208-216` · `dir2/docs/16-doc-git-conventions.md:51` | 없음 | N | — |
| PROC-024 | journal·DEVLOG 형식 | 일자 상세 `docs/journal/YYYY-MM-DD.md`(내부 시간 역순), DEVLOG에는 그날 요약을 맨 위에. 단위 시각 표기 `> ⏱ …(커밋)` = git 커밋 시각(KST) | `dir2/docs/15-dev-methodology.md:19-22` · `dir2/docs/DEVLOG.md:1-6` | 없음 | N | — |
| PROC-025 | 릴리스마다 정규 점검 | `pwsh scripts/audit.ps1`(자동 판정) + docs/29 §3~§8 리뷰 + §9 회차 기록. 발견은 TODO 항목화 | `dir2/CLAUDE.md:49` · `dir2/docs/29-audit-checklist.md:9-22` | 스크립트가 pwsh·PE 전용 | P(하네스 재작성 — PROC-068) | — |
| PROC-026 | 회귀 고정 규칙 | 실측으로 잡은 결함은 **재현 테스트를 먼저 커밋**하고 수정한다. 결함 수정 작업은 자체 회귀 테스트를 같은 커밋에 포함 | `dir2/docs/29-audit-checklist.md:78-79` · `dir2/docs/audit/20261002-ultracode/plan.md:292` | 없음 | N | — |
| PROC-027 | 점검 원칙 4 | ① 실측 없는 "통과"는 쓰지 않는다(UNKNOWN) ② 기준을 바꾸면 규격 문서에서 바꾸고 회차 표에 "기준 개정" ③ 점검용 벤치·픽스처는 저장소에 둔다(`crates/*/examples/audit_*.rs`) ④ 점검 중 발견한 결함 수정은 별도 커밋 | `dir2/docs/29-audit-checklist.md:24-26` | 없음 | N | — |
| PROC-028 | 중립성 규율 | "새 로직은 우선 중립 크레이트에 놓고, 플랫폼 API는 앱 경계 안에서만 호출한다"(docs/15 편입 후보로 제안된 상태) | `dir2/docs/23-cross-platform-feasibility.md:214-219` | 없음 | N — dir3의 1급 규칙으로 승격 권고 | CI core 잡 |
| PROC-029 | 타 OS 경로 검사 + CI 결과 실확인 | Windows `cargo test`가 green이어도 `cfg` 소거 경로는 미검증 → `cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu`(기준 = 오류 0 + 경고 0). `cargo check`만으로는 부족(런타임 차이) → **push 후 CI 3잡 결과를 실제로 확인** | `dir2/CLAUDE.md:38` · `dir2/docs/18-build-and-test.md:22-24,96-114` | 없음 | N — dir3는 3타깃 clippy로 확장(`sql/scripts/check-3os.sh:1-46`) | `audit.ps1` T-3 |
| PROC-030 | 배치(대량 작업) 진행 규칙 | 병렬 레인 = 파일 교집합 없는 작업 / 직렬 레인 = 같은 파일을 건드리는 작업은 순서대로 하나씩 커밋. 작업 1개 = 커밋 1개(커밋 꼬리에 원 발견 ID). 각 배치 끝에서 품질 게이트 Q1~Q9 통과 후 다음 배치. 편집 후 `rustfmt <편집 파일>`만(`cargo fmt -p` 금지) | `dir2/docs/audit/20261002-ultracode/plan.md:319-326` | 없음 | N | PROC-077 |
| PROC-031 | 외부 crate 원장(DR-8) | 외부 crate 기본 0 지향. 추가는 건별로 crate·용도·라이선스·예산 영향·승인일 기록. 현 승인 = `windows` · `windows-core` · `regex-lite` · `wasmi 1.1.0`(dev-dep `wat`). 제거 이력(`starlark`·`anyhow`)도 지우지 않고 취소선 | `dir2/docs/10-decision-record.md:17,30-39` · `dir2/crates/nexa-app/Cargo.toml:11-28` | 없음 | N — dir3는 "0 지향" 문안 개정 필요(§6-1 ③) | — |
| PROC-032 | 개발 환경 모델 | 맥 = 일상 개발(`cargo test` + windows 타깃 `cargo check`) · Windows PC/CI = 실행·QA·예산 실측. CI(windows-latest)가 실행 신뢰 원천 | `dir2/docs/11-dev-environment.md:5-39` · `dir2/CLAUDE.md:34-38` | 실행은 Windows만 | P — dir3는 3-OS 모두 실행 가능해지므로 "어느 OS에서든 개발, 나머지 둘은 clippy 타깃 + CI" 모델(`sql/CLAUDE.md:4`)로 교체 | — |
| PROC-033 | 툴체인·린트 고정 | `rust-toolchain.toml` = stable + rustfmt + clippy. 워크스페이스 공통 린트: `unsafe_op_in_unsafe_fn = allow` · clippy `redundant_clone`/`inefficient_to_string = warn`, 각 크레이트 `[lints] workspace = true` | `dir2/rust-toolchain.toml:1-3` · `dir2/Cargo.toml:24-30` | 없음 | N | clippy(audit T-2) |
| PROC-034 | 자동 모드에서 차단되는 원격 조작 | `gh variable set`·`gh workflow run … confirm=yes`는 자동 모드 권한 분류기가 차단 → 명령을 제시하고 사용자 승인 뒤 실행 | `dir2/CLAUDE.md:64` · `dir2/docs/21-distribution.md:243-246` | 없음 | N | — |

### 1-B. 라이선스 정책 (PROC-040~057)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점(경로:줄) | Win32/OS 의존 | 이식 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-040 | 본체 라이선스 = PolyForm Noncommercial 1.0.0 | 영문 `LICENSE.md`가 정본. DR-6으로 확정(원본 DR-5 계승) | `dir2/LICENSE.md:1-139` · `dir2/docs/10-decision-record.md:15` · `dir2/docs/05-requirements.md:44` | 없음 | N | — |
| PROC-041 | 한글 라이선스본 | `LICENSE.ko.md` = 이해를 돕기 위한 **비공식 번역**, 해석 차이 시 영문본 우선 | `dir2/LICENSE.ko.md:3-9` | 없음 | N | — |
| PROC-042 | 필수 고지(Required Notice) | `Required Notice: Copyright (c) 2026 SosomLab — Nexa Dir` + `(<https://github.com/SosomLab/nexa-dir>)`. 재배포자는 약관(또는 URL)과 이 줄을 함께 전달해야 한다 | `dir2/LICENSE.md:3-4,38-46` · `dir2/LICENSE.ko.md:11-12,35-41` | 없음 | N(제품명은 "Nexa Dir" 그대로 — 저장소 URL은 dir3에서 사용자 확인 필요) | — |
| PROC-043 | 무료 범위 | ① 모든 비상업적 목적 ② 개인적 사용(연구·실험·테스트·학습·오락·취미·아마추어·종교, 상업적 응용을 예상하지 않는 것) ③ 비영리 조직(자선 단체·교육 기관·공공 연구 기관·공공 안전/보건·환경 보호·정부 기관 — 자금 출처 무관) | `dir2/LICENSE.md:60-79` · `dir2/LICENSE.ko.md:54-68` | 없음 | N | — |
| PROC-044 | 유료 범위·문의처 | **상업적 사용은 별도 유료 라이선스 필요.** 문의 = `kiros33@sosomlab.com` · `https://sosomlab.com`. README·nuspec·winget 설명도 같은 문구("free for personal and other noncommercial use … Commercial use requires a separate paid license") | `dir2/LICENSE.md:6-9` · `dir2/README.md:59` · `dir2/packaging/chocolatey/nexa-dir/nexa-dir.nuspec:47-51` · `dir2/packaging/winget/0.22.0/SosomLab.NexaDir.locale.en-US.yaml:36-37` | 없음 | N | — |
| PROC-045 | **기능 제한·정품 인증 없음** | nexa-dir2 코드에는 라이선스 검증·기능 게이트·체험판·등록 화면이 **전혀 없다**. `crates/` 소스에서 `license` 문자열은 About 표시 한 곳뿐(Grep 확인). 즉 현 정책 = "전 기능 무료 사용 가능, 상업 사용자는 약관상 구매 의무" | `dir2/crates/nexa-app/src/about.rs:255`(유일 참조) | 없음 | N — dir3에서 nexa-license를 붙일 때 **게이트 대상 기능 0**이 dir2 계승의 기본값(§6-1 ④) | — |
| PROC-046 | Cargo `license` 필드 | `LicenseRef-PolyForm-Noncommercial-1.0.0`, 전 크레이트 `license.workspace = true` 상속. `authors = Sangyong Bae <kiros33@gmail.com>` · `homepage = https://sosomlab.com` | `dir2/Cargo.toml:15-22` | 없음 | N | — |
| PROC-047 | 앱 내 라이선스·저작권 표시 | About 창 하단 흐린(Dim) 두 줄. `about.license` = en `License: PolyForm Noncommercial 1.0.0` / ko `라이선스: PolyForm Noncommercial 1.0.0` / ja `ライセンス: PolyForm Noncommercial 1.0.0`. `about.copyright` = 3언어 공통 `© 2026 SosomLab · Sangyong Bae` | `dir2/crates/nexa-app/src/about.rs:255-256` · `dir2/crates/nexa-app/lang/en.lang:93-94` · `ko.lang:92-93` · `ja.lang:93-94` | GDI 텍스트 | A | i18n 3종 키 집합 일치 테스트(`dir2/docs/29-audit-checklist.md:196`) |
| PROC-048 | 의존성 라이선스 정책 | 퍼미시브 온리(MIT/Apache/BSD/ISC/MPL-2.0). GPL/AGPL 금지(Slint 배제 근거). 승인 crate는 전부 MIT/Apache-2.0 | `dir2/docs/05-requirements.md:40` · `dir2/docs/10-decision-record.md:15,32-39` | 없음 | N — Linux 원시 계층(X11·Wayland·xkbcommon·HarfBuzz = MIT, FreeType = FTL)도 준수 가능, GTK(LGPL)·Qt 배제(`dir2/docs/23-cross-platform-feasibility.md:159`) | 없음(자동 게이트 부재 — `cargo-deny` 미도입) |
| PROC-049 | 제3자 고지(THIRD-PARTY-NOTICES) | **nexa-dir2에는 제3자 고지 파일·생성 스크립트가 없다**(저장소 전체 Grep — 해당 파일 0). 배포 zip에는 exe + `README.txt`(+ `plugins`)만, 설치형에는 exe + `plugins`만 들어간다. 근거 자료는 docs/10 §1-2 crate 원장뿐 | `dir2/.github/workflows/release.yml:103-111` · `dir2/installer/nexa.iss:70-76` | 없음 | N — dir3는 외부 crate가 크게 늘어나므로 고지 생성 필요. 기준 = `sql/scripts/third-party-notices.sh:1-13`(cargo metadata 목록 → `THIRD-PARTY-NOTICES.txt`), 스테이징 = `sql/docs/33-distribution-and-packaging.md:69` | — |
| PROC-050 | 설치 시 라이선스 동의 | Inno Setup `LicenseFile=..\LICENSE.md` — 설치 마법사에 약관 동의 페이지 | `dir2/installer/nexa.iss:46` | Inno Setup | W(타 OS: pkg/dmg·deb/rpm은 LICENSE 파일 동봉으로 대체 — `sql/docs/33…:69`) | — |
| PROC-051 | Chocolatey 패키지의 라이선스 표기 | `licenseUrl` = 저장소 `LICENSE.md` · `requireLicenseAcceptance=true` · `<copyright>Copyright (c) 2026 SosomLab</copyright>` · 설명에 License 절. **바이너리 미동봉**(PolyForm NC = 비-FOSS → 공식 URL 다운로드가 정석). 다운로드 전용 패키지에는 `tools/VERIFICATION.txt`·`tools/LICENSE.txt`를 넣지 않는다 | `dir2/packaging/chocolatey/nexa-dir/nexa-dir.nuspec:2-6,19-21,47-51` · `dir2/docs/21-distribution.md:147-150,201-208` | Chocolatey | W | — |
| PROC-052 | winget 매니페스트의 라이선스 표기 | `License: PolyForm Noncommercial License 1.0.0` · `LicenseUrl`/`CopyrightUrl` = 저장소 `LICENSE.md` · `Copyright: Copyright (c) 2026 SosomLab` | `dir2/packaging/winget/0.22.0/SosomLab.NexaDir.locale.en-US.yaml:12-15` | winget | W | `winget validate`(수동) |
| PROC-053 | 실행 파일·설치 파일의 저작권 메타 | exe VERSIONINFO: `CompanyName=SosomLab` · `FileDescription`/`ProductName=Nexa Dir` · `InternalName=nexa-dir` · `OriginalFilename=NexaDir.exe` · `LegalCopyright=(C) SosomLab`. 설치형: `VersionInfoCopyright`/`AppCopyright = Copyright (c) 2026 SosomLab` · `AppPublisher=SosomLab` · `AppPublisherURL=https://sosomlab.com` | `dir2/crates/nexa-app/build.rs:48-75` · `dir2/installer/nexa.iss:27-33,53-60` | rc.exe · Inno | P(mac `Info.plist` `NSHumanReadableCopyright` · Linux `.desktop`/deb control — `sql/docs/33…:112-114`) | — |
| PROC-054 | 오프라인 라이선스 인증(X-15) — **설계 등록만, 미구현** | 방향: ① 서명된 텍스트 파일 `license.key`(exe 옆 — 포터블 부합), 페이로드 = 이름·이메일·에디션·발급일·만료·기기 바인딩 옵션(HWID 해시) + ECDSA P-256 서명 ② 앱 내 검증 = Windows CNG(bcrypt.dll — 인박스, 외부 crate 0), 공개키만 exe 임베드 ③ 발급 = 오프라인 CLI 서명 도구(별도 크레이트 `nexa-lic`, 배포 제외, 개인키는 발급 PC 로컬) ④ 위변조 = 서명 검증 실패 시 **미등록 동작**. 착수 시 ADR 작성 | `dir2/docs/TODO.md:96` · `dir2/docs/10-decision-record.md:54` | CNG(Windows 전용) | P — dir3는 `nexa-license`로 대체: 제품 id = `nexa-dir`(`lic/crates/nexa-license/src/types.rs:8`), 서명 = `ed25519` feature(3-OS 공통). `lic/README.md:64`의 "nexa-dir2 → p256 CNG 어댑터" 계획은 Windows 전용 전제라 dir3에는 맞지 않는다 | — |
| PROC-055 | 정책 배경 문서 | `LICENSE.md`가 가리키는 `docs/13-licensing.md`는 nexa-dir2에 없다(원본 nexa-dir 저장소 문서). 가격·에디션 표·체험 기간 등은 **dir2에서 확인 불가**. 계열 공통 방향은 `sql/docs/13-licensing.md:19-21`에만 있다("무료 기능 제한은 두지 않고 상업 사용자에게 라이선스 등록 화면만 제공하는 방향 권장") | `dir2/LICENSE.md:9` · `sql/docs/13-licensing.md:3,19-21` | 없음 | N(추정 포함 — 사용자 확인 대상) | — |
| PROC-056 | 코드 서명 정책 = 무서명 유지 | DR-3 서명 결정(07-15): 무서명 유지 확정, SmartScreen 경고 감수. SignPath Foundation 무료 서명은 **라이선스 때문에 결격**("OSI-approved license without commercial dual-licensing" 요구 — PolyForm NC + 상업 라이선스 이중 위배) | `dir2/docs/10-decision-record.md:12` · `dir2/docs/12-packaging-single-exe.md:28-59` | Authenticode | P(mac 공증 문제 — PROC-128) | — |
| PROC-057 | 동봉 플러그인 샘플의 라이선스 | `samples/*-wasm/Cargo.toml`에 `license` 필드 없음(워크스페이스 밖 독립 크레이트라 상속도 없음). 계열 문서는 "플러그인 SDK는 MIT로 분리 권장"이라고만 적음 | `dir2/samples/markdown-viewer-wasm/Cargo.toml` · `dir2/samples/archive-viewer-wasm/Cargo.toml` · `sql/docs/13-licensing.md:10` | 없음 | N(미결 — 사용자 결정 대상) | — |

### 1-C. 빌드·테스트·점검(audit)·UI QA 하네스 (PROC-060~081)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점(경로:줄) | Win32/OS 의존 | 이식 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-060 | 표준 빌드·테스트 명령 | `cargo test`(전체) · `cargo check --target x86_64-pc-windows-msvc --workspace`(맥에서 Windows 코드 타입 검증) · `cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu`(Windows에서 비Windows 경로) · `cargo run -p nexa-app` · `cargo build --release -p nexa-app` → `target/release/nexa-app.exe` | `dir2/docs/18-build-and-test.md:13-37` | MSVC 링커 | P | — |
| PROC-061 | 릴리스 프로파일 | `opt-level=3` · `lto="fat"` · `codegen-units=1` · `panic="abort"` · `strip="symbols"` | `dir2/Cargo.toml:40-46` | 없음 | N | B-2 |
| PROC-062 | 링크 플래그(Windows) | `x86_64`·`aarch64-pc-windows-msvc` 둘 다: `+crt-static`(재배포 런타임 0) · `control-flow-guard=yes` · `/DEPENDENTLOADFLAG:0x800`(DLL 검색 = System32 한정). **`/CETCOMPAT`는 의도적 제외**(셸 확장 호스트 즉사 — §6-2 ①) | `dir2/.cargo/config.toml:1-21` | MSVC 링커·PE | W(Windows 타깃 블록 그대로 유지. mac/linux는 해당 없음) | `audit.ps1` S-1/S-1b |
| PROC-063 | exe 리소스 임베드 | `build.rs`가 Windows SDK `rc.exe`를 직접 호출(외부 crate 0): 아이콘 ID 1(`assets/nexa-dir.ico`) + VERSIONINFO(Cargo 버전 → `x,y,z,0`) + 매니페스트(RT_MANIFEST 24: `asInvoker`·`uiAccess=false`·`PerMonitorV2`·`longPathAware`·supportedOS Win10/11 · `assemblyIdentity name="SosomLab.NexaDir"`). rc.exe 미발견 시 경고 후 스킵. 타깃 OS가 windows가 아니면 즉시 반환 | `dir2/crates/nexa-app/build.rs:12-131,133-162` | rc.exe · PE 리소스 | P(mac = `.app` 번들 `Info.plist` + `.icns` · Linux = `.desktop` + hicolor 아이콘 — `sql/packaging/{macos,linux}/`. Windows 리소스 삽입 선례 = `sql/packaging/windows/winres.rs`) | `audit.ps1` S-2 |
| PROC-064 | CI(push/PR마다) | 트리거 = 모든 브랜치 push + PR + 수동. **core**(ubuntu·macos, fail-fast 끔): `rustup show` → `cargo test --workspace`. **windows**: `cargo test --workspace` → `cargo build --release -p nexa-app` → `rustup target add wasm32-unknown-unknown` → `build-plugins.ps1 -OutDir target/release/plugins -SkipDist` → B2(>10MB면 throw) → B3(`budget-b3.ps1`) → exe 아티팩트 업로드 | `dir2/.github/workflows/ci.yml:1-50` | windows-latest | P — dir3는 3-OS 동등 매트릭스(fmt·clippy `-D warnings`·test·`--smoke`) + 형제 저장소 나란히 체크아웃(`sql/.github/workflows/ci.yml:17-54`) | 자체 |
| PROC-065 | 예산 측정 방법 | B1: 앱 기동 → 10k 폴더 로드 → 유휴 5분 → `WorkingSet64`, 3회 중앙값 · B2: exe 크기 · B3: `dumpbin /imports` · B4: 기동 로그 타임스탬프 · B5: 코어 벤치 + 실기 스크롤. 결과는 journal 기록 + STATUS 최신값 유지 | `dir2/docs/18-build-and-test.md:143-153` | Get-Process · dumpbin | P | `audit.ps1 -Idle`(약식 60s) |
| PROC-066 | B3 임포트 화이트리스트 게이트 | `dumpbin /imports` 결과가 화이트리스트(22개: kernel32·user32·gdi32·ntdll·oleaut32·dwrite·combase·ole32·bcryptprimitives·shell32·dwmapi·advapi32·imm32·uiautomationcore·gdiplus·shlwapi·winhttp·crypt32·ws2_32·bcrypt·propsys) + `api-ms-win-*` 안인지 검사. 위반 시 throw. 항목 추가는 근거와 함께 커밋 메시지에 남긴다. CI·로컬 공용 단일 출처 | `dir2/scripts/budget-b3.ps1:1-39` | dumpbin(VS Build Tools) | W(Windows 타깃 전용 게이트로 유지 — dumpbin 없이 PE 헤더를 직접 읽는 선례 `sql/scripts/check-imports.ps1`. mac = `otool -L`, Linux = `ldd`/`readelf -d` 대응은 추정) | 자체 |
| PROC-067 | 동봉 플러그인 빌드 | `samples/*-wasm` 각각을 `wasm32-unknown-unknown` release로 빌드 → ① 샘플 `dist\<이름>.wasm`(저장소 동봉본·E2E 테스트가 로드) ② `-OutDir`(배포 스테이징). `-SkipDist` = dist는 그대로. 대상 표 = `$plugins` 배열 2줄(markdown.wasm · archive.wasm). 플러그인을 고치면 `cargo test -p nexa-app preview::sample` 후 `dist/*.wasm`까지 같은 커밋 | `dir2/scripts/build-plugins.ps1:15-64` · `dir2/docs/18-build-and-test.md:63-92` | pwsh(스크립트 언어만) | P(스크립트를 bash/pwsh 겸용 또는 `cargo xtask`로 — wasm 산출물 자체는 플랫폼 중립) | `preview::sample_tests`(`dir2/crates/nexa-app/src/preview/mod.rs:13`) |
| PROC-068 | 정규 점검 하네스 `audit.ps1` | 옵션 `-Quick`·`-Coverage`·`-Idle`·`-BigDir`·`-Exe`·`-OutDir`·`-NoSave`. 판정 항목: **T-1** `cargo test --workspace`(failed=0·passed>0) · **T-2** clippy 경고/오류 0 · **T-3** linux 타깃 check 경고/오류 0 · **T-4** llvm-cov(INFO) · **B-0** release 빌드 · **B-2** exe ≤ 10,000,000B · **B-3** budget-b3 · **S-1** PE DllCharacteristics(DYNAMIC_BASE·HIGH_ENTROPY_VA·NX_COMPAT 필수) · **S-1b** GUARD_CF 없으면 WARN · **S-2** `requestedExecutionLevel=asInvoker` · **P-1** `nexa-vfs --example audit_enum` 중앙값 ≤ max(10ms, n/250 ms) · **P-4** `nexa-term --example audit_vt` ≥ 10MB/s + nasty 1만 건 · `-Idle`: **P-0** 창 표시 ≤ 1500ms · **B-1** WS ≤ 30MB(60s) · **P-5** 10s CPU ≤ 2%. 출력 = 항목별 PASS/FAIL/WARN/SKIP/INFO 한 줄 + 요약, `docs/audit/<yyyyMMdd-HHmmss>/summary.md`·`audit.log` 저장, **종료 코드 = FAIL 개수** | `dir2/scripts/audit.ps1:1-162` | pwsh · PE 헤더 파싱 · `Get-Process` · `MainWindowHandle` | P — 점검 로직(T/P 계열)은 중립, B-3·S-1·S-2·B-1·P-0·P-5 측정부는 OS별 | 자체(회차 `dir2/docs/audit/20260904-174325/summary.md`) |
| PROC-069 | 점검 규격 구조 | 런북 6단계(정적 게이트 ~5분 → 벤치·커버리지 ~15분 → 실기 실측 ~3분 → 정적 코드 리뷰 ~1시간 → 수동 실기 → 기록). 항목 ID 접두 T(테스트)·B(예산)·P(성능)·S(보안)·F(파일 조작)·C(설정)·G(플러그인)·X(견고성)·R(최근 보완 회귀). 회차 폴더 = `README.md`(회차 보고) + `0N-<영역>.md`(조사 원본, file:line 증거 필수) + 로그. 위험도 HIGH/MED/LOW 정의 | `dir2/docs/29-audit-checklist.md:9-57,198-202` | 없음 | N | — |
| PROC-070 | 적대적 입력 테스트(저장소 상주) | X-1 VT 파서 `nasty_sequences_do_not_panic_and_stay_bounded` · X-2 설정 파서 `parse_garbage_is_harmless_and_clamped` · X-3 압축 파싱(zip-slip·잘림·garbage) · X-4 가상 파일 `sanitize_rel` · X-5 플러그인 연료 소진·손상 모듈 스킵 · X-6 복사 서식(HTML 이스케이프·CF_HTML 오프셋·RTF). 보강 대상 = VT fuzz·플러그인 반환 버퍼 경계·메모리 상한·비UTF-8 설정 | `dir2/docs/29-audit-checklist.md:67-76` | X-4·X-6 일부가 Windows 클립보드 형식 | N(X-1·X-2·X-3·X-5) / P(X-4·X-6) | 명시된 테스트 자체 |
| PROC-071 | 플러그인 점검 기준 A1~A30 | 연료 상한·1초 내 복귀·벽시계 타임아웃(1,500ms 구현)·메모리 상한·손상 모듈 스킵·8MB 초과 거부·ABI v1/v2·경계 검사·파일 접근 = 미리보기 대상뿐·암호 비잔존·비활성 플러그인 미실행·콜드 로드 < 50ms·dist = 소스(드리프트 없음)·사용자 사본 우선·출처 검증·서킷 브레이커 | `dir2/docs/29-audit-checklist.md:81-106` | 없음(wasmi는 순수 Rust) | N | 일부(`preview::wasm`·`preview::sample_tests`) |
| PROC-072 | 보안 점검 표 A~G | A 권한(asInvoker·토큰 API 없음·DPI/longPath 선언) · B PE 완화(ASLR·DEP·CFG·CET 의도적 부재·DLL 검색 하드닝·런타임 LoadLibrary 없음·overflow-checks 권장) · C 자식 프로세스(전체 경로 실행·핸들 비상속·`cmd /c` 문자열 조립 없음·런처 인용) · D 비밀(DPAPI·시크릿 미기록·루프백·PKCE·TLS·자동 업데이트 없음) · E 플러그인 · F 파일(zip-slip·임시 폴더·MOTW·링크 추적 없는 재귀) · G 메시지(WM_APP 포인터 인증·`cargo audit`·위협 모델) | `dir2/docs/29-audit-checklist.md:108-134` | A·B·D1~D3·F(MOTW)·G1이 Windows 고유 | P(원칙은 계승, 항목은 OS별 대응 — §4) | 수동 |
| PROC-073 | 파일 조작·설정·성능 점검 질문 | F-1~F-10(워커·진행·오류 격리·덮어쓰기는 새 파일 커밋 후 교체·O(N²) 금지·rename 폴백·완료 통지 유실 방어·동시 작업 명시 거부·undo 상한·휴지통 단일 배치) · C-1~C-8(원자적 저장·dirty 비교·매핑 단일 원천·세션 종료 저장·파서 견고성·다중 인스턴스·데이터 폴더 1회 판정) · P-6a~i(rcPaint 존중·비활성 시 타이머 0·`update_status` 비용·해시 검색·워커 열거·캐시 LRU·활성 탭만 즉시 열거·터미널 런 병합·지표 노출) | `dir2/docs/29-audit-checklist.md:136-176` | 없음(질문은 중립) | N | 수동 |
| PROC-074 | UI 자동 조작·캡처 하네스 `ui-qa.ps1` | 도트 소싱해서 쓰는 함수 모음: `Start-NexaDev`(release exe 기동 — `NO_COLOR` 제거) · `Get-NexaHwnd` · `Get-Title` · `Get-ClientSize` · `Capture-Win`(PrintWindow 플래그 3 = CLIENTONLY + RENDERFULLCONTENT → PNG) · `Click`(-Right·-Double) · `Drag`(8단계 MOUSEMOVE) · `Wheel`(-Shift·-Ctrl·-Horizontal) · `Key`(-Ctrl·-Shift) · `Click-Child`(자식 컨트롤로 좌표 변환) · `Find-TopWindow`(클래스명 — `NexaPrefs`·`#32768`) · `Real-Click`(실제 커서 — 팝업 메뉴 항목 전용) · `Close-Win`(WM_CLOSE = 세션 저장 정상 종료, **kill 금지**) · `Get-NexaStats`(WS/Private/CPU). 원칙 = SendInput이 아니라 **PostMessage**(포커스를 훔치지 않고 가려진 창에서도 동작), 좌표 = 클라이언트 좌표 = 캡처 PNG 픽셀 | `dir2/scripts/ui-qa.ps1:1-173` · `dir2/docs/18-build-and-test.md:39-56` | user32 PostMessage·PrintWindow·EnumWindows | P — **가장 큰 재설계 대상**(§4 PROC-074 행). winit 창에는 타 프로세스 PostMessage 주입이 OS마다 달라 앱 내장 입력 주입/프레임 덤프 훅이 필요(추정) | — |
| PROC-075 | UI 회귀 시나리오 R01~R16 | 기동 프레임 · 행 클릭/Ctrl 다중/재클릭 · 폴더 진입/Alt+←/XBUTTON/탭 바 더블클릭 · F2→우클릭 2회 메뉴→Ctrl+X · 가로 스크롤 후 반대 패널 클릭 · 도크 전환 + 휠 · 싱글 패널/정보 토글 · 터미널 `exit` 후 키 입력 · 드래그 후 ESC · 설정 창(리사이즈·Tab·플러그인 체크) · F3 미리보기·압축 그리드·암호 zip · 타일 보기 첫 프레임 · 경로바 `İ`+`%` 입력 생존 · 테마 전환·DPI 150% 대화상자 · File>Exit 직후 session 파일 · 최소화 65s·복원 | `dir2/docs/audit/20261002-ultracode/plan.md:296-315` | 하네스 의존 | N(시나리오) / P(실행 수단) | 캡처 비교(수동 판정) |
| PROC-076 | 셸 컨텍스트 메뉴 격리 재현기·지연 측정 | `cargo run --release -p nexa-app --example ctxmenu_probe -- <경로>`: 앱과 같은 순서로 `IContextMenu` 취득 → `QueryContextMenu`까지. 종료 0 = 정상 · 0xC0000409 = fast-fail. `scripts/ctxmenu-timing.ps1`: 우클릭 PostMessage 후 `#32768` 팝업이 뜰 때까지 1ms 폴링, 탐색기(Shift+F10·Menu 키)와 비교. 환경변수 `NEXA_CTX_TIMING=1` = 앱 내 계측 | `dir2/docs/18-build-and-test.md:33-36` · `dir2/scripts/ctxmenu-timing.ps1:1-78` · `dir2/crates/nexa-app/src/shellmenu.rs:106-124` | IContextMenu·explorer | W | 자체 |
| PROC-077 | 품질 게이트 Q1~Q9(배치 종료 시) | Q1 `cargo test --workspace` 실패 0 · Q2 clippy 경고 0 · Q3 linux 타깃 check 오류·경고 0 · Q4 커버리지 하락 금지(기준선 전체 39.3%·nexa-app 21.2%·nexa-gui 76.1%) · Q5 `audit.ps1 -Quick` PASS · Q6 예산(exe·유휴 RSS·임포트) · Q7 자동 UI 회귀(R01 + 해당 배치 항목, 기준 캡처와 비교) · Q8 성능(시나리오 5회 평균, 기준선 대비 악화 없음) · Q9 기록(journal·DEVLOG·TODO·STATUS 한 트랜잭션) | `dir2/docs/audit/20261002-ultracode/plan.md:613-627` | Q2·Q5·Q6·Q7이 Windows 도구 | N(게이트 체계) / P(실행 수단) | — |
| PROC-078 | 커버리지 측정 | `cargo-llvm-cov` + `llvm-tools-preview`. 크레이트별로 본다(UI 크레이트가 전체를 끌어내림). 코어(core/vfs/tree/term/ops) ≥ 70% 목표, 1차 실측 89~97% | `dir2/docs/29-audit-checklist.md:61-65,224-226` | 없음 | N | T-4 |
| PROC-079 | 패닉 흔적 남기기(`crash.txt`) | 기동 시 panic 후크를 걸어 패닉 메시지를 `data\crash.txt`에 저장 — "그냥 꺼졌다" 보고의 유일한 단서. 실제로 09-22 경로 바 한글 입력 패닉이 이 파일로 발견됐다 | `dir2/crates/nexa-app/src/win.rs:1406-1411` · `dir2/crates/nexa-app/src/shellpath.rs:19` | 없음(std) | N — dir3 "핵심 기능 오류 즉시 확인" 수단의 1순위 계승 대상 | — |
| PROC-080 | 앱 내 계측 노출 | 제목줄에 `first render`·`avg` paint 시간 표시(성능 베이스라인 측정에 사용) · F3 벤치(200프레임 스크롤) | `dir2/docs/audit/20261002-ultracode/baseline.md:3-19` · `dir2/docs/29-audit-checklist.md:45-46` | 없음 | N | — |
| PROC-081 | 점검용 예제 바이너리 | `nexa-vfs --example audit_enum <dir> 5`(폴더 열거 중앙값) · `nexa-term --example audit_vt`(VT 처리량·견고성) · `nexa-app --example ctxmenu_probe`/`ctxmenu_handlers`/`preview_image` | `dir2/scripts/audit.ps1:99,108` · `dir2/crates/nexa-app/examples/` | ctxmenu_* = Windows | N(audit_enum·audit_vt) / W(ctxmenu_*) | — |

### 1-D. 배포 채널·패키징 (PROC-090~109)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점(경로:줄) | Win32/OS 의존 | 이식 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-090 | 배포 채널 5개 | ① GitHub Release(상시) ② winget `SosomLab.NexaDir`(설치형) ③ winget `SosomLab.NexaDir.Portable` ④ Chocolatey `nexa-dir`(설치형 래퍼) ⑤ Chocolatey `nexa-dir.portable`. 포터블 단일 exe가 **기본 채널**, 설치형이 보조(DR-3 개정 07-16) | `dir2/docs/21-distribution.md:7-15,132-143,253-257,299-315` · `dir2/README.md:20-28` | 전부 Windows | W(기존 채널 유지) + P(mac/linux 채널 신설 — §4) | — |
| PROC-091 | 릴리스 파이프라인 | 트리거 = 버전 태그 push(`0.5.0` 또는 `v0.5.0` 형식) 또는 수동(수동은 게이트+아티팩트까지만). 순서: `cargo test --workspace` → release 빌드 → 플러그인 빌드(`-OutDir plugins -SkipDist`) → B2 → B3 → 포터블 exe 개명 → ISCC 설치형 빌드(`/DAppVersion` `/DExePath` `/DPluginsDir`) → zip 3종 + `README.txt` → `SHA256SUMS.txt` → choco pack → (조건부) choco push → 아티팩트 업로드 → `softprops/action-gh-release@v2`(자동 릴리스 노트). 권한 `contents: write` | `dir2/.github/workflows/release.yml:1-193` | windows-latest · Inno Setup 6 · choco | W + P | 자체 |
| PROC-092 | 릴리스 자산 6종 | `NexaDir-<버전>-win-x64.exe` · `NexaDir-Setup-<버전>.exe` · `NexaDir-<버전>-win-x64.zip`(exe + README.txt + `plugins\`) · `NexaDir-Setup-<버전>.zip`(설치형 + README.txt) · `NexaDir-Plugins-<버전>.zip`(`plugins\` 폴더째) · `SHA256SUMS.txt`(앞 5개의 해시, ASCII, `<hash>  <파일명>`). **기존 exe 자산은 절대 교체하지 않는다**(winget·choco가 URL+SHA256으로 직접 참조) | `dir2/docs/21-distribution.md:60-89` · `dir2/.github/workflows/release.yml:48-54,71-133,184-193` | 없음(명명 규칙) | P(자산명에 OS·아키텍처 축 추가 필요) | `Get-FileHash` 대조(수동) |
| PROC-093 | 포터블 최소파일 규율 | 배포 파일 = exe 1개가 전부(DLL·설정·리소스 동봉 없음, i18n·아이콘 내장). 영속물은 첫 저장 시 exe 옆 `data\` 자동 생성(`settings.cfg`·`session.cfg`·`renames\*.cfg`·`lang\*.lang`). 제거 = exe 삭제. 레지스트리·%APPDATA% 흔적 0 | `dir2/docs/21-distribution.md:17-24` · `dir2/docs/12-packaging-single-exe.md:5-20` | exe 옆 쓰기 | P(mac `.app` 번들·Linux FHS 설치는 exe 옆 쓰기가 부적절 — §4) | PROC-095 테스트 |
| PROC-094 | 설치형(Inno Setup) | `AppId={7E4B1C9D-3A52-4F8E-9B70-6C2D815FA3E1}`(불변 — 업그레이드 연속성) · `AppName=Nexa Dir` · `DefaultDirName={autopf}\Nexa Dir` · `PrivilegesRequired=lowest`(기본 사용자별 설치 = `%LOCALAPPDATA%\Programs`, 관리자 불요) · `PrivilegesRequiredOverridesAllowed=commandline dialog`(choco/winget `/ALLUSERS`·`/CURRENTUSER`) · x64 전용 · `lzma2/max` + solid · `WizardStyle=modern` · 언어 영어만(Korean.isl은 Inno 공식 미포함) · Tasks = 바탕화면 아이콘(기본 해제) · Files = exe를 `NexaDir.exe`로 + `plugins\*.wasm`(없으면 생략) · Icons = 시작 메뉴 + 바탕화면(선택) · Run = 설치 후 실행(무인 설치 시 생략). 제거 시 사용자 데이터 보존 | `dir2/installer/nexa.iss:1-83` · `dir2/docs/21-distribution.md:26-37` | Inno Setup | W | 수동 체크리스트(PROC-109) |
| PROC-095 | 데이터 폴더 판정·폴백 | `data_dir()` = 프로세스당 1회 판정(OnceLock). 후보 = exe 옆 `data\` → 디렉터리 생성 + 쓰기 프로브 성공이면 그대로(포터블·사용자별 설치), 실패하면 `%LOCALAPPDATA%\NexaDir\data`(설치형 폴백), LOCALAPPDATA조차 없으면 후보 유지. 구 경로 `%LOCALAPPDATA%\NexaDir2\data` → 신 경로 rename 마이그레이션(실패 시 구 경로 유지) | `dir2/crates/nexa-app/src/config.rs:358-400` · `dir2/docs/21-distribution.md:39-52` | `%LOCALAPPDATA%` | P — nexa-conf에 대응물 있음(§3) | `choose_data_dir_portable_first_installed_fallback`(`config.rs:1626`) |
| PROC-096 | zip 안내문 `README.txt` | 한/영, UTF-8 BOM. 내용: 무서명 안내("DR-3 무서명 배포 방침") · SmartScreen 경고 문구 · [추가 정보] > [실행] · `Get-FileHash … -Algorithm SHA256` 대조법 · 배포처 URL · 문의 `kiros33@gmail.com`. YAML 안에서 here-string 금지(종료자가 컬럼 0이어야 해 블록 스칼라가 깨짐) → 문자열 배열 + `Set-Content` | `dir2/.github/workflows/release.yml:71-100` · `dir2/docs/21-distribution.md:80-84` | SmartScreen 문구 | P(mac = Gatekeeper 안내로 대체) | — |
| PROC-097 | 동봉 플러그인 배포·탐색 순서 | `markdown.wasm`·`archive.wasm`을 태그의 소스로 CI가 빌드해 싣는다. 포터블 zip = exe 옆 `plugins\` · 설치형 = `{app}\plugins\`(제거 시 함께 삭제) · 단일 exe 자산에는 없음(플러그인 zip으로 보충). 앱 탐색 순서 = ① `data\plugins\`(사용자 설치분) → ② `<exe 폴더>\plugins\`(동봉분·읽기 전용), **같은 id면 ①이 이긴다** | `dir2/docs/21-distribution.md:91-112` · `dir2/crates/nexa-app/src/preview/mod.rs:271-282` | exe 옆 경로 | P(mac = `Contents/Resources/plugins` · Linux = `/usr/share/<앱>/plugins` 대응 — 추정, nexa-sql `Packages/` 배치 선례 `sql/docs/33…:37-44`) | `plugin_dirs_lists_user_first_then_bundled_without_duplicates`(`preview/mod.rs:385`) |
| PROC-098 | Chocolatey 2패키지 | `nexa-dir`: 설치 시 Release의 설치형 exe를 SHA-256 검증 후 내려받아 `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /ALLUSERS`로 무인 설치(머신 전역 → 데이터는 LOCALAPPDATA 폴백), 제거 스크립트는 제거 레지스트리 키를 찾아 무인 실행(데이터 보존). `nexa-dir.portable`: 포터블 exe를 `tools\NexaDir.exe`로 내려받고 shim 자동 등록, `NexaDir.exe.gui` 빈 파일 = GUI 마커. **포터블 패키지는 제거 시 `data\`도 함께 소멸**. `owners=kiros33` / `authors=SosomLab` 분리. `{{VERSION}}`·`{{CHECKSUM64}}` 자리표시자는 CI가 치환(수동 편집 금지), ps1은 UTF-8 BOM | `dir2/packaging/chocolatey/nexa-dir/nexa-dir.nuspec:1-60` · `tools/chocolateyinstall.ps1:1-22` · `tools/chocolateyuninstall.ps1:1-25` · `dir2/packaging/chocolatey/nexa-dir.portable/tools/chocolateyinstall.ps1:1-16` · `dir2/docs/21-distribution.md:132-208` | Chocolatey | W | — |
| PROC-099 | choco 게시 스위치·재제출 | `choco push`는 **시크릿 `CHOCO_API_KEY` + 저장소 변수 `CHOCO_PUSH == 'true'`가 모두 있을 때만**. 미충족 시 pack까지만(nupkg는 아티팩트). `resubmit-chocolatey` 워크플로(수동 dispatch): 빌드 없이 지정 버전의 Release 자산을 내려받아 해시 → 치환(잔여 `{{`·`}}` 검사) → pack → `confirm == 'yes'`일 때만 push. main을 체크아웃하므로 패키징 수정분이 반영된다. 로컬 대안 = `packaging/chocolatey/pack-and-push.ps1`(치환은 사본에만, 원본 복원) | `dir2/.github/workflows/release.yml:135-170` · `dir2/.github/workflows/resubmit-chocolatey.yml:1-96` · `dir2/packaging/chocolatey/pack-and-push.ps1:30-86` | Chocolatey | W | — |
| PROC-100 | winget 매니페스트 | 스키마 1.12.0, 3파일(installer·locale.en-US·version). 설치형: `InstallerType: inno`, user(`/CURRENTUSER`)·machine(`/ALLUSERS`, `elevatesSelf`) 두 스코프, `ProductCode: '{AppId}_is1'`, `MinimumOSVersion: 10.0.17763.0`. **`DisplayVersion`은 `PackageVersion`과 같으면 넣지 않는다**(반려 이력). 포터블: `InstallerType: portable` + `PortableCommandAlias: nexadir`, 경로 `…/NexaDir/Portable/<버전>/`. 저장소 사본 = `packaging/winget/<버전>/`·`packaging/winget/portable/<버전>/`. 제출은 **수동**(포크 브랜치 + Contents API → PR). 체크섬은 `SHA256SUMS.txt`에서 가져오고 자산을 다시 내려받아 재대조 | `dir2/packaging/winget/0.22.0/*.yaml` · `dir2/packaging/winget/portable/0.22.0/*.yaml` · `dir2/docs/21-distribution.md:253-339,420-429` | winget | W | `winget validate`(수동) |
| PROC-101 | 릴리스 시 채널 제출 규칙(상시) | 릴리스마다 채널의 배포 요청 상태를 **원천 실측**: 대기 중인 버전이 없으면 그 버전을 제출, 대기 중이면(PR OPEN / 모더레이션 미승인) 그 채널은 **이번 제출에서 제외**하고 해소 후 그 시점 최신 버전만 제출(중간 버전 생략). 판정 원천 = winget `gh pr view`/`gh search prs` · choco OData `Packages(Id='…',Version='…')` 직접 조회(`Submitted` = 대기, `Approved` = 해소). 결과는 journal·채널 표에 기록 | `dir2/docs/21-distribution.md:401-418` · `dir2/CLAUDE.md:62` | 없음(절차) | N | — |
| PROC-102 | 서명 경로 조사 결론 | Microsoft Store(MSIX)만 경고를 완전히 없앤다(무료·한국 가용, MSIX여야 함) · Azure Artifact Signing은 한국 불가 · SignPath 결격 · OV 연 $150~300 + HSM · EV 즉시 통과는 2024년 폐지. 서명해도 초기 경고는 뜬다(서명 = 버전 간 평판 승계). 현 방침 = 무서명 + zip 자산으로 다운로드 단계만 완화 | `dir2/docs/12-packaging-single-exe.md:33-59` · `dir2/docs/21-distribution.md:60-89` | Authenticode·SmartScreen | W + P(mac 공증 — PROC-128) | — |
| PROC-103 | Defender ML 오탐 대응 | 증상 = 설치형만 다운로드 직후 `Trojan:Win32/Wacatac.*!ml` 격리. 원인 = 무서명 + 새 해시 + 프리밸런스 0인 다운로드 시점 클라우드 판정. 유효 조치 = ① **설치형 VERSIONINFO 채우기**(`0.18.1`에서 해소 확인) ② WDSI 오탐 신고(문안 템플릿 보관) ③ 사용자 안내는 포터블 우선. **압축 완화는 기각**(실측: 알고리즘 변경은 엔트로피 불변, 무압축은 크기 +81%에 효과 미미) | `dir2/docs/12-packaging-single-exe.md:61-111` · `dir2/packaging/av-false-positive.md:1-76` · `dir2/installer/nexa.iss:48-60` | Defender | W | 릴리스 체크리스트(수동) |
| PROC-104 | 버전 동기 지점 | 릴리스 승격 커밋이 함께 바꾸는 곳: `Cargo.toml` `[workspace.package] version` · `Cargo.lock` · 언어팩 3종 머리 `@app = <버전>` · (제출 시) `packaging/winget/<버전>/` 사본. exe VERSIONINFO·About 표시(`env!("CARGO_PKG_VERSION")`)·매니페스트 `assemblyIdentity version`은 Cargo 버전에서 파생. 설치형은 태그명에서 `/DAppVersion` 주입. **태그 ≠ Cargo 버전 검사는 없다**(nexa-sql은 release.yml meta 잡이 대조 — `sql/docs/33…:102`) | `dir2/Cargo.toml:16-17` · `dir2/crates/nexa-app/lang/en.lang:7` · `dir2/crates/nexa-app/build.rs:37-44` · `dir2/crates/nexa-app/src/about.rs:241` · `dir2/docs/STATUS.md:7` | 없음 | N — dir3는 태그·Cargo 버전 대조 게이트 추가 권고 | — |
| PROC-105 | 제품 식별자 | 제품명 `Nexa Dir`(저장소명만 nexa-dir2) · exe 설치명 `NexaDir.exe` · 데이터 폴더명 `NexaDir` · winget 별칭 `nexadir` · Inno `AppId` GUID · 매니페스트 `SosomLab.NexaDir` · winget `SosomLab.NexaDir`(+`.Portable`) · choco `nexa-dir`(+`.portable`) · 제품 홈 `https://sosomlab.com/apps/nexa-dir/` · 조직 `https://sosomlab.com` · 지원 `kiros33@gmail.com` · 라이선스 문의 `kiros33@sosomlab.com` | `dir2/installer/nexa.iss:1-2,27-33` · `dir2/crates/nexa-app/build.rs:6-7,86` · `dir2/packaging/branding/README.md:30-38` | 없음 | N(식별자 유지 여부는 사용자 확인 — 같은 `AppId`·패키지 ID를 쓰면 dir3가 dir2를 대체 업그레이드) | — |
| PROC-106 | 브랜딩 자산·클라우드 콘솔 문구 | `packaging/branding/` = 3사 개발자 콘솔(Dropbox·Entra·Google)에 넣는 앱 정보·아이콘의 SSOT(`nexa-dir-64.png`·`nexa-dir-256.png` — `nexa-dir.ico`에서 무손실 추출). 영문 Description·권한 사유 표·콘솔별 입력 위치 | `dir2/packaging/branding/README.md:1-119` | 없음 | N(자원 그대로 사용) | — |
| PROC-107 | 릴리스 노트·위키 | GitHub Release 본문 = 앱 페이지용 한/영 노트(자동 릴리스 노트 + 수기). 릴리스마다 위키(`docs/wiki/*` → GitHub Wiki) 해당 쪽 갱신·발행 | `dir2/docs/STATUS.md:29-32,55-58` · `dir2/.github/workflows/release.yml:193` | 없음 | N | — |
| PROC-108 | arm64 대응 | `aarch64-pc-windows-msvc` 타깃 추가로 대응 가능(코드 변경 불요 전망), 수요 확인 후 CI 매트릭스에 추가. `.cargo/config.toml`에는 aarch64 블록이 이미 있다 | `dir2/docs/12-packaging-single-exe.md:113-115` · `dir2/.cargo/config.toml:16-21` | — | P(mac은 Universal 2 선례 — `sql/docs/33…:52`) | — |
| PROC-109 | 배포 검증 체크리스트(실기) | 포터블 `data\` 생성·영속 · 사용자별 설치 → exe 옆 `data\` · 관리자 설치 → LOCALAPPDATA 폴백 · 제거 후 데이터 보존·재설치 복원 · 자산 6종 첨부 · zip 해제 후 `plugins\` 인식 · 설치형 `{app}\plugins` 동봉·제거 · zip 다운로드 비차단 · **설치형을 브라우저로 내려받아 Defender 격리 여부 확인** · `README.txt` 한글 표시 · `Get-FileHash` 일치 · `choco install/uninstall` | `dir2/docs/21-distribution.md:114-130` | Windows | W + P(OS별 설치→스모크→제거→잔여 0 자동화 선례 `sql/docs/33…:72`) | 수동 |

### 1-E. 크로스플랫폼 검토(docs/23) 결론 (PROC-120~130)

| ID | 기능(사용자 관점) | 동작 상세(조건·예외·기본값) | 진입점(경로:줄) | Win32/OS 의존 | 이식 분류 | 기존 테스트 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-120 | 결론 요약 | "기술적으로 가능하다. 다만 '이식'이 아니라 **UI 셸 재작성**이다." 비용 = 코어 재사용 + 창/입력/텍스트/셸 통합 신규 구현. 가장 큰 제약은 기술이 아니라 확정 결정(DR-1·2·8). 기능 패리티는 100%가 될 수 없다 | `dir2/docs/23-cross-platform-feasibility.md:12-31` | — | — | — |
| PROC-121 | 중립/결합 비율(0.11.0 시점 실측) | 전체 40,041 LOC 중 중립 16,102(40.2%) / Windows 결합 23,939(59.8%). 크레이트별: `nexa-core`·`nexa-tree`·`nexa-ops`·`nexa-term`·`nexa-gui` = `windows::` 참조 0 · `nexa-vfs` = cfg 분기 1파일 · `nexa-app` = 46파일 중 38파일 결합. nexa-app 안 중립 = `panel.rs`·`config.rs`·`svg.rs`·`i18n.rs`·`pathinput.rs`·`nav.rs`·`main.rs`·`icons/mod`(4,204 LOC). 맥에서 6크레이트 147 테스트 green | `dir2/docs/23-cross-platform-feasibility.md:37-74` | — | N(해당 크레이트·모듈) | CI core 잡 |
| PROC-122 | 화면 로직의 접합면 = `DrawCtx` | 파일 목록 본체(`rows.rs`)·경로 바·크롬·도크·탭 바·메뉴 바가 `nexa-gui`(중립)에 있고 `DrawCtx` 트레이트(~10 메서드)에만 의존 → 플랫폼별 `DrawCtx` 구현 하나로 주 화면이 그려진다 | `dir2/docs/23-cross-platform-feasibility.md:76-91` | — | A(dir3는 사용자 지시에 따라 nexa-ui 컨트롤로 교체 — nexa-gui 위젯을 그대로 쓸지는 다른 인벤토리 문서 14의 판단 대상) | — |
| PROC-123 | 재작성 대상(UI 셸 약 19.5K LOC) | `win.rs`(창·메시지 루프·레이아웃·입력 라우팅) · `ctl/*` 18종 HWND 커스텀 컨트롤(**핵심 비용**) · `prefs.rs` · `bulkrename.rs` · `dialog.rs` · `ordereditor.rs`/`about.rs`/`tip.rs`/`ctldemo.rs` · `dw.rs`(DirectWrite) | `dir2/docs/23-cross-platform-feasibility.md:101-116` | HWND·DirectWrite | A | — |
| PROC-124 | OS 통합 대체 설계 대상(약 3K LOC) | 셸 컨텍스트 메뉴(**기능 상실 → 자체 메뉴로 대체**) · 접근성(UIA → NSAccessibility / AT-SPI) · DnD(OLE → NSDraggingSource / XDND·wl_data_device) · 클립보드(CF_HDROP → NSPasteboard / X11 selection·Wayland) · 터미널(ConPTY → `forkpty(3)` — 오히려 단순) · 휴지통(SHFileOperationW → `NSFileManager.trashItem` / XDG Trash 스펙 직접 구현) · 파일 감시(RDCW → FSEvents·kqueue / inotify) · 셸 아이콘(SHGetFileInfoW → `NSWorkspace.icon(forFile:)` / XDG icon theme) · 특수 폴더 · 프로그램 실행(ShellExecuteW → `NSWorkspace.open` / `xdg-open`) · 앱 아이콘/리소스 | `dir2/docs/23-cross-platform-feasibility.md:118-132` | 표 그대로 | P / W(셸 컨텍스트 메뉴) | — |
| PROC-125 | 모델 층에서 새로 생기는 문제 | ① 드라이브 vs 마운트: 가상 최상위 `::PC::` 센티널과 `C:\` 절대 경로 모델을 `/Volumes`·`/media`·`/mnt`·`/run/media`로 재해석 ② 경로 의미론: 구분자·대소문자 구분(APFS 기본 비구분 / ext4 구분 / NTFS 비구분)·심링크·허용 문자·숨김 규칙(`.` 접두 vs 속성 비트) → 대소문자 정렬 규약 재정의 ③ 글리프: Segoe MDL2 Assets는 Windows 전용 → SVG 자산으로 이전(`svg.rs` 파서가 이미 있음) ④ UI 관례: 메뉴 바 위치·수식자(Ctrl↔⌘)·창 버튼·스크롤 방향 | `dir2/docs/23-cross-platform-feasibility.md:134-149` | — | P | — |
| PROC-126 | 확정 결정과의 충돌 | DR-1(프레임워크 금지): mac은 조건부 성립, **Linux는 문안대로면 성립 불가**. DR-2(인박스 DLL만): Linux에는 "인박스" 개념이 없음 → 게이트 재정의. DR-6(퍼미시브 온리): 원시 계층으로 가면 준수 가능. DR-8(crate 0 지향): 대폭 개정 필요. → 확장을 결정하는 순간 DR-1·2·8 개정 ADR 필요(문서상 "ADR-0005(가칭)"라 적었으나 그 번호는 이후 wasmi가 사용 → CLAUDE.md는 **ADR-0007**로 지정) | `dir2/docs/23-cross-platform-feasibility.md:153-166` · `dir2/CLAUDE.md:67` | — | — | — |
| PROC-127 | 선택지 비교와 현실 후보 | A 현행 유지 / B 자체 백엔드 전면 구현 / **C 얇은 크로스 crate 채택**(`winit` + `softbuffer` + 텍스트 스택 위에 커스텀 드로잉 유지) / D 코어만 재사용. "현실적 후보는 C" — 커스텀 드로잉 정체성을 유지하면서 창·입력·텍스트만 퍼미시브 crate로 조달. 예산은 증가 확실(재측정·게이트 재설정 필요) | `dir2/docs/23-cross-platform-feasibility.md:170-188` | — | — (dir3의 기준 스택 nexa-sql = winit + softbuffer + nexa-ui 이므로 **C안과 일치**) | — |
| PROC-128 | 결정 시 함께 필요한 5건 | ① DR-1·2·8 개정 ADR ② 예산 재정의(B3의 플랫폼별 등가 규칙, exe·RSS 게이트를 플랫폼별로 둘지) ③ 패리티 정책(대응물 없는 기능의 처리 — 기능 축소 / 자체 메뉴 대체 / 플랫폼별 기능 표 공개) ④ 배포 채널 설계(mac `.app` + dmg + **공증** — 무서명 방침과 정면 충돌, Linux AppImage/deb/rpm/Flatpak 선택) ⑤ QA 환경(실기 QA 부담 3배) | `dir2/docs/23-cross-platform-feasibility.md:192-206` | — | — | — |
| PROC-129 | 권고 3단계 | ① 중립성 회귀 방지(비용 0 — PROC-028) ② 맥 렌더 스파이크(1~2주, "`rows.rs` 무수정으로 맥 창에 파일 목록을 그릴 수 있는가", 측정 = 첫 렌더·스크롤 프레임·유휴 RSS·바이너리 크기) ③ 그 결과로 판단. **macOS 먼저, Linux는 그다음** | `dir2/docs/23-cross-platform-feasibility.md:210-238` | — | — | — |
| PROC-130 | 미해결 질문 5건(검토 당시) | ① 동기 ② 어느 플랫폼이 먼저 ③ 패리티를 어디까지 포기 ④ 무서명 방침 유지 여부(mac 배포는 공증 없이는 사실상 불가능) ⑤ 실기 QA 3배 부담 체제. dir3 착수로 ①②는 사용자 요청이 답했다(3-OS 전부). ③은 "OS 분기는 불가피" 지시로 방향이 정해졌고, **④⑤는 여전히 미결** | `dir2/docs/23-cross-platform-feasibility.md:242-249` | — | — | — |

---

## 2. 화면·컨트롤 배치

담당 범위의 화면은 **About 창** 하나와 배포물에 포함된 화면(설치 마법사·zip 안내문)이다.

### 2-1. About 창 (`dir2/crates/nexa-app/src/about.rs`)

**진입**: 메뉴 바 Help ▸ `menu.help.about`("About Nexa Dir" / "Nexa Dir 정보" / "Nexa Dir について") → `CMD_ABOUT`(값 66, `dir2/crates/nexa-app/src/win.rs:255,481`) → `PostMessageW(WM_APP_ABOUT = 0x800C)`(`win.rs:152,5369-5371`) → 핸들러가 `st.dlg_font`를 복제해 `about::show(hwnd, &font)`(`win.rs:9050-9054`). 단축키 없음. Help 메뉴의 유일한 항목이다(`win.rs:481`).

**창**(`about.rs:36-38,310-349`)

| 속성 | 값 |
| --- | --- |
| 창 클래스 | `NexaAbout` |
| 스타일 | `WS_POPUP \| WS_CAPTION \| WS_SYSMENU \| WS_VISIBLE` + 확장 `WS_EX_DLGMODALFRAME`(0x1) — 크기 조절·최대화·최소화 없음 |
| 제목 | `about.title`("About Nexa Dir" / "Nexa Dir 정보" / "Nexa Dir について") |
| 아이콘 | 앱 아이콘 32px(`crate::icon::load(32)`, `about.rs:118`) |
| 배경 | `COLOR_BTNFACE` 시스템 브러시(`about.rs:113-116`) — **테마(다크) 미추종, 라이트 고정** |
| 위치 | 소유자 창 중앙(`about.rs:318-328`), 소유자 rect 조회 실패 시 (200, 200) |
| 클라이언트 폭 | `max(가장 긴 행 폭 + PAD×2, 360)`(`about.rs:303`) |
| 클라이언트 높이 | `마지막 행 아래 y + gap + btn_h + PAD`(`about.rs:304`) |
| 여백 상수 | `PAD = 16`(`about.rs:37`) |
| 모달 | `EnableWindow(owner, false)` + 자체 메시지 루프(`IsWindow(dlg)`가 거짓이 될 때까지) → 끝나면 소유자 재활성·전경(`about.rs:382-390`) |

**글꼴**(`about.rs:67-91,237-238`): 설정의 대화상자 글꼴 `dlg_font`에서 파생. 패밀리 = 폴백 체인의 "설치된 첫 패밀리"(`fontchain::first_installed`, 기본 "Segoe UI"). 본문 = `size_pt`(7~24로 클램프) × 100% · Normal. 제목 = × 170% · Semibold. DPI = 소유자 창 DPI.

**행 구성(위 → 아래, 전부 왼쪽 정렬 x = PAD)**(`about.rs:241-296`)

| # | 텍스트 | 종류 | 색 | 글꼴 | 비고 |
| --- | --- | --- | --- | --- | --- |
| 1 | `Nexa Dir`(고정 문자열) | Title | 검정 `#000000` | 제목(170% Semibold) | 다음 행까지 `title_h + gap` |
| 2 | `about.desc` — en "Ultra-low-memory portable file explorer for Windows" / ko "초저메모리 포터블 Windows 파일 탐색기" / ja "Windows 向けの超低メモリ・ポータブルファイルエクスプローラー" | Body | 검정 | 본문 | dir3에서는 "for Windows" 문구 수정 필요 |
| 3 | `{about.version} {CARGO_PKG_VERSION}` — 예 "Version 0.22.0" / "버전 0.22.0" / "バージョン 0.22.0" | Body | 검정 | 본문 | |
| — | (빈 줄) | — | — | — | 간격 = `line_h / 2` |
| 4 | `about.link.repo` — "Nexa Dir repository (GitHub)" / "Nexa Dir 저장소 (GitHub)" / "Nexa Dir リポジトリ (GitHub)" | Link | accent `#267BD4` + 밑줄 1px | 본문 | URL `https://github.com/SosomLab/nexa-dir2`(`about.rs:32`) |
| 5 | `about.link.releases` — "Download (GitHub Releases)" / "다운로드 (GitHub Releases)" / "ダウンロード (GitHub Releases)" | Link | 같음 | 본문 | URL `https://github.com/SosomLab/nexa-dir2/releases`(`about.rs:34`) |
| 6 | `SosomLab (GitHub)`(고정 문자열) | Link | 같음 | 본문 | URL `https://github.com/SosomLab`(`about.rs:30`) |
| — | (빈 줄) | — | — | — | 간격 = `line_h / 2` |
| 7 | `about.license` — "License: PolyForm Noncommercial 1.0.0"(ko/ja는 PROC-047) | Dim | 회색 `#666666` | 본문 | |
| 8 | `about.copyright` — "© 2026 SosomLab · Sangyong Bae" | Dim | 회색 | 본문 | |

- 줄 높이 `line_h` = 본문 글꼴로 "Ag"를 `DT_CALCRECT` 측정한 높이(최소 14), `gap = line_h / 3`, 일반 행 간격 = `행 높이 + gap`(`about.rs:94-103,260-262,295`).
- 링크 히트 존 = `(PAD, y) ~ (PAD + 텍스트 폭, y + 높이)`(`about.rs:276-286`). 링크 위에서 손 커서(`IDC_HAND`, `about.rs:154-162`), 왼쪽 버튼 누름으로 실행(`about.rs:164-182`): `ShellExecuteW("open", url)` — 그 전에 `crate::win::allow_foreground_handoff()`(이미 떠 있는 브라우저가 앞으로 오도록).
- 링크 URL 주석: "홈페이지 완성 시 URL만 교체"(`about.rs:4-5,29-34`) — 배포 메타데이터는 이미 `https://sosomlab.com/apps/nexa-dir/`를 쓰고 있어(PROC-105) About만 GitHub 링크로 남아 있다.

**버튼**(`about.rs:298-302,356-381`)

| 컨트롤 | 종류 | 위치·크기 | 동작 |
| --- | --- | --- | --- |
| [확인] `about.ok`("OK" / "확인" / "OK") | 네이티브 `BUTTON` + `BS_DEFPUSHBUTTON`, ID 1 | 우하단: x = `client_w − PAD − btn_w`, y = `client_h − PAD − btn_h`. `btn_h = line_h + 10`, `btn_w = max(텍스트 폭 + 24, 72)` | `WM_COMMAND` → `DestroyWindow`(`about.rs:144-149`) |

**탭 순서·키보드**: 포커스 대상은 [확인] 하나. 메시지 루프가 `TranslateMessage`/`DispatchMessageW`뿐이고 `IsDialogMessage`·`WM_KEYDOWN` 분기가 없다(`about.rs:143-231,384-388`) → **Esc·Enter로 닫는 처리 코드는 없다**(닫기 = [확인] 클릭 또는 제목줄 X). 실기 동작은 미확인(추정: 키보드로는 닫히지 않음) — dir3에서는 nexa-sql About 창처럼 Esc = 닫기를 넣는 것이 자연스럽다(`sql/crates/nexa-sql/src/about_win.rs:2`).

### 2-2. 설치 마법사(Inno Setup, 영어 UI) — `dir2/installer/nexa.iss`

`WizardStyle=modern`. 페이지 흐름(Inno 기본 순서 — 스크립트에 명시된 설정 기준, 실제 화면 순서는 추정): 설치 모드 선택 대화상자(`PrivilegesRequiredOverridesAllowed=dialog` — 나만/모든 사용자) → **License Agreement**(`LICENSE.md` 전문, 동의해야 진행) → 설치 폴더(기본 `{autopf}\Nexa Dir`) → 추가 작업(바탕화면 아이콘 — 기본 해제) → 설치 → 완료(Launch Nexa Dir 체크). 프로그램 그룹 페이지는 숨김(`DisableProgramGroupPage=yes`).

### 2-3. zip 동봉 `README.txt` — `dir2/.github/workflows/release.yml:78-99`

한글 블록(제목 "Nexa Dir <버전> — 실행 안내 / How to run" · 무서명 안내 · SmartScreen 경고 인용 · [추가 정보] > [실행] · 무결성 확인 명령 · 배포처 · 문의) + 구분선 + 영문 3줄. UTF-8 BOM.

---

## 3. nexa-ui 매핑

`ui/crates/nexa-ctl/src`·`nexa-dlg`·`nexa-conf`·`nexa-fs`를 Grep해 실제 존재를 확인했다.

| dir2 요소 | 근거 | nexa-ui 대응 | 상태 |
| --- | --- | --- | --- |
| About 창(자체 클래스 + 모달 루프) | `dir2/…/about.rs:235-394` | 창 골격은 nexa-ui가 아니라 **앱 층 패턴**: winit 창 + `Presenter` + nexa-ctl 컨트롤(`sql/crates/nexa-sql/src/about_win.rs:1-70` — 520×300 논리 크기, 내용에 맞춰 높이 조정, Esc = 닫기) | 있음(패턴 차용) |
| [확인] 네이티브 `BUTTON`(`BS_DEFPUSHBUTTON`) | `about.rs:356-381` | `nexa_ctl::Button`(`ui/crates/nexa-ctl/src/controls/button.rs:90`) + `ButtonTone`(`button.rs:76`) | 있음 |
| 정적 텍스트 행(Title/Body/Dim) — `DrawTextW` | `about.rs:184-226` | 전용 Label 컨트롤 **없음**(`struct Label` Grep 0건). `DrawCtx` 텍스트 그리기 + `FontSlot`로 직접 그린다(nexa-sql About이 이 방식) | 직접 그리기로 충분 — 추가 불요 |
| 링크 행(accent + 밑줄 + 손 커서 + 클릭 실행) | `about.rs:125-134,154-182,210-222` | 독립 LinkLabel 컨트롤 **없음**. `TextBox` 내부 링크용 `LinkStyle`/`LinkLine`만 존재(`ui/crates/nexa-ctl/src/controls/textbox.rs:67,78`) | **없음 — 추가 필요**: `LinkLabel`(텍스트·URL 또는 클릭 이벤트 · hover 시 손 커서 요청 · 밑줄 · 키보드 포커스/Enter 활성 · `prefer_size`) |
| URL을 기본 브라우저로 열기(`ShellExecuteW "open"`) | `about.rs:166-178` | nexa-fs에 URL 열기 함수 **없음**(`open_url` Grep 0건). 인접 기능 `reveal_in_file_manager`만 있음(`ui/crates/nexa-fs/src/shell.rs:856` — explorer `/select` · `open -R` · `xdg-open`) | **없음 — 추가 필요**: `nexa_fs::shell::open_url(&str)`(Windows `ShellExecuteW` · macOS `open` · Linux `xdg-open`). 파일 실행(`shell_open`)과 한 묶음으로 설계 |
| 모달(소유자 입력 차단) | `about.rs:382-390` | nexa-dlg는 `FilePicker` 하나뿐(`ui/crates/nexa-dlg/src/lib.rs:154`) — 범용 모달/메시지 상자 프레임 **없음**. nexa-sql의 About·License 창은 모덜리스 | **없음** — 대화상자 공용 호스트는 인벤토리 문서 16(app-controls-dialogs)의 판단 대상. About 자체는 모덜리스 + "이미 열려 있으면 포커스"로 충분(`about_win.rs:63-67`) |
| 대화상자 글꼴(`dlg_font` 파생·170% 제목) | `about.rs:67-91` | `nexa_ctl::theme::FontPrefs` + `FontSlot`(`about_win.rs:5-6`) | 있음(슬롯 매핑은 문서 17 몫) |
| 라이선스 등록 화면(dir2에는 없음 — PROC-045/054) | — | 패턴 = `sql/crates/nexa-sql/src/license_win.rs:1-40`(상태 표 · 요청 코드 · [라이선스 파일 열기…] [제거] [닫기], 컨트롤 = `Button`·`TextBox`·`Flash`) + 얇은 앱 층 `sql/crates/nsql-license/src/lib.rs:1-45` | 있음(패턴 차용) — 파일 열기는 `nexa_dlg::FilePicker` |
| 데이터 폴더 쓰기 프로브(`dir_writable`) | `dir2/…/config.rs:381-383` | `nexa_conf::dir_writable`(`ui/crates/nexa-conf/src/lib.rs:294` — PID + 시퀀스로 프로브 이름 유일화) | 있음 |
| 설치형 폴백 경로(`%LOCALAPPDATA%\NexaDir\data`) | `config.rs:385-398` | `nexa_conf::user_config_dir(app)`(`lib.rs:314` — `%APPDATA%\{app}` · `~/Library/Application Support/{app}` · `$XDG_CONFIG_HOME/{app}`) | 있음 — **단 Windows 폴백 위치가 다르다**(dir2 = LOCALAPPDATA, nexa-conf = APPDATA). 기존 설치본 데이터를 잇려면 마이그레이션 또는 dir3 전용 판정이 필요(§5-2) |
| 포터블 판정의 예외(업그레이드 때 폴더째 교체되는 자리) | dir2에 없음 | `nexa_conf::is_replaced_on_upgrade`(`lib.rs:348` — `.app` 번들·Homebrew keg이면 exe 옆을 건너뛴다) | 있음 — mac 포터블 규율에 필수 |
| 원자적 저장·디바운스 | `dir2/docs/29-audit-checklist.md:155` | `nexa_conf::write_atomic`(`lib.rs:125`) · `SaveScheduler`(`lib.rs:174`) · `Store`(`lib.rs:235`) | 있음 |
| UI 자동 조작 하네스(PostMessage/PrintWindow) | `dir2/scripts/ui-qa.ps1` | nexa-ui에는 없음. nexa-sql은 OS별 스크립트 군(`sql/scripts/win-capture.ps1`·`mac-capture.sh`·`linux-probe.sh` 등)과 `--smoke` 자기 점검(`sql/.github/workflows/ci.yml:53-54`)을 쓴다 | 앱/스크립트 층에서 구현 — nexa-ui 추가 대상 아님(§4) |
| 3-OS 사전 검사 | `dir2/docs/18-build-and-test.md:22-24` | `ui/scripts/check-3os.sh` · `sql/scripts/check-3os.sh` | 있음(스크립트 이식) |

---

## 4. OS 분기점

| 항목(ID) | Windows 현 구현 | macOS 대응 | Linux 대응 |
| --- | --- | --- | --- |
| 링크 플래그(PROC-062) | `+crt-static` · CFG · `/DEPENDENTLOADFLAG:0x800` · CET 금지 | 해당 없음. 배포 타깃 = arm64 + x86_64 → `lipo`로 Universal 2(`sql/docs/33…:52`) | 해당 없음. glibc 동적 링크(추정) · X11/Wayland는 런타임 dlopen이라 빌드 시 시스템 라이브러리 불요(`sql/.github/workflows/ci.yml:41`) |
| 실행 파일 리소스(PROC-063) | `build.rs` → rc.exe: 아이콘·VERSIONINFO·매니페스트 | `.app` 번들: `Info.plist`(`CFBundleIdentifier`·`CFBundleVersion`·`CFBundleShortVersionString`·`NSHumanReadableCopyright`·`LSMinimumSystemVersion`·`NSHighResolutionCapable`) + `.icns`(`sql/packaging/macos/Info.plist` · `sql/docs/33…:51,114`) | `.desktop` 파일 + hicolor 아이콘 8종(`sql/packaging/linux/nexa-sql.desktop` · `sql/docs/33…:69`) |
| 예산 B3(PROC-066) | `dumpbin /imports` 화이트리스트 | `otool -L`로 시스템 프레임워크만 링크하는지 검사(추정 — 저장소 선례 없음) | `ldd`/`readelf -d`로 NEEDED 목록 검사(추정). "인박스" 개념이 없으므로 게이트 문안 재정의 필요(PROC-126) |
| 예산 B1·B2(PROC-019) | WorkingSet64 ≤ 30MB · exe ≤ 10MB | RSS 측정 = `ps -o rss`/`footprint`(추정) · 번들 크기 기준 재설정 | RSS = `/proc/<pid>/status` VmRSS(추정). nexa-sql은 OS별 측정 스크립트 보유(`sql/scripts/mac-perf-all.sh`·`linux-perf-all.sh`) |
| 점검 하네스(PROC-068) | `audit.ps1`(pwsh) — PE 헤더 파싱·`Get-Process`·`MainWindowHandle` | 공통부(T-1~T-4·P-1·P-4)는 그대로 실행 가능(pwsh 7은 mac에도 있으나 의존을 줄이려면 bash/`cargo xtask`). S-1/S-2 = codesign·Hardened Runtime 여부(추정) | 공통부 동일. S-1 = PIE·RELRO·NX(`readelf` — 추정) |
| UI 자동 조작(PROC-074) | 타 프로세스에서 `PostMessageW`로 WM_MOUSE*/WM_KEY* 주입 + `PrintWindow` 캡처 | 타 프로세스 이벤트 주입 = CGEvent(접근성 권한 필요·포커스 영향), 캡처 = `screencapture -l <windowid>`(nexa-sql `sql/scripts/mac-capture.sh` 선례) | X11 = `xdotool`/XTEST, Wayland = 주입 수단 제한(추정). 캡처도 컴포지터 의존 |
| ↳ 권고 | — | **3-OS 공통 수단을 앱에 내장**하는 편이 OS간 차이를 최소화한다(추정 — 설계 제안): 환경변수/인자로 켜는 테스트 모드에서 ① 스크립트(파일 또는 stdin)의 입력 이벤트를 앱의 `InputEvent` 경로에 직접 주입 ② softbuffer 프레임 버퍼를 PNG로 덤프 ③ 상태 JSON 덤프. nexa-sql의 `--smoke` 방식(`sql/.github/workflows/ci.yml:53-54`)을 확장하는 형태 | 같음 |
| 정상 종료 규약(PROC-074) | `WM_CLOSE` = 세션 저장, kill 금지 | 창 닫기 이벤트(winit `CloseRequested`)에서 세션 저장 · 테스트 모드 종료 명령 제공 | 같음 |
| 패닉 흔적(PROC-079) | `data\crash.txt` | 같은 방식(std panic hook) — 위치는 데이터 폴더 또는 `~/Library/Logs/<앱>/`(`sql/docs/33…:43`) | 데이터 폴더 |
| About의 URL 열기(§2-1) | `ShellExecuteW("open")` + 전경 양도 | `open <url>` | `xdg-open <url>` |
| 데이터 폴더(PROC-093·095) | exe 옆 `data\` → 실패 시 `%LOCALAPPDATA%\NexaDir\data` | `.app` 번들·Homebrew keg은 업그레이드 때 통째로 교체되므로 **exe 옆 금지** → `~/Library/Application Support/<앱>/`(`ui/crates/nexa-conf/src/lib.rs:319-326,336-348`) | exe 옆이 쓰기 가능하면 포터블(압축 해제 실행), 아니면 `$XDG_CONFIG_HOME/<앱>` 또는 `~/.config/<앱>` |
| 플러그인 동봉 위치(PROC-097) | `<exe 폴더>\plugins\` | `Contents/Resources/plugins/`(추정 — nexa-sql `Packages` 배치 `sql/docs/33…:42` 준용) | `/usr/share/<앱>/plugins/` 또는 exe 옆(포터블) (추정) |
| 설치 패키지(PROC-094) | Inno Setup exe(사용자별 기본) | `.pkg`(pkgbuild + productbuild) + `.dmg`(`sql/packaging/macos/build-app.sh`·`build-pkg.sh`·`build-dmg.sh`·`uninstall.sh`) | `.deb`(dpkg-deb) + `.rpm`(spec)(`sql/packaging/linux/build-deb.sh`·`build-rpm.sh`). nexa-sql은 AppImage를 두지 않았으나(`sql/docs/33…:60`) dir2는 포터블이 기본 채널이므로 **tar.gz/AppImage 같은 포터블 산출물이 필요**(추정 — 사용자 결정) |
| 패키지 매니저(PROC-098·100) | winget 2종 + Chocolatey 2종 | Homebrew cask(`sql/packaging/homebrew/nexa-sql.rb` · `sql/.github/workflows/homebrew.yml` — 릴리스 공개 시 dmg 해시로 Cask 갱신 → tap 반영 → 설치 스모크) | 저장소 매니페스트는 nexa-sql도 후속(`sql/docs/33…:60`) |
| 서명(PROC-056·102) | 무서명(SmartScreen 경고 감수) · zip으로 다운로드 차단만 완화 | 공증 없는 앱은 Gatekeeper가 차단. nexa-sql 선례 = 서명 단계는 자리만 두고(시크릿 있으면 실행) Cask `postflight`로 quarantine xattr 제거 + caveats 공개(`sql/docs/33…:54,71,104`) | 서명 불요(패키지 서명은 저장소 운영 시 — 추정) |
| 오탐·평판(PROC-103) | Defender ML · VERSIONINFO 채우기 · WDSI 신고 | 해당 없음(공증 문제로 대체) | 해당 없음 |
| 릴리스 CI(PROC-091) | windows-latest 단일 잡 | 3-OS 매트릭스 package 잡 + publish 잡, OS마다 "실제 설치 → `--version`/`--smoke` → 제거 → 잔여 0" 스모크(`sql/.github/workflows/release.yml:65-285` · `sql/docs/33…:72`) | 같음 |
| CI(PROC-064) | core(ubuntu·macos test) + windows(전체) | 3-OS 동등: fmt · clippy `-D warnings` · test · smoke. 형제 저장소 `nexa-ui`·`nexa-license`를 나란히 체크아웃(path 의존) | 같음 + 한글 글꼴 패키지 설치(`fonts-noto-cjk` 등 — `sql/.github/workflows/ci.yml:41-45`) |
| 라이선스 검증(PROC-054) | (계획) CNG P-256 | `nexa-license` `ed25519` + `machine-id`(3-OS 기기 ID — `lic/README.md:29`) + `fs` | 같음 |
| 셸 컨텍스트 메뉴 회귀 도구(PROC-076) | `ctxmenu_probe`·`ctxmenu-timing.ps1` | 비활성(셸 메뉴 자체가 대체 설계) | 비활성 |
| 설치 UI 언어(PROC-094) | 영어만 | pkg `distribution.xml` 로컬라이즈 가능(추정) | 해당 없음 |

---

## 5. 상태·영속 · 스레딩·메시지 흐름

### 5-1. 버전·식별자 상태(릴리스가 바꾸는 값)

| 값 | 저장 위치 | 형식 |
| --- | --- | --- |
| 제품 버전(단일 원천) | `dir2/Cargo.toml:17` `[workspace.package] version` | `MAJOR.MINOR.PATCH`(현재 `0.22.0`) |
| 언어팩 대상 앱 버전 | `dir2/crates/nexa-app/lang/{en,ko,ja}.lang` 머리 `@app = 0.22.0` | `.lang` properties(`@code`·`@name`·`@name.en`·`@author`·`@app`·`@fallback`) |
| exe VERSIONINFO·매니페스트 버전 | `build.rs`가 `CARGO_PKG_VERSION`에서 생성 | `x,y,z,0` |
| 릴리스 태그 | git 태그 `0.22.0`(또는 `v0.22.0`) — `release.yml` 트리거 | — |
| winget 매니페스트 사본 | `dir2/packaging/winget/<버전>/` · `portable/<버전>/` | YAML 3파일 |
| choco 치환 자리 | `chocolateyinstall.ps1`의 `{{VERSION}}`·`{{CHECKSUM64}}` — 저장소에는 자리표시자 그대로 | — |
| 형제 저장소 복원 지점 태그 | `ui`·`lic`에 `baseline/pre-nexa-dir3-2026-10-03` 태그가 이미 있다(`git tag` 확인). 명명 `baseline/<사유>-<날짜>`는 버전 태그 패턴과 겹치지 않아 릴리스 워크플로를 발화시키지 않는다 | — |

### 5-2. 영속 파일(이 문서 범위에서 확인된 것)

| 파일 | 위치 | 내용 |
| --- | --- | --- |
| `settings.cfg` · `session.cfg` · `renames\*.cfg` · `lang\*.lang` | `data_dir()` — exe 옆 `data\` 또는 `%LOCALAPPDATA%\NexaDir\data` | 설정·세션·프리셋·사용자 언어팩(상세 형식은 인벤토리 문서 15 몫). `dir2/docs/12-packaging-single-exe.md:15`의 `settings.json`/`session.json` 표기는 **낡은 서술**(실제 `.cfg` — `dir2/docs/21-distribution.md:21-22`) |
| `plugins\*.wasm` | `data\plugins\`(사용자) · `<exe 폴더>\plugins\`(동봉) | 미리보기 플러그인 |
| `crash.txt` | `data\` | 마지막 패닉 메시지(PROC-079) |
| 라이선스 파일 | **dir2에는 없음**. dir3 = `<설정 폴더>/license/nexa-dir.license`(nexa-license `fs` 규약 — `sql/crates/nsql-license/src/lib.rs:43` `LICENSE_SUBDIR = "license"`, 파일명 `<id>.license` `lic/crates/nexa-license/src/types.rs:19-21`). X-15 설계의 "exe 옆 `license.key`"와 위치·이름이 다르다 → 포터블 계승을 위해 폴더 순서에 exe 옆 `data\license\`를 넣을지 결정 필요 | key=value 서명 문서 |
| 점검 결과 | `docs/audit/<yyyyMMdd-HHmmss>/summary.md` · `audit.log`(+ 사람이 `README.md`·`0N-*.md` 추가) | Markdown 표 · 텍스트 |
| 빌드 산출물(커밋 제외) | `/target/` · `/dist/` · `/installer/out/` · `/packaging/chocolatey/out/`(`.gitignore`) | — |
| 비밀(커밋 금지 규율) | `*.pfx` · `*.snk` · `secrets/`(`.gitignore`) · `.claude/settings.local.json` | — |

### 5-3. CI 설정 상태

| 이름 | 종류 | 쓰임 |
| --- | --- | --- |
| `CHOCO_API_KEY` | 저장소 시크릿 | choco push(없으면 push 건너뜀) — `release.yml:19-21,162` |
| `CHOCO_PUSH` | 저장소 변수(`'true'`/그 외) | 태그 릴리스의 choco 자동 게시 스위치 — `release.yml:158-162`. 릴리스 시점의 채널 상태에 따라 사람이 전환(PROC-101) |
| `github.token` | 자동 | Release 생성(`contents: write`) · `gh release download` |

### 5-4. 메시지 흐름(About)

메뉴 클릭 → `WM_COMMAND(CMD_ABOUT)` → **즉시 열지 않고** `PostMessageW(WM_APP_ABOUT)`로 지연(`dir2/…/win.rs:5369-5371`) → 메시지 루프가 `WM_APP_ABOUT`을 꺼낼 때 `State` 차용 밖에서 글꼴만 복제해 `about::show` 호출(`win.rs:9050-9054`) → `show` 안에서 자체 모달 메시지 루프. 이 "모달은 `WM_APP_*`로 지연 실행" 재진입 규약은 설정·일괄 이름변경·ctldemo 창과 공통이다(`win.rs:5361-5368` 주석). 스레드는 UI 스레드 하나만 쓴다.

dir3(winit) 대응: 명령 디스패치에서 창을 직접 만들지 말고 "창 열기 요청" 플래그/이벤트를 남겨 이벤트 루프의 안전한 지점에서 `ActiveEventLoop`로 창을 만든다(`sql/…/about_win.rs:57-70`의 `open(&mut self, el, theme, owner)` 시그니처가 이 구조).

### 5-5. 릴리스 흐름(상태 전이)

```
release: X.Y.Z 승격 커밋(Cargo.toml · Cargo.lock · lang 3종 @app)
  → main push → CI 3잡 green 확인
  → 채널 실측(winget PR OPEN? · choco Submitted?) → CHOCO_PUSH 값 결정   [사용자 승인 필요 조작]
  → 태그 push(사용자 승인)                                                [release.yml 발화]
      test → release 빌드 → plugins → B2 → B3 → 포터블 개명 → ISCC → zip 3 + README.txt
      → SHA256SUMS.txt → choco pack → (CHOCO_PUSH=true & 시크릿) choco push → Release 생성(자산 6종)
  → 자산 실측(크기·해시 3중 대조: SHA256SUMS = GitHub digest = 로컬 Get-FileHash)
  → winget 매니페스트 사본 작성 → winget validate → winget-pkgs PR 2건(대기 없을 때만)
  → Release 본문(한/영 노트) · 위키 발행
  → docs: X.Y.Z 배포 결과 동기 커밋(STATUS · DEVLOG · journal · docs/21 채널 표)
```
근거: `dir2/docs/16-doc-git-conventions.md:107-112` · `dir2/docs/STATUS.md:7,26-32` · `dir2/docs/21-distribution.md:388-390,420-429`.

---

## 6. 이식 시 주의

### 6-1. dir2 규칙 가운데 dir3에서 조정이 필요한 것(사용자 요청과의 충돌 지점)

1. **push 규칙(PROC-012)** — dir2는 "push는 사용자 명시 요청 시에만"이지만, 이번 사용자 요청은 "중간중간 commit과 push도 하면서 진행"을 명시했다. dir3 규칙 문서에는 "이 프로젝트는 사용자가 상시 push를 승인했다(2026-10-03 요청)"고 **근거와 함께** 적고, 예외로 남길 것 = 버전 태그 push(= 공개 릴리스, PROC-013)·force push 등 파괴적 작업(PROC-014)·저장소 변수/워크플로 dispatch(PROC-034).
2. **형제 저장소 push 순서** — dir3는 nexa-ui·nexa-license를 path 의존하게 되므로 nexa-sql 규칙 "push는 nexa-ui 먼저"(`sql/CLAUDE.md:66`)와 "push 전 `scripts/check-3os.sh`"(`sql/CLAUDE.md:77`)를 함께 계승해야 CI가 깨지지 않는다(CI가 형제 저장소의 main을 체크아웃 — `sql/.github/workflows/ci.yml:24-33`).
3. **DR-1·DR-2·DR-8은 문안 그대로 계승 불가**(PROC-126). dir3의 결정 기록은 dir2 DR 표를 복사하되 "개정" 항목으로: DR-1 = 올 러스트 + **winit·softbuffer·nexa-ui 커스텀 드로잉**(OS 기본 컨트롤 금지 기조는 유지) · DR-2 = 플랫폼별 예산 재정의(수치는 실측 후) · DR-8 = "0 지향" → "원장 기록 + 퍼미시브 확인". dir2 CLAUDE.md가 예고한 ADR 번호는 0007(`dir2/CLAUDE.md:67`).
4. **라이선스 정책 계승의 정확한 의미**(PROC-045·054·055) — dir2에서 확인되는 정책은 "PolyForm NC · 전 기능 무제한 · 상업 사용은 구매 의무 · 문의처"가 전부다. nexa-sql은 Pro/Org 기능 게이트를 갖지만(`sql/crates/nsql-license/src/lib.rs:53-78`) dir2에는 게이트가 없으므로, **dir3의 기본값은 `Feature` 열거 없음(또는 빈 집합) + "라이선스 등록" 화면·상태 표시만**이어야 dir2 정책을 넘어서지 않는다. 미등록(Free) 상태에서 기능이 줄어드는 동작을 넣으려면 사용자 확인이 필요하다. 계열 문서의 방향도 같다(`sql/docs/13-licensing.md:21`).
5. **개발 환경 모델**(PROC-032) — "Windows만 실행 신뢰 원천"은 폐기. 3-OS 모두 실행·QA 대상이므로 실기 QA 부담 3배 경고(PROC-128 ⑤)에 대응해 자동 회귀 하네스(§7)의 비중을 높여야 한다.
6. **제품 식별자**(PROC-105) — dir2와 같은 `AppId`·winget/choco ID·데이터 폴더명을 쓰면 dir3가 dir2를 제자리 업그레이드하고 설정을 이어받는다. 설정 구조를 nexa-sql식으로 바꾸므로 **기존 `settings.cfg`/`session.cfg`와의 호환·마이그레이션 여부**가 함께 결정돼야 한다(미결 — 사용자 확인 대상).

### 6-2. 실측 교훈(회귀 방지에 필요한 것)

| # | 교훈 | 근거 |
| --- | --- | --- |
| ① | **`/CETCOMPAT` 금지**(Windows): 셸 컨텍스트 메뉴가 프로세스 안에 로드하는 서드파티 확장(.NET 2.0 CLR 기반 실측)이 `FAST_FAIL_SET_CONTEXT_DENIED`(0xC0000409)로 앱을 즉사시킨다. explorer.exe도 CET OFF. dir3가 Windows에서 `IContextMenu` 호스팅을 유지하는 한 그대로 유효 | `dir2/.cargo/config.toml:4-8` · `dir2/docs/29-audit-checklist.md:50,117` |
| ② | **타 OS 경로는 Windows green으로 검증되지 않는다**: `cfg` 소거 경로에서 타입 추론 끊김(E0282)·미사용 임포트로 CI core 잡이 일주일 넘게 붉었다(07-27~08-02). 소비자가 전부 `cfg(windows)` 뒤인 순수 모듈에는 `#[cfg_attr(not(windows), allow(dead_code))]` | `dir2/docs/18-build-and-test.md:98-107` · `dir2/CLAUDE.md:38` |
| ③ | **`cargo check`만으로는 부족 — push 후 CI 결과를 실제로 본다**: 해제된 메모리를 읽는 테스트가 Windows 힙에서만 우연히 통과(UB)해 core 잡이 9일·8커밋 동안 붉었다 | `dir2/docs/18-build-and-test.md:109-114` |
| ④ | **도구 셸의 `NO_COLOR`를 상속한 앱은 내장 터미널 색이 꺼진다** — 하네스가 앱을 띄울 때 env에서 제거 | `dir2/scripts/audit.ps1:119` · `dir2/scripts/ui-qa.ps1:75` |
| ⑤ | **앱 종료는 정상 닫기로**(세션 저장 경로를 밟아야 한다) — 하네스에서 kill 금지 | `dir2/scripts/ui-qa.ps1:14,167` · `dir2/docs/18-build-and-test.md:44` |
| ⑥ | **캡처 기반 검증이 코드 리뷰로 못 잡는 결함을 잡는다**: DW 글리프가 GDI 클립을 무시해 왼쪽으로 번지는 결함을 캡처에서 발견 | `dir2/docs/18-build-and-test.md:54-56` |
| ⑦ | **팝업 메뉴는 posted 좌표를 무시**한다(TrackPopupMenu) — 메뉴 항목 선택만 실제 커서 클릭이 필요했다. dir3는 메뉴를 nexa-ui로 직접 그리므로 이 제약이 사라진다 | `dir2/scripts/ui-qa.ps1:46,164-165` |
| ⑧ | **문자 경계 패닉**: 경로 바 한글 입력 제출 시 `[..6]` 바이트 슬라이스로 패닉(설치본 `crash.txt`로 발견) · 압축 항목 긴 CJK 이름 `truncate(4096)` 패닉. 문자열 절단은 `get(..n)`/문자 경계 확인 | `dir2/crates/nexa-app/src/shellpath.rs:19` · `dir2/docs/29-audit-checklist.md:254` |
| ⑨ | **exe 자산은 교체 금지**: 패키지 매니저가 URL + SHA256으로 직접 참조 → 공개 뒤 고칠 것은 새 버전으로 | `dir2/docs/21-distribution.md:74-75` · `sql/docs/33…:106` |
| ⑩ | **설치형에 VERSIONINFO가 비면 Defender ML 오탐**(무서명 + 버전 정보 없음). 모든 exe에 ProductName·FileVersion·Company·Copyright·OriginalFilename을 채운다. 압축 방식 변경은 효과 없음 | `dir2/docs/12-packaging-single-exe.md:82-111` · `dir2/installer/nexa.iss:48-60` |
| ⑪ | **GitHub Actions `run: \|` 안에서 PowerShell here-string 금지**(종료자가 컬럼 0이어야 해 YAML이 깨짐) · ps1의 BOM 보존(한글 주석 모지바케는 CI에서만 드러남) | `dir2/.github/workflows/release.yml:76-77` · `dir2/.github/workflows/resubmit-chocolatey.yml:64-65` |
| ⑫ | **Chocolatey 심사 포인트**: 다운로드 전용 패키지에 VERIFICATION.txt/LICENSE.txt 금지 · `owners`≠`authors` · 설명에 이메일 금지 · 미승인 버전이 있으면 새 버전 push 불가(같은 버전 재제출만) · 첫 승인 44일, 이후 1.6~6일 | `dir2/docs/21-distribution.md:201-251` · `sql/docs/33…:121-123` |
| ⑬ | **winget 심사 포인트**: `DisplayVersion`이 `PackageVersion`과 같으면 넣지 않는다 · 신규 기여자는 파이프라인 재실행 권한 없음 · 포터블은 `winget uninstall`이 데이터까지 삭제 · 중간 버전은 건너뛰어도 무방 | `dir2/docs/21-distribution.md:281-285,312-315,331-333` |
| ⑭ | **플러그인 dist 고정본과 배포본의 분리**: 저장소 `samples/*/dist/*.wasm`은 E2E 테스트용 고정 산출물, 배포본은 태그 소스로 다시 빌드(`-SkipDist`). 드리프트 검출(빌드 후 `git diff --exit-code samples/*/dist`)은 아직 미도입(점검 A25) | `dir2/docs/18-build-and-test.md:83-87` · `dir2/docs/29-audit-checklist.md:101` |
| ⑮ | **플러그인 zip은 폴더째 담는다**(`-Path plugins`) — 파일만 담으면 사용자가 폴더를 직접 만들어야 한다 | `dir2/.github/workflows/release.yml:107-111` |
| ⑯ | **쓰기 프로브 이름은 호출마다 유일해야 한다**(PID만 쓰면 같은 프로세스 두 스레드가 충돌해 Windows에서 순간 폴백으로 튐) — nexa-conf가 이미 PID + 시퀀스로 해결 | `ui/crates/nexa-conf/src/lib.rs:286-309` |
| ⑰ | **macOS `.app`·Homebrew keg 안에 데이터를 두면 업그레이드 때 소실**(nexa-clip 실측) — "exe 옆이 쓰기 가능하면 포터블" 판정을 mac에 그대로 옮기면 안 된다 | `ui/crates/nexa-conf/src/lib.rs:336-348` |
| ⑱ | **점검 1차의 "견고한 부분(회귀 금지)" 목록**: 단일 전송 퍼널·바이트/항목 진행·취소 · 휴지통 단일 배치 + 사후 존재 diff · 완료 통지 재시도 · `sanitize_rel` · 세션 저장 디바운스 · 관대한 클램프 파서 · 데이터 폴더 1회 판정 · 행 가상화·`Invalidations` 모델 · 정렬은 열거 시 1회 · 감시자 수명 · 아이콘 동기/비동기 분리 · 플러그인 샌드박스(임포트 6개·연료·메모리·8MB) · `Secret` zeroize · OAuth PKCE/state/루프백/TLS · zip-slip 정규화 | `dir2/docs/29-audit-checklist.md:262` |
| ⑲ | **점검 1차 즉시 조치 6건은 이식 시 다시 깨지기 쉽다**: 덮어쓰기는 스테이징 후 커밋 교체 · 전송 중 드롭은 거부 + 원위치 복귀 + 안내 · 터미널 셸은 전체 경로로 실행(이름만으로 실행 금지 — 바이너리 플랜팅) · 설정 저장 원자화(선삭제 금지·sync·pid 임시명) · 플러그인 벽시계 1,500ms + 임포트 연료 과금 + 브레이커 | `dir2/docs/29-audit-checklist.md:232-237,260` |

### 6-3. 문서 간 불일치(이식 시 낡은 서술을 그대로 옮기지 않도록)

- `dir2/docs/18-build-and-test.md:60-61`은 rustflags를 `+crt-static`만 적었으나 실제 `.cargo/config.toml:10-14`에는 CFG·`/DEPENDENTLOADFLAG`가 더 있다.
- `dir2/docs/29-audit-checklist.md:37`(T-3 기준 "오류 0")과 달리 실제 `audit.ps1:46-48`은 경고도 판정한다(10-02 강화 — `docs/18:103-104`).
- `dir2/README.md:18`·`packaging/av-false-positive.md:42`는 임포트 "21종"이라 쓰지만 화이트리스트는 `propsys.dll` 추가로 22개(`budget-b3.ps1:25`).
- `dir2/docs/12-packaging-single-exe.md:3,15-20`의 "단일 exe 단독 채널"·`settings.json`·"항상 포터블 모드가 유일"은 DR-3 개정(07-16) 이전 서술.
- `dir2/docs/23-cross-platform-feasibility.md:165,196`의 "ADR-0005(가칭)"은 이후 wasmi가 그 번호를 썼다 → ADR-0007.
- nuspec 설명의 "dual-language UI (English/Korean)"(`nexa-dir.nuspec:37`)는 3개 언어 이전 문구.
- About `about.desc`의 "for Windows"·winget/choco 설명의 "Win32 API"·"no UI framework" 문구는 dir3에서 사실과 달라진다.

---

## 7. 회귀 테스트 후보

자동화: **○** = 3-OS 공통 `cargo test`/스크립트로 자동화 가능 · **△** = OS별 도구 또는 앱 내장 테스트 훅이 필요 · **✕** = 수동(실기).

| # | 시나리오 | 기대 | 자동화 | 관련 ID |
| --- | --- | --- | --- | --- |
| RT-01 | 3타깃 정적 게이트: `cargo fmt --check` + 호스트 및 나머지 두 OS 타깃 `cargo clippy --workspace --all-targets -- -D warnings` | 경고·오류 0 | ○ | PROC-029·064 |
| RT-02 | `cargo test --workspace`(3-OS CI) | 실패 0 | ○ | PROC-010·064 |
| RT-03 | 적대적 입력 묶음: VT 비정상 시퀀스 · 설정 파서 garbage · 압축 zip-slip/잘림 · 플러그인 연료 소진/손상 모듈 | 무패닉·상한 유지·기본값 수렴 | ○ | PROC-070 |
| RT-04 | 동봉 플러그인 빌드 + E2E(`preview::sample`) + dist 드리프트 검사(`git diff --exit-code samples/*/dist`) | 빌드 성공·diff 0 | ○ | PROC-067·071 |
| RT-05 | 플러그인 탐색 순서: 사용자 폴더와 동봉 폴더에 같은 id | 사용자 사본 우선·중복 없음 | ○ | PROC-097 |
| RT-06 | 데이터 폴더 판정: 쓰기 가능 = exe 옆 / 불가 = 사용자 폴더 / mac 번들·keg = 사용자 폴더 | 판정표 일치 | ○ | PROC-095 |
| RT-07 | i18n 3종 키 집합 일치(about.* 포함) + 코드 참조 키 전부 해석 | 누락 0 | ○ | PROC-047 |
| RT-08 | 버전 동기: 태그 = Cargo 버전 = lang `@app` = 표시 버전 | 전부 일치(불일치 시 릴리스 실패) | ○ | PROC-104 |
| RT-09 | 라이선스 상태 판정(순수 함수): 파일 없음 = Free(전 기능 사용 가능) · 유효 = Licensed · 서명 불일치 = Invalid(= Free와 같은 기능) · 만료/버전 조항 | 판정표 일치·**어느 상태에서도 기능 축소 없음**(dir2 계승) | ○ | PROC-045·054 |
| RT-10 | 의존성 라이선스 게이트(`cargo-deny` 또는 `cargo metadata` 화이트리스트) + THIRD-PARTY-NOTICES 생성 | GPL/AGPL 0 · 고지 파일 생성 | ○ | PROC-048·049 |
| RT-11 | 스모크 자기 점검(`--smoke`류): 글꼴 로드·설정 로드·플러그인 로드·창 1프레임 렌더 후 종료 코드 0 | 3-OS CI에서 0 | △(앱 내장 훅) | PROC-064·074 |
| RT-12 | UI 회귀 R01(기동 프레임: 듀얼 패널·도크 Info·라이트/다크) 프레임 덤프 비교 | 기준 캡처와 비교 영역 일치 | △ | PROC-075 |
| RT-13 | UI 회귀 R02~R09(선택·탐색·리네임·스크롤·도크·패널 토글·터미널·드래그 취소) | 상태바·도크·캐럿·선택 일치 | △ | PROC-075 |
| RT-14 | UI 회귀 R10·R14(설정 창 리사이즈·Tab 순회·테마 전환·DPI 150%) | 컨트롤 포커스·하단 버튼 영역 일치·설정 파일 무변경 저장 없음 | △ | PROC-075 |
| RT-15 | UI 회귀 R13(경로바에 `İ`·`%`·한글 입력 후 제출) | 창 생존·패닉 없음 | △(입력 주입) / ○(순수 함수 단위 테스트) | PROC-075 · §6-2 ⑧ |
| RT-16 | UI 회귀 R15(종료 직후 세션 파일) · R16(최소화 65초 후 복원) | 세션 mtime·내용 갱신 · 최소화 중 CPU ~0 | △ | PROC-075 |
| RT-17 | About 창: 열기 → 행 8개·링크 3개·[확인] 표시 → 링크 클릭 시 URL 열기 호출 → Esc/[확인]으로 닫힘 | 표시 문자열 = lang 값 · 버전 = Cargo 버전 · 라이선스 줄 존재 | △(URL 열기는 주입 가능한 포트로 대체해 호출만 검증) | PROC-047 · §2-1 |
| RT-18 | 패닉 흔적: 테스트 모드에서 의도적 패닉 → `crash.txt` 생성 | 파일 존재·메시지 포함 | ○ | PROC-079 |
| RT-19 | 예산 게이트: 실행 파일 크기 · 유휴 RSS(기동 → 10k 폴더 → 유휴) · 링크 의존 목록 | 플랫폼별 기준 이내 | △(OS별 측정) | PROC-019·065·066 |
| RT-20 | 성능 벤치: 폴더 열거 중앙값(≤ 4µs/엔트리) · VT 처리량(≥ 10MB/s) · 기동 → 창 표시(≤ 1.5s) · 유휴 CPU(≤ 2%) | 기준 이내·기준선 대비 악화 없음 | ○(열거·VT) / △(기동·CPU) | PROC-068·080 |
| RT-21 | Windows PE 완화: DYNAMIC_BASE·HIGH_ENTROPY_VA·NX_COMPAT·GUARD_CF 존재 + **CETCOMPAT 부재** + 매니페스트 `asInvoker` | 전부 충족 | ○(Windows 타깃 한정) | PROC-062·063 · §6-2 ① |
| RT-22 | Windows 셸 컨텍스트 메뉴 격리 재현기(`ctxmenu_probe` 계승 시) | 종료 코드 0 | △(Windows 한정·서드파티 확장 설치 환경 의존) | PROC-076 |
| RT-23 | 패키지 스모크: OS별 "설치 → 버전 출력/스모크 → 제거 → 잔여 0"(Windows 설치형·포터블 zip · mac pkg/dmg · Linux deb/rpm) | 전부 통과·사용자 데이터 보존 규칙 일치 | △(릴리스 CI) | PROC-091·109 |
| RT-24 | 릴리스 자산 무결성: 자산 목록·명명·`SHA256SUMS` = 로컬 해시 | 일치 | ○(릴리스 CI) | PROC-092 |
| RT-25 | 다운로드 평판·오탐: 브라우저로 설치형 내려받기 → Defender 격리 여부 · SmartScreen/Gatekeeper 경고 문구 | 격리 없음(경고는 예상 동작) | ✕ | PROC-102·103 |
| RT-26 | 패키지 매니저 실설치(`winget install`·`choco install`·`brew install --cask`) | 설치·실행·제거 | ✕(심사 후) / △(brew — CI 선례) | PROC-098·100 |
| RT-27 | 커버리지 추이: 크레이트별 라인 커버리지 | 기준선 대비 하락 금지 | ○ | PROC-078 |
| RT-28 | 기록 게이트: 작업 종료 시 journal·DEVLOG·TODO·STATUS 한 트랜잭션 갱신 여부 | 누락 없음 | ✕(리뷰) | PROC-003·077(Q9) |
