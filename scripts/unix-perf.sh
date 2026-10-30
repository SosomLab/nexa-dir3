#!/usr/bin/env bash
# unix-perf.sh — **Linux · macOS 성능 프로브**(T-176 ⑤ · Windows `win-startup-probe.ps1` · `win-leak-cycle.ps1` · `win-mem-reclaim.ps1`의 Unix판 ·
#   docs/25 §5 · 입력 주입 없음 · 격리 NDIR_HOME · 자기 PID만 종료 — docs/18 §10). 외부 도구 0(bash · python3 · perl Time::HiRes · ps · lsof).
#   열 이름은 Windows판과 같다(window_ms · cpu_ms · priv_mb · ws_mb · fds · threads) — 표를 나란히 비교할 수 있게.
#
#   startup : 창 기동 비용 — ① 준비(`@ready` = 첫 프레임 + 초기 열거 뒤 · `layout.dump` 파일 mtime − 기동 시각) ② SETTLE초까지 CPU
#             ③ 그때의 Private(앱 `mem.dump` 첫 줄 footprint/private) · 워킹셋(ps rss) · fd 수 · 스레드 수를 RUNS번 재고 중앙값.
#   leak    : 같은 동작 묶음(CYCLE_CMDS · `;` 구분)을 CYCLES번 되풀이시키고 주기마다 `mem.dump`로 Private를 찍는다 → 뒤 절반 기울기(MB/주기).
#   reclaim : ① 기준 ② 올림(RAISE) ③ 놓음(RELEASE) ④ 유휴 트림 뒤 네 점의 `mem.dump` → ④ − ①이 +1 MB 또는 +5 %를 넘으면 WARN.
#
# 사용:
#   scripts/unix-perf.sh startup --home /tmp/ndir-home [--exe target/release/nexa-dir] [--runs 5] [--settle 3] [--cmd "nav:/usr/lib"] [--tag startup]
#   scripts/unix-perf.sh leak    --home /tmp/ndir-home --cycle-cmds "nav:/usr/lib;nav:/tmp" [--cycles 10] [--period-ms 4000] [--first "nav:/tmp"]
#   scripts/unix-perf.sh reclaim --home /tmp/ndir-home --raise "nav:/usr/lib" --release "nav:/tmp" [--settle-ms 3000] [--trim-sec 5]
#   공통: --exe 기본 = target/release/nexa-dir(release로 재는 것이 기준 · debug는 참고) · 결과 = 표준 출력(+ <home>/perf-*.txt 덤프).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-}"; shift || true
EXE="$ROOT/target/release/nexa-dir"; HOME_DIR=""; RUNS=5; SETTLE=3; CMD=""; TAG=""
CYCLES=10; PERIOD_MS=4000; CYCLE_CMDS=""; FIRST=""; RAISE=""; RELEASE=""; SETTLE_MS=3000; TRIM_SEC=5
while [[ $# -gt 0 ]]; do
    case "$1" in
        --exe) EXE="$2"; shift 2;; --home) HOME_DIR="$2"; shift 2;; --runs) RUNS="$2"; shift 2;; --settle) SETTLE="$2"; shift 2;;
        --cmd) CMD="$2"; shift 2;; --tag) TAG="$2"; shift 2;; --cycles) CYCLES="$2"; shift 2;; --period-ms) PERIOD_MS="$2"; shift 2;;
        --cycle-cmds) CYCLE_CMDS="$2"; shift 2;; --first) FIRST="$2"; shift 2;; --raise) RAISE="$2"; shift 2;; --release) RELEASE="$2"; shift 2;;
        --settle-ms) SETTLE_MS="$2"; shift 2;; --trim-sec) TRIM_SEC="$2"; shift 2;;
        *) echo "unknown arg: $1" >&2; exit 2;;
    esac
done
[[ "$MODE" == startup || "$MODE" == leak || "$MODE" == reclaim ]] || { sed -n 2,20p "$0"; exit 2; }
[[ -n "$HOME_DIR" && -d "$HOME_DIR" ]] || { echo "--home <격리 폴더>가 있어야 한다(먼저 만들 것): '$HOME_DIR'" >&2; exit 2; }
[[ -x "$EXE" ]] || { echo "exe 없음: $EXE (cargo build --release -p nexa-dir)" >&2; exit 2; }
PY="${PYTHON:-$(command -v python3 || command -v python)}"
OS="$(uname -s)"
TAG="${TAG:-$MODE}"
export NDIR_HOME="$HOME_DIR" NDIR_NO_ACTIVATE=1
unset NO_COLOR

now_ms() { perl -MTime::HiRes=time -e 'printf "%d\n", time()*1000'; }
mtime_ms() { perl -MTime::HiRes=stat -e 'my @s=stat($ARGV[0]); printf "%d\n", ($s[9]//0)*1000' "$1"; }
# 누적 CPU ms — macOS `ps -o time=`은 분:초.센티초 · Linux는 /proc/PID/stat utime+stime(틱).
cpu_ms() {
    if [[ "$OS" == Darwin ]]; then
        ps -o time= -p "$1" 2>/dev/null | awk -F'[:.]' '{ if (NF==3) printf "%d\n", ($1*60+$2)*1000+$3*10; else if (NF==4) printf "%d\n", (($1*60+$2)*60+$3)*1000+$4*10; else print 0 }'
    else
        local hz; hz=$(getconf CLK_TCK 2>/dev/null || echo 100)
        awk -v hz="$hz" '{ printf "%d\n", ($14+$15)*1000/hz }' "/proc/$1/stat" 2>/dev/null || echo 0
    fi
}
ws_mb() { ps -o rss= -p "$1" 2>/dev/null | awk '{ printf "%.1f\n", $1/1024 }'; }
threads_of() { if [[ "$OS" == Darwin ]]; then ps -M -p "$1" 2>/dev/null | tail -n +2 | wc -l | tr -d ' '; else ps -o nlwp= -p "$1" 2>/dev/null | tr -d ' '; fi; }
fds_of() { if [[ "$OS" == Darwin ]]; then lsof -p "$1" 2>/dev/null | tail -n +2 | wc -l | tr -d ' '; else ls "/proc/$1/fd" 2>/dev/null | wc -l | tr -d ' '; fi; }
# `mem.dump` 첫 줄에서 Private 에 해당하는 값(macOS footprint · Linux private · 없으면 resident) → MB.
priv_mb_of_dump() { "$PY" - "$1" <<'PYEOF'
import re, sys
try: line = open(sys.argv[1], encoding="utf-8", errors="replace").readline()
except OSError: print("0"); sys.exit()
m = re.search(r"\b(footprint|private)\s+(\d+)", line) or re.search(r"\bresident\s+(\d+)", line)
print("%.2f" % (int(m.group(m.lastindex)) / 1048576) if m else "0")
PYEOF
}
median() { "$PY" -c 'import sys; xs=sorted(float(x) for x in sys.argv[1:] if x); print(xs[(len(xs)-1)//2] if xs else 0)' "$@"; }
# 앱 기동(백그라운드) → PID. 끝나지 않으면 자기 PID만 kill.
launch() { NDIR_STARTUP_CMD="$1" "$EXE" >/dev/null 2>&1 & echo $!; }
wait_or_kill() { local pid="$1" limit_s="$2" t=0; while kill -0 "$pid" 2>/dev/null && (( t < limit_s*10 )); do sleep 0.1; t=$((t+1)); done; if kill -0 "$pid" 2>/dev/null; then kill "$pid" 2>/dev/null || true; echo "  ⚠ pid $pid did not exit — killed (0 = measurement failure)"; fi; }

case "$MODE" in
startup)
    rows_w=(); rows_c=(); rows_p=(); rows_ws=(); rows_f=(); rows_t=()
    for r in $(seq 1 "$RUNS"); do
        ready="$HOME_DIR/perf-ready-$r.txt"; memf="$HOME_DIR/perf-mem-$r.txt"; rm -f "$ready" "$memf"
        quit_at=$(( SETTLE*1000 + 500 )); mem_at=$(( SETTLE*1000 ))
        cmds="@ready:layout.dump:$ready"; [[ -n "$CMD" ]] && cmds="$cmds,${CMD//;/,}"
        cmds="$cmds,@after:${mem_at}:mem.dump:$memf,@after:${quit_at}:quit"
        t0=$(now_ms); pid=$(launch "$cmds")
        # 안정 시각(SETTLE) 직전 표본 — 프로세스가 살아 있을 때 ps로 CPU · rss · fd · 스레드.
        sleep "$(awk -v s="$SETTLE" 'BEGIN{ printf "%.1f", s-0.3 }')"
        if ! kill -0 "$pid" 2>/dev/null; then echo "$TAG	 run$r: process exited early - 0 = measurement failure, rerun"; wait "$pid" 2>/dev/null || true; continue; fi
        cpu=$(cpu_ms "$pid"); ws=$(ws_mb "$pid"); fds=$(fds_of "$pid"); th=$(threads_of "$pid")
        wait_or_kill "$pid" $(( SETTLE + 5 ))
        win=$(( $( [[ -f "$ready" ]] && mtime_ms "$ready" || echo "$t0" ) - t0 )); [[ -f "$ready" ]] || win=0
        priv=$(priv_mb_of_dump "$memf")
        printf '%s\t run%d: window %5d ms | cpu(%ds) %5d ms | priv %6.2f MB | ws %5.1f MB | fds %s | threads %s\n' "$TAG" "$r" "$win" "$SETTLE" "$cpu" "$priv" "$ws" "$fds" "$th"
        rows_w+=("$win"); rows_c+=("$cpu"); rows_p+=("$priv"); rows_ws+=("$ws"); rows_f+=("$fds"); rows_t+=("$th")
        sleep 0.6
    done
    (( ${#rows_w[@]} > 0 )) || { echo "$TAG	 no measurement (all runs failed)"; exit 1; }
    printf '%s\t MEDIAN(%d): window %s ms | cpu %s ms | priv %s MB | ws %s MB | fds %s | threads %s\n' "$TAG" "${#rows_w[@]}" \
        "$(median "${rows_w[@]}")" "$(median "${rows_c[@]}")" "$(median "${rows_p[@]}")" "$(median "${rows_ws[@]}")" "$(median "${rows_f[@]}")" "$(median "${rows_t[@]}")"
    ;;
leak)
    [[ -n "$CYCLE_CMDS" ]] || { echo "--cycle-cmds 필요" >&2; exit 2; }
    cmds=""; [[ -n "$FIRST" ]] && cmds="@ready:${FIRST//;/,@ready:}"
    at=$PERIOD_MS
    for c in $(seq 1 "$CYCLES"); do
        IFS=';' read -r -a parts <<<"$CYCLE_CMDS"
        for p in "${parts[@]}"; do cmds="${cmds:+$cmds,}@after:${at}:${p}"; at=$(( at + PERIOD_MS / (${#parts[@]} + 1) )); done
        cmds="$cmds,@after:${at}:mem.dump:$HOME_DIR/perf-leak-$c.txt"; at=$(( at + PERIOD_MS / (${#parts[@]} + 1) ))
    done
    cmds="$cmds,@after:$(( at + 500 )):quit"
    rm -f "$HOME_DIR"/perf-leak-*.txt
    pid=$(launch "$cmds"); wait_or_kill "$pid" $(( at/1000 + 10 ))
    vals=(); for c in $(seq 1 "$CYCLES"); do v=$(priv_mb_of_dump "$HOME_DIR/perf-leak-$c.txt"); vals+=("$v"); printf '%s\tcycle%-3d priv=%7.2fMB\n' "$TAG" "$c" "$v"; done
    "$PY" - "$TAG" "${vals[@]}" <<'PYEOF'
import sys
tag, xs = sys.argv[1], [float(x) for x in sys.argv[2:]]
half = xs[len(xs)//2:]
slope = (half[-1]-half[0])/max(1, len(half)-1) if len(half) > 1 else 0.0
verdict = "WARN(step-like growth)" if slope > 0.5 else "ok"
print(f"{tag}\tslope(last half) = {slope:.3f} MB/cycle | first {xs[0]:.2f} -> last {xs[-1]:.2f} MB | {verdict}")
PYEOF
    ;;
reclaim)
    [[ -n "$RAISE" && -n "$RELEASE" ]] || { echo "--raise · --release 필요" >&2; exit 2; }
    # 격리 설정에 유휴 트림을 짧게(mem.idle_trim_s) — 있으면 덧붙이고 없으면 추가.
    conf="$HOME_DIR/settings.conf"; touch "$conf"; grep -q '^mem.idle_trim_s=' "$conf" && sed -i.bak "s/^mem.idle_trim_s=.*/mem.idle_trim_s=$TRIM_SEC/" "$conf" || echo "mem.idle_trim_s=$TRIM_SEC" >> "$conf"
    t1=$SETTLE_MS; t2=$(( t1 + SETTLE_MS )); t3=$(( t2 + SETTLE_MS )); t4=$(( t3 + TRIM_SEC*1000 + SETTLE_MS ))
    cmds="@after:${t1}:mem.dump:$HOME_DIR/perf-reclaim-1.txt"
    IFS=';' read -r -a rs <<<"$RAISE"; for p in "${rs[@]}"; do cmds="$cmds,@after:$(( t1 + 100 )):$p"; done
    cmds="$cmds,@after:${t2}:mem.dump:$HOME_DIR/perf-reclaim-2.txt"
    IFS=';' read -r -a rl <<<"$RELEASE"; for p in "${rl[@]}"; do cmds="$cmds,@after:$(( t2 + 100 )):$p"; done
    cmds="$cmds,@after:${t3}:mem.dump:$HOME_DIR/perf-reclaim-3.txt,@after:${t4}:mem.dump:$HOME_DIR/perf-reclaim-4.txt,@after:$(( t4 + 500 )):quit"
    rm -f "$HOME_DIR"/perf-reclaim-*.txt
    pid=$(launch "$cmds"); wait_or_kill "$pid" $(( t4/1000 + 10 ))
    p1=$(priv_mb_of_dump "$HOME_DIR/perf-reclaim-1.txt"); p2=$(priv_mb_of_dump "$HOME_DIR/perf-reclaim-2.txt"); p3=$(priv_mb_of_dump "$HOME_DIR/perf-reclaim-3.txt"); p4=$(priv_mb_of_dump "$HOME_DIR/perf-reclaim-4.txt")
    "$PY" - "$TAG" "$p1" "$p2" "$p3" "$p4" <<'PYEOF'
import sys
tag, b, r, l, t = sys.argv[1], *[float(x) for x in sys.argv[2:6]]
d = t - b
warn = d > 1.0 or (b > 0 and d / b > 0.05)
print(f"{tag}\tbase {b:.2f} | raise {r:.2f} | release {l:.2f} | after-trim {t:.2f} MB | delta {d:+.2f} MB ({(d/b*100 if b else 0):+.1f} %) | {'WARN(not reclaimed)' if warn else 'ok'}")
PYEOF
    ;;
esac
