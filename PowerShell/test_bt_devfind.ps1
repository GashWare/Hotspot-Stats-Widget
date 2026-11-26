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

public class BthDevFind {
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

if (-not ([System.Management.Automation.PSTypeName]'BthDevFind').Type) {
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

$hFind = [BthDevFind]::BluetoothFindFirstDevice([ref]$sp, [ref]$di)
if ($hFind -ne [IntPtr]::Zero) {
    try {
        do {
            $addrHex = $di.Address.ToString("X12")
            Write-Output "Device: $($di.szName), Addr: $addrHex, Conn: $($di.fConnected)"
            if ($addrHex -eq "A4403E0E51CF") {
                $count = [uint32]20
                $guids = New-Object Guid[] 20
                $res = [BthDevFind]::BluetoothEnumerateInstalledServices([IntPtr]::Zero, [ref]$di, [ref]$count, $guids)
                Write-Output "Enumerate with hRadio=0: res=$res, count=$count"
                for ($i = 0; $i -lt $count; $i++) {
                    $g = $guids[$i]
                    Write-Output "  Service: $g"
                    # Try BluetoothSetServiceState with hRadio = IntPtr.Zero
                    $setRes = [BthDevFind]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g, 0)
                    Write-Output "  Disable result: $setRes"
                }
            }
        } while ([BthDevFind]::BluetoothFindNextDevice($hFind, [ref]$di))
    } finally {
        [BthDevFind]::BluetoothFindDeviceClose($hFind)
    }
}
