#Requires -Version 5.1
$ErrorActionPreference = "Stop"

Write-Host "Installing mc-cli..." -ForegroundColor Cyan

if (-not [System.Environment]::Is64BitOperatingSystem) {
    Write-Host "Error: Only 64-bit Windows is supported." -ForegroundColor Red
    exit 1
}

$Target  = "x86_64-pc-windows-msvc"
$BinName = "mc-cli-${Target}.exe"

# Fetch latest release tag from GitHub API
Write-Host "Fetching latest release..."
try {
    $Release = Invoke-RestMethod `
        -Uri "https://api.github.com/repos/Mosuzzzz/mc-cli/releases/latest" `
        -Headers @{ "Accept" = "application/vnd.github+json"; "User-Agent" = "mc-cli-installer" }
} catch {
    Write-Host "Error: Could not fetch the latest release tag from GitHub." -ForegroundColor Red
    exit 1
}
$Tag = $Release.tag_name
if (-not $Tag) {
    Write-Host "Error: Unexpected response from GitHub releases API." -ForegroundColor Red
    exit 1
}
Write-Host "Latest release: $Tag"

$BaseUrl        = "https://github.com/Mosuzzzz/mc-cli/releases/download/$Tag"
$TempBin        = Join-Path $env:TEMP "mc-cli-new-$([System.Diagnostics.Process]::GetCurrentProcess().Id).exe"
$TempChecksums  = Join-Path $env:TEMP "mc-cli-sha256sums.txt"

# Download binary and checksums file
Write-Host "Downloading $BinName..."
Invoke-WebRequest -Uri "$BaseUrl/$BinName"        -OutFile $TempBin       -UseBasicParsing
Invoke-WebRequest -Uri "$BaseUrl/sha256sums.txt"  -OutFile $TempChecksums -UseBasicParsing

# Extract expected hash for this binary
Write-Host "Verifying SHA-256 checksum..."
$Expected = Get-Content $TempChecksums |
    Where-Object { $_ -match "^([0-9a-f]{64})\s+$([regex]::Escape($BinName))$" } |
    ForEach-Object { $Matches[1] } |
    Select-Object -First 1

Remove-Item $TempChecksums -ErrorAction SilentlyContinue

if (-not $Expected) {
    Write-Host "Error: Could not find checksum for '$BinName' in sha256sums.txt." -ForegroundColor Red
    Remove-Item $TempBin -ErrorAction SilentlyContinue
    exit 1
}

$Actual = (Get-FileHash -Algorithm SHA256 $TempBin).Hash.ToLower()
if ($Actual -ne $Expected) {
    Write-Host "Error: SHA-256 mismatch — refusing to install." -ForegroundColor Red
    Write-Host "  Expected: $Expected"
    Write-Host "  Got:      $Actual"
    Remove-Item $TempBin -ErrorAction SilentlyContinue
    exit 1
}
Write-Host "Checksum OK." -ForegroundColor Green

$InstallDir = Join-Path $env:USERPROFILE ".cargo\bin"
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Dest = Join-Path $InstallDir "mc-cli.exe"
Move-Item -Force $TempBin $Dest

Write-Host ""
Write-Host "mc-cli $Tag installed to $Dest" -ForegroundColor Green
Write-Host "Make sure $InstallDir is in your PATH." -ForegroundColor Yellow
