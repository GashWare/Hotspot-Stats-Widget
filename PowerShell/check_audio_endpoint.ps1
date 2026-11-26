Get-PnpDevice | Where-Object { $_.FriendlyName -like '*Baseus*' } | Select-Object FriendlyName, Status, Present | Format-Table -AutoSize
