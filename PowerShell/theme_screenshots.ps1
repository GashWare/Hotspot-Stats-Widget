param([string[]]$Themes = @('matrix','ferrari_dash','corvette_red','vfd_green','hackers','synthwave','retro_amber'))
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[W]::SetProcessDPIAware() | Out-Null

$root = 'c:\Users\User\Desktop\Python\Stats Widget'
$exe  = "$root\Rust\hotspot_stats_widget.exe"
$cfgp = "$root\Rust\widget_config.json"
$out  = "$root\Rust\theme_shots"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item $cfgp "$out\widget_config.backup.json" -Force

foreach ($t in $Themes) {
    Get-Process hotspot_stats_widget -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 400
    $c = Get-Content "$out\widget_config.backup.json" -Raw | ConvertFrom-Json
    $c.theme = $t
    foreach ($k in 'net','cpu','ram','swap','disk') { $c."${k}_graph_style" = 'dash' }
    $c.show_swap = $true; $c.show_disk = $true
    $c.show_bluetooth = $false
    $c.x = 100; $c.y = 40; $c.width = 420; $c.height = 800
    $c | ConvertTo-Json -Depth 6 | Set-Content $cfgp
    Start-Process $exe -WorkingDirectory "$root\Rust"
    Start-Sleep -Seconds 4
    $bmp = New-Object System.Drawing.Bitmap 440, 830
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen(90, 30, 0, 0, $bmp.Size)
    $bmp.Save("$out\$t.png")
    $g.Dispose(); $bmp.Dispose()
}
Get-Process hotspot_stats_widget -ErrorAction SilentlyContinue | Stop-Process -Force
Copy-Item "$out\widget_config.backup.json" $cfgp -Force
Write-Host "done"
