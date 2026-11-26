//! RAM optimizer: trims the working set of every accessible process (the same
//! operation as the PowerShell `psapi.dll!EmptyWorkingSet` script), runs on a
//! background thread and exposes progress/results for animated UI feedback.

use crate::logger::{log_info, log_success, log_warn};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanPhase {
    Idle,
    Running,
    Done,
}

#[derive(Clone, Debug)]
pub struct CleanStatus {
    pub phase: CleanPhase,
    pub started: Instant,
    pub finished: Option<Instant>,
    /// Sum of per-process working-set reductions (bytes).
    pub trimmed_bytes: u64,
    /// Change in system "available physical memory" (bytes, can be negative).
    pub avail_gain_bytes: i64,
    pub processes_trimmed: u32,
    pub processes_denied: u32,
    pub message: String,
}

impl Default for CleanStatus {
    fn default() -> Self {
        Self {
            phase: CleanPhase::Idle,
            started: Instant::now(),
            finished: None,
            trimmed_bytes: 0,
            avail_gain_bytes: 0,
            processes_trimmed: 0,
            processes_denied: 0,
            message: String::new(),
        }
    }
}

impl CleanStatus {
    /// Best user-facing "freed" figure in bytes.
    pub fn freed_bytes(&self) -> u64 {
        self.trimmed_bytes.max(self.avail_gain_bytes.max(0) as u64)
    }
}

pub struct MemoryCleaner {
    status: Arc<Mutex<CleanStatus>>,
}

impl MemoryCleaner {
    pub fn new() -> Self {
        Self { status: Arc::new(Mutex::new(CleanStatus::default())) }
    }

    pub fn status(&self) -> CleanStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_running(&self) -> bool {
        self.status.lock().unwrap().phase == CleanPhase::Running
    }

    /// Starts the memory clearing sequence (no-op if one is already running).
    pub fn start(&self) {
        {
            let mut st = self.status.lock().unwrap();
            if st.phase == CleanPhase::Running {
                return;
            }
            *st = CleanStatus { phase: CleanPhase::Running, started: Instant::now(), ..Default::default() };
        }
        let status = Arc::clone(&self.status);
        thread::spawn(move || {
            log_info("RAM", "Reclaiming working set memory (EmptyWorkingSet on all accessible processes)...");
            let started = Instant::now();
            let result = run_clean();
            // Keep the "running" animation visible for at least ~0.9s so it reads as an action.
            let min_anim = Duration::from_millis(900);
            let el = started.elapsed();
            if el < min_anim {
                thread::sleep(min_anim - el);
            }
            let mut st = status.lock().unwrap();
            st.trimmed_bytes = result.trimmed_bytes;
            st.avail_gain_bytes = result.avail_gain_bytes;
            st.processes_trimmed = result.trimmed;
            st.processes_denied = result.denied;
            st.message = result.message.clone();
            st.phase = CleanPhase::Done;
            st.finished = Some(Instant::now());
            if result.supported {
                log_success(
                    "RAM",
                    &format!(
                        "Working sets trimmed: {} processes ({} skipped/protected), ~{:.1} MB reclaimed, available RAM change {:+.1} MB",
                        result.trimmed,
                        result.denied,
                        result.trimmed_bytes as f64 / 1_048_576.0,
                        result.avail_gain_bytes as f64 / 1_048_576.0
                    ),
                );
            } else {
                log_warn("RAM", &result.message);
            }
        });
    }
}

struct CleanResult {
    supported: bool,
    trimmed: u32,
    denied: u32,
    trimmed_bytes: u64,
    avail_gain_bytes: i64,
    message: String,
}

#[cfg(target_os = "windows")]
fn run_clean() -> CleanResult {
    use std::ffi::c_void;

    #[repr(C)]
    struct MemoryStatusEx {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn CloseHandle(h: *mut c_void) -> i32;
        fn GetCurrentProcessId() -> u32;
        fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
        fn K32EnumProcesses(pids: *mut u32, cb: u32, needed: *mut u32) -> i32;
        fn K32EmptyWorkingSet(h: *mut c_void) -> i32;
        fn K32GetProcessMemoryInfo(h: *mut c_void, pmc: *mut ProcessMemoryCounters, cb: u32) -> i32;
    }

    const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_SET_QUOTA: u32 = 0x0100;
    const PROCESS_VM_READ: u32 = 0x0010;

    let avail_phys = || -> u64 {
        let mut ms = MemoryStatusEx {
            dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };
        unsafe {
            if GlobalMemoryStatusEx(&mut ms) != 0 { ms.ull_avail_phys } else { 0 }
        }
    };

    let before_avail = avail_phys();

    // Enumerate PIDs
    let mut pids: Vec<u32> = vec![0; 4096];
    let mut needed: u32 = 0;
    unsafe {
        loop {
            let cb = (pids.len() * 4) as u32;
            if K32EnumProcesses(pids.as_mut_ptr(), cb, &mut needed) == 0 {
                needed = 0;
                break;
            }
            if needed < cb {
                break;
            }
            pids.resize(pids.len() * 2, 0);
        }
    }
    pids.truncate((needed / 4) as usize);

    let self_pid = unsafe { GetCurrentProcessId() };
    let mut trimmed = 0u32;
    let mut denied = 0u32;
    let mut trimmed_bytes: u64 = 0;
    let pmc_size = std::mem::size_of::<ProcessMemoryCounters>() as u32;

    for &pid in &pids {
        if pid == 0 || pid == 4 || pid == self_pid {
            continue;
        }
        unsafe {
            let mut h = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA | PROCESS_VM_READ, 0, pid);
            if h.is_null() {
                h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA, 0, pid);
            }
            if h.is_null() {
                denied += 1;
                continue;
            }
            let mut pmc = ProcessMemoryCounters { cb: pmc_size, ..Default::default() };
            let ws_before = if K32GetProcessMemoryInfo(h, &mut pmc, pmc_size) != 0 { pmc.working_set_size as u64 } else { 0 };

            if K32EmptyWorkingSet(h) != 0 {
                trimmed += 1;
                let mut pmc2 = ProcessMemoryCounters { cb: pmc_size, ..Default::default() };
                if ws_before > 0 && K32GetProcessMemoryInfo(h, &mut pmc2, pmc_size) != 0 {
                    trimmed_bytes += ws_before.saturating_sub(pmc2.working_set_size as u64);
                }
            } else {
                denied += 1;
            }
            CloseHandle(h);
        }
    }

    // Let the memory manager settle before measuring system-wide availability
    thread::sleep(Duration::from_millis(400));
    let after_avail = avail_phys();

    CleanResult {
        supported: true,
        trimmed,
        denied,
        trimmed_bytes,
        avail_gain_bytes: after_avail as i64 - before_avail as i64,
        message: String::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn run_clean() -> CleanResult {
    // On Linux, dropping page caches requires root. Try it; otherwise report unsupported.
    let read_avail = || -> u64 {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("MemAvailable:"))
                    .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<u64>().ok()))
            })
            .map(|kb| kb * 1024)
            .unwrap_or(0)
    };
    let before = read_avail();
    let _ = std::process::Command::new("sync").status();
    let ok = std::fs::write("/proc/sys/vm/drop_caches", "1").is_ok();
    thread::sleep(Duration::from_millis(400));
    let after = read_avail();
    CleanResult {
        supported: ok,
        trimmed: 0,
        denied: 0,
        trimmed_bytes: 0,
        avail_gain_bytes: after as i64 - before as i64,
        message: if ok { String::new() } else { "RAM purge requires root on Linux".to_string() },
    }
}
