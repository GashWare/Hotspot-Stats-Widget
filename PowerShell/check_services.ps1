$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
    public uint dwSize;
    public int fReturnAuthenticated;
    public int fReturnRemembered;
    public int fReturnUnknown;
    public int fReturnConnected;
    public int fIssueInquiry;
    public byte cTimeoutMultiplier;
    public IntPtr hRadio;
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
public struct BLUETOOTH_DEVICE_INFO {
    public uint dwSize;
    public ulong Address;
    public uint ulClassofDevice;
    public int fConnected;
    public int fRemembered;
    public int fAuthenticated;
    public SYSTEMTIME stLastSeen;
    public SYSTEMTIME stLastUsed;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
    public string szName;
}

[StructLayout(LayoutKind.Sequential)]
public struct SYSTEMTIME {
    public ushort wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds;
}

public class BthCheck {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS searchParams, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServiceInout, [In, Out] Guid[] pGuidServices);
}
"@

Add-Type -TypeDefinition $cdef -ErrorAction SilentlyContinue

$sp = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS
$sp.dwSize = [uint32]40
$sp.fReturnAuthenticated = 1
$sp.fReturnRemembered = 1
$sp.fReturnConnected = 1

$di = New-Object BLUETOOTH_DEVICE_INFO
$di.dwSize = [uint32]560

$hFind = [BthCheck]::BluetoothFindFirstDevice([ref]$sp, [ref]$di)
if ($hFind -ne [IntPtr]::Zero) {
    try {
        do {
            if ($di.szName -like '*Baseus*' -or $di.szName -like '*Monster*') {
                Write-Output "Device: $($di.szName) ($($di.Address.ToString('X12')))"
                $count = [uint32]20
                $guids = New-Object Guid[] 20
                $res = [BthCheck]::BluetoothEnumerateInstalledServices([IntPtr]::Zero, [ref]$di, [ref]$count, $guids)
                Write-Output "Installed services ($count):"
                for ($i = 0; $i -lt $count; $i++) {
                    Write-Output "  - $($guids[$i])"
                }
            }
        } while ([BthCheck]::BluetoothFindNextDevice($hFind, [ref]$di))
    } finally {
        $null = [BthCheck]::BluetoothFindDeviceClose($hFind)
    }
}
