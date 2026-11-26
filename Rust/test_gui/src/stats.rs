#![allow(dead_code)]

use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct SystemStatsData {
    pub cpu_percent: f32,
    pub cpu_temp: String,
    pub cpu_freq_ghz: f32,
    pub cpu_cores: usize,
    pub cpu_model: String,
    pub top_cpu_process: String,

    pub ram_percent: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub top_ram_process: String,

    pub gpu_percent: f32,
    pub gpu_name: String,
    pub gpu_temp: String,
    pub vram_percent: f32,
    pub vram_used_gb: f32,
    pub vram_total_gb: f32,

    pub swap_percent: f32,
    pub swap_used_gb: f32,
    pub swap_total_gb: f32,

    pub disk_percent: f32,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub uptime_secs: u64,

    pub net_dl_bps: f64,
    pub net_ul_bps: f64,
    pub net_max_dl: f64,
    pub net_max_ul: f64,
    pub net_total_dl: u64,
    pub net_total_ul: u64,
}

pub struct StatsCollector {
    pub data: Arc<Mutex<SystemStatsData>>,
    pub hotspot_only: Arc<Mutex<bool>>,
    pub hotspot_iface_name: Arc<Mutex<String>>,
    pub reset_flag: Arc<Mutex<bool>>,
}

impl StatsCollector {
    pub fn new() -> Self {
        let data = Arc::new(Mutex::new(SystemStatsData::default()));
        let hotspot_only = Arc::new(Mutex::new(false));
        let hotspot_iface_name = Arc::new(Mutex::new(String::new()));
        let reset_flag = Arc::new(Mutex::new(false));

        let data_clone = Arc::clone(&data);
        let hotspot_only_clone = Arc::clone(&hotspot_only);
        let iface_clone = Arc::clone(&hotspot_iface_name);
        let reset_clone = Arc::clone(&reset_flag);

        let gpu_shared = Arc::new(Mutex::new(GpuStatsData::default()));
        let gpu_thread_clone = Arc::clone(&gpu_shared);

        // Dedicated GPU & VRAM sampler thread
        thread::spawn(move || {
            loop {
                let stats = Self::query_gpu_stats();
                *gpu_thread_clone.lock().unwrap() = stats;
                thread::sleep(Duration::from_millis(500));
            }
        });

        let gpu_reader_clone = Arc::clone(&gpu_shared);

        thread::spawn(move || {
            let mut sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            );
            let mut networks = Networks::new_with_refreshed_list();
            let mut disks = Disks::new_with_refreshed_list();

            let mut last_instant = Instant::now();
            let mut last_total_rx: u64 = 0;
            let mut last_total_tx: u64 = 0;
            let mut max_dl = 0.0;
            let mut max_ul = 0.0;
            let mut total_accum_dl: u64 = 0;
            let mut total_accum_ul: u64 = 0;

            let mut tick_counter = 0usize;
            let mut cached_cpu_model = String::new();
            let mut cached_top_cpu = "-".to_string();
            let mut cached_top_ram = "-".to_string();
            let mut cached_disk_pct = 0.0f32;
            let mut cached_disk_used = 0.0f32;
            let mut cached_disk_total = 0.0f32;

            // Initial Disk check
            if let Some(d) = disks.list().first() {
                let total = d.total_space() as f32 / (1024.0 * 1024.0 * 1024.0);
                let avail = d.available_space() as f32 / (1024.0 * 1024.0 * 1024.0);
                let used = (total - avail).max(0.0);
                cached_disk_total = total;
                cached_disk_used = used;
                cached_disk_pct = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
            }

            // CPU Name extraction
            if let Some(cpu) = sys.cpus().first() {
                let raw = cpu.brand().to_string();
                let clean = raw
                    .replace("(R)", "")
                    .replace("(TM)", "")
                    .replace("CPU", "")
                    .replace("Processor", "")
                    .replace("Intel", "")
                    .replace("Core", "")
                    .replace("AMD", "")
                    .replace("Ryzen", "")
                    .trim()
                    .to_string();
                cached_cpu_model = clean;
            }

            loop {
                let now = Instant::now();
                let dt = now.duration_since(last_instant).as_secs_f64().max(0.05);
                last_instant = now;

                // Handle manual stats reset request
                {
                    let mut rf = reset_clone.lock().unwrap();
                    if *rf {
                        max_dl = 0.0;
                        max_ul = 0.0;
                        total_accum_dl = 0;
                        total_accum_ul = 0;
                        *rf = false;
                    }
                }

                sys.refresh_cpu_usage();
                sys.refresh_memory();
                networks.refresh(tick_counter % 10 == 0);

                // CPU
                let cpu_pct = sys.global_cpu_usage();
                let cpu_cores = sys.cpus().len();
                let cpu_freq = sys.cpus().first().map(|c| c.frequency() as f32 / 1000.0).unwrap_or(0.0);

                // Memory & Swap
                let ram_total = sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
                let ram_used = sys.used_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
                let ram_pct = if ram_total > 0.0 { (ram_used / ram_total) * 100.0 } else { 0.0 };

                let swap_total = sys.total_swap() as f32 / (1024.0 * 1024.0 * 1024.0);
                let swap_used = sys.used_swap() as f32 / (1024.0 * 1024.0 * 1024.0);
                let swap_pct = if swap_total > 0.0 { (swap_used / swap_total) * 100.0 } else { 0.0 };

                // Process & Disk scanning every ~6 ticks (~3.0s) with minimal CPU+RAM specifics only
                tick_counter += 1;
                if tick_counter % 6 == 0 {
                    sys.refresh_processes_specifics(
                        sysinfo::ProcessesToUpdate::All,
                        true,
                        sysinfo::ProcessRefreshKind::nothing().with_cpu().with_memory(),
                    );
                    let mut max_proc_cpu = 0.0f32;
                    let mut max_proc_ram = 0u64;
                    let mut best_cpu_name = "-".to_string();
                    let mut best_ram_name = "-".to_string();

                    for (_, proc) in sys.processes() {
                        let name = proc.name().to_string_lossy().to_string();
                        if name == "System Idle Process" {
                            continue;
                        }
                        let cpu = proc.cpu_usage();
                        if cpu > max_proc_cpu {
                            max_proc_cpu = cpu;
                            best_cpu_name = name.clone();
                        }
                        let mem = proc.memory();
                        if mem > max_proc_ram {
                            max_proc_ram = mem;
                            best_ram_name = name;
                        }
                    }
                    cached_top_cpu = best_cpu_name;
                    cached_top_ram = best_ram_name;

                    disks.refresh(false);
                    if let Some(d) = disks.list().first() {
                        let total = d.total_space() as f32 / (1024.0 * 1024.0 * 1024.0);
                        let avail = d.available_space() as f32 / (1024.0 * 1024.0 * 1024.0);
                        let used = (total - avail).max(0.0);
                        cached_disk_total = total;
                        cached_disk_used = used;
                        cached_disk_pct = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
                    }
                }

                // Network
                let is_hotspot_only = *hotspot_only_clone.lock().unwrap();
                let target_iface = iface_clone.lock().unwrap().clone();

                let mut current_rx: u64 = 0;
                let mut current_tx: u64 = 0;

                for (name, data) in &networks {
                    if is_hotspot_only {
                        if !target_iface.is_empty() && name.contains(&target_iface) {
                            current_rx += data.total_received();
                            current_tx += data.total_transmitted();
                        }
                    } else {
                        current_rx += data.total_received();
                        current_tx += data.total_transmitted();
                    }
                }

                let dl_delta = if last_total_rx > 0 && current_rx >= last_total_rx {
                    current_rx - last_total_rx
                } else {
                    0
                };
                let ul_delta = if last_total_tx > 0 && current_tx >= last_total_tx {
                    current_tx - last_total_tx
                } else {
                    0
                };

                last_total_rx = current_rx;
                last_total_tx = current_tx;

                let dl_bps = (dl_delta as f64) / dt;
                let ul_bps = (ul_delta as f64) / dt;

                total_accum_dl += dl_delta;
                total_accum_ul += ul_delta;

                if dl_bps > max_dl { max_dl = dl_bps; }
                if ul_bps > max_ul { max_ul = ul_bps; }

                let uptime = System::uptime();
                let gpu_stats = gpu_reader_clone.lock().unwrap().clone();

                // Update shared data
                {
                    let mut d = data_clone.lock().unwrap();
                    d.cpu_percent = cpu_pct;
                    d.cpu_freq_ghz = cpu_freq;
                    d.cpu_cores = cpu_cores;
                    d.cpu_model = cached_cpu_model.clone();
                    d.top_cpu_process = cached_top_cpu.clone();

                    d.ram_percent = ram_pct;
                    d.ram_used_gb = ram_used;
                    d.ram_total_gb = ram_total;
                    d.top_ram_process = cached_top_ram.clone();

                    d.gpu_percent = gpu_stats.gpu_percent;
                    d.gpu_name = gpu_stats.gpu_name;
                    d.gpu_temp = gpu_stats.gpu_temp;
                    d.vram_percent = gpu_stats.vram_percent;
                    d.vram_used_gb = gpu_stats.vram_used_gb;
                    d.vram_total_gb = gpu_stats.vram_total_gb;

                    d.swap_percent = swap_pct;
                    d.swap_used_gb = swap_used;
                    d.swap_total_gb = swap_total;

                    d.disk_percent = cached_disk_pct;
                    d.disk_used_gb = cached_disk_used;
                    d.disk_total_gb = cached_disk_total;
                    d.uptime_secs = uptime;

                    d.net_dl_bps = dl_bps;
                    d.net_ul_bps = ul_bps;
                    d.net_max_dl = max_dl;
                    d.net_max_ul = max_ul;
                    d.net_total_dl = total_accum_dl;
                    d.net_total_ul = total_accum_ul;
                }

                thread::sleep(Duration::from_millis(500));
            }
        });

        Self {
            data,
            hotspot_only,
            hotspot_iface_name,
            reset_flag,
        }
    }

    pub fn get(&self) -> SystemStatsData {
        self.data.lock().unwrap().clone()
    }

    pub fn set_hotspot_only(&self, enable: bool) {
        *self.hotspot_only.lock().unwrap() = enable;
    }

    pub fn set_hotspot_interface(&self, iface: String) {
        *self.hotspot_iface_name.lock().unwrap() = iface;
    }

    pub fn reset_stats(&self) {
        *self.reset_flag.lock().unwrap() = true;
    }

    fn query_gpu_stats() -> GpuStatsData {
        // 1. Try nvidia-smi (works on Windows & Linux with NVIDIA GPUs)
        if let Some(gpu) = Self::query_nvidia_smi() {
            return gpu;
        }

        // 2. Linux fallback: DRM sysfs for AMD / Intel
        #[cfg(target_os = "linux")]
        {
            if let Some(gpu) = Self::query_linux_drm_gpu() {
                return gpu;
            }
        }

        GpuStatsData::default()
    }

    fn query_nvidia_smi() -> Option<GpuStatsData> {
        #[cfg(target_os = "windows")]
        use std::os::windows::process::CommandExt;
        #[cfg(target_os = "windows")]
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut cmd = std::process::Command::new("nvidia-smi");
        cmd.args([
            "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu,name",
            "--format=csv,noheader,nounits",
        ]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 5 {
                let gpu_pct = parts[0].parse::<f32>().unwrap_or(0.0);
                let vram_used_mb = parts[1].parse::<f32>().unwrap_or(0.0);
                let vram_total_mb = parts[2].parse::<f32>().unwrap_or(0.0);
                let gpu_temp = parts[3].to_string();
                let gpu_name = parts[4].to_string();

                let vram_used_gb = vram_used_mb / 1024.0;
                let vram_total_gb = vram_total_mb / 1024.0;
                let vram_pct = if vram_total_mb > 0.0 {
                    (vram_used_mb / vram_total_mb * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };

                return Some(GpuStatsData {
                    gpu_percent: gpu_pct,
                    gpu_name,
                    gpu_temp,
                    vram_percent: vram_pct,
                    vram_used_gb,
                    vram_total_gb,
                });
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    fn query_linux_drm_gpu() -> Option<GpuStatsData> {
        use std::fs;
        for card in &["/sys/class/drm/card0/device", "/sys/class/drm/card1/device"] {
            let busy_path = format!("{}/gpu_busy_percent", card);
            let used_path = format!("{}/mem_info_vram_used", card);
            let total_path = format!("{}/mem_info_vram_total", card);

            if let Ok(busy_s) = fs::read_to_string(&busy_path) {
                let gpu_pct = busy_s.trim().parse::<f32>().unwrap_or(0.0);
                let mut vram_used_gb = 0.0f32;
                let mut vram_total_gb = 0.0f32;
                let mut vram_pct = 0.0f32;

                if let (Ok(u_s), Ok(t_s)) = (fs::read_to_string(&used_path), fs::read_to_string(&total_path)) {
                    let u_bytes = u_s.trim().parse::<f32>().unwrap_or(0.0);
                    let t_bytes = t_s.trim().parse::<f32>().unwrap_or(0.0);
                    vram_used_gb = u_bytes / (1024.0 * 1024.0 * 1024.0);
                    vram_total_gb = t_bytes / (1024.0 * 1024.0 * 1024.0);
                    if vram_total_gb > 0.0 {
                        vram_pct = (vram_used_gb / vram_total_gb * 100.0).clamp(0.0, 100.0);
                    }
                }

                return Some(GpuStatsData {
                    gpu_percent: gpu_pct,
                    gpu_name: "AMD GPU".to_string(),
                    gpu_temp: String::new(),
                    vram_percent: vram_pct,
                    vram_used_gb,
                    vram_total_gb,
                });
            }
        }
        None
    }
}

#[derive(Clone, Debug, Default)]
pub struct GpuStatsData {
    pub gpu_percent: f32,
    pub gpu_name: String,
    pub gpu_temp: String,
    pub vram_percent: f32,
    pub vram_used_gb: f32,
    pub vram_total_gb: f32,
}
