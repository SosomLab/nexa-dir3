# 18 · 빌드 · 테스트 · 점검 하네스 (SSOT)

> 빌드·테스트·게이트·회귀 하네스·자가 점검 절차의 **단일 원천**. 절차를 바꾼 커밋에서 같이 고친다([16 §2-4](16-doc-git-conventions.md)). 설계 근거 = [port/44 §5](port/44-sql-ci-test-docs.md)(CI-101~120) · DR-10.

## 1. 전제

- 형제 저장소가 나란히: `../nexa-ui` · `../nexa-license`(path 의존) · 원본 `../nexa-dir2` · 기준 `../nexa-sql`(대조용).
- 툴체인 = `rust-toolchain.toml`(stable · 5타깃 std). 교차 타깃 std가 없으면 `rustup target add x86_64-apple-darwin x86_64-unknown-linux-gnu`.
- Windows MSVC 정적 CRT = `.cargo/config.toml`(`+crt-static` · `RUSTFLAGS` 환경 변수 금지).

## 2. 빌드

```bash
cargo build --workspace                 # Debug(의존 크레이트는 opt-level 2)
cargo build --release -p nexa-dir       # 배포 프로필(lto fat · panic abort · strip)
cargo run -p nexa-dir -- --version      # 버전 = 루트 Cargo.toml 하나
```

### 2-1. 동봉 플러그인(.wasm) 빌드

`plugins/sdk/plugins.list`(단일 출처)의 게스트 크레이트를 `scripts/plugin-build.sh`(mac/Linux) · `scripts/plugin-build.ps1`(Windows)이 `wasm32-unknown-unknown`으로 빌드해 `plugins/<이름>.wasm`(동봉본)에 복사한다. 사전 준비 `rustup target add wasm32-unknown-unknown`. 옵션 `--out-dir <폴더>`(스테이징) · `--skip-dist`(동봉본 유지). 동봉본은 dir2 dist **무수정**이 회귀 기준(DR-7)이므로 소스를 고친 뒤에만 갱신한다. 앱이 보는 폴더는 `NDIR_PLUGINS_DIR` → `<설정 폴더>/plugins` → `<exe>/plugins`(docs/port/20 §4-7).

## 3. push 전 게이트(순서 고정 — 빨강이면 push 금지)

```bash
scripts/check-3os.sh                            # T0: fmt + 호스트 clippy + 다른 두 OS clippy(-D warnings)
cargo test --workspace                          # T1~T3
cargo run -q -p nexa-dir -- --smoke             # 창 없음: 설정·i18n·자원·글꼴·플러그인 런타임·라이선스 루트 키
cargo run -q -p nexa-dir -- --selfcheck --ci    # T5 부분집합(표시·사용자 자원 필요 항목 SKIP)
```

한 번에: `scripts/check-all.sh`(T-05에서 작성 · nexa-ui → nexa-license → dir3 순으로 fmt·clippy·test → 위 4단계 → `tests/out/summary.txt`).

## 4. 하네스 7층

| 층 | 무엇 | 도구 · 위치 | CI | 비고 |
| --- | --- | --- | --- | --- |
| T0 | 포맷 · 경고 0 · 3-OS 컴파일 | `cargo fmt` · `clippy -D warnings` · `scripts/check-3os.sh` | ✅ 3-OS | |
| T1 | 순수 로직(정렬·필터·이름 규칙·경로·ops 계획·VT 파서·설정 레지스트리·i18n·라이선스 check·OS 분기 판정 MC/DC) | `#[cfg(test)]` · dir2 테스트 이식(DR-11) | ✅ | 실제 설정 폴더·사용자 파일 금지(`temp_dir()`/`from_text`) |
| T2 | 컨트롤(nexa-ui 쪽): 히트 x·y · 포커스 링 ≤ 1 · 클립 · 스크롤 · 키 이동 | nexa-ctl `RecordCtx`(T-21에서 공용화) | ✅(nexa-ui CI) | 글자 폭 = 글자 × 7 고정 |
| T3 | **앱 시나리오(창 없음)**: `AppCore::new(FakePlatform, 샌드박스)` → `route(InputEvent)`/`command(id)` → 상태·가짜 호출 기록·**레이아웃 덤프 골든**(영역 이름 + Rect 트리 · 픽셀 아님) | `crates/nexa-dir/tests/scenarios_*.rs` · 골든 `tests/golden/*.layout` | ✅ | "컨트롤 배치 유지" 자동 검증 수단 |
| T4 | 실제 프로세스: 창 · present · 설정 영속 · 세션 복원 | `ndir-check`(T-06) + `NDIR_HOME` + `NDIR_STARTUP_CMD` + `*.dump:<파일>` + 검사식 `파일:정규식`/`!` · 시나리오 `tests/scenarios/*.scn` | ✅ Windows(CI 단계 `scenarios`) · ◐ Linux xvfb·macOS 후속 | 고정 sleep 대신 `@ready`/`@idle`/`quit` · 로컬 = `cargo build -p nexa-dir && cargo run -p ndir-check` |
| T5 | 실제 OS 자원: 셸 · PTY · 휴지통 · 클립보드 · 컨텍스트 메뉴 · 플러그인 · 라이선스 | `nexa-dir --selfcheck [--ci] [--json] [--only 그룹] [--with-clipboard]` · Help ▸ 자가 점검 창(같은 함수) | ✅(`--ci`) | 격리 홈·임시 폴더 안에서만 · 네트워크 0 · 포트 공개 메서드를 그대로 호출 |
| T6 | 성능·누수·용량 | `scripts/perf-*.{ps1,sh}`(T-08) | ✗ | 마일스톤마다 · Release |

## 5. 기동 명령 · 환경 변수(T4·T5 · 앱 훅)

| 이름 | 뜻 |
| --- | --- |
| `NDIR_HOME` | 설정·세션·라이선스·플러그인 폴더 재지정(격리) |
| `NDIR_STARTUP_CMD` | 쉼표 구분 명령: `<명령id>` · `@ready:<명령>`(첫 프레임 + 초기 열거 뒤) · `@idle:<명령>`(작업 큐 빈 뒤) · `@after:<ms>:<명령>` · `quit[:코드]` · `assert.<대상>:<식>` · `ui.click:@<영역>`/`ui.key:<조합>` · `ui.type:<text>`(`\b` = Backspace) · `ui.press:<enter|escape|up|down|left|right|home|end|pageup|pagedown|delete|space>` · `list.select:<n>` · `dock.kind:<n>` · `term.focus` · `term.send:<text>` · `ops.cancel` · `dlg.pick:<id>`/`dlg.type:<text>` · `prefs.search:<q>`/`prefs.cat:<키>` · `<영역>.dump:<파일>`(layout·panel·list·tabs·status·menu·prefs·dock·preview·term·ops·dlg·check) |
| `NDIR_NO_ACTIVATE` | 창을 앞으로 가져오지 않음(사용자 작업 방해 금지) |
| `NDIR_TRACE_FRAMES` / `_IME` / `_CLIP` / `_WINDOW` | 추적 로그 |
| 덤프 어휘 | `layout` · `panel` · `list` · `tree` · `tabs` · `status` · `menu` · `ops` · `term` · `preview` · `prefs` · `plugin` · `license` · `log` — 컨트롤·패널마다 `dump()` 하나가 구현 요건 |

## 6. 자가 점검(`--selfcheck`) 항목

> 창: **Help ▸ 자가 점검…**(`help.selfcheck` · T-54) — 같은 `selfcheck::run`을 작업 스레드에서 돌려 표로 보이고 [복사]는 CLI 표 텍스트와 동일. 기동 명령 `help.selfcheck` + `check.dump:<파일>`/`assert.check:<식>`으로 시나리오 검증(`selfcheck-win.scn`).

| 그룹 | 점검 | `--ci` |
| --- | --- | --- |
| env | OS·아키·버전·빌드일 · 홈/임시 폴더 쓰기 | 실행 |
| config | 레지스트리 로드 · 기본값 전수 유효 · 저장→재적재 왕복(격리) · 이주 표 | 실행 |
| resources | 내장 아이콘 전수 디코드 · 언어 파일 3종 키 파리티 · 샘플 | 실행 |
| fonts | UI·고정폭 글꼴 · 한글 지원 | 실행 |
| fs | 샌드박스에서 만들기·복사·이동·이름 바꾸기·삭제 · 긴 경로 · 유니코드(NFD) · 대소문자 감지 · 감시 통지 | 실행 |
| trash | 샌드박스 파일 휴지통 이동 | SKIP |
| shell | 기본 셸 탐지 · 실행 파일 존재 | 실행 |
| pty | PTY 생성 → `echo <표식>` → 수신 → 종료 | 실행(WARN으로 시작) |
| ctxmenu | 백엔드 가용성 · 샘플 항목 수 | SKIP |
| clipboard | 텍스트·파일 왕복(`--with-clipboard` opt-in) | SKIP |
| dnd · open | 등록 가능 여부 · 열기 수단 존재 | SKIP / 실행 |
| preview · plugin · archive | 내장 디코드 · WASM 런타임 · 동봉 플러그인 적재 · 고정물 기대 출력 · 연료/메모리 상한 | 실행 |
| license | 루트 키 등재 · 시험 벡터 검증 · 기기 ID · 설치 상태 · 경로 쓰기 | 실행 |
| window | (창 모드) 표면 생성 · 첫 프레임 · 배율 | SKIP |

출력 = 표(사람) 또는 `--json` · 항목 = PASS/FAIL/WARN/SKIP(사유) + ms · 종료 코드 = FAIL 수. 항목 하나의 실패가 다음을 막지 않는다.

## 7. 실행 중 진단(DR-10)

- 기동 점검(수 ms): 설정 읽기 실패 · 자원 누락 · 플러그인 폴더 · 셸 탐지 실패 · 라이선스 손상 → 상태줄 배지 + 로그 + 토스트.
- 패닉 훅 → `<HOME>/crash/<시각>.txt`(메시지·위치·버전·OS·마지막 명령 id) · 다음 기동 때 안내.
- 기능 실패(휴지통 불가 · PTY 실패 · 메뉴 백엔드 없음)는 사유 문구를 한 번만.

## 8. CI(`.github/workflows/ci.yml`)

3-OS 매트릭스 · 형제 2 체크아웃 · Linux 한글 글꼴 설치 · fmt → clippy → test → `--smoke` → `--selfcheck --ci` → `plugins` 잡(T-07 ✅: `plugin-build.sh` → `NDIR_PLUGINS_DIR` 자가 점검 `plugin` 로드 검증) → (Windows) 임포트 화이트리스트·용량 측정. 별도 `e2e.yml`(수동·main): Linux `xvfb-run` + `ndir-check`. 릴리스 = `release.yml`(M7).

## 9. 검증 매트릭스 · 교차 검증

[port/90-verification-matrix.md](port/90-verification-matrix.md): 행 = 이식 원장 ID · 열 = 구현 위치 · 시험 층 · 시험 이름/시나리오 id · 상태(☐/🚧/✅/의도된 차이/실기 필요). 규칙 "기능 ID 1개 = 시험 1개 이상 또는 사유". 마일스톤 끝마다 빈칸을 센다.

## 10. 실기 시험 규칙

- 격리: `NDIR_HOME=<임시>`로 띄우고 **격리 폴더의 `settings.conf` 수정 시각**과 실제 설정 폴더 불변을 확인.
- 입력 주입·포커스 탈취 금지(사용자가 자리에 있을 때) — 기동 명령으로 몰고 창 단위 캡처(Windows `PrintWindow` · macOS `screencapture -l`).
- 프로세스는 내가 띄운 PID만 종료(예외: 이 저장소 `target/` 아래 `nexa-dir.exe`).
- 사용자 클립보드를 덮어쓰는 명령은 자동 시험에 넣지 않는다.
