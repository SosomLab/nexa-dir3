$ErrorActionPreference = 'Stop'

# The portable build lives only in the package folder - Chocolatey removes the folder and the shim.
# A data folder the user created next to the executable (portable settings) is left untouched.
Write-Host 'Removing Nexa Dir (Portable) - only the package folder is cleaned up.'
