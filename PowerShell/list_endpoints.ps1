$code = @"
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
class MMDeviceEnumeratorComObject {}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceEnumerator {
    int EnumAudioEndpoints(int dataFlow, int stateMask, out IMMDeviceCollection devices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint);
    int GetDevice(string id, out IMMDevice endpoint);
    int RegisterEndpointNotificationCallback(IntPtr client);
    int UnregisterEndpointNotificationCallback(IntPtr client);
}

[Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDeviceCollection {
    int GetCount(out int count);
    int Item(int index, out IMMDevice device);
}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IMMDevice {
    int Activate(ref Guid id, int clsCtx, IntPtr activationParams, out IntPtr interfacePointer);
    int OpenPropertyStore(int stgmAccess, out IPropertyStore properties);
    int GetId(out string id);
    int GetState(out int state);
}

[Guid("886d8eeb-8cf2-4446-8d02-cdba1dbdcf99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPropertyStore {
    int GetCount(out int count);
    int GetAt(int iProp, out PropertyKey pkey);
    int GetValue(ref PropertyKey key, out PropVariant pv);
    int SetValue(ref PropertyKey key, ref PropVariant pv);
    int Commit();
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct PropertyKey {
    public Guid fmtid;
    public int pid;
}

[StructLayout(LayoutKind.Explicit)]
struct PropVariant {
    [FieldOffset(0)] public short vt;
    [FieldOffset(8)] public IntPtr pwszVal;
}

public class AudioHelper {
    public static void ListEndpoints() {
        var enumerator = (IMMDeviceEnumerator)new MMDeviceEnumeratorComObject();
        IMMDeviceCollection coll;
        // eRender = 0, DEVICE_STATEMASK_ALL = 0x0000000F
        enumerator.EnumAudioEndpoints(0, 0x0F, out coll);
        int count;
        coll.GetCount(out count);
        Console.WriteLine("Total Audio Endpoints: " + count);
        var PKEY_Device_FriendlyName = new PropertyKey { fmtid = new Guid("a45c254e-df1c-4efd-8020-67d146a850e0"), pid = 14 };
        for (int i = 0; i < count; i++) {
            IMMDevice dev;
            coll.Item(i, out dev);
            string id;
            dev.GetId(out id);
            int state;
            dev.GetState(out state);
            IPropertyStore store;
            dev.OpenPropertyStore(0, out store);
            PropVariant pv;
            store.GetValue(ref PKEY_Device_FriendlyName, out pv);
            string name = Marshal.PtrToStringUni(pv.pwszVal);
            string stateStr = state == 1 ? "Active" : state == 2 ? "Disabled" : state == 4 ? "NotPresent" : "Unplugged";
            Console.WriteLine("[" + stateStr + "] " + name + " | ID: " + id);
        }
    }
}
"@

Add-Type -TypeDefinition $code
[AudioHelper]::ListEndpoints()
