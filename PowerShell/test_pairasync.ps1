Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceInformationCustomPairing, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}

# Try on Monster Open Ear AC510
$addr = [Convert]::ToUInt64('CF3A1EFCDFB8', 16)
$dev = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($addr)) ([Windows.Devices.Bluetooth.BluetoothDevice])
Write-Host "Dev name: $($dev.Name) ConnStatus: $($dev.ConnectionStatus)"

try {
    Write-Host "Attempting PairAsync on paired device..."
    $pairTask = $dev.DeviceInformation.Pairing.PairAsync()
    $res = Await $pairTask ([Windows.Devices.Enumeration.DevicePairingResult])
    Write-Host "PairAsync Status: $($res.Status)"
} catch {
    Write-Host "PairAsync Err: $($_.Exception.Message)"
}
