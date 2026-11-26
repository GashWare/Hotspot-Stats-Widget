Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
Write-Host "Internet Connection Profile: $($icp.ProfileName)"

# Find NetworkAdapter for Wi-Fi 2
$profiles = $netInfoType::GetConnectionProfiles()
$wf2Guid = (Get-NetAdapter -Name "Wi-Fi 2").InterfaceGuid.Trim('{}')
Write-Host "Wi-Fi 2 GUID: $wf2Guid"

# Check profiles and network adapters
$wf2Adapter = $null
foreach ($p in $profiles) {
    if ($p.NetworkAdapter -and $p.NetworkAdapter.NetworkAdapterId -eq $wf2Guid) {
        $wf2Adapter = $p.NetworkAdapter
        Write-Host "Found NetworkAdapter from profile $($p.ProfileName)"
        break
    }
}

if (-not $wf2Adapter) {
    Write-Host "NetworkAdapter not found in ConnectionProfiles, checking LanId..."
}

Write-Host "Attempting CreateFromConnectionProfile with adapter: $($wf2Adapter)"
if ($wf2Adapter) {
    try {
        $mgr = $tetherMgrType::CreateFromConnectionProfile($icp, $wf2Adapter)
        Write-Host "SUCCESS! TetheringManager created for Wi-Fi 2 adapter!"
        Write-Host "Current Operational State: $($mgr.TetheringOperationalState)"
    } catch {
        Write-Host "Failed: $($_.Exception.Message)"
    }
}
