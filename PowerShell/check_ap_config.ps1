Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$mgr = $tetherMgrType::CreateFromConnectionProfile($icp)
$cfg = $mgr.GetCurrentAccessPointConfiguration()

Write-Host "SSID: '$($cfg.Ssid)' (Length: $($cfg.Ssid.Length))"
Write-Host "Passphrase: '$($cfg.Passphrase)' (Length: $($cfg.Passphrase.Length))"
Write-Host "Band: $($cfg.Band)"
