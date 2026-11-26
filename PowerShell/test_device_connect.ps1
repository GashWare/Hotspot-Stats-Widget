param([string]$Address = "A4403E0E51CF")

Write-Output "Testing connection methods for Bluetooth address $Address..."

# Method 1: Windows.Devices.Bluetooth.BluetoothDevice
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    [Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
    function Await($WinRtTask, $ResultType) {
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(5000) | Out-Null
        $netTask.Result
    }

    $mac = [Convert]::ToUInt64($Address, 16)
    Write-Output "Mac integer: $mac"
    $dev = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($mac)) ([Windows.Devices.Bluetooth.BluetoothDevice])
    if ($dev) {
        Write-Output "Device found: $($dev.Name) | ConnectionStatus: $($dev.ConnectionStatus)"
        # Request RFCOMM services to initiate classic connection
        $rfcomm = Await ($dev.GetRfcommServicesAsync()) ([Windows.Devices.Bluetooth.Rfcomm.RfcommDeviceServicesResult])
        Write-Output "Rfcomm Services Count: $($rfcomm.Services.Count) | Status: $($rfcomm.Error)"
        Write-Output "After RFCOMM - ConnectionStatus: $($dev.ConnectionStatus)"
    } else {
        Write-Output "Device was null from FromBluetoothAddressAsync"
    }
} catch {
    Write-Output "Classic error: $($_.Exception.Message)"
}

# Method 2: BLE Device
try {
    [Windows.Devices.Bluetooth.BluetoothLEDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
    $mac = [Convert]::ToUInt64($Address, 16)
    $ble = Await ([Windows.Devices.Bluetooth.BluetoothLEDevice]::FromBluetoothAddressAsync($mac)) ([Windows.Devices.Bluetooth.BluetoothLEDevice])
    if ($ble) {
        Write-Output "BLE Device found: $($ble.Name) | ConnectionStatus: $($ble.ConnectionStatus)"
        $gatt = Await ($ble.GetGattServicesAsync()) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult])
        Write-Output "Gatt Services Count: $($gatt.Services.Count) | Status: $($gatt.Status)"
        Write-Output "After GATT - ConnectionStatus: $($ble.ConnectionStatus)"
    }
} catch {
    Write-Output "BLE error: $($_.Exception.Message)"
}
