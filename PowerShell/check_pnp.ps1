Get-NetAdapter -Name "Wi-Fi", "Wi-Fi 2" | Select-Object Name, InterfaceDescription, Status, State, MediaConnectionState, MacAddress | Format-List
Get-PnpDevice -Class "Net" | Where-Object { $_.FriendlyName -like "*Intel*" -or $_.FriendlyName -like "*Realtek*" } | Select-Object FriendlyName, Status, Problem, InstanceId | Format-List
