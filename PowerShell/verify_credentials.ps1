Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
$cfg = $mgr.GetCurrentAccessPointConfiguration()

Write-Output "VERIFIED_SSID: $($cfg.Ssid)"
Write-Output "VERIFIED_PASS: $($cfg.Passphrase)"
Write-Output "VERIFIED_BAND: $($cfg.Band)"
