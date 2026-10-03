# check-all.ps1 — `check-all.sh`의 PowerShell 래퍼(Windows · Git Bash 필요 · T-05 CI-115).
#   사용: scripts\check-all.ps1 [--quick] [--out <폴더>]
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$bash = Get-Command bash -ErrorAction SilentlyContinue
if (-not $bash) {
    Write-Error "bash(Git for Windows)가 PATH에 없습니다 — scripts/check-all.sh 를 Git Bash에서 실행하세요."
    exit 2
}
& $bash.Source "$root/scripts/check-all.sh" @args
exit $LASTEXITCODE
