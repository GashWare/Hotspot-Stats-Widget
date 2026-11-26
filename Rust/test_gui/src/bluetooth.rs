#![allow(dead_code)]

use std::collections::HashSet;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
#[allow(unused_imports)]
use crate::logger::{log_error, log_info, log_success, log_warn};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BluetoothDeviceType {
    Audio,
    Keyboard,
    Mouse,
    Phone,
    Other,
}

impl BluetoothDeviceType {
    pub fn icon(&self) -> &'static str {
        match self {
            BluetoothDeviceType::Audio => "🎧",
            BluetoothDeviceType::Keyboard => "⌨",
            BluetoothDeviceType::Mouse => "🖱",
            BluetoothDeviceType::Phone => "📱",
            BluetoothDeviceType::Other => "📶",
        }
    }

    pub fn from_name_or_cat(name: &str, cat: &str) -> Self {
        let n = name.to_lowercase();
        let c = cat.to_lowercase();
        if n.contains("head") || n.contains("ear") || n.contains("bass") || n.contains("audio") || n.contains("sound")
            || n.contains("speaker") || n.contains("airpod") || n.contains("buds") || c.contains("audio") || c.contains("headset") {
            BluetoothDeviceType::Audio
        } else if n.contains("keyboard") || n.contains("keyborad") || n.contains("kbd") || c.contains("keyboard") {
            BluetoothDeviceType::Keyboard
        } else if n.contains("mouse") || n.contains("trackpad") || c.contains("mouse") {
            BluetoothDeviceType::Mouse
        } else if n.contains("phone") || n.contains("galaxy") || n.contains("iphone") || n.contains("pixel") {
            BluetoothDeviceType::Phone
        } else {
            BluetoothDeviceType::Other
        }
    }
}

#[derive(Clone, Debug)]
pub struct BluetoothDevice {
    pub name: String,
    pub instance_id: String,
    pub address: String,
    pub connected: bool,
    pub dev_type: BluetoothDeviceType,
    pub is_favorite: bool,
    pub is_connecting: bool,
}

pub struct BluetoothManager {
    pub is_enabled: Arc<Mutex<bool>>,
    pub is_transitioning: Arc<Mutex<bool>>,
    pub devices: Arc<Mutex<Vec<BluetoothDevice>>>,
    pub favorite_ids: Arc<Mutex<HashSet<String>>>,
    pub connecting_ids: Arc<Mutex<HashSet<String>>>,
}

impl BluetoothManager {
    pub fn new(initial_favorites: Vec<String>) -> Self {
        let is_enabled = Arc::new(Mutex::new(true));
        let is_transitioning = Arc::new(Mutex::new(false));
        let devices = Arc::new(Mutex::new(Vec::new()));
        let favorite_ids: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(initial_favorites.into_iter().collect()));
        let connecting_ids = Arc::new(Mutex::new(HashSet::new()));

        let enabled_clone = Arc::clone(&is_enabled);
        let trans_clone = Arc::clone(&is_transitioning);
        let devs_clone = Arc::clone(&devices);
        let favs_clone = Arc::clone(&favorite_ids);
        let conn_clone = Arc::clone(&connecting_ids);

        // Background polling worker (polls every 3.5s)
        thread::spawn(move || {
            let mut tick = 0usize;
            loop {
                // 1. Check Bluetooth radio adapter state
                let radio_on = Self::check_radio_state();
                *enabled_clone.lock().unwrap() = radio_on;

                // 2. Discover paired devices & their connection status
                if radio_on || tick == 0 {
                    let fav_set = favs_clone.lock().unwrap().clone();
                    let conn_set = conn_clone.lock().unwrap().clone();
                    let mut polled = Self::query_devices(&fav_set, &conn_set);

                    // Sort: Favorites first, then Connected devices, then alphabetical
                    polled.sort_by(|a, b| {
                        b.is_favorite
                            .cmp(&a.is_favorite)
                            .then_with(|| b.connected.cmp(&a.connected))
                            .then_with(|| a.name.cmp(&b.name))
                    });

                    *devs_clone.lock().unwrap() = polled;
                } else {
                    let mut lock = devs_clone.lock().unwrap();
                    for d in lock.iter_mut() {
                        d.connected = false;
                    }
                }

                *trans_clone.lock().unwrap() = false;
                tick += 1;
                thread::sleep(Duration::from_millis(3500));
            }
        });

        Self {
            is_enabled,
            is_transitioning,
            devices,
            favorite_ids,
            connecting_ids,
        }
    }

    pub fn is_on(&self) -> bool {
        *self.is_enabled.lock().unwrap()
    }

    pub fn is_transitioning(&self) -> bool {
        *self.is_transitioning.lock().unwrap()
    }

    pub fn get_devices(&self) -> Vec<BluetoothDevice> {
        self.devices.lock().unwrap().clone()
    }

    pub fn toggle_favorite(&self, instance_id: &str) -> bool {
        let mut favs = self.favorite_ids.lock().unwrap();
        let is_fav = if favs.contains(instance_id) {
            favs.remove(instance_id);
            false
        } else {
            favs.insert(instance_id.to_string());
            true
        };

        // Update in-memory devices list
        let mut devs = self.devices.lock().unwrap();
        for d in devs.iter_mut() {
            if d.instance_id == instance_id {
                d.is_favorite = is_fav;
            }
        }
        is_fav
    }

    pub fn get_favorites(&self) -> Vec<String> {
        self.favorite_ids.lock().unwrap().iter().cloned().collect()
    }

    pub fn toggle_bluetooth(&self) {
        let current = self.is_on();
        let target_on = !current;
        *self.is_transitioning.lock().unwrap() = true;

        let enabled_arc = Arc::clone(&self.is_enabled);
        let trans_arc = Arc::clone(&self.is_transitioning);
        let devs_arc = Arc::clone(&self.devices);
        let favs_arc = Arc::clone(&self.favorite_ids);
        let conn_arc = Arc::clone(&self.connecting_ids);

        thread::spawn(move || {
            log_info(
                "BLUETOOTH",
                &format!("User requested Bluetooth toggle -> {}", if target_on { "ENABLE/ON" } else { "DISABLE/OFF" }),
            );

            #[cfg(target_os = "windows")]
            {
                let success = Self::set_bluetooth_state_windows(target_on);
                if success {
                    *enabled_arc.lock().unwrap() = target_on;
                    log_success("BLUETOOTH", &format!("Bluetooth radio set to {}", if target_on { "ON" } else { "OFF" }));
                } else {
                    log_warn("BLUETOOTH", "Failed to switch Bluetooth radio state or operation timed out");
                }
            }

            #[cfg(target_os = "linux")]
            {
                let cmd_arg = if target_on { "unblock" } else { "block" };
                let res = Command::new("rfkill").args([cmd_arg, "bluetooth"]).output();
                match res {
                    Ok(o) if o.status.success() => {
                        *enabled_arc.lock().unwrap() = target_on;
                        log_success("BLUETOOTH", &format!("Linux rfkill {} bluetooth OK", cmd_arg));
                    }
                    _ => {
                        log_error("BLUETOOTH", "Failed to toggle bluetooth via rfkill");
                    }
                }
            }

            // Immediately re-poll devices so UI updates fast
            thread::sleep(Duration::from_millis(800));
            let radio_on = Self::check_radio_state();
            *enabled_arc.lock().unwrap() = radio_on;
            if radio_on {
                let fav_set = favs_arc.lock().unwrap().clone();
                let conn_set = conn_arc.lock().unwrap().clone();
                let mut polled = Self::query_devices(&fav_set, &conn_set);
                polled.sort_by(|a, b| {
                    b.is_favorite
                        .cmp(&a.is_favorite)
                        .then_with(|| b.connected.cmp(&a.connected))
                        .then_with(|| a.name.cmp(&b.name))
                });
                *devs_arc.lock().unwrap() = polled;
            } else {
                let mut lock = devs_arc.lock().unwrap();
                for d in lock.iter_mut() {
                    d.connected = false;
                }
            }

            *trans_arc.lock().unwrap() = false;
        });
    }

    pub fn open_settings() {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = Command::new("cmd")
                .args(["/c", "start", "ms-settings:bluetooth"])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("gnome-control-center")
                .arg("bluetooth")
                .spawn()
                .or_else(|_| Command::new("blueman-manager").spawn());
        }
    }

    pub fn toggle_device_connection(&self, dev: BluetoothDevice) {
        let dev_id = dev.instance_id.clone();
        let dev_name = dev.name.clone();
        let dev_addr = dev.address.clone();
        let is_connected = dev.connected;
        let target_connect = !is_connected;

        self.connecting_ids.lock().unwrap().insert(dev_id.clone());
        {
            let mut lock = self.devices.lock().unwrap();
            for d in lock.iter_mut() {
                if d.instance_id == dev_id {
                    d.is_connecting = true;
                }
            }
        }

        let conn_arc = Arc::clone(&self.connecting_ids);
        let devs_arc = Arc::clone(&self.devices);
        let favs_arc = Arc::clone(&self.favorite_ids);

        thread::spawn(move || {
            log_info(
                "BLUETOOTH",
                &format!(
                    "User requested {} for device [{}] ({})",
                    if target_connect { "CONNECT" } else { "DISCONNECT" },
                    dev_name,
                    dev_id
                ),
            );

            #[cfg(target_os = "windows")]
            {
                let ok = Self::toggle_device_windows(&dev_id, &dev_name, &dev_addr, target_connect);
                if ok {
                    log_success(
                        "BLUETOOTH",
                        &format!(
                            "Device [{}] {} successfully",
                            dev_name,
                            if target_connect { "connected" } else { "disconnected" }
                        ),
                    );
                } else {
                    log_warn(
                        "BLUETOOTH",
                        &format!(
                            "Device [{}] {} command completed",
                            dev_name,
                            if target_connect { "connect" } else { "disconnect" }
                        ),
                    );
                }
            }

            #[cfg(target_os = "linux")]
            {
                let cmd_action = if target_connect { "connect" } else { "disconnect" };
                let addr = if !dev_addr.is_empty() { &dev_addr } else { &dev_id };
                let _ = Command::new("bluetoothctl").args([cmd_action, addr]).output();
            }

            // Immediately re-poll devices to reflect real state
            thread::sleep(Duration::from_millis(1200));
            let fav_set = favs_arc.lock().unwrap().clone();
            let mut polled = Self::query_devices(&fav_set, &conn_arc.lock().unwrap());
            polled.sort_by(|a, b| {
                b.is_favorite
                    .cmp(&a.is_favorite)
                    .then_with(|| b.connected.cmp(&a.connected))
                    .then_with(|| a.name.cmp(&b.name))
            });

            conn_arc.lock().unwrap().remove(&dev_id);
            for d in polled.iter_mut() {
                if d.instance_id == dev_id {
                    d.is_connecting = false;
                }
            }
            *devs_arc.lock().unwrap() = polled;
        });
    }

    #[cfg(target_os = "windows")]
    fn check_radio_state() -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let script = r#"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime -ErrorAction SilentlyContinue
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') })[0]
    function Await($WinRtTask, $ResultType) {
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(3000) | Out-Null
        $netTask.Result
    }
    $null = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::RequestAccessAsync()) ([Windows.Devices.Radios.RadioAccessStatus])
    $radios = Await ([Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
    foreach ($r in $radios) {
        if ($r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {
            Write-Output "RADIO_STATE:$($r.State)"
            exit
        }
    }
} catch {}
# Fallback: check adapter device status
$dev = Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -like 'USB\*' } | Select-Object -First 1
if ($dev -and $dev.Status -eq 'OK') {
    Write-Output "RADIO_STATE:On"
} else {
    Write-Output "RADIO_STATE:Off"
}
"#;

        let b64 = Self::encode_b64_utf16(script);
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed == "RADIO_STATE:On" {
                    return true;
                } else if trimmed == "RADIO_STATE:Off" || trimmed == "RADIO_STATE:Disabled" {
                    return false;
                }
            }
        }
        true
    }

    #[cfg(target_os = "windows")]
    fn set_bluetooth_state_windows(enable: bool) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let target_state = if enable { "On" } else { "Off" };
        let script = format!(
            r#"
try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime -ErrorAction SilentlyContinue
    [Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime] | Out-Null
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') }})[0]
    function Await($WinRtTask, $ResultType) {{
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(4000) | Out-Null
        $netTask.Result
    }}
    $null = Await ([Windows.Devices.Radios.Radio]::RequestAccessAsync()) ([Windows.Devices.Radios.RadioAccessStatus])
    $radios = Await ([Windows.Devices.Radios.Radio]::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
    foreach ($r in $radios) {{
        if ($r.Kind -eq [Windows.Devices.Radios.RadioKind]::Bluetooth) {{
            $target = [Windows.Devices.Radios.RadioState]::{target_state}
            $op = $r.SetStateAsync($target)
            $res = Await $op ([Windows.Devices.Radios.RadioAccessStatus])
            Write-Output "RES:$res"
        }}
    }}
}} catch {{
    Write-Output "ERR:$($_.Exception.Message)"
}}
Write-Output "DONE"
"#
        );

        let b64 = Self::encode_b64_utf16(&script);
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(o) = out {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            log_info("BLUETOOTH-STATE", &format!("Output: {} | Stderr: {}", stdout.trim(), stderr.trim()));
            if stdout.contains("RES:Allowed") || stdout.contains("RES:Success") || stdout.contains("DONE") {
                return true;
            }
        }
        false
    }

    #[cfg(target_os = "windows")]
    fn query_devices(favorites: &HashSet<String>, connecting_ids: &HashSet<String>) -> Vec<BluetoothDevice> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let script = r#"
$devices = Get-PnpDevice -Class Bluetooth -ErrorAction SilentlyContinue | Where-Object { $_.InstanceId -like 'BTHENUM\DEV_*' -or $_.InstanceId -like 'BTHLE\DEV_*' }
foreach ($d in $devices) {
    $inst = $d.InstanceId
    $name = $d.FriendlyName
    if (-not $name -or $name -eq 'Bluetooth Device') { continue }
    $prop = Get-PnpDeviceProperty -InstanceId $inst -KeyName '{83DA6326-97A6-4088-9453-A1923F573B29} 15' -ErrorAction SilentlyContinue
    $conn = $false
    if ($prop -and $prop.Data -eq $true -and $d.Status -eq 'OK') {
        $conn = $true
    }
    $addr = ""
    if ($inst -match 'DEV_([A-Fa-f0-9]+)') {
        $addr = $matches[1]
    }
    Write-Output "DEV|$name|$inst|$addr|$conn|$($d.Status)"
}
"#;

        let b64 = Self::encode_b64_utf16(script);
        let mut list = Vec::new();

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.trim().split('|').collect();
                if parts.len() >= 5 && parts[0] == "DEV" {
                    let name = parts[1].trim().to_string();
                    let instance_id = parts[2].trim().to_string();
                    let addr = parts[3].trim().to_string();
                    let connected = parts[4].trim().eq_ignore_ascii_case("True");

                    if name.is_empty() || name == "Bluetooth Device" {
                        continue;
                    }

                    let dev_type = BluetoothDeviceType::from_name_or_cat(&name, "");
                    let is_fav = favorites.contains(&instance_id);
                    let is_conn = connecting_ids.contains(&instance_id);

                    list.push(BluetoothDevice {
                        name,
                        instance_id,
                        address: addr,
                        connected,
                        dev_type,
                        is_favorite: is_fav,
                        is_connecting: is_conn,
                    });
                }
            }
        }

        list
    }

    #[cfg(target_os = "windows")]
    fn toggle_device_windows(instance_id: &str, device_name: &str, address: &str, connect: bool) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let connect_bool_str = if connect { "$true" } else { "$false" };
        let script = format!(
            r#"
$cdef = @"
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct BLUETOOTH_DEVICE_SEARCH_PARAMS {{
    public uint dwSize;
    public int fReturnAuthenticated;
    public int fReturnRemembered;
    public int fReturnUnknown;
    public int fReturnConnected;
    public int fIssueInquiry;
    public byte cTimeoutMultiplier;
    public IntPtr hRadio;
}}

[StructLayout(LayoutKind.Sequential)]
public struct SYSTEMTIME {{
    public ushort wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds;
}}

[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
public struct BLUETOOTH_DEVICE_INFO {{
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
}}

[Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IPolicyConfig {{
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
}}

[ComImport, Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
public class PolicyConfigClient {{ }}

public class AudioSwitcher {{
    public static int SetDefault(string devId) {{
        try {{
            var policy = (IPolicyConfig)new PolicyConfigClient();
            policy.SetDefaultEndpoint(devId, 0); // eConsole
            policy.SetDefaultEndpoint(devId, 1); // eMultimedia
            policy.SetDefaultEndpoint(devId, 2); // eCommunications
            return 0;
        }} catch {{
            return -1;
        }}
    }}
}}

public class BthNativeToggle {{
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
}}
"@

if (-not ([System.Management.Automation.PSTypeName]'BthNativeToggle').Type) {{
    Add-Type -TypeDefinition $cdef -ErrorAction SilentlyContinue
}}

try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime -ErrorAction SilentlyContinue
    [Windows.Devices.Bluetooth.BluetoothDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null
    [Windows.Devices.Bluetooth.BluetoothLEDevice, Windows.Devices.Bluetooth, ContentType=WindowsRuntime] | Out-Null

    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name.StartsWith('IAsyncOperation') }})[0]
    function Await($WinRtTask, $ResultType) {{
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(4500) | Out-Null
        $netTask.Result
    }}

    $inst = '{instance_id}'
    $name = '{device_name}'
    $addr = '{address}'.ToUpper()
    $connect = {connect_bool_str}

    # 1. Classic / Audio Bluetooth device toggle
    $handled = $false
    if ($addr -or $name) {{
        $sp = New-Object BLUETOOTH_DEVICE_SEARCH_PARAMS
        $sp.dwSize = [uint32]40
        $sp.fReturnAuthenticated = 1
        $sp.fReturnRemembered = 1
        $sp.fReturnConnected = 1

        $di = New-Object BLUETOOTH_DEVICE_INFO
        $di.dwSize = [uint32]560

        $hFind = [BthNativeToggle]::BluetoothFindFirstDevice([ref]$sp, [ref]$di)
        if ($hFind -ne [IntPtr]::Zero) {{
            try {{
                do {{
                    $foundAddr = $di.Address.ToString("X12").ToUpper()
                    if (($addr -and $foundAddr -eq $addr) -or ($name -and $di.szName -eq $name)) {{
                        $null = [BthNativeToggle]::BluetoothGetDeviceInfo([IntPtr]::Zero, [ref]$di)

                        # Audio Sink (A2DP) and Handsfree (HFP) GUIDs
                        $a2dpGuid = [Guid]"0000110b-0000-1000-8000-00805f9b34fb"
                        $hfpGuid  = [Guid]"0000111e-0000-1000-8000-00805f9b34fb"
                        $avrcpTgt = [Guid]"0000110c-0000-1000-8000-00805f9b34fb"
                        $avrcpCtl = [Guid]"0000110e-0000-1000-8000-00805f9b34fb"

                        if ($connect) {{
                            # CRITICAL: Always ensure AVRCP Target & Controller remain ENABLED (1).
                            # Never disable AVRCP (0), which uninstalls BthAvrcp and severs Absolute Volume sync!
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$avrcpTgt, [uint32]1) | Out-Null
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$avrcpCtl, [uint32]1) | Out-Null

                            # Enable PnP device node
                            Enable-PnpDevice -InstanceId $inst -Confirm:$false -ErrorAction SilentlyContinue

                            # Cycle Audio Sink and trigger Hands-Free to initiate HCI connection handshake
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$a2dpGuid, [uint32]0) | Out-Null
                            Start-Sleep -Milliseconds 250
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$a2dpGuid, [uint32]1) | Out-Null
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$hfpGuid, [uint32]1) | Out-Null
                        }} else {{
                            # Clean disconnect: Only disconnect streaming channels (A2DP / HFP).
                            # Leave AVRCP enabled so future connections have volume control ready!
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$a2dpGuid, [uint32]0) | Out-Null
                            [BthNativeToggle]::BluetoothSetServiceState([IntPtr]::Zero, [ref]$di, [ref]$hfpGuid, [uint32]0) | Out-Null
                        }}

                        $handled = $true
                        break
                    }}
                }} while ([BthNativeToggle]::BluetoothFindNextDevice($hFind, [ref]$di))
            }} finally {{
                $null = [BthNativeToggle]::BluetoothFindDeviceClose($hFind)
            }}
        }}
    }}

    # 2. BLE or Fallback Device
    if (-not $handled) {{
        if ($connect) {{
            Enable-PnpDevice -InstanceId $inst -Confirm:$false -ErrorAction SilentlyContinue
            if ($addr -and $inst -like 'BTHLE*') {{
                try {{
                    $mac = [Convert]::ToUInt64($addr, 16)
                    $ble = Await ([Windows.Devices.Bluetooth.BluetoothLEDevice]::FromBluetoothAddressAsync($mac)) ([Windows.Devices.Bluetooth.BluetoothLEDevice])
                    if ($ble) {{
                        $null = Await ($ble.GetGattServicesAsync()) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult])
                    }}
                }} catch {{}}
            }}
        }} else {{
            if ($inst -notlike '*D1003059E720*' -and $inst -notlike '*1308AC00270B*') {{
                Disable-PnpDevice -InstanceId $inst -Confirm:$false -ErrorAction SilentlyContinue
            }}
        }}
    }}

    # 3. Verify actual connection outcome
    $finalConn = $false
    for ($check = 0; $check -lt 8; $check++) {{
        Start-Sleep -Milliseconds 450
        $prop = Get-PnpDeviceProperty -InstanceId $inst -KeyName '{{83DA6326-97A6-4088-9453-A1923F573B29}} 15' -ErrorAction SilentlyContinue
        if ($prop -and $prop.Data -eq $true) {{
            $finalConn = $true
            if ($connect) {{ break }}
        }} else {{
            $finalConn = $false
            if (-not $connect) {{ break }}
        }}
    }}

    # 4. If connected, automatically bind to Windows default audio playback device
    if ($connect -and $finalConn) {{
        $containerId = (Get-PnpDeviceProperty -InstanceId $inst -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction SilentlyContinue).Data
        for ($waitEp = 0; $waitEp -lt 8; $waitEp++) {{
            $ep = Get-PnpDevice -Class AudioEndpoint -Status OK -ErrorAction SilentlyContinue | Where-Object {{
                $_.Present -eq $true -and (
                    ($containerId -and (Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction SilentlyContinue).Data -eq $containerId) -or
                    ($name -and $_.FriendlyName -like "*$name*")
                ) -and $_.FriendlyName -like 'Headphones*'
            }} | Select-Object -First 1

            if (-not $ep) {{
                $ep = Get-PnpDevice -Class AudioEndpoint -Status OK -ErrorAction SilentlyContinue | Where-Object {{
                    $_.Present -eq $true -and (
                        ($containerId -and (Get-PnpDeviceProperty -InstanceId $_.InstanceId -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction SilentlyContinue).Data -eq $containerId) -or
                        ($name -and $_.FriendlyName -like "*$name*")
                    ) -and $_.InstanceId -like 'SWD\MMDEVAPI\{{0.0.0.*'
                }} | Select-Object -First 1
            }}

            if ($ep) {{
                $epId = $ep.InstanceId -replace '^SWD\\MMDEVAPI\\', ''
                [AudioSwitcher]::SetDefault($epId) | Out-Null
                break
            }}
            Start-Sleep -Milliseconds 350
        }}
    }}

    if ($connect -eq $finalConn) {{
        Write-Output "STATUS:SUCCESS"
    }} else {{
        Write-Output "STATUS:FAILED"
    }}
}} catch {{
    Write-Output "ERR:$($_.Exception.Message)"
}}
"#
        );

        let b64 = Self::encode_b64_utf16(&script);
        let res = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(o) = res {
            let out_str = String::from_utf8_lossy(&o.stdout);
            let err_str = String::from_utf8_lossy(&o.stderr);
            log_info("BLUETOOTH-CMD", &format!("Output: {} | Stderr: {}", out_str.trim(), err_str.trim()));
            return out_str.contains("STATUS:SUCCESS");
        }
        false
    }

    #[cfg(not(target_os = "windows"))]
    fn check_radio_state() -> bool {
        if let Ok(out) = Command::new("rfkill").args(["list", "bluetooth"]).output() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            return !s.contains("soft blocked: yes") && !s.contains("hard blocked: yes");
        }
        true
    }

    #[cfg(not(target_os = "windows"))]
    fn query_devices(favorites: &HashSet<String>, connecting_ids: &HashSet<String>) -> Vec<BluetoothDevice> {
        let mut list = Vec::new();
        if let Ok(out) = Command::new("bluetoothctl").args(["devices"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0] == "Device" {
                    let addr = parts[1].to_string();
                    let name = parts[2..].join(" ");
                    let is_fav = favorites.contains(&addr);
                    let is_conn = connecting_ids.contains(&addr);
                    let dev_type = BluetoothDeviceType::from_name_or_cat(&name, "");
                    list.push(BluetoothDevice {
                        name,
                        instance_id: addr.clone(),
                        address: addr,
                        connected: false,
                        dev_type,
                        is_favorite: is_fav,
                        is_connecting: is_conn,
                    });
                }
            }
        }
        list
    }

    fn encode_b64_utf16(script: &str) -> String {
        let utf16_bytes: Vec<u8> = script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in utf16_bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

            out.push(CHARS[(b0 >> 2) as usize] as char);
            out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                out.push(CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(CHARS[(b2 & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }
}
