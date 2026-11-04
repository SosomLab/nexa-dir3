#!/usr/bin/env bash
# kill-resume-check.sh — T-202 ④ 잔여 ①(11-03 자동화 · 종전 11-01 §16 수동 절차): **바이트 전송 중 강제 종료 → 재기동 자동 복원 → 무결성**.
#
#   ① 격리 설정 폴더(NDIR_HOME) + `transfer.native=off · threads=1 · queue_restore=auto · close_ms=0`
#   ② src/big(N × MB MiB · 기본 8 × 300) 폴더를 dst로 붙여넣기 → KILL_MS 뒤 `ops.dump`가 "transfer running"이면 **강제 종료**
#      (Windows `taskkill /F` · unix `kill -9`) → `queue/`에 `<id>.job.json` + `<id>.journal`이 남아야 한다
#   ③ 같은 NDIR_HOME으로 재기동(`@ready:ops.wait` ×3 = 상한 60 s) → `ops.dump` = "transfer idle · queue 0" → `quit`
#   ④ 판정 = dst/big 파일 N개가 원본과 바이트 동일 · `.nexa-tmp*`/`.nexa-old*` 잔여 0 · `queue/` 비움(정상 종료가 지움)
#
# 사용: scripts/kill-resume-check.sh [--exe <nexa-dir>] [--files N] [--mb M] [--kill-ms T] [--native] [--keep]
#   --native = 기본 복사 경로(`transfer.native=on` · 스레드 기본값 · macOS = clonefile · Linux = copy_file_range)를 시험한다. 복제는 파일 하나가
#     즉시 끝나 큰 파일로는 끊을 틈이 없으므로 기본값을 작은 파일 다수로 바꾼다(30000 × 1 MiB 희소 · kill 2500 ms = 복제 진행 중 · --files/--mb/--kill-ms가 우선).
#   환경: NDIR_KR_FILES · NDIR_KR_MB · NDIR_KR_KILL_MS · NDIR_KR_KEEP=1(샌드박스 보존) · NDIR_EXE
#   종료 코드: 0 = ★ OK · 1 = 판정 실패 · 2 = 전송이 KILL_MS 안에 끝나 강제 종료를 못 함(파일 수·크기를 키운다) · 3 = 환경 오류
#   임시 폴더 = OS Temp `ndir-killres-<pid>`(disk-check `--trim` ⑦ 접두사 · 끝나면 스스로 지운다 · --keep = 보존).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
N="${NDIR_KR_FILES:-}"; MB="${NDIR_KR_MB:-}"; KILL_MS="${NDIR_KR_KILL_MS:-}"; KEEP="${NDIR_KR_KEEP:-0}"; NATIVE=0
EXE="${NDIR_EXE:-}"
while [ $# -gt 0 ]; do
    case "$1" in
        --exe) EXE="$2"; shift ;;
        --files) N="$2"; shift ;;
        --mb) MB="$2"; shift ;;
        --kill-ms) KILL_MS="$2"; shift ;;
        --native) NATIVE=1 ;;
        --keep) KEEP=1 ;;
        *) echo "unknown arg: $1" >&2; exit 3 ;;
    esac
    shift
done
if [ "$NATIVE" = 1 ]; then
    N="${N:-30000}"; MB="${MB:-1}"; KILL_MS="${KILL_MS:-2500}"
else
    N="${N:-8}"; MB="${MB:-300}"; KILL_MS="${KILL_MS:-1500}"
fi
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) WIN=1 ;; *) WIN=0 ;; esac
if [ -z "$EXE" ]; then
    EXE="$ROOT/target/debug/nexa-dir"; [ "$WIN" = 1 ] && EXE="$EXE.exe"
fi
[ -x "$EXE" ] || { echo "✗ 실행 파일 없음: $EXE (cargo build -p nexa-dir)" >&2; exit 3; }

TMP="${TEMP:-${TMPDIR:-/tmp}}/ndir-killres-$$"
HOME_D="$TMP/home"; SRC="$TMP/root/src"; DST="$TMP/root/dst"; OUT="$TMP/out"
mkdir -p "$HOME_D" "$SRC/big" "$DST" "$OUT"
cleanup() { [ "$KEEP" = 1 ] && { echo "  (보존) $TMP"; return; }; rm -rf "$TMP"; }
trap cleanup EXIT
# 네이티브 경로(기동 명령 · Windows = 역슬래시).
np() { if [ "$WIN" = 1 ]; then cygpath -w "$1"; else printf '%s' "$1"; fi; }

if [ "$NATIVE" = 1 ]; then ENGINE='transfer.native=on\n'; else ENGINE='transfer.native=off\ntransfer.threads=1\n'; fi
printf "_schema=1\nui.lang=en\n${ENGINE}transfer.queue_restore=auto\ntransfer.close_ms=0\n" > "$HOME_D/settings.conf"
echo "▶ kill-resume-check — $N × ${MB} MiB · native $NATIVE · kill ${KILL_MS} ms · $TMP"
if [ "$WIN" = 1 ]; then
    for i in $(seq 1 "$N"); do
        fsutil.exe file createnew "$(np "$SRC/big/f$i.bin")" $((MB * 1024 * 1024)) >/dev/null || { echo "✗ fsutil" >&2; exit 3; }
    done
else
    # 파일 수가 많아도(--native) 빠르게: 프로세스 하나에 모아 만든다.
    (cd "$SRC/big" && seq 1 "$N" | sed 's/.*/f&.bin/' | xargs truncate -s "${MB}M") || { echo "✗ truncate" >&2; exit 3; }
fi

run_app() { # <기동 명령> → 백그라운드 PID
    NDIR_HOME="$(np "$HOME_D")" NDIR_NO_ACTIVATE=1 NDIR_FAKE_CLIPBOARD=1 NDIR_STARTUP_CMD="$1" "$EXE" >"$OUT/stderr.$2.txt" 2>&1 &
    echo $!
}
# 강제 종료 — Git Bash의 `$!`는 MSYS pid라 `taskkill`에는 Windows pid(`ps` WINPID 열)를 넘겨야 한다(11-03 1차 실행: MSYS pid로는 안 죽어
# 1번 인스턴스가 살아 남고 2번이 owner.lock을 보고 복원을 건너뛰었다 — 설계대로).
hard_kill() {
    if [ "$WIN" = 1 ]; then
        local wp; wp=$(ps -p "$1" 2>/dev/null | awk 'NR==2 {print $4}')
        [ -n "$wp" ] && taskkill.exe //F //PID "$wp" >/dev/null 2>&1
    else
        kill -9 "$1" 2>/dev/null
    fi
    for _ in $(seq 1 50); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.1; done
    echo "✗ 강제 종료 실패(pid $1)"; return 1
}

# ② 전송 시작 → KILL_MS 뒤 덤프 → 진행 중이면 강제 종료.
RUN1="$OUT/run1.txt"
PID=$(run_app "@ready:nav:$(np "$SRC"),@ready:list.select:0,@ready:edit.copy,@ready:nav:$(np "$DST"),@ready:edit.paste,@after:${KILL_MS}:ops.dump:$(np "$RUN1")" 1)
for _ in $(seq 1 100); do [ -s "$RUN1" ] && break; sleep 0.1; done
if [ ! -s "$RUN1" ]; then hard_kill "$PID"; echo "✗ 덤프 없음(기동 실패?) — $OUT/stderr.1.txt"; exit 3; fi
if ! grep -q '^transfer running' "$RUN1"; then
    hard_kill "$PID"; wait "$PID" 2>/dev/null
    echo "✗ ${KILL_MS} ms 시점에 전송 중이 아님: $(head -1 "$RUN1") — 파일 수·크기를 키운다(--files/--mb)"; exit 2
fi
echo "  전송 중 $(head -1 "$RUN1") → 강제 종료 pid $PID"
hard_kill "$PID"; wait "$PID" 2>/dev/null
JOBS=$(ls "$HOME_D/queue/"*.job.json 2>/dev/null | wc -l | tr -d ' '); JRN=$(ls "$HOME_D/queue/"*.journal 2>/dev/null | wc -l | tr -d ' ')
echo "  queue/: job $JOBS · journal $JRN · $(ls "$DST/big" 2>/dev/null | wc -l | tr -d ' ')개 부분 대상"
[ "$JOBS" -ge 1 ] || { echo "✗ 강제 종료 뒤 queue/<id>.job.json 없음"; exit 1; }

# ③ 재기동 = 자동 복원 → 끝날 때까지 → 덤프 → 종료.
RUN2="$OUT/run2.txt"
PID=$(run_app "@ready:ops.wait,@ready:ops.wait,@ready:ops.wait,@ready:ops.dump:$(np "$RUN2"),@ready:quit" 2)
for _ in $(seq 1 1200); do kill -0 "$PID" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$PID" 2>/dev/null; then hard_kill "$PID"; echo "✗ 재기동 120 s 안에 안 끝남"; exit 1; fi
echo "  재기동 덤프: $(head -2 "$RUN2" | tr '\n' ' ')"

# ④ 판정.
FAIL=0
grep -q '^transfer idle' "$RUN2" || { echo "✗ transfer idle 아님"; FAIL=1; }
grep -q '^queue 0' "$RUN2" || { echo "✗ queue 0 아님"; FAIL=1; }
diff -rq "$SRC/big" "$DST/big" >"$OUT/diff.txt" 2>&1 || { echo "✗ 원본과 다름 $(wc -l <"$OUT/diff.txt" | tr -d ' ')건: $(head -3 "$OUT/diff.txt" | tr '\n' ' ')"; FAIL=1; }
STRAY=$(find "$DST" -name '.nexa-*' 2>/dev/null | wc -l | tr -d ' ')
[ "$STRAY" = 0 ] || { echo "✗ 스테이징 잔여 $STRAY"; FAIL=1; }
LEFT=$(ls -A "$HOME_D/queue" 2>/dev/null | wc -l | tr -d ' ')
[ "$LEFT" = 0 ] || { echo "✗ queue/ 잔여 $LEFT: $(ls -A "$HOME_D/queue" | tr '\n' ' ')"; FAIL=1; }
if [ "$FAIL" = 0 ]; then echo "★ KILL-RESUME OK — ${N}개 바이트 동일 · 잔여 0 · queue/ 비움"; exit 0; fi
echo "✗ KILL-RESUME 실패 — $OUT (--keep로 보존)"; exit 1
