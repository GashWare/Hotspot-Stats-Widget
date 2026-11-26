$tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
$methods = $tetherMgrType.GetMethods() | Where-Object { $_.IsStatic }
foreach ($m in $methods) {
    $paramTypes = ($m.GetParameters() | ForEach-Object { $_.ParameterType.FullName }) -join ", "
    Write-Host "$($m.ReturnType.Name) $($m.Name)($paramTypes)"
}
