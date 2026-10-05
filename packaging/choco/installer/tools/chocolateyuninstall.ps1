$ErrorActionPreference = 'Stop'

# Uninstall by the MSI ProductCode of this exact version (rendered at release time).
# Matching by display name is avoided on purpose: an old Inno Setup entry named "Nexa Dir" may still exist.
$packageArgs = @{
  packageName    = 'nexa-dir'
  fileType       = 'msi'
  silentArgs     = '/qn /norestart'
  file           = '@PRODUCT_CODE@'
  validExitCodes = @(0, 3010, 1605)
}
Uninstall-ChocolateyPackage @packageArgs
