#!/usr/bin/env bash
# gate.sh — 커밋/push 전 게이트(단계형 · 사용자 10-04 "모든 수정에 전수 테스트를 할 필요는 없다").
#
#   scripts/gate.sh            # auto — 아래 규칙으로 quick/full을 고른다(이유를 한 줄로 알린다)
#   scripts/gate.sh quick      # 빠른 게이트만
#   scripts/gate.sh full       # 전수(배포 전 · 태그 전 · 마일스톤 마감 같은 중요 시점에는 반드시)
#
# quick = fmt + 호스트 clippy(-D warnings) + **바뀐 크레이트의 시험** + --smoke
# full  = fmt + 3-OS clippy + 워크스페이스 전체 시험 + --smoke + --selfcheck --ci + T4 시나리오(ndir-check)
#
# auto가 full을 고르는 때(하나라도):
#   ① 전수 기록이 없다(target/gate/last-full — PC마다 따로)
#   ② 마지막 전수가 NDIR_GATE_FULL_HOURS(기본 24)시간보다 오래됐다
#   ③ 마지막 전수 뒤로 **핵심 경로**가 바뀌었다(영향도 높음 — 아래 CORE) · 형제 저장소 nexa-ui/nexa-license가 바뀌었다
# 그 밖 = quick. push 뒤 CI(3-OS 전수)는 그대로 돈다 — quick으로 push했으면 CI 결과 확인이 전수 역할을 한다.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
MODE="${1:-auto}"
REC="target/gate/last-full"
HOURS="${NDIR_GATE_FULL_HOURS:-24}"
# 핵심 경로: 코어 rlib(코어 · VFS · 트리 · 파일 작업 · 터미널) · OS 분기 · 설정 엔진 · 라이선스 · 빌드/CI/게이트 · 시나리오.
CORE='^(crates/(ndir-core|ndir-vfs|ndir-tree|ndir-ops|ndir-term|ndir-license)/|crates/ndir-settings/src/lib\.rs|crates/nexa-dir/src/platform/|crates/nexa-dir/Cargo\.toml|crates/ndir-check/|Cargo\.(toml|lock)|\.github/|scripts/|tests/scenarios/)'

sib() { git -C "../$1" rev-parse HEAD 2>/dev/null || echo none; }
changed_since() { { git diff --name-only "$1" HEAD 2>/dev/null; git diff --name-only HEAD; git ls-files --others --exclude-standard; } | sort -u; }

WHY=""
if [[ "$MODE" == auto ]]; then
    if [[ ! -f "$REC" ]]; then
        MODE=full; WHY="전수 기록 없음"
    else
        read -r AT HASH UI LIC < "$REC"
        AGE=$(( ($(date +%s) - AT) / 3600 ))
        if (( AGE >= HOURS )); then
            MODE=full; WHY="마지막 전수가 ${AGE}시간 전(기준 ${HOURS}시간)"
        elif [[ "$UI" != "$(sib nexa-ui)" || "$LIC" != "$(sib nexa-license)" ]]; then
            MODE=full; WHY="형제 저장소(nexa-ui/nexa-license)가 바뀜"
        else
            HIT="$(changed_since "$HASH" | grep -E "$CORE" | head -3 | tr '\n' ' ')"
            if [[ -n "$HIT" ]]; then MODE=full; WHY="핵심 경로 변경: $HIT"; else MODE=quick; WHY="마지막 전수 ${AGE}시간 전 · 핵심 경로 변경 없음"; fi
        fi
    fi
fi
echo "▶ gate: $MODE${WHY:+ — $WHY}"

fail() { echo "★ GATE 실패($MODE): $1"; exit 1; }

if [[ "$MODE" == quick ]]; then
    bash scripts/check-3os.sh --quick >/tmp/ndir-gate-check.$$ 2>&1 || { grep -E "^(error|warning)" -A12 /tmp/ndir-gate-check.$$ | head -60; rm -f /tmp/ndir-gate-check.$$; fail "fmt/clippy(호스트)"; }
    rm -f /tmp/ndir-gate-check.$$; echo "  ✓ fmt + clippy(호스트)"
    BASE="$( [[ -f "$REC" ]] && cut -d' ' -f2 "$REC" || echo HEAD )"
    PKGS="$(changed_since "$BASE" | sed -n 's|^crates/\([^/]*\)/.*|\1|p' | sort -u | tr '\n' ' ')"
    # 언어 파일 · 설정 표는 앱 시험이 함께 본다.
    case " $PKGS " in *" ndir-i18n "*|*" ndir-settings "*) PKGS="$PKGS nexa-dir";; esac
    PKGS="$(echo "$PKGS" | tr ' ' '\n' | sort -u | tr '\n' ' ')"
    if [[ -z "${PKGS// }" ]]; then
        echo "  · 바뀐 크레이트 없음 — 시험 생략"
    else
        ARGS=(); for p in $PKGS; do ARGS+=(-p "$p"); done
        # 판정 = cargo 종료 코드 + 실패 수. 출력의 `^error` 줄로 판정하지 않는다 — 런처 시험이 띄운 자식 프로세스(시험 바이너리 --version)의
        # stderr "error: Unrecognized option"이 줄 맨 앞에 떨어지면 통과한 시험을 실패로 읽었다(10-05 적발 · 흔들림).
        OUT="$(cargo test -q "${ARGS[@]}" 2>&1)"; RC=$?
        T="$(echo "$OUT" | grep -E '^test result' | awk '{p+=$4;f+=$6} END{print p+0" "f+0}')"
        [[ $RC -eq 0 && "$T" == *" 0" ]] || { echo "$OUT" | grep -E "FAILED|panicked|^error" -A10 | head -50; fail "시험($PKGS)"; }
        echo "  ✓ 시험($PKGS) — 통과/실패 $T"
    fi
    SM="$(cargo run -q -p nexa-dir -- --smoke 2>&1 | tail -1)"; [[ "$SM" == "smoke ok"* ]] || fail "smoke: $SM"; echo "  ✓ $SM"
    echo "★ GATE OK(quick) — push 뒤 CI(3-OS 전수) 결과를 확인한다"
    exit 0
fi

# ── full
OUT="$(bash scripts/check-3os.sh 2>&1)"
[[ "$OUT" == *"검사 통과"* ]] || { echo "$OUT" | grep -E "^(error|warning)" -A12 | head -60; fail "fmt/clippy(3-OS)"; }
echo "  ✓ fmt + clippy(3-OS)"
OUT="$(cargo test --workspace -q 2>&1)"; RC=$?
T="$(echo "$OUT" | grep -E '^test result' | awk '{p+=$4;f+=$6} END{print p+0" "f+0}')"
[[ $RC -eq 0 && "$T" == *" 0" ]] || { echo "$OUT" | grep -E "FAILED|panicked|^error" -A10 | head -50; fail "시험(전체)"; }
echo "  ✓ 시험(전체) — 통과/실패 $T"
SM="$(cargo run -q -p nexa-dir -- --smoke 2>&1 | tail -1)"; [[ "$SM" == "smoke ok"* ]] || fail "smoke: $SM"; echo "  ✓ $SM"
SC="$(cargo run -q -p nexa-dir -- --selfcheck --ci 2>&1 | tail -1)"; [[ "$SC" == *"fail 0"* ]] || fail "selfcheck: $SC"; echo "  ✓ selfcheck $SC"
T4="$(cargo run -q -p ndir-check -- --ci 2>&1 | grep -E '^FAIL|scenario')"
# Windows 전제 시나리오 3개는 다른 OS에서 실패가 기준선이다(T-117).
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) KNOWN='^$';; *) KNOWN='^FAIL (ctx-menu|launcher|selfcheck-win) ';; esac
BAD="$(echo "$T4" | grep '^FAIL' | grep -vE "$KNOWN")"
[[ -z "$BAD" ]] || { echo "$T4"; fail "T4 시나리오"; }
echo "  ✓ T4 — $(echo "$T4" | tail -1)"
mkdir -p "$(dirname "$REC")"
echo "$(date +%s) $(git rev-parse HEAD) $(sib nexa-ui) $(sib nexa-license)" > "$REC"
echo "★ GATE OK(full) — 전수 기록 갱신($REC)"
