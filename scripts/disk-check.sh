#!/usr/bin/env bash
# disk-check.sh — 개발 디스크 사용 점검·정리(사용자 11-02 규칙 · CLAUDE.md §5 "디스크 예산" · docs/15): 저장소·워크트리 `target/` ·
#   벤치 자료 · 게이트 로그 · 편집기(Sublime) 색인이 수십 GB를 차지하지 않게 **회차 마감 때** 한 번 돌린다.
#
# 사용: scripts/disk-check.sh [--trim] [--budget-gb N] [--root <형제 저장소 상위>] [--repo <이 저장소>](둘은 시험용 · 기본 = 실제 자리)
#   보고(기본): 형제 저장소 target/ · 워크트리 target/ · 이 저장소 target/ 하위 · Sublime 캐시 · 세션 스크래치 · cargo home 크기(GB) ·
#              합계가 예산(기본 30 GB · `NDIR_DISK_BUDGET_GB`)을 넘으면 경고 + 종료 코드 1(게이트는 막지 않는다 — 사람이 본다).
#   --trim : **안전한 것만** 지운다(다시 만들 수 있는 캐시 · 끝난 실기 자료) — ① 워크트리(`_collab/*` · `*-wt-*`)의 target/ 전체
#            ② target/perf/bench-* · target/tmp ③ target/*.log(게이트 로그) ④ target/{debug,release}/incremental ⑤ 다른 OS 타깃 폴더
#            (`target/<triple>/` = check-3os clippy 메타데이터 · 다음 full 게이트가 다시 만든다) ⑥ 세션별 target(`target/collab` · `target/dev*` =
#            협업 격리 인스턴스 · 개발 세션의 `CARGO_TARGET_DIR` · 11-02 측정 collab 9.8 GB · dev 6.7 GB) — 단, nexa-dir 프로세스가 떠 있으면 ⑥은 건너뛴다
#            (실행 중인 인스턴스의 exe를 지우지 않게). 소스 · 설정 · 문서 · ~/.cargo는 건드리지 않는다.
#            release/debug 본체를 비우려면 사람이 `cargo clean --release`(다음 게이트 full 재빌드 10분+).
#   측정은 Windows에서 느리다(du · 수십만 파일 = 수 분) — 백그라운드로 돌리고 결과만 본다. 측정·기록 = 협업 세션 · 삭제 실행 = 개발 세션(사용자 확인).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PARENT="$(cd "$ROOT/.." && pwd)"
TRIM=0; BUDGET="${NDIR_DISK_BUDGET_GB:-30}"
while [ $# -gt 0 ]; do
    case "$1" in
        --trim) TRIM=1 ;;
        --budget-gb) BUDGET="$2"; shift ;;
        --root) PARENT="$2"; shift ;;
        --repo) ROOT="$2"; shift ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
    shift
done

# KiB → "x.y G" · 없으면 "-".
gb() { awk -v k="$1" 'BEGIN { printf "%.1f", k / 1024 / 1024 }'; }
size_k() { [ -e "$1" ] && du -sk "$1" 2>/dev/null | cut -f1 || echo 0; }

TOTAL_K=0
row() { # 라벨 경로
    local k; k=$(size_k "$2")
    TOTAL_K=$((TOTAL_K + k))
    [ "$k" -gt 0 ] && printf "  %7s G  %s\n" "$(gb "$k")" "$2"
}

echo "▶ disk-check — 예산 ${BUDGET} GB · 기준 폴더 $PARENT"
echo "[저장소 target/]"
for r in nexa-dir3 nexa-ui nexa-license nexa-sql nexa-dir2; do row "$r" "$PARENT/$r/target"; done
echo "[워크트리 target/ — 빌드 금지 자리(규칙 ②) · 있으면 전부 삭제 대상]"
WT_TARGETS=()
for d in "$PARENT"/_collab/*/target "$PARENT"/*-wt-*/target; do
    [ -d "$d" ] || continue
    WT_TARGETS+=("$d"); row "wt" "$d"
done
echo "[이 저장소 target/ 하위 — 큰 순]"
if [ -d "$ROOT/target" ]; then
    du -sk "$ROOT"/target/* 2>/dev/null | sort -rn | head -8 | while read -r k p; do printf "  %7s G  %s\n" "$(gb "$k")" "${p#"$ROOT"/}"; done
fi
echo "[편집기 · 세션 · cargo home — 참고(지우지 않는다)]"
for p in "${LOCALAPPDATA:-}/Sublime Text" "${LOCALAPPDATA:-}/Sublime Text 3" "${HOME}/.cache/sublime-text" "${HOME}/Library/Caches/Sublime Text"; do
    [ -d "$p" ] || continue
    for sub in Index Cache Backup; do [ -d "$p/$sub" ] && printf "  %7s G  %s\n" "$(gb "$(size_k "$p/$sub")")" "$p/$sub"; done
done
[ -d "${LOCALAPPDATA:-}/Temp/claude" ] && printf "  %7s G  %s\n" "$(gb "$(size_k "$LOCALAPPDATA/Temp/claude")")" "$LOCALAPPDATA/Temp/claude(세션 스크래치)"
[ -d "$HOME/.cargo/registry" ] && printf "  %7s G  %s\n" "$(gb "$(size_k "$HOME/.cargo/registry")")" "$HOME/.cargo/registry"

echo "[여유]"
df -h "$ROOT" | awk 'NR==2 {printf "  %s 여유 %s(사용 %s)\n", $1, $4, $5}'
TOTAL_G=$(gb "$TOTAL_K")
echo "합계(저장소 + 워크트리 target/) = ${TOTAL_G} G"

if [ "$TRIM" = 1 ]; then
    echo "▶ trim — 안전한 캐시·끝난 자료만"
    for d in "${WT_TARGETS[@]}"; do echo "  rm -rf $d"; rm -rf "$d"; done
    for d in "$ROOT"/target/perf/bench-* "$ROOT"/target/tmp "$ROOT"/target/debug/incremental "$ROOT"/target/release/incremental \
             "$ROOT"/target/*-apple-darwin "$ROOT"/target/*-pc-windows-msvc "$ROOT"/target/*-unknown-linux-gnu; do
        [ -e "$d" ] || continue
        echo "  rm -rf ${d#"$ROOT"/}"; rm -rf "$d"
    done
    for f in "$ROOT"/target/*.log; do [ -e "$f" ] && { echo "  rm ${f#"$ROOT"/}"; rm -f "$f"; }; done
    # ⑥ 세션별 target — **target/ 아래에서 실행 중인** nexa-dir 프로세스가 있으면 건너뛴다(설치본 · 다른 자리의 인스턴스는 무관 ·
    #   Windows = PowerShell Get-Process 경로 · unix = pgrep -a · `NDIR_DISK_IGNORE_RUNNING=1` = 가드 무시).
    RUNNING=""
    if [ "${NDIR_DISK_IGNORE_RUNNING:-0}" != 1 ]; then
        if command -v powershell.exe >/dev/null 2>&1; then
            RUNNING=$(powershell.exe -NoProfile -Command "Get-Process nexa-dir -ErrorAction SilentlyContinue | ForEach-Object { \$_.Path }" 2>/dev/null | tr -d '\r' | grep -i "target" || true)
        elif command -v pgrep >/dev/null 2>&1; then
            RUNNING=$(pgrep -ax nexa-dir 2>/dev/null | grep "/target/" || true)
        fi
    fi
    for d in "$ROOT"/target/collab "$ROOT"/target/dev "$ROOT"/target/dev[0-9]*; do
        [ -d "$d" ] || continue
        if [ -n "$RUNNING" ]; then echo "  (skip ${d#"$ROOT"/} — target/ 아래 nexa-dir 실행 중: $(echo "$RUNNING" | head -1))"; continue; fi
        echo "  rm -rf ${d#"$ROOT"/}"; rm -rf "$d"
    done
    echo "  (release/debug 본체는 그대로 — 비우려면 cargo clean --release)"
    df -h "$ROOT" | awk 'NR==2 {printf "  여유 %s(사용 %s)\n", $4, $5}'
fi

if awk -v t="$TOTAL_G" -v b="$BUDGET" 'BEGIN { exit !(t > b) }'; then
    echo "✗ 예산 초과: ${TOTAL_G} G > ${BUDGET} G — scripts/disk-check.sh --trim · 끝난 워크트리는 git worktree remove · 그래도 크면 cargo clean --release"
    exit 1
fi
echo "★ DISK OK(${TOTAL_G} G ≤ ${BUDGET} G)"
