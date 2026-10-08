# win-addrspace.ps1 — **프로세스 주소 공간 분류**(읽기 전용 · 10-08 메모리 점검 2차 · docs/25 §5): 실행 중인 nexa-dir(또는 아무 프로세스)의
#   커밋 메모리를 VirtualQueryEx로 훑어 ① 이미지(DLL) · 그중 쓰기 가능(커밋 과금) ② 파일/페이지파일 매핑 ③ 전용(스택 = 가드 페이지 묶음 /
#   그 밖)으로 나누고, 쓰기 페이지가 큰 모듈 · 큰 매핑 · 큰 전용 할당을 보여 준다. 메모리 창의 "런타임 · 라이브러리 · 미집계"가 무엇인지
#   바깥에서 확인하는 용도(앱 안 분해 행 = `procmem::runtime_breakdown` · mem.dump `rt` 줄과 대조). 입력 주입 없음 · 대상 프로세스 무변경.
#
# 사용:
#   pwsh -NoProfile -File scripts/win-addrspace.ps1 -ProcId <PID> [-Top 25]
#   예) 우클릭 전/후: 모듈 54 → 152 · 전용 11 → 41 MB(스택 0.5 → 6.8) · 쓰기 가능 이미지 0.8 → 9.2 MB(10-08 §16).
param([Parameter(Mandatory = $true)][int]$ProcId, [int]$Top = 25)
$src = @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class VQ {
  [StructLayout(LayoutKind.Sequential)]
  public struct MBI { public IntPtr BaseAddress; public IntPtr AllocationBase; public uint AllocationProtect; public IntPtr RegionSize; public uint State; public uint Protect; public uint Type; }
  [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr VirtualQueryEx(IntPtr h, IntPtr addr, out MBI mbi, IntPtr len);
  [DllImport("psapi.dll", CharSet=CharSet.Unicode)] public static extern uint GetMappedFileNameW(IntPtr h, IntPtr addr, StringBuilder name, uint size);
}
'@
Add-Type -TypeDefinition $src
$h = [VQ]::OpenProcess(0x0400 -bor 0x0010, $false, $ProcId)  # PROCESS_QUERY_INFORMATION | PROCESS_VM_READ
if ($h -eq [IntPtr]::Zero) { throw "OpenProcess failed: $ProcId" }
$addr = [IntPtr]::Zero
$mbi = New-Object VQ+MBI
$size = [System.Runtime.InteropServices.Marshal]::SizeOf([type][VQ+MBI])
$img = @{}; $imgWritable = @{}; $mapped = 0; $mappedNames = @{}; $priv = 0; $reservedPriv = 0
$allocs = @{}
$maxAddr = [Int64]0x7FFFFFFFFFFF
while ([Int64]$addr -lt $maxAddr) {
  $r = [VQ]::VirtualQueryEx($h, $addr, [ref]$mbi, [IntPtr]$size)
  if ($r -eq [IntPtr]::Zero) { break }
  $rs = [Int64]$mbi.RegionSize
  if ($mbi.State -eq 0x1000) {
    switch ($mbi.Type) {
      0x1000000 {
        $sb = New-Object System.Text.StringBuilder 1024
        [void][VQ]::GetMappedFileNameW($h, $mbi.AllocationBase, $sb, 1024)
        $n = [System.IO.Path]::GetFileName($sb.ToString())
        if (-not $img.ContainsKey($n)) { $img[$n] = 0; $imgWritable[$n] = 0 }
        $img[$n] += $rs
        # 쓰기 가능(READWRITE 0x04 · WRITECOPY 0x08 · EXECUTE_READWRITE 0x40 · EXECUTE_WRITECOPY 0x80) = 커밋 과금 대상
        if (($mbi.Protect -band 0xCC) -ne 0) { $imgWritable[$n] += $rs }
      }
      0x40000 {
        $mapped += $rs
        $sb = New-Object System.Text.StringBuilder 1024
        [void][VQ]::GetMappedFileNameW($h, $mbi.BaseAddress, $sb, 1024)
        $n = $sb.ToString(); if ($n -eq '') { $n = '<pagefile-backed>' } else { $n = [System.IO.Path]::GetFileName($n) }
        if (-not $mappedNames.ContainsKey($n)) { $mappedNames[$n] = 0 }
        $mappedNames[$n] += $rs
      }
      0x20000 {
        $priv += $rs
        $k = [Int64]$mbi.AllocationBase
        if (-not $allocs.ContainsKey($k)) { $allocs[$k] = @{ commit = 0; guard = $false } }
        $allocs[$k].commit += $rs
        if (($mbi.Protect -band 0x100) -ne 0) { $allocs[$k].guard = $true }
      }
    }
  } elseif ($mbi.State -eq 0x2000 -and $mbi.Type -eq 0x20000) {
    $reservedPriv += $rs
  }
  $addr = [IntPtr]([Int64]$mbi.BaseAddress + $rs)
}
[void][VQ]::CloseHandle($h)
$imgTotal = ($img.Values | Measure-Object -Sum).Sum
$imgW = ($imgWritable.Values | Measure-Object -Sum).Sum
$stacks = 0; $stackCount = 0
foreach ($k in $allocs.Keys) { if ($allocs[$k].guard) { $stacks += $allocs[$k].commit; $stackCount++ } }
$p = Get-Process -Id $ProcId -ErrorAction SilentlyContinue
if ($p) { "PID $ProcId $($p.ProcessName) · Private {0:N1} MB · WS {1:N1} MB · handles {2} · threads {3} · modules {4}" -f ($p.PrivateMemorySize64/1MB), ($p.WorkingSet64/1MB), $p.HandleCount, $p.Threads.Count, $p.Modules.Count }
"image committed {0:N1} MB (writable = private-charged {1:N1} MB) · images {2}" -f ($imgTotal/1MB), ($imgW/1MB), $img.Count
"mapped committed {0:N1} MB" -f ($mapped/1MB)
"private committed {0:N1} MB = stacks {1:N1} MB ({2} guard groups) + other {3:N1} MB · private reserved(uncommitted) {4:N1} MB" -f ($priv/1MB), ($stacks/1MB), $stackCount, (($priv - $stacks)/1MB), ($reservedPriv/1MB)
""
"-- top image modules by writable committed --"
$imgWritable.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First $Top | ForEach-Object { "{0,10:N0} KB  {1}" -f ($_.Value/1KB), $_.Key }
""
"-- top mapped --"
$mappedNames.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 12 | ForEach-Object { "{0,10:N0} KB  {1}" -f ($_.Value/1KB), $_.Key }
""
"-- top private allocations (by AllocationBase, committed) --"
$allocs.GetEnumerator() | Sort-Object { $_.Value.commit } -Descending | Select-Object -First $Top | ForEach-Object { "{0,10:N0} KB  base 0x{1:X12} guard={2}" -f ($_.Value.commit/1KB), $_.Key, $_.Value.guard }
if ($p) {
  ""
  "-- non-system modules --"
  $p.Modules | Where-Object { $_.FileName -notlike 'C:\Windows\*' } | ForEach-Object { "{0,8:N0} KB {1}" -f ($_.ModuleMemorySize/1KB), $_.FileName }
}
