$params = New-Object BtTest.BLUETOOTH_FIND_RADIO_PARAMS
$params.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($params)
$hRadio = [IntPtr]::Zero
$hFind = [BtTest.Native]::BluetoothFindFirstRadio([ref]$params, [ref]$hRadio)

if ($hFind -ne [IntPtr]::Zero -and $hRadio -ne [IntPtr]::Zero) {
    $dev = New-Object BtTest.BLUETOOTH_DEVICE_INFO
    $dev.dwSize = [System.Runtime.InteropServices.Marshal]::SizeOf($dev)
    $dev.Address = [Convert]::ToUInt64("A4403E0E51CF", 16)

    $audioSink = [Guid]::Parse("0000110b-0000-1000-8000-00805f9b34fb")
    $handsfree = [Guid]::Parse("0000111e-0000-1000-8000-00805f9b34fb")
    $headset = [Guid]::Parse("00001108-0000-1000-8000-00805f9b34fb")

    $res1 = [BtTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$audioSink, 0)
    $res2 = [BtTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$handsfree, 0)
    $res3 = [BtTest.Native]::BluetoothSetServiceState($hRadio, [ref]$dev, [ref]$headset, 0)

    Write-Output "Disable: Sink=$res1, HF=$res2, Headset=$res3"

    [BtTest.Native]::CloseHandle($hRadio)
    [BtTest.Native]::BluetoothFindRadioClose($hFind)
}

Start-Sleep -Seconds 1
$p = Get-PnpDeviceProperty -InstanceId "BTHENUM\DEV_A4403E0E51CF\8&F66BE4B&0&BLUETOOTHDEVICE_A4403E0E51CF" -KeyName '{83DA6326-97A6-4088-9453-A1923F573B29} 15'
Write-Output "Baseus Connected: $($p.Data)"
