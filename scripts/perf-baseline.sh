#!/usr/bin/env bash
# perf-baseline.sh — 성능 기준선(T6 · T-91 · CI-118 간소 절차 ①②④ · T-176 ① 기준선 저장/비교) · 외부 도구 0(bash + cargo + python).
#   ① 기동: `nexa-dir --smoke`(창 없는 기동 경로 = 자가 점검 ci 부분집합) 벽시계 N회 → 중앙값 ms(앞 1회는 예열로 버린다)
#   ② fs 점검: `--selfcheck --ci --only fs` 프로세스 벽시계(생성·복사·이름 바꾸기·삭제 + 폴더 감시 통지)
#      ※ 대량 폴더(1만/10만 항목) 목록·정렬은 헤드리스 경로가 없어 미측정 — fs 점검은 자기 임시 폴더만 쓴다(T-08 잔여)
#   ③ 자가 점검 그룹별 ms(`--selfcheck --ci --json`)
#   ④ 산출물 크기: release exe 바이트
# 사용: scripts/perf-baseline.sh [--runs N] [--debug] [--save-baseline] [--baseline <json>] [--strict] [--out <json>]
#   기본 runs 5 · release 빌드 사용
#   `--save-baseline`   = 이번 수치를 target/perf/baseline.json에(PC별 · git 미추적) — 같은 PC의 다음 실행과 비교하는 기준
#   `--baseline <json>` = 그 기준선과 비교해 **회귀선**(docs/25 §3 · nexa-sql docs/71 차용: 기동 +20 % · fs +20 % · 그룹 +20 %(기준 ≥ 200 ms만) ·
#                         exe +5 %)을 넘으면 ⚠ 표시 + 마지막 줄 "회귀 N건"
#   `--strict`          = 회귀가 있거나 측정 실패(0 ms)면 종료 코드 1(게이트·CI에 끼울 때)
#   `--out <json>`      = 이번 수치 JSON을 그 경로에도(기본 target/perf/last.json)
# 출력: 마크다운 표(표준 출력) — journal에 그대로 붙인다. 숫자는 이 PC 기준 참고치(CI 비교는 상대값으로).
# 측정 교정(T-176 ①): 벽시계는 `date +%s%N`(GNU date · Git Bash 포함)으로 잰다 — 종전 `python -c` 2회가 회차마다 파이썬 기동(수십 ms)을
#   `--smoke` 값에 섞었다(10-05 §67 ⚠ · nexa-sql cli-wall.py 교훈). `%N`이 없는 date(macOS BSD)면 perl Time::HiRes로 대신한다.
#   중앙값 · JSON 집계만 파이썬. **0 ms = 측정 실패**(docs/25 §3 · nexa-sql docs/71:230) — 값으로 쓰지 않고 다시 잰다.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PYTHONIOENCODING=utf-8   # Windows 파이썬의 표준 출력 기본(cp949)으로 한글 표가 깨지지 않게
RUNS=5; PROFILE=release; SAVE=0; BASE=""; STRICT=0; OUT=""
while [ $# -gt 0 ]; do
    case "$1" in
        --runs) RUNS="$2"; shift ;;
        --debug) PROFILE=debug ;;
        --save-baseline) SAVE=1 ;;
        --baseline) BASE="$2"; shift ;;
        --strict) STRICT=1 ;;
        --out) OUT="$2"; shift ;;
        *) echo "unknown arg: $1"; exit 2 ;;
    esac
    shift
done
# Git Bash는 `[ -x x ]`를 x.exe로 암묵 해석한다 → .exe가 실제로 있으면 명시적으로 붙인다(경로·크기 표기 정확히).
exe_path() { local e="$ROOT/target/$PROFILE/nexa-dir"; [ -f "$e.exe" ] && e="$e.exe"; echo "$e"; }
EXE="$(exe_path)"
if [ ! -f "$EXE" ]; then
    echo "빌드 필요: cargo build --$PROFILE -p nexa-dir" >&2
    if [ "$PROFILE" = release ]; then (cd "$ROOT" && cargo build --release -p nexa-dir >/dev/null 2>&1)
    else (cd "$ROOT" && cargo build -p nexa-dir >/dev/null 2>&1); fi
    EXE="$(exe_path)"
fi
[ -f "$EXE" ] || { echo "exe 없음: $EXE"; exit 1; }
HOME_DIR="$(mktemp -d 2>/dev/null || echo "${TMP:-/tmp}/ndir-perf-$$")"
mkdir -p "$HOME_DIR"
export NDIR_HOME="$HOME_DIR"
PERF_DIR="$ROOT/target/perf"; mkdir -p "$PERF_DIR"
[ -n "$OUT" ] || OUT="$PERF_DIR/last.json"

# 벽시계(ms) — 파이썬 기동 없이. `date +%s%N`이 'N'을 그대로 내면(BSD date) perl로.
if [ "$(date +%N 2>/dev/null)" != "N" ] && [ -n "$(date +%N 2>/dev/null)" ]; then
    now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
else
    now_ms() { perl -MTime::HiRes=time -e 'printf("%d\n", time()*1000)'; }
fi
median() { python -c "import sys; v=sorted(int(x) for x in sys.argv[1:]); print(v[len(v)//2])" "$@"; }
HEAD_SHA="$(cd "$ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"

# ① 기동 — 예열 1회(파일 캐시 · 백신 검사)는 버린다.
"$EXE" --smoke >/dev/null 2>&1
times=()
for _ in $(seq 1 "$RUNS"); do
    t0=$(now_ms); "$EXE" --smoke >/dev/null 2>&1; t1=$(now_ms)
    times+=($((t1 - t0)))
done
SMOKE_MED=$(median "${times[@]}")

# ② fs 점검 벽시계
t0=$(now_ms); "$EXE" --selfcheck --ci --only fs --json >/dev/null 2>&1; t1=$(now_ms)
FS_MS=$((t1 - t0))

# ③ 자가 점검 그룹별 ms — JSON 파일
#    (pyenv-win 셔임은 여러 줄 `python -c '…'` 인자를 망가뜨린다 → 한 줄 -c 또는 `python - <<'EOF'`만 쓴다)
JSON="$HOME_DIR/selfcheck.json"
"$EXE" --selfcheck --ci --json >"$JSON" 2>/dev/null

# ④ 크기
SIZE=$(python -c "import os,sys; print(os.path.getsize(sys.argv[1]))" "$EXE")

# 집계 · 기준선 비교 · JSON 저장(한 번의 파이썬) — 표준 출력 = 마크다운 표 · 마지막 줄 = `REGRESSIONS=<n> FAILED=<n>`
RESULT=$(python - "$JSON" "$OUT" "$BASE" "$SAVE" "$PERF_DIR/baseline.json" "$HEAD_SHA" "$PROFILE" "$RUNS" "$SMOKE_MED" "$FS_MS" "$SIZE" "${times[*]}" <<'EOF'
import json, sys, time
from collections import defaultdict
(sc_json, out_path, base_path, save, base_default, head, profile, runs, smoke, fs_ms, size, raw) = sys.argv[1:13]
smoke, fs_ms, size, runs = int(smoke), int(fs_ms), int(size), int(runs)
groups = {}
try:
    with open(sc_json, encoding="utf-8") as f:
        r = json.load(f)
    g = defaultdict(int)
    for it in r.get("items", []):
        g[it["group"]] += int(it.get("ms", 0))
    groups = dict(sorted(g.items()))
except Exception:
    groups = {}
cur = {
    "at": time.strftime("%Y-%m-%dT%H:%M:%S"), "head": head, "profile": profile, "runs": runs,
    "smoke_med_ms": smoke, "smoke_runs_ms": [int(x) for x in raw.split()], "fs_ms": fs_ms,
    "exe_bytes": size, "groups_ms": groups,
}
for p in [out_path] + ([base_default] if save == "1" else []):
    with open(p, "w", encoding="utf-8") as f:
        json.dump(cur, f, ensure_ascii=False, indent=1)
base = None
if base_path:
    try:
        with open(base_path, encoding="utf-8") as f:
            base = json.load(f)
    except Exception:
        base = None
# 회귀선(docs/25 §3): 기동 +20 % · fs +20 % · 그룹 +20 %(기준 ≥ 200 ms만 — 작은 그룹은 잡음) · exe +5 %
LINES = {"smoke": 20.0, "fs": 20.0, "group": 20.0, "exe": 5.0}
regress, failed = 0, 0
def cmp(cur_v, base_v, pct, floor=0):
    """(기준선 글, Δ 글, 회귀 여부)."""
    global regress
    if base_v is None:
        return "", "", False
    if base_v <= 0 and cur_v <= 0:
        return "0", "±0", False
    if base_v <= 0 or cur_v <= 0:
        return str(base_v), "(비교 불가)", False
    d = (cur_v - base_v) * 100.0 / base_v
    bad = base_v >= floor and d > pct
    if bad:
        regress += 1
    return str(base_v), f"{d:+.1f} %" + (" ⚠" if bad else ""), bad
def fail_mark(v):
    global failed
    if v <= 0:
        failed += 1
        return " ⚠ 측정 실패(0 = 다시 잰다)"
    return ""
rows = []
b = base or {}
bs, ds, _ = cmp(smoke, b.get("smoke_med_ms"), LINES["smoke"])
rows.append((f"기동 `--smoke` 중앙값({runs}회)", f"{smoke} ms{fail_mark(smoke)}", bs, ds,
             f"창 없는 기동 경로(자가 점검 ci 부분집합 포함) · 예열 1회 제외 · 실측 {raw}"))
bs, ds, _ = cmp(fs_ms, b.get("fs_ms"), LINES["fs"])
rows.append(("`--selfcheck --ci --only fs`", f"{fs_ms} ms{fail_mark(fs_ms)}", bs, ds,
             "프로세스 벽시계(생성·복사·이름·삭제 + 감시 통지) · 대량 폴더 목록은 미측정"))
if not groups:
    rows.append(("selfcheck (json 없음)", "—", "", "", "그룹 합"))
for k, v in groups.items():
    bs, ds, _ = cmp(v, (b.get("groups_ms") or {}).get(k), LINES["group"], floor=200)
    rows.append((f"selfcheck `{k}`", f"{v} ms", bs, ds, "그룹 합"))
bs, ds, _ = cmp(size, b.get("exe_bytes"), LINES["exe"])
rows.append((f"exe 크기({profile})", f"{size} B", bs, ds, "예산 ≤ 10 MB(CI budget)"))
if base:
    print("| 항목 | 값 | 기준선 | Δ | 비고 |")
    print("| --- | --- | --- | --- | --- |")
    for n, v, bs, ds, note in rows:
        print(f"| {n} | {v} | {bs} | {ds} | {note} |")
    print(f"\n기준선 = {base_path}({base.get('at', '?')} · {base.get('head', '?')}) · 이번 = {head} · "
          f"회귀선 기동/fs/그룹 +20 % · exe +5 % → **회귀 {regress}건**" + (f" · 측정 실패 {failed}건" if failed else ""))
else:
    print("| 항목 | 값 | 비고 |")
    print("| --- | --- | --- |")
    for n, v, _, _, note in rows:
        print(f"| {n} | {v} | {note} |")
    print(f"\n이번 = {head} · 저장 = {out_path}" + (f" · 기준선 저장 = {base_default}" if save == "1" else "")
          + (f" · 측정 실패 {failed}건" if failed else ""))
print(f"REGRESSIONS={regress} FAILED={failed}")
EOF
)
rm -rf "$HOME_DIR"
echo "$RESULT" | sed '$d'
TAIL="$(echo "$RESULT" | tail -n 1)"
REG="${TAIL#REGRESSIONS=}"; REG="${REG%% *}"
FAILN="${TAIL##*FAILED=}"
if [ "$STRICT" = 1 ] && { [ "$REG" != 0 ] || [ "$FAILN" != 0 ]; }; then
    echo "✗ perf-baseline: 회귀 $REG건 · 측정 실패 $FAILN건(--strict)" >&2
    exit 1
fi
exit 0
