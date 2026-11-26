$dev = Get-PnpDevice | Where-Object { $_.FriendlyName -like '*Baseus*' -or $_.FriendlyName -like '*Monster*' }
$dev | Select-Object FriendlyName, InstanceId, Status, Present | Format-Table -AutoSize
