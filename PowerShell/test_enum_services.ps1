$code = @'
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct SYSTEMTIME {
    public ushort wYear;
    public ushort wMonth;
    public ushort wDayOfWeek;
    public ushort wDay;
    public ushort wHour;
    public ushort wMinute;
    public ushort wSecond;
    public ushort wMilliseconds;
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct BLUETOOTH_DEVICE_INFO_V2 {
    public uint dwSize;
    public ulong Address;
    public uint ulClassofDevice;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fConnected;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fRemembered;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fAuthenticated;
    public SYSTEMTIME stLastSeen;
    public SYSTEMTIME stLastUsed;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
    public string szName;
}

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS_V2 {
    public uint dwSize;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fReturnAuthenticated;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fReturnRemembered;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fReturnUnknown;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fReturnConnected;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fIssueInquiry;
    public byte cTimeoutMultiplier;
    public IntPtr hRadio;
}

public class BluetoothHelperV3 {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS_V2 pbtsp, ref BLUETOOTH_DEVICE_INFO_V2 pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO_V2 pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO_V2 pbtDeviceInfo, ref uint pcServices, [In, Out] Guid[] pGuidServices);
}
'@

Add-Type -TypeDefinition $code

$searchParams = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS_V2
$searchParams.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($searchParams)
$searchParams.fReturnRemembered = $true
$searchParams.fReturnConnected = $true

$deviceInfo = New-Object BLUETOOTH_DEVICE_INFO_V2
$deviceInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($deviceInfo)

$hFind = [BluetoothHelperV3]::BluetoothFindFirstDevice([ref]$searchParams, [ref]$deviceInfo)
if ($hFind -ne [IntPtr]::Zero) {
    do {
        [uint32]$count = 20
        $guids = New-Object Guid[] 20
        $res = [BluetoothHelperV3]::BluetoothEnumerateInstalledServices([IntPtr]::Zero, [ref]$deviceInfo, [ref]$count, $guids)
        Write-Output "Device: $($deviceInfo.szName), Res: $res, Count: $count"
        for ($i = 0; $i -lt $count; $i++) {
            Write-Output "   Service: $($guids[$i])"
        }
    } while ([BluetoothHelperV3]::BluetoothFindNextDevice($hFind, [ref]$deviceInfo))
    [BluetoothHelperV3]::BluetoothFindDeviceClose($hFind) | Out-Null
}
