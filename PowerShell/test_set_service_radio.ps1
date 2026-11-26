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

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_FIND_RADIO_PARAMS {
    public uint dwSize;
}

public class BluetoothHelperV4 {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstRadio(ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp, out IntPtr phRadio);

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

    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
}
'@

Add-Type -TypeDefinition $code

$radioParams = New-Object BLUETOOTH_FIND_RADIO_PARAMS
$radioParams.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($radioParams)
$hRadio = [IntPtr]::Zero
$hFindRadio = [BluetoothHelperV4]::BluetoothFindFirstRadio([ref]$radioParams, [ref]$hRadio)
Write-Output "hRadio: $hRadio, hFindRadio: $hFindRadio"

$searchParams = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS_V2
$searchParams.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($searchParams)
$searchParams.fReturnRemembered = $true
$searchParams.fReturnConnected = $true
$searchParams.hRadio = $hRadio

$deviceInfo = New-Object BLUETOOTH_DEVICE_INFO_V2
$deviceInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($deviceInfo)

$AudioSinkGuid = [Guid]"0000110b-0000-1000-8000-00805f9b34fb"

$hFind = [BluetoothHelperV4]::BluetoothFindFirstDevice([ref]$searchParams, [ref]$deviceInfo)
if ($hFind -ne [IntPtr]::Zero) {
    do {
        if ($deviceInfo.szName -like "*Baseus*") {
            Write-Output "Found Baseus! Setting AudioSink service state with hRadio..."
            $res = [BluetoothHelperV4]::BluetoothSetServiceState($hRadio, [ref]$deviceInfo, [ref]$AudioSinkGuid, 1)
            Write-Output "Result with hRadio: $res"
            $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
            Write-Output "LastWin32Error: $err"
        }
    } while ([BluetoothHelperV4]::BluetoothFindNextDevice($hFind, [ref]$deviceInfo))
    [BluetoothHelperV4]::BluetoothFindDeviceClose($hFind) | Out-Null
}

if ($hRadio -ne [IntPtr]::Zero) { [BluetoothHelperV4]::CloseHandle($hRadio) | Out-Null }
if ($hFindRadio -ne [IntPtr]::Zero) { [BluetoothHelperV4]::CloseHandle($hFindRadio) | Out-Null }
