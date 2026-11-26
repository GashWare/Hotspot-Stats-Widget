# File: benchmark_profiler.py
# Description: Rigorous CPU, RAM, and Thread profiling comparing the original Python widget and the native Rust widget.

import subprocess
import time
import os
import sys
import psutil
import json

def profile_process(name, cmd_args, cwd, duration=10):
    print(f"\n==================================================")
    print(f"Profiling: {name}")
    print(f"Command: {' '.join(cmd_args)}")
    print(f"Directory: {cwd}")
    print(f"==================================================")

    proc = subprocess.Popen(cmd_args, cwd=cwd)
    pid = proc.pid
    
    try:
        ps_proc = psutil.Process(pid)
    except Exception as e:
        print(f"Failed to attach to process: {e}")
        return None

    # Wait 2.5 seconds for full GUI initialization & settling
    time.sleep(2.5)
    
    cpu_samples = []
    rss_samples = []
    vms_samples = []
    thread_samples = []
    
    # Warmup cpu_percent
    try:
        ps_proc.cpu_percent(interval=None)
    except:
        pass

    end_time = time.time() + duration
    while time.time() < end_time:
        try:
            if not ps_proc.is_running() or ps_proc.status() == psutil.STATUS_ZOMBIE:
                break
            
            # Sample metrics
            cpu = ps_proc.cpu_percent(interval=0.4)
            mem = ps_proc.memory_info()
            rss_mb = mem.rss / (1024 * 1024)
            vms_mb = mem.vms / (1024 * 1024)
            threads = ps_proc.num_threads()
            
            cpu_samples.append(cpu)
            rss_samples.append(rss_mb)
            vms_samples.append(vms_mb)
            thread_samples.append(threads)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            break

    # Terminate process cleanly
    try:
        ps_proc.terminate()
        ps_proc.wait(timeout=2)
    except:
        try:
            ps_proc.kill()
        except:
            pass

    if not rss_samples:
        print("No samples gathered.")
        return None

    avg_cpu = sum(cpu_samples) / len(cpu_samples)
    peak_cpu = max(cpu_samples)
    avg_rss = sum(rss_samples) / len(rss_samples)
    peak_rss = max(rss_samples)
    avg_threads = sum(thread_samples) / len(thread_samples)

    result = {
        "name": name,
        "samples": len(rss_samples),
        "avg_cpu_pct": avg_cpu,
        "peak_cpu_pct": peak_cpu,
        "avg_rss_mb": avg_rss,
        "peak_rss_mb": peak_rss,
        "avg_threads": avg_threads,
    }
    print(f"Completed {name}: Avg RAM: {avg_rss:.2f} MB, Avg CPU: {avg_cpu:.2f}%, Threads: {avg_threads:.0f}")
    return result

if __name__ == "__main__":
    py_path = sys.executable
    
    # 1. Original Python Hotspot Widget
    orig_dir = r"C:\Users\User\Desktop\Python\Hotspot_Widget"
    orig_cmd = [
        py_path, "-c",
        "import sys, os; sys.path.insert(0, r'" + orig_dir + "'); "
        "import utils; utils.AdminHandler.is_admin = lambda: True; "
        "import customtkinter as ctk; ctk.set_appearance_mode('Dark'); "
        "import main; app = main.HotspotWidget(); app.mainloop()"
    ]
    orig_res = profile_process(
        "Original Python Widget (1 FPS Tkinter/psutil)",
        orig_cmd,
        orig_dir,
        duration=10
    )
    
    # 2. Enhanced Python Hotspot Widget
    enh_dir = r"c:\Users\User\Desktop\Python\Stats Widget\Python"
    enh_cmd = [
        py_path, "-c",
        "import sys, os; sys.path.insert(0, r'" + enh_dir + "'); "
        "import utils; utils.AdminHandler.is_admin = lambda: True; "
        "import customtkinter as ctk; ctk.set_appearance_mode('Dark'); "
        "import main; app = main.HotspotWidget(); app.mainloop()"
    ]
    enh_res = profile_process(
        "Enhanced Python Widget (CustomTkinter/Sniffer)",
        enh_cmd,
        enh_dir,
        duration=10
    )

    # 3. Native Rust Hotspot Widget (Fluid Animation Mode)
    rust_dir = r"c:\Users\User\Desktop\Python\Stats Widget\Rust"
    rust_exe = os.path.join(rust_dir, "hotspot_stats_widget.exe")
    rust_fluid_res = profile_process(
        "Native Rust Widget (Fluid Animation - GPU)",
        [rust_exe],
        rust_dir,
        duration=10
    )

    # 4. Native Rust Hotspot Widget (Standard 1s Interval Mode)
    # Write temporary config with fluid_graphs: false
    cfg_path = os.path.join(rust_dir, "widget_config.json")
    with open(cfg_path, "w") as f:
        json.dump({"fluid_graphs": False, "graph_interval_ms": 1000}, f)
    
    rust_std_res = profile_process(
        "Native Rust Widget (Standard 1s Mode - GPU)",
        [rust_exe],
        rust_dir,
        duration=10
    )
    # Restore fluid_graphs: true
    with open(cfg_path, "w") as f:
        json.dump({"fluid_graphs": True, "graph_interval_ms": 250}, f)

    # Print Comparison Table
    logical_cores = psutil.cpu_count(logical=True) or 1
    print("\n" + "="*95)
    print("                    EMPIRICAL PERFORMANCE BENCHMARK REPORT                    ")
    print(f"               (System Core Count: {logical_cores} Logical Cores / {psutil.cpu_count(logical=False)} Physical Cores)")
    print("="*95)
    header = f"{'Implementation':<42} | {'Avg RAM':<10} | {'Peak RAM':<9} | {'Core CPU %':<10} | {'TaskMgr %':<9} | {'Threads':<7}"
    print(header)
    print("-" * len(header))
    
    for r in [orig_res, enh_res, rust_fluid_res, rust_std_res]:
        if r:
            taskmgr_pct = r['avg_cpu_pct'] / logical_cores
            line = f"{r['name']:<42} | {r['avg_rss_mb']:>6.1f} MB  | {r['peak_rss_mb']:>5.1f} MB | {r['avg_cpu_pct']:>8.2f} % | {taskmgr_pct:>7.2f} % | {r['avg_threads']:>6.0f}"
            print(line)
    print("="*95)
