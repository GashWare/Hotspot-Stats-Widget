$target_state = "On"
$script = @"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    `$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { `$_.Name -eq 'AsTask' -and `$_.GetParameters().Count -eq 1 -and `$_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation``1' })[0]
    function Await(`$WinRtTask, `$ResultType) {
        `$asTask = `$asTaskGeneric.MakeGenericMethod(`$ResultType)
        `$netTask = `$asTask.Invoke(`$null, @(`$WinRtTask))
        `$netTask.Wait(-1) | Out-Null
        `$netTask.Result
    }
    `$radios = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
    foreach (`$r in `$radios) {
        if (`$r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {
            `$target = [Windows.Devices.Radios.RadioState]::$target_state
            `$op = `$r.SetStateAsync(`$target)
            `$res = Await `$op ([Windows.Devices.Radios.RadioAccessStatus])
            Write-Output "RES:`$res"
            exit
        }
    }
} catch {
    Write-Output "ERR:`$(`$_.Exception.Message)"
}
"@

$bytes = [System.Text.Encoding]::Unicode.GetBytes($script)
$b64 = [Convert]::ToBase64String($bytes)
$out = powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand $b64
Write-Host "Output from powershell: $out"
