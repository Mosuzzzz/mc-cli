# mc-cli uninstaller for PowerShell
# Removes the mc-cli binary from ~/.cargo/bin or from the path

$targetBin = Join-Path $HOME ".cargo\bin\mc-cli.exe"

if (Test-Path $targetBin) {
    Write-Host "Removing mc-cli from $targetBin..."
    Remove-Item $targetBin -Force
    if ($?) {
        Write-Host "mc-cli uninstalled successfully." -ForegroundColor Green
    } else {
        Write-Host "Failed to remove mc-cli." -ForegroundColor Red
    }
} else {
    # Try finding it in path
    $cmdPath = Get-Command mc-cli -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if ($cmdPath) {
        Write-Host "Found mc-cli at $cmdPath. Removing..."
        Remove-Item $cmdPath -Force
        if ($?) {
            Write-Host "mc-cli uninstalled successfully." -ForegroundColor Green
        } else {
            Write-Host "Failed to remove $cmdPath. You might need to run as Administrator." -ForegroundColor Red
        }
    } else {
        Write-Host "mc-cli not found in ~/.cargo/bin or your PATH." -ForegroundColor Yellow
    }
}
