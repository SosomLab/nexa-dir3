#!/usr/bin/env bash
# install-dev-desktop.sh — 개발 빌드용 사용자 영역 .desktop + hicolor 아이콘 설치(T-113 · Linux · 10-03 §100 수동 설치의 스크립트화).
#
#   scripts/install-dev-desktop.sh            # target/debug/nexa-dir 를 가리키는 nexa-dir-dev.desktop 설치
#   scripts/install-dev-desktop.sh --release  # target/release/nexa-dir
#   scripts/install-dev-desktop.sh --bin <경로> # 임의 바이너리
#   scripts/install-dev-desktop.sh --remove   # 설치한 .desktop + 아이콘 제거
#
# 왜: Wayland GNOME은 창의 app_id(`nexa-dir`)와 같은 이름의 .desktop + 아이콘 테마 항목이 없으면 도크/작업 전환기에 톱니바퀴를 보인다.
# 배포본(deb/rpm)은 /usr/share에 같은 것을 넣는다(packaging/linux/build-deb.sh) — 이 스크립트는 **사용자 영역**(~/.local/share)에만 쓰고
# 시스템 파일은 건드리지 않는다. .desktop 본문은 packaging/linux/nexa-dir.desktop을 그대로 쓰고 Exec만 바꾼다(한 원천).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS="$DATA/applications"
ICONS="$DATA/icons/hicolor"
# 파일 이름은 app_id와 같아야 한다(StartupWMClass/app_id 대조) — 개발 빌드도 `nexa-dir.desktop`.
DESKTOP="$APPS/nexa-dir.desktop"
BIN="$ROOT/target/debug/nexa-dir"
REMOVE=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) BIN="$ROOT/target/release/nexa-dir" ;;
        --bin) BIN="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"; shift ;;
        --remove) REMOVE=1 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
    shift
done
[[ "$(uname -s)" == Linux ]] || { echo "Linux 전용"; exit 2; }

refresh() {
    command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f "$ICONS" 2>/dev/null || true
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$APPS" 2>/dev/null || true
}

if [[ $REMOVE -eq 1 ]]; then
    rm -f "$DESKTOP"
    for n in 16 24 32 48 64 128 256 512; do rm -f "$ICONS/${n}x${n}/apps/nexa-dir.png"; done
    refresh
    echo "제거: $DESKTOP + hicolor 아이콘 8종"
    exit 0
fi

[[ -x "$BIN" ]] || { echo "바이너리 없음: $BIN (먼저 cargo build -p nexa-dir)" >&2; exit 1; }
mkdir -p "$APPS"
# Exec만 절대 경로로 바꾼다(공백 있는 경로 대비 따옴표).
sed "s|^Exec=.*|Exec=\"$BIN\" %f|" "$ROOT/packaging/linux/nexa-dir.desktop" > "$DESKTOP"
chmod 0644 "$DESKTOP"
for n in 16 24 32 48 64 128 256 512; do
    src="$ROOT/packaging/branding/png/nexa-dir-$n.png"
    [[ -f "$src" ]] || continue
    d="$ICONS/${n}x${n}/apps"; mkdir -p "$d"
    install -m 0644 "$src" "$d/nexa-dir.png"
done
refresh
echo "설치: $DESKTOP → $BIN"
echo "      아이콘: $ICONS/<N>x<N>/apps/nexa-dir.png (16…512)"
echo "GNOME에서 바로 반영되지 않으면 로그아웃/로그인 또는 Alt+F2 → r (X11)."
