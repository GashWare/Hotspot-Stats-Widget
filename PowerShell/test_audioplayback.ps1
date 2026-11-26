Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Media.Audio.AudioPlaybackConnection, Windows.Media.Audio, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceInformation, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}

$sel = [Windows.Media.Audio.AudioPlaybackConnection]::GetDeviceSelector()
Write-Host "AudioPlaybackConnection selector: $sel"
$devs = Await ([Windows.Devices.Enumeration.DeviceInformation]::FindAllAsync($sel)) ([Windows.Devices.Enumeration.DeviceInformationCollection])
Write-Host "Found $($devs.Count) audio playback connection devices:"
foreach ($d in $devs) {
    Write-Host "Name: $($d.Name) Id: $($d.Id)"
}
