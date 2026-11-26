$code = @"
using System;
using System.Runtime.InteropServices;

public class BthTest2 {
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
    public static extern uint BluetoothGetDeviceInfo(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServices, [In, Out] Guid[] pGuidServices);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    public static void Test2(ulong addr, bool connect) {
        IntPtr hRadio = IntPtr.Zero;
        var p = new BLUETOOTH_FIND_RADIO_PARAMS();
        p.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_FIND_RADIO_PARAMS));
        IntPtr hFind = BluetoothFindFirstRadio(ref p, out hRadio);
        if (hFind != IntPtr.Zero) BluetoothFindRadioClose(hFind);

        var dev = new BLUETOOTH_DEVICE_INFO();
        dev.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_DEVICE_INFO));
        dev.Address.ullLong = addr;

        uint getErr = BluetoothGetDeviceInfo(hRadio, ref dev);
        Console.WriteLine("BluetoothGetDeviceInfo err: " + getErr + " Name: " + dev.szName + " Connected: " + dev.fConnected + " Remembered: " + dev.fRemembered);

        uint numServices = 0;
        BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, null);
        Console.WriteLine("Num services: " + numServices);

        if (numServices > 0) {
            var guids = new Guid[numServices];
            BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, guids);
            uint flags = connect ? 1u : 0u;
            for (int i = 0; i < numServices; i++) {
                uint res = BluetoothSetServiceState(IntPtr.Zero, ref dev, ref guids[i], flags);
                Console.WriteLine("SetServiceState (radio=null) for " + guids[i] + " -> " + res);
                if (res != 0) {
                    uint res2 = BluetoothSetServiceState(hRadio, ref dev, ref guids[i], flags);
                    Console.WriteLine("SetServiceState (radio=handle) for " + guids[i] + " -> " + res2);
                }
            }
        }
        if (hRadio != IntPtr.Zero) CloseHandle(hRadio);
    }
}
"@

Add-Type -TypeDefinition $code
[BthTest2]::Test2([Convert]::ToUInt64("CF3A1EFCDFB8", 16), $true)
