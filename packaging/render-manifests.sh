#!/usr/bin/env bash
# 패키지 관리자 매니페스트 생성 — winget · Chocolatey · Homebrew(T-160 · 이식 원천 = ../nexa-clip/packaging/render-manifests.sh f3a6781 +
# ../nexa-sql/packaging/render-manifests.ps1(MSI ProductCode)).
#
#   packaging/render-manifests.sh <VERSION> <자산디렉터리> <출력디렉터리>
#
# 자산 디렉터리에는 릴리스에 올라간 파일이 그대로 있어야 한다(msi · 포터블 zip · pkg · `*.msi.productcode.txt`).
# 체크섬은 **그 파일들에서 직접 계산한다** · MSI ProductCode는 빌드가 남긴 곁 파일(build-msi.ps1)에서 읽는다(WiX v4는 빌드마다 새
# ProductCode를 만든다 · UpgradeCode는 고정). ★ 이 스크립트가 **유일한 치환 지점**이다(릴리스 · 제출 워크플로가 같은 것을 부른다).
#
# 🔴 게이트(nexa-clip 0.1.5 반려 · beep 반려 3건의 교훈): ① nuspec `<copyright>` 필수 ② 영어 라이선스 줄(`License: … noncommercial use`)
#   필수 ③ winget · choco 제출 파일은 전부 **영어/ASCII**(ps1 머리 BOM만 예외) ④ 치환되지 않은 자리표시자 0. 어기면 exit 1.
set -euo pipefail

VERSION="${1:?사용법: render-manifests.sh <VERSION> <자산디렉터리> <출력디렉터리>}"
ASSETS="${2:?자산 디렉터리}"
OUT="${3:?출력 디렉터리}"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

sha() {
  local f="$ASSETS/$1"
  [ -f "$f" ] || { echo "::error::자산 없음: $1" >&2; exit 1; }
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$f" | cut -d' ' -f1; else shasum -a 256 "$f" | cut -d' ' -f1; fi
}

V="$VERSION"
MSI="nexa-dir-$V-windows-x64.msi"
SHA_WIN_X64_MSI=$(sha "$MSI")
SHA_WIN_X64_PORTABLE=$(sha "nexa-dir-$V-windows-x64-portable.zip")
SHA_MAC_PKG=$(sha "nexa-dir-$V-macos-universal.pkg")
PC_FILE="$ASSETS/$MSI.productcode.txt"
[ -f "$PC_FILE" ] || { echo "::error::ProductCode 곁 파일 없음: $MSI.productcode.txt(build-msi.ps1이 만든다)" >&2; exit 1; }
PRODUCT_CODE=$(tr -d '\r\n[:space:]' < "$PC_FILE")
case "$PRODUCT_CODE" in
  \{[0-9A-Fa-f]*-*\}) ;;
  *) echo "::error::ProductCode 모양이 아니다: '$PRODUCT_CODE'" >&2; exit 1 ;;
esac
DATE=$(date -u +%Y-%m-%d)

fill() {
  sed -e "s/@VERSION@/$V/g" \
      -e "s/@DATE@/$DATE/g" \
      -e "s/@PRODUCT_CODE@/$PRODUCT_CODE/g" \
      -e "s/@SHA_WIN_X64_MSI@/$SHA_WIN_X64_MSI/g" \
      -e "s/@SHA_WIN_X64_PORTABLE@/$SHA_WIN_X64_PORTABLE/g" \
      -e "s/@SHA_MAC_PKG@/$SHA_MAC_PKG/g" \
      "$1" > "$2"
}

mkdir -p "$OUT"

# ── winget(설치본 · 포터블) — microsoft/winget-pkgs 경로 규약: 소문자 첫 글자 / 식별자의 점을 디렉터리로 ──
for ch in installer portable; do
  id="SosomLab.NexaDir"; [ "$ch" = portable ] && id="SosomLab.NexaDir.Portable"
  dir="$OUT/winget/manifests/s/$(echo "$id" | tr '.' '/')/$V"
  mkdir -p "$dir"
  fill "$here/winget/$ch/version.yaml"   "$dir/$id.yaml"
  fill "$here/winget/$ch/locale.yaml"    "$dir/$id.locale.en-US.yaml"
  fill "$here/winget/$ch/installer.yaml" "$dir/$id.installer.yaml"
done

# ── Chocolatey(설치본 · 포터블) — 그대로 `choco pack` 가능한 형태 · 식별자 = dir2 계승(nexa-dir · nexa-dir.portable) ──
for ch in installer portable; do
  pkg="nexa-dir"; [ "$ch" = portable ] && pkg="nexa-dir.portable"
  dir="$OUT/choco/$pkg"
  mkdir -p "$dir/tools"
  fill "$here/choco/$ch/$pkg.nuspec"                   "$dir/$pkg.nuspec"
  fill "$here/choco/$ch/tools/chocolateyinstall.ps1"   "$dir/tools/chocolateyinstall.ps1"
  fill "$here/choco/$ch/tools/chocolateyuninstall.ps1" "$dir/tools/chocolateyuninstall.ps1"
  nus="$dir/$pkg.nuspec"
  grep -q '<copyright>[^<]\{1,\}</copyright>' "$nus" \
    || { echo "::error::$pkg.nuspec: <copyright>가 없다(Chocolatey 검수 요구)" >&2; exit 1; }
  grep -qi '^License: .*noncommercial use' "$nus" \
    || { echo "::error::$pkg.nuspec: 영어 라이선스 줄(License: … noncommercial use …)이 없다" >&2; exit 1; }
done

# 🔴 winget · Chocolatey 제출 파일은 전부 영어 — ASCII 밖 바이트가 있으면 멈춘다(ps1 머리 BOM만 예외).
bad=0
while IFS= read -r f; do
  if hit=$(sed $'1s/^\xEF\xBB\xBF//' "$f" | LC_ALL=C grep -n $'[\x80-\xFF]'); then
    echo "$f:"; echo "$hit"; bad=1
  fi
done < <(find "$OUT/winget" "$OUT/choco" -type f \( -name '*.yaml' -o -name '*.nuspec' -o -name '*.ps1' \))
if [ "$bad" = 1 ]; then
  echo "::error::winget·choco 제출 파일에 영어가 아닌 글자가 있다(위 줄) — 전부 영어로 쓴다" >&2
  exit 1
fi

# ── Homebrew(탭 저장소 배치 그대로: Casks/) — 우리 탭이라 영어 게이트 대상 아님 ──
mkdir -p "$OUT/homebrew/Casks"
fill "$here/homebrew/nexa-dir.rb" "$OUT/homebrew/Casks/nexa-dir.rb"

if grep -rn '@[A-Z_]*@' "$OUT"; then
  echo "::error::치환되지 않은 자리표시자가 남았다" >&2
  exit 1
fi

echo "생성 완료 ($V · ProductCode $PRODUCT_CODE):"
find "$OUT" -type f | sort
