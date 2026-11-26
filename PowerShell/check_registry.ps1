Get-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings' -ErrorAction SilentlyContinue
Get-ItemProperty -Path 'HKLM:\Software\Microsoft\WlanSvc' -ErrorAction SilentlyContinue
Get-ItemProperty -Path 'HKLM:\Software\Microsoft\PolicyManager\default\WiFi\AllowWiFiHotSpot' -ErrorAction SilentlyContinue
Get-ItemProperty -Path 'HKLM:\Software\Microsoft\PolicyManager\default\Network\AllowInternetSharing' -ErrorAction SilentlyContinue
