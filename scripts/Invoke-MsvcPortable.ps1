$ErrorActionPreference = 'Stop'
$Command = $args[0]
$CommandArgs = if ($args.Count -gt 1) { @($args[1..($args.Count - 1)]) } else { @() }
if ([string]::IsNullOrWhiteSpace($Command)) { throw 'A command is required' }
$root = Join-Path $env:USERPROFILE 'msvc_portable'
$cl = Get-ChildItem (Join-Path $root 'installed') -Recurse -File -Filter 'cl.exe' |
    Select-Object -First 1
if (-not $cl) { throw "Portable MSVC cl.exe not found under $root\installed" }

$vcBin = $cl.Directory.FullName
$unpack = Join-Path $root 'xwin-cache\unpack'
$crtHeaders = Join-Path $unpack 'Microsoft.VC.14.44.17.14.CRT.Headers.base.vsix\include'
$ucrtHeaders = Join-Path $unpack 'ucrt.msi\include\ucrt'
$sdkHeaders = Join-Path $unpack 'Win11SDK_10.0.26100_headers.msi\include'
$sdkStoreHeaders = Join-Path $unpack 'Win11SDK_10.0.26100_store_headers.msi\include'
$sdkOneCoreHeaders = Join-Path $unpack 'Win11SDK_10.0.26100_store_headers_onecoreuap.msi\include'
$sdkUapHeaders = Join-Path $unpack 'Win11SDK_10.0.26100_uap_headers.msi\include'
$crtLibs = Join-Path $unpack 'Microsoft.VC.14.44.17.14.CRT.x64.Desktop.base.vsix\lib\x64'
$crtStoreLibs = Join-Path $unpack 'Microsoft.VC.14.44.17.14.CRT.x64.Store.base.vsix\lib\x64'
$ucrtLibs = Join-Path $unpack 'ucrt.msi\lib\ucrt\x64'
$sdkLibs = Join-Path $unpack 'Win11SDK_10.0.26100_libs_x86_64.msi\lib\um\x64'
$sdkStoreLibs = Join-Path $unpack 'Win11SDK_10.0.26100_store_libs.msi\lib\um\x64'

$required = @($vcBin, $crtHeaders, $ucrtHeaders, $sdkHeaders, $sdkOneCoreHeaders, $sdkUapHeaders, $crtLibs, $crtStoreLibs, $ucrtLibs, $sdkLibs, $sdkStoreLibs)
foreach ($path in $required) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Portable MSVC component missing: $path" }
}

$env:PATH = "$vcBin;$env:PATH"
$env:INCLUDE = @($crtHeaders, $ucrtHeaders, (Join-Path $sdkHeaders 'shared'), (Join-Path $sdkHeaders 'um'), (Join-Path $sdkHeaders 'winrt'), (Join-Path $sdkStoreHeaders 'shared'), (Join-Path $sdkStoreHeaders 'um'), (Join-Path $sdkStoreHeaders 'winrt'), (Join-Path $sdkOneCoreHeaders 'shared'), (Join-Path $sdkUapHeaders 'shared')) -join ';'
$env:LIB = @($crtLibs, $crtStoreLibs, $ucrtLibs, $sdkLibs, $sdkStoreLibs) -join ';'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = Join-Path $vcBin 'link.exe'

& $Command @CommandArgs
exit $LASTEXITCODE
