# Hotspot & Stats Widget Test Suite (Python & Rust)
$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BaseDir = Join-Path $ScriptDir ".."

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   HOTSPOT & STATS WIDGET VERIFICATION TEST SUITE" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# Test 1: Python syntax and compilation
Write-Host "`n[Test 1] Verifying Python code compilation..." -ForegroundColor Yellow
$PythonFiles = Get-ChildItem -Path (Join-Path $BaseDir "Python") -Filter "*.py"
foreach ($file in $PythonFiles) {
    Write-Host "  Checking $($file.Name)..." -NoNewline
    & python -m py_compile $file.FullName
    if ($LASTEXITCODE -eq 0) {
        Write-Host " OK" -ForegroundColor Green
    } else {
        Write-Error "Compilation error in $($file.FullName)"
    }
}

# Test 2: Python Adapter Selection & Traffic Inspector verification
Write-Host "`n[Test 2] Running Python Backend, Adapter Discovery & Inspector Unit Test..." -ForegroundColor Yellow
$pyTestScript = @"
import sys
sys.path.insert(0, r'$($BaseDir -replace '\\', '/')/Python')

from backend import WirelessAdapterManager, HotspotDirect
from traffic_inspector import TrafficInspector

# 1. Test Adapter Manager
adapters = WirelessAdapterManager.get_wireless_adapters()
print(f'Discovered {len(adapters)} wireless adapter(s):')
for a in adapters:
    print(f'  - {a}')
assert len(adapters) > 0, 'Should detect at least 1 wireless adapter'

# 2. Test Traffic Inspector domain matching
ti = TrafficInspector(config={'blocked_domains': ['doubleclick.net', 'track.ads.com']})
assert ti.is_domain_blocked('ad.doubleclick.net'), 'Subdomain matching failed'
assert ti.is_domain_blocked('doubleclick.net'), 'Exact domain matching failed'
assert not ti.is_domain_blocked('google.com'), 'False positive on unblocked domain'

# 3. Test IP blocking
ti.add_blocked_ip('192.168.1.100')
assert '192.168.1.100' in ti.blocked_ips, 'IP block failed'
ti.remove_blocked_ip('192.168.1.100')
assert '192.168.1.100' not in ti.blocked_ips, 'IP unblock failed'

print('Python unit tests passed successfully!')
"@

& python -c $pyTestScript
if ($LASTEXITCODE -eq 0) {
    Write-Host "  Python Backend & Inspector Tests: PASSED" -ForegroundColor Green
} else {
    Write-Error "Python Backend tests failed"
}

# Test 3: Rust Release Binary Verification
Write-Host "`n[Test 3] Verifying Rust Release Binary..." -ForegroundColor Yellow
$RustExe = Join-Path $BaseDir "Rust\hotspot_stats_widget.exe"
if (Test-Path $RustExe) {
    $item = Get-Item $RustExe
    $sizeMB = [math]::Round($item.Length / 1MB, 2)
    Write-Host "  Found Rust Executable: $($item.FullName) ($sizeMB MB)" -ForegroundColor Green
} else {
    Write-Error "Rust Executable not found at $RustExe"
}

# Test 4: Verify Folder Organization according to rules
Write-Host "`n[Test 4] Verifying File Structure Organization..." -ForegroundColor Yellow
$folders = @("Python", "Rust", "Batch", "PowerShell", "Shell")
foreach ($folder in $folders) {
    $p = Join-Path $BaseDir $folder
    if (Test-Path $p) {
        Write-Host "  Folder '$folder' exists." -ForegroundColor Green
    } else {
        Write-Error "Expected folder '$folder' missing"
    }
}

Write-Host "`n==========================================================" -ForegroundColor Cyan
Write-Host "   ALL TESTS PASSED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
