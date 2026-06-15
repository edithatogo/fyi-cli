$ErrorActionPreference = 'Stop'

$packageName = 'fyi-cli'
$version     = '0.1.0'
$url64       = "https://github.com/yourusername/fyi-cli/releases/download/v$version/fyi-cli-windows-amd64.zip"
$checksum64  = 'PLACEHOLDER_WINDOWS_AMD64_SHA256'

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
  url64         = $url64
  checksum64    = $checksum64
  checksumType64 = 'sha256'
  softwareName  = 'fyi-cli'
}

Install-ChocolateyZipPackage @packageArgs

$binPath = Join-Path $packageArgs.unzipLocation "fyi-cli.exe"
# Expose executable on system path
Install-ChocolateyPath $packageArgs.unzipLocation 'Machine'
