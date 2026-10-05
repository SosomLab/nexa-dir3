$ErrorActionPreference = 'Stop'

# Uninstall by the MSI ProductCode of this exact version (rendered at release time).
# Matching by display name is avoided on purpose: an old Inno Setup entry named "Nexa Dir" may still exist.
# For fileType msi, Uninstall-ChocolateyPackage runs "msiexec /x <silentArgs>" and ignores -File,
# so the ProductCode goes first in silentArgs (same shape as the Chocolatey auto-uninstaller template).
$packageArgs = @{
  packageName    = 'nexa-dir'
  fileType       = 'msi'
  silentArgs     = '@PRODUCT_CODE@ /qn /norestart'
  file           = ''
  validExitCodes = @(0, 3010, 1605, 1614, 1641)
}
Uninstall-ChocolateyPackage @packageArgs
