<#
.SYNOPSIS
  build-zip.ps1 — Windows 포터블 zip(dir2 채널 · docs/port/44 §3-4 결정 ①: MSI + 포터블 zip 둘 다).
.DESCRIPTION
  build-msi.ps1이 만든 스테이징(target\packaging\windows\stage — nexa-dir.exe · plugins\ · LICENSE… · license.rtf 제외)을 그대로
  `nexa-dir-<ver>-windows-<x64|arm64>-portable.zip`으로 담는다. 포터블 = 압축을 풀어 바로 실행 · 설정은 %APPDATA%\nexa-dir.
  스테이징이 없으면 build-msi.ps1 -SkipBuild를 먼저 돌린다(같은 바이너리 보증).
#>
[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$Version = ""
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Out = Join-Path $Root "target\packaging\windows"
$Stage = Join-Path $Out "stage"
if (-not $Version) { $Version = $env:NDIR_VERSION }
if (-not $Version) {
    $Version = (Select-String -Path (Join-Path $Root "Cargo.toml") -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
}
$Arch = if ($Target -like "aarch64-*") { "arm64" } else { "x64" }
if (-not (Test-Path (Join-Path $Stage "nexa-dir.exe"))) { throw "스테이징 없음($Stage) — 먼저 packaging\windows\build-msi.ps1" }
$Zip = Join-Path $Out "nexa-dir-$Version-windows-$Arch-portable.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
$items = Get-ChildItem $Stage | Where-Object { $_.Name -ne "license.rtf" }
Compress-Archive -Path ($items | ForEach-Object { $_.FullName }) -DestinationPath $Zip -CompressionLevel Optimal
$sha = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLower()
Write-Host ("  · {0}  {1:N0} bytes  sha256 {2}" -f (Split-Path -Leaf $Zip), (Get-Item $Zip).Length, $sha)
Write-Output "ZIP=$Zip"
