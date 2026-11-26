$code = @"
using System;
using System.Runtime.InteropServices;

namespace Win32Bluetooth {
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

    public static class BtApi {
        [DllImport("bthprops.cpl", SetLastError = true)]
        public static extern uint BluetoothSetServiceState(
            IntPtr hRadio,
            ref BLUETOOTH_DEVICE_INFO pbtdi,
            ref Guid pguidService,
            uint dwServiceFlags
        );
    }
}
"@

Add-Type -TypeDefinition $code

$devInfo = New-Object Win32Bluetooth.BLUETOOTH_DEVICE_INFO
$devInfo.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($devInfo)
$devInfo.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

$audioSinkGuid = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
$handsfreeGuid = [Guid]::Parse("0000111e-0000-1000-8000-00805f9b34fb")

Write-Output "Calling BluetoothSetServiceState ENABLE..."
$res1 = [Win32Bluetooth.BtApi]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$devInfo, [ref]$audioSinkGuid, 1)
Write-Output "Enable AudioSink Result: $res1"
