[CmdletBinding()]
param(
    [switch]$NoBuild,
    [string]$Target = "x86_64-pc-windows-msvc"
)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if (-not $NoBuild) {
        & cargo build --release --locked --target $Target
        if ($LASTEXITCODE -ne 0) { throw "Native Windows release build failed." }
    }
    $binary = Join-Path "target" "$Target/release/battlestar_voxel.exe"
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Native Windows binary missing: $binary"
    }
    & python script/package_native.py --platform windows --target $Target --binary $binary
    if ($LASTEXITCODE -ne 0) { throw "Windows package verification failed." }
} finally {
    Pop-Location
}
