Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Bluetooth.BluetoothLEDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
foreach ($name in @('IBluetoothLEDevice', 'IBluetoothLEDevice2', 'IBluetoothLEDevice3', 'IBluetoothLEDevice4', 'IBluetoothLEDevice5')) {
    Write-Host "--- $name ---"
    $t = [Windows.Devices.Bluetooth.BluetoothLEDevice].GetInterface($name)
    if ($t) {
        $t.GetMethods() | Select-Object Name | Format-Table
    }
}
