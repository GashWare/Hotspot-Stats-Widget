Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
Write-Host "State: $($mgr.TetheringOperationalState)"
for ($i = 1; $i -le 6; $i++) {
    Start-Sleep -Seconds 2
    Write-Host "T+$(2*$i)s State: $($mgr.TetheringOperationalState)"
}
