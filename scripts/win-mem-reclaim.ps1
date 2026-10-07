# win-mem-reclaim.ps1 — **메모리 회수 4점 시험**(T-176 ④ · nexa-sql scripts/win-mem-reclaim.ps1 개념 이식 · docs/25 §2 D · 입력 주입 없음):
#   앱을 격리 홈으로 띄우고 기동 명령의 시차 실행(`@after:<ms>:<명령>`)으로 ① 기준(기동 + 안정) ② 올림(큰 폴더 · 미리보기 등)
#   ③ 놓음(작은 폴더로 돌아감) ④ 유휴 트림 뒤(격리 설정 `mem.idle_trim_s`를 짧게) 네 점에서 `mem.dump:<파일>`을 찍어 비교한다.
#   판정(docs/25 §3): ④ − ① 전용 워킹셋이 **+1 MB** 또는 **+5 %** 를 넘으면 WARN(피크가 돌아오지 않음 = 회수 결함 후보).
#
# 사용:
#   pwsh -NoProfile -File scripts/win-mem-reclaim.ps1 -HomeDir C:\tmp\ndir-reclaim -Raise "nav:C:\Windows\System32" -Release "nav:C:\Windows"
#   pwsh -NoProfile -File scripts/win-mem-reclaim.ps1 -HomeDir C:\tmp\ndir-reclaim -Raise "nav:C:\tmp\pics;list.select:0;list.select:1" -Release "nav:C:\Windows" -TrimSecs 5
#   -Exe 기본 target\release\nexa-dir.exe · -SettleMs 점마다 안정 시간(기본 3000) · 결과 = 표준 출력 + <HomeDir>\reclaim-*.txt 덤프 4개
param(
    [Parameter(Mandatory = $true)][string]$HomeDir,
    [Parameter(Mandatory = $true)][string]$Raise,
    [Parameter(Mandatory = $true)][string]$Release,
    [string]$Exe = "",
    [int]$SettleMs = 3000,
    [int]$TrimSecs = 5,
    [string]$Tag = "reclaim"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root "target\release\nexa-dir.exe" }
if ($HomeDir -match '\$') { throw "HomeDir looks wrong: '$HomeDir'" }
if (-not (Test-Path -LiteralPath $HomeDir)) { New-Item -ItemType Directory -Path $HomeDir -Force | Out-Null }
# 격리 설정: 유휴 트림 주기를 짧게(기본 60 s → TrimSecs) — 변경분만 쓰는 settings.conf(`key=value`).
$conf = Join-Path $HomeDir "settings.conf"
$keep = @()
if (Test-Path -LiteralPath $conf) { $keep = Get-Content -LiteralPath $conf -Encoding UTF8 | Where-Object { $_ -notmatch '^\s*mem\.idle_trim_s\s*=' } }
Set-Content -LiteralPath $conf -Value (@($keep) + "mem.idle_trim_s=$TrimSecs") -Encoding UTF8
$dumps = 1..4 | ForEach-Object { Join-Path $HomeDir "reclaim-$_.txt" }
$dumps | ForEach-Object { Remove-Item -LiteralPath $_ -ErrorAction SilentlyContinue }
# 시각표: 기준 = 기동 뒤 Settle · 올림 = 그 뒤 명령 + Settle · 놓음 = 명령 + Settle · 트림 = 마지막 입력 없는 상태로 TrimSecs + Settle.
$t1 = $SettleMs
$t2 = $t1 + $SettleMs
$t3 = $t2 + $SettleMs
$t4 = $t3 + $TrimSecs * 1000 + $SettleMs
$cmds = @("@after:${t1}:mem.dump:$($dumps[0])")
$k = 0
foreach ($c in ($Raise.Split(';') | Where-Object { $_ })) { $cmds += "@after:$($t1 + 100 + 200 * $k):$c"; $k++ }
$cmds += "@after:${t2}:mem.dump:$($dumps[1])"
$k = 0
foreach ($c in ($Release.Split(';') | Where-Object { $_ })) { $cmds += "@after:$($t2 + 100 + 200 * $k):$c"; $k++ }
$cmds += "@after:${t3}:mem.dump:$($dumps[2])"
$cmds += "@after:${t4}:mem.dump:$($dumps[3])"
$cmds += "@after:$($t4 + 500):quit"
$env:NDIR_HOME = $HomeDir
$env:NDIR_NO_ACTIVATE = "1"
$env:NDIR_STARTUP_CMD = ($cmds -join ',')
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru
try {
    $p.WaitForExit($t4 + 15000) | Out-Null
}
finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
}
function Parse([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    $line = (Get-Content -LiteralPath $path -Encoding UTF8 | Select-Object -First 1)
    # "sys footprint N resident N anon N file_backed N compressed N heap_used N heap_held N private_ws N"
    $m = [regex]::Matches($line, '(\w+) (\d+)')
    $h = @{}
    foreach ($x in $m) { $h[$x.Groups[1].Value] = [int64]$x.Groups[2].Value }
    $h
}
$pts = $dumps | ForEach-Object { Parse $_ }
$names = @("base", "raised", "released", "trimmed")
"{0}`t{1,-9} {2,10} {3,10} {4,10} {5,10} {6,10}" -f $Tag, "point", "footprint", "private_ws", "resident", "heap_used", "heap_held"
for ($i = 0; $i -lt 4; $i++) {
    $h = $pts[$i]
    if ($null -eq $h) { "{0}`t{1,-9} (no dump - 0 = measurement failure, rerun)" -f $Tag, $names[$i]; continue }
    "{0}`t{1,-9} {2,10:N2} {3,10:N2} {4,10:N2} {5,10:N2} {6,10:N2}  MB" -f $Tag, $names[$i], ($h.footprint / 1MB), ($h.private_ws / 1MB), ($h.resident / 1MB), ($h.heap_used / 1MB), ($h.heap_held / 1MB)
}
if ($pts[0] -and $pts[3]) {
    $b = $pts[0].private_ws; $t = $pts[3].private_ws
    if ($b -le 0) { $b = $pts[0].footprint; $t = $pts[3].footprint }
    $d = $t - $b
    $pct = if ($b -gt 0) { $d * 100.0 / $b } else { 0 }
    $verdict = if ($d -gt 1MB -or $pct -gt 5) { "WARN" } else { "OK" }
    "{0}`ttrimmed - base = {1:N2} MB ({2:N1} %) | peak(raised) = {3:N2} MB | {4}" -f $Tag, ($d / 1MB), $pct, ($pts[1].private_ws / 1MB), $verdict
}
else { "$Tag`tverdict unavailable (missing dumps)"; exit 1 }
