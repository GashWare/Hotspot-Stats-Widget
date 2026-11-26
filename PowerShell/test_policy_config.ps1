$typeDef = @"
using System;
using System.Runtime.InteropServices;

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
            policy.SetDefaultEndpoint(devId, 0);
            policy.SetDefaultEndpoint(devId, 1);
            policy.SetDefaultEndpoint(devId, 2);
            return 0;
        } catch {
            return -1;
        }
    }
}
"@
Add-Type -TypeDefinition $typeDef
$res = [AudioSwitcher]::SetDefault("{0.0.0.00000000}.{92edeb5b-219e-4961-9ca9-b431fb00d1b3}")
Write-Output "Result: $res"
