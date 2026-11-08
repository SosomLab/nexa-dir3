#!/usr/bin/env bash
# release-notes.sh — GitHub Release 본문 조립(T-206 · 11-09).
#
# 사용: bash scripts/release-notes.sh <version> <body.md> [tag-notes.txt] > notes.md
#       bash scripts/release-notes.sh --selftest
#   version        = 0.30.2 (앞의 v 없이)
#   body.md        = release.yml이 만든 공통 본문(설치 표 · 패키지 관리자 · 서명 · 라이선스 · 제목 줄 없음)
#   tag-notes.txt  = docs/release/tag-v<version>.txt(태그 주석 원고 · 영어). 없으면 "What's new" 절을 생략하고
#                    stderr에 알린다(수동 실행 · 선검증 초안 · 원고 미작성).
#
# 변환(0.30.1 · 0.30.2 수동 보강과 같은 모양):
#   - 1행(제목 "Nexa Dir <ver> — …") · 뒤따르는 빈 줄은 뺀다(Release 제목이 이미 있다).
#   - "Packages:"로 시작하는 문단은 뺀다(본문의 설치 표와 중복).
#   - "Fixes since 0.30.1:"처럼 콜론으로 끝나는 홑줄 소제목은 굵게.
#   - 그 뒤 `---` 구분선 + 공통 본문.
# bash 3.2(macOS)에서도 돌아야 한다 — 배열 · mapfile · ${var,,} 금지(d022630 교훈).
set -euo pipefail

whats_new() {
    # $1 = tag-notes.txt → 변환된 본문을 stdout에.
    awk '
        NR == 1 { next }                       # 제목 줄
        NR == 2 && $0 == "" { next }           # 제목 뒤 빈 줄
        /^Packages:/ { skip = 1 }
        skip && $0 == "" { skip = 0; next }    # Packages 문단 끝(빈 줄)
        skip { next }
        /^[A-Z][A-Za-z0-9 .,()\/-]*:$/ { print "**" $0 "**"; next }
        { print }
    ' "$1" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'   # 끝의 빈 줄 제거
}

compose() {
    # $1 = version · $2 = body.md · $3 = tag-notes.txt(없어도 됨)
    local ver="$1" body="$2" tag="${3:-}"
    echo "# Nexa Dir $ver"
    echo
    if [ -n "$tag" ] && [ -f "$tag" ]; then
        echo "## What's new in $ver"
        echo
        whats_new "$tag"
        echo
        echo "---"
        echo
    else
        echo "release-notes: $tag 없음 — What's new 절 생략(수동 실행 또는 태그 주석 원고 미작성)" >&2
    fi
    cat "$body"
}

selftest() {
    local root tmp body out n=0
    root="$(cd "$(dirname "$0")/.." && pwd)"
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/ndir-release-notes.XXXXXX")"
    # shellcheck disable=SC2064 — 지금 값으로 고정(local 변수는 EXIT 때 없다)
    trap "rm -rf '$tmp'" EXIT
    body="$tmp/body.md"
    printf 'Body line 1.\n\n## Package managers\n\nbody end\n' > "$body"
    fail() { echo "release-notes --selftest: $*" >&2; exit 1; }

    for f in "$root"/docs/release/tag-v*.txt; do
        [ -f "$f" ] || continue
        n=$((n + 1))
        ver="${f##*/tag-v}"; ver="${ver%.txt}"
        out="$tmp/notes-$ver.md"
        compose "$ver" "$body" "$f" > "$out" 2>/dev/null
        [ "$(head -1 "$out")" = "# Nexa Dir $ver" ] || fail "$ver: 제목 줄"
        grep -qx "## What's new in $ver" "$out" || fail "$ver: What's new 소제목"
        grep -qx -- '---' "$out" || fail "$ver: 구분선"
        grep -q '^Packages:' "$out" && fail "$ver: Packages 문단이 남음"
        grep -qx 'body end' "$out" || fail "$ver: 공통 본문 누락"
        grep -q "^Nexa Dir $ver — " "$out" && fail "$ver: 원고 제목 줄이 남음"
        # 원고에 "Xxx since …:" 소제목이 있으면 굵게 바뀌어 있어야 한다.
        if grep -q '^Fixes since [^:]*:$' "$f"; then
            grep -q '^\*\*Fixes since [^:]*:\*\*$' "$out" || fail "$ver: 소제목 굵게"
        fi
        # What's new가 구분선 앞 · 공통 본문이 뒤.
        [ "$(grep -n -x -- '---' "$out" | head -1 | cut -d: -f1)" -lt "$(grep -n -x 'body end' "$out" | cut -d: -f1)" ] || fail "$ver: 순서"
    done
    [ "$n" -ge 1 ] || fail "docs/release/tag-v*.txt 없음"

    # 원고가 없을 때 = 제목 + 공통 본문만 · stderr 알림.
    out="$tmp/notes-none.md"
    compose "9.9.9" "$body" "$tmp/no-such.txt" > "$out" 2> "$tmp/err"
    grep -q "What's new" "$out" && fail "원고 없음: What's new가 있음"
    grep -q '없음' "$tmp/err" || fail "원고 없음: stderr 알림"
    [ "$(sed -n 3p "$out")" = "Body line 1." ] || fail "원고 없음: 본문 위치"
    echo "release-notes --selftest: OK ($n 원고 · 원고 없음 1)"
}

case "${1:-}" in
    --selftest) selftest ;;
    -h|--help|"") sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit "${1:+0}" ;;
    *) [ $# -ge 2 ] || { echo "사용: $0 <version> <body.md> [tag-notes.txt]" >&2; exit 2; }
       compose "$1" "$2" "${3:-}" ;;
esac
