#!/usr/bin/env bash
# build-deb.sh — Linux 설치본 `.deb`(dpkg-deb · FHS 레이아웃 docs/33 §2 · T-72/T-62).
#
#   packaging/linux/build-deb.sh [--skip-build] [--target x86_64-unknown-linux-gnu]
#
# 레이아웃: /usr/bin/nexa-dir · /usr/share/nexa-dir/plugins/ · /usr/share/applications/nexa-dir.desktop ·
#           /usr/share/icons/hicolor/<N>x<N>/apps/nexa-dir.png(16…512) · /usr/share/doc/nexa-dir/{LICENSE.md,…}
# 산출: target/packaging/linux/nexa-dir_<ver>_<amd64|arm64>.deb
# 이식 원천: ../nexa-clip/.github/workflows/release.yml "Linux 설치본(.deb)" 단계 + packaging/linux/control.
# 배포판 비종속(glibc ≥ 2.17)은 nexa-clip처럼 zig 링커(cargo zigbuild)를 쓸 때 성립 — release.yml이 담당 · 로컬은 호스트 glibc.
# 검증(설치 없이): dpkg-deb --info / --contents. 설치 스모크는 CI(sudo dpkg -i → nexa-dir --version → dpkg -r).
source "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

SKIP_BUILD=0
TARGET="${NDIR_LINUX_TARGET:-x86_64-unknown-linux-gnu}"
while [ $# -gt 0 ]; do
    case "$1" in
        --skip-build) SKIP_BUILD=1 ;;
        --target) shift; TARGET="$1" ;;
        *) die "알 수 없는 인자: $1" ;;
    esac
    shift
done
case "$TARGET" in aarch64-*) ARCH=arm64 ;; *) ARCH=amd64 ;; esac
have dpkg-deb || die "dpkg-deb 없음(apt-get install dpkg-dev)"

OUT="$OUT_ROOT/linux"; PKG="$OUT/deb-root"; rm -rf "$PKG"
FINAL="$OUT/nexa-dir_${VERSION}_${ARCH}.deb"
BIN="$ROOT/target/$TARGET/release"

step "빌드 (release · $TARGET)"
if [ "$SKIP_BUILD" = 0 ]; then
    if [ -n "${NDIR_ZIGBUILD:-}" ]; then
        "$(cargo_bin)" zigbuild --release --locked -p nexa-dir --target "${TARGET}.2.17"
    else
        "$(cargo_bin)" build --release --locked -p nexa-dir --target "$TARGET"
    fi
fi
[ -x "$BIN/nexa-dir" ] || die "산출물 없음: $BIN"

step "스테이징(FHS) → $PKG"
mkdir -p "$PKG/DEBIAN" "$PKG/usr/bin" "$PKG/usr/share/applications" "$PKG/usr/share/doc/nexa-dir" "$PKG/usr/share/nexa-dir"
install -m 0755 "$BIN/nexa-dir" "$PKG/usr/bin/nexa-dir"
install -m 0644 "$ROOT/packaging/linux/nexa-dir.desktop" "$PKG/usr/share/applications/nexa-dir.desktop"
for n in 16 24 32 48 64 128 256 512; do
    d="$PKG/usr/share/icons/hicolor/${n}x${n}/apps"; mkdir -p "$d"
    install -m 0644 "$BRANDING/png/nexa-dir-$n.png" "$d/nexa-dir.png"
done
# 공통(라이선스·README·THIRD-PARTY-NOTICES·plugins) → /usr/share/nexa-dir · 문서는 /usr/share/doc 규약 자리에도.
stage_common "$PKG/usr/share/nexa-dir"
for f in LICENSE.md LICENSE.ko.md README.md README.ko.md THIRD-PARTY-NOTICES.txt; do mv "$PKG/usr/share/nexa-dir/$f" "$PKG/usr/share/doc/nexa-dir/$f"; done
# Debian 정책: /usr/share/doc/<pkg>/copyright
{ echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/"; echo "Upstream-Name: nexa-dir"; echo "Source: $HOMEPAGE"; echo;
  echo "Files: *"; echo "Copyright: 2026 SosomLab"; echo "License: PolyForm-Noncommercial-1.0.0"; echo " See LICENSE.md in this directory."; } > "$PKG/usr/share/doc/nexa-dir/copyright"
size_kb=$(du -sk "$PKG/usr" | cut -f1)
sed -e "s/@VERSION@/$VERSION/g" -e "s/@ARCH@/$ARCH/g" -e "s/@SIZE@/$size_kb/g" "$ROOT/packaging/linux/control" > "$PKG/DEBIAN/control"
# 설치 후 아이콘·데스크톱 캐시 갱신(없으면 무시).
cat > "$PKG/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q /usr/share/applications 2>/dev/null || true
exit 0
EOF
cp "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/postrm"; chmod 0755 "$PKG/DEBIAN/postinst" "$PKG/DEBIAN/postrm"

step "dpkg-deb --build"
mkdir -p "$OUT"; rm -f "$FINAL"
dpkg-deb --build --root-owner-group "$PKG" "$FINAL" >/dev/null
note "$(basename "$FINAL") $(du -h "$FINAL" | cut -f1) · sha256 $(sha256_of "$FINAL")"

step "검증(설치 없이)"
dpkg-deb --info "$FINAL" | sed -n '1,6p' | sed 's/^/    /'
dpkg-deb --contents "$FINAL" | grep -E 'usr/bin/nexa-dir$|nexa-dir.desktop$|256x256/apps/nexa-dir.png$|plugins/.*wasm$' | awk '{print "    " $NF}'
echo "DEB=$FINAL"
