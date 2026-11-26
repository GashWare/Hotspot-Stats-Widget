param([string]$AddressHex, [int]$Flag) # Flag: 0 = disable/disconnect, 1 = enable/connect

$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
    public uint dwSize;
    public bool fReturnAuthenticated;
    public bool fReturnRemembered;
    public bool fReturnUnknown;
    public bool fReturnConnected;
    public bool fIssueInquiry;
    public byte cTimeoutMultiplier;
    public IntPtr hRadio;
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
public struct BLUETOOTH_DEVICE_INFO {
    public uint dwSize;
    public ulong Address;
    public uint ulClassofDevice;
    public bool fConnected;
    public bool fRemembered;
    public bool fAuthenticated;
    public SYSTEMTIME stLastSeen;
    public SYSTEMTIME stLastUsed;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
    public string szName;
}

[StructLayout(LayoutKind.Sequential)]
public struct SYSTEMTIME {
    public ushort wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds;
}

public class BthDevToggle {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS pbtsp, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServiceInout, [In, Out] Guid[] pGuidServices);
}
"@

if (-not ([System.Management.Automation.PSTypeName]'BthDevToggle').Type) {
    Add-Type -TypeDefinition $cdef
}

$sp = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS
$sp.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_SEARCH_PARAMS])
$sp.fReturnAuthenticated = $true
$sp.fReturnRemembered = $true
$sp.fReturnUnknown = $false
$sp.fReturnConnected = $true
$sp.fIssueInquiry = $false
$sp.cTimeoutMultiplier = 0
$sp.hRadio = [IntPtr]::Zero

$di = New-Object BLUETOOTH_DEVICE_INFO
$di.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_INFO])

$hFind = [BthDevToggle]::BluetoothFindFirstDevice([ref]$sp, [ref]$di)
$successCount = 0
if ($hFind -ne [IntPtr]::Zero) {
    try {
        do {
            $addrHex = $di.Address.ToString("X12")
            if ($addrHex -eq $AddressHex) {
                $count = [uint32]20
                $guids = New-Object Guid[] 20
                $res = [BthDevToggle]::BluetoothEnumerateInstalledServices([IntPtr]::Zero, [ref]$di, [ref]$count, $guids)
                Write-Output "Enumerate: res=$res, count=$count"
                for ($i = 0; $i -lt $count; $i++) {
                    $g = $guids[$i]
                    $setRes = [BthDevToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g, [uint32]$Flag)
                    Write-Output "Service $g -> $setRes"
                    if ($setRes -eq 0) { $successCount++ }
                }
                break
            }
        } while ([BthDevToggle]::BluetoothFindNextDevice($hFind, [ref]$di))
    } finally {
        [BthDevToggle]::BluetoothFindDeviceClose($hFind)
    }
}

if ($successCount -gt 0) {
    Write-Output "STATUS:SUCCESS"
} else {
    Write-Output "STATUS:FAILED"
}
