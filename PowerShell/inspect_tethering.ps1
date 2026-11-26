Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
if ($icp) {
    Write-Host "InternetProfile Name: $($icp.ProfileName)"
    Write-Host "InternetProfile AdapterId: $($icp.NetworkAdapter.NetworkAdapterId)"
    $cap = $tetherMgrType::GetTetheringCapabilityFromConnectionProfile($icp)
    Write-Host "TetheringCapability: $cap"
    try {
        $mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
        Write-Host "TetheringOperationalState: $($mgr.TetheringOperationalState)"
        $cfg = $mgr.GetCurrentAccessPointConfiguration()
        Write-Host "SSID: $($cfg.Ssid) Band: $($cfg.Band)"
    } catch {
        Write-Host "Error creating mgr: $($_.Exception.Message)"
    }
}

Write-Host "--- ALL ADAPTERS ---"
$adapters = Get-NetAdapter
foreach ($a in $adapters) {
    Write-Host "Adapter: $($a.Name) | Desc: $($a.InterfaceDescription) | Status: $($a.Status) | Guid: $($a.InterfaceGuid)"
}

Write-Host "--- ALL PROFILES ---"
$profiles = $netInfoType::GetConnectionProfiles()
foreach ($p in $profiles) {
    $cap = $tetherMgrType::GetTetheringCapabilityFromConnectionProfile($p)
    Write-Host "Profile: $($p.ProfileName) | Cap: $cap | AdapterId: $($p.NetworkAdapter.NetworkAdapterId)"
}
