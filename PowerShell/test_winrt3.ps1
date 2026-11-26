Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Enumeration.DeviceInformationPairing, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceInformationCustomPairing, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null
Write-Host "--- DeviceInformationPairing ---"
[Windows.Devices.Enumeration.DeviceInformationPairing].GetMethods() | Select-Object Name, IsStatic | Sort-Object Name -Unique | Format-Table
Write-Host "--- DeviceInformationCustomPairing ---"
[Windows.Devices.Enumeration.DeviceInformationCustomPairing].GetMethods() | Select-Object Name, IsStatic | Sort-Object Name -Unique | Format-Table
