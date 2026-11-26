# Check if headphones are reachable / powered on
$addrBaseus = [Convert]::ToUInt64('A4403E0E51CF', 16)
$addrMonster = [Convert]::ToUInt64('CF3A1EFCDFB8', 16)

Add-Type -AssemblyName System.Runtime.WindowsRuntime -ErrorAction SilentlyContinue
[Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(4500) | Out-Null
    $netTask.Result
}

$b = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($addrBaseus)) ([Windows.Devices.Bluetooth.BluetoothDevice])
Write-Output "Baseus: $($b.Name), Status: $($b.ConnectionStatus)"

$m = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($addrMonster)) ([Windows.Devices.Bluetooth.BluetoothDevice])
Write-Output "Monster: $($m.Name), Status: $($m.ConnectionStatus)"
