[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Join-Path $env:USERPROFILE 'msvc_portable'
$launcher = Join-Path $PSScriptRoot 'Invoke-MsvcPortable.ps1'

$output = & $launcher cl.exe 2>&1
$output | Write-Output
if (-not ($output -match 'C/C\+\+ Optimizing Compiler Version')) {
    throw 'Portable MSVC verification did not report a compiler version'
}

Write-Output "Portable MSVC verification passed under $root"
