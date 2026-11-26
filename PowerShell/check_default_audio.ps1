$typeDef = @"
using System;
using System.Runtime.InteropServices;

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IMMDevice {
    int Activate(ref Guid id, int clsCtx, IntPtr activationParams, [MarshalAs(UnmanagedType.IUnknown)] out object pInterface);
    int OpenPropertyStore(int stgmAccess, out IntPtr properties);
    int GetId([MarshalAs(UnmanagedType.LPWStr)] out string ppstrId);
    int GetState(out int pdwState);
}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IMMDeviceEnumerator {
    int EnumAudioEndpoints(int dataFlow, int dwStateMask, out IntPtr ppDevices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice ppEndpoint);
}

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
public class MMDeviceEnumeratorComObject { }

public class AudioHelper {
    public static string GetDefaultAudioEndpoint() {
        var enumerator = (IMMDeviceEnumerator)(new MMDeviceEnumeratorComObject());
        IMMDevice dev;
        // dataFlow = 0 (eRender), role = 0 (eConsole)
        int hr = enumerator.GetDefaultAudioEndpoint(0, 0, out dev);
        if (hr == 0 && dev != null) {
            string id;
            dev.GetId(out id);
            return id;
        }
        return null;
    }
}
"@
Add-Type -TypeDefinition $typeDef -ErrorAction SilentlyContinue
$defaultId = [AudioHelper]::GetDefaultAudioEndpoint()
Write-Output "Default Render Endpoint ID: $defaultId"
Get-PnpDevice | Where-Object { $_.InstanceId -eq $defaultId } | Select-Object FriendlyName, InstanceId
