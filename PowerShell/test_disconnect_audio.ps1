$code = @"
using System;
using System.Runtime.InteropServices;

namespace BtTest2 {
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

$params = New-Object BtTest2.BLUETOOTH_FIND_RADIO_PARAMS
$params.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($params)
$hRadio = [IntPtr]::Zero
$hFind = [BtTest2.Native]::BluetoothFindFirstRadio([ref]$params, [ref]$hRadio)

Write-Output "hFind: $hFind | hRadio: $hRadio"

if ($hRadio -ne [IntPtr]::Zero) {
    $dev = New-Object BtTest2.BLUETOOTH_DEVICE_INFO
    $dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($dev)
    $dev.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

    # Disconnect Audio Sink and Handsfree
    $audioSink = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
    $handsfree = [Guid]::Parse("0000111e-0000-1000-8000-00805f9b34fb")

    $res1 = [BtTest2.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$audioSink, 0)
    $res2 = [BtTest2.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$handsfree, 0)

    Write-Output "Disable: AudioSink=$res1, Handsfree=$res2"

    [BtTest2.Native]::CloseHandle($hRadio)
    [BtTest2.Native]::BluetoothFindRadioClose($hFind)
}

Start-Sleep -Seconds 2
$p = Get-PnpDeviceProperty -InstanceId "BTHENUM\DEV_A4403E0E51CF\8&F66BE4B&0&BLUETOOTHDEVICE_A4403E0E51CF" -KeyName '{83DA6326-97A6-4088-9453-A1923F573B29} 15'
Write-Output "Baseus Connected: $($p.Data)"
