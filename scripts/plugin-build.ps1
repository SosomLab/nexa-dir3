# plugin-build.ps1 — 동봉 미리보기 플러그인(.wasm) 빌드(dir2 build-plugins.ps1 이식 · sh 판과 같은 목록·옵션 · docs/18 §2-1 · T-63/T-07).
#   plugins/sdk/plugins.list(단일 출처)의 각 크레이트를 wasm32-unknown-unknown으로 빌드해
#   ① plugins\<이름>.wasm(저장소 동봉본)  ② -OutDir <폴더>(배포 스테이징·CI 검증)에 복사한다.
# 사용:  pwsh scripts/plugin-build.ps1                                   # 동봉본(plugins\) 갱신
#        pwsh scripts/plugin-build.ps1 -OutDir target/plugins-ci -SkipDist  # 동봉본은 그대로 · 지정 폴더만
# 사전 준비(1회): rustup target add wasm32-unknown-unknown
[CmdletBinding()]
param(
    [string]$OutDir,
    [switch]$SkipDist
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot

if (-not (rustup target list --installed | Select-String -SimpleMatch "wasm32-unknown-unknown")) {
    throw "wasm32-unknown-unknown 타깃이 없습니다 — rustup target add wasm32-unknown-unknown"
}
if ($OutDir) {
    $OutDir = [System.IO.Path]::GetFullPath((Join-Path $repo $OutDir))
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
}
$list = Join-Path $repo "plugins\sdk\plugins.list"
$built = 0
foreach ($line in Get-Content $list) {
    $t = $line.Trim()
    if ($t -eq "" -or $t.StartsWith("#")) { continue }
    $parts = $t -split "\s+"
    $dir, $artifact, $name = $parts[0], $parts[1], $parts[2]
    $src = Join-Path $repo "plugins\sdk\$dir"
    if (-not (Test-Path $src)) { throw "sdk 폴더 없음: $src" }
    Push-Location $src
    try {
        cargo build --release --target wasm32-unknown-unknown
        if ($LASTEXITCODE -ne 0) { throw "cargo build 실패: $dir" }
    } finally {
        Pop-Location
    }
    $out = Join-Path $src "target\wasm32-unknown-unknown\release\$artifact"
    if (-not (Test-Path $out)) { throw "산출물 없음: $out" }
    $kb = [math]::Round((Get-Item $out).Length / 1KB, 1)
    if (-not $SkipDist) { Copy-Item $out (Join-Path $repo "plugins\$name") -Force }
    if ($OutDir) { Copy-Item $out (Join-Path $OutDir $name) -Force }
    Write-Output "✓ $name = $kb KB  ($dir)"
    $built++
}
Write-Output "— built $built · dist $(if ($SkipDist) { 'kept' } else { 'updated' }) · out-dir $(if ($OutDir) { $OutDir } else { '(none)' })"
if ($built -eq 0) { throw "빌드된 플러그인이 없습니다(plugins.list 비어 있음?)" }
