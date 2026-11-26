$netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
$methods = $netInfoType.GetMethods() | Where-Object { $_.IsStatic }
foreach ($m in $methods) {
    Write-Host "$($m.ReturnType.Name) $($m.Name)($(($m.GetParameters() | ForEach-Object { $_.ParameterType.Name }) -join ', '))"
}
