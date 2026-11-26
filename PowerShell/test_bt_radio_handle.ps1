$code = @"
using System;
using System.Runtime.InteropServices;

namespace BtTest {
    [StructLayout(LayoutKind.Sequential)]
    public struct BLUETOOTH_FIND_RADIO_PARAMS {
        public uint dwSize;
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

    public static class Native {
        [DllImport("bthprops.cpl", SetLastError = true)]
        public static extern IntPtr BluetoothFindFirstRadio(
            ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp,
            out IntPtr phRadio
        );

        [DllImport("bthprops.cpl", SetLastError = true)]
        public static extern bool BluetoothFindRadioClose(IntPtr hFind);

        [DllImport("kernel32.dll", SetLastError = true)]
        public static extern bool CloseHandle(IntPtr handle);

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

$params = New-Object BtTest.BLUETOOTH_FIND_RADIO_PARAMS
$params.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($params)
$hRadio = [IntPtr]::Zero
$hFind = [BtTest.Native]::BluetoothFindFirstRadio([ref]$params, [ref]$hRadio)

Write-Output "hFind: $hFind | hRadio: $hRadio"

if ($hFind -ne [IntPtr]::Zero -and $hRadio -ne [IntPtr]::Zero) {
    # Test on Baseus Bass BH1: A4403E0E51CF
    $dev = New-Object BtTest.BLUETOOTH_DEVICE_INFO
    $dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($dev)
    $dev.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

    $audioSink = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
    $res = [BtTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$audioSink, 0) # DISABLE
    Write-Output "BluetoothSetServiceState DISABLE Result: $res (0 = Success)"

    [BtTest.Native]::CloseHandle($hRadio)
    [BtTest.Native]::BluetoothFindRadioClose($hFind)
}
