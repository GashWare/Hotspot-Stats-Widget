$s = Get-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings'
foreach ($p in $s.PSObject.Properties) {
    if ($p.Value -is [byte[]]) {
        $str = [System.Text.Encoding]::Unicode.GetString($p.Value)
        $ascii = [System.Text.Encoding]::ASCII.GetString($p.Value)
        Write-Host "$($p.Name) (bytes): length $($p.Value.Length) | Unicode: '$str' | Ascii: '$ascii'"
    } else {
        Write-Host "$($p.Name): $($p.Value)"
    }
}
