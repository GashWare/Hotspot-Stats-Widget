#![allow(dead_code)]

use std::process::Command;

pub const TASK_NAME: &str = "NetMonitorAutoStart";

/// Returns the usable desktop work area (left, top, right, bottom) excluding the taskbar.
pub fn get_work_area() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct RECT {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SystemParametersInfoW(
                uiAction: u32,
                uiParam: u32,
                pvParam: *mut RECT,
                fWinIni: u32,
            ) -> i32;
            fn GetDC(hWnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn ReleaseDC(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void) -> i32;
        }
        #[link(name = "gdi32")]
        unsafe extern "system" {
            fn GetDeviceCaps(hdc: *mut std::ffi::c_void, index: i32) -> i32;
        }

        const SPI_GETWORKAREA: u32 = 0x0030;
        const LOGPIXELSX: i32 = 88;

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        unsafe {
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect, 0);
            let hdc = GetDC(std::ptr::null_mut());
            let dpi = if !hdc.is_null() {
                let d = GetDeviceCaps(hdc, LOGPIXELSX);
                ReleaseDC(std::ptr::null_mut(), hdc);
                if d > 0 { d as f32 } else { 96.0 }
            } else {
                96.0
            };
            let scale = dpi / 96.0;
            return (
                rect.left as f32 / scale,
                rect.top as f32 / scale,
                rect.right as f32 / scale,
                rect.bottom as f32 / scale,
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0.0, 0.0, 1920.0, 1040.0)
    }
}

/// Returns the primary monitor resolution (width, height) in logical points.
pub fn get_screen_resolution() -> (f32, f32) {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
            fn GetDC(hWnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn ReleaseDC(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void) -> i32;
        }
        #[link(name = "gdi32")]
        unsafe extern "system" {
            fn GetDeviceCaps(hdc: *mut std::ffi::c_void, index: i32) -> i32;
        }

        const LOGPIXELSX: i32 = 88;
        const SM_CXSCREEN: i32 = 0;
        const SM_CYSCREEN: i32 = 1;

        unsafe {
            let hdc = GetDC(std::ptr::null_mut());
            let dpi = if !hdc.is_null() {
                let d = GetDeviceCaps(hdc, LOGPIXELSX);
                ReleaseDC(std::ptr::null_mut(), hdc);
                if d > 0 { d as f32 } else { 96.0 }
            } else {
                96.0
            };
            let scale = dpi / 96.0;
            let w = GetSystemMetrics(SM_CXSCREEN) as f32 / scale;
            let h = GetSystemMetrics(SM_CYSCREEN) as f32 / scale;
            (w, h)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        (1920.0, 1080.0)
    }
}

/// Applies true OS-level whole-window alpha transparency (0.20 ..= 1.0) to the NetMonitor window.
pub fn apply_window_opacity(opacity: f32) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        #[link(name = "user32")]
        unsafe extern "system" {
            fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut c_void;
            fn GetWindowLongW(hWnd: *mut c_void, nIndex: i32) -> i32;
            fn SetWindowLongW(hWnd: *mut c_void, nIndex: i32, dwNewLong: i32) -> i32;
            fn SetLayeredWindowAttributes(
                hwnd: *mut c_void,
                crKey: u32,
                bAlpha: u8,
                dwFlags: u32,
            ) -> i32;
        }

        const GWL_EXSTYLE: i32 = -20;
        const WS_EX_LAYERED: i32 = 0x00080000;
        const LWA_ALPHA: u32 = 0x00000002;

        let title: Vec<u16> = "NetMonitor\0".encode_utf16().collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if !hwnd.is_null() {
                let alpha = (opacity.clamp(0.20, 1.0) * 255.0) as u8;
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                if (ex_style & WS_EX_LAYERED) == 0 {
                    SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED);
                }
                SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = opacity;
    }
}

/// Ensures that a window (such as the taskbar micro-gauges) stays topmost
/// and has the tool-window attribute so it is not clipped or occluded by the taskbar.
pub fn pin_taskbar_window_topmost(title: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        #[link(name = "user32")]
        unsafe extern "system" {
            fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> *mut c_void;
            fn GetWindowLongW(hWnd: *mut c_void, nIndex: i32) -> i32;
            fn SetWindowLongW(hWnd: *mut c_void, nIndex: i32, dwNewLong: i32) -> i32;
            fn SetWindowPos(
                hWnd: *mut c_void,
                hWndInsertAfter: *mut c_void,
                X: i32,
                Y: i32,
                cx: i32,
                cy: i32,
                uFlags: u32,
            ) -> i32;
        }

        const GWL_EXSTYLE: i32 = -20;
        const WS_EX_TOOLWINDOW: i32 = 0x00000080;
        const WS_EX_TOPMOST: i32 = 0x00000008;
        const HWND_TOPMOST: *mut c_void = -1isize as *mut c_void;
        const SWP_NOSIZE: u32 = 0x0001;
        const SWP_NOMOVE: u32 = 0x0002;
        const SWP_NOACTIVATE: u32 = 0x0010;

        let w_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), w_title.as_ptr());
            if !hwnd.is_null() {
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                if (ex_style & WS_EX_TOOLWINDOW) == 0 || (ex_style & WS_EX_TOPMOST) == 0 {
                    SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TOOLWINDOW | WS_EX_TOPMOST);
                }
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = title;
    }
}

/// Applies a full-screen hardware overlay dimmer across the primary monitor.
/// `dim_percent` is 0 (no dimming) up to 90 (very dark night mode).
pub fn set_screen_dimmer(dim_percent: u8) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        use std::sync::atomic::{AtomicPtr, Ordering};

        static DIMMER_HWND: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

        #[repr(C)]
        struct WNDCLASSEXW {
            cb_size: u32,
            style: u32,
            lpfn_wnd_proc: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
            cb_cls_extra: i32,
            cb_wnd_extra: i32,
            h_instance: *mut c_void,
            h_icon: *mut c_void,
            h_cursor: *mut c_void,
            h_br_background: *mut c_void,
            lpsz_menu_name: *const u16,
            lpsz_class_name: *const u16,
            h_icon_sm: *mut c_void,
        }

        #[link(name = "user32")]
        unsafe extern "system" {
            fn DefWindowProcW(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
            fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> u16;
            fn CreateWindowExW(
                dwExStyle: u32,
                lpClassName: *const u16,
                lpWindowName: *const u16,
                dwStyle: u32,
                X: i32,
                Y: i32,
                nWidth: i32,
                nHeight: i32,
                hWndParent: *mut c_void,
                hMenu: *mut c_void,
                hInstance: *mut c_void,
                lpParam: *mut c_void,
            ) -> *mut c_void;
            fn ShowWindow(hWnd: *mut c_void, nCmdShow: i32) -> i32;
            fn DestroyWindow(hWnd: *mut c_void) -> i32;
            fn GetSystemMetrics(nIndex: i32) -> i32;
            fn SetWindowPos(
                hWnd: *mut c_void,
                hWndInsertAfter: *mut c_void,
                X: i32,
                Y: i32,
                cx: i32,
                cy: i32,
                uFlags: u32,
            ) -> i32;
            fn SetLayeredWindowAttributes(
                hwnd: *mut c_void,
                crKey: u32,
                bAlpha: u8,
                dwFlags: u32,
            ) -> i32;
        }

        #[link(name = "gdi32")]
        unsafe extern "system" {
            fn GetStockObject(i: i32) -> *mut c_void;
        }

        unsafe extern "system" fn dimmer_wnd_proc(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        let dim = dim_percent.min(90);
        let mut hwnd = DIMMER_HWND.load(Ordering::SeqCst);

        if dim == 0 {
            if !hwnd.is_null() {
                unsafe {
                    ShowWindow(hwnd, 0); // SW_HIDE
                    DestroyWindow(hwnd);
                }
                DIMMER_HWND.store(std::ptr::null_mut(), Ordering::SeqCst);
            }
            return;
        }

        unsafe {
            const BLACK_BRUSH: i32 = 4;
            const WS_EX_LAYERED: u32 = 0x00080000;
            const WS_EX_TRANSPARENT: u32 = 0x00000020;
            const WS_EX_TOOLWINDOW: u32 = 0x00000080;
            const WS_EX_NOACTIVATE: u32 = 0x08000000;
            const WS_POPUP: u32 = 0x80000000;
            const SW_SHOWNOACTIVATE: i32 = 4;
            const LWA_ALPHA: u32 = 0x00000002;
            const HWND_TOPMOST: *mut c_void = -1isize as *mut c_void;
            const SWP_NOSIZE: u32 = 0x0001;
            const SWP_NOMOVE: u32 = 0x0002;
            const SWP_NOACTIVATE: u32 = 0x0010;
            const SM_CXSCREEN: i32 = 0;
            const SM_CYSCREEN: i32 = 1;

            let class_name: Vec<u16> = "NetMonitorScreenDimmerClass\0".encode_utf16().collect();
            let win_name: Vec<u16> = "NetMonitorScreenDimmer\0".encode_utf16().collect();

            if hwnd.is_null() {
                let wnd_class = WNDCLASSEXW {
                    cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: 0,
                    lpfn_wnd_proc: dimmer_wnd_proc,
                    cb_cls_extra: 0,
                    cb_wnd_extra: 0,
                    h_instance: std::ptr::null_mut(),
                    h_icon: std::ptr::null_mut(),
                    h_cursor: std::ptr::null_mut(),
                    h_br_background: GetStockObject(BLACK_BRUSH),
                    lpsz_menu_name: std::ptr::null(),
                    lpsz_class_name: class_name.as_ptr(),
                    h_icon_sm: std::ptr::null_mut(),
                };
                let _ = RegisterClassExW(&wnd_class);

                let scr_w = GetSystemMetrics(SM_CXSCREEN).max(800);
                let scr_h = GetSystemMetrics(SM_CYSCREEN).max(600);

                let ex_style = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
                let new_hwnd = CreateWindowExW(
                    ex_style,
                    class_name.as_ptr(),
                    win_name.as_ptr(),
                    WS_POPUP,
                    0,
                    0,
                    scr_w,
                    scr_h,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );

                if !new_hwnd.is_null() {
                    DIMMER_HWND.store(new_hwnd, Ordering::SeqCst);
                    hwnd = new_hwnd;
                }
            }

            if !hwnd.is_null() {
                let alpha = ((dim as f32 / 100.0) * 255.0) as u8;
                SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = dim_percent;
    }
}

/// Checks if the current process is running with Administrator / Root privileges.
pub fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn IsUserAnAdmin() -> i32;
        }
        unsafe { IsUserAnAdmin() != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(out) = Command::new("id").arg("-u").output() {
            String::from_utf8_lossy(&out.stdout).trim() == "0"
        } else {
            false
        }
    }
}

/// Relaunches the executable as Administrator via Windows UAC (`runas`).
pub fn elevate_self() {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn ShellExecuteW(
                hwnd: *mut c_void,
                lpOperation: *const u16,
                lpFile: *const u16,
                lpParameters: *const u16,
                lpDirectory: *const u16,
                nShowCmd: i32,
            ) -> isize;
        }

        if let Ok(exe) = std::env::current_exe() {
            let op: Vec<u16> = "runas\0".encode_utf16().collect();
            let file: Vec<u16> = format!("{}\0", exe.display()).encode_utf16().collect();
            let dir: Vec<u16> = exe
                .parent()
                .map(|p| format!("{}\0", p.display()).encode_utf16().collect())
                .unwrap_or_default();
            let dir_ptr = if dir.is_empty() {
                std::ptr::null()
            } else {
                dir.as_ptr()
            };
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    op.as_ptr(),
                    file.as_ptr(),
                    std::ptr::null(),
                    dir_ptr,
                    1,
                );
                if res > 32 {
                    std::process::exit(0);
                }
            }
        }
    }
}

/// Checks whether "Start on System Startup" is currently registered.
pub fn check_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // 1. Check Scheduled Task
        if let Ok(out) = Command::new("schtasks")
            .args(["/query", "/tn", TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            if out.status.success() {
                return true;
            }
        }

        // 2. Check HKCU Run Registry Key
        if let Ok(out) = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                TASK_NAME,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            if out.status.success() {
                return true;
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let desktop_path = std::path::Path::new(&home)
                .join(".config/autostart/netmonitor.desktop");
            desktop_path.exists()
        } else {
            false
        }
    }
}

/// Enables or disables "Start on System Startup" across Windows (Scheduled Task + HKCU Run) and Linux (XDG Autostart).
pub fn set_autostart_enabled(enable: bool) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let exe_path = match std::env::current_exe() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => return false,
        };

        if enable {
            // Try highest-privilege Scheduled Task first (works when running as Admin)
            let tr_arg = format!("\"{}\"", exe_path);
            let _ = Command::new("schtasks")
                .args(["/delete", "/tn", TASK_NAME, "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            let task_ok = Command::new("schtasks")
                .args([
                    "/create",
                    "/tn",
                    TASK_NAME,
                    "/tr",
                    &tr_arg,
                    "/sc",
                    "onlogon",
                    "/rl",
                    "highest",
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if task_ok {
                return true;
            }

            // Fallback: Register in HKCU\Software\Microsoft\Windows\CurrentVersion\Run (works without Admin)
            Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    TASK_NAME,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &tr_arg,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            let _ = Command::new("schtasks")
                .args(["/delete", "/tn", TASK_NAME, "/f"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let _ = Command::new("reg")
                .args([
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    TASK_NAME,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            true
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let autostart_dir = std::path::Path::new(&home).join(".config/autostart");
            let desktop_path = autostart_dir.join("netmonitor.desktop");
            if enable {
                let _ = std::fs::create_dir_all(&autostart_dir);
                let exe_path = std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "hotspot_stats_widget".to_string());
                let content = format!(
                    "[Desktop Entry]\nType=Application\nName=NetMonitor Hotspot & Stats Widget\nExec=\"{}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
                    exe_path
                );
                std::fs::write(desktop_path, content).is_ok()
            } else {
                let _ = std::fs::remove_file(desktop_path);
                true
            }
        } else {
            false
        }
    }
}
