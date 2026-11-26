Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function AwaitOp($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(12000) | Out-Null
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
$cfg = $mgr.GetCurrentAccessPointConfiguration()
Write-Host "Current Band: $($cfg.Band)"

Write-Host "Setting Band to TwoPointFourGigahertz..."
$cfg.Band = $bType::TwoPointFourGigahertz
AwaitAct ($mgr.ConfigureAccessPointAsync($cfg))

Write-Host "Calling StartTetheringAsync..."
$op = $mgr.StartTetheringAsync()
$res = AwaitOp $op $opResultType
Write-Host "Result: Status=$($res.Status) AdditionalErrorCode=$($res.AdditionalErrorCode)"
Write-Host "State: $($mgr.TetheringOperationalState)"
