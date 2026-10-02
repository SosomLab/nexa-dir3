# 44 · 기준 구조 가이드 — nexa-sql 프로젝트 운영(문서·git 규칙 · 3-OS CI·패키징 · 테스트 하네스 · 점검 스크립트) → nexa-dir3 회귀 하네스 · 자가 점검 설계 (CI)

> 조사일 2026-10-03 · 읽기 전용 조사(소스·git 변경 0 · cargo 빌드 0). ID 접두사 = `CI-NNN`(CI-001~099 = nexa-sql/nexa-ui에 **있는 것** · CI-101~ = nexa-dir3용 **제안**).
> 근거 표기 = `저장소/경로:줄`. 확인하지 못한 것은 "추정" 또는 "미확인"이라고 적었다.

## 0. 범위

**읽은 것(전부 끝까지)**

| 대상 | 파일 |
|---|---|
| 진입·규칙 | `nexa-sql/CLAUDE.md`(104줄 전부) · `nexa-sql/README.md` · `nexa-sql/docs/16-doc-git-conventions.md` · `61-core-design-and-working-rules.md`(306줄 전부) · `10-decision-record.md`(§1 DR 표 · §3 머리 · §4 crate 원장) · `docs/README.md` · `STATUS.md`/`DEVLOG.md`/`MILESTONES.md`/`TODO.md`/`BRANCHES.md`(머리 = 구조만) · `nexa-ui/docs/16-doc-git-conventions.md`(nexa-sql 판과 diff) |
| 시험·성능·정리·배포 문서 | `nexa-sql/docs/20-testing-codespaces.md` · `81-session-100-features-and-tests.md` · `71-performance-review-process.md` · `93-code-health-and-refactoring.md` · `33-distribution-and-packaging.md` |
| CI | `nexa-sql/.github/workflows/{ci,integration,release,homebrew}.yml` 전부 · `nexa-ui/.github/workflows/ci.yml` · `nexa-license/.github/workflows/ci.yml` |
| 패키징 | `nexa-sql/packaging/lib.sh` · `linux/{build-deb.sh,build-rpm.sh,control,nexa-sql.desktop}` · `macos/{build-app.sh,build-pkg.sh,build-dmg.sh,uninstall.sh,scripts/postinstall}` · `windows/{build-msi.ps1,winres.rs}` · `nexa-sql.wxs`(머리 + Feature 줄) · `homebrew/nexa-sql.rb` · `branding/README.md` |
| 빌드 설정 | `nexa-sql/.cargo/config.toml` · `rust-toolchain.toml` · `Cargo.toml`(워크스페이스·프로필) · `.gitignore` · `.devcontainer/*` 4개 · `nexa-ui/Cargo.toml` · `nexa-ui/.cargo/config.toml` |
| 자동 시험 진입점 | `nexa-sql/crates/nexa-sql/src/app/startup_cmd.rs`(646줄 전부) · `app/demo.rs`(전부) · `probe.rs`(머리 + 함수 윤곽) · `main.rs:1270-1379`(`--smoke`) · `app/event_loop.rs:155-185`(`NSQL_STARTUP_CMD` 해석) |
| 스크립트 | `nexa-sql/scripts/` 66개 전부의 머리 주석 + `check-3os.sh` · `it.sh` · `wait-for-db.sh` · `linux-all-tests.sh` · `func-block-comment.sh` · `win-func-check.ps1`(374줄 전부) · `linux-func-check.sh`(실행기 + 끝) · `check-imports.ps1`(화이트리스트·PE 파서 머리) · `nexa-ui/scripts/check-3os.sh` |
| 헤드리스 그리기 | `nexa-ui/crates/nexa-ctl/src/{draw.rs,raster.rs,controls/mod.rs,controls/textbox.rs,controls/tree.rs}`의 시험용 `DrawCtx` 구현 |

**확인 못 한 것**: ① `nexa-sql/.claude/settings.json`은 구조(allow 항목 수 · `ask` 위치)만 봤다 ② `STATUS.md`·`TODO.md`·`journal/`은 본문 전수가 아니라 형식만 ③ `packaging/linux/nexa-sql.spec` · `macos/{Info.plist,distribution.xml}` · `windows/{nexa-sql.rc,nsql.rc,make-ico.py}` 본문 미독(이름·역할만) ④ nexa-license 태그의 원격 반영 여부.

**먼저 답하는 다섯 가지(결론)**

1. `probe.rs`는 **시험 도구가 아니다** — 접속 창의 서버 도달성 신호등(TCP 프로브)이다(`nexa-sql/crates/nexa-sql/src/probe.rs:1-18`). `demo.rs`도 시험 진입점이 아니라 Demo 프로필·샘플 SQLite 생성이다(`app/demo.rs:1-11`, `:46-65`). **자동 시험·스모크 진입점은 `startup_cmd.rs`(기동 명령) + `main.rs`의 `--smoke` 둘뿐**이다.
2. **헤드리스 UI 시험은 `RasterCtx` 오프스크린 픽셀 검증이 아니다.** nexa-ui의 컨트롤 시험은 그리기를 **기록만 하는 `DrawCtx` 구현**(공개 `ProbeCtx` + 시험마다 만든 `RecCtx`·`TextRec`·`Rec` …)으로 사건 → 상태·사각형·클립을 검증한다(`nexa-ui/crates/nexa-ctl/src/controls/mod.rs:783-796`, `controls/textbox.rs:6075`, `:6097`, `controls/tree.rs:1108`). `RasterCtx`를 오프스크린으로 쓰는 곳은 벤치(`nexa-ui/crates/nexa-ctl/examples/bench_editor.rs:79`)와 운영 paint뿐이다.
3. GUI 전체 흐름은 **실제 프로세스를 격리 홈으로 띄워 기동 명령(`NSQL_STARTUP_CMD`)으로 몰고 덤프 파일·창 캡처로 판정**한다(OS 키·마우스 주입 0). 이 E2E·기능 점검은 **CI에 없다** — 개발 PC에서 OS별 스크립트로 돌린다(Windows 79 시나리오 · Linux 81 시나리오).
4. CI는 3개 저장소 모두 **3-OS 매트릭스 × fmt · clippy `-D warnings` · test**이고, nexa-sql만 `--smoke`와 실서버 통합(`integration`) · 릴리스(`release`) · Homebrew(`homebrew`) 워크플로가 더 있다.
5. 실제 쓰는 패키징 = **Windows MSI(WiX) · macOS Universal 2 `.app` → pkg + dmg · Linux deb + rpm · Homebrew Cask**. AppImage·포터블·winget·choco는 **쓰지 않는다**(`nexa-sql/docs/33-distribution-and-packaging.md:60`, `:74`).

---

## 1. 요소 표

`dir3 적용` 표기: **【차용】** 그대로 · **【수정】** 고쳐서 · **【신규】** nexa-sql에 없어 새로 · **【제외】** 해당 없음. dir3의 환경 변수 접두·크레이트 이름은 40번 문서(SKEL) 결정에 따른다 — 이 문서는 가칭 `NDIR_*`를 쓴다.

### 1-A. 문서 체계

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-001 | 4층 문서 체계 | 진입(`CLAUDE.md`) / 현황(`STATUS`·`MILESTONES`·`TODO`) / 경과(`DEVLOG`·`journal/YYYY-MM-DD`·`BRANCHES`) / 지식(`NN-주제.md`·ADR) + `docs/README.md` 색인 | `nexa-sql/docs/16-doc-git-conventions.md:51-62` | 【차용】 골격 9파일을 첫 커밋에 생성(`:127-140`) |
| CI-002 | 세 축 분리 | 시간축(DEVLOG/journal) · 목적축(MILESTONES/TODO) · 상태축(STATUS) — 같은 사실은 링크로 잇고 상세는 journal 한 곳 | `nexa-sql/docs/16-doc-git-conventions.md:64-69` | 【차용】 |
| CI-003 | 문서 작성 규칙 9 | journal이 원본 · 시간 역순 · 한 작업 = 한 트랜잭션 갱신 · SSOT 동시 갱신 · 결정은 DR/ADR · "왜 + 실측값" · 기존 자산 우선 · 문서 번호 불변 · D-/T- 번호 = 최댓값 + 1 | `nexa-sql/docs/16-doc-git-conventions.md:73-81` | 【차용】 9번(번호 충돌 규칙)은 nexa-sql 판에만 있다(nexa-ui 판은 8규칙 · diff 결과) → **nexa-sql 판을 복사** |
| CI-004 | `CLAUDE.md` = 이식용 메모리 | 정체성 · 참조 원천 표 · 확정 결정 요약 · 작업 규약 · 새 세션 오리엔테이션 — "세션 로컬 메모리는 따라오지 않는다 → 규칙은 이 파일과 docs/61이 전부" | `nexa-sql/CLAUDE.md:3-4`, `:27-35`, `:37-57`, `:62-98`, `:100-103` | 【수정】 같은 5절 구조. ⚠ nexa-sql의 "현 단계" 줄은 한 줄이 수만 자로 비대해졌다(`CLAUDE.md:16`) → dir3는 **현 단계 = 5줄 이내 + STATUS 링크**로 제한 |
| CI-005 | `STATUS.md` 형식 | 머리 "시간 역순 · 상세는 journal" + **다음 세션 시작점** 한 줄 + `## 날짜 (차수 · OS) — 제목([journal §n])` 블록 | `nexa-sql/docs/STATUS.md:1-9` | 【차용】 "N차 · OS" 표기까지 |
| CI-006 | `DEVLOG.md` 형식 | 날짜별 한 항목(1~2줄 원칙) · 역순 | `nexa-sql/docs/DEVLOG.md:1-5` | 【차용】 항목 길이 상한을 지킨다(nexa-sql은 항목이 수백 자로 늘어남) |
| CI-007 | `MILESTONES.md` 형식 | `M0…` 절 + 표(상태 ✅/🚧/📐/☐ · 항목 · 근거 링크) | `nexa-sql/docs/MILESTONES.md:1-11` | 【수정】 dir3는 마일스톤 = 이식 단계(골격 → 패널/목록 → 작업 엔진 → 터미널 → 셸 통합 → 미리보기/플러그인 → 클라우드 → 라이선스 → 배포) |
| CI-008 | `TODO.md` 형식 | 표(ID `T-n` · 우선 P0~P2 · 규모 소/중/대 · 항목 · 의존 · 상태) | `nexa-sql/docs/TODO.md:1-11` | 【수정】 `T-n` + **포트 문서 ID 열**(WINA-·PANEL-·TERM-… 연결) 추가 |
| CI-009 | `BRANCHES.md` 형식 | 표(브랜치 · 생성 · 병합(삭제) · 커밋 수 · 요약) 역순 | `nexa-sql/docs/BRANCHES.md:1-12` | 【차용】 |
| CI-010 | `journal/YYYY-MM-DD.md` | 날짜별 상세 · `§n` 절 번호로 STATUS/DEVLOG/커밋이 가리킨다 · 파일 15개(09-12~09-30) | `nexa-sql/docs/journal/`(목록) · 인용 예 `nexa-sql/docs/STATUS.md:5` | 【차용】 |
| CI-011 | 결정 기록 3구분 | DR(확정 · 사용자 발언 근거 열) / DP(권장 대기) / D(열린 결정) + **외부 crate 원장**(crate · 계층 · 사유 · 라이선스 · 상태) | `nexa-sql/docs/10-decision-record.md:1-9`, `:45-51`, `:138-141` | 【차용】 dir3 DR-1 = "nexa-dir2 기능·배치 유지 · 컨트롤만 nexa-ui" 등 사용자 요청문을 근거 열에 그대로 |
| CI-012 | 핵심 설계·작업 규칙 문서(61) | ① 불변식·금지 ② OS 공통 작업 규칙 ③ OS별 차이 표 ④ 자체 검증 도구 ⑤ 이어 받을 일 | `nexa-sql/docs/61-core-design-and-working-rules.md:7`, `:81`, `:177-191`, `:231-250`, `:287-298` | 【수정】 dir3판 = "OS 분기점 원장"(터미널 셸 · 셸 컨텍스트 메뉴 · 휴지통 · 클립보드 · DnD · 열기) + 검증 도구 절 |
| CI-013 | 문서 색인 | 층별 표 + 추천 읽기 순서 | `nexa-sql/docs/README.md:3-12` | 【수정】 ⚠ nexa-sql 색인은 70번 문서에서 갱신이 멈췄다(`README.md:12`가 끝 · 실제 문서는 101번까지) → dir3는 **문서를 추가한 그 커밋에서 색인 갱신**을 규칙으로 |
| CI-014 | 사용자 위키 원본 | `docs/wiki/*.md`(Home = 목차) → `scripts/wiki-publish.sh`로 GitHub 위키 저장소에 동기(`--push`일 때만 push) | `nexa-sql/scripts/wiki-publish.sh:1-23` | 【수정】 후순위(이식 완료 뒤) |

### 1-B. git · 커밋 · 브랜치 · 푸시 · 태그

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-015 | 커밋 형식 | Conventional Commits `type(scope): 제목` · type = feat/fix/refactor/docs/chore/ci/test/release/merge · 제목에 맥락 태그(`(사용자 요청)`·백로그 ID) · 본문 = 근거·출처 경로·실측 수치 | `nexa-sql/docs/16-doc-git-conventions.md:85-90` · 실제 예 `git log`(`c1970ae` · `f40551a`) | 【차용】 scope 규약을 시작 때 고정(제안 = `core` `ops` `vfs` `tree` `term` `app` `ui` `lic` `plugin` `pkg` `ci` `port`) |
| CI-016 | 스테이징 규칙 | `git add <파일>`만 · `git add -A`/`.` 금지 · 커밋 전 `git status --short` | `nexa-sql/docs/16-doc-git-conventions.md:92-97` | 【차용】 병렬 에이전트가 같은 저장소를 만지므로 **필수** |
| CI-017 | 수직 슬라이스 · main green | 단위 = 커밋 1개 · 초안 먼저 · main은 테스트·린트 통과 전 병합 금지 | `nexa-sql/docs/16-doc-git-conventions.md:88-91` | 【차용】 |
| CI-018 | 브랜치 흐름 | 작업 브랜치(`feat/`·`fix/`·`refactor/`·`docs/`) → 커밋 → `main`에 `--ff-only` 병합 → 로컬 브랜치 삭제 → `BRANCHES.md` 한 줄 → push(형제 저장소는 **nexa-ui 먼저**) → CI 결과를 journal에 | `nexa-sql/docs/61-core-design-and-working-rules.md:87` · `nexa-sql/CLAUDE.md:66` | 【차용】 push 순서 = nexa-ui → nexa-license → nexa-dir3(path 의존 순) |
| CI-019 | push 정책 | "push는 사용자가 명시적으로 요청할 때만 · 자동 push 금지" | `nexa-sql/docs/16-doc-git-conventions.md:111` · `nexa-sql/CLAUDE.md:77` | 【수정】 ★ 이번 요청은 사용자가 "중간중간 commit과 push도 하면서 진행"을 **명시** → dir3 규약에 "이 이식 작업 기간 = 단계 완료마다 push 허용(사용자 2026-10-03)"을 DR로 적는다. 그 밖(버전 태그·강제 push)은 원 규칙 유지 |
| CI-020 | push 전 3-OS 검사 | `scripts/check-3os.sh` = `cargo fmt --all -- --check` + 호스트 clippy + 나머지 두 OS 타깃 clippy(`--all-targets -D warnings` · 컴파일만) · `--quick` = fmt + 호스트만 · 타깃 std 없으면 건너뜀 · push 뒤 `gh run watch` | `nexa-sql/scripts/check-3os.sh:1-46` · `nexa-ui/scripts/check-3os.sh:1-45`(동일) · `nexa-sql/docs/16-doc-git-conventions.md:112` | 【차용】 그대로 복사. ⚠ nexa-sql은 `ring` 때문에 Windows·Linux PC에서 교차 타깃이 실패한다(`docs/61:185`) — dir3는 DB 드라이버가 없으니 **교차 clippy가 통과할 가능성이 높다(추정)** → 의존을 넣을 때마다 확인 |
| CI-021 | 버전 태그 = 별도 승인 | 태그 push = 공개 릴리스 생성 → main push와 분리 승인 | `nexa-sql/docs/16-doc-git-conventions.md:113`, `:120-123` | 【차용】 |
| CI-022 | 파괴적 작업 확인 | 삭제·reset/revert·force push·덮어쓰기는 실행 전 확인 | `nexa-sql/docs/16-doc-git-conventions.md:114` | 【차용】 |
| CI-023 | 원복 기준점 태그 | 세 저장소 `main`에 **같은 이름의 주석 태그**(`baseline/pre-refactor-YYYY-MM-DD`) · 원복 = 새 브랜치 `restore/<날짜>`(main 강제 되돌림 금지) · push는 `git push origin <태그>` | `nexa-sql/docs/93-code-health-and-refactoring.md:22-33` · 실제 태그 `baseline/pre-refactor-2026-09-27`(`git tag -l`) | 【차용】 이미 적용됨: nexa-ui·nexa-license에 `baseline/pre-nexa-dir3-2026-10-03` 존재(로컬 `git tag -l` 확인 · nexa-ui는 원격에도 있음 `git ls-remote --tags` · nexa-license 원격은 미확인) |
| CI-024 | 릴리스 커밋 절차 | `release: X.Y.Z 승격` 커밋 → 승인 뒤 태그 push → 파이프라인 → `docs: 배포 결과 동기` | `nexa-sql/docs/16-doc-git-conventions.md:118-123` · 실제 `6787717`(release) → `c1970ae`(docs) | 【차용】 |
| CI-025 | 커밋 서명 줄 | 커밋 끝에 세션이 안내하는 `Co-Authored-By:` 줄을 그대로(모델 이름을 지어내지 않는다) | `nexa-sql/docs/61-core-design-and-working-rules.md:88` | 【차용】 |
| CI-026 | 형제 저장소 clone 규칙 | `git@kiros33.github.com:SosomLab/<repo>.git`(SSH 별칭) · path 의존이라 형제를 **먼저** pull | `nexa-sql/CLAUDE.md:25`, `:103` · `nexa-sql/docs/61-core-design-and-working-rules.md:4` | 【차용】 dir3 = `../nexa-ui` + `../nexa-license` path 의존 |

### 1-C. 작업 규칙(격리 · 안전 · 검증)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-027 | 답은 한글 | 진행 안내·중간/최종 보고 전부 한글(식별자·경로·명령·오류 원문만 그대로) | `nexa-sql/CLAUDE.md:65` · `docs/61:85` | 【차용】(전역 규칙과 같다) |
| CI-028 | 격리 홈 | 앱을 띄워 시험할 때 `NSQL_HOME`을 임시 폴더로 + **적용됐는지 확인**(격리 폴더 `settings.conf` 수정 시각) | `nexa-sql/docs/61-core-design-and-working-rules.md:145` · 구현 `nexa-sql/crates/nsql-settings/src/lib.rs:157` | 【차용】 `NDIR_HOME`(가칭) — 설정·세션·플러그인·라이선스·최근 경로가 전부 이 아래 |
| CI-029 | 단위 시험은 실제 설정 폴더에 안 쓴다 | 파일을 다루는 모듈은 폴더를 인자로(`*_in(dir, …)`) · 시험은 `std::env::temp_dir()` 아래만 | `nexa-sql/docs/61-core-design-and-working-rules.md:146` | 【차용】 ★ 파일 탐색기라 더 중요 — **파일 작업 엔진 시험은 반드시 임시 샌드박스 폴더 안에서만** |
| CI-030 | 전역 상태 시험은 가드로 직렬화 | 프로세스 전역 스위치·환경 변수·현재 폴더를 만지는 시험은 정적 뮤텍스 가드(본보기 `GdiOn`) — 가드 없던 두 시험이 Windows CI에서만 4번 실패 | `nexa-sql/docs/61-core-design-and-working-rules.md:147` · `nexa-ui/crates/nexa-font/src/lib.rs:522-527` | 【차용】 |
| CI-031 | CI 실패는 로그를 본 뒤 | `gh run view <id> --log-failed` · "되돌리니 통과"는 원인의 증거가 아니다 | `nexa-sql/docs/61-core-design-and-working-rules.md:147` | 【차용】 |
| CI-032 | 입력 주입·포커스 탈취 금지 | `SendKeys`·System Events 키·`SetForegroundWindow` 금지 → 앱은 기동 명령으로 몰고 창 단위 캡처 · 키가 있어야 보이는 것은 단위 시험 + 사용자 실기로 보고 | `nexa-sql/docs/61-core-design-and-working-rules.md:148` · `nexa-sql/CLAUDE.md:68` | 【차용】 ⚠ dir2의 `scripts/ui-qa.ps1`은 `PostMessage`로 창에 직접 메시지를 보낸다(`nexa-dir2/scripts/ui-qa.ps1:1-3`) — winit 앱에는 맞지 않으니 **기동 명령 방식으로 대체** |
| CI-033 | 클립보드 덮어쓰기 금지 | 잘라내기·복사 명령을 자동 시험에 넣지 않는다 | `nexa-sql/docs/61-core-design-and-working-rules.md:149` | 【수정】 dir3는 클립보드가 핵심 기능 → 자동 시험은 **가짜 클립보드 포트**로, 실제 OS 클립보드 점검은 자가 점검의 opt-in 항목(§5 CI-110) |
| CI-034 | 확인 요청 대원칙 | 묻지 않고 진행(스크립트 실행 · 프로젝트 내부 파일 · 프로젝트가 만든 폴더·파일 · `target/` 인스턴스 종료) / 먼저 묻는다(프로젝트 밖 사용자 리소스 · 되돌릴 수 없는 git · 다른 앱 프로세스) / 한 줄 고지 뒤 진행 | `nexa-sql/docs/61-core-design-and-working-rules.md:150-158` · `nexa-sql/CLAUDE.md:69` | 【차용】 + "**자동 시험이 사용자의 실제 파일·휴지통을 건드리지 않는다**" 한 줄 추가 |
| CI-035 | 하네스 권한 설정 | 저장소 `.claude/settings.json`(커밋됨 · 3-OS 공용 · `defaultMode = acceptEdits` · allow = 명령 첫 단어 기준) · 덮어쓰기 금지·병합만 · 에이전트 편집이 막히면 `settings.proposed.json`을 만들어 사용자가 복사 | `nexa-sql/docs/61-core-design-and-working-rules.md:199-229` · `nexa-sql/.claude/settings.json:5`, `:201` · `nexa-sql/CLAUDE.md:93` | 【수정】 dir3용 파일은 사용자 승인으로 만든다(이 조사 단계에서는 만들지 않음) |
| CI-036 | 측정 먼저 · Release로 | 병목은 재서 확인 · 결과는 Release · 빌드 직후 첫 실행은 버린다 | `nexa-sql/docs/61-core-design-and-working-rules.md:136`, `:140`, `:183` | 【차용】 |
| CI-037 | 자료 구조 교체 = 단순 모델 난수 대조 | 자체 xorshift(외부 crate 0)로 새 구조 ↔ 단순 모델을 수십 시드 × 수백 편집 대조 · 호환 경로로 기존 시험을 먼저 통과시킨 뒤 이관 | `nexa-sql/docs/61-core-design-and-working-rules.md:137-139` | 【차용】 가상 목록(대량 파일 리스트)·정렬·선택 모델에 적용 |
| CI-038 | 분기 2개 이상 = 순수 함수 + MC/DC | 판정 본체를 순수 함수로 떼고 조건별 독립 영향 쌍 시험 | `nexa-sql/CLAUDE.md:82` · `nexa-sql/docs/93-code-health-and-refactoring.md:98`, `:220` | 【차용】 OS 분기 판정(어느 셸을 띄울까 · 휴지통 가능 여부 · 메뉴 항목 활성)을 **OS 값을 인자로 받는 순수 함수**로 → 한 OS의 CI에서 3-OS 분기를 전부 시험 |
| CI-039 | 정직한 보고 | 하지 않은 것 · 시험하지 못한 것(키 입력 필요) · 기존 실패를 따로 적는다 | `nexa-sql/docs/61-core-design-and-working-rules.md:141` | 【차용】 |
| CI-040 | Debug 시험 + Release 병행 빌드 | 테스트 실행 = Debug exe · 그 전에 Release도 빌드(사용자가 `target/release`로 병행 시험) | `nexa-sql/CLAUDE.md:90` · `docs/61:90` | 【차용】 |

### 1-D. CI 워크플로

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-041 | `ci` 트리거 | push(main) + pull_request | `nexa-sql/.github/workflows/ci.yml:4-7` | 【차용】 |
| CI-042 | 3-OS 매트릭스 | `os: [windows-latest, macos-latest, ubuntu-latest]` · `fail-fast: false` | `nexa-sql/.github/workflows/ci.yml:18-22` · `nexa-ui/.github/workflows/ci.yml:14-18` | 【차용】 |
| CI-043 | 형제 저장소 나란히 체크아웃 | `actions/checkout@v4` 3번(`path: nexa-sql` · `SosomLab/nexa-ui` → `nexa-ui` · `SosomLab/nexa-license` → `nexa-license`) + `defaults.run.working-directory: nexa-sql` | `nexa-sql/.github/workflows/ci.yml:12-14`, `:24-34` | 【차용】 `path: nexa-dir3` + 형제 둘. ⚠ 형제는 **기본 브랜치 최신**을 받는다(ref 고정 없음) → nexa-ui를 먼저 push하지 않으면 CI가 깨진다(CI-018) |
| CI-044 | 툴체인·캐시 | `dtolnay/rust-toolchain@stable`(components rustfmt, clippy) · `Swatinem/rust-cache@v2`(workspaces 지정) | `nexa-sql/.github/workflows/ci.yml:35-40` | 【차용】 |
| CI-045 | Linux 러너 글꼴 | `apt-get install fonts-noto-cjk fonts-dejavu-core fonts-noto-core`(nexa-font 실측용 · 시스템 라이브러리는 dlopen이라 빌드에 불요) | `nexa-sql/.github/workflows/ci.yml:41-45` | 【차용】 |
| CI-046 | 검사 4단계 | `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` → `cargo run -q -p nexa-sql -- --smoke` | `nexa-sql/.github/workflows/ci.yml:47-54` | 【수정】 + 자가 점검(`--selfcheck --ci`) + WASM 플러그인 빌드 검증 + 헤드리스 시나리오(§5 CI-113) |
| CI-047 | nexa-ui `ci` | 같은 매트릭스 · fmt/clippy/test(스모크 없음 · 형제 의존 없음) | `nexa-ui/.github/workflows/ci.yml:1-35` | 【차용】 nexa-ui에 컨트롤을 추가하면 이 CI가 먼저 초록이어야 한다 |
| CI-048 | nexa-license `ci` | 같은 매트릭스 · clippy/test에 `--all-features` | `nexa-license/.github/workflows/ci.yml`(전문 30줄) | 【차용】(변경 없음) |
| CI-049 | `integration` 워크플로 | 실서버 통합 = DBMS 서비스 컨테이너(Oracle Free 23 · SQL Server 2022 · PostgreSQL 17) + Instant Client 설치 + `cargo test -p nsql-drivers --test integration -- --test-threads=1` + `nsql run` 실기 스크립트 · 트리거 = main push(paths 필터)·PR·수동 · 작업 브랜치 검증은 `gh workflow run integration.yml --ref <브랜치>` | `nexa-sql/.github/workflows/integration.yml:4-9`, `:27-68`, `:85-101` · `nexa-sql/docs/61-core-design-and-working-rules.md:294` | 【제외】 DB 없음. **구조(외부 자원이 필요한 시험을 별도 워크플로로)** 만 차용 → dir3는 "display가 필요한 E2E(xvfb)" 워크플로 후보(CI-113) |
| CI-050 | 환경변수 게이트 통합 시험 | `NSQL_*_URL`/`NSQL_*_PROFILE`이 없으면 `[skip]` 출력 후 통과 | `nexa-sql/crates/nsql-drivers/tests/integration.rs:1-6`, `:13-28` | 【수정】 같은 틀로 "실제 OS 자원이 있을 때만"(PTY · 휴지통 · 클립보드) 도는 시험을 게이트 |
| CI-051 | devcontainer/Codespaces | Rust + Instant Client + 한글 글꼴 이미지 · DBMS compose · `post-create.sh`가 형제 저장소 clone | `nexa-sql/.devcontainer/devcontainer.json:1-30` · `Dockerfile:1-18` · `docker-compose.yml`(전문) · `post-create.sh:1-12` | 【제외】(DB용). Linux 실기 환경이 필요해지면 그때 재검토 |

### 1-E. 릴리스 · 패키징

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-052 | `release` 트리거·meta 잡 | 태그 `v*` push 또는 수동 · 버전 = 태그에서(`v` 뗌) · `-` 포함 = prerelease · **태그 ≠ `Cargo.toml` 버전이면 중단** | `nexa-sql/.github/workflows/release.yml:15-23`, `:37-62` | 【차용】 |
| CI-053 | package 매트릭스 | `ubuntu-22.04`(glibc 2.35 하한) · `windows-latest` · `macos-latest`(arm64 러너 + x86_64 크로스 → lipo) · `RUSTUP_TOOLCHAIN: stable` 고정(rust-toolchain.toml 타깃 자동 설치 충돌 회피) | `nexa-sql/.github/workflows/release.yml:65-87` | 【차용】 |
| CI-054 | "스크립트가 원장" | 워크플로는 `packaging/<os>/build-*`를 부르기만 → 로컬과 CI가 같은 명령 | `nexa-sql/.github/workflows/release.yml:4` · `nexa-sql/packaging/lib.sh:5-6` | 【차용】 |
| CI-055 | 공용 조각 `lib.sh` | `ROOT`·`OUT_ROOT=target/packaging`·`VERSION`(Cargo.toml 단일 원천 · `NSQL_VERSION` 덮어쓰기) · `step/note/die/have/sha256_of/cargo_bin/stage_common/third_party_notices` | `nexa-sql/packaging/lib.sh:14-64` | 【수정】 변수 이름만 dir3로 |
| CI-056 | Linux deb | `cargo build --release --locked` → FHS 스테이징(`/usr/bin` · `/usr/share/applications/*.desktop` · hicolor 8종 · `/usr/share/doc`) → `control` 치환 → postinst/postrm(아이콘·데스크톱 캐시) → `dpkg-deb --build --root-owner-group` → 설치 없는 검증 | `nexa-sql/packaging/linux/build-deb.sh:12-76` · `control:1-16` · `nexa-sql.desktop:1-13` | 【수정】 `.desktop`에 `MimeType=inode/directory;` + `Categories=System;FileManager;` 필요(파일 탐색기 · 제안) |
| CI-057 | Linux rpm | deb 스테이징을 그대로 `rpmbuild -bb`(같은 바이너리 보증) · 사전 릴리스 `-` → `~` | `nexa-sql/packaging/linux/build-rpm.sh:9-40` | 【차용】 |
| CI-058 | macOS `.app` | 타깃별 빌드 → `lipo` Universal 2 → 번들 스테이징(`Contents/MacOS` · `Resources` · `Info.plist` `@VERSION@` 치환) → 아이콘(sips → iconutil 또는 branding `.icns`) → 서명(Developer ID 있으면 · 없으면 애드혹) → 번들 안 `--version` 검증 | `nexa-sql/packaging/macos/build-app.sh:32-101` | 【수정】 CLI 바이너리가 없으면 `BINS` 한 개 |
| CI-059 | macOS pkg | `pkgbuild`(★ `BundleIsRelocatable false` — 첫 릴리스 스모크에서 발견한 재배치 함정) + `productbuild` + postinstall(`/usr/local/bin` 링크) + `uninstall.sh`(번들 Resources 동봉 · `--purge`) | `nexa-sql/packaging/macos/build-pkg.sh:27-37`, `:52-81` · `scripts/postinstall:6-23` · `uninstall.sh:10-29` | 【수정】 CLI 링크가 없으면 postinstall 생략 |
| CI-060 | macOS dmg | drag-to-Applications + 안내문 + `hdiutil create UDZO` + 마운트 검증 · 공증 자리 | `nexa-sql/packaging/macos/build-dmg.sh:13-54` | 【차용】 |
| CI-061 | Windows MSI | WiX **5.0.2 고정**(v4 스키마 · 버전 없이 설치하면 최신 판과 UI 확장 불일치 `WIX0144`) · 스테이징 · `license.rtf` 생성 · `wix build` · signtool 자리 · UpgradeCode 고정 · Feature = Main / PathEnv / SqlAssoc | `nexa-sql/packaging/windows/build-msi.ps1:58-77`, `:95-132`, `:134-146` · `nexa-sql.wxs:34`, `:131-139` | 【수정】 ★ **결정 필요** — dir2는 포터블 단일 exe + Inno 설치형 + ZIP + Chocolatey(`nexa-dir2/.github/workflows/release.yml:48`, `:55`, `:71`, `:135`). dir3가 "nexa-sql 기준"이면 MSI, "dir2 자원·정책 계승"이면 Inno/포터블 → 23번 문서(PROC)와 함께 DR로 확정 |
| CI-062 | 임포트 게이트 | `scripts/check-imports.ps1` = PE 헤더 직접 파싱(일반 + 지연 임포트) → 인박스 DLL 화이트리스트 대조 · `vcruntime140.dll` 등은 사유와 함께 실패 · `build-msi.ps1` 안에서 호출(로컬·CI 공통) | `nexa-sql/scripts/check-imports.ps1:37-59`, `:61-101` · `nexa-sql/packaging/windows/build-msi.ps1:89-93` | 【차용】 dir2에도 같은 게이트 `scripts/budget-b3.ps1`가 CI에 있다(`nexa-dir2/.github/workflows/ci.yml` "예산 B3") → 하나로 통일 |
| CI-063 | 정적 CRT | 두 MSVC 타깃 `-C target-feature=+crt-static` · ⚠ env `RUSTFLAGS`가 있으면 통째로 무시됨 → 워크플로에 `RUSTFLAGS`를 두지 않는다 | `nexa-sql/.cargo/config.toml:3-13` · `nexa-sql/docs/33-distribution-and-packaging.md:99-100` | 【차용】 |
| CI-064 | Windows 리소스 삽입 | `packaging/windows/winres.rs`를 각 bin의 `build.rs`가 `include!` · `.rc` + 버전 define 래퍼 → `rc.exe`/`llvm-rc`/`windres` · 도구 없으면 조용히 건너뜀 · **외부 crate 0** · 버전 단일 원천 = Cargo.toml | `nexa-sql/packaging/windows/winres.rs:1-62`, `:65-89`, `:132-174` · `nexa-sql/crates/nexa-sql/build.rs:1-6` | 【차용】 dir2 아이콘(`.ico`) 자원을 그대로 연결 |
| CI-065 | 설치 스모크(릴리스 잡 안) | 실제 설치 → `nsql --version` 일치 · `nexa-sql --smoke` → 제거 → 잔여 0(Linux `dpkg -i/-r` · macOS `installer`/`uninstall.sh`/`pkgutil` · Windows `msiexec /i /x` + 레지스트리 확인) · rpm은 목록·서명만 | `nexa-sql/.github/workflows/release.yml:121-136`, `:162-175`, `:191-211` | 【차용】 |
| CI-066 | publish 잡 | 산출물 모음 → `sha256sums.txt` → 릴리스 노트 → `gh release create --draft`(있으면 `upload --clobber`) — **초안**으로만, 공개는 사람이 | `nexa-sql/.github/workflows/release.yml:213-223`, `:226-285` | 【차용】 |
| CI-067 | Homebrew 워크플로 | 릴리스 **published** 때(또는 수동 · dry_run) dmg 해시로 Cask 채움 → `ruby -c` + `brew style` → 탭 저장소 push(`TAP_TOKEN` 없으면 아티팩트만) → 실제 `brew install --cask` → 버전·`--smoke` → uninstall · 사전 릴리스 건너뜀 | `nexa-sql/.github/workflows/homebrew.yml:10-23`, `:42-58`, `:69-100` · `nexa-sql/packaging/homebrew/nexa-sql.rb:9-51` | 【수정】 배포 채널은 사용자 결정 뒤(후순위) |
| CI-068 | 서명 자리 | 시크릿 있을 때만 서명/공증(macOS `MACOS_*` · Windows `WINDOWS_SIGN_*`) · 없으면 unsigned + 릴리스 노트 안내 | `nexa-sql/.github/workflows/release.yml:9-12`, `:142-155`, `:178-186` | 【차용】 |
| CI-069 | THIRD-PARTY-NOTICES | `cargo metadata` 라이선스 목록(python 표준 라이브러리만) · bash 래퍼 | `nexa-sql/scripts/third-party-notices.py`(머리) · `third-party-notices.sh:1-13` · `nexa-sql/packaging/lib.sh:60-64` | 【차용】 |
| CI-070 | 브랜딩 SSOT | `packaging/branding/icon.svg` → png 세트·`.ico`·`.icns`(`scripts/pack_icon.py` · PIL) · 런타임 창 아이콘은 코드로 그림 | `nexa-sql/packaging/branding/README.md:18-42` | 【수정】 dir2 아이콘 자원을 SSOT로(사용자: "자원은 그대로") — 재생성 불필요 |
| CI-071 | 배포 교훈표 | 동적 CRT · `RUSTFLAGS` 덮어쓰기 · 버전 정보 공란(오탐) · 태그≠버전 · Cask 문법 · quarantine · `release: published`는 GITHUB_TOKEN으로 공개하면 트리거 안 됨 · 공개 뒤 수정 = 새 버전 + 릴리스마다 메타데이터 점검표 | `nexa-sql/docs/33-distribution-and-packaging.md:97-106`, `:110-117` | 【차용】 체크리스트로 복사 |
| CI-072 | 릴리스 프로필 | `opt-level 3` · `lto fat` · `codegen-units 1` · `panic abort` · `strip symbols` + `profile.dev.package."*" opt-level 2`(Debug에서도 래스터·컨트롤은 최적화) | `nexa-sql/Cargo.toml:90-101` | 【차용】 `panic = "abort"`이므로 패닉 = 즉시 종료 → 패닉 훅(CI-112)이 유일한 기록 수단 |
| CI-073 | 워크스페이스 메타·린트 | `[workspace.package]` 버전 단일 원천 · `rust-version 1.89` · lints(`missing_debug_implementations`·`unreachable_pub`·`unwrap_used` warn → CI `-D warnings`로 오류) | `nexa-sql/Cargo.toml:32-48` · `nexa-ui/Cargo.toml:18-34` | 【차용】 ⚠ `unwrap_used`가 사실상 오류다 — 시험 파일은 `#![allow(clippy::unwrap_used)]`(`nsql-drivers/tests/integration.rs:7`) |
| CI-074 | 툴체인 고정 | `channel = "stable"` + 5타깃 목록 | `nexa-sql/rust-toolchain.toml:3-11` | 【차용】 |

### 1-F. 테스트 하네스(단위 · 헤드리스 · 통합)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-075 | 단위 시험 배치 | 모듈 안 `#[cfg(test)] mod tests`가 기본(오늘 `#[test]` 수: nexa-sql 736 · nexa-ui 457 · nexa-license 32 — `grep` 집계) · `tests/` 디렉터리는 통합 시험만(`nsql-drivers/tests`) | `nexa-sql/crates/nexa-sql/src/main.rs:731`, `:2550` · `grid.rs:6610` · `explorer.rs:8047` | 【차용】 dir2의 `#[test]` 425개(`grep` 집계)를 크레이트 이식과 함께 가져온다 |
| CI-076 | 공개 시험 백엔드 `ProbeCtx` | 그리기 no-op · `text_width` = 글자 수 × 7(결정적) · **다운스트림 위젯 시험용으로 공개** | `nexa-ui/crates/nexa-ctl/src/controls/mod.rs:783-796` | 【차용】 dir3 패널·목록 시험이 이것을 쓴다 |
| CI-077 | 기록기 `DrawCtx`(시험 로컬) | `RecCtx`(fill_rect 사각형 기록) · `TextRec`(text 문자열 기록) · `ClipRec` · `BlitCtx` · tree의 `Rec`(clip·fill) · scroll의 `ScaleCtx` · draw의 `Quirky` — **시험마다 따로 정의**(공용 부품 아님) | `nexa-ui/crates/nexa-ctl/src/controls/textbox.rs:6075`, `:6097`, `:6146`, `:6763` · `controls/tree.rs:1104-1135` · `controls/scroll.rs:1128` · `draw.rs:368` | 【수정】 nexa-ui에 **공용 기록기**를 한 벌 추가(§5 CI-104) — 컨트롤을 많이 추가할 dir3 이식에서 중복을 막는다 |
| CI-078 | `DrawCtx` 기본 구현 | `surface_size()` 기본 `None`(크기를 모르는 기록기) · `caret_on()` 기본 true · `select_font` 기본 no-op → 시험 백엔드는 4메서드만 구현하면 된다 | `nexa-ui/crates/nexa-ctl/src/draw.rs:31-49` | 【차용】 |
| CI-079 | 운영 래스터 `RasterCtx` | `Surface` + 글꼴 위의 CPU 래스터 · 자체 시험은 SDF 영역 분해 대조 1건 · 오프스크린 사용 = 벤치뿐 | `nexa-ui/crates/nexa-ctl/src/raster.rs:60`, `:339`, `:722-769` · `examples/bench_editor.rs:79` | 【신규】 오프스크린 프레임 렌더 시험은 nexa-sql에 **없다** → dir3에서 새로(§5 CI-105) |
| CI-080 | 고정물 시험 | 디코더 시험 = `tests/*.hex` 고정물을 `include_str!` | `nexa-ui/crates/nexa-gfx/src/image.rs:729-731` · `jpeg.rs:531` | 【차용】 미리보기·압축 목록·아이콘 디코드에 같은 방식 |
| CI-081 | 시험용 훅(운영 코드 안) | `*_for_test`(그리드 셀 설정/선택) · `dump_*`(상태 → 글) · `capture_*`(메뉴 열기·펼치기·고르기) — 기동 명령이 부르므로 `cfg(test)`가 아니다 | `nexa-sql/crates/nexa-sql/src/grid.rs:2176`, `:2219`, `:2226` · `explorer.rs:7426`, `:7451`, `:7510`, `:7543` | 【차용】 컨트롤·패널마다 `dump()` 하나를 **설계 요건**으로 |
| CI-082 | 알려진 흠 | nexa-sql 시험 바이너리가 병렬 실행에서 간헐 SIGSEGV(살아 있는 백그라운드 스레드 의심 · 미해결) | `nexa-sql/docs/TODO.md:180`(T-191), `:220`(T-238) | 【수정】 dir3 규칙: **시험이 만든 스레드(감시·열거·PTY)는 Drop에서 join** |
| CI-083 | 벤치 | `cargo run --release -p nexa-ctl --example bench_editor`·`bench_undo` · nsql-run `bench_vars` | `nexa-sql/docs/61-core-design-and-working-rules.md:239` · `nexa-sql/docs/71-performance-review-process.md:132-134` | 【수정】 dir3 벤치 = 대량 목록(10만 항목) 정렬·그리기 · 터미널 VT 파서 처리량 |

### 1-G. 자동 시험 진입점(스모크 · 기동 명령 · 환경 변수)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-084 | `--smoke` | 설정 열기 → UI·고정폭 글꼴 탐색(없으면 exit 1) → 글꼴 사슬·한글 지원·언어·테마 한 줄 출력 → `smoke ok — 드라이버: […]` 출력 후 **이벤트 루프를 만들기 전에 종료**(창 없음) | `nexa-sql/crates/nexa-sql/src/main.rs:1284-1325` | 【수정】 드라이버 자리에 "자원(아이콘·문자열)·플러그인 런타임·라이선스 루트 키" 확인(§5 CI-109) |
| CI-085 | `NSQL_STARTUP_CMD` 해석 | 쉼표로 나눈 명령 목록 · `@after:<ms>:<명령>` = 시차 실행(`startup_timed`) · `@connected:<명령>` = 첫 접속 뒤 · 그 밖 = 즉시 `startup_cmd(id)` · 변수 없으면 비용 0 | `nexa-sql/crates/nexa-sql/src/app/event_loop.rs:162-184` · 저장 필드 `main.rs:493`, `:496` | 【차용】 문법 그대로 + 조건 대기·정상 종료 확장(§5 CI-106) |
| CI-086 | 기동 명령 = 명령 id 공용 | 접두 분기에 안 걸리면 `open:<경로>` = 파일 열기, 나머지는 **`menu_action(id)`** → 메뉴·키맵·팔레트의 모든 명령 id가 곧 기동 명령 | `nexa-sql/crates/nexa-sql/src/app/startup_cmd.rs:641-644` | 【차용】 ★ dir2의 명령(복사·이동·이름 바꾸기·탭·패널 전환…)을 **명령 id 표 하나**로 만들어 메뉴·단축키·기동 명령이 같이 쓰게 |
| CI-087 | 앱 안 마우스 사건 | `ui.move/click/dclick/sclick/cclick/rclick/wheel/hwheel:x/y[/delta]` — OS 주입이 아니라 `InputEvent`를 만들어 **실제 라우팅 경로 `route()`** 에 넣는다(컨트롤 직접 호출은 라우팅 결함을 못 본다) | `nexa-sql/crates/nexa-sql/src/app/startup_cmd.rs:26-75` · `nexa-sql/docs/61-core-design-and-working-rules.md:237` | 【차용】 + 키 사건(`ui.key:`) · 드래그(`ui.drag:`) 추가(파일 탐색기는 드래그·키가 핵심) |
| CI-088 | 덤프 명령군 | `<대상>.dump:<파일>` = 상태를 글로: explorer(`:94`) · details(`:106`) · grid(`:188`) · result(`:193`) · drop(`:220`) · output(`:225`) · intel(`:248`) · editor(`:252`) · txlog(`:265`) · ime(`:316`) · about(`:328`) · license(`:332`) · mem(`:346`) · bm.stat(`:382`) · import(`:425`) · sqlprev(`:434`) · log(`:455`) · objlink(`:478`) · explorer.selpath(`:483`) · explorer.stat(`:110`) | `nexa-sql/crates/nexa-sql/src/app/startup_cmd.rs`(각 줄) | 【수정】 dir3 어휘 = §5 CI-107 |
| CI-089 | 캡처·조작 명령군 | 메뉴 열기/고르기(`explorer.menu`·`explorer.pick` `:529-543` · `conn.menu` `:616` · `result.menu` `:576` · `objlink.menu/pick` `:459-477`) · 펼치기/선택/필터(`:77-105`) · 확인 창 통과(`drop.confirm` `:212`) · 상태 팝업 픽(`multi.`/`tx.`/`disc.`/`close.` `:596-603`) · 대화상자 없이 다중 열기(`file.open_many:` `:605`) · 설정 창 검색어로 열기(`edit.prefs:` `:581`) · 가짜 IME(`ime.fake:` `:311`) · 비밀번호 창 응답(`pw.answer_from_profile:` `:293`) | `nexa-sql/crates/nexa-sql/src/app/startup_cmd.rs` | 【수정】 원칙만 차용: **사용자 입력 경로(열기 → 확정)를 같은 함수로 밟는 명령을 둔다**(`docs/61:242`) |
| CI-090 | 기동 명령 함정 | 쉼표 = 명령 구분자 → 인자 구분은 `;`·`/` · 새 명령 id는 기존 접두와 `strip_prefix`가 겹치지 않게(`explorer.selected:`가 `explorer.select`에 먹힘) · Debug 첫 기동은 수 초 → 캡처 대기 12초 이상 · 비활성 앱은 App Nap으로 `@after`가 늦게 깸 → 마지막 `@after` + 5 s | `nexa-sql/docs/61-core-design-and-working-rules.md:237`, `:242`, `:250` · `startup_cmd.rs:116` | 【수정】 시간 대기 대신 **조건 대기**로 근본 해소(§5 CI-106) |
| CI-091 | 시험용 환경 변수 | `NSQL_HOME`(격리) · `NSQL_NO_ACTIVATE=1`(창을 활성화하지 않고 띄움 — 자체 시험 인스턴스는 **반드시**) · `NSQL_TRACE_FRAMES=1`(`[startup]`·`[frames]`·`[load]` stderr) · `NSQL_TRACE_MEM`·`_IME`·`_CLIP`·`_META`·`_OBJLINK`·`_VAULT`·`_WINDOW`·`_CONNECT` | `nexa-sql/crates/nexa-sql/src/icon.rs:175-177` · `winfocus.rs:175-178` · `main.rs:1277`, `:1680-1681` · `app/event_loop.rs:53` · `docs/61:238` | 【차용】 `NDIR_HOME`·`NDIR_NO_ACTIVATE`·`NDIR_STARTUP_CMD`·`NDIR_TRACE_FRAMES` + 기능별 `NDIR_TRACE_<기능>` |
| CI-092 | `probe.rs`(참고 · 시험 아님) | 서버 도달성 신호등: TCP 연결 1개 · 지수 백오프 · 별도 스레드 · 동시 상한 — "앱이 스스로 만드는 부하의 통제" 본보기 | `nexa-sql/crates/nexa-sql/src/probe.rs:1-18`, `:200`, `:370` | 【제외】 기능은 불필요. 패턴(백오프 순수 함수 + 시험)만 클라우드 연결 점검에 참고 |
| CI-093 | `demo.rs`(참고 · 시험 아님) | 최초 실행 1회 "데모 만들기" 팝업 → 배경 스레드로 `demo.sqlite` 생성 → 프로필 저장. Debug 자동 접속(`dev.start_demo`)은 정리 때 제거됨 | `nexa-sql/crates/nexa-sql/src/app/demo.rs:8-92` · `nexa-sql/docs/93-code-health-and-refactoring.md:165` | 【제외】 단 "시험·캡처용 샘플 폴더 생성기"는 dir3 하네스에 필요(§5 CI-108) |

### 1-H. 절차서(성능 · 코드 건강)

| ID | 요소 | 구현 | 위치 | dir3 적용 |
|---|---|---|---|---|
| CI-094 | 성능 종합 점검 프로세스 | 규정 대상 12차원(D1~D12) · 순서 A 인벤토리 → B 기동 → C 시나리오 → D 향상 모드 A/B → E 누수 → F 벤치 → G 병목 · 생략 규칙 · 판정선(기동 ≤ 300 ms · 프레임 ≤ 8 ms · 유휴 CPU ≈ 0 · 누수 기울기 ≈ 0) · 회귀 판정(+5 %/+1 MB · CPU 2배 · 기동 +20 %) · 기능 추가 체크리스트 10 | `nexa-sql/docs/71-performance-review-process.md:21-34`, `:43-58`, `:150-162`, `:168-179` | 【수정】 차원·순서는 그대로, 시나리오를 파일 탐색기용으로(§5 CI-118). "0은 값이 아니라 측정 실패다"(`:230`) 교훈 포함 |
| CI-095 | 메모리 회수 시험 C-2 | 기준선 → 올림 → 놓음 → 회수 4점 · 허용치 표 | `nexa-sql/docs/71-performance-review-process.md:109-122` | 【수정】 대상 = 대량 폴더 열기/닫기 · 탭 닫기 · 미리보기 닫기 · 터미널 닫기 |
| CI-096 | 코드 건강 절차 | P0 원복 태그 → P1 기준선 → P2 허용 표시 걷기(컴파일러 판정) → P3 컴파일러 밖 미사용 → P4 임시 코드 → P5 구조 → P6 중복 → P7 검증 → P8 기록 · "목적 있는 시험 자산은 청소 대상 아님" | `nexa-sql/docs/93-code-health-and-refactoring.md:11-16`, `:39-49` | 【차용】 이식 완료 뒤 1회 |
| CI-097 | `code-health.py` | 외부 패키지 0 · A 무조건 `allow` · B 참조 0 `pub` · C 안 쓰는 `Msg` · D 안 쓰는 설정 키 · E 안 쓰는 의존 · F 크기 · G 중복 블록 · H 커버리지(`cargo llvm-cov`) · 기준선 JSON 비교 | `nexa-sql/docs/93-code-health-and-refactoring.md:55-72` · `nexa-sql/scripts/code-health.py`(머리) | 【수정】 저장소 목록·크레이트 이름만 바꿔 재사용(C·D는 dir3 i18n·설정 레지스트리 이름에 맞춤) |
| CI-098 | 변경 체크리스트 14 | `must_use`·가시성·`allow(dead_code)` 금지(OS별은 `cfg_attr` + 이유)·150줄 함수 금지·시험 정리·검증 | `nexa-sql/docs/93-code-health-and-refactoring.md:213-226` | 【차용】 ★ `cfg_attr(not(<OS>), allow(dead_code))`는 **3-OS 컴파일로 판정**(`:89`) — OS 분기가 많은 dir3에서 가장 자주 밟을 함정 |
| CI-099 | 기능 목록 · 확인 방법 문서 | 세션 단위로 "기능 · 확인 방법 · 관련 설정" 표 + 자동 시험 명령 목록 | `nexa-sql/docs/81-session-100-features-and-tests.md:5-12`, `:54-59` | 【수정】 dir3는 이 형식을 **검증 매트릭스**(포트 ID ↔ 시험)로 승격(§5 CI-116) |

---

## 2. 문서 · git 규칙

### 2-1. 규약 원문과 그 계보

- 규약 SSOT = `docs/16-doc-git-conventions.md`. 계보는 nexa-dir2 → nexa-beep → nexa-clip → nexa-sql(2026-09-12 차용)이다(`nexa-sql/docs/16-doc-git-conventions.md:3`). 즉 **dir3가 차용할 규약의 뿌리가 dir2**이므로 충돌이 없다. nexa-ui 판과의 차이는 규칙 9(D-/T- 번호 충돌) 한 줄뿐이다(diff 확인).
- `§0 지시문`(`:13-45`)이 통째로 복사해 쓰는 원문이다 — dir3 `CLAUDE.md`에는 이것을 축약해 넣고(`:143`), 문서 16은 그대로 복사한다.

### 2-2. dir3에 그대로 가져올 것

1. **문서 골격**(CI-001): `CLAUDE.md` · `docs/README.md` · `STATUS.md` · `DEVLOG.md` · `journal/` · `MILESTONES.md` · `TODO.md` · `BRANCHES.md` · `10-decision-record.md` · `16-doc-git-conventions.md`. 각 문서 머리에 역할·정렬 한 줄(`:142`).
2. **기존 `docs/port/NN-*.md`**(이 인벤토리 묶음)는 "지식" 층의 하위 폴더로 둔다 — 번호 공간이 `docs/NN`과 겹쳐 보이므로 색인에서 `port/` 접두로 구분한다(제안).
3. **커밋·스테이징·브랜치 규칙**(CI-015~018) 전부.
4. **작업 규칙**(CI-027~040) 전부 — 특히 격리 홈 · 실제 설정 폴더 금지 · 입력 주입 금지 · 전역 상태 가드.

### 2-3. dir3에서 고쳐야 할 것

| 항목 | nexa-sql | dir3 |
|---|---|---|
| push | 사용자가 말할 때만(CI-019) | 사용자가 이번 작업에 "중간중간 commit과 push"를 명시 → **단계(마일스톤) 완료 + 게이트 초록일 때 push**를 DR로 기록. 버전 태그 push는 여전히 별도 |
| push 전 게이트 | `check-3os.sh` | `check-3os.sh` + `--smoke` + `--selfcheck --ci`(§5) — 자동 진행이므로 사람 대신 게이트가 막아야 한다 |
| 형제 저장소 | nexa-ui · nexa-license | 같음. **컨트롤을 nexa-ui에 추가하는 커밋이 dir3 커밋보다 먼저 push**(CI는 형제의 기본 브랜치를 받는다 · CI-043) |
| `CLAUDE.md` "현 단계" | 한 줄이 계속 길어짐 | 5줄 이내 + STATUS 링크(CI-004) |
| 색인 | 70번에서 멈춤 | 문서 추가 커밋에서 같이 갱신(CI-013) |
| 번호 충돌 | 두 PC가 같은 D-/T- 구간을 썼다(`docs/16:81`) | 병렬 에이전트가 번호를 잡으면 같은 일이 난다 → **ID는 접두로 구획**(이 포트 문서들의 `WINA-`·`CI-`처럼)하고 D-/T-는 병합 지점에서 한 번에 부여 |
| 원복 태그 | `baseline/pre-refactor-…` | `baseline/pre-nexa-dir3-2026-10-03`(nexa-ui·nexa-license 로컬에 이미 있음 · CI-023). dir3 자체에도 큰 단계 앞에 같은 방식으로 |

### 2-4. 릴리스 규칙

`release:` 커밋 → 승인 → 태그 → 파이프라인 → `docs:` 결과 동기(CI-024). 공개된 버전의 자산·태그는 다시 만들지 않는다(매니페스트 SHA가 깨짐 · `nexa-sql/docs/33-distribution-and-packaging.md:106`).

---

## 3. CI · 패키징

### 3-1. 워크플로 한눈에

| 워크플로 | 저장소 | 트리거 | 러너 | 하는 일 |
|---|---|---|---|---|
| `ci` | nexa-sql | push(main) · PR | win · mac · ubuntu | fmt → clippy `-D warnings` → test → `--smoke`(`ci.yml:47-54`) |
| `ci` | nexa-ui | push(main) · PR | win · mac · ubuntu | fmt → clippy → test(`nexa-ui/.github/workflows/ci.yml:30-35`) |
| `ci` | nexa-license | push(main) · PR | win · mac · ubuntu | fmt → clippy `--all-features` → test `--all-features` |
| `integration` | nexa-sql | main push(paths) · PR · 수동 | ubuntu | DBMS 컨테이너 3 + 통합 시험 + CLI 실기(`integration.yml:94-101`) |
| `release` | nexa-sql | 태그 `v*` · 수동 | ubuntu-22.04 · win · mac | meta → 3-OS 포장 + 설치 스모크 → 초안 릴리스(`release.yml:37`, `:65`, `:226`) |
| `homebrew` | nexa-sql | release published · 수동 | mac | Cask 생성 → 탭 반영 → 설치 검증(`homebrew.yml:27-100`) |

### 3-2. CI에 **없는** 것(중요)

- GUI 기능 점검·E2E(기동 명령 시나리오) — 전부 로컬 스크립트.
- 교차 타깃 clippy — 각 러너가 자기 OS만 본다(3-OS 매트릭스가 대신).
- 커버리지·코드 건강·성능 — 로컬 절차.
- Windows arm64 · Linux arm64 산출물 — 설정(`.cargo/config.toml:12-13`)과 스크립트 인자(`build-deb.sh:24` · `build-msi.ps1:55`)는 있으나 릴리스 워크플로는 x64만 부른다(`release.yml:119`, `:190`).

### 3-3. 실제 쓰는 패키징

| OS | 형식 | 도구 | 산출물 이름 | 설치 위치 |
|---|---|---|---|---|
| Windows | MSI | WiX 5.0.2(v4 스키마) + UI 확장 | `nexa-sql-<ver>-windows-x64.msi`(`build-msi.ps1:125`) | `%ProgramFiles%\Nexa SQL` · HKLM · PATH 기능 |
| macOS | pkg(주) | pkgbuild + productbuild | `nexa-sql-<ver>-macos-universal.pkg`(`build-pkg.sh:22`) | `/Applications/Nexa SQL.app` + `/usr/local/bin` 링크 |
| macOS | dmg(보조) | hdiutil | `nexa-sql-<ver>-macos-universal.dmg`(`build-dmg.sh:16`) | 끌어 놓기 |
| macOS | Homebrew Cask | 탭 `kiros33/homebrew-tap` | — | dmg 기반(`nexa-sql.rb:14`) |
| Linux | deb | dpkg-deb | `nexa-sql_<ver>_amd64.deb`(`build-deb.sh:28`) | `/usr/bin` · `/usr/share` |
| Linux | rpm | rpmbuild | `nexa-sql-<ver>-1.x86_64.rpm`(`build-rpm.sh:7`) | 같은 스테이징 |

쓰지 않는 것: AppImage(`docs/33:60`) · 포터블(DR-27 · `docs/33:11` — D-78 재검토 중) · winget/choco(`docs/33:74`, `:119-124`).

### 3-4. dir3 패키징에서 정해야 할 것(결정 대기)

1. **Windows 채널**: MSI(nexa-sql 기준) vs 포터블 exe + Inno + ZIP + Chocolatey(dir2 현행 · `nexa-dir2/.github/workflows/release.yml:48-161`). 사용자 요청은 "기술구조는 nexa-sql 기준 · 자원은 dir2 그대로"라 어느 쪽인지 명시가 없다 → **추정: MSI로 통일하되 dir2의 choco/winget 등록 자산은 별도 결정**.
2. **예산 게이트**: dir2 CI는 "단일 exe ≤ 10 MB"(B2)와 임포트 화이트리스트(B3)를 건다(`nexa-dir2/.github/workflows/ci.yml` 예산 단계). nexa-sql 예산은 30 MB(`docs/71:159`). winit·softbuffer·wasmi가 들어가면 10 MB를 넘을 수 있다(추정) → 예산 수치를 다시 정한다.
3. **동봉 플러그인**: dir2는 릴리스에서 `markdown.wasm`·`archive.wasm`을 빌드해 동봉한다(`nexa-dir2/.github/workflows/release.yml:31`) → `stage_common`의 `Packages/` 자리(`nexa-sql/packaging/lib.sh:46-58`)에 대응시킨다.
4. **Linux `.desktop`**: 파일 관리자 MIME(`inode/directory`) 등록 여부.

---

## 4. 테스트 하네스

### 4-1. 층 구조(있는 그대로)

```
① 정적          cargo fmt --check · clippy --all-targets -D warnings · check-3os.sh(교차 clippy)
② 단위          #[cfg(test)] mod tests — 순수 함수 · MC/DC · 난수 대조 · 고정물(include_str!)
③ 헤드리스 위젯  DrawCtx 기록기(ProbeCtx · RecCtx …) + InputEvent 직접 주입 → 상태·사각형·클립 단언
④ 통합(환경 게이트) nsql-drivers/tests/integration.rs — 환경 변수 없으면 [skip]
⑤ 스모크        nexa-sql --smoke (CI · 설치 스모크)
⑥ 프로세스 E2E   격리 홈 + NSQL_STARTUP_CMD + *.dump:<파일> → 파일 내용 grep (로컬 스크립트)
⑦ 기능 점검      시나리오 표: 생존 · 패닉 없음 · 창 수 (+ Windows = PrintWindow 캡처 · Linux = 덤프 검사)
⑧ 성능·누수·회수  perf-all · leak · mem-reclaim · inventory (docs/71)
```

### 4-2. 헤드리스 위젯 시험의 실제 모양(③)

- 컨트롤을 만들고 `set_bounds` → `on_event(&InputEvent::…, &mut inv)`를 직접 부른 뒤 상태를 단언한다(예: `nexa-ui/crates/nexa-ctl/src/controls/textbox.rs:6060-6072` — 드래그가 상자 밖으로 나가도 캐럿이 한 칸씩).
- 그리기 검증이 필요하면 **기록기**를 `paint`에 넘겨 사각형·클립·문자열을 모아 단언한다(예: 가로 스크롤 시 클립이 컨트롤 왼쪽 밖으로 안 나감 — `controls/tree.rs:1104-1135`).
- 글자 폭은 `chars × 7`로 고정해 OS 글꼴과 무관하게 결정적이다(`controls/mod.rs:793-795`).
- **픽셀을 비교하는 시험은 없다.** 글꼴 래스터는 OS마다 다르다(Windows GDI ClearType · macOS CoreText · Linux ab_glyph — `nexa-sql/docs/61-core-design-and-working-rules.md:186`). 맥의 글자 품질 대조는 별도 수동 도구(`scripts/mac-text-ref.swift` + `mac-text-compare.py`)로 했다.

### 4-3. 프로세스 E2E의 실제 모양(⑥)

가장 단순한 본보기 `scripts/func-block-comment.sh`(31줄 · 3-OS 공통):

1. 격리 홈 생성 + `settings.conf`에 방해 요소 끄기(`demo.prompted=on` 등)(`:10-13`)
2. `NSQL_HOME` · `NSQL_NO_ACTIVATE=1` · `NSQL_STARTUP_CMD="@after:900:edit.select_all,@after:1100:edit.toggle_block_comment,@after:1300:file.save,@after:1700:file.exit"`로 앱 실행(`:18`, `:27`)
3. 프로세스 종료를 기다린 뒤(15초 상한) **결과 파일을 기대값과 비교**(`:21-24`) → 종료 코드.

덤프 기반 본보기는 `linux-func-check.sh`의 `run_s <id> <제목> <기동명령> [인자] [대기초] [추가설정] [검사식]` — 검사식 = `파일:정규식`(앞에 `!` = 없어야 함)(`nexa-sql/scripts/linux-func-check.sh:83-118`). 판정 = 생존 → 패닉 없음 → 창 ≥ 1 → 검사식.

### 4-4. 기능 점검 시나리오 표(⑦)

| | Windows `win-func-check.ps1` | Linux `linux-func-check.sh` |
|---|---|---|
| 시나리오 수 | `Run-Scenario` 79 | `run_s` 81 |
| 판정 | 프로세스 생존 · 패닉 문자열 없음 · 보이는 창 ≥ 1 → `auto-ok`(`:153-155`) | 같음 + 덤프·저장 파일 검사식 → 통과/실패 합계 · 종료 코드(`:98-117`, 끝) |
| 눈으로 볼 것 | PrintWindow 캡처 PNG + "눈으로 볼 것" 열(`:126-146`, `:162`) | 캡처 도구 없음 → 사용자 실기로 넘김(`:4-7`) |
| 좌표 | 창 크기 고정 기준의 장치 px를 주석에 기록(`:165-167`) | `window.main_size` 고정(`:77`) |
| 빌드 | Release(Debug는 기동이 느려 명령이 밀림) | Release(`:12`) |
| 규칙 | "새 기능을 넣으면 시나리오 한 줄을 더한다"(`docs/61:246`) | 같음 |

**약점(dir3가 고칠 것)**: ① 같은 시나리오가 OS별로 **두 벌**(ps1 · sh) ② 고정 대기(`WaitMs`) 뒤 `Kill` — 느리고 타이밍에 민감 ③ 좌표 하드코딩 → 레이아웃이 바뀌면 전부 깨짐 ④ Windows판은 "창이 살아 있다"까지만 자동(내용은 사람이 캡처를 봄) ⑤ macOS판 전체 시나리오 표는 없다(`mac-grid-edit-e2e.sh` 등 개별 E2E만).

### 4-5. 점검 스크립트 전체 목록(66개 · `nexa-sql/scripts/`)

| 묶음 | 스크립트 | 역할 | dir3 |
|---|---|---|---|
| 게이트 | `check-3os.sh` | push 전 fmt + 3타깃 clippy | 【차용】 |
| 게이트 | `check-imports.ps1` | Windows 임포트 화이트리스트(PE 직접 파싱) | 【차용】 |
| 게이트 | `linux-all-tests.sh` | **세 저장소** fmt → clippy → test → check-3os → 실서버 통합 → CLI 기능 → `summary.txt`(단계별 rc·시간·passed/failed 집계 `:15-18`) | 【수정】 3-OS 공용 `check-all`로 일반화(§5 CI-115) |
| 기능 점검 | `win-func-check.ps1` · `linux-func-check.sh` · `func-block-comment.sh` | 시나리오 표 · 편집 명령 왕복 | 【수정】 러너 1벌로(§5 CI-108) |
| GUI E2E | `mac-grid-edit-e2e.sh` · `win-mssql-explorer-e2e.sh` · `win-objlink-reveal-e2e.sh` · `win-reveal-matrix-e2e.sh` · `win-output-result-e2e.sh` · `win-paste-run-e2e.sh` · `win-schema-switch-e2e.sh` · `win-intel-3part-e2e.sh` · `linux-vault-e2e.sh` · `conn-cmd-e2e.sh`(GUI 부분) | 격리 홈 + 기동 명령 + 덤프 → PASS/FAIL · 조합 매트릭스(MC/DC 케이스 표) | 【수정】 방식만(도메인은 DB) |
| CLI E2E | `mac-bulk-e2e.sh` · `linux-dbms-e2e.sh` · `oracle-ddl-e2e.sh` · `dbms-source-e2e.sh` · `it.sh` · `wait-for-db.sh` | CLI로 실서버 왕복 · 임시 객체 생성/삭제 | 【제외】 |
| 캡처 | `win-capture.ps1` · `win-burst-capture.ps1` · `mac-capture.sh`(System Events 사용 → 사용자 부재 때만) · `mac-capture-ascii.py` | 창 단위 캡처(PrintWindow · screencapture) · ASCII 덤프 | 【차용】 win 두 개 그대로 · mac은 `screencapture -l`만 |
| 글자 품질 | `mac-text-ref.swift` · `mac-text-compare.py` | CoreText 참조 렌더와 픽셀 비교 | 【제외】(nexa-ui 몫) |
| 성능 실행기 | `win-perf-all.ps1` · `linux-perf-all.sh` · `mac-perf-all.sh` · `mac-common.sh` | docs/71 A~F 전수 | 【수정】 시나리오 교체 |
| 성능 탐침 | `win-startup-probe.ps1` · `linux-startup.sh` · `mac-startup.sh` / `win-big-probe.ps1` · `linux-probe.sh` · `mac-probe.sh` / `win-latency-probe.ps1`(SendKeys 사용) / `bench-boost.ps1` · `cli-wall.py` | 기동 · 초 단위 CPU/메모리 · 입력 지연 · 향상 모드 A/B · CLI wall | 【수정】 기동·탐침은 exe 이름만 바꿔 재사용 |
| 누수·회수 | `win-leak-cycle.ps1` · `linux-leak.sh` · `mac-leak.sh` / `win-mem-reclaim.ps1` · `linux-mem-reclaim.sh` / `memcycle.ps1`(SendKeys · 옛 탐침) | N주기 기울기 · 4점 회수 | 【수정】 |
| 인벤토리 | `win-inventory.ps1` · `linux-inventory.sh` | 용량·섹션·crate 수·동적 라이브러리·구성 파일·설정 키 | 【차용】 |
| 개발 편의 | `mac-restart-debug.sh` · `linux-restart-debug.sh` | Debug 빌드 → 내가 띄운 PID만 종료 → 재기동 | 【차용】 + Windows판 신규 |
| 코드 건강 | `code-health.py` · `split-app-impl.py`(1회성) · `narrow-app-visibility.py`(멱등) | 미사용·중복·크기 · 구조 이관 | 【수정】 code-health만 |
| 확장 | `ext-build.ps1` · `ext-build.sh` · `ext-sync-installed.ps1` | SDK 샘플 → `.wasm` 배치 + sha256 갱신 · 설치본 반영 | 【수정】 dir2 `scripts/build-plugins.ps1`과 합쳐 3-OS판으로(43번 문서 EXT 참조) |
| 배포 보조 | `third-party-notices.py`/`.sh` · `pack_icon.py` · `install-desktop-linux.sh` · `wiki-publish.sh` | 고지 목록 · 아이콘 세트 · 개발 PC `.desktop` 설치 · 위키 동기 | 【차용】 |
| 환경 | `install-instantclient-linux.sh` · `install-instantclient-mac.sh` | Oracle 클라이언트 설치 | 【제외】 |
| 기타 | `win-badge-probe.ps1`(SendKeys 옛 탐침) · `win-kill-stale-hooks.ps1`(Claude Code 훅 좀비 프로세스 정리) | — | `win-kill-stale-hooks.ps1`만 【차용】(개발 환경 공통) |

참고: `.gitignore:26-28`이 가리키는 `scripts/linux-watch-e2e.sh`는 현재 작업 트리에 없다(추정: 다른 PC의 미병합 작업).

---

## 5. dir3 하네스 · 자가 점검 제안

> 목표(사용자 요청): "회귀 테스트를 감안하면서 테스트 하네스를 잡아주고, 핵심 기능에 대한 오류가 발생하면 바로 확인이 가능하도록 점검 로직". 아래는 **제안**이며 이름은 가칭이다.

### 5-1. 제안 요소 표

| ID | 요소 | 내용 | 근거(왜) |
|---|---|---|---|
| CI-101 | 하네스 7층 | T0 정적 → T1 단위 → T2 헤드리스 위젯 → **T3 헤드리스 앱 시나리오(신규)** → T4 프로세스 E2E → **T5 자가 점검(신규)** → T6 성능 | nexa-sql 층(§4-1)에 T3·T5를 더한다 — nexa-sql은 앱 수준 회귀를 CI에서 못 돈다(§3-2) |
| CI-102 | 창 없는 앱 코어 | 앱을 `AppCore`(상태 + `route(InputEvent)` + `paint(&mut dyn DrawCtx)` + 명령 분배 · winit 무관)와 `Shell`(winit 창 · present · OS 사건 변환)로 나눈다 → `cargo test`가 `AppCore`를 직접 만든다 | nexa-sql `App`은 창·이벤트 루프에 묶여 시나리오가 전부 실제 프로세스다(`event_loop.rs:168` · `startup_cmd.rs:25` `self.window`). 분리하면 3-OS CI에서 시나리오 회귀가 수 초에 돈다 |
| CI-103 | OS 분기 포트 + 가짜 구현 | `Platform` 묶음 = 포트(trait) 6개: `ShellLauncher`(pwsh / `$SHELL`) · `Pty` · `ContextMenuProvider` · `Trash` · `Clipboard` · `DragDrop`(+ `Opener`). 운영 = OS별 구현, 시험 = `FakePlatform`(기록·주입) | "확장점 = 포트 + 레지스트리 + 설정"(`nexa-sql/CLAUDE.md:79`) · OS 분기 판정은 순수 함수로(CI-038). 가짜로 로직을, 자가 점검(CI-110)으로 실제 구현을 검증 — 두 겹 |
| CI-104 | 공용 기록기 `RecordCtx` | nexa-ui `nexa-ctl`에 공개 추가: `fills: Vec<(Rect, Color)>` · `texts: Vec<(i32, i32, Rect, String)>` · 지정 가능한 `surface_size` · `text_width = 글자 × 7` | 지금은 시험마다 구조체를 새로 쓴다(CI-077). dir3 이식은 nexa-ui에 컨트롤을 여럿 추가하므로 공용화 효과가 크다 |
| CI-105 | 프레임 골든 = 구조 덤프 | T3에서 `AppCore.paint(RecordCtx)` → ① 패닉 0 ② 표면 밖 사각형 0 ③ 팝업이 표면 안(`surface_size`) ④ **레이아웃 덤프**(영역 이름 + Rect 트리)를 골든 파일과 비교. 픽셀 해시는 하지 않는다 | 글꼴 래스터가 OS마다 달라 픽셀 골든은 깨진다(§4-2). 사용자 요구 "컨트롤 배치 유지"는 **배치(Rect)** 가 대상이므로 구조 덤프가 맞다. dir2 대비 배치 대조표의 자동화 수단 |
| CI-106 | 기동 명령 확장 | nexa-sql 문법(CI-085) + ① `@ready:<명령>`(첫 프레임·초기 폴더 열거 완료 뒤) ② `@idle:<명령>`(작업 큐가 빈 뒤) ③ `quit[:코드]`(정상 종료 경로) ④ `assert.<대상>:<식>`(실패 = stderr 한 줄 + 종료 코드 ≠ 0) ⑤ `ui.key:` · `ui.drag:` | 고정 `sleep` + kill(§4-4 약점 ②)을 없앤다 → 빠르고 타이밍 무관. 앱이 스스로 종료 코드를 내면 러너가 단순해진다 |
| CI-107 | dir3 덤프 어휘 | `layout.dump` · `panel.dump`(활성 패널·경로·선택·정렬·필터) · `list.dump`(보이는 행) · `tree.dump` · `tabs.dump` · `status.dump` · `menu.dump`(열린 메뉴 항목 id·활성) · `ops.dump`(작업 큐·진행·결과) · `term.dump`(화면 격자 글 · 셸 · cwd) · `preview.dump` · `prefs.dump` · `plugin.dump` · `license.dump` · `log.dump` | CI-088의 dir3판. **컨트롤·패널마다 `dump()` 하나**를 구현 요건으로(CI-081) |
| CI-108 | 시나리오 러너 1벌 | 워크스페이스 bin `ndir-check`(의존 0 · 가칭): 시나리오 파일(`tests/scenarios/*.scn` — id · 제목 · 샘플 폴더 틀 · 설정 · 기동 명령 · 검사식 `파일:정규식`/`!`)을 읽어 격리 홈·샘플 트리 생성 → 앱 실행 → 종료 코드·stderr 패닉·검사식 판정 → 표 + `summary.txt` + 종료 코드. 캡처는 OS별 선택 플러그(없으면 건너뜀) | nexa-sql의 두 벌(ps1 79 · sh 81)을 한 벌로(§4-4 약점 ①). 샘플 폴더 생성기 = `demo.rs` 자리의 dir3판(CI-093) |
| CI-109 | `--smoke`(창 없음 · CI) | 설정 레지스트리 로드 → 글꼴 → **내장 자원**(아이콘·문자열 표) 디코드 → i18n 표 완전성 → WASM 런타임 초기화 → 라이선스 루트 키 등재 → `smoke ok` + exit 0 | CI-084 계승. ★ nexa-sql은 루트 키가 비어 있던 때가 있었다(`nexa-sql/CLAUDE.md:12` "ROOT_KEYS 비어 있음") → 스모크가 잡게 |
| CI-110 | `--selfcheck`(자가 점검 · doctor) | 아래 §5-3 표. 출력 = 사람용 표 + `--json` · 항목 = PASS / FAIL / WARN / SKIP(사유) + ms · 종료 코드 = FAIL 수 · `--ci` = display·사용자 자원이 필요한 항목 자동 SKIP · `--only <그룹>` | "핵심 기능 오류를 바로 확인"의 본체. 실제 OS 구현(CI-103의 운영 쪽)을 사용자의 기기에서 검증하는 유일한 층 |
| CI-111 | 앱 안 진단 | ① **기동 점검**(경량 · 수 ms): 설정 읽기 실패 · 자원 누락 · 플러그인 폴더 · 셸 탐지 실패 · 라이선스 파일 손상 → 상태줄 배지 + 로그 + 토스트 ② Help ▸ 자가 점검 창 = `--selfcheck`와 **같은 함수**의 결과 표 + "복사" ③ 기능 실패 시(휴지통 불가 · PTY 생성 실패 · 컨텍스트 메뉴 백엔드 없음) 한 번만 알리는 사유 문구 | CLI와 창이 같은 점검 함수를 쓴다(판정 두 벌 금지 — `docs/61:242`의 원칙). 지원 요청 때 사용자가 표를 붙여 넣을 수 있다 |
| CI-112 | 패닉 훅 · 크래시 기록 | `std::panic::set_hook`으로 `<HOME>/crash/<시각>.txt`(메시지 · 위치 · 버전 · OS · 마지막 명령 id)에 쓰고 다음 기동 때 안내. dir2에 이미 있다(`nexa-dir2/crates/nexa-app/src/win.rs:1410`) | Release는 `panic = "abort"`(CI-072)라 훅이 없으면 흔적이 없다. nexa-sql에는 앱 수준 훅이 없다(`set_hook` grep 0건) → dir2 것을 계승 |
| CI-113 | dir3 `ci.yml` | 3-OS 매트릭스 · 형제 2 체크아웃 · fmt → clippy → test(T1~T3 포함) → `--smoke` → `--selfcheck --ci` → `wasm32-unknown-unknown` 플러그인 빌드 검증(dir2 `ci.yml` 계승) → (Windows) 임포트 게이트 · 예산. 별도 워크플로 `e2e`(수동·main): Linux `xvfb-run` + `ndir-check` | CI-046 + dir2 CI의 플러그인 빌드·예산 단계 |
| CI-114 | dir3 `release.yml` | nexa-sql 구조(meta → package 매트릭스 + 설치 스모크 → 초안 publish) 그대로. 채널은 §3-4 결정 뒤 | CI-052~066 |
| CI-115 | 로컬 전체 게이트 | `scripts/check-all.sh`(+ `.ps1` 래퍼): nexa-ui → nexa-license → nexa-dir3 순으로 fmt · clippy · test → check-3os → `--smoke` → `--selfcheck` → `ndir-check` → `summary.txt` | `linux-all-tests.sh`(`:15-43`)의 3-OS 일반화. **push 전 자동 진행의 문지기** |
| CI-116 | 검증 매트릭스 | `docs/port/9x-verification-matrix.md`(제안): 행 = 포트 문서 ID(WINA·WINB·WINC·PANEL·GUI·PREFS·DLG·RENDER·TERM·SHELL·PLUG·CLOUD·OPS·PROC·SKEL·LIC·EXT), 열 = 구현 위치 · 시험 층(T1~T5) · 시험 이름/시나리오 id · 상태. 규칙 "기능 ID 1개 = 시험 1개 이상 또는 '실기 필요' 사유" | 사용자 요구 "교차 검증 · 누락 없이". CI-099의 승격. 교차 검증 단계가 이 표의 빈칸을 센다 |
| CI-117 | dir2 패리티 시험 | dir2의 순수 로직 시험 425개를 크레이트 이식과 함께 가져오고(코어·ops·tree·term·vfs), 이식이 달라지는 부분은 "dir2의 기대값 = dir3의 기대값" 고정물로 남긴다 | "nexa-dir2의 기능을 유지" 검증의 가장 싼 수단 |
| CI-118 | 성능 간소 절차 | 시나리오: ① 기동 ② 대량 폴더(1만·10만 항목) 열기·정렬·스크롤 ③ 듀얼 패널 + 탭 8 ④ 터미널 대량 출력 ⑤ 미리보기 연속 전환 ⑥ 대량 복사 진행 중 UI 프레임 · 누수 주기 = 폴더 진입/복귀 · 탭 여닫기 · 터미널 여닫기 · 미리보기 여닫기 | CI-094·095의 dir3판. 예산 수치는 §3-4에서 재결정 |
| CI-119 | 규칙 문서 dir3판 | `docs/16`(복사) + `docs/61` 대응 문서(OS 분기 원장 · 격리·안전 · 검증 도구 · 자동 진행 push 규칙) + `CLAUDE.md` 축약 | §2 |
| CI-120 | 하네스 권한 | `.claude/settings.json`은 사용자 승인으로 생성·병합(에이전트가 덮어쓰지 않음) | CI-035 |

### 5-2. 층별 책임과 어디서 도는가

| 층 | 무엇을 | 도구 | CI | push 전 로컬 |
|---|---|---|---|---|
| T0 | 포맷 · 경고 0 · 3-OS 컴파일 | fmt · clippy · `check-3os.sh` | ✅ 3-OS | ✅ |
| T1 | 순수 로직: 정렬·필터·이름 규칙·경로·작업 계획(`ops`)·VT 파서·설정 레지스트리·i18n·라이선스 `check`·OS 분기 판정 함수(MC/DC) | `cargo test` | ✅ | ✅ |
| T2 | 컨트롤(nexa-ui 쪽): 히트 테스트 x·y · 포커스 링 ≤ 1 · 클립 · 스크롤 · 키 이동 | `ProbeCtx`/`RecordCtx` | ✅(nexa-ui CI) | ✅ |
| T3 | 앱 시나리오: 폴더 열기 → 선택 → 명령(복사/이동/이름 바꾸기/삭제) → 가짜 플랫폼 호출 기록 · 레이아웃 골든 · 메뉴 항목 활성 | `AppCore` + `FakePlatform` + 임시 샌드박스 폴더 | ✅ | ✅ |
| T4 | 실제 프로세스: 창 생성 · present · 실제 라우팅 · 설정 영속 · 세션 복원 | `ndir-check` + 기동 명령 + 덤프 | ◐(Linux xvfb · 수동) | ✅(호스트 OS) |
| T5 | 실제 OS 자원: 셸 · PTY · 휴지통 · 클립보드 · 컨텍스트 메뉴 · 플러그인 · 라이선스 | `--selfcheck` | ✅(`--ci` 부분집합) | ✅ |
| T6 | 성능·누수·용량 | perf 스크립트 | ✗ | 마일스톤마다 |

### 5-3. 자가 점검(`--selfcheck`) 항목 초안

| 그룹 | 점검 | 판정 | `--ci`에서 |
|---|---|---|---|
| env | OS · 아키텍처 · 버전 · 빌드일 · 홈 폴더 쓰기 가능 · 임시 폴더 쓰기 가능 | FAIL = 쓰기 불가 | 실행 |
| config | 설정 레지스트리 로드 · 기본값 전수 유효 · 저장 → 재적재 왕복(격리 홈) · 옛 키 이주 표 | FAIL = 왕복 불일치 | 실행 |
| resources | 내장 아이콘 전수 디코드 · 문자열 표(언어별 누락 0) · 샘플 자원 | FAIL = 누락·디코드 실패 | 실행 |
| fonts | UI·고정폭 글꼴 탐색 · 한글 지원 | FAIL = 없음 | 실행 |
| fs | **임시 샌드박스**에서 만들기·복사·이동·이름 바꾸기·삭제 · 긴 경로 · 유니코드(한글·NFD) 이름 · 대소문자 구분 여부 감지 · 폴더 감시 통지 | FAIL = 결과 불일치 · WARN = 감시 미지원 | 실행 |
| trash | 샌드박스 파일을 휴지통으로 → 존재 확인(복원/정리는 플랫폼별) | SKIP = 백엔드 없음(사유) | SKIP(헤드리스) |
| shell | 터미널 셸 탐지 결과(Windows = `pwsh` → 없으면 폴백 · macOS/Linux = `$SHELL` → `/bin/sh`) · 실행 파일 존재 | FAIL = 실행 가능한 셸 0 · WARN = 폴백 사용 | 실행 |
| pty | PTY 생성(Windows ConPTY · Unix openpty) → 셸에 `echo <표식>` → 출력에서 표식 수신 → 종료 | FAIL = 표식 미수신 | 실행(러너에 셸 있음 — 추정) |
| ctxmenu | 셸 컨텍스트 메뉴 백엔드 가용성(Windows = 셸 확장 열거 · macOS/Linux = 대체 항목 공급자) · 샘플 파일의 항목 수 | WARN = 대체 메뉴만 | SKIP |
| clipboard | 텍스트·파일 목록 왕복(**opt-in** `--with-clipboard` — 사용자 클립보드를 덮어쓰므로 기본 SKIP) | FAIL = 왕복 불일치 | SKIP |
| dnd | 드래그 소스/타깃 등록 가능 여부 | WARN | SKIP |
| open | 기본 앱으로 열기 수단 존재(`ShellExecute` · `open` · `xdg-open`) | WARN = 없음 | 실행(존재만) |
| preview | 텍스트·이미지 내장 미리보기로 고정물 디코드 | FAIL | 실행 |
| plugin | WASM 런타임 초기화 · 동봉 플러그인 적재 · 고정물 입력 → 기대 출력 · 연료/메모리 상한 동작 | FAIL = 적재·실행 실패 | 실행 |
| archive | 압축 목록(동봉 플러그인) 고정물 | FAIL | 실행 |
| license | 루트 키 등재 · 시험 벡터 서명 검증 · 기기 ID 산출 · 설치된 라이선스 상태 · 저장 경로 쓰기 가능 | FAIL = 키 없음·검증 실패 | 실행 |
| cloud | 구성 유효성(네트워크 호출 없음) | WARN | 실행 |
| window | (창 모드에서만) 표면 생성 · 첫 프레임 present · 배율 | FAIL | SKIP |

원칙: ① 점검은 **격리 홈·임시 폴더 안에서만** 쓴다(CI-029·034) ② 네트워크를 만들지 않는다 ③ 항목 하나의 실패가 다음 항목을 막지 않는다 ④ 각 항목 = 포트(CI-103) 구현의 공개 메서드를 **그대로** 호출한다(점검 전용 우회 경로 금지).

### 5-4. 회귀를 "바로" 보이게 하는 장치(요약)

1. **push 게이트**: `check-all`(CI-115)이 실패하면 push하지 않는다 — 자동 진행에서 사람 대신 막는 유일한 문.
2. **CI**: 3-OS에서 T0~T3 + 스모크 + 자가 점검 부분집합. 빨강이면 `gh run view --log-failed`로 원인부터(CI-031).
3. **검증 매트릭스**(CI-116): 구현 진척과 시험 유무를 포트 ID 단위로 한 표에서 — 빈칸 = 누락.
4. **실행 중**: 기동 점검 배지 + 기능 실패 사유 문구 + 크래시 기록(CI-111·112).
5. **지원**: Help ▸ 자가 점검 → 복사.

### 5-5. 위험 · 주의

| # | 위험 | 대응 |
|---|---|---|
| R1 | `AppCore`/`Shell` 분리(CI-102)는 nexa-sql 골격과 다르다 → 40번 문서(SKEL)의 "nexa-sql 구조 그대로"와 충돌 가능 | 상태·동작은 nexa-sql처럼 `app/<기능>.rs`에 두되(`nexa-sql/docs/93-code-health-and-refactoring.md:118-151`), **창 핸들·present만 바깥으로** 빼는 최소 분리. 불가하면 T3를 포기하고 T4를 CI(xvfb)로 올린다 |
| R2 | 파일 작업 자동 시험이 사용자 파일·휴지통·클립보드를 건드릴 수 있다 | 샌드박스 강제 · 휴지통/클립보드는 가짜 포트(T3) + opt-in 자가 점검(T5)만 |
| R3 | 형제 저장소가 ref 고정 없이 체크아웃된다(CI-043) → nexa-ui가 앞서거나 뒤처지면 dir3 CI가 깨진다 | push 순서 규칙(nexa-ui → nexa-license → dir3) + 실패 시 형제 CI부터 확인. 필요하면 워크플로에 `ref:` 고정(제안) |
| R4 | `cfg` 분기 코드의 `dead_code`·경로 오류는 한 OS에서 안 보인다(`docs/93:89` · `docs/16:112`) | `check-3os.sh` 교차 clippy + 3-OS CI. OS 분기는 포트 구현 파일에만 둔다 |
| R5 | CI 러너에서 PTY·셸 점검이 환경에 따라 흔들릴 수 있다(추정) | `--ci`에서 FAIL 대신 WARN으로 시작 → 안정 확인 뒤 FAIL로 승격 |
| R6 | 시험이 띄운 스레드가 프로세스 종료와 경합(nexa-sql T-191·T-238) | Drop에서 join 규칙(CI-082) |
| R7 | 기동 명령 접두 충돌·쉼표 구분 함정(CI-090) | 명령 id 표를 한 파일에 두고 접두 중복 시험 1건 |
| R8 | Windows 배포 채널·예산이 미정(§3-4) | 구현 착수 전 DR로 확정(23번 문서와 함께) |
| R9 | nexa-sql 기능 점검의 좌표 하드코딩 방식을 그대로 옮기면 레이아웃 변경마다 전부 깨진다 | 좌표 대신 `layout.dump`의 **영역 이름**으로 클릭 대상 지정(`ui.click:@<영역>` — 제안) |
| R10 | 자동 진행 중 push가 사용자 규약(자동 push 금지)과 다르다 | CI-019대로 사용자 발언을 근거로 DR에 기록 · 태그·강제 push는 제외 |
