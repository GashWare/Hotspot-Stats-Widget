$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS {
    public uint dwSize;
    public int fReturnAuthenticated;
    public int fReturnRemembered;
    public int fReturnUnknown;
    public int fReturnConnected;
    public int fIssueInquiry;
    public byte cTimeoutMultiplier;
    public IntPtr hRadio;
}

[StructLayout(LayoutKind.Sequential)]
public struct SYSTEMTIME {
    public ushort wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds;
}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct BLUETOOTH_DEVICE_INFO {
    public uint dwSize;
    public ulong Address;
    public uint ulClassofDevice;
    public int fConnected;
    public int fRemembered;
    public int fAuthenticated;
    public SYSTEMTIME stLastSeen;
    public SYSTEMTIME stLastUsed;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 248)]
    public string szName;
}

[Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IPolicyConfig {
    int GetMixFormat(string deviceName, IntPtr format);
    int GetDeviceFormat(string deviceName, int isDefault, IntPtr format);
    int ResetDeviceFormat(string deviceName);
    int SetDeviceFormat(string deviceName, IntPtr endpointFormat, IntPtr mixFormat);
    int GetProcessingPeriod(string deviceName, int isDefault, IntPtr period, IntPtr minPeriod);
    int SetProcessingPeriod(string deviceName, IntPtr period);
    int GetShareMode(string deviceName, IntPtr mode);
    int SetShareMode(string deviceName, IntPtr mode);
    int GetPropertyValue(string deviceName, IntPtr key, IntPtr pv);
    int SetPropertyValue(string deviceName, IntPtr key, IntPtr pv);
    int SetDefaultEndpoint(string deviceName, int role);
    int SetEndpointVisibility(string deviceName, int isVisible);
}

[ComImport, Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
public class PolicyConfigClient { }

public class AudioSwitcher {
    public static int SetDefault(string devId) {
        try {
            var policy = (IPolicyConfig)new PolicyConfigClient();
            policy.SetDefaultEndpoint(devId, 0); // eConsole
            policy.SetDefaultEndpoint(devId, 1); // eMultimedia
            policy.SetDefaultEndpoint(devId, 2); // eCommunications
            return 0;
        } catch {
            return -1;
        }
    }
}

public class BthNativeToggle {
    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern IntPtr BluetoothFindFirstDevice(ref BLUETOOTH_DEVICE_SEARCH_PARAMS pbtsp, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern bool BluetoothFindNextDevice(IntPtr hFind, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl")]
    public static extern bool BluetoothFindDeviceClose(IntPtr hFind);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothSetServiceState(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref Guid pGuidService, uint dwServiceFlags);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothGetDeviceInfo(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi);

    [DllImport("bthprops.cpl", SetLastError = true)]
    public static extern uint BluetoothEnumerateInstalledServices(IntPtr hRadio, ref BLUETOOTH_DEVICE_INFO pbtdi, ref uint pcServiceInout, [In, Out] Guid[] pGuidServices);
}
"@

if (-not ([System.Management.Automation.PSTypeName]'BthNativeToggle').Type) {
    Add-Type -TypeDefinition $cdef -ErrorAction SilentlyContinue
}

Write-Output "Compilation successful!"
