Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(5000) | Out-Null
    $netTask.Result
}

$devType = [Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime]
Write-Host "--- BluetoothDevice Static Methods ---"
$devType.GetMethods() | Where-Object { $_.IsStatic } | ForEach-Object { Write-Host "$($_.ReturnType.Name) $($_.Name)" }

Write-Host "--- BluetoothDevice Instance Methods ---"
$devType.GetMethods() | Where-Object { -not $_.IsStatic } | ForEach-Object { Write-Host "$($_.ReturnType.Name) $($_.Name)" }
