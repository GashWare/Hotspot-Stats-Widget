$code = @"
using System;
using System.Runtime.InteropServices;

public class BthTest3 {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode, Pack = 1)]
    public struct BLUETOOTH_DEVICE_INFO_PACK1 {
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

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct BLUETOOTH_DEVICE_INFO_DEFAULT {
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

    public static void ShowSizes() {
        Console.WriteLine("Pack1 size: " + Marshal.SizeOf(typeof(BLUETOOTH_DEVICE_INFO_PACK1)));
        Console.WriteLine("Default size: " + Marshal.SizeOf(typeof(BLUETOOTH_DEVICE_INFO_DEFAULT)));
    }
}
"@
Add-Type -TypeDefinition $code
[BthTest3]::ShowSizes()
