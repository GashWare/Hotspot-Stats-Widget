Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(4000) | Out-Null
    $netTask.Result
}
$null = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::RequestAccessAsync()) ([Windows.Devices.Radios.RadioAccessStatus])
$radios = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
foreach ($r in $radios) {
    if ($r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {
        Write-Output "Current State: $($r.State)"
        # Try turning Off then On
        $op = $r.SetStateAsync([Windows.Devices.Radios.RadioState]::Off)
        $res = Await $op ([Windows.Devices.Radios.RadioAccessStatus])
        Write-Output "SetState Off result: $res"
        Start-Sleep -Seconds 1
        Write-Output "New State: $($r.State)"
        $op2 = $r.SetStateAsync([Windows.Devices.Radios.RadioState]::On)
        $res2 = Await $op2 ([Windows.Devices.Radios.RadioAccessStatus])
        Write-Output "SetState On result: $res2"
    }
}
