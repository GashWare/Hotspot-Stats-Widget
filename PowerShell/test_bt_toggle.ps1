Add-Type -AssemblyName System.Runtime.WindowsRuntime
[Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime] | Out-Null
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(4000) | Out-Null
    $netTask.Result
}

$access = Await ([Windows.Devices.Radios.Radio]::RequestAccessAsync()) ([Windows.Devices.Radios.RadioAccessStatus])
$radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
foreach ($r in $radios) {
    if ($r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {
        Write-Output "Current State: $($r.State)"
        $res = Await ($r.SetStateAsync([Windows.Devices.Radios.RadioState]::Off)) ([Windows.Devices.Radios.RadioAccessStatus])
        Write-Output "Set Off Result: $res"
        Start-Sleep -Seconds 1
        $radios2 = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
        $btr = ($radios2 | Where-Object { $_.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth })[0]
        Write-Output "After Off: $($btr.State)"
        # Turn back On
        $resOn = Await ($btr.SetStateAsync([Windows.Devices.Radios.RadioState]::On)) ([Windows.Devices.Radios.RadioAccessStatus])
        Write-Output "Set On Result: $resOn"
        Start-Sleep -Seconds 1
        $radios3 = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
        $btr3 = ($radios3 | Where-Object { $_.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth })[0]
        Write-Output "After On: $($btr3.State)"
    }
}
