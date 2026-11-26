$signature = @"
[DllImport("bthprops.cpl", SetLastError = true)]
public static extern uint BluetoothSetServiceState(
    IntPtr hRadio,
    ref BLUETOOTH_DEVICE_INFO pbtdi,
    ref Guid pguidService,
    uint dwServiceFlags
);

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
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
    public ushort wYear;
    public ushort wMonth;
    public ushort wDayOfWeek;
    public ushort wDay;
    public ushort wHour;
    public ushort wMinute;
    public ushort wSecond;
    public ushort wMilliseconds;
}
"@

Add-Type -MemberDefinition $signature -Name "BtNative2" -Namespace "Win32"

$devInfo = New-Object Win32.BLUETOOTH_DEVICE_INFO
$devInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($devInfo)
# Baseus BH1: A4403E0E51CF -> 180595825988047
$devInfo.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

$audioSinkGuid = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
$handsfreeGuid = [Guid]::Parse("0000111e-0000-1000-8000-00805f9b34fb")

Write-Output "Calling BluetoothSetServiceState ENABLE..."
$res1 = [Win32.BtNative2]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$devInfo, [ref]$audioSinkGuid, 1)
Write-Output "Enable AudioSink Result (0 is ERROR_SUCCESS): $res1"

$res2 = [Win32.BtNative2]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$devInfo, [ref]$handsfreeGuid, 1)
Write-Output "Enable Handsfree Result: $res2"
