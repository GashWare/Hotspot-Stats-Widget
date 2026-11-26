param([string]$AddressHex, [string]$Action) # Action: 'disconnect' or 'connect'

$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_FIND_RADIO_PARAMS {
    public uint dwSize;
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

public class BthApi {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstRadio(ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp, out IntPtr phRadio);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindNextRadio(IntPtr hFind, out IntPtr phRadio);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindRadioClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
}
"@

if (-not ([System.Management.Automation.PSTypeName]'BthApi').Type) {
    Add-Type -TypeDefinition $cdef
}

$p = New-Object BLUETOOTH_FIND_RADIO_PARAMS
$p.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_FIND_RADIO_PARAMS])
$hRadio = [IntPtr]::Zero
$hFind = [BthApi]::BluetoothFindFirstRadio([ref]$p, [ref]$hRadio)

if ($hFind -eq [IntPtr]::Zero -or $hRadio -eq [IntPtr]::Zero) {
    Write-Output "RADIO_NOT_FOUND"
    exit 1
}

try {
    $dev = New-Object BLUETOOTH_DEVICE_INFO
    $dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf([Type][BLUETOOTH_DEVICE_INFO])
    $dev.Address = [Convert]::ToUInt64($AddressHex, 16)

    # AudioSink Service Guid: 0000110B-0000-1000-8000-00805F9B34FB
    # Handsfree Service Guid: 0000111E-0000-1000-8000-00805F9B34FB
    $AudioSink = [Guid]"0000110B-0000-1000-8000-00805F9B34FB"
    $Handsfree = [Guid]"0000111E-0000-1000-8000-00805F9B34FB"

    $flag = if ($Action -eq 'connect') { 1 } else { 0 }

    $res1 = [BthApi]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$AudioSink, $flag)
    $res2 = [BthApi]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$Handsfree, $flag)

    Write-Output "RESULT: AudioSink=$res1, Handsfree=$res2"
} finally {
    [BthApi]::CloseHandle($hRadio)
    [BthApi]::BluetoothFindRadioClose($hFind)
}
