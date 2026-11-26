Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function AwaitOp($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(15000) | Out-Null
    $netTask.Result
}

$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
$opResultType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$profiles = $netInfoType::GetConnectionProfiles()
$intelGuid = (Get-NetAdapter -Name "Wi-Fi").InterfaceGuid.Trim('{}')
$intelAdapter = $null
foreach ($p in $profiles) {
    if ($p.NetworkAdapter -and $p.NetworkAdapter.NetworkAdapterId -eq $intelGuid) {
        $intelAdapter = $p.NetworkAdapter
        break
    }
}

Write-Host "--- TEST 1: Default CreateFromConnectionProfile(icp) ---"
$mgr1 = $tetherMgrType::CreateFromConnectionProfile($icp)
$op1 = $mgr1.StartTetheringAsync()
$res1 = AwaitOp $op1 $opResultType
Write-Host "Test 1 Result: Status=$($res1.Status), Error=$($res1.AdditionalErrorCode)"

Write-Host "--- TEST 2: CreateFromConnectionProfile(icp, intelAdapter) ---"
if ($intelAdapter) {
    $mgr2 = $tetherMgrType::CreateFromConnectionProfile($icp, $intelAdapter)
    $op2 = $mgr2.StartTetheringAsync()
    $res2 = AwaitOp $op2 $opResultType
    Write-Host "Test 2 Result: Status=$($res2.Status), Error=$($res2.AdditionalErrorCode)"
}
