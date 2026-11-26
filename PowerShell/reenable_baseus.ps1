$code = @"
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

public class BthDevToggle3 {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS pbtsp, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);
}
"@
if (-not ([System.Management.Automation.PSTypeName]'BthDevToggle3').Type) {
    Add-Type -TypeDefinition $code
}

$sp = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS
$sp.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_SEARCH_PARAMS])
$sp.fReturnAuthenticated = $true
$sp.fReturnRemembered = $true
$sp.fReturnUnknown = $false
$sp.fReturnConnected = $false
$sp.fIssueInquiry = $false
$sp.cTimeoutMultiplier = 0
$sp.hRadio = [IntPtr]::Zero

$di = New-Object BLUETOOTH_DEVICE_INFO
$di.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_INFO])

$hFind = [BthDevToggle3]::BluetoothFindFirstDevice([ref]$sp, [ref]$di)
if ($hFind -ne [IntPtr]::Zero) {
    try {
        do {
            $addrHex = $di.Address.ToString("X12")
            if ($addrHex -eq "A4403E0E51CF") {
                $g1 = [Guid]"00001101-0000-1000-8000-00805F9B34FB"
                $g2 = [Guid]"0000110C-0000-1000-8000-00805F9B34FB"
                $g3 = [Guid]"0000110E-0000-1000-8000-00805F9B34FB"
                $g4 = [Guid]"0000110B-0000-1000-8000-00805F9B34FB"
                $g5 = [Guid]"0000111E-0000-1000-8000-00805F9B34FB"

                $r1 = [BthDevToggle3]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g1, 1)
                $r2 = [BthDevToggle3]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g2, 1)
                $r3 = [BthDevToggle3]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g3, 1)
                $r4 = [BthDevToggle3]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g4, 1)
                $r5 = [BthDevToggle3]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$g5, 1)

                Write-Output "Results: $r1, $r2, $r3, $r4, $r5"
            }
        } while ([BthDevToggle3]::BluetoothFindNextDevice($hFind, [ref]$di))
    } finally {
        [BthDevToggle3]::BluetoothFindDeviceClose($hFind)
    }
}
