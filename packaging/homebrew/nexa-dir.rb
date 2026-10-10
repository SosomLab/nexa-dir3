# Homebrew Cask — Nexa Dir(macOS 설치본 채널: Universal 2 .pkg · /Applications/Nexa Dir.app + /usr/local/bin/nexa-dir 링크).
#
# 버전 · 체크섬 자리는 릴리스 워크플로가 실제 산출물 해시로 채운다(packaging/render-manifests.sh) — 손으로 적은 해시는 언젠가 틀린다.
# ★ 서명 · 공증이 없는 앱은 격리 표식(quarantine)이 붙어 있으면 실행 즉시 SIGKILL 된다(nexa-beep 08-11 실측) → postflight에서 뗀다
#   (caveats에 그대로 밝힌다). 게시물 문구(caveats 등)는 영어(docs/16 §5-6 · 사용자 10-10) — 주석만 한글.
cask "nexa-dir" do
  version "@VERSION@"
  sha256 "@SHA_MAC_PKG@"

  url "https://github.com/SosomLab/nexa-dir3/releases/download/v#{version}/nexa-dir-#{version}-macos-universal.pkg",
      verified: "github.com/SosomLab/nexa-dir3/"
  name "Nexa Dir"
  desc "Lightweight dual-panel file explorer with the same look everywhere"
  homepage "https://github.com/SosomLab/nexa-dir3"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :big_sur"

  pkg "nexa-dir-#{version}-macos-universal.pkg"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "/Applications/Nexa Dir.app"],
                   sudo: true, must_succeed: false
  end

  # brew style(T-181 · 10-08): 해시 들여쓰기 2칸 · stanza 순서 = uninstall → zap → caveats.
  uninstall script:  {
              executable: "/Applications/Nexa Dir.app/Contents/Resources/uninstall.sh",
              sudo:       true,
            },
            pkgutil: "com.sosomlab.nexa-dir"

  zap trash: [
    "~/Library/Application Support/nexa-dir",
    "~/Library/Saved Application State/com.sosomlab.nexa-dir.savedState",
  ]

  caveats <<~EOS
    This app is not code-signed or notarized (no certificate yet).
    The installer removes the macOS quarantine flag so the app launches right away.
    Verify your download against sha256sums.txt on the release page:
      https://github.com/SosomLab/nexa-dir3/releases

    License: PolyForm Noncommercial 1.0.0 - free for noncommercial use only.
  EOS
end
