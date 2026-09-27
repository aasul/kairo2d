[CmdletBinding()]
param(
    [string]$Project = "",
    [switch]$Release,
    [switch]$LowDisk,
    [switch]$Validate
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Game = if ($Project) { (Resolve-Path -LiteralPath $Project).Path } else { "" }

function Invoke-Cargo {
    & cargo @args
    if ($LASTEXITCODE -ne 0) { throw "cargo $($args -join ' ') failed ($LASTEXITCODE)." }
}

Push-Location $Root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo was not found. This is a source update; install Rust and the native build prerequisites first."
    }
    if ($LowDisk) {
        $env:CARGO_PROFILE_DEV_DEBUG = "0"
        $env:CARGO_PROFILE_TEST_DEBUG = "0"
        $env:CARGO_INCREMENTAL = "0"
    }
    if ($Validate) {
        Invoke-Cargo fmt --all
        Invoke-Cargo fmt --all -- --check
        Invoke-Cargo check --workspace --all-targets --all-features -j 1
        Invoke-Cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings
        Invoke-Cargo test --workspace --all-features -j 1
    }
    $ProfileArgs = @()
    if ($Release) { $ProfileArgs += "--release" }
    Invoke-Cargo build -p kairo-cli -p kairo-editor @ProfileArgs -j 1
    Write-Host "Both runtime and editor were built. Opening the 3.4 Workflow Tools editor."
    Write-Host "Use Preferences > Use automatic detection if a runtime from an old folder is still selected."
    $LaunchArgs = @("run", "-p", "kairo-editor") + $ProfileArgs
    if ($Game) { $LaunchArgs += @("--", $Game) }
    Invoke-Cargo @LaunchArgs
} finally {
    Pop-Location
}
