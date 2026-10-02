#!/usr/bin/env bash
# plugin-build.sh — 동봉 미리보기 플러그인(.wasm) 빌드(dir2 build-plugins.ps1 3-OS 이식 · docs/18 §2-1 · T-63/T-07).
#   plugins/sdk/plugins.list(단일 출처)의 각 크레이트를 wasm32-unknown-unknown으로 빌드해
#   ① plugins/<이름>.wasm(저장소 동봉본 — E2E 시험·앱이 로드)  ② --out-dir <폴더>(배포 스테이징·CI 검증)에 복사한다.
# 사용:  scripts/plugin-build.sh                       # 동봉본(plugins/) 갱신
#        scripts/plugin-build.sh --out-dir target/plugins-ci --skip-dist   # 동봉본은 그대로 · 지정 폴더만(CI)
# 사전 준비(1회): rustup target add wasm32-unknown-unknown
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
OUT_DIR=""; SKIP_DIST=0
while [ $# -gt 0 ]; do
    case "$1" in
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --skip-dist) SKIP_DIST=1; shift ;;
        -h|--help) sed -n 2,8p "$0"; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done
if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
    echo "✗ wasm32-unknown-unknown 타깃이 없습니다 — rustup target add wasm32-unknown-unknown" >&2; exit 1
fi
if [ -n "$OUT_DIR" ]; then mkdir -p "$OUT_DIR"; OUT_DIR="$(cd "$OUT_DIR" && pwd)"; fi
LIST="$ROOT/plugins/sdk/plugins.list"
FAIL=0; BUILT=0
while read -r dir artifact name; do
    case "$dir" in ''|'#'*) continue ;; esac
    src="$ROOT/plugins/sdk/$dir"
    if [ ! -d "$src" ]; then echo "✗ sdk 폴더 없음: $src" >&2; FAIL=$((FAIL+1)); continue; fi
    # 게스트 크레이트는 앱 워크스페이스 밖(빈 [workspace]) — 각자 target/ 에 쌓인다.
    if ! (cd "$src" && cargo build --release --target wasm32-unknown-unknown); then
        echo "✗ cargo build 실패: $dir" >&2; FAIL=$((FAIL+1)); continue
    fi
    built="$src/target/wasm32-unknown-unknown/release/$artifact"
    if [ ! -f "$built" ]; then echo "✗ 산출물 없음: $built" >&2; FAIL=$((FAIL+1)); continue; fi
    kb=$(( $(wc -c < "$built") / 1024 ))
    if [ "$SKIP_DIST" -eq 0 ]; then cp -f "$built" "$ROOT/plugins/$name"; fi
    if [ -n "$OUT_DIR" ]; then cp -f "$built" "$OUT_DIR/$name"; fi
    echo "✓ $name = ${kb} KB  ($dir)"; BUILT=$((BUILT+1))
done < "$LIST"
echo "— built $BUILT · failed $FAIL · dist $([ "$SKIP_DIST" -eq 0 ] && echo updated || echo kept) · out-dir ${OUT_DIR:-(none)}"
[ "$FAIL" -eq 0 ] && [ "$BUILT" -gt 0 ]
