Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
foreach ($name in @('IBluetoothDevice', 'IBluetoothDevice2', 'IBluetoothDevice3', 'IBluetoothDevice4', 'IBluetoothDevice5')) {
    Write-Host "--- $name ---"
    $t = [Windows.Devices.Bluetooth.BluetoothDevice].GetInterface($name)
    if ($t) {
        $t.GetMethods() | Select-Object Name | Format-Table
    }
}
