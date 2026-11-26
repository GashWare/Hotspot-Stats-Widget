param([string]$AddressHex)

try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    [Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
    function Await($WinRtTask, $ResultType) {
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(4500) | Out-Null
        $netTask.Result
    }

    $mac = [Convert]::ToUInt64($AddressHex, 16)
    $dev = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync($mac)) ([Windows.Devices.Bluetooth.BluetoothDevice])
    Write-Output "Name: $($dev.Name), Status: $($dev.ConnectionStatus)"
    $dev.Dispose()
    Write-Output "Disposed"
} catch {
    Write-Output "Error: $($_.Exception.Message)"
}
