Get-PnpDevice | Where-Object { $_.FriendlyName -like '*Baseus*' } | ForEach-Object {
    [PSCustomObject]@{
        FriendlyName = $_.FriendlyName
        InstanceId = $_.InstanceId
        Status = $_.Status
        Present = $_.Present
    }
} | Format-Table -AutoSize
