@echo off
setlocal
cd /d "%~dp0\..\Rust"
if exist "hotspot_stats_widget.exe" (
    echo Starting Rust Hotspot & Stats Widget...
    start "" "hotspot_stats_widget.exe"
) else (
    echo Binary not found in Rust folder. Checking target release...
    if exist "test_gui\target\release\hotspot_stats_widget.exe" (
        start "" "test_gui\target\release\hotspot_stats_widget.exe"
    ) else (
        echo Please build the project first using Batch\build_rust.bat
        pause
    )
)
