$devices = Get-PnpDevice -Class Bluetooth | Where-Object { $_.InstanceId -like 'BTHENUM\DEV_*' -or $_.InstanceId -like 'BTHLE\DEV_*' }
foreach ($d in $devices) {
    $inst = $d.InstanceId
    $name = $d.FriendlyName
    $prop = Get-PnpDeviceProperty -InstanceId $inst -KeyName '{83DA6326-97A6-4088-9453-A1923F573B29} 15' -ErrorAction SilentlyContinue
    $conn = $false
    if ($prop -and $prop.Data -eq $true) {
        $conn = $true
    }
    Write-Host "$name | $inst | Connected: $conn | Status: $($d.Status)"
}
