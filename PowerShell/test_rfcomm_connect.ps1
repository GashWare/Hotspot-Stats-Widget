Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.Rfcomm.RfcommDeviceServicesResult, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
[Windows.Networking.Sockets.StreamSocket, Windows.Networking.Sockets, ContentType=WindowsRuntime] | Out-Null

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}
$asTaskAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]
function AwaitAction($WinRtAction) {
    $netTask = $asTaskAction.Invoke($null, @($WinRtAction))
    $netTask.Wait(-1) | Out-Null
}

$addr = [Convert]::ToUInt64('CF3A1EFCDFB8', 16)
$dev = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($addr)) ([Windows.Devices.Bluetooth.BluetoothDevice])
Write-Host "Dev name: $($dev.Name) ConnStatus: $($dev.ConnectionStatus)"

$res = Await ($dev.GetRfcommServicesAsync()) ([Windows.Devices.Bluetooth.Rfcomm.RfcommDeviceServicesResult])
Write-Host "Services: $($res.Services.Count)"
if ($res.Services.Count -gt 0) {
    $svc = $res.Services[0]
    Write-Host "Service host: $($svc.ConnectionHostName.DisplayName) port: $($svc.ConnectionServiceName)"
    try {
        $socket = New-Object Windows.Networking.Sockets.StreamSocket
        Write-Host "Connecting socket to RFCOMM service..."
        $op = $socket.ConnectAsync($svc.ConnectionHostName, $svc.ConnectionServiceName)
        # 3 second timeout
        $task = $asTaskAction.Invoke($null, @($op))
        $task.Wait(3000) | Out-Null
        Write-Host "Socket connected! Dev status: $($dev.ConnectionStatus)"
        $socket.Dispose()
    } catch {
        Write-Host "Socket connect failed: $($_.Exception.Message)"
    }
}
