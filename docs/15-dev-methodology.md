# 15 · 개발 방법론 · 개발 기준 (프로젝트 관리 기준 · 코드 규칙 · OS 분기 · 컨트롤 규칙)

> 이 저장소의 **개발 규칙 SSOT**. 문서·git 규약은 [16](16-doc-git-conventions.md), 빌드·테스트·점검 절차는 [18](18-build-and-test.md), 결정은 [10](10-decision-record.md). 출처 = nexa-dir2 `docs/15` + nexa-sql `docs/61`(핵심 설계·작업 규칙)을 dir3 사정에 맞게 합침(2026-10-03).

## 1. 프로젝트 관리 기준

| 항목 | 기준 |
| --- | --- |
| 목표 | nexa-dir2 `0.22.0`의 **기능·배치·자원을 그대로** 3-OS에서 — "차이"는 의도된 것만([port/90](port/90-verification-matrix.md)에 "의도된 차이"로 등재) |
| 단위 | **수직 슬라이스** = 사용자가 관찰할 수 있는 얇은 끝단(예: "탭 바가 그려지고 클릭으로 전환"). 슬라이스 = 커밋 1개(+ 기록 1 트랜잭션) |
| 순서 | [MILESTONES](MILESTONES.md) M0 → M8 · 각 M 안은 [TODO](TODO.md)의 T-번호 순. 의존 역전 금지(예: 컨트롤이 없으면 먼저 nexa-ui에) |
| 원천 | 기능 = **이식 원장**([port/00](port/00-index.md) · 접두 ID) → dir2 코드 → dir2 문서. 골격 = nexa-sql. 원장에 없는 것을 발견하면 **먼저 원장에 `GAP-NNN`** 등재 후 구현 |
| 완료 정의(DoD) | ① 코드 ② 시험(단위/T3 ≥ 1 · 실기만 가능하면 사유) ③ 검증 매트릭스 행 ④ 게이트 green(`scripts/gate.sh` — 평소 auto(quick/full 자동 판정) · 중요 시점 full · quick으로 push했으면 CI 3-OS 결과 확인까지 · DR-26 · docs/18 §3) ⑤ 기록(journal → DEVLOG → STATUS → MILESTONES/TODO) |
| 교차 검증 | 마일스톤 끝마다 **원장 ID 전수 대조**(구현 위치·시험·상태) — 빈칸 = 누락. 결과는 journal + 매트릭스 |
| 자동 진행 | 사용자 개입 최소화 — 묻지 않고 진행: 저장소 내부 파일 · 격리 홈 · 스크래치 · `target/` · 내가 띄운 프로세스 · cargo/git/gh/스크립트 실행. **먼저 묻는 것**: 프로젝트 밖 사용자 자원 · 되돌릴 수 없는 git 작업 · 버전 태그 · 공개 릴리스 · 남이 띄운 프로세스 |
| 보고 | 한글 · 슬라이스마다 중간 보고(완료 + 남은 목록) · 지시 충돌은 "충돌·조정" 절로(나중 지시 우선) |
| 확인 · 결정 질문 | **Action Item 1개 = 질문 1개**(사용자 11-01): 확인이 필요한 사항은 원자 단위로 최대한 세분화해 묻고 묶어 묻지 않는다. 항목마다 **배경**(왜 지금 결정이 필요한가) · **선택지와 각각의 영향** · **권고와 이유**를 상세히 적는다. 결정이 여럿이면 질문을 여럿으로(한 번에 4개 넘으면 호출을 나눈다) · 서로 의존하는 결정은 순서를 밝히고 앞 결정 뒤에 다음 것을 묻는다 |

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
4. 순서 = **최신화**(`git -C ../nexa-ui fetch` → `git log HEAD..origin/main` 확인 → ff-pull · 상류에 같은 기능이 있으면 그것을 기본으로) → nexa-ui 커밋(**기능 단위 1개**) → nexa-ui 테스트 + **nexa-sql 빌드**(`cargo check -p nexa-sql`) → **즉시 push** → dir3 사용. 여러 기능을 한 push로 모으지 않는다 — 다른 PC·세션(nexa-sql · nexa-beep 등)이 같은 저장소를 쓰며 바꾸고 있을 수 있다(사용자 10-10). nexa-license도 같은 절차.
4-1. **소비자 기록(DR-24 · 사용자 10-03)**: nexa-ui · nexa-license를 고치는 커밋은 그 저장소 `docs/CONSUMER-CHANGES.md`에 한 줄을 더한다 — 종류(추가/**동작 변경**/수정/문서) · 소비자별 영향(nexa-sql · dir3 · 발급 도구) · 검증 방법(명령 + 화면에서 볼 곳) · **검증 상태를 정직하게**(`check` / `시험` / `실기` / `미검증`). `동작 변경`·`수정`이면 nexa-sql `cargo test --workspace`를 별도 target-dir(`target/dir3check` · 끝나면 삭제 · 실행 중 nexa-sql.exe 미접촉)로 돌린 결과까지 적는다. 인계 요약에도 "종류 · 소비자 영향 · 검증 방법"을 쓴다. 형제 저장소를 pull했을 때는 그 파일의 `동작 변경`·`미검증` 행부터 dir3 쪽에서 확인한다.
5. dir2 위젯 이식 시 **배치 수치·동작은 그대로**(DrawCtx 어휘만 어댑트 — 대조표 [port/17](port/17-dir2-rendering-text.md)·[port/14 §3](port/14-dir2-gui-widgets.md)).
6. 포커스 링 ≤ 1 · 마우스는 커서 아래 컨트롤에만 · 팝업은 표면 밖으로 나가지 않음 · 바깥 클릭은 닫고 흘림 · hover는 `IntentFade`(nexa-sql 규칙 계승 — [port/40 §4](port/40-sql-app-skeleton.md)).

### 2-3. OS 분기 규칙(DR-5)

1. `#[cfg(target_os)]`는 **`platform/` 모듈 안에서만**. 그 밖의 분기는 `cfg!(target_os)` 상수 판정(설정 OS 기본값 · 주 조합키)으로 한정하고 `OS_DEFAULTS` 표·`primary_mod()` 같은 한 자리에 모은다.
2. 포트 trait마다 `fake.rs` 구현(기록·주입) — T3 시나리오는 가짜로, 실제 구현은 `--selfcheck`로.
3. 분기 조건 ≥ 2이면 순수 함수로 뽑고 **MC/DC 시험**(조건별 독립 영향 쌍).
4. 미지원은 **조용히 넘기지 않는다**: `Err(Unsupported("…"))` → 상태줄/토스트 한 번 + 로그. 메뉴 항목은 비활성(숨기지 않음 — 배치 유지).
5. 교차 clippy(`check-3os.sh`)를 push 전에 — `cfg` 모듈 경로 실수는 다른 OS에서만 드러난다. `allow(dead_code)` 무조건 금지 · OS별은 `cfg_attr` + 이유.
6. OS별 차이 원장 = [port/19](port/19-dir2-shell-integration.md)(셸·클립보드·DnD·휴지통·감시) · [port/18](port/18-dir2-terminal.md)(PTY·셸) · [port/40 §3](port/40-sql-app-skeleton.md)(창·입력·IME·클립보드) — 새 분기를 만들면 해당 문서 표에 한 줄.
7. **시험의 `cfg!(windows)`(OS별 기대값) 분기는 push 전에 다른 OS 쪽 의미를 제품 코드와 대조한다** — 개발 PC gate는 호스트 OS 시험만 실행하고 다른 두 OS는 clippy뿐이라, 틀린 비호스트 기대값은 CI에서야 드러난다(10-24 af63f73 T-196 `fav_file_round_trip_and_merge`: `same_fav`는 끝 구분자 차이를 모든 OS에서 같게 보는데 unix 기대값이 "다르다"로 적혀 ubuntu·macOS 빨강 → db92084 정정). 대조 근거(어느 함수가 OS별로 갈리는지)를 시험 주석에 한 줄.
8. **자식 프로세스는 콘솔 창을 띄우지 않는다**(사용자 11-02): 앱이 내부적으로 띄우는 명령줄 도구(상태줄 git · 열기/보기 · 미리보기 · 테마/설정 조회 · 동기화 등)는 `platform::quiet_command`(Windows `CREATE_NO_WINDOW` 0x0800_0000 · 다른 OS = 그대로)로만 만든다 — `std::process::Command::new` 직접 호출 금지 · `cmd /C` 경유 금지(GAP-010 메타문자). 예외(의도된 창) = 런처 `launcher.rs`(사용자가 고른 cmd/pwsh/프로그램) · PTY(`winpty.rs` ConPTY = 창 없음) · `ShellExecuteW` 열기 · 콘솔 보조 `bin/ndir.rs`. **정적 가드** = nexa-dir `platform::tests::command_spawn_sites_are_allowlisted`(파일별 `Command::new(` 건수 표 · 주석 제외) — 새 호출은 `quiet_command`로 바꾸거나 표에 사유를 적어 등재. 자동 실행 경로(기동 · 셸 탐지(PATH 검색만 · 프로세스 없음) · 상태줄 git · 세션 복원)도 같은 규칙. 점검 11-02: Windows GUI에서 도는 생성 지점 = git(`quiet_command`) · `explorer /select,`(→ `quiet_command`로 통일) · ShellExecuteW · ConPTY · 런처 — 콘솔 창이 뜨는 경로 없음.

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

### 2-8. 시험 작성 함정(10-24~25 반복 실측)

- **이름 · 키 · 블록 판정은 토큰 일치**로 — `contains("mem")`는 `appmem`에, `contains("ctx.copy_name")`는 `ctx.copy_name_noext`에 걸린다(412dc0a 상태바 보충 규칙 · 440e521 순서 편집 시험). 구분자로 나눈 토큰 비교 또는 `==`.
- **시험은 프로세스 전역 상태를 단언하지 않는다** — `cargo test`는 같은 프로세스에서 병렬로 돈다. 전역 원자값 · static · 마지막 명령 기록(`crash::last_command`) · 전역 조절값(`fastcopy::set_tuning`) · 로그 마스크(`ndir_log` DETAIL_MASK)를 쓰거나 읽어 단언하면 다른 시험이 끼어든다(T-198 · f36a9ae · 7d89e05 · ecbf93c CI 흔들림). 처방 순서: ① 값을 인자로 넘김(`transfer_with(&Tuning)`) ② 시험 빌드에서만 스레드 지역(feature `test-local-mask` · `with_tuning`) ③ 단언을 이 시험이 만든 결과(보고 본문 · 반환값)로 바꿈 ④ 마지막 수단 = 그 시험군만 Mutex 직렬화.
- **픽스처 태그는 시험마다 고유**(f34cf61 "extract" 충돌) · **`thread_local!` 앞에 `///` 금지**(unused_doc_comments · 0096e50) · **`DEPENDS` 자식 키는 부모 뒤 등재**(1b7ed98) · **메뉴 항목 추가 = `MENU_SECTIONS` 함께**(T-200 처방 전까지).

### 2-9. 디스크 예산(사용자 11-02 · CLAUDE.md §5와 같은 문구)

- 개발 산출물(저장소·워크트리 `target/` · 벤치 자료 · 게이트 로그 · 편집기 색인)이 수십 GB를 차지하지 않게 관리한다. ① 회차 마감 때 `bash scripts/disk-check.sh`(크기 보고 · 예산 기본 30 GB `NDIR_DISK_BUDGET_GB` · `--trim` = 워크트리 target · `target/perf/bench-*` · tmp · 게이트 로그 · incremental · 교차 타깃 폴더 삭제 · 측정은 느리니 백그라운드) ② 워크트리(`_collab/*` · `*-wt-*`)에서는 **빌드하지 않는다**(target 중복) · 끝난 워크트리는 `git worktree remove` ③ 실기·벤치 자료는 끝나면 스스로 지운다(bench-copy · perf-baseline · T4 샌드박스) ④ Sublime은 저장소의 `nexa-dir3.sublime-project`로 열어 `target/`을 색인에서 제외(전역 `index_exclude_patterns`도 같게 · `*.sublime-workspace`는 커밋하지 않음) ⑤ 측정·기록 = 협업 세션 · 삭제 실행 = 개발 세션(사용자 확인). 게이트의 여유 5 GB 하한(`NDIR_GATE_MIN_FREE_GB`)은 그대로.
- **왜**: 사용자 "개발에 사용된 폴더가 Sublime에 캐시되거나 Clean 되지 않은 상태로 수십 GB를 차지하는 경우가 많아 과다한 디스크 사용이 되지 않도록 관리해줘. 규칙에 넣어줘"(11-02). 첫 측정(journal 11-02 §3) = `target/debug/incremental` 하나가 30.6 GB · 세션별 `CARGO_TARGET_DIR`(target/collab · dev) · 교차 타깃 메타데이터가 쌓인다.
- **세션별 `CARGO_TARGET_DIR`**(`target/collab` · `target/dev*` 같은 격리 빌드)도 같은 예산에 든다 — **협업 격리 빌드(`target/collab`)는 인스턴스 종료 뒤 trim 대상**(`--trim` ⑥ · 79b0ebd · 실행 중인 nexa-dir가 `target/` 아래에 있으면 ⑥만 건너뜀).
- **형제 · 원본 저장소**(nexa-sql · nexa-dir2 · nexa-ui)의 `target/`은 disk-check 보고에 들지만 `--trim` 범위 밖 — 첫 측정에서 nexa-sql 245 GB · nexa-dir2 38 GB(journal 11-02 §3) · 정리는 사용자 판단.
- **프로젝트 밖 임시 폴더/파일(사용자 11-03 "임시 폴더/파일을 사용한 경우 정리하는 작업을 배치해서 과다하게 누적되지 않도록")**: 시험 고정물(`core_tests::fixture` = `Temp/ndir-core-<이름>-<pid>` · ndir-vfs/tree/ops `nexa_*` · 미리보기 `nexa-preview*` · xfer `xfer-home-*`) · 스크립트 홈(`perf-baseline` `ndir-perf-*` · gate `/tmp/ndir-gate-check.*`) · 세션 스크래치(`Temp/claude/<프로젝트>/<세션>`)는 ① 만든 쪽이 끝나면 지운다(시험 = `remove_dir_all` · 스크립트 = trap) ② 그래도 남는 것(패닉한 시험 · 끊긴 세션)은 `disk-check.sh`가 보고하고 `--trim` ⑦이 **나이 가드**로 거둔다 — 고정물은 `NDIR_TMP_AGE_H`(기본 24) 시간, 스크래치는 7일 넘은 것만(게이트 full이 도는 동안 그 시험의 폴더가 Temp에 살아 있다 · 11-03 확인) ③ 프로젝트 밖에 임시 폴더를 새로 두는 코드는 위 접두사 중 하나를 쓰거나 disk-check `tmp_left()` 목록에 접두사를 추가한다(다른 프로젝트의 임시 파일은 건드리지 않기 위해 접두사 화이트리스트 방식) ④ 실행 시점 = 회차 마감 `disk-check.sh --trim`(규칙 ①과 같은 자리 · 별도 타이머 없음).

## 3. 이식 절차(한 기능)

1. 원장에서 ID를 고른다(예 `PANEL-031`) → dir2 원본 코드를 **읽는다**(요약 믿지 않음).
2. 필요한 컨트롤이 nexa-ui에 있는지 확인(없으면 §2-2 절차 먼저).
3. 플랫폼 중립 로직 → 라이브러리 크레이트 또는 `app/`; OS 의존 → `platform/` 포트.
4. dir2 테스트가 있으면 **그대로 이식**(DR-11) + 새 시험(단위/T3).
5. 커밋(`출처: nexa-dir2/crates/…:줄` + 원장 ID) → 매트릭스 행 → 기록.

## 3-1. Windows 개발 PC 함정(실측)

- **Git Bash heredoc이 백슬래시·따옴표를 망가뜨린다**(`\\n` → 실제 개행): 인라인 `python3 - <<EOF`·`perl -pi` 코드 패치는 "치환 2건"이라 보고하면서 파일은 그대로인 유령 현상이 난다(10-03 `.lang` 결함 수정에서 3회 헛돎). → 패치는 **스크립트 파일**(스크래치패드 `.py`)로 쓰고 바이너리 모드로 읽고 쓴 뒤 `grep -c`/`od -c`로 검증.
- **python 패치 스크립트의 `newline` 인자 오류는 파일을 먼저 비운다**(10-03 `filelist.rs` 0바이트 사고): 치환·assert가 전부 끝난 뒤 임시 파일에 쓰고 `os.replace` · `newline="\n"`(백슬래시 하나) · 사고 시 `git checkout -- <파일>`.
- 실행 중인 exe가 링크를 막는다 → 내가 띄운 `target/` 아래 `nexa-dir.exe`는 종료하고 진행.
- 두 세션(개발 · 협업)이 한 작업 트리를 쓸 때 실기 앱은 **커밋 그대로** 빌드한다 — 메인 트리엔 다음 기능의 미커밋 변경이 섞여 있어 `target/debug`는 혼합본이고, **형제 저장소(`../nexa-ui` · `../nexa-license`)도 개발이 작업 중이면 미커밋**이다(path 의존이라 dir3 worktree만 따로 두어도 형제는 섞임 · 10-25 nexa-ui 229) → 협업은 **형제 세트째 분리**: `D:/Projects/kiros33/_collab/{nexa-dir3, nexa-ui, nexa-license}` = 세 저장소의 detached worktree(원격 커밋 그대로 · `git -C _collab/<r> checkout --detach <커밋>`)에서 빌드하고 exe 사본(scratchpad)을 띄운다(링크 잠금도 피함 · 10-24 dir3만 분리 → 10-25 세트 분리).
- **두 세션이 한 작업 트리를 쓸 때 파일 소유(10-25 합의)**: docs/TODO.md · docs/22 · docs/port/90은 **개발이 기능 커밋에 함께** 바꾼다 — 협업이 넣을 행(등재 · ✅ 갱신)은 문구를 개발에 보내 다음 기능 커밋에 포함. 협업 단독 커밋 = journal · DEVLOG · STATUS · docs/15 · nexa-ui CONSUMER-CHANGES. 급히 개발 소유 파일을 바꿔야 하면 "지금 <파일> 씁니다" 선통지 → 상대 "OK"(저장 안 함) 뒤. 어느 쪽이든 공유 트리에서는 **쓰기 직후 `git diff <파일>`이 제 변경뿐인지 확인한 뒤** `git add <파일>`(10-25 1fcd1ea: 확인 직후 상대가 저장한 행이 섞여 push된 사고) · `pull --rebase --autostash`는 상대의 편집 중 파일을 건드리므로 쓰지 않는다.
- **gate 판정은 마지막 판정 줄로만 · 커밋을 그 줄에 묶는다(10-25 8b8bbdc 빨강 push)**: gate 출력 중간의 `test result: ok` 같은 줄을 보고 성공으로 오독해 빨강 커밋이 push됐다 → 판정은 gate.sh의 고정 문구 한 줄로만 — 성공 `★ GATE OK(quick) …` · `★ GATE OK(full) …` / 실패 `★ GATE 실패(<mode>): <단계>` — 커밋은 `bash scripts/gate.sh 2>&1 | tee <로그> ; grep -q "★ GATE OK" <로그> && git commit …`처럼 그 줄에 조건으로 묶는다.
- Git Bash에서 `target\x` 같은 백슬래시 경로 인자는 `targetx`가 된다 → 슬래시로.
- `autocrlf`: `.gitattributes`가 텍스트를 LF로 고정 · `.sh`는 체크아웃도 LF.

## 4. 금지 목록

- `git add -A`/`.` · 자동 태그 push · 강제 push · 사용자 자원 수정.
- `platform::quiet_command` 밖의 `std::process::Command::new`(콘솔 창 · §2-3 ⑧ · 예외 = 런처 · PTY · ShellExecuteW · `ndir`) · `cmd /C` 경유 실행.
- 앱 크레이트 안 범용 컨트롤 · `platform/` 밖 `#[cfg(target_os)]` · 문자열 리터럴 UI 문구 · 레지스트리 밖 설정 키 · 자체 타이머/스레드 틱 · 모달 중첩 루프 · 입력 주입(SendKeys 등) · 사용자 클립보드 덮어쓰기 시험 · 픽셀 골든 시험(글꼴 OS 차이).

## 5. 세션 분담 · 세마포어(사용자 11-05 · 11-06 "Fable 사용량 절감")

> 원본 = PC 전역 `~/.claude/CLAUDE.md`(이 PC의 모든 세션에 적용) · 이식용 사본 = `../nexa-sql/docs/claude-global-rules.md` + `../nexa-sql/scripts/install-global-rules.sh`(새 PC 첫 일 = `bash ../nexa-sql/scripts/install-global-rules.sh --check` → 없으면 인자 없이 설치). 이 절은 **dir3에서 덧붙는 것만** 적는다(충돌 시 사용자 지시 > 이 절 > 전역 원문).

**① 모델별 분담**

| 세션 | 맡는 일 |
| --- | --- |
| **개발(Fable)** | 꼭 더 나은 결과가 필요한 일만 — 설계 결정(DR/ADR · GAP 해결 방식) · 새 기능·다층 변경(크레이트 2개 이상 또는 ≈200줄 이상 · 형제 저장소 nexa-ui/nexa-license 컨트롤 추가) · 코어 rlib·`platform/`·설정 엔진·라이선스·플러그인 ABI·동시성(워커·세대 가드) · 성능/결함 **원인 분석** · 릴리스(버전 승격·태그·채널) |
| **협업(Opus · 필요하면 Agent 병렬)** | 그 밖의 전부 — 상태 점검(git · CI `gh run` · 프로세스 · disk-check 측정) · 기록(journal · DEVLOG · STATUS · MILESTONES · BRANCHES) · `gate.sh`·T4·`--selfcheck`·실기 인스턴스(`_collab/` 세트 · `NDIR_HOME` 격리) **실행** · 간단 수정(오타·문구 · i18n `.lang` 문자열 · 주석 · 단일 스크립트 · 설정 기본값 · fmt/clippy 위반 · 원인이 명확한 단일 함수 버그 ≤30줄) · 이식 원장·dir2 원본 조사 보고 |

- 일을 받은 세션이 위 표로 **즉시** 분류한다. 협업이 하다가 설계 판단이 필요해지면 멈추고 개발에 넘긴다. 개발은 사용자가 직접 준 간단 작업·점검도 협업에 `SendMessage`로 넘기고 결과만 중계한다(협업이 없을 때만 Opus Agent).
- 개발 세션은 설계 전 조사를 직접 읽지 않는다 — 질문 목록만 협업/`Agent(model: opus, subagent_type: Explore)`에 보내 요약을 받는다. 개발은 `cargo check -p <크레이트>`까지 · 그 뒤 `gate.sh`·CI 감시·기록은 협업에 `[P0~P3]` 위임.

**② 파일 소유(세마포어 기본 소유자)**

| 소유자 | 파일 |
| --- | --- |
| 개발 | `crates/**` · `tests/**` · `Cargo.*` · `.github/**` · `packaging/**`(빌드 정의) · `CLAUDE.md` · `.claude/` · 형제 저장소 코드 · **`docs/TODO.md` · `docs/22-dir3-features.md` · `docs/port/90-verification-matrix.md`**(기능 커밋에 함께 바꾼다 · 10-25 합의 계승 — 협업은 넣을 행 문구를 개발에 보낸다) · `docs/10`(결정 내용) |
| 협업 | 그 밖의 `docs/**`(journal · DEVLOG · STATUS · MILESTONES · BRANCHES · docs/15·16·18 · archive · release 노트 초안) · nexa-ui `docs/CONSUMER-CHANGES` · `scripts/`의 **실행**(수정은 간단 수정 범위 + 개발 SEM 승인) |

- 남의 파일은 `[SEM 요청] <파일> — <이유> — <범위>` → `[SEM 승인]`(그동안 소유자도 저장하지 않음) → 작업 → `[SEM 해제] <파일> <해시>`. 토큰은 파일당 하나. 소유자가 idle인데 10분 무응답이면 `[SEM 자동승인]`으로 진행하고 사용자 보고에 남긴다(busy면 기다림). 사용자가 특정 세션에 직접 지시한 편집은 그 지시가 승인을 대신한다(상대에 통지만).
- 공유 트리 커밋 = 자기 파일만 `git add <파일>` · 쓰기 직후 `git diff <파일>`이 제 변경뿐인지 확인(§3-1 10-25 사고) · `pull --rebase --autostash` 금지.
- **push(11-05 개정 · 11-01 "협업은 push 안 함"과 11-04 §1-6 "빌드·시험·게이트·push = 개발"을 대체 — 게이트·시험 실행은 협업 · 개발은 `cargo check`까지)**: 협업도 자기 소유 기록·간단 수정은 스스로 `fetch` → `git add <파일>` → commit(메시지 끝 `(협업 세션)`) → `pull --ff-only` → push. 코드가 섞인 push 전 게이트는 종전대로 `gate.sh`(★ GATE OK 판정 줄 조건). 태그는 개발 세션 · 사용자 승인.
- `target/` 빌드는 한 번에 한 세션 — 빌드 요청·완료 메시지로 묶는다(협업 실기는 `_collab/` 세트 · 끝나면 `--trim`).
