#!/usr/bin/env bash
# perf-baseline.sh — 성능 기준선(T6 · T-91 · CI-118 간소 절차 ①②④) · 외부 도구 0(bash + cargo + python).
#   ① 기동: `nexa-dir --smoke`(창 없는 기동 경로 = 자가 점검 ci 부분집합) 벽시계 N회 → 중앙값 ms
#   ② fs 점검: `--selfcheck --ci --only fs` 프로세스 벽시계(생성·복사·이름 바꾸기·삭제 + 폴더 감시 통지)
#      ※ 대량 폴더(1만/10만 항목) 목록·정렬은 헤드리스 경로가 없어 미측정 — fs 점검은 자기 임시 폴더만 쓴다(T-08 잔여)
#   ③ 자가 점검 그룹별 ms(`--selfcheck --ci --json`)
#   ④ 산출물 크기: release exe 바이트
# 사용: scripts/perf-baseline.sh [--runs N] [--debug]   (기본 runs 5 · release 빌드 사용)
# 출력: 마크다운 표(표준 출력) — journal에 그대로 붙인다. 숫자는 이 PC 기준 참고치(CI 비교는 상대값으로).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNS=5; PROFILE=release
while [ $# -gt 0 ]; do
    case "$1" in
        --runs) RUNS="$2"; shift ;;
        --debug) PROFILE=debug ;;
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
now_ms() { python -c "import time; print(int(time.time()*1000))"; }
median() { python -c "import sys; v=sorted(int(x) for x in sys.argv[1:]); print(v[len(v)//2])" "$@"; }

# ① 기동
times=()
for _ in $(seq 1 "$RUNS"); do
    t0=$(now_ms); "$EXE" --smoke >/dev/null 2>&1; t1=$(now_ms)
    times+=($((t1 - t0)))
done
SMOKE_MED=$(median "${times[@]}")

# ② fs 점검 벽시계
t0=$(now_ms); "$EXE" --selfcheck --ci --only fs --json >/dev/null 2>&1; t1=$(now_ms)
FS_MS=$((t1 - t0))

# ③ 자가 점검 그룹별 ms — JSON을 파일로 받은 뒤 heredoc 스크립트로 집계
#    (pyenv-win 셔임은 여러 줄 `python -c '…'` 인자를 망가뜨린다 → 한 줄 -c 또는 `python - <<'EOF'`만 쓴다)
JSON="$HOME_DIR/selfcheck.json"
"$EXE" --selfcheck --ci --json >"$JSON" 2>/dev/null
GROUPS_MD=$(python - "$JSON" <<'EOF'
import json, sys
from collections import defaultdict
try:
    with open(sys.argv[1], encoding="utf-8") as f:
        r = json.load(f)
except Exception:
    print("| (json 없음) | — |"); sys.exit()
g = defaultdict(int)
for it in r.get("items", []):
    g[it["group"]] += int(it.get("ms", 0))
for k in sorted(g):
    print(f"| selfcheck `{k}` | {g[k]} ms |")
EOF
)

# ④ 크기
SIZE=$(python -c "import os,sys; print(os.path.getsize(sys.argv[1]))" "$EXE")

echo "| 항목 | 값 | 비고 |"
echo "| --- | --- | --- |"
echo "| 기동 \`--smoke\` 중앙값(${RUNS}회) | ${SMOKE_MED} ms | 창 없는 기동 경로(자가 점검 ci 부분집합 포함) · 실측 ${times[*]} |"
echo "| \`--selfcheck --ci --only fs\` | ${FS_MS} ms | 프로세스 벽시계(생성·복사·이름·삭제 + 감시 통지) · 대량 폴더 목록은 미측정 |"
echo "$GROUPS_MD" | sed 's/| \([^|]*\) | \([^|]*\) |/| \1 | \2 | 그룹 합 |/'
echo "| exe 크기(${PROFILE}) | ${SIZE} B | 예산 ≤ 10 MB(CI budget) |"
rm -rf "$HOME_DIR"
