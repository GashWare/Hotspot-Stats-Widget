$signature = @"
[DllImport("bthprops.cpl", SetLastError = true)]
public static extern uint BluetoothSetServiceState(
    IntPtr hRadio,
    ref BLUETOOTH_DEVICE_INFO pbtdi,
    ref Guid pguidService,
    uint dwServiceFlags
);

[DllImport("bthprops.cpl", SetLastError = true)]
public static extern IntPtr BluetoothFindFirstRadio(
    ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp,
    out IntPtr phRadio
);

[DllImport("bthprops.cpl", SetLastError = true)]
public static extern bool BluetoothFindRadioClose(IntPtr hFind);

[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool CloseHandle(IntPtr handle);

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_FIND_RADIO_PARAMS {
    public uint dwSize;
}

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

Add-Type -MemberDefinition $signature -Name "BtNative" -Namespace "Win32"

Write-Output "BtNative loaded successfully!"
