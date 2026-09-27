$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE"
    }
}

Invoke-Checked "cargo" @("check", "--workspace", "--all-targets", "--all-features")
Invoke-Checked "cargo" @("fmt", "--all", "--", "--check")
Invoke-Checked "cargo" @("clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings")
Invoke-Checked "cargo" @("test", "--workspace", "--all-features")
Invoke-Checked "cargo" @("build", "--workspace")
Invoke-Checked "python" @("scripts/source_audit.py")
Invoke-Checked "python" @("scripts/smoke.py")
