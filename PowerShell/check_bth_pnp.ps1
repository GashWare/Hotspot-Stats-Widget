$devs = Get-PnpDevice | Where-Object { $_.FriendlyName -like '*Baseus*' -or $_.FriendlyName -like '*Monster*' }
$devs | Select-Object FriendlyName, InstanceId, Status, Class | Format-Table -AutoSize
