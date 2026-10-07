# 25 · 성능 · 누수 · 코드 건강 점검 절차서 (T-176 · nexa-sql docs/71 · 93 차용)

> 작성 2026-10-08(사용자 10-05 "nexa-sql의 테스트 목적 · 과정 · 기법 · 정리 방향을 확인하고 차용해서 다시 테스트" · 조사 = [journal 10-05 §67](journal/2026-10-05.md)).
> [18 빌드·테스트](18-build-and-test.md)가 *무엇을 언제 돌리나*(하네스 7층 · 게이트)라면, 이 문서는 **T6(성능 · 누수 · 용량)** 과 **소스 자체의 건강**을 *같은 기준으로 되풀이해 재는 법*이다.
> 원천 = nexa-sql [docs/71](../../nexa-sql/docs/71-performance-review-process.md)(성능) · [docs/93](../../nexa-sql/docs/93-code-health-and-refactoring.md)(코드 건강). dir3에 맞게 줄였다 — 실서버 · 결과 그리드 · 향상 모드 A/B 같은 nexa-sql 전용 단계는 뺐다.

---

## 0. 원칙 다섯 줄

1. **같은 기기 · 같은 빌드 종류(Release) · 전/후 A/B** — 숫자는 PC마다 다르다. 비교는 이 PC의 기준선(`target/perf/baseline.json`)과만.
2. **입력 주입 0 · 포커스 탈취 0** — 동작은 기동 명령(`NDIR_STARTUP_CMD` · `@after:<ms>:<명령>`)으로만 건다([18 §10](18-build-and-test.md)). 격리 홈(`NDIR_HOME`)만 쓴다.
3. **0 = 측정 실패** — 0 ms · 0 B는 값이 아니다. 다시 잰다(perf-baseline은 ⚠로 표시하고 `--strict`면 실패).
4. **빌드 직후 첫 실행은 버린다**(파일 캐시 · 백신) — perf-baseline은 예열 1회를 뺀다 · 누수 주기는 첫 1~2주기를 뺀 **뒤 절반 기울기**로 본다.
5. **회귀는 설명할 때까지** — 회귀선(§3)을 넘으면 원인을 지목할 때까지 §2 G로 내려간다. 넘지 않아도 "왜 늘었는지 설명할 수 있는가"가 진짜 기준이다.

---

## 1. 규정 대상 — 차원(원장)

| # | 차원 | 재는 것 | 도구(§2) |
| --- | --- | --- | --- |
| D1 | 용량 | release exe 바이트(예산 ≤ 10 MB · CI budget) | perf-baseline ④ |
| D2 | 기동 | `--smoke`(창 없는 기동 경로) 벽시계 중앙값 · **창이 보일 때까지** + 안정 뒤 CPU/Private | perf-baseline ① · `win-startup-probe.ps1`(§5-2) |
| D3 | 자가 점검 그룹별 ms | `--selfcheck --ci --json` 그룹 합(ctxmenu · license · fs …) | perf-baseline ③ |
| D4 | 파일 작업 | `--selfcheck --ci --only fs` 벽시계(생성·복사·이름·삭제 + 감시 통지) · 대량 전송 = [24](24-fast-copy.md) T6 | perf-baseline ② · `membench.ps1` |
| D5 | 상주 메모리 | 유휴 Private · 워킹셋 · 전용 워킹셋(작업 관리자 기준) · 핸들 · GDI · USER · 스레드 | 메모리 창 · `mem.dump:<파일>` · win-leak-cycle `base` 행 |
| D6 | 누수 | 같은 동작 N주기 뒤 절반 기울기(MB/주기) · 핸들 증가 | win-leak-cycle |
| D7 | 회수 | 큰 작업 뒤 힙 반납(`mem_after_job` · [힙 정리] · 유휴 트림) — 기준 → 올림 → 놓음 → 트림 뒤 **4점** | `win-mem-reclaim.ps1`(§5-3 · `mem.dump` 4회) · 메모리 창(실기) |
| D8 | 벤치 | 해시 5종 병렬(T-175) · 압축 풀기 스트리밍(T-179 D) | `cargo test --release -- --ignored bench_` |
| D9 | 코드 건강 | 참조 0 pub · 미사용 번역 키/설정 키/의존 · 큰 파일/함수 · 중복 · 커버리지 | code-health(§6) |

---

## 2. 순서 — A부터 G까지(마일스톤 · 배포 전 · "성능 다시 점검" 요청 때)

| 단계 | 할 일 | 명령 | 끝 조건 |
| --- | --- | --- | --- |
| A 인벤토리 | exe 크기 · crate 수 · 설정 키 수 · 번역 키 수 | perf-baseline ④ · code-health 요약(`lines` · `unused_*`) | 수치 기록 |
| B 기동 · 자가 점검 | `--smoke` 중앙값 · fs · 그룹별 ms | `bash scripts/perf-baseline.sh --baseline target/perf/baseline.json` | 회귀 0건(§3) |
| B-2 창 기동 | 창이 보일 때까지 ms · 안정(3 s) 뒤 CPU · Private · 핸들 · 스레드 중앙값 | `pwsh -NoProfile -File scripts/win-startup-probe.ps1 -HomeDir C:\tmp\ndir-probe -Runs 5` | 기준선 +20 % 안 · 상주 +5 %/+1 MB 안 |
| C 상주 | 메모리 창을 열어 유휴 60 s 뒤 값(또는 `mem.dump`) | `NDIR_STARTUP_CMD="@after:60000:mem.dump:C:\tmp\mem.txt,@after:61000:quit"` · win-startup-probe `priv` 열 | 전용 워킹셋 · 핸들 · 스레드가 기준선 ±5 % |
| D 피크 · 회수 | 올림(큰 폴더 · 미리보기 · 큰 작업) → 놓음 → 유휴 트림 뒤 4점 · 큰 작업(압축 풀기 · 체크섬 · 중복 찾기)은 실기 [힙 정리] 전/후 | `pwsh -NoProfile -File scripts/win-mem-reclaim.ps1 -HomeDir C:\tmp\ndir-reclaim -Raise "nav:C:\Windows\System32" -Release "nav:C:\Windows"` · 메모리 창(실기) · 10-05 §72~§74 표 | 트림 뒤 − 기준 ≤ +1 MB · +5 %(WARN) |
| E 누수 주기 | 같은 동작 10주기(큰 폴더 ↔ 작은 폴더 · 보조 창 열고 닫기 · 미리보기 교체) | `pwsh -NoProfile -File scripts/win-leak-cycle.ps1 …`(§5) | 뒤 절반 기울기 ≤ 0.5 MB/주기 · 핸들 증가 0 |
| F 벤치 | 해시 · inflate | `cargo test --release -p ndir-ops -- --ignored bench_hash --nocapture` | 기준선 −20 % 안 |
| G 병목(회귀 때만) | `NDIR_TRACE_FRAMES=1` · 로그 창 계측 · 메모리 창 영역별 ▲ | — | 원인 지목 + 조치 또는 "설명 가능" 기록 |

**기준선 갱신** = 회귀가 없고 변경을 설명할 수 있을 때만: `bash scripts/perf-baseline.sh --save-baseline` · `python scripts/code-health.py --save-baseline`.

---

## 3. 판정 기준 · 회귀선

| 지표 | 예산 | 회귀선(같은 PC 기준선 대비) |
| --- | --- | --- |
| exe 크기 | ≤ 10 MB(CI budget) | **+5 %** |
| 기동 `--smoke` 중앙값 | 이 PC 참고치(§8) | **+20 %** |
| `--only fs` · 자가 점검 그룹(기준 ≥ 200 ms인 그룹만 — 작은 그룹은 잡음) | — | **+20 %** |
| 상주(유휴 Private · 전용 워킹셋) | 10-05 §74 뒤 select Private ≈ 11 MB | **+5 % 또는 +1 MB** |
| 유휴 CPU | ≈ 0(폴링 = 유휴 틱 `WaitUntil`만 · docs/01 §3) | **2배** |
| 누수 기울기(뒤 절반) | ≈ 0 | **> 0.5 MB/주기 또는 핸들이 주기마다 증가** |
| 벤치(해시 · inflate) | 10-05 §65 · 10-08 §2 | **−20 %(느려짐)** |
| 측정값 0 | — | **측정 실패 → 다시 잰다**(값으로 쓰지 않는다) |

회귀선을 넘으면 `perf-baseline.sh --strict`가 종료 코드 1 · win-leak-cycle은 `WARN`. 게이트(`gate.sh`)에는 넣지 않는다 — T6는 마일스톤 · 배포 전 · 요청 때(18 §4). CI는 exe 예산만 본다.

---

## 4. 도구 — perf-baseline(`scripts/perf-baseline.sh` · 3-OS · bash + python)

```bash
bash scripts/perf-baseline.sh                                  # 표(이번 수치) · target/perf/last.json
bash scripts/perf-baseline.sh --save-baseline                  # 이번 수치 = 이 PC 기준선(target/perf/baseline.json · git 미추적)
bash scripts/perf-baseline.sh --baseline target/perf/baseline.json [--strict]   # 기준선 열 + Δ + "회귀 N건" · --strict = 회귀/측정 실패면 exit 1
bash scripts/perf-baseline.sh --runs 10 --debug                # 회차 수 · debug 빌드
```

- 벽시계 = `date +%s%N`(GNU date · Git Bash) — 종전 `python -c` 2회가 회차마다 파이썬 기동 수십 ms를 `--smoke` 값에 섞었다(10-05 §67 ⚠ · nexa-sql `cli-wall.py` 교훈). `%N`이 없는 BSD date(macOS)는 perl `Time::HiRes`.
- 예열 1회 제외 · 중앙값 · 측정 실패(0 ms) ⚠ 표시 · JSON(`at` · `head` · `smoke_runs_ms` · `groups_ms` · `exe_bytes`).
- 출력 표는 journal에 그대로 붙인다(📌 §7 형식).

---

## 5. 도구 — 누수 주기(`scripts/win-leak-cycle.ps1` · Windows · 입력 주입 0)

```powershell
New-Item -ItemType Directory -Force C:\tmp\ndir-leak | Out-Null
# 큰 폴더 ↔ 작은 폴더 이동 10회(목록 적재 · 해제 되풀이)
pwsh -NoProfile -File scripts/win-leak-cycle.ps1 -HomeDir C:\tmp\ndir-leak -Cycles 10 -PeriodMs 4000 -CycleCmds "nav:C:\Windows\System32;nav:C:\Windows"
# 보조 창 열기 → 닫기(토글 명령) · 미리보기 교체
pwsh -NoProfile -File scripts/win-leak-cycle.ps1 -HomeDir C:\tmp\ndir-leak -CycleCmds "view.memory;view.memory" -PeriodMs 2000
pwsh -NoProfile -File scripts/win-leak-cycle.ps1 -HomeDir C:\tmp\ndir-leak -First "nav:C:\tmp\big" -CycleCmds "list.select:0;list.select:1" -PeriodMs 3000
```

- 주기 i의 동작을 주기 앞 60 %에 고르게 놓고 뒤 40 %는 쉬게 둔 뒤 그 끝에서 Private · WS · 핸들 · GDI · USER · 스레드를 찍는다. 마지막 측정 뒤 `quit`으로 스스로 끝난다(이름으로 죽이지 않는다 · 18 §10).
- 판정 = `slope(last half)`(MB/주기) · 핸들 증감 · `OK`/`WARN`(§3). 첫 1~2주기 상승은 캐시 · 글리프 적재(정상).
- Linux · macOS 실행기는 T-176 ⑤(후속) — 같은 기동 명령으로 `ps -o rss,nlwp` 표본.

### 5-2. 창 기동 프로브(`scripts/win-startup-probe.ps1`)

```powershell
pwsh -NoProfile -File scripts/win-startup-probe.ps1 -HomeDir C:\tmp\ndir-probe -Runs 5 [-SettleSecs 3] [-Cmd "nav:C:\Windows\System32"]
pwsh -NoProfile -File scripts/win-startup-probe.ps1 -HomeDir C:\tmp\ndir-probe -Exe "C:\Program Files\Nexa Dir\nexa-dir.exe" -Tag installed
```

- `window` = `MainWindowHandle`이 생길 때까지(창 생성 · 첫 present 직전) · `cpu(Ns)` = 안정 시각까지 쓴 CPU ms · `priv` · `ws` · `handles` · `threads` = 그때 값 · 마지막 줄 = 중앙값. `NDIR_NO_ACTIVATE=1`(포커스 탈취 0) · 스스로 `quit`.
- 두 빌드 비교는 번갈아(`-Exe` · `-Tag`) · 첫 실행(캐시 · 백신)은 버린다.

### 5-3. 회수 4점(`scripts/win-mem-reclaim.ps1`)

```powershell
pwsh -NoProfile -File scripts/win-mem-reclaim.ps1 -HomeDir C:\tmp\ndir-reclaim -Raise "nav:C:\Windows\System32" -Release "nav:C:\Windows" [-SettleMs 3000] [-TrimSecs 5]
pwsh -NoProfile -File scripts/win-mem-reclaim.ps1 -HomeDir C:\tmp\ndir-reclaim -Raise "nav:C:\tmp\pics;list.select:0;list.select:1" -Release "nav:C:\Windows"
```

- 격리 홈 `settings.conf`에 `mem.idle_trim_s=<TrimSecs>`를 써 유휴 트림을 당긴다 · `mem.dump:<파일>` 4회(base · raised · released · trimmed) → 표(footprint · private_ws · resident · heap_used · heap_held) + 판정 `trimmed − base`(> +1 MB 또는 +5 % = WARN).
- 올림 명령은 기동 명령 어휘(18 §5)면 무엇이든 — 큰 작업(압축 풀기 · 체크섬 · 중복 찾기)은 기동 명령이 없어 실기 [힙 정리] 전/후로 본다.

### 5-4. 입력 지연 · 프레임 계측(`NDIR_TRACE_FRAMES=1` + `scripts/frame-stats.py` · 3-OS)

```bash
NDIR_HOME=/tmp/ndir-frames NDIR_TRACE_FRAMES=1 \
NDIR_STARTUP_CMD="@ready:nav:C:\Windows\System32,@after:1500:ui.press:down,@after:1600:ui.press:down,@after:1700:ui.press:down,@after:3000:quit" \
  target/release/nexa-dir.exe 2> /tmp/frames.log
python scripts/frame-stats.py /tmp/frames.log [--warm 2] [--json]
```

- 메인 창 프레임마다 stderr `[frame] n= wait= paint= present= size= backend=`(µs) — `wait` = 다시 그리기 **요청**(입력 처리 끝) → 그리기 시작 · `paint` = CPU 래스터 · `present` = 표면 → 창. 꺼져 있으면 비용 = 분기 1(기동 때 한 번 읽음).
- 집계 = 프레임 수 · 중앙값 / p95 / 최대 · 예산 초과 수(paint ≤ 8 ms · 합 ≤ 16 ms) · 예열 2프레임 제외 · 0프레임 = 측정 실패.
- 키 이동 개선(10-05 §68 · 60 ms 지연 후 정보/미리보기) 전후 비교처럼 **같은 기동 명령열**로 전/후를 잰다.

---

## 6. 코드 건강 — `scripts/code-health.py`(nexa-sql docs/93 §3 차용 · 외부 패키지 0)

```bash
python scripts/code-health.py                  # nexa-dir3 + ../nexa-ui + ../nexa-license(+ ../nexa-sql은 참조 말뭉치)
python scripts/code-health.py --cov            # + cargo llvm-cov 요약(수 분)
python scripts/code-health.py --save-baseline  # 이번 결과 = 기준선(target/code-health/baseline.json)
python scripts/code-health.py --baseline target/code-health/baseline.json   # 표에 기준선 열
# 산출: target/code-health/report.md · result.json(추적 안 됨) · 소요 ≈ 3분(중복 블록 해시)
```

| 기호 | 점검 | 판정 방식 | 흔한 오탐과 처리 |
| --- | --- | --- | --- |
| A | 무조건 `allow(dead_code/unused)` | 줄 grep(`cfg_attr` 조건부는 따로 셈 — OS별 코드는 정상) | 이유가 있으면 조건부(`cfg_attr`)로 좁힌다 |
| B | 참조 0인 `pub` 항목 | 세 저장소 + **nexa-sql 참조 말뭉치** 낱말 빈도 1 | 공용 API(nexa-ui · 라이선스 프로토콜 상수) → `ALLOW_PUB`에 이유와 함께 |
| C | 안 쓰는 번역 키 | `en.lang` 키가 어떤 .rs에도 `"키"`로 없음 + 다른 언어 파일의 **번역 누락**(en에만 있는 키) | 조립 키 · 레지스트리가 상수로 찾는 라벨(`pref.cat.` · `pref.grp.`) · 이식 대기 자원(`cloud.`) → `ALLOW_I18N_PREFIX` |
| D | 안 쓰는 설정 키 | `REGISTRY`의 `e!("키", …)`가 다른 .rs에 `"키"`로 없음 · 또는 **ndir-settings 안에서만** | 조립 키(`key.` · `window.`) · 이식 대기(`cloud.`) → `ALLOW_SETTING_PREFIX` |
| E | 안 쓰는 의존 | 크레이트 소스에 의존 이름(`-`→`_`)이 없음 | feature 활성 · 링크 전용 → `ALLOW_DEP` |
| F | 크기 | 3000줄 넘는 파일 · 150줄 넘는 함수 수 · 상위 목록 | 레지스트리 · 번역 표 · 시나리오는 예외로 읽는다 |
| G | 중복 블록 | 주석 · 빈 줄 · 괄호 줄을 뺀 10줄 창 해시(350자 이상) | 시험 코드 제외 |
| H | 커버리지 | `cargo llvm-cov` 크레이트별 줄 % | GUI 크레이트(nexa-dir)는 낮은 것이 정상(화면은 T3 · 실기가 덮는다) |

판정은 **후보**다 — 지움 · 시험 전용 · 유지(이유를 `ALLOW_*`에) 세 갈래로 사람이 가른다(docs/93 §4-1). 목적 있는 시험 자산(벤치 · 탐침 · 시나리오)은 청소 대상이 아니다.

---

## 7. 산출물 · 기록 위치

| 무엇 | 어디에 |
| --- | --- |
| 원자료 | `target/perf/{last,baseline,validate}.json` · `target/code-health/{report.md,result.json,baseline.json}` · 누수 주기 표준 출력(journal에 붙인다) — 저장소에 넣지 않는다 |
| 회차 해석 | 이 문서 §8에 날짜 · 차수로 새 절(수치 표 + 회귀/설명) |
| 📌 전수 기록 줄 | journal 그날 절 첫 줄에 `📌 T6 <날짜> <HEAD> · 기동 N ms · fs N ms · exe N B · 누수 기울기 N MB/주기 · code-health unused_pub N` 한 줄 |
| 발견 · 조치 · 남은 것 | journal → DEVLOG → STATUS → TODO · 검증 매트릭스 CI-118 · MEM-SAVE 행 |

새 기능을 넣을 때(docs/71 §5 차용 · 짧게): ① 스레드 · 폴링 · 캐시를 쓰면 **끄거나 상한을 두는 설정 키**(`REGISTRY` · HIDDEN 가능) ② 상주라면 왜 상주인지 한 줄 ③ UI 스레드에서 몇 ms인지 ④ 측정 시나리오가 없으면 §2 C/E 목록에 **늘리기만** 한다.

---

## 8. 실행 기록

### 8-1. 2026-10-08 · 189차 · Windows(첫 기준선 · T-176 ①②③ 도구 자체 검증)

- 도구 검증: perf-baseline `--runs 2` → 표 · JSON · 자기 자신을 기준선으로 `--strict` = 회귀 0건 exit 0 ✓ · win-leak-cycle 3주기(`nav` System32 ↔ Windows · 2.5 s) = base 11.87 MB → 12.55 · 12.61 · 12.20 · 기울기 −0.41 MB/주기 · 핸들 345 불변 · GDI 55 · USER 19 · 스레드 15 → OK ✓ · code-health 3저장소 ≈ 3분 ✓.
- 코드 건강 1차(허용 표 반영 전): 참조 0 pub 47 · 미사용 번역 키 120(그중 `cloud.*` 42 = 이식 대기 → 허용 · 나머지 = `bulk.*` 2차 대기(T-162) · `pref.taPos.*` · `mem.cat.*` 옛 키 등 후보) · 미사용 설정 키 10(`cloud.*` 6 = 대기 → 허용 · `license.gates` · `list.hide_empty_glyph` · `typeahead.scope` = ndir-settings 안에서만 → 확인 후보) · 미사용 의존 1 · 3000줄 넘는 파일 4 · 150줄 넘는 함수 49 · 중복 24.
- 코드 건강 2차(허용 표 반영 · `--save-baseline`): 참조 0 pub 47 · 미사용 번역 키 **78** · 번역 누락 0(ko · ja) · 미사용 설정 키 **3**(`license.gates` · `list.hide_empty_glyph` · `typeahead.scope` — ndir-settings 안에서만 · 확인 후보) · 미사용 의존 1(`nexa-dir/windows-core` — `windows` crate 짝 · 링크 전용 여부 확인 후보) · 3000줄 파일 4 · 150줄 함수 49 · 중복 24.
- ④ 도구 검증(release ad8764b · gate full과 동시 실행 = 잡음 있음): win-startup-probe 2회 = window 2442 → **77 ms**(1회차 = 캐시 + 동시 부하) · cpu(3 s) 266 ms · priv 11.07 MB · ws 33.4 MB · handles 330 · threads 14 ✓ · win-mem-reclaim(`nav` System32 → Windows · trim 5 s) = base private_ws 8.70 → raised 7.02 → released 6.98 → trimmed 6.98 MB · footprint 11.79 → 13.47 → 12.22 → 12.22 · 판정 OK ✓(trimmed = released — 5 s 트림이 안 돈 것인지 확인 후보 · `NDIR_NO_ACTIVATE` 창은 resident가 33.8 → 10.9 MB로 줄어 상주 비교는 `private_ws`/`footprint`로).
- gate full(d2c5120): fmt + 3-OS clippy ✓ · 시험 583/0 ✓ · **smoke 단계 실패** = T-178로 bin이 둘(nexa-dir · ndir)이 되어 `cargo run -p nexa-dir`가 `--bin`을 요구 → `default-run = "nexa-dir"`(Cargo.toml)로 처방 · 재실행.
- 기준선 수치 = 아래 표(커밋 뒤 release 재빌드 · `--save-baseline`).

(수치 표는 실행 뒤 이 자리에 붙인다.)
