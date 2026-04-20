# mc-cli installer for Windows

Write-Host "Installing mc-cli..." -ForegroundColor Cyan

if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: cargo (Rust) is not installed." -ForegroundColor Red
    Write-Host "Please install Rust first: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

cargo install --git https://github.com/Mosuzzzz/mc-cli.git

Write-Host "mc-cli has been installed to ~/.cargo/bin" -ForegroundColor Green
Write-Host "Please ensure ~/.cargo/bin is in your Environment PATH." -ForegroundColor Yellow
