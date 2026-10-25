$ErrorActionPreference = 'Stop'

# Portable - unzip into the package folder; Chocolatey creates shims for nexa-dir.exe and nexa-copy.exe (no install footprint).
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

Install-ChocolateyZipPackage `
  -PackageName   'nexa-dir.portable' `
  -Url64bit      'https://github.com/SosomLab/nexa-dir3/releases/download/v@VERSION@/nexa-dir-@VERSION@-windows-x64-portable.zip' `
  -Checksum64    '@SHA_WIN_X64_PORTABLE@' -ChecksumType64 'sha256' `
  -UnzipLocation $toolsDir
