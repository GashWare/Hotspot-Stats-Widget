$s = Get-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings'
Write-Host "--- PrivateConnectionSettings hex dump ---"
[System.BitConverter]::ToString($s.PrivateConnectionSettings)
Write-Host "--- PublicConnectionSettings hex dump ---"
[System.BitConverter]::ToString($s.PublicConnectionSettings)
