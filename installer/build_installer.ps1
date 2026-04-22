# ============================================================
# Build script: compiles release binary + packages installer
# Usage: .\installer\build_installer.ps1
# Prerequisites: Rust toolchain, Inno Setup 6 installed
# ============================================================

param(
    [switch]$SkipBuild,       # Skip cargo build (use existing binary)
    [switch]$SkipInstaller,   # Skip installer packaging
    [string]$InnoSetupPath    # Custom path to ISCC.exe
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Toolpath Viewer - Build & Package Script" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# ----------------------------------------------------------
# Step 1: Build release binary
# ----------------------------------------------------------
if (-not $SkipBuild) {
    Write-Host "`n[1/3] Building release binary..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed with exit code $LASTEXITCODE"
        }
        Write-Host "  Build successful." -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
}
else {
    Write-Host "`n[1/3] Skipping build (--SkipBuild)." -ForegroundColor DarkGray
}

# Verify binary exists
$BinaryPath = Join-Path $ProjectRoot "target\release\toolpath_viewer.exe"
if (-not (Test-Path $BinaryPath)) {
    throw "Release binary not found at: $BinaryPath`nRun without -SkipBuild to compile first."
}
Write-Host "  Binary: $BinaryPath" -ForegroundColor DarkGray

# ----------------------------------------------------------
# Step 2: Locate Inno Setup compiler (ISCC.exe)
# ----------------------------------------------------------
Write-Host "`n[2/3] Locating Inno Setup compiler..." -ForegroundColor Yellow

if ($InnoSetupPath -and (Test-Path $InnoSetupPath)) {
    $ISCC = $InnoSetupPath
}
else {
    # Common install locations
    $Candidates = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 5\ISCC.exe"
    )
    $ISCC = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1

    if (-not $ISCC) {
        throw @"
Inno Setup compiler (ISCC.exe) not found.
Install Inno Setup 6 from: https://jrsoftware.org/isdl.php
Or pass the path with: -InnoSetupPath "C:\path\to\ISCC.exe"
"@
    }
}
Write-Host "  ISCC: $ISCC" -ForegroundColor DarkGray

# ----------------------------------------------------------
# Step 3: Compile installer
# ----------------------------------------------------------
if (-not $SkipInstaller) {
    Write-Host "`n[3/3] Compiling installer..." -ForegroundColor Yellow

    $IssFile = Join-Path $ProjectRoot "installer\toolpath_viewer.iss"
    $OutputDir = Join-Path $ProjectRoot "installer\output"

    # Ensure output directory exists
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    }

    & $ISCC $IssFile
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
    }

    Write-Host "`n  Installer created successfully!" -ForegroundColor Green

    # Show output
    $Installer = Get-ChildItem $OutputDir -Filter "*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($Installer) {
        $SizeMB = [math]::Round($Installer.Length / 1MB, 2)
        Write-Host "  Output: $($Installer.FullName)" -ForegroundColor DarkGray
        Write-Host "  Size:   $SizeMB MB" -ForegroundColor DarkGray
    }
}
else {
    Write-Host "`n[3/3] Skipping installer (--SkipInstaller)." -ForegroundColor DarkGray
}

Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host "  Done!" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
