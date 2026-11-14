# Review packet only; release integrity and native smoke are still blocked.
$ErrorActionPreference = 'Stop'
$packageName = 'fyi-cli'
$version = '0.1.2'
# The published v1.2.0 source declares the Rust component as 0.1.2.
$url64 = 'https://github.com/edithatogo/fyi-cli/releases/download/v1.2.0/fyi-cli-windows-amd64.exe.zip'
$checksum64 = '8b4c38ead64c8efb80dfc53ff4a4c875be41169b6469fefaef96cf56211875f5'

$packageArgs = @{
  packageName = $packageName
  unzipLocation = Split-Path -Parent $MyInvocation.MyCommand.Definition
  url64 = $url64
  checksum64 = $checksum64
  checksumType64 = 'sha256'
}
Install-ChocolateyZipPackage @packageArgs
# Chocolatey creates/removes its executable shim; do not modify machine PATH.
