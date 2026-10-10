# 16 · 문서 관리 · 커밋/푸시 규약 (이식용 표준)

> **출처**: `SosomLab/nexa-dir2` → `nexa-beep` → `nexa-clip` → `nexa-sql` `docs/16-doc-git-conventions.md`를 **2026-10-03 차용**(§0·§4의 push 규칙만 이 저장소 사정으로 정정 — 아래 "dir3 정정" 표).
> **목적**: 검증된 **문서 체계 + git 작업 규약**을 이 저장소에 그대로 적용하기 위한 **휴대용 표준 문서**.
> **사용법**: 아래 [§0 지시문](#0-지시문-복사해서-붙여넣기)이 이 저장소의 규약 원문이다. 본 저장소의 적용 실체:
> [README](README.md)(색인) · [10 결정 기록](10-decision-record.md)(DR 표) · [15 개발 방법론](15-dev-methodology.md) · [18 빌드·테스트](18-build-and-test.md).

---

## 0. 지시문 (복사해서 붙여넣기)

```text
이 프로젝트의 문서·git 규약은 다음을 따른다.

[문서 체계]
- docs/를 4층으로 운영한다: 진입(CLAUDE.md) / 현황(STATUS·MILESTONES·TODO) /
  경과(DEVLOG·journal/YYYY-MM-DD·BRANCHES) / 지식(NN-주제.md·ADR) + docs/README.md 색인.
- 상세는 journal에만 쓰고, DEVLOG·STATUS는 요약 + journal 링크로 끝낸다(중복 서술 금지).
- 모든 진행 기록은 시간 역순(최신이 맨 위). 같은 날 여러 건이면 STATUS에 "N차"로 쌓는다.
- 한 작업 = 한 트랜잭션 갱신: 코드 커밋 → journal 상세 → DEVLOG 한 줄 →
  MILESTONES/TODO 상태 → (브랜치 작업이면) BRANCHES 표.
- 절차 문서(빌드/테스트/배포)는 SSOT를 하나로 지정하고, 절차를 바꾼 그 커밋에서 같이 고친다.
- 확정 사항은 결정 기록(DR-n) 표에, 대안 비교가 필요한 큰 결정은 ADR 문서로 남긴다.
  변경 시 과거 기록을 지우지 말고 새 항목으로 정정한다.
- 기록에는 "무엇을"보다 "왜 그렇게 했는가 + 실측값"(테스트 수·크기·시간)을 남긴다.
- 문서 번호(NN-)는 불변. 재번호 금지, 신규는 뒤에 append.
- 설계 전 기존 문서·코드를 먼저 확인하고(재발명 금지), 이식·참조 시 출처 경로를 커밋 본문에 명기한다.

[커밋]
- Conventional Commits: type(scope): 제목. scope는 모듈 규약을 미리 정해 재사용한다.
- 단위 = 커밋 1개. 수직 슬라이스(관찰·테스트 가능한 얇은 끝단). 초안 먼저, 확장은 별도 커밋.
- 제목에 맥락 태그를 붙인다: (사용자 요청) / (사용자 QA) / (사용자 확정) / 백로그 ID.
- main은 항상 green. 테스트·린트 통과 전 병합 금지.
- 스테이징은 `git add <파일>`로 내가 고친 것만. `git add -A`/`git add .` 금지(남의 변경이 섞여 그쪽 커밋 사유가 유실된다).

[브랜치·푸시 — 반드시 준수]
- 큰 단위 = 브랜치(feat/ fix/ refactor/ docs/), 세부 기능 = 커밋.
- 병합·green 확인 후 로컬 브랜치를 삭제하고, 이력은 BRANCHES.md + journal에 남긴다.
- push는 사용자가 허용한 범위에서만 한다. 이 저장소는 사용자가 "중간중간 commit과 push"를
  명시(2026-10-03)했으므로 **단계(마일스톤·수직 슬라이스) 완료 + 게이트 green**이면 main을 push한다(DR-12).
  형제 저장소(nexa-ui·nexa-license)는 dir3 커밋이 그 변경에 의존하는 경우 **먼저** push한다.
- 버전 태그 push는 공개 릴리스를 만드는 행위이므로 main push와 분리해 별도 승인을 받는다.
- 파괴적 작업(삭제·되돌리기·강제 push·덮어쓰기)은 실행 전 확인받는다.
- 그 외 일상 작업은 사용자 개입 최소화로 자동 진행한다. 상태 기록 md 갱신은 묻지 않고 진행한다.
- 권한 설정 파일(.claude/settings.json)은 덮어쓰기 금지, 병합만 한다.
```

---

## 1. 문서 체계 — 축이 겹치지 않는 4층

| 층 | 파일 | 성격 | 정렬 |
| --- | --- | --- | --- |
| **진입** | `CLAUDE.md` | 이식용 프로젝트 메모리 — 정체성·확정 결정(DR)·규약·다음 단계. clone 즉시 컨텍스트 복원 | — |
| **현황** | `docs/STATUS.md` | "지금 상태" 한 장 — 최신 차수 + 직전 N차 스택 | 최신 위 |
| | `docs/MILESTONES.md` | **기능·목적** 관점 현황(✅ 완료 / 🚧 진행 / 📐 설계 / ☐ 미착수) | 목표순 |
| | `docs/TODO.md` | 순차 백로그 — ID·우선(P0~P2)·규모(소/중/대)·의존·상태 | 목표순 |
| **경과** | `docs/DEVLOG.md` | 날짜별 **요약**(항목당 1~2줄) | 시간 역순 |
| | `docs/journal/YYYY-MM-DD.md` | 날짜별 **상세** — 원인·판단·수치. 기록의 원본 | 시간 역순 |
| | `docs/BRANCHES.md` | 브랜치 생성/병합(커밋)/삭제/커밋수/요약 표 | 시간 역순 |
| **지식** | `docs/NN-주제.md` | 아키텍처·요구사항·결정 기록·빌드/테스트·배포 등. 번호 고정 | 번호순 |
| | `docs/NN-adr-000N-*.md` | ADR — 대안 비교가 필요한 큰 결정 | 번호순 |
| | `docs/port/NN-*.md` | **이식 원장** — nexa-dir2 기능 인벤토리·기준 구조 조사(ID 접두로 구획). 교차 검증의 체크리스트 | 번호순 |
| **색인** | `docs/README.md` | 문서 홈 — **추천 읽기 순서** + 전 문서 표 | — |

**설계 원리** — 세 축을 분리한다.
- **시간축** = DEVLOG(요약) / journal(상세) → "언제 무슨 일이 있었나"
- **목적축** = MILESTONES / TODO → "목표 대비 어디까지 왔나"
- **상태축** = STATUS → "지금 당장 무엇이 걸려 있나"

같은 사실을 세 곳에 길게 쓰지 않고 **링크로 잇는다**. 상세는 언제나 journal 한 곳.

## 2. 문서 작성 — 필수 규칙 9

1. **journal이 원본, 나머지는 요약.** 상세는 journal에만. DEVLOG·STATUS는 요약 + `[journal/YYYY-MM-DD](journal/YYYY-MM-DD.md)` 링크로 끝낸다.
2. **진행 기록은 전부 시간 역순**(최신이 맨 위). 하루에 여러 건이면 STATUS는 `10-03 3차 / 직전(10-03 2차) …` 형태로 쌓는다.
3. **한 작업 = 한 트랜잭션 갱신.** 코드 커밋 → journal → DEVLOG → MILESTONES/TODO → BRANCHES. 하나라도 누락하면 기록이 어긋난다.
4. **SSOT 지정 + 동시 갱신.** 빌드·테스트·배포 절차는 문서 1개를 SSOT로 지정하고, **절차를 바꾼 그 커밋에서 함께 고친다**(사후 정리 금지).
5. **결정은 문서로 고정.** 확정은 DR 표, 큰 결정은 ADR. 변경 시 과거를 지우지 말고 **새 항목으로 정정**하며 "정정"임을 명기한다.
6. **"왜 + 실측값"을 남긴다.** 무엇을 했는지보다 판단 근거와 수치(테스트 green 수, 산출물 크기, 소요 시간, 재현 조건). 다음 세션의 재발명을 막는 것이 기록의 목적.
7. **기존 자산 우선.** 설계 전 기존 문서·코드부터 확인. 이식·차용 시 **출처 경로를 커밋 본문에 명기**(이 저장소는 거의 모든 코드가 이식이다 — `출처: nexa-dir2/crates/…` 또는 `nexa-sql/crates/…` 한 줄).
8. **문서 번호는 불변.** 링크가 깨지지 않도록 재번호 금지, 신규는 뒤에 append. 폐기 문서는 삭제 대신 상단에 "폐기 — 대체: NN" 명기.
9. **결정(DR-)·할 일(T-) 번호는 한 줄로 늘어난다.** 번호를 잡기 전에 `git pull` → [10](10-decision-record.md)·[TODO](TODO.md)의 **최댓값 + 1**. 병렬 작업(여러 세션·에이전트)이 같은 구간을 쓰지 않도록, 조사·이식 항목은 **접두 ID**(`WINA-`·`SET-`·`LIC-`… = `docs/port/`)로 구획하고 DR-/T-는 병합 지점에서 한 번에 부여한다.

## 3. 커밋 규약

- 형식: **`type(scope): 제목`** (Conventional Commits).
- `type`: `feat` `fix` `refactor` `docs` `chore` `ci` `test` `release` `merge`.
- `scope`(이 저장소 고정 어휘): `docs` `ws`(워크스페이스·빌드 설정) `settings` `i18n` `license` `core` `vfs` `tree` `ops` `term` `app` `app/<영역>`(`app/list` `app/dock` `app/prefs` `app/menu` …) `platform` `platform/<os>` `plugin` `cloud` `ui`(nexa-ui 쪽 변경을 가리킬 때) `test` `ci` `pkg`.
- **단위 = 커밋 1개** · **수직 슬라이스**(관찰·테스트 가능한 얇은 끝단) · **초안 먼저, 확장은 별도 커밋**.
- 제목에 **맥락 태그**: `(사용자 요청)` `(사용자 QA)` `(사용자 확정)` 또는 백로그 ID(`T-12`) · 이식 커밋은 **이식 원장 ID**(`WINB-041` · `SET-062`)를 제목 또는 본문에.
- 본문에는 근거·출처 경로·실측 수치(테스트 수, 산출물 크기).
- **main은 항상 green** — 테스트·린트 통과 전 병합 금지.
- ★ **스테이징은 내가 고친 파일만** — `git add <파일>`로 좁혀 담는다. **`git add -A`·`git add .` 금지.**
  - **왜**: 여러 세션·작업이 한 저장소를 동시에 만지면 남의 미완성 변경이 내 커밋에 섞인다. 코드는 안 잃지만 **그쪽 커밋 메시지가 사라져 "왜 그렇게 고쳤는지"가 남지 않는다**.
  - 커밋 전 `git status --short`로 **의도한 파일만 staged인지** 확인한다.
  - 정말 전부 담아야 하면 그 사실을 커밋 본문에 적는다(예: 일괄 포맷·대량 이동·**초기 이식 묶음**).
- 커밋 메시지 끝에는 세션이 안내하는 `Co-Authored-By:` 줄을 그대로 둔다.

예시:
```
feat(app/list): 파일 목록 인라인 이름 바꾸기 — 확장자 제외 선택(PANEL-031 · dir2 rows.rs 이식)
fix(platform/linux): 휴지통 .trashinfo 경로 인코딩(사용자 QA 10-05)
docs: 진행사항 최신화 — STATUS/DEVLOG/journal
release: 0.23.0 승격(크로스플랫폼 1차) — GitHub Release 초안
```

## 4. 브랜치 · 푸시 — 반드시 준수

- **큰 단위 = 브랜치, 세부 기능 = 커밋.** 브랜치명 `feat/…` `fix/…` `refactor/…` `docs/…`. 초기 이식 단계(M0~M2)는 수직 슬라이스가 곧 커밋이므로 `main` 직행을 허용한다(DR-12) — 단 게이트는 같다.
- 병합·green 확인 후 **로컬 브랜치 삭제**, 이력은 `BRANCHES.md` 표 + journal에 보존.
- 🔴 ★ **push 전 게이트 = `scripts/check-3os.sh` + `cargo test --workspace` + `nexa-dir --smoke` + `nexa-dir --selfcheck --ci`**(절차 SSOT = [18](18-build-and-test.md)). 한 OS에서 `cfg` 모듈 경로를 손대면 다른 OS가 깨진다 — 교차 타깃 clippy가 push 전에 잡는다. **push 뒤 `gh run watch`** 로 CI green까지 본다.
- 🟢 **push 시점(dir3 정정 · DR-12)**: 사용자가 2026-10-03 "중간중간 commit과 push도 하면서 진행"을 명시 → **수직 슬라이스/마일스톤 완료 + 게이트 green**이면 main을 push한다. 형제 저장소 변경이 선행 조건이면 **nexa-ui → nexa-license → nexa-dir3** 순서.
- 🔴 **버전 태그 push는 별도 승인.** 태그 = 공개 릴리스 생성 행위이므로 main push와 분리해 다시 확인. (예외: `baseline/…` 복원 지점 태그는 사용자가 요청한 것이므로 생성·push한다.)
- 🔴 **파괴적 작업**(파일·브랜치 삭제, reset/revert, force push, 덮어쓰기)은 실행 전 확인.
- 🟢 그 외 일상 작업은 **사용자 개입 최소화로 자동 진행**. 상태 기록 md 갱신은 묻지 않고 진행.
- 🔴 권한 설정(`.claude/settings.json`)은 **덮어쓰기 금지, 병합만** — 세션 승인 항목 유실 방지.

### dir3 정정 표(원문과 다른 곳)

| 항목 | 원문(nexa-sql) | dir3 |
| --- | --- | --- |
| push | 사용자가 말할 때만 | 슬라이스 완료 + 게이트 green이면 push(사용자 10-03 지시 · DR-12) |
| push 전 게이트 | `check-3os.sh` | **`check-all.sh`**(= 형제 fmt/clippy/test + check-3os + test + `--smoke` + `--selfcheck --ci` + `ndir-check --ci` · `--quick` 가능 · T-05) |
| 형제 저장소 | nexa-ui | nexa-ui · nexa-license — 의존 변경은 형제 먼저 push |
| ID 구획 | D-/T- 한 줄 | 조사·이식 항목은 접두 ID(`docs/port/`) · DR-/T-는 병합 지점에서 |

## 5. 릴리스 (배포가 있는 프로젝트)

1. `release: X.Y.Z 승격(주요 변경 요약) — 채널` 커밋으로 버전 동기(버전 자리 = `Cargo.toml` 하나 · 나머지는 치환).
2. **승인 후** 태그 push → 파이프라인. **태그 전 선검증**(10-10 v0.24.0 교훈): `gh workflow run release.yml -f tag=vX.Y.Z`(git 태그 없이 태그 이름만 입력 → **초안** Release · 3-OS 패키징 + 스모크 · 게시 잡은 건너뜀) → 성공하면 초안 삭제 → 실제 태그. 입력 없이 돌리면 버전이 `0.0.0-dev.<sha>`로 잡혀 버전 검사에서 멈춘다. 이 PC에서 만들 수 없는 다른 OS 패키지(dmg · MSI)를 바꾼 커밋은 이 선검증으로 확인한다.
3. 배포 결과를 다시 기록: `docs: X.Y.Z 배포 결과 동기 — 자산·채널 상태`.
4. 외부 심사 채널(패키지 매니저·스토어) 대기 중이면 **보류 방침을 문서에 명시**하고 그때까지 신규 태그를 만들지 않는다.
5. 공개된 버전의 자산·태그는 다시 만들지 않는다(매니페스트 SHA가 깨진다).
6. **게시물의 언어 = 영어**(사용자 10-10): 패키지 메타데이터(deb `control` description · rpm summary/description · MSI/pkg 제품 설명 · winget/choco/homebrew 매니페스트의 ShortDescription/Description/summary · `.desktop`의 `Name`/`GenericName`/`Comment` · `Cargo.toml` description) · 릴리스 노트/CHANGELOG · 동봉 문서(README · LICENSE · NOTICE · 설치/제거 안내) · 버전/정보(about) 창 문구 · 게시 채널(GitHub Release · 스토어) 본문은 영어로 쓴다. 현지화는 `Comment[ko]` · `[ja]`처럼 **덧붙이는 항목**으로만(영어 원본을 대체하지 않는다). 앱 UI 문자열은 i18n 키(en이 원본 · ko/ja 번역)라 이 규칙과 어긋나지 않는다. 저장소의 개발 문서(docs/ · journal · 커밋 본문 · 코드 주석)는 종전대로 한글.

## 6. 새 프로젝트 적용 체크리스트

```
CLAUDE.md                       # 정체성·확정 결정(DR)·규약 요약·다음 단계·새 세션 오리엔테이션
docs/README.md                  # 문서 홈(추천 읽기 순서 + 색인)
docs/STATUS.md                  # 현황 한 장(역순 차수)
docs/DEVLOG.md                  # 날짜 요약(역순)
docs/journal/YYYY-MM-DD.md      # 날짜 상세(역순) — 기록 원본
docs/MILESTONES.md              # 기능·목표 현황
docs/TODO.md                    # 순차 백로그
docs/BRANCHES.md                # 브랜치 이력
docs/10-decision-record.md      # DR 표 + ADR 색인
docs/15-dev-methodology.md      # 개발 방법론·OS 분기·컨트롤 규칙
docs/16-doc-git-conventions.md  # ← 이 문서
docs/18-build-and-test.md       # 빌드·테스트·점검 하네스 SSOT
docs/port/                      # 이식 원장(인벤토리·기준 구조·검증 매트릭스)
```
