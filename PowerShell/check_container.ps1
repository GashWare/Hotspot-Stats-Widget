$cid = '{E8B004A4-54E7-57EE-9BBD-8345A3D19B86}'
Get-PnpDevice | Where-Object {
    $prop = Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction SilentlyContinue
    $prop -and $prop.Data -eq $cid
} | Select-Object FriendlyName, InstanceId, Status, Class | Format-Table -AutoSize
