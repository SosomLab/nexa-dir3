#!/usr/bin/env bash
# check-all.sh — push 전 **전체 게이트**(T-05 · docs/port/44 CI-115 · nexa-sql `linux-all-tests.sh`의 3-OS 일반화).
#   형제 저장소 → dir3 순서로: nexa-ui(fmt · clippy · test) → nexa-license(같음) → nexa-dir3(fmt · clippy · test → check-3os →
#   --smoke → --selfcheck --ci → ndir-check --ci) → `summary.txt`(단계별 rc · 초 · 집계). 한 단계라도 실패하면 종료 코드 ≠ 0 = push 금지.
# 사용:  scripts/check-all.sh                 # 전부
#        scripts/check-all.sh --quick         # 형제 저장소 생략 · check-3os --quick · 시나리오 생략
#        scripts/check-all.sh --out <폴더>    # summary.txt 자리(기본 target/check-all)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QUICK=0
OUT="$ROOT/target/check-all"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick) QUICK=1 ;;
        --out) OUT="$2"; shift ;;
        *) echo "unknown arg: $1"; exit 2 ;;
    esac
    shift
done
mkdir -p "$OUT"
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
FAIL=0
PASS=0
START_ALL=$(date +%s)

# 단계 실행: 이름 · 폴더 · 명령… — rc·초를 summary에 적고 실패를 센다.
step() {
    local name="$1" dir="$2"; shift 2
    local t0 t1 rc
    printf '\n── %s ──\n' "$name"
    t0=$(date +%s)
    ( cd "$dir" && "$@" ) > "$OUT/$(echo "$name" | tr ' /:' '___').log" 2>&1
    rc=$?
    t1=$(date +%s)
    if [[ $rc -eq 0 ]]; then
        PASS=$((PASS+1)); echo "  ✓ ($((t1-t0))s)"
    else
        FAIL=$((FAIL+1)); echo "  ✗ rc=$rc ($((t1-t0))s) — $OUT/$(echo "$name" | tr ' /:' '___').log"
        tail -n 25 "$OUT/$(echo "$name" | tr ' /:' '___').log" | sed 's/^/    /'
    fi
    printf '%-40s rc=%-3s %4ss\n' "$name" "$rc" "$((t1-t0))" >> "$SUMMARY"
}

# rustup 툴체인 우선(check-3os.sh와 같은 이유 — Homebrew cargo가 앞서면 크로스 타깃 std를 못 찾는다).
if command -v rustup >/dev/null 2>&1; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

if [[ $QUICK -eq 0 ]]; then
    for SIB in nexa-ui nexa-license; do
        D="$ROOT/../$SIB"
        if [[ ! -d "$D" ]]; then
            echo "⚠ 형제 저장소 없음: $D (건너뜀)"; continue
        fi
        step "$SIB fmt" "$D" cargo fmt --all -- --check
        step "$SIB clippy" "$D" cargo clippy --workspace --all-targets -- -D warnings
        step "$SIB test" "$D" cargo test --workspace
    done
fi

step "nexa-dir3 fmt" "$ROOT" cargo fmt --all -- --check
step "nexa-dir3 clippy" "$ROOT" cargo clippy --workspace --all-targets -- -D warnings
step "nexa-dir3 test" "$ROOT" cargo test --workspace
if [[ $QUICK -eq 1 ]]; then
    step "nexa-dir3 check-3os --quick" "$ROOT" bash scripts/check-3os.sh --quick
else
    step "nexa-dir3 check-3os" "$ROOT" bash scripts/check-3os.sh
fi
step "nexa-dir3 build" "$ROOT" cargo build -p nexa-dir
if cargo run -q -p nexa-dir -- --help 2>/dev/null | grep -q -- '--smoke'; then
    step "nexa-dir --smoke" "$ROOT" cargo run -q -p nexa-dir -- --smoke
fi
step "nexa-dir --selfcheck --ci" "$ROOT" cargo run -q -p nexa-dir -- --selfcheck --ci
if [[ $QUICK -eq 0 ]]; then
    step "ndir-check --ci" "$ROOT" cargo run -q -p ndir-check -- --ci
fi

END_ALL=$(date +%s)
{
    echo "----"
    echo "passed=$PASS failed=$FAIL total_s=$((END_ALL-START_ALL))"
} >> "$SUMMARY"
echo
cat "$SUMMARY"
if [[ $FAIL -eq 0 ]]; then
    echo "★ check-all 통과 — push 가능 (push 뒤 gh run watch 로 CI도 확인)"
else
    echo "★ check-all 실패 ${FAIL}건 — push 금지"
fi
[[ $FAIL -eq 0 ]]
