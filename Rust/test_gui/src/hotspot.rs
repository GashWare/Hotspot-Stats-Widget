#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
#[allow(unused_imports)]
use crate::logger::{log_error, log_info, log_success, log_warn};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotspotState {
    Off,
    On,
    InTransition,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct WirelessAdapterInfo {
    pub name: String,
    pub description: String,
    pub status: String,
    pub mac: String,
    pub is_virtual: bool,
}

#[derive(Clone, Debug)]
pub struct ConnectedClient {
    pub ip: String,
    pub mac: String,
    pub hostname: String,
    pub down_bps: f64,
    pub up_bps: f64,
    pub total_bytes: u64,
    pub open_ports: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HotspotApConfig {
    pub ssid: String,
    pub password: String,
    pub band: String, // "5 GHz", "2.4 GHz", "Auto"
}

pub struct HotspotManager {
    pub state: Arc<Mutex<HotspotState>>,
    pub clients: Arc<Mutex<Vec<ConnectedClient>>>,
    pub adapters: Arc<Mutex<Vec<WirelessAdapterInfo>>>,
    pub selected_adapter: Arc<Mutex<String>>,
    pub hotspot_ip: Arc<Mutex<String>>,
    pub last_toggle_result: Arc<Mutex<Option<(bool, String)>>>,
    pub ap_config: Arc<Mutex<Option<HotspotApConfig>>>,
    pub preferred_band: Arc<Mutex<String>>,
}

impl HotspotManager {
    pub fn new() -> Self {
        let state = Arc::new(Mutex::new(HotspotState::Unknown));
        let clients: Arc<Mutex<Vec<ConnectedClient>>> = Arc::new(Mutex::new(Vec::new()));
        let adapters = Arc::new(Mutex::new(Vec::new()));
        let selected_adapter = Arc::new(Mutex::new(String::new()));
        let hotspot_ip = Arc::new(Mutex::new("192.168.137.1".to_string()));
        let last_toggle_result = Arc::new(Mutex::new(None));
        let ap_config = Arc::new(Mutex::new(None));
        let preferred_band = Arc::new(Mutex::new("5ghz".to_string()));

        #[cfg(target_os = "windows")]
        if let Some((win_st, win_ip)) = Self::query_windows_hotspot_status() {
            *state.lock().unwrap() = win_st;
            if !win_ip.is_empty() {
                *hotspot_ip.lock().unwrap() = win_ip;
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(cfg) = Self::query_windows_ap_config() {
            *ap_config.lock().unwrap() = Some(cfg);
        }

        let s_clone = Arc::clone(&state);
        let c_clone = Arc::clone(&clients);
        let a_clone = Arc::clone(&adapters);
        let sel_clone = Arc::clone(&selected_adapter);
        let ip_clone = Arc::clone(&hotspot_ip);
        #[allow(unused_variables)]
        let ap_clone = Arc::clone(&ap_config);

        // Background worker for state, client discovery, and adapter enumeration
        thread::spawn(move || {
            let mut tick = 0usize;
            let mut transition_ticks = 0usize;
            let mut net_list = sysinfo::Networks::new_with_refreshed_list();
            loop {
                // Refresh interface list only when needed
                net_list.refresh(true);

                // Failsafe: never allow InTransition to remain stuck indefinitely
                {
                    let mut st_lock = s_clone.lock().unwrap();
                    if *st_lock == HotspotState::InTransition {
                        transition_ticks += 1;
                        if transition_ticks > 5 {
                            // After ~10s in transition, force resolution to current detected state
                            let (current_st, _) = Self::detect_status_and_ip(&net_list);
                            *st_lock = current_st;
                            transition_ticks = 0;
                        }
                    } else {
                        transition_ticks = 0;
                    }
                }

                // 1. In-memory sysinfo status & IP check
                let (current_st, hs_ip) = Self::detect_status_and_ip(&net_list);
                {
                    let mut st_lock = s_clone.lock().unwrap();
                    if *st_lock != HotspotState::InTransition {
                        if current_st == HotspotState::On {
                            *st_lock = HotspotState::On;
                        }
                    } else if current_st == HotspotState::On {
                        *st_lock = HotspotState::On;
                    }
                }
                if !hs_ip.is_empty() && hs_ip != "192.168.137.1" {
                    *ip_clone.lock().unwrap() = hs_ip.clone();
                }

                // 2. Authoritative Windows Hotspot query & dynamic adapter discovery (every 3 ticks = ~6s and on startup)
                if tick % 3 == 0 {
                    #[cfg(target_os = "windows")]
                    {
                        if let Some((win_st, win_ip)) = Self::query_windows_hotspot_status() {
                            let mut st_lock = s_clone.lock().unwrap();
                            if *st_lock != HotspotState::InTransition || win_st == HotspotState::On || win_st == HotspotState::Off {
                                *st_lock = win_st.clone();
                            }
                            if !win_ip.is_empty() {
                                *ip_clone.lock().unwrap() = win_ip;
                            }
                        }
                    }

                    let adps = Self::discover_adapters(&net_list);
                    let mut adp_lock = a_clone.lock().unwrap();
                    *adp_lock = adps;
                    
                    // Validate currently selected adapter; if missing or unplugged from system, smart-select optimal adapter
                    let mut sel_lock = sel_clone.lock().unwrap();
                    let current_valid = !sel_lock.is_empty() && adp_lock.iter().any(|a| a.name == *sel_lock);
                    if !current_valid {
                        if let Some(smart_choice) = Self::select_smart_hotspot_adapter(&adp_lock) {
                            log_info("HOTSPOT", &format!("Smart adapter selection defaulted Hotspot to: {}", smart_choice));
                            *sel_lock = smart_choice;
                        }
                    }
                }

                // 3. Discover Clients if Hotspot is ON (every 4 ticks = ~8s)
                let effective_st = s_clone.lock().unwrap().clone();
                let effective_ip = ip_clone.lock().unwrap().clone();
                if effective_st == HotspotState::On {
                    if tick % 4 == 0 {
                        let list = Self::discover_clients(&effective_ip);
                        let mut c_lock = c_clone.lock().unwrap();
                        
                        // Retain port scan results
                        let mut port_cache: HashMap<String, String> = HashMap::new();
                        for old in c_lock.iter() {
                            if let Some(ref p) = old.open_ports {
                                port_cache.insert(old.ip.clone(), p.clone());
                            }
                        }

                        let mut updated = list;
                        for client in updated.iter_mut() {
                            if let Some(ports) = port_cache.get(&client.ip) {
                                client.open_ports = Some(ports.clone());
                            }
                        }
                        *c_lock = updated;
                    }
                } else {
                    let mut c_lock = c_clone.lock().unwrap();
                    if !c_lock.is_empty() {
                        c_lock.clear();
                    }
                }

                // 4. Query Access Point config (SSID, Band, Password) periodically
                if tick % 15 == 0 {
                    #[cfg(target_os = "windows")]
                    if let Some(cfg) = Self::query_windows_ap_config() {
                        *ap_clone.lock().unwrap() = Some(cfg);
                    }
                }

                tick += 1;
                thread::sleep(Duration::from_millis(2000));
            }
        });

        Self {
            state,
            clients,
            adapters,
            selected_adapter,
            hotspot_ip,
            last_toggle_result,
            ap_config,
            preferred_band,
        }
    }

    pub fn get_state(&self) -> HotspotState {
        self.state.lock().unwrap().clone()
    }

    pub fn get_clients(&self) -> Vec<ConnectedClient> {
        self.clients.lock().unwrap().clone()
    }

    pub fn get_adapters(&self) -> Vec<WirelessAdapterInfo> {
        self.adapters.lock().unwrap().clone()
    }

    pub fn get_selected_adapter(&self) -> String {
        self.selected_adapter.lock().unwrap().clone()
    }

    pub fn set_selected_adapter(&self, name: String) {
        *self.selected_adapter.lock().unwrap() = name;
    }

    pub fn get_hotspot_ip(&self) -> String {
        self.hotspot_ip.lock().unwrap().clone()
    }

    pub fn get_ap_config(&self) -> Option<HotspotApConfig> {
        self.ap_config.lock().unwrap().clone()
    }

    pub fn get_preferred_band(&self) -> String {
        self.preferred_band.lock().unwrap().clone()
    }

    pub fn set_preferred_band(&self, band: String) {
        *self.preferred_band.lock().unwrap() = band.clone();
        #[cfg(target_os = "windows")]
        {
            let ap_clone = Arc::clone(&self.ap_config);
            thread::spawn(move || {
                Self::apply_windows_ap_band(&band);
                if let Some(cfg) = Self::query_windows_ap_config() {
                    *ap_clone.lock().unwrap() = Some(cfg);
                }
            });
        }
    }

    #[allow(unused_variables)]
    pub fn set_ap_credentials(&self, ssid: &str, password: &str) {
        let s = ssid.to_string();
        let p = password.to_string();
        #[cfg(target_os = "windows")]
        {
            let ap_clone = Arc::clone(&self.ap_config);
            thread::spawn(move || {
                Self::apply_windows_ap_credentials(&s, &p);
                if let Some(cfg) = Self::query_windows_ap_config() {
                    *ap_clone.lock().unwrap() = Some(cfg);
                }
            });
        }
    }

    #[allow(unused_variables)]
    pub fn select_smart_hotspot_adapter(adapters: &[WirelessAdapterInfo]) -> Option<String> {
        let physical: Vec<&WirelessAdapterInfo> = adapters.iter().filter(|a| !a.is_virtual).collect();
        if physical.is_empty() {
            return adapters.first().map(|a| a.name.clone());
        }

        // Find adapter currently in use for system Wi-Fi connection (physical wireless with status "Up")
        let in_use_wifi = physical.iter().find(|a| a.status.eq_ignore_ascii_case("Up"));

        // If an in-use Wi-Fi connection is detected, check if another secondary physical wireless adapter is available
        if let Some(in_use) = in_use_wifi {
            if let Some(secondary) = physical.iter().find(|a| a.name != in_use.name) {
                return Some(secondary.name.clone());
            }
            return Some(in_use.name.clone());
        }

        // Fallback to first physical adapter
        physical.first().map(|a| a.name.clone())
    }

    #[allow(unused_variables)]
    pub fn apply_throughput_optimizations(&self, adapter: &str) {
        #[allow(unused_variables)]
        let adp = adapter.to_string();
        #[cfg(target_os = "windows")]
        thread::spawn(move || {
            Self::optimize_network_adapter(&adp);
        });
    }

    pub fn get_last_result(&self) -> Option<(bool, String)> {
        self.last_toggle_result.lock().unwrap().clone()
    }

    pub fn start_hotspot(&self) {
        let current = self.get_state();
        if current == HotspotState::On {
            log_info("HOTSPOT", "Hotspot is already running; skipping start.");
            return;
        }
        self.trigger_toggle(true);
    }

    pub fn stop_hotspot(&self) {
        let current = self.get_state();
        if current == HotspotState::Off {
            log_info("HOTSPOT", "Hotspot is already stopped; skipping stop.");
            return;
        }
        self.trigger_toggle(false);
    }

    pub fn toggle_hotspot(&self) {
        let current = self.get_state();
        let target_on = current != HotspotState::On;
        self.trigger_toggle(target_on);
    }

    fn trigger_toggle(&self, target_on: bool) {
        #[allow(unused_variables)]
        let adapter = self.get_selected_adapter();
        #[allow(unused_variables)]
        let band = self.get_preferred_band();
        *self.state.lock().unwrap() = HotspotState::InTransition;

        let state_arc = Arc::clone(&self.state);
        let result_arc = Arc::clone(&self.last_toggle_result);

        thread::spawn(move || {
            log_info(
                "HOTSPOT",
                &format!("User initiated Hotspot toggle: request target state = {}", if target_on { "START/ON" } else { "STOP/OFF" }),
            );

            #[cfg(target_os = "windows")]
            {
                let success = Self::toggle_hotspot_windows(target_on, &adapter, &band);
                let (final_state, msg) = if success {
                    let st = if target_on { HotspotState::On } else { HotspotState::Off };
                    let text = format!("Hotspot {} successfully.", if target_on { "started" } else { "stopped" });
                    log_success("HOTSPOT", &text);
                    (st, text)
                } else {
                    let text = format!("Failed to {} hotspot. Check logs or verify Wi-Fi adapter is enabled.", if target_on { "start" } else { "stop" });
                    log_error("HOTSPOT", &text);
                    (if target_on { HotspotState::Off } else { HotspotState::On }, text)
                };

                *state_arc.lock().unwrap() = final_state;
                *result_arc.lock().unwrap() = Some((success, msg));
            }

            #[cfg(target_os = "linux")]
            {
                let iface = if adapter.is_empty() { "wlan0" } else { &adapter };
                let output = if target_on {
                    Command::new("nmcli")
                        .args(["dev", "wifi", "hotspot", "ifname", iface])
                        .output()
                } else {
                    Command::new("nmcli")
                        .args(["con", "down", "Hotspot"])
                        .output()
                };

                match output {
                    Ok(out) if out.status.success() => {
                        let text = format!("Linux hotspot {} OK.", if target_on { "started" } else { "stopped" });
                        log_success("HOTSPOT", &text);
                        *state_arc.lock().unwrap() = if target_on { HotspotState::On } else { HotspotState::Off };
                        *result_arc.lock().unwrap() = Some((true, text));
                    }
                    Ok(out) => {
                        let err_str = String::from_utf8_lossy(&out.stderr).to_string();
                        let text = format!("nmcli error: {}", err_str.trim());
                        log_error("HOTSPOT", &text);
                        *state_arc.lock().unwrap() = if target_on { HotspotState::Off } else { HotspotState::On };
                        *result_arc.lock().unwrap() = Some((false, text));
                    }
                    Err(e) => {
                        let text = format!("Failed to invoke nmcli: {}", e);
                        log_error("HOTSPOT", &text);
                        *state_arc.lock().unwrap() = HotspotState::Unknown;
                        *result_arc.lock().unwrap() = Some((false, text));
                    }
                }
            }
        });
    }

    #[cfg(target_os = "windows")]
    fn toggle_hotspot_windows(target_on: bool, adapter: &str, preferred_band: &str) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let action_name = if target_on { "StartTetheringAsync" } else { "StopTetheringAsync" };
        log_info("HOTSPOT-WIN", &format!("Executing Windows Mobile Hotspot API via PowerShell (Action: {}, Target Adapter: '{}', Band: '{}')...", action_name, adapter, preferred_band));

        let safe_adapter = adapter.replace('\'', "").replace('"', "");
        // Method 1: Windows Runtime NetworkOperatorTetheringManager (Native Windows 10/11 Mobile Hotspot)
        let ps_script = if target_on {
            format!(r#"
try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
    function Await($WinRtTask, $ResultType) {{
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(12000) | Out-Null
        $netTask.Result
    }}

    $netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
    $tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
    $opResultType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

    # Ensure Wi-Fi radio is active
    try {{
        $radioType = [Windows.Devices.Radios.Radio, Windows.Devices.Radios, ContentType=WindowsRuntime]
        $radios = Await ($radioType::GetRadiosAsync()) ([System.Collections.Generic.IReadOnlyList[Windows.Devices.Radios.Radio]])
        foreach ($r in $radios) {{
            if ($r.Kind -eq [Windows.Devices.Radios.RadioKind]::WiFi -and $r.State -ne [Windows.Devices.Radios.RadioState]::On) {{
                Await ($r.SetStateAsync([Windows.Devices.Radios.RadioState]::On)) ([Windows.Devices.Radios.RadioAccessStatus]) | Out-Null
            }}
        }}
    }} catch {{}}

    $allProfiles = $netInfoType::GetConnectionProfiles()
    $internetProfile = $netInfoType::GetInternetConnectionProfile()

    $targetAdapter = '{0}'
    $targetNetworkAdapter = $null
    if ($targetAdapter -ne '') {{
        $adpObj = Get-NetAdapter -Name $targetAdapter -ErrorAction SilentlyContinue
        if ($adpObj) {{
            $adpGuid = $adpObj.InterfaceGuid.Trim('{{}}')
            foreach ($p in $allProfiles) {{
                if ($p.NetworkAdapter -and $p.NetworkAdapter.NetworkAdapterId -eq $adpGuid) {{
                    $targetNetworkAdapter = $p.NetworkAdapter
                    break
                }}
            }}
        }}
    }}

    # Build prioritized candidate tethering managers:
    # 1. Dedicated Dual-Adapter Tethering: Internet from $internetProfile, AP broadcast on $targetNetworkAdapter
    # 2. Single-Adapter Tethering: $internetProfile
    # 3. Fallback to other available profiles
    $mgrList = @()
    if ($internetProfile -and $targetNetworkAdapter) {{
        try {{
            $m1 = $tetherMgrType::CreateFromConnectionProfile($internetProfile, $targetNetworkAdapter)
            if ($m1) {{ $mgrList += $m1 }}
        }} catch {{}}
    }}
    if ($internetProfile) {{
        try {{
            $m2 = $tetherMgrType::CreateFromConnectionProfile($internetProfile)
            if ($m2) {{ $mgrList += $m2 }}
        }} catch {{}}
    }}
    foreach ($p in $allProfiles) {{
        try {{
            $mp = $tetherMgrType::CreateFromConnectionProfile($p)
            if ($mp -and ($mgrList -notcontains $mp)) {{ $mgrList += $mp }}
        }} catch {{}}
    }}

    $started = $false
    $lastErr = ""
    $bType = [Windows.Networking.NetworkOperators.TetheringWiFiBand, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
    $desiredBand = if ('{1}' -eq '2.4ghz') {{ $bType::TwoPointFourGigahertz }} elseif ('{1}' -eq 'auto') {{ $bType::Auto }} else {{ $bType::FiveGigahertz }}

    foreach ($mgr in $mgrList) {{
        try {{
            if ($mgr.TetheringOperationalState.ToString() -eq 'On') {{
                $started = $true
                Write-Output "STATUS:Success|ALREADY_ON"
                break
            }}

            # Configure band
            try {{
                $curCfg = $mgr.GetCurrentAccessPointConfiguration()
                if ($curCfg.Band -ne $desiredBand) {{
                    $curCfg.Band = $desiredBand
                    $asAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' }})[0]
                    $tAct = $asAction.Invoke($null, @($mgr.ConfigureAccessPointAsync($curCfg)))
                    $tAct.Wait(4000) | Out-Null
                }}
            }} catch {{}}

            $op = $mgr.StartTetheringAsync()
            $res = Await $op $opResultType
            if ($res -and ($res.Status.ToString() -eq 'Success' -or [int]$res.Status -eq 0)) {{
                $started = $true
                Write-Output "STATUS:Success"
                break
            }} else {{
                $lastErr = "Result: $($res.Status) (err: $($res.AdditionalErrorCode))"
                # Failsafe: If starting on 5 GHz returned WiFiDeviceOff or failure due to hardware SoftAP limits, auto-fallback to Auto band
                if ($res -and ($res.Status.ToString() -eq 'WiFiDeviceOff' -or [int]$res.Status -ne 0)) {{
                    try {{
                        $curCfg = $mgr.GetCurrentAccessPointConfiguration()
                        if ($curCfg.Band -ne $bType::Auto) {{
                            $curCfg.Band = $bType::Auto
                            $asAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' }})[0]
                            $tAct = $asAction.Invoke($null, @($mgr.ConfigureAccessPointAsync($curCfg)))
                            $tAct.Wait(3000) | Out-Null
                            $op2 = $mgr.StartTetheringAsync()
                            $res2 = Await $op2 $opResultType
                            if ($res2 -and ($res2.Status.ToString() -eq 'Success' -or [int]$res2.Status -eq 0)) {{
                                $started = $true
                                Write-Output "STATUS:Success|BAND_FALLBACK"
                                break
                            }}
                        }}
                    }} catch {{}}
                }}
            }}
        }} catch {{
            $lastErr = $_.Exception.Message
        }}
    }}

    if ($started) {{
        # Apply high-throughput settings & zero timeout
        Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings' -Name 'PeerlessTimeoutEnabled' -Value 0 -ErrorAction SilentlyContinue
        netsh int tcp set global autotuninglevel=normal | Out-Null
    }} else {{
        Write-Output "STATUS:Failed|ERR:$lastErr"
    }}
}} catch {{
    Write-Output "STATUS:Exception|ERR:$($_.Exception.Message)"
}}
"#, safe_adapter, preferred_band)
        } else {
            r#"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
    function Await($WinRtTask, $ResultType) {
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(12000) | Out-Null
        $netTask.Result
    }

    $netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
    $tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
    $opResultType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringOperationResult, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

    $allProfiles = $netInfoType::GetConnectionProfiles()
    foreach ($prof in $allProfiles) {
        try {
            $mgr = $tetherMgrType::CreateFromConnectionProfile($prof)
            if ($mgr -and $mgr.TetheringOperationalState -ne 0 -and $mgr.TetheringOperationalState -ne 'Off') {
                $op = $mgr.StopTetheringAsync()
                $res = Await $op $opResultType
            }
        } catch {}
    }
    $ip = $netInfoType::GetInternetConnectionProfile()
    if ($ip) {
        try {
            $mgr = $tetherMgrType::CreateFromConnectionProfile($ip)
            if ($mgr -and $mgr.TetheringOperationalState -ne 0 -and $mgr.TetheringOperationalState -ne 'Off') {
                $op = $mgr.StopTetheringAsync()
                $res = Await $op $opResultType
            }
        } catch {}
    }
    Write-Output "STATUS:Success"
} catch {
    Write-Output "STATUS:Exception|ERR:$($_.Exception.Message)"
}
"#.to_string()
        };

        // Convert script into UTF-16LE Base64 for 100% reliable execution via powershell.exe -EncodedCommand
        let utf16_bytes: Vec<u8> = ps_script
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let b64_encoded = Self::base64_encode(&utf16_bytes);

        let ps_run = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64_encoded])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match ps_run {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                log_info("HOTSPOT-WIN", &format!("PowerShell Output: [{}] Stderr: [{}]", stdout, stderr));

                if stdout.contains("STATUS:Success") {
                    log_success("HOTSPOT-WIN", "NetworkOperatorTetheringManager reported Success.");
                    return true;
                }

                if !stdout.is_empty() {
                    log_warn("HOTSPOT-WIN", &format!("NetworkOperatorTetheringManager responded with non-success: {}", stdout));
                }
            }
            Err(e) => {
                log_error("HOTSPOT-WIN", &format!("Failed to spawn powershell: {}", e));
            }
        }

        // Method 2: Fallback to netsh hostednetwork (Supported on virtual hosted adapters / legacy Wi-Fi)
        log_info("HOTSPOT-WIN", "Attempting Method 2: netsh hostednetwork fallback...");
        let netsh_cmd = if target_on { "start" } else { "stop" };
        let netsh_out = Command::new("netsh")
            .args(["wlan", netsh_cmd, "hostednetwork"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(no) = netsh_out {
            let out_str = String::from_utf8_lossy(&no.stdout);
            let err_str = String::from_utf8_lossy(&no.stderr);
            log_info("HOTSPOT-WIN", &format!("netsh hostednetwork {}: Stdout=[{}], Stderr=[{}]", netsh_cmd, out_str.trim(), err_str.trim()));
            let lower = out_str.to_lowercase();
            if target_on {
                if (lower.contains("the hosted network started") || lower.contains("has been started")) && !lower.contains("couldn't") && !lower.contains("failed") && !lower.contains("error") {
                    return true;
                }
            } else {
                if (lower.contains("the hosted network stopped") || lower.contains("has been stopped")) && !lower.contains("couldn't") && !lower.contains("failed") && !lower.contains("error") {
                    return true;
                }
            }
        }

        false
    }

    fn base64_encode(data: &[u8]) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in data.chunks(3) {
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

    pub fn scan_client_ports(&self, ip: String) {
        let clean_ip = ip.trim().to_string();
        if clean_ip.is_empty() || clean_ip == "-" {
            log_warn("SCAN", "Cannot scan client with invalid or unassigned IP address");
            return;
        }

        // Immediate visual feedback in UI
        {
            let mut lock = self.clients.lock().unwrap();
            for c in lock.iter_mut() {
                if c.ip == clean_ip {
                    c.open_ports = Some("[Scanning...]".to_string());
                    break;
                }
            }
        }

        let clients_clone = Arc::clone(&self.clients);
        let target_ip = clean_ip.clone();
        thread::spawn(move || {
            log_info("SCAN", &format!("Initiating fast parallel port scan for device: {}", target_ip));

            // Expanded list of common IoT, Mobile, Desktop, Media, and Admin ports
            let common_ports: &[u16] = &[
                21,    // FTP
                22,    // SSH
                23,    // Telnet
                53,    // DNS
                80,    // HTTP
                135,   // MS RPC
                137,   // NetBIOS Name
                139,   // NetBIOS Session
                443,   // HTTPS
                445,   // SMB
                554,   // RTSP (Cams)
                1900,  // SSDP / UPnP
                3389,  // RDP
                5000,  // UPnP / Synology
                5353,  // mDNS
                7000,  // AirPlay
                8000,  // Dev HTTP
                8008,  // Chromecast
                8080,  // HTTP Alternate
                8443,  // HTTPS Alternate
                9000,  // Media Server
                62078, // Apple iDevice Sync
            ];

            // Resolve target address: parse IPv4/IPv6 or fallback to lookup_host
            let ip_addr = match target_ip.parse::<std::net::IpAddr>() {
                Ok(a) => Some(a),
                Err(_) => {
                    use std::net::ToSocketAddrs;
                    format!("{}:80", target_ip)
                        .to_socket_addrs()
                        .ok()
                        .and_then(|mut iter| iter.next().map(|s| s.ip()))
                }
            };

            let Some(addr_ip) = ip_addr else {
                log_warn("SCAN", &format!("Failed to parse or resolve IP address: '{}'", target_ip));
                let mut lock = clients_clone.lock().unwrap();
                for c in lock.iter_mut() {
                    if c.ip == target_ip {
                        c.open_ports = Some("[Invalid IP]".to_string());
                        break;
                    }
                }
                return;
            };

            let open_ports_arc = Arc::new(Mutex::new(Vec::new()));
            let mut handles = Vec::with_capacity(common_ports.len());

            for &p in common_ports {
                let op_arc = Arc::clone(&open_ports_arc);
                let sa = SocketAddr::new(addr_ip, p);
                handles.push(thread::spawn(move || {
                    // Try with 450ms timeout; retry with brief window if timeout is close
                    if TcpStream::connect_timeout(&sa, Duration::from_millis(450)).is_ok() {
                        op_arc.lock().unwrap().push(p);
                    }
                }));
            }

            for h in handles {
                let _ = h.join();
            }

            let mut open_list: Vec<u16> = open_ports_arc.lock().unwrap().clone();
            open_list.sort_unstable();

            let result_str = if open_list.is_empty() {
                "[No Open Ports Found]".to_string()
            } else {
                let ports_joined = open_list
                    .iter()
                    .map(|p| match p {
                        22 => "22(SSH)",
                        53 => "53(DNS)",
                        80 => "80(HTTP)",
                        135 => "135(RPC)",
                        139 => "139(NetBIOS)",
                        443 => "443(HTTPS)",
                        445 => "445(SMB)",
                        3389 => "3389(RDP)",
                        5353 => "5353(mDNS)",
                        8008 => "8008(Cast)",
                        8080 => "8080(HTTP)",
                        8443 => "8443(HTTPS)",
                        62078 => "62078(Apple)",
                        _ => "",
                    })
                    .enumerate()
                    .map(|(i, tag)| {
                        if tag.is_empty() {
                            open_list[i].to_string()
                        } else {
                            tag.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[Open: {}]", ports_joined)
            };

            log_info("SCAN", &format!("Scan finished for {}: {}", target_ip, result_str));

            let mut lock = clients_clone.lock().unwrap();
            for c in lock.iter_mut() {
                if c.ip == target_ip {
                    c.open_ports = Some(result_str.clone());
                    break;
                }
            }
        });
    }

    #[cfg(target_os = "windows")]
    fn query_windows_hotspot_status() -> Option<(HotspotState, String)> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let ps_script = r#"
[Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime] | Out-Null
[Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime] | Out-Null
$state = 'Off'
$ip = '192.168.137.1'
$profiles = [Windows.Networking.Connectivity.NetworkInformation]::GetConnectionProfiles()
foreach ($p in $profiles) {
    try {
        $mgr = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager]::CreateFromConnectionProfile($p)
        if ($mgr) {
            $stStr = $mgr.TetheringOperationalState.ToString()
            if ($stStr -eq 'On') {
                $state = 'On'
                break
            } elseif ($stStr -eq 'InTransition') {
                $state = 'InTransition'
            }
        }
    } catch {}
}
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { 
    $_.Status -eq 'Up' -and ($_.InterfaceDescription -like '*Wi-Fi Direct*' -or $_.Name -like '*Local Area Connection**')
}
foreach ($a in $adapters) {
    $ipObj = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -like '192.168.*' -or $_.IPAddress -like '10.42.*' } | Select-Object -First 1
    if ($ipObj -and $ipObj.IPAddress) {
        $state = 'On'
        $ip = $ipObj.IPAddress
        break
    }
}
Write-Output "HS_STATUS:$state|$ip"
"#;

        let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = Self::base64_encode(&utf16_bytes);

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("HS_STATUS:") {
                    if let Some((st_str, ip_str)) = rest.split_once('|') {
                        let st = match st_str.trim() {
                            "On" => HotspotState::On,
                            "InTransition" => HotspotState::InTransition,
                            _ => HotspotState::Off,
                        };
                        return Some((st, ip_str.trim().to_string()));
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    pub fn query_windows_ap_config() -> Option<HotspotApConfig> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let ps_script = r#"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
    $tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
    $prof = $netInfoType::GetInternetConnectionProfile()
    if (-not $prof) {
        $prof = ($netInfoType::GetConnectionProfiles() | Select-Object -First 1)
    }
    if ($prof) {
        $mgr = $tetherMgrType::CreateFromConnectionProfile($prof)
        if ($mgr) {
            $cfg = $mgr.GetCurrentAccessPointConfiguration()
            $b = switch ([int]$cfg.Band) {
                1 { "2.4 GHz" }
                2 { "5 GHz" }
                3 { "6 GHz" }
                default { "Auto" }
            }
            Write-Output "AP_CONFIG|$($cfg.Ssid)|$($cfg.Passphrase)|$b"
        }
    }
} catch {}
"#;
        let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = Self::base64_encode(&utf16_bytes);

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("AP_CONFIG|") {
                    let parts: Vec<&str> = rest.split('|').collect();
                    if parts.len() >= 3 {
                        return Some(HotspotApConfig {
                            ssid: parts[0].to_string(),
                            password: parts[1].to_string(),
                            band: parts[2].to_string(),
                        });
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    pub fn apply_windows_ap_band(band: &str) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let b = band.to_lowercase();
        let ps_script = format!(r#"
try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asActionGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' }})[0]
    function AwaitAction($WinRtTask) {{
        $netTask = $asActionGeneric.Invoke($null, @($WinRtTask))
        $netTask.Wait(10000) | Out-Null
        $netTask.Result
    }}

    $netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
    $tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]
    $bandType = [Windows.Networking.NetworkOperators.TetheringWiFiBand, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

    $prof = $netInfoType::GetInternetConnectionProfile()
    if (-not $prof) {{
        $prof = ($netInfoType::GetConnectionProfiles() | Select-Object -First 1)
    }}
    if ($prof) {{
        $mgr = $tetherMgrType::CreateFromConnectionProfile($prof)
        if ($mgr) {{
            $cfg = $mgr.GetCurrentAccessPointConfiguration()
            if ('{0}' -eq '5ghz') {{
                $cfg.Band = $bandType::FiveGigahertz
            }} elseif ('{0}' -eq '2.4ghz') {{
                $cfg.Band = $bandType::TwoPointFourGigahertz
            }} else {{
                $cfg.Band = $bandType::Auto
            }}
            $act = $mgr.ConfigureAccessPointAsync($cfg)
            AwaitAction $act
            Write-Output "SUCCESS"
        }}
    }}
}} catch {{}}
"#, b);

        let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = Self::base64_encode(&utf16_bytes);

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            return text.contains("SUCCESS");
        }
        false
    }

    #[cfg(target_os = "windows")]
    pub fn apply_windows_ap_credentials(ssid: &str, password: &str) -> bool {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let s = ssid.replace('\'', "").replace('"', "");
        let p = password.replace('\'', "").replace('"', "");

        let ps_script = format!(r#"
try {{
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asActionGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' }})[0]
    function AwaitAction($WinRtTask) {{
        $netTask = $asActionGeneric.Invoke($null, @($WinRtTask))
        $netTask.Wait(10000) | Out-Null
        $netTask.Result
    }}

    $netInfoType = [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime]
    $tetherMgrType = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime]

    $prof = $netInfoType::GetInternetConnectionProfile()
    if (-not $prof) {{
        $prof = ($netInfoType::GetConnectionProfiles() | Select-Object -First 1)
    }}
    if ($prof) {{
        $mgr = $tetherMgrType::CreateFromConnectionProfile($prof)
        if ($mgr) {{
            $cfg = $mgr.GetCurrentAccessPointConfiguration()
            if ('{0}' -ne '') {{ $cfg.Ssid = '{0}' }}
            if ('{1}' -ne '') {{ $cfg.Passphrase = '{1}' }}
            $act = $mgr.ConfigureAccessPointAsync($cfg)
            AwaitAction $act
            Write-Output "SUCCESS"
        }}
    }}
}} catch {{}}
"#, s, p);

        let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = Self::base64_encode(&utf16_bytes);

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            return text.contains("SUCCESS");
        }
        false
    }

    #[cfg(target_os = "windows")]
    pub fn optimize_network_adapter(adapter: &str) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        log_info("HOTSPOT-OPT", &format!("Applying high-throughput optimizations to adapter '{}'...", adapter));
        let safe_adp = adapter.replace('\'', "").replace('"', "");

        let ps_script = format!(r#"
Set-ItemProperty -Path 'HKLM:\System\CurrentControlSet\Services\icssvc\Settings' -Name 'PeerlessTimeoutEnabled' -Value 0 -ErrorAction SilentlyContinue
netsh int tcp set global autotuninglevel=normal | Out-Null
$adp = '{0}'
if ($adp -ne '') {{
    # Restore/set Wireless Mode to Auto (supports 802.11a/b/g/n/ac) so Wi-Fi Direct SoftAP works harmoniously with 5GHz AC
    Set-NetAdapterAdvancedProperty -Name $adp -DisplayName 'Wireless Mode' -DisplayValue 'Auto' -ErrorAction SilentlyContinue
    Set-NetAdapterAdvancedProperty -Name $adp -DisplayName 'VHT 2.4G' -DisplayValue 'Enable' -ErrorAction SilentlyContinue
    Set-NetAdapterAdvancedProperty -Name $adp -DisplayName 'QoS Support' -DisplayValue 'Support QoS' -ErrorAction SilentlyContinue
}}
Write-Output "OPTIMIZED"
"#, safe_adp);

        let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let b64 = Self::base64_encode(&utf16_bytes);

        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("OPTIMIZED") {
                log_success("HOTSPOT-OPT", "Successfully applied 5GHz/802.11ac high-throughput and always-on optimizations!");
            }
        }
    }

    fn detect_status_and_ip(networks: &sysinfo::Networks) -> (HotspotState, String) {
        #[cfg(target_os = "windows")]
        {
            for (name, data) in networks {
                let nl = name.to_lowercase();
                for ip in data.ip_networks() {
                    let ip_str = ip.addr.to_string();
                    if ip_str == "192.168.137.1"
                        || (ip_str.starts_with("192.168.137.") && ip_str != "192.168.137.255")
                        || ((nl.contains("wi-fi direct") || nl.contains("hosted network") || nl.contains('*')) && ip_str.starts_with("192.168."))
                    {
                        return (HotspotState::On, ip_str);
                    }
                }
            }

            // Also check Windows netsh status
            (HotspotState::Off, "192.168.137.1".to_string())
        }
        #[cfg(target_os = "linux")]
        {
            for (_name, data) in networks {
                for ip in data.ip_networks() {
                    let ip_str = ip.addr.to_string();
                    if ip_str == "10.42.0.1" || ip_str == "192.168.137.1" {
                        return (HotspotState::On, ip_str);
                    }
                }
            }
            if let Ok(out) = Command::new("nmcli").args(["con", "show", "--active"]).output() {
                let text = String::from_utf8_lossy(&out.stdout);
                if text.contains("Hotspot") {
                    return (HotspotState::On, "10.42.0.1".to_string());
                }
            }
            (HotspotState::Off, "10.42.0.1".to_string())
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            let _ = networks;
            (HotspotState::Unknown, "192.168.137.1".to_string())
        }
    }

    fn discover_adapters(networks: &sysinfo::Networks) -> Vec<WirelessAdapterInfo> {
        let mut list = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            // 1. Authoritative discovery via PowerShell Get-NetAdapter
            let ps_script = r#"
Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object {
    $_.PhysicalMediaType -like '*802.11*' -or
    $_.InterfaceDescription -like '*Wi-Fi*' -or
    $_.InterfaceDescription -like '*Wireless*' -or
    $_.InterfaceDescription -like '*802.11*' -or
    $_.InterfaceDescription -like '*Wi-Fi Direct*'
} | ForEach-Object {
    $isVirt = if ($_.InterfaceDescription -like '*Virtual*' -or $_.InterfaceDescription -like '*Direct*') { '1' } else { '0' }
    Write-Output "ADAPTER|$($_.Name)|$($_.InterfaceDescription)|$($_.Status)|$($_.MacAddress)|$isVirt"
}
"#;
            let utf16_bytes: Vec<u8> = ps_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            let b64 = Self::base64_encode(&utf16_bytes);

            if let Ok(out) = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let line = line.trim();
                    if let Some(rest) = line.strip_prefix("ADAPTER|") {
                        let parts: Vec<&str> = rest.split('|').collect();
                        if parts.len() >= 5 {
                            let name = parts[0].trim().to_string();
                            let desc = parts[1].trim().to_string();
                            let status = parts[2].trim().to_string();
                            let mac = parts[3].trim().to_uppercase();
                            let is_virt = parts[4].trim() == "1";
                            if !name.is_empty() {
                                list.push(WirelessAdapterInfo {
                                    name,
                                    description: if desc.is_empty() { "Wireless Adapter".to_string() } else { desc },
                                    status: if status.is_empty() { "Up".to_string() } else { status },
                                    mac,
                                    is_virtual: is_virt,
                                });
                            }
                        }
                    }
                }
            }

            // 2. Also incorporate any Wi-Fi adapters from sysinfo::Networks not yet listed (strictly filtering out WAN miniports, virtual debug, and tunnel adapters)
            for (name, data) in networks {
                let nl = name.to_lowercase();
                let is_junk = nl.contains("wan miniport")
                    || nl.contains("miniport")
                    || nl.contains("teredo")
                    || nl.contains("kernel")
                    || nl.contains("loopback")
                    || nl.contains("bluetooth")
                    || nl.contains("wireguard")
                    || nl.contains("pdanet");

                if !is_junk
                    && (nl.contains("wi-fi") || nl.contains("wlan") || nl.contains("wireless") || nl.contains("wi-fi direct"))
                    && !list.iter().any(|a| a.name.eq_ignore_ascii_case(name))
                {
                    let is_virt = nl.contains("virtual") || nl.contains("direct") || nl.contains('*');
                    list.push(WirelessAdapterInfo {
                        name: name.clone(),
                        description: name.clone(),
                        status: "Up".to_string(),
                        mac: data.mac_address().to_string(),
                        is_virtual: is_virt,
                    });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let _ = networks;
            if let Ok(out) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE", "dev"]).output() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 && parts[1] == "wifi" {
                        list.push(WirelessAdapterInfo {
                            name: parts[0].to_string(),
                            description: format!("Wi-Fi ({})", parts[0]),
                            status: parts[2].to_string(),
                            mac: String::new(),
                            is_virtual: false,
                        });
                    }
                }
            }
        }

        list
    }

    fn discover_clients(hotspot_ip: &str) -> Vec<ConnectedClient> {
        let mut clients_map: HashMap<String, ConnectedClient> = HashMap::new();
        let prefix = format!("{}.", hotspot_ip.rsplit_once('.').map(|(p, _)| p).unwrap_or("192.168.137"));

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            // 1. Authoritative query: Query Windows Runtime NetworkOperatorTetheringManager for currently authenticated Hotspot stations
            let winrt_script = r#"
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    [Windows.Networking.Connectivity.NetworkInformation, Windows.Networking.Connectivity, ContentType=WindowsRuntime] | Out-Null
    [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager, Windows.Networking.NetworkOperators, ContentType=WindowsRuntime] | Out-Null
    $profiles = [Windows.Networking.Connectivity.NetworkInformation]::GetConnectionProfiles()
    foreach ($p in $profiles) {
        try {
            $mgr = [Windows.Networking.NetworkOperators.NetworkOperatorTetheringManager]::CreateFromConnectionProfile($p)
            if ($mgr -and $mgr.TetheringOperationalState -eq 1) {
                $clients = $mgr.GetTetheringClients()
                foreach ($c in $clients) {
                    $h = $c.HostNames
                    $hostName = ""
                    $ip = ""
                    foreach ($item in $h) {
                        $dn = $item.DisplayName
                        if ($dn -match '^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$') {
                            $ip = $dn
                        } elseif ($dn -ne "") {
                            $hostName = $dn
                        }
                    }
                    Write-Output "WINRT_CLIENT|$($c.MacAddress)|$ip|$hostName"
                }
                break
            }
        } catch {}
    }
} catch {}
"#;
            let utf16_bytes: Vec<u8> = winrt_script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            let b64_encoded = Self::base64_encode(&utf16_bytes);
            let mut winrt_found_any = false;

            if let Ok(out) = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &b64_encoded])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with("WINRT_CLIENT|") {
                        let parts: Vec<&str> = line.split('|').collect();
                        if parts.len() >= 4 {
                            let mac = parts[1].to_uppercase();
                            let ip = parts[2].trim();
                            let host = parts[3].trim();
                            if !mac.is_empty() {
                                winrt_found_any = true;
                                clients_map.insert(mac.clone(), ConnectedClient {
                                    ip: if ip.is_empty() { "-".to_string() } else { ip.to_string() },
                                    mac,
                                    hostname: if host.is_empty() { "Client Device".to_string() } else { host.to_string() },
                                    down_bps: 0.0,
                                    up_bps: 0.0,
                                    total_bytes: 0,
                                    open_ports: None,
                                });
                            }
                        }
                    }
                }
            }

            // 2. Query ARP table to discover or correlate IP/MAC
            if let Ok(out) = Command::new("arp")
                .arg("-a")
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let ip = parts[0];
                        let mac = parts[1].to_uppercase().replace('-', ":");
                        let entry_type = parts[2];
                        if ip.starts_with(&prefix) && ip != hotspot_ip && !ip.ends_with(".255") {
                            if entry_type == "dynamic" || entry_type == "static" {
                                if winrt_found_any {
                                    // Correlate IP to matching WinRT client MAC if missing
                                    if let Some(c) = clients_map.get_mut(&mac) {
                                        if c.ip.is_empty() || c.ip == "-" {
                                            c.ip = ip.to_string();
                                        }
                                    }
                                } else {
                                    // Fallback when WinRT is unavailable: check reachable
                                    clients_map.insert(mac.clone(), ConnectedClient {
                                        ip: ip.to_string(),
                                        mac,
                                        hostname: "Client Device".to_string(),
                                        down_bps: 0.0,
                                        up_bps: 0.0,
                                        total_bytes: 0,
                                        open_ports: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(out) = Command::new("ip").args(["neigh"]).output() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let ip = parts[0];
                        let mac = parts[4].to_uppercase();
                        if ip.starts_with(&prefix) && ip != hotspot_ip {
                            clients_map.insert(mac.clone(), ConnectedClient {
                                ip: ip.to_string(),
                                mac,
                                hostname: "Client Device".to_string(),
                                down_bps: 0.0,
                                up_bps: 0.0,
                                total_bytes: 0,
                                open_ports: None,
                            });
                        }
                    }
                }
            }
        }

        let mut list: Vec<ConnectedClient> = clients_map.into_values().collect();
        // Deterministic sorting by IP numerical address so list never reshuffles unexpectedly
        list.sort_by(|a, b| {
            let parse_octets = |s: &str| -> [u8; 4] {
                let mut octets = [0u8; 4];
                for (i, part) in s.split('.').take(4).enumerate() {
                    octets[i] = part.parse::<u8>().unwrap_or(0);
                }
                octets
            };
            parse_octets(&a.ip).cmp(&parse_octets(&b.ip))
        });
        list
    }
}
