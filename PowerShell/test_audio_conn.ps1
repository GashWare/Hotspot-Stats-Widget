Add-Type -AssemblyName System.Runtime.WindowsRuntime -ErrorAction SilentlyContinue
[Windows.Media.Audio.AudioPlaybackConnection, Windows.Media.Audio, ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceInformation, Windows.Devices.Enumeration, ContentType=WindowsRuntime] | Out-Null

$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(4500) | Out-Null
    $netTask.Result
}

$selector = [Windows.Media.Audio.AudioPlaybackConnection]::GetDeviceSelector()
Write-Output "Selector: $selector"

$devices = Await ([Windows.Devices.Enumeration.DeviceInformation]::FindAllAsync($selector)) ([Windows.Devices.Enumeration.DeviceInformationCollection])
Write-Output "Found $($devices.Count) audio playback connection devices:"
foreach ($d in $devices) {
    Write-Output " - Name: $($d.Name) | Id: $($d.Id)"
}
