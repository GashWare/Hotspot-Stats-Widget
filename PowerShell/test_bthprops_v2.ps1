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

public class BluetoothHelperV2 {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS_V2 pbtsp, ref BLUETOOTH_DEVICE_INFO_V2 pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO_V2 pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO_V2 pbtDeviceInfo, ref Guid pguidService, uint dwServiceFlags);
}
'@

Add-Type -TypeDefinition $code

$searchParams = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS_V2
$searchParams.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($searchParams)
$searchParams.fReturnRemembered = $true
$searchParams.fReturnConnected = $true

$deviceInfo = New-Object BLUETOOTH_DEVICE_INFO_V2
$deviceInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($deviceInfo)

Write-Output "Search params size: $($searchParams.dwSize), Device info size: $($deviceInfo.dwSize)"

$hFind = [BluetoothHelperV2]::BluetoothFindFirstDevice([ref]$searchParams, [ref]$deviceInfo)
if ($hFind -ne [IntPtr]::Zero) {
    do {
        Write-Output "FOUND: $($deviceInfo.szName) | Connected: $($deviceInfo.fConnected) | Remembered: $($deviceInfo.fRemembered) | Addr: $($deviceInfo.Address.ToString('X12'))"
    } while ([BluetoothHelperV2]::BluetoothFindNextDevice($hFind, [ref]$deviceInfo))
    [BluetoothHelperV2]::BluetoothFindDeviceClose($hFind) | Out-Null
} else {
    $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
    Write-Output "No devices returned. Win32 Error: $err"
}
