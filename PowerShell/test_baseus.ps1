$code = @"
using System;
using System.Runtime.InteropServices;

public class BthBaseus {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct BLUETOOTH_DEVICE_INFO {
        public uint dwSize;
        public ulong Address;
        public uint ulClassofDevice;
        public int fConnected;
        public int fRemembered;
        public int fAuthenticated;
        public ushort stYear, stMonth, stDayOfWeek, stDay, stHour, stMinute, stSecond, stMilliseconds;
        public ushort ltYear, ltMonth, ltDayOfWeek, ltDay, ltHour, ltMinute, ltSecond, ltMilliseconds;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
        public string szName;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct BLUETOOTH_FIND_RADIO_PARAMS {
        public uint dwSize;
    }

    [DllImport("BluetoothApis.dll", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstRadio(ref BLUETOOTH_FIND_RADIO_PARAMS pbtfrp, out IntPtr phRadio);

    [DllImport("BluetoothApis.dll", SetLastError = true)]
    public static extern bool BluetoothFindRadioClose(IntPtr hFind);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool CloseHandle(IntPtr hObject);

    [DllImport("BluetoothApis.dll", SetLastError = true)]
    public static extern uint BluetoothGetDeviceInfo(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("BluetoothApis.dll", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServices, [In, Out] Guid[] pGuidServices);

    [DllImport("BluetoothApis.dll", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    public static void CheckBaseus(ulong addr) {
        IntPtr hRadio = IntPtr.Zero;
        var p = new BLUETOOTH_FIND_RADIO_PARAMS();
        p.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_FIND_RADIO_PARAMS));
        IntPtr hFind = BluetoothFindFirstRadio(ref p, out hRadio);
        if (hFind != IntPtr.Zero) BluetoothFindRadioClose(hFind);

        var dev = new BLUETOOTH_DEVICE_INFO();
        dev.dwSize = (uint)Marshal.SizeOf(typeof(BLUETOOTH_DEVICE_INFO));
        dev.Address = addr;

        uint err = BluetoothGetDeviceInfo(hRadio, ref dev);
        Console.WriteLine("GetDeviceInfo err: " + err + " Name: " + dev.szName + " Connected: " + dev.fConnected + " Remembered: " + dev.fRemembered);

        uint numServices = 0;
        BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, null);
        Console.WriteLine("Num services: " + numServices);

        if (numServices > 0) {
            var guids = new Guid[numServices];
            BluetoothEnumerateInstalledServices(hRadio, ref dev, ref numServices, guids);
            for (int i = 0; i < numServices; i++) {
                Console.WriteLine("Service: " + guids[i]);
            }
        }
        if (hRadio != IntPtr.Zero) CloseHandle(hRadio);
    }
}
"@
Add-Type -TypeDefinition $code
[BthBaseus]::CheckBaseus([Convert]::ToUInt64("A4403E0E51CF", 16))
