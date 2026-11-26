$script = @"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    `$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { `$_.Name -eq 'AsTask' -and `$_.GetParameters().Count -eq 1 -and `$_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
    function Await(`$WinRtTask, `$ResultType) {
        `$asTask = `$asTaskGeneric.MakeGenericMethod(`$ResultType)
        `$netTask = `$asTask.Invoke(`$null, @(`$WinRtTask))
        `$netTask.Wait(-1) | Out-Null
        `$netTask.Result
    }
    `$access = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::RequestAccessAsync()) ([Windows.Devices.Radios.RadioAccessStatus])
    `$radios = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
    foreach (`$r in `$radios) {
        if (`$r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {
            Write-Output "RADIO_STATE:`$(`$r.State)"
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
Write-Host "Radio check output: $out"
