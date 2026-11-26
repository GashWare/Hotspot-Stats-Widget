$code = @"
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfigVB {
    [PreserveSig] int Dummy1(string p, IntPtr p2);
    [PreserveSig] int Dummy2(string p, bool b, IntPtr p2);
    [PreserveSig] int Dummy3(string p);
    [PreserveSig] int Dummy4(string p, IntPtr p2, IntPtr p3);
    [PreserveSig] int Dummy5(string p, bool b, IntPtr p2, IntPtr p3);
    [PreserveSig] int Dummy6(string p, IntPtr p2);
    [PreserveSig] int Dummy7(string p, IntPtr p2);
    [PreserveSig] int Dummy8(string p, IntPtr p2);
    [PreserveSig] int Dummy9(string p, IntPtr p2, IntPtr p3);
    [PreserveSig] int Dummy10(string p, IntPtr p2, IntPtr p3);
    [PreserveSig] int Dummy11(string id, uint role);
    [PreserveSig] int SetEndpointVisibility(string pszDeviceName, bool bVisible);
}

[ComImport, Guid("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9")]
class CPolicyConfigClientVB {}

public class TestPolicy {
    public static int Test() {
        try {
            var policy = (IPolicyConfigVB)new CPolicyConfigClientVB();
            Console.WriteLine("CPolicyConfigClientVB instantiated successfully!");
            return 0;
        } catch (Exception ex) {
            Console.WriteLine("Error: " + ex.Message);
            return 1;
        }
    }
}
"@

Add-Type -TypeDefinition $code
$res = [TestPolicy]::Test()
Write-Host "Instantiate result: $res"
