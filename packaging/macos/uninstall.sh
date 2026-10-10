#!/bin/sh
# uninstall.sh — Nexa Dir 제거(macOS). pkg 설치본에는 제거기가 없으므로 이 스크립트가 그 역할이다
#   (번들 `Contents/Resources/uninstall.sh`에도 동봉 · docs/33 §3 verify "제거 → 잔여 파일 0(사용자 폴더는 보존)").
#
#   sudo "/Applications/Nexa Dir.app/Contents/Resources/uninstall.sh"          # 앱 + CLI 링크 + pkg 영수증
#   sudo … uninstall.sh --purge                                                  # + 사용자 데이터(설정·프로필·로그·캐시)까지
#
# 지우는 것: /Applications/Nexa Dir.app · /usr/local/bin/nexa-dir(우리 링크일 때만) · pkgutil 영수증(com.sosomlab.nexa-dir).
# 보존(기본): ~/Library/Application Support/nexa-dir · ~/Library/Logs/nexa-dir · ~/Library/Caches/nexa-dir — --purge로만.
set -eu
APP="/Applications/Nexa Dir.app"
LINK="/usr/local/bin/nexa-dir"
PKG_ID="com.sosomlab.nexa-dir"
PURGE=0; [ "${1:-}" = "--purge" ] && PURGE=1

# 사용자에게 보이는 메시지는 영어(게시물 언어 규칙 · docs/16 §5-6 · 사용자 10-10).
if [ -L "$LINK" ] && [ "$(readlink "$LINK")" = "$APP/Contents/MacOS/nexa-dir" ]; then rm -f "$LINK"; echo "removed: $LINK"; fi
[ -d "$APP" ] && { rm -rf "$APP"; echo "removed: $APP"; }
pkgutil --pkgs 2>/dev/null | grep -q "^$PKG_ID\$" && { pkgutil --forget "$PKG_ID" >/dev/null && echo "receipt forgotten: $PKG_ID"; }

if [ "$PURGE" = 1 ]; then
    # sudo로 돌리면 $HOME이 root라 실제 사용자 홈을 찾는다.
    U="${SUDO_USER:-$(id -un)}"; H="$(dscl . -read "/Users/$U" NFSHomeDirectory 2>/dev/null | awk '{print $2}')"; H="${H:-$HOME}"
    for d in "$H/Library/Application Support/nexa-dir" "$H/Library/Logs/nexa-dir" "$H/Library/Caches/nexa-dir"; do
        [ -e "$d" ] && { rm -rf "$d"; echo "removed (user data): $d"; }
    done
else
    echo "User data was kept (~/Library/Application Support/nexa-dir etc.) - run with --purge to remove it too"
fi
echo "done"
