$ErrorActionPreference = 'Stop'

# Nexa Dir 0.22.0 and earlier shipped a per-user Inno Setup installer (AppId {7E4B1C9D-3A52-4F8E-9B70-6C2D815FA3E1}).
# The MSI uses a different registration, so an old Inno install would stay behind as a second entry.
# Remove it first; settings and sessions are kept by the Inno uninstaller.
$innoKeys = @(
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{7E4B1C9D-3A52-4F8E-9B70-6C2D815FA3E1}_is1',
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\{7E4B1C9D-3A52-4F8E-9B70-6C2D815FA3E1}_is1',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{7E4B1C9D-3A52-4F8E-9B70-6C2D815FA3E1}_is1'
)
foreach ($key in $innoKeys) {
  if (Test-Path $key) {
    $uninst = (Get-ItemProperty $key).UninstallString
    if ($uninst) {
      $exe = $uninst.Trim('"')
      if (Test-Path $exe) {
        Write-Host "Removing the previous Inno Setup installation of Nexa Dir ($exe)..."
        Start-ChocolateyProcessAsAdmin -ExeToRun $exe -Statements '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -ValidExitCodes @(0)
      }
    }
  }
}

$packageArgs = @{
  packageName    = 'nexa-dir'
  fileType       = 'msi'
  url64bit       = 'https://github.com/SosomLab/nexa-dir3/releases/download/v@VERSION@/nexa-dir-@VERSION@-windows-x64.msi'
  checksum64     = '@SHA_WIN_X64_MSI@'
  checksumType64 = 'sha256'
  silentArgs     = '/qn /norestart'
  validExitCodes = @(0, 3010)
}
Install-ChocolateyPackage @packageArgs
