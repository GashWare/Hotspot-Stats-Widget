@echo off
taskkill /f /im hotspot_stats_widget.exe >nul 2>&1
timeout /t 1 /nobreak >nul
copy /y "C:\Users\User\Desktop\Python\Stats Widget\Rust\hotspot_stats_widget_new.exe" "C:\Users\User\Desktop\Python\Stats Widget\Rust\hotspot_stats_widget.exe" >nul
start "" "C:\Users\User\Desktop\Python\Stats Widget\Rust\hotspot_stats_widget.exe"
