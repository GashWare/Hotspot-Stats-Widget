Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Enumeration.DeviceInformation, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(4000) | Out-Null
    $netTask.Result
}

$devs = Await ([Windows.Devices.Enumeration.DeviceInformation]::FindAllAsync([Windows.Devices.Bluetooth.BluetoothDevice]::GetDeviceSelector())) ([Windows.Devices.Enumeration.DeviceInformationCollection])
foreach ($d in $devs) {
    if ($d.Name -like "*Baseus*" -or $d.Name -like "*K18S*") {
        Write-Output "Device: $($d.Name) | Id: $($d.Id)"
        Write-Output "Pairing CanPair: $($d.Pairing.CanPair) | IsPaired: $($d.Pairing.IsPaired)"
    }
}
