$code = @"
using System;
using System.Runtime.InteropServices;

namespace BtAudioTest {
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

function Set-BtService($macHex, [bool]$enable) {
    $params = New-Object BtAudioTest.BLUETOOTH_FIND_RADIO_PARAMS
    $params.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($params)
    $hRadio = [IntPtr]::Zero
    $hFind = [BtAudioTest.Native]::BluetoothFindFirstRadio([ref]$params, [ref]$hRadio)

    if ($hRadio -eq [IntPtr]::Zero) {
        Write-Output "No radio handle found"
        return $false
    }

    $dev = New-Object BtAudioTest.BLUETOOTH_DEVICE_INFO
    $dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($dev)
    $dev.Address = [Convert]::ToUInt64($macHex, 16)

    $flag = if ($enable) { 1 } else { 0 }
    $audioSink = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
    $handsfree = [Guid]::Parse("0000111e-0000-1000-8000-00805f9b34fb")
    $headset = [Guid]::Parse("00001108-0000-1000-8000-00805f9b34fb")

    $res1 = [BtAudioTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$audioSink, $flag)
    $res2 = [BtAudioTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$handsfree, $flag)
    $res3 = [BtAudioTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$headset, $flag)

    [BtAudioTest.Native]::CloseHandle($hRadio)
    [BtAudioTest.Native]::BluetoothFindRadioClose($hFind)

    Write-Output "SetServiceState (Enable=$enable): Sink=$res1, HF=$res2, Headset=$res3"
    return ($res1 -eq 0 -or $res2 -eq 0)
}

Write-Output "=== Testing Re-enable / Connect on Baseus ==="
Set-BtService "A4403E0E51CF" $true
