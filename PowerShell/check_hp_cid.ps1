$hp = Get-PnpDevice | Where-Object { $_.InstanceId -like 'SWD\MMDEVAPI*' -and $_.FriendlyName -like '*Baseus*' }
foreach ($d in $hp) {
    $cid = (Get-PnpDeviceProperty -InstanceId $d.InstanceId -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction SilentlyContinue).Data
    Write-Output "Name: $($d.FriendlyName), ID: $($d.InstanceId), ContainerId: $cid"
}
