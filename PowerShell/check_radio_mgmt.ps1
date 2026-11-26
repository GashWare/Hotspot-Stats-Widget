Get-ItemProperty 'HKLM:\System\CurrentControlSet\Control\RadioManagement\SystemRadioState' -ErrorAction SilentlyContinue
Get-ItemProperty 'HKLM:\System\CurrentControlSet\Control\RadioManagement\Interfaces\*' -ErrorAction SilentlyContinue
Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Control Center' -ErrorAction SilentlyContinue
Get-ItemProperty 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Control Center' -ErrorAction SilentlyContinue
