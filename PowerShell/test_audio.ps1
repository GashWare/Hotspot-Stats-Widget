$code = @"
using System;
using System.Runtime.InteropServices;

[ComImport]
[Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumeratorComObject { }

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {
    int EnumAudioEndpoints(int dataFlow, int dwStateMask, out IntPtr ppDevices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, out IntPtr ppEndpoint);
    int GetDevice(string pwstrId, out IntPtr ppDevice);
    int RegisterEndpointNotificationCallback(IntPtr pClient);
    int UnregisterEndpointNotificationCallback(IntPtr pClient);
}

public class AudioTest {
    public static void ListAudioDevices() {
        var enumerator = (IMMDeviceEnumerator)new MMDeviceEnumeratorComObject();
        IntPtr pCollection;
        // DEVICE_STATE_ACTIVE = 1, DEVICE_STATE_DISABLED = 2, DEVICE_STATE_NOTPRESENT = 4, DEVICE_STATE_UNPLUGGED = 8, ALL = 0xF
        int hr = enumerator.EnumAudioEndpoints(0, 0xF, out pCollection);
        Console.WriteLine("EnumAudioEndpoints hr=" + hr);
    }
}
"@
Add-Type -TypeDefinition $code
[AudioTest]::ListAudioDevices()
