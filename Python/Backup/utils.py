import ctypes
from ctypes import windll, byref, Structure, c_long
import sys
import os
import subprocess
import time
from config import TASK_NAME

def log_error(msg):
    try:
        with open("debug_log.txt", "a") as f:
            f.write(f"{time.strftime('%Y-%m-%d %H:%M:%S')} - {msg}\n")
    except: pass
    print(msg)

# Stealth Mode
try:
    if os.name == 'nt':
        hwnd = ctypes.windll.kernel32.GetConsoleWindow()
        if hwnd != 0:
            # Uncomment next line to hide console in production
            # ctypes.windll.user32.ShowWindow(hwnd, 0)
            pass
except Exception as e:
    log_error(f"Stealth mode error: {e}")

try: ctypes.windll.shcore.SetProcessDpiAwareness(1)
except: pass 

class RECT(Structure):
    _fields_ = [("left", c_long), ("top", c_long), ("right", c_long), ("bottom", c_long)]

def get_work_area():
    rect = RECT()
    windll.user32.SystemParametersInfoW(48, 0, byref(rect), 0)
    return rect.left, rect.top, rect.right, rect.bottom

class AdminHandler:
    @staticmethod
    def is_admin():
        try: return ctypes.windll.shell32.IsUserAnAdmin()
        except: return False

    @staticmethod
    def elevate():
        print("DEBUG: Attempting to elevate privileges...")
        exe = sys.executable 
        script = f'"{os.path.abspath(sys.argv[0])}"'
        ctypes.windll.shell32.ShellExecuteW(None, "runas", exe, script, None, 1)
        sys.exit()

    @staticmethod
    def check_task_exists():
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        cmd = f'schtasks /query /tn "{TASK_NAME}"'
        result = subprocess.run(cmd, capture_output=True, shell=True, startupinfo=startupinfo)
        return result.returncode == 0

    @staticmethod
    def create_startup_task():
        exe = sys.executable.replace("python.exe", "pythonw.exe") if "python.exe" in sys.executable else sys.executable
        script = os.path.abspath(sys.argv[0])
        startupinfo = subprocess.STARTUPINFO()
        startupinfo.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        subprocess.run(f'schtasks /delete /tn "{TASK_NAME}" /f', shell=True, startupinfo=startupinfo)
        cmd = (f'schtasks /create /tn "{TASK_NAME}" /tr "\'{exe}\' \'{script}\'" /sc onlogon /rl highest /f')
        subprocess.run(cmd, shell=True, startupinfo=startupinfo)