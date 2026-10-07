# win-leak-cycle.ps1 — **메모리 누수 주기 점검**(T-176 ② · nexa-sql scripts/win-leak-cycle.ps1 이식 · docs/18 §10 규칙 = 입력 주입 없음):
#   같은 동작 묶음을 N번 되풀이시키고 주기마다(묶음이 끝난 "쉬는 상태"에서) Private · 워킹셋 · 핸들 · GDI · USER · 스레드를 찍는다.
#   주기가 늘어도 값이 계단처럼 오르지 않아야 한다. 동작은 기동 명령의 시차 실행(`@after:<ms>:<명령>` · NDIR_STARTUP_CMD)으로 건다 —
#   키·마우스를 보내지 않고 포커스도 빼앗지 않는다(격리 NDIR_HOME).
#
# 사용:
#   # 큰 폴더 ↔ 작은 폴더 이동 10회(목록 적재·해제 되풀이)
#   pwsh -NoProfile -File scripts/win-leak-cycle.ps1 -HomeDir C:\tmp\ndir-home -Cycles 10 -PeriodMs 4000 `
#        -CycleCmds "nav:C:\Windows\System32;nav:C:\Windows"
#   # 보조 창 열기 → 닫기(토글 명령: 메모리 창 · 로그 창)
#   ... -CycleCmds "view.memory;view.memory" -PeriodMs 2000
#   # 미리보기 도크로 큰 파일을 고르기 되풀이(목록 선택 명령)
#   ... -First "nav:C:\tmp\big" -CycleCmds "list.select:0;list.select:1" -PeriodMs 3000
#
# 읽는 법: 첫 1~2주기는 캐시·글리프가 채워지며 오른다(정상). 그 뒤 **마지막 절반의 기울기**(MB/주기)가 0에 가까우면 누수 없음.
#   회귀선(T-176 ①): 기울기 > 0.5 MB/주기 또는 핸들이 주기마다 늘면 조사 대상.
param(
    [string]$Exe = "",
    [Parameter(Mandatory = $true)][string]$HomeDir,
    [string]$First = "",
    [Parameter(Mandatory = $true)][string]$CycleCmds,
    [int]$Cycles = 10,
    [int]$PeriodMs = 4000,
    [int]$StartMs = 4000,
    [string]$ArgList = "",
    [string]$Tag = "leak"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root "target\release\nexa-dir.exe" }
if ($HomeDir -match '\$' -or -not (Test-Path -LiteralPath $HomeDir -PathType Container)) {
    throw "HomeDir does not exist or looks wrong: '$HomeDir' - create the sandbox folder first and check the path."
}
Add-Type @"
using System; using System.Runtime.InteropServices;
public class NxLeak { [DllImport("user32.dll")] public static extern uint GetGuiResources(IntPtr h, uint flags); }
"@
# 주기 i의 동작들을 주기의 앞 60% 안에 고르게 놓고, 나머지 40%는 쉬게 둔다(그 끝에서 잰다).
# @(…) = 동작이 하나여도 배열로(문자열이면 `$acts[0]`이 첫 '글자'가 된다).
$acts = @($CycleCmds.Split(';') | Where-Object { $_ })
$cmds = @()
if ($First) { $cmds += $First.Split(';') | Where-Object { $_ } }
for ($i = 0; $i -lt $Cycles; $i++) {
    $t0 = $StartMs + $i * $PeriodMs
    for ($k = 0; $k -lt $acts.Count; $k++) {
        $at = $t0 + [int]($PeriodMs * 0.6 * $k / [Math]::Max(1, $acts.Count))
        $cmds += "@after:${at}:$($acts[$k])"
    }
}
# 마지막 측정 뒤 스스로 끝나게(종료 코드 0) — 프로세스를 이름으로 죽이지 않는다(docs/18 §10).
$cmds += "@after:$($StartMs + $Cycles * $PeriodMs + 500):quit"
$env:NDIR_HOME = $HomeDir
$env:NDIR_STARTUP_CMD = ($cmds -join ',')
# 에이전트 셸 환경(NO_COLOR · CLAUDE_*)이 PTY 셸로 내려가지 않게 — 앱 쪽 차단(T-107)이 있지만 측정 환경도 깨끗하게.
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
if ($ArgList) { $p = Start-Process -FilePath $Exe -ArgumentList $ArgList -WorkingDirectory (Split-Path $Exe) -PassThru }
else { $p = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru }
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$rows = @()
try {
    function Snap([string]$name) {
        $p.Refresh()
        $row = [pscustomobject]@{
            at = $name; priv = [math]::Round($p.PrivateMemorySize64 / 1MB, 2); ws = [math]::Round($p.WorkingSet64 / 1MB, 1)
            handles = $p.HandleCount; gdi = [NxLeak]::GetGuiResources($p.Handle, 0); user = [NxLeak]::GetGuiResources($p.Handle, 1)
            threads = $p.Threads.Count
        }
        "{0}`t{1,-8} priv={2,7:N2}MB ws={3,6:N1}MB handles={4,4} gdi={5,3} user={6,3} threads={7,2}" -f `
            $Tag, $row.at, $row.priv, $row.ws, $row.handles, $row.gdi, $row.user, $row.threads
        $row
    }
    # 기준선: 첫 주기 직전.
    $wait = $StartMs - 300 - $sw.ElapsedMilliseconds
    if ($wait -gt 0) { Start-Sleep -Milliseconds $wait }
    $out = Snap "base"; $out | Select-Object -First 1; $rows += , ($out | Select-Object -Last 1)
    for ($i = 0; $i -lt $Cycles; $i++) {
        $target = $StartMs + ($i + 1) * $PeriodMs - 300
        $wait = $target - $sw.ElapsedMilliseconds
        if ($wait -gt 0) { Start-Sleep -Milliseconds $wait }
        if ($p.HasExited) { "$Tag`t process exited (code $($p.ExitCode))"; break }
        $out = Snap ("cycle" + ($i + 1))
        $out | Select-Object -First 1
        $rows += , ($out | Select-Object -Last 1)
    }
    # 마지막 절반의 기울기(MB/주기) + 회귀선 판정.
    $half = $rows | Select-Object -Last ([Math]::Max(2, [int]($rows.Count / 2)))
    if ($half.Count -ge 2) {
        $slope = ($half[-1].priv - $half[0].priv) / ($half.Count - 1)
        $hd = $half[-1].handles - $half[0].handles
        $verdict = if ($slope -gt 0.5 -or $hd -gt ($half.Count - 1)) { "WARN" } else { "OK" }
        "{0}`tslope(last half) = {1:N3} MB/cycle | handles {2} -> {3} | gdi {4} -> {5} | user {6} -> {7} | {8}" -f `
            $Tag, $slope, $half[0].handles, $half[-1].handles, $half[0].gdi, $half[-1].gdi, $half[0].user, $half[-1].user, $verdict
    }
}
finally {
    if (-not $p.HasExited) {
        # 자체 quit 명령이 못 돌았을 때만(내가 띄운 PID만 종료).
        $p.WaitForExit(3000) | Out-Null
        if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    }
}
