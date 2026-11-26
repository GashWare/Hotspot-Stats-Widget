$currentWifiGuid = (Get-NetAdapter -Name "Wi-Fi").InterfaceGuid
$guidBytes = [System.Guid]::Parse($currentWifiGuid).ToByteArray()
Write-Host "Updating PreferredPublicInterface to $currentWifiGuid..."
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings' -Name 'PreferredPublicInterface' -Value $guidBytes -ErrorAction SilentlyContinue

# Also check restart-service icssvc or testing tethering
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
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
Write-Host "Starting tethering..."
$op = $mgr.StartTetheringAsync()
$res = AwaitOp $op $opResultType
Write-Host "Result: Status=$($res.Status), Error=$($res.AdditionalErrorCode)"
Write-Host "OperationalState: $($mgr.TetheringOperationalState)"
