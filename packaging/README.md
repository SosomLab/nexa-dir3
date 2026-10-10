# packaging — 설치본 · 패키지 관리자 매니페스트

> **언어 규칙(docs/16 §5-6 · 사용자 10-10)**: 이 폴더의 매니페스트·설명·동봉 문서·릴리스 노트 등 **게시물에 들어가는 문구는 영어**가 원본이다. 한글·일본어는 `Comment[ko]` 같은 현지화 항목으로만 덧붙인다. (이 README처럼 개발자용 설명은 한글.)

| 폴더/파일 | 내용 |
| --- | --- |
| `windows/` | WiX v4 MSI(`build-msi.ps1` · perMachine · UpgradeCode 고정 `7D1F7E3A-…`) + 포터블 zip(`build-zip.ps1` · zip 뿌리에 `nexa-dir.exe`) · 빌드가 `<msi>.productcode.txt`를 곁에 남긴다 |
| `macos/` | Universal 2 `.app` → `.pkg`(`com.sosomlab.nexa-dir`) · `.dmg` · `uninstall.sh` |
| `linux/` | deb · rpm |
| `winget/` · `choco/` · `homebrew/` | 패키지 관리자 **틀**(자리표시자 `@VERSION@` `@DATE@` `@SHA_*@` `@PRODUCT_CODE@`) |
| `render-manifests.sh` | 릴리스 자산에서 해시 · ProductCode를 읽어 틀을 채운다 — **유일한 치환 지점**(릴리스 · 제출 워크플로가 함께 부른다) |

## 채널(T-160 · 10-05 결정 = dir2 이름 승계 · 태그 `v0.23.0` 정식)

| 채널 | 식별자 | 워크플로 | 스위치 |
| --- | --- | --- | --- |
| GitHub Release | 태그 `v*` → **즉시 공개**(초안은 수동 실행만) | `release.yml` | — |
| winget | `SosomLab.NexaDir`(MSI · `UpgradeBehavior: uninstallPrevious`) · `SosomLab.NexaDir.Portable`(zip) | `publish-windows-packages.yml` | 변수 `WINGET_PUBLISH=true` + 시크릿 `WINGET_TOKEN` |
| Chocolatey | `nexa-dir`(MSI · dir2 Inno 선제거) · `nexa-dir.portable` | 같음 | 변수 `CHOCO_PUSH=true` + 시크릿 `CHOCO_API_KEY` |
| Homebrew | Cask `kiros33/tap/nexa-dir`(Universal pkg · postflight quarantine 제거) | `homebrew.yml` | 시크릿 `TAP_TOKEN` |
| pkg.sosomlab.com(apt/rpm) | `repository_dispatch app-released` → SosomLab/linux-repo | `release.yml` linux-repo 잡 | 시크릿 `LINUX_REPO_DISPATCH_TOKEN` · 첫 공개 뒤 `linux-repo/apps/nexa-dir.toml` 등록 |

## 🔴 제출 파일 규칙(nexa-clip 0.1.5 · nexa-beep 반려의 교훈 · `render-manifests.sh`가 강제)

1. **winget · Chocolatey 제출 틀(`winget/**` · `choco/**`)은 전부 영어** — 설명 · 스크립트 주석 · `Write-Host` 문구 · YAML 주석까지. ASCII 밖 글자가 있으면 멈춘다(ps1 머리 BOM만 예외).
2. nuspec `<copyright>` 필수 · `License: … noncommercial use` 영어 문장 필수 · `iconUrl` = jsDelivr + 태그 고정 · 설명에 이메일 금지.
3. 치환되지 않은 자리표시자(`@…@`)가 남으면 멈춘다 · 해시는 산출물에서 계산 · ProductCode는 MSI 곁 파일에서.
4. winget 설치본에 `DisplayVersion`을 넣지 않는다(dir2 Needs-Author-Feedback) · 정적 CRT(`.cargo/config.toml`) · 워크플로 env `RUSTFLAGS` 금지.
5. 반려 뒤에는 **같은 버전**으로 재제출(`publish-windows-packages` 수동 `tag=vX.Y.Z force=true` · 그동안 `WINGET_PUBLISH=false`).
6. 점검은 패키지 페이지의 상태 문구 + 댓글(choco `https://community.chocolatey.org/packages/nexa-dir/<ver>` · "Waiting for Maintainer" = 우리 차례) · winget = PR 라벨 + 댓글.
7. linux-repo는 공개된 릴리스 자산만 본다 — 재업로드(`--clobber`)는 서명 색인을 깨므로 금지.
