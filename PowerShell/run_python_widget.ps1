$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PyDir = Join-Path $ScriptDir "..\Python"
$MainPy = Join-Path $PyDir "main.py"

if (Test-Path $MainPy) {
    Write-Host "Launching Python Hotspot & Stats Widget..." -ForegroundColor Green
    Start-Process -FilePath "python" -ArgumentList "main.py" -WorkingDirectory $PyDir
} else {
    Write-Warning "Could not find main.py in $PyDir"
}
