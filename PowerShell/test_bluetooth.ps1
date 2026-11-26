$code = @"
using System;
using System.Runtime.InteropServices;

public class BthTest {
    [StructLayout(LayoutKind.Sequential)]
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
        public ushort stYear, stMonth, stDayOfWeek, stDay, stHour, stMinute, stSecond, stMilliseconds;
        public ushort ltYear, ltMonth, ltDayOfWeek, ltDay, ltHour, ltMinute, ltSecond, ltMilliseconds;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
        public string szName;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct BLUETOOTH_FIND_RADIO_PARAMS {
        public uint dwSize;
    }

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstRadio(ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp, out IntPtr phRadio);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindRadioClose(IntPtr hFind);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr hObject);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServices, [In, Out] Guid[] pGuidServices);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    public static uint ToggleDevice(ulong addr, bool connect) {
        var p = new BLUETOOTH_FIND_RADIO_PARAMS();
        p.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_FIND_RADIO_PARAMS));
        IntPtr hRadio = IntPtr.Zero;
        IntPtr hFind = BluetoothFindFirstRadio(ref p, out hRadio);
        if (hFind != IntPtr.Zero) {
            BluetoothFindRadioClose(hFind);
        }
        if (hRadio == IntPtr.Zero) return 9999;

        var dev = new BLUETOOTH_DEVICE_INFO();
        dev.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_DEVICE_INFO));
        dev.Address.ullLong = addr;

        uint numServices = 0;
        BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, null);

        uint lastErr = 0;
        if (numServices > 0) {
            var guids = new Guid[numServices];
            BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, guids);
            uint flags = connect ? 1u : 0u;
            for (int i = 0; i < numServices; i++) {
                var g = guids[i];
                uint res = BluetoothSetServiceState(hRadio, ref dev, ref g, flags);
                Console.WriteLine("Service " + g + " set to " + flags + " -> res=" + res);
                if (res != 0) lastErr = res;
            }
        } else {
            // Try standard Audio Sink / Handsfree GUIDs if none enumerated
            var audioSink = new Guid("0000110b-0000-1000-8000-00805f9b34fb");
            var handsfree = new Guid("0000111e-0000-1000-8000-00805f9b34fb");
            uint flags = connect ? 1u : 0u;
            BluetoothSetServiceState(hRadio, ref dev, ref audioSink, flags);
            BluetoothSetServiceState(hRadio, ref dev, ref handsfree, flags);
        }
        CloseHandle(hRadio);
        return lastErr;
    }
}
"@

Add-Type -TypeDefinition $code
$res = [BthTest]::ToggleDevice([Convert]::ToUInt64("CF3A1EFCDFB8", 16), $true)
Write-Host "Result: $res"
