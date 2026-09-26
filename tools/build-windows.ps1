[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string]$TargetDirectory = "",
    [switch]$Package,
    [switch]$LowDisk
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
function Invoke-Cargo {
    & cargo @args
    if ($LASTEXITCODE -ne 0) { throw "cargo $($args -join ' ') failed ($LASTEXITCODE)." }
}
Push-Location $Root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Install Rust and the Visual Studio C++ build tools before running this script."
    }
    if ($TargetDirectory) {
        $env:CARGO_TARGET_DIR = [System.IO.Path]::GetFullPath($TargetDirectory)
    }
    if ($LowDisk) {
        $env:CARGO_PROFILE_DEV_DEBUG = "0"
        $env:CARGO_PROFILE_TEST_DEBUG = "0"
        $env:CARGO_INCREMENTAL = "0"
    }
    & python scripts/source_audit.py
    if ($LASTEXITCODE -ne 0) { throw "Source consistency checks failed." }
    & python scripts/generate_api_docs.py --check
    if ($LASTEXITCODE -ne 0) { throw "Generated API reference is out of date." }
    # Formatting is explicit here; strict CI only checks and never edits sources.
    Invoke-Cargo fmt --all
    Invoke-Cargo fmt --all -- --check
    if (-not (Test-Path Cargo.lock)) { Invoke-Cargo generate-lockfile }
    Invoke-Cargo check --locked --workspace --all-targets --all-features --target $Target -j 1
    Invoke-Cargo clippy --locked --workspace --all-targets --all-features --target $Target -j 1 -- -D warnings
    Invoke-Cargo test --locked --workspace --all-features --target $Target -j 1
    Invoke-Cargo build --locked --release --workspace --target $Target -j 1
    $MetadataText = & cargo metadata --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw "Cannot determine Cargo target directory." }
    $Metadata = $MetadataText | ConvertFrom-Json
    $Build = Join-Path (Join-Path $Metadata.target_directory $Target) "release"
    & python scripts/smoke.py --binary (Join-Path $Build "kairo.exe")
    if ($LASTEXITCODE -ne 0) { throw "Runtime smoke tests failed." }
    & python scripts/package_smoke.py --binary (Join-Path $Build "kairo.exe")
    if ($LASTEXITCODE -ne 0) { throw "Package relocation tests failed." }
    if ($Package) {
        Invoke-Cargo install cargo-bundle-licenses --version 4.2.0 --locked
        Invoke-Cargo bundle-licenses --format json --output dependency-licenses.json
        & python scripts/desktop_release.py --build-dir $Build
        if ($LASTEXITCODE -ne 0) { throw "Desktop release packaging failed." }
        Write-Host "Review third-party license warnings and test the ZIP on a clean Windows machine."
    } else {
        Write-Host "Built editor: $(Join-Path $Build 'kairo-editor.exe')"
        Write-Host "Built runtime: $(Join-Path $Build 'kairo.exe')"
    }
} finally {
    Pop-Location
}
