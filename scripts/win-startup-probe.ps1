# win-startup-probe.ps1 — **창 기동 비용** 측정(T-176 ④ · nexa-sql scripts/win-startup-probe.ps1 이식 · docs/25 §2 B · 입력 주입 없음):
#   프로세스를 띄워 ① 메인 창이 보일 때까지의 시간 ② `-SettleSecs`초까지 쓴 CPU ③ 그때의 Private · 워킹셋 · 핸들 · 스레드를
#   `-Runs`번 재고 중앙값을 낸다(perf-baseline.sh의 `--smoke`는 창 없는 경로 — 이 프로브가 실제 창까지 잰다).
#   두 빌드를 비교할 때는 번갈아 돌린다(파일 캐시 · 백신 검사의 영향을 고르게). 빌드 직후 첫 실행은 버린다(docs/25 §0).
#
# 사용:
#   pwsh -NoProfile -File scripts/win-startup-probe.ps1 -HomeDir C:\tmp\ndir-home -Runs 5
#   pwsh -NoProfile -File scripts/win-startup-probe.ps1 -HomeDir C:\tmp\ndir-home -Exe "C:\Program Files\Nexa Dir\nexa-dir.exe" -Tag installed
#   -Cmd "nav:C:\Windows\System32" = 기동 뒤 바로 실행할 기동 명령(큰 폴더로 시작하는 비용)
param(
    [string]$Exe = "",
    [Parameter(Mandatory = $true)][string]$HomeDir,
    [string]$Cmd = "",
    [string]$ArgList = "",
    [int]$Runs = 5,
    [int]$SettleSecs = 3,
    [string]$Tag = "startup"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root "target\release\nexa-dir.exe" }
if ($HomeDir -match '\$' -or -not (Test-Path -LiteralPath $HomeDir -PathType Container)) {
    throw "HomeDir does not exist or looks wrong: '$HomeDir' - create the sandbox folder first and check the path."
}
function Median($xs) { $s = @($xs | Sort-Object); $s[[int][Math]::Floor(($s.Count - 1) / 2)] }
$env:NDIR_HOME = $HomeDir
$env:NDIR_NO_ACTIVATE = "1"   # 시험 창이 사용자의 전경 포커스를 가져가지 않게(docs/18 §10)
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
# 측정이 끝나는 시각에 스스로 끝나게(종료 코드 0) — 이름으로 죽이지 않는다. 못 끝나면 내가 띄운 PID만.
$quitAt = $SettleSecs * 1000 + 500
$cmds = @()
if ($Cmd) { $cmds += $Cmd.Split(';') | Where-Object { $_ } }
$cmds += "@after:${quitAt}:quit"
$env:NDIR_STARTUP_CMD = ($cmds -join ',')
$rows = @()
foreach ($r in 1..$Runs) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    if ($ArgList) { $p = Start-Process -FilePath $Exe -ArgumentList $ArgList -WorkingDirectory (Split-Path $Exe) -PassThru }
    else { $p = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru }
    try {
        $shown = -1
        while ($sw.ElapsedMilliseconds -lt 15000) {
            $p.Refresh()
            if ($p.HasExited) { break }
            if ($p.MainWindowHandle -ne [IntPtr]::Zero) { $shown = $sw.ElapsedMilliseconds; break }
            Start-Sleep -Milliseconds 10
        }
        $left = $SettleSecs * 1000 - $sw.ElapsedMilliseconds
        if ($left -gt 0) { Start-Sleep -Milliseconds $left }
        $p.Refresh()
        if ($p.HasExited) { "$Tag`t run${r}: process exited early (code $($p.ExitCode)) - 0 = measurement failure, rerun"; continue }
        $row = [pscustomobject]@{
            shown = $shown; cpu = [math]::Round($p.TotalProcessorTime.TotalMilliseconds)
            priv = [math]::Round($p.PrivateMemorySize64 / 1MB, 2); ws = [math]::Round($p.WorkingSet64 / 1MB, 1)
            handles = $p.HandleCount; threads = $p.Threads.Count
        }
        "{0}`t run{1}: window {2,5} ms | cpu({3}s) {4,5} ms | priv {5,6:N2} MB | ws {6,5:N1} MB | handles {7} | threads {8}" -f `
            $Tag, $r, $row.shown, $SettleSecs, $row.cpu, $row.priv, $row.ws, $row.handles, $row.threads
        $rows += $row
    }
    finally {
        if (-not $p.HasExited) {
            $p.WaitForExit(3000) | Out-Null
            if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
        }
    }
    Start-Sleep -Milliseconds 600
}
if ($rows.Count -eq 0) { "$Tag`t no measurement (all runs failed)"; exit 1 }
"{0}`t MEDIAN({1}): window {2} ms | cpu {3} ms | priv {4:N2} MB | ws {5:N1} MB | handles {6} | threads {7}" -f `
    $Tag, $rows.Count, (Median ($rows | ForEach-Object shown)), (Median ($rows | ForEach-Object cpu)), (Median ($rows | ForEach-Object priv)), `
    (Median ($rows | ForEach-Object ws)), (Median ($rows | ForEach-Object handles)), (Median ($rows | ForEach-Object threads))
