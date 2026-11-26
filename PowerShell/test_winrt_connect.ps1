param(
    [string]$Id = "BluetoothLE#BluetoothLE3c:f0:11:05:3e:9d-13:08:ac:00:27:0b",
    [string]$Action = "connect"
)

Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.BluetoothLEDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(6000) | Out-Null
    $netTask.Result
}

Write-Output "Target Id: $Id, Action: $Action"
if ($Id -like "BluetoothLE#*") {
    $dev = Await ([Windows.Devices.Bluetooth.BluetoothLEDevice]::FromIdAsync($Id)) ([Windows.Devices.Bluetooth.BluetoothLEDevice])
    if ($dev) {
        Write-Output "BLE Device: $($dev.Name) | Initial Status: $($dev.ConnectionStatus)"
        if ($Action -eq "connect") {
            $gatt = Await ($dev.GetGattServicesAsync()) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult])
            Write-Output "Gatt status: $($gatt.Status) | Current Status: $($dev.ConnectionStatus)"
        } else {
            $dev.Dispose()
            Write-Output "Disposed BLE handle"
        }
    } else {
        Write-Output "Device not found"
    }
} else {
    $dev = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromIdAsync($Id)) ([Windows.Devices.Bluetooth.BluetoothDevice])
    if ($dev) {
        Write-Output "Classic Device: $($dev.Name) | Initial Status: $($dev.ConnectionStatus)"
        if ($Action -eq "connect") {
            $rf = Await ($dev.GetRfcommServicesAsync()) ([Windows.Devices.Bluetooth.Rfcomm.RfcommDeviceServicesResult])
            Write-Output "RFCOMM status: $($rf.Error) | Current Status: $($dev.ConnectionStatus)"
        } else {
            $dev.Dispose()
            Write-Output "Disposed Classic handle"
        }
    } else {
        Write-Output "Device not found"
    }
}
