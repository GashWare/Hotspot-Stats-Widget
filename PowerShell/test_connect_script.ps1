param(
    [string]$InstanceId,
    [string]$DeviceName,
    [string]$MacAddress,
    [string]$Connect = "true"
)

$isConnect = ($Connect -eq "true" -or $Connect -eq "1")

$script = @"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    `$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { `$_.Name -eq 'AsTask' -and `$_.GetParameters().Count -eq 1 -and `$_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
    function Await(`$WinRtTask, `$ResultType) {
        `$asTask = `$asTaskGeneric.MakeGenericMethod(`$ResultType)
        `$netTask = `$asTask.Invoke(`$null, @(`$WinRtTask))
        `$netTask.Wait(4000) | Out-Null
        `$netTask.Result
    }

    [Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
    [Windows.Devices.Bluetooth.BluetoothLEDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null

    `$name = '$DeviceName'
    `$addrStr = '$MacAddress'
    `$inst = '$InstanceId'
    `$connect = `$$isConnect

    Write-Output "Processing: `$name (Addr: `$addrStr, Connect: `$connect)"

    # Audio endpoint control via IPolicyConfig
    `$policyCode = @'
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
    public class Endpoints {
        public static void SetVisibility(string id, bool visible) {
            try {
                var p = (IPolicyConfigVB)new CPolicyConfigClientVB();
                p.SetEndpointVisibility(id, visible);
            } catch {}
        }
    }
'@
    Add-Type -TypeDefinition `$policyCode -ErrorAction SilentlyContinue

    # If it's an audio device, toggle its audio endpoints
    `$audioDevs = Get-PnpDevice -Class AudioEndpoint -ErrorAction SilentlyContinue | Where-Object { `$_.FriendlyName -like "*`$name*" }
    foreach (`$ad in `$audioDevs) {
        `$mmId = `$ad.InstanceId.Replace("SWD\MMDEVAPI\", "")
        [Endpoints]::SetVisibility(`$mmId, `$connect)
        Write-Output "AUDIO_ENDPOINT:`$mmId -> `$connect"
    }

    # If we have a MAC address, trigger connection / wake via WinRT
    if (`$addrStr) {
        `$mac = [Convert]::ToUInt64(`$addrStr, 16)
        if (`$connect) {
            # 1. Try BLE first if BTHLE
            if (`$inst -like 'BTHLE*') {
                try {
                    `$ble = Await ([Windows.Devices.Bluetooth.BluetoothLEDevice]::FromBluetoothAddressAsync(`$mac)) ([Windows.Devices.Bluetooth.BluetoothLEDevice])
                    if (`$ble) {
                        `$res = Await (`$ble.GetGattServicesAsync()) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult])
                        Write-Output "BLE_GATT:`$(`$res.Status)"
                    }
                } catch {
                    Write-Output "BLE_ERR:`$(`$_.Exception.Message)"
                }
            } else {
                # 2. Try Classic Bluetooth RFCOMM
                try {
                    `$classic = Await ([Windows.Devices.Bluetooth.BluetoothDevice]::FromBluetoothAddressAsync(`$mac)) ([Windows.Devices.Bluetooth.BluetoothDevice])
                    if (`$classic) {
                        `$rf = Await (`$classic.GetRfcommServicesAsync()) ([Windows.Devices.Bluetooth.Rfcomm.RfcommDeviceServicesResult])
                        Write-Output "CLASSIC_RFCOMM:Count=`$(`$rf.Services.Count)"
                    }
                } catch {
                    Write-Output "CLASSIC_ERR:`$(`$_.Exception.Message)"
                }
            }
        }
    }

    Write-Output "DONE_OK"
} catch {
    Write-Output "ERR:`$(`$_.Exception.Message)"
}
"@

$bytes = [System.Text.Encoding]::Unicode.GetBytes($script)
$b64 = [Convert]::ToBase64String($bytes)
powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand $b64
