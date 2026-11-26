import subprocess
import time
import psutil
import os

exe = r"c:\Users\User\Desktop\Python\Stats Widget\Rust\hotspot_stats_widget.exe"
cwd = r"c:\Users\User\Desktop\Python\Stats Widget\Rust"

proc = subprocess.Popen([exe], cwd=cwd)
pid = proc.pid
time.sleep(3.0)

ps_proc = psutil.Process(pid)
print(f"Profiling PID {pid}...")

# Sample thread CPU times
threads_before = {t.id: (t.user_time, t.system_time) for t in ps_proc.threads()}
time.sleep(3.0)
threads_after = {t.id: (t.user_time, t.system_time) for t in ps_proc.threads()}

total_proc_cpu = ps_proc.cpu_percent()
print(f"Overall Process CPU: {total_proc_cpu}%")
print(f"Threads count: {len(threads_after)}")
print("Top CPU consuming threads:")

deltas = []
for tid, (u1, s1) in threads_before.items():
    if tid in threads_after:
        u2, s2 = threads_after[tid]
        total_time = (u2 - u1) + (s2 - s1)
        deltas.append((tid, total_time, (u2 - u1), (s2 - s1)))

deltas.sort(key=lambda x: x[1], reverse=True)
for tid, total_t, u_t, s_t in deltas[:8]:
    pct = (total_t / 3.0) * 100.0
    print(f"Thread ID {tid}: Total={total_t:.3f}s ({pct:.2f}% of one core) [User={u_t:.3f}s, Sys={s_t:.3f}s]")

proc.terminate()
