$inst = 'BTHENUM\DEV_A4403E0E51CF\8&F66BE4B&0&BLUETOOTHDEVICE_A4403E0E51CF'
Get-PnpDeviceProperty -InstanceId $inst | Select-Object KeyName, Type, Data | Format-Table -AutoSize
