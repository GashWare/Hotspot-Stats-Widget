Add-Type -AssemblyName System.Runtime.WindowsRuntime
$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]

# Check GetLanIdentifiers
$lans = $netInfoType::GetLanIdentifiers()
Write-Host "Lan count: $($lans.Count)"
foreach ($l in $lans) {
    Write-Host "Lan: $($l.NetworkAdapterId)"
}

# Check GetConnectionProfiles
$profs = $netInfoType::GetConnectionProfiles()
Write-Host "Profile count: $($profs.Count)"
foreach ($p in $profs) {
    Write-Host "Profile: $($p.ProfileName) | Adapter: $($p.NetworkAdapter.NetworkAdapterId)"
}
