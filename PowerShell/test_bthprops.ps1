$code = @'
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct BLUETOOTH_ADDRESS {
    public ulong ullLong;
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct BLUETOOTH_DEVICE_INFO {
    public uint dwSize;
    public BLUETOOTH_ADDRESS Address;
    public uint ulClassofDevice;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fConnected;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fRemembered;
    [MarshalAs(UnmanagedType.Bool)]
    public bool fAuthenticated;
    public System.Runtime.InteropServices.ComTypes.FILETIME stLastSeen;
    public System.Runtime.InteropServices.ComTypes.FILETIME stLastUsed;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
    public string szName;
}

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
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

public class BluetoothHelper {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS pbtsp, ref BLUETOOTH_DEVICE_INFO pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO pbtfi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtDeviceInfo, ref Guid pguidService, uint dwServiceFlags);
}
'@

Add-Type -TypeDefinition $code

$searchParams = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS
$searchParams.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($searchParams)
$searchParams.fReturnRemembered = $true
$searchParams.fReturnConnected = $true

$deviceInfo = New-Object BLUETOOTH_DEVICE_INFO
$deviceInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($deviceInfo)

$hFind = [BluetoothHelper]::BluetoothFindFirstDevice([ref]$searchParams, [ref]$deviceInfo)
if ($hFind -ne [IntPtr]::Zero) {
    do {
        Write-Output "FOUND: $($deviceInfo.szName) | Connected: $($deviceInfo.fConnected) | Remembered: $($deviceInfo.fRemembered) | Addr: $($deviceInfo.Address.ullLong.ToString('X12'))"
    } while ([BluetoothHelper]::BluetoothFindNextDevice($hFind, [ref]$deviceInfo))
    [BluetoothHelper]::BluetoothFindDeviceClose($hFind) | Out-Null
} else {
    Write-Output "No devices returned by BluetoothFindFirstDevice"
}
