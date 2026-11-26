Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(12000) | Out-Null
    $netTask.Result
}

$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
$opResultType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
Write-Host "InternetProfile: $($icp.ProfileName)"

$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
Write-Host "Current Tethering State: $($mgr.TetheringOperationalState)"

$op = $mgr.StartTetheringAsync()
$res = Await $op $opResultType
Write-Host "StartTetheringAsync Result: Status=$($res.Status), AdditionalErrorCode=$($res.AdditionalErrorCode)"
Write-Host "Tethering State After Start: $($mgr.TetheringOperationalState)"
