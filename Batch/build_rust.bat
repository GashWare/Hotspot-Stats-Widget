@echo off
setlocal
cd /d "%~dp0\..\Rust\test_gui"
echo Building Rust Hotspot & Stats Widget (Release Mode)...
cargo build --release
if %errorlevel% equ 0 (
    echo Build succeeded! Copying executable to Rust directory...
    copy /y "target\release\hotspot_stats_widget.exe" "..\hotspot_stats_widget.exe"
    echo Done! Executable is ready at Rust\hotspot_stats_widget.exe
) else (
    echo Build failed with error code %errorlevel%.
)
pause
