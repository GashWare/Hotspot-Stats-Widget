$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RustExe = Join-Path $ScriptDir "..\Rust\hotspot_stats_widget.exe"
if (-not (Test-Path $RustExe)) {
    $RustExe = Join-Path $ScriptDir "..\Rust\test_gui\target\release\hotspot_stats_widget.exe"
}

if (Test-Path $RustExe) {
    Write-Host "Launching Native Rust Hotspot & Stats Widget..." -ForegroundColor Cyan
    Start-Process -FilePath $RustExe -WorkingDirectory (Split-Path -Parent $RustExe)
} else {
    Write-Warning "Could not find hotspot_stats_widget.exe. Run Batch\build_rust.bat first."
}
