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
$profiles = $netInfoType::GetConnectionProfiles()
$wf2Guid = (Get-NetAdapter -Name "Wi-Fi 2").InterfaceGuid.Trim('{}')
$wf2Adapter = $null
foreach ($p in $profiles) {
    if ($p.NetworkAdapter -and $p.NetworkAdapter.NetworkAdapterId -eq $wf2Guid) {
        $wf2Adapter = $p.NetworkAdapter
        break
    }
}

Write-Host "Creating TetheringManager with Wi-Fi 2 adapter..."
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp, $wf2Adapter)

$cfg = $mgr.GetCurrentAccessPointConfiguration()
Write-Host "Configuring Band = TwoPointFourGigahertz..."
$cfg.Band = $bType::TwoPointFourGigahertz
AwaitAct ($mgr.ConfigureAccessPointAsync($cfg))

Write-Host "Starting tethering on Wi-Fi 2..."
$op = $mgr.StartTetheringAsync()
$res = AwaitOp $op $opResultType
Write-Host "Start Result: Status=$($res.Status) Error=$($res.AdditionalErrorCode)"
Write-Host "OperationalState: $($mgr.TetheringOperationalState)"
