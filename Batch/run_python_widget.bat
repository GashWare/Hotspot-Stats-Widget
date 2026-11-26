@echo off
setlocal
cd /d "%~dp0\..\Python"
echo Starting Python Hotspot & Stats Widget...
start "" python main.py
