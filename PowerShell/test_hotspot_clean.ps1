Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function AwaitOp($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(15000) | Out-Null
    $netTask.Result
}
$asAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]
function AwaitAct($WinRtTask) {
    $tAct = $asAction.Invoke($null, @($WinRtTask))
    $tAct.Wait(5000) | Out-Null
}

$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
$opResultType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
$bType = [Windows.Networking.NetworkOperators.TetheringWiFiBand, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)

# Check if already on
if ($mgr.TetheringOperationalState.ToString() -eq 'On') {
    Write-Host "Hotspot is ON. Stopping first..."
    $opStop = $mgr.StopTetheringAsync()
    $resStop = AwaitOp $opStop $opResultType
    Write-Host "Stopped: $($resStop.Status)"
    Start-Sleep -Seconds 2
}

# Set band to Auto
$cfg = $mgr.GetCurrentAccessPointConfiguration()
Write-Host "Configuring Band = Auto..."
$cfg.Band = $bType::Auto
AwaitAct ($mgr.ConfigureAccessPointAsync($cfg))

# Also ensure SSID and Passphrase are clean
$cfg.Ssid = "Hotspot_Turbo"
$cfg.Passphrase = "turbo12345"
AwaitAct ($mgr.ConfigureAccessPointAsync($cfg))

Write-Host "Starting tethering..."
$op = $mgr.StartTetheringAsync()
$res = AwaitOp $op $opResultType
Write-Host "Start Result: Status=$($res.Status) Error=$($res.AdditionalErrorCode)"
Write-Host "OperationalState: $($mgr.TetheringOperationalState)"
