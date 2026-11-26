param(
    [string]$TargetSsid = "MyHotspotNetwork",
    [string]$TargetPassphrase = "HotspotPassword123"
)

Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asActionGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]
function AwaitAction($WinRtTask) {
    $netTask = $asActionGeneric.Invoke($null, @($WinRtTask))
    $netTask.Wait(10000) | Out-Null
    $netTask.Result
}

$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

$icp = $netInfoType::GetInternetConnectionProfile()
$allProfiles = $netInfoType::GetConnectionProfiles()

$applied = $false
foreach ($p in ($allProfiles + @($icp))) {
    if (-not $p) { continue }
    try {
        $mgr = $tetherMgrType::CreateFromConnectionProfile($p)
        if ($mgr) {
            $cfg = $mgr.GetCurrentAccessPointConfiguration()
            $cfg.Ssid = $TargetSsid
            $cfg.Passphrase = $TargetPassphrase
            AwaitAction ($mgr.ConfigureAccessPointAsync($cfg))
            Write-Output "SUCCESS: Restored SSID to '$($cfg.Ssid)' with passphrase '$($cfg.Passphrase)' on profile '$($p.ProfileName)'"
            $applied = $true
            break
        }
    } catch {
        Write-Output "ERR on profile $($p.ProfileName): $($_.Exception.Message)"
    }
}

if (-not $applied) {
    Write-Output "FAILED to apply configuration."
}
