$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_FIND_RADIO_PARAMS { public uint dwSize; }

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

public class BthServiceEnum {
    [DllImport("bthprops.cpl")]
    public static extern IntPtr BluetoothFindFirstRadio(ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp, out IntPtr phRadio);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindRadioClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServiceInout, [In, Out] Guid[] pGuidServices);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
}
"@

if (-not ([System.Management.Automation.PSTypeName]'BthServiceEnum').Type) {
    Add-Type -TypeDefinition $cdef
}

$p = New-Object BLUETOOTH_FIND_RADIO_PARAMS
$p.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_FIND_RADIO_PARAMS])
$hRadio = [IntPtr]::Zero
$hFind = [BthServiceEnum]::BluetoothFindFirstRadio([ref]$p, [ref]$hRadio)

$dev = New-Object BLUETOOTH_DEVICE_INFO
$dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_INFO])
$dev.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

$count = [uint32]20
$guids = New-Object Guid[] 20
$err = [BthServiceEnum]::BluetoothEnumerateInstalledServices($hRadio, [ref]$dev, [ref]$count, $guids)
Write-Output "Enum err: $err, Count: $count"
for ($i = 0; $i -lt $count; $i++) {
    $g = $guids[$i]
    Write-Output "Found Guid: $g"
}

[BthServiceEnum]::CloseHandle($hRadio)
[BthServiceEnum]::BluetoothFindRadioClose($hFind)
