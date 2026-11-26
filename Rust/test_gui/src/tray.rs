#![allow(dead_code)]

use eframe::egui::IconData;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    ToggleHotspot,
    OpenInspector,
    Exit,
}

/// Generates a crisp 64x64 anti-aliased cyberpunk radar/hotspot RGBA icon.
pub fn create_cyberpunk_icon(hotspot_on: bool, has_traffic: bool) -> IconData {
    let size = 64u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32 - 1.0) / 2.0;

    // Primary ring color: Neon Green when Hotspot ON, Neon Cyan otherwise
    let (r_main, g_main, b_main) = if hotspot_on {
        (0u8, 255u8, 136u8)
    } else {
        (0u8, 255u8, 204u8)
    };

    // Core pulse color
    let (r_core, g_core, b_core) = if has_traffic {
        (0u8, 255u8, 204u8)
    } else if hotspot_on {
        (0u8, 220u8, 110u8)
    } else {
        (70u8, 90u8, 100u8)
    };

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;

            if dist <= 30.5 {
                // Dark cyberpunk badge background
                let bg_alpha = ((30.5 - dist).clamp(0.0, 1.5) / 1.5 * 245.0) as u8;
                rgba[idx] = 18;
                rgba[idx + 1] = 22;
                rgba[idx + 2] = 28;
                rgba[idx + 3] = bg_alpha;

                // Outer glowing ring (radius 26.5..29.5)
                if (26.0..=29.8).contains(&dist) {
                    let ring_t = 1.0 - ((dist - 27.9).abs() / 1.9).clamp(0.0, 1.0);
                    rgba[idx] = ((18.0 * (1.0 - ring_t)) + (r_main as f32 * ring_t)) as u8;
                    rgba[idx + 1] = ((22.0 * (1.0 - ring_t)) + (g_main as f32 * ring_t)) as u8;
                    rgba[idx + 2] = ((28.0 * (1.0 - ring_t)) + (b_main as f32 * ring_t)) as u8;
                    rgba[idx + 3] = 255;
                }

                // Middle radar wave arc (radius 17.5..20.5, upper & lower signal waves)
                if (17.2..=20.4).contains(&dist) && dy.abs() > dx.abs() * 0.35 {
                    let wave_t = 1.0 - ((dist - 18.8).abs() / 1.6).clamp(0.0, 1.0);
                    let intensity = wave_t * if hotspot_on { 0.95 } else { 0.65 };
                    rgba[idx] = ((rgba[idx] as f32 * (1.0 - intensity)) + (r_main as f32 * intensity)) as u8;
                    rgba[idx + 1] = ((rgba[idx + 1] as f32 * (1.0 - intensity)) + (g_main as f32 * intensity)) as u8;
                    rgba[idx + 2] = ((rgba[idx + 2] as f32 * (1.0 - intensity)) + (b_main as f32 * intensity)) as u8;
                }

                // Inner radar wave arc (radius 10.5..13.2)
                if (10.5..=13.2).contains(&dist) && dy.abs() > dx.abs() * 0.25 {
                    let wave_t = 1.0 - ((dist - 11.85).abs() / 1.35).clamp(0.0, 1.0);
                    let intensity = wave_t * if has_traffic { 1.0 } else { 0.75 };
                    rgba[idx] = ((rgba[idx] as f32 * (1.0 - intensity)) + (r_main as f32 * intensity)) as u8;
                    rgba[idx + 1] = ((rgba[idx + 1] as f32 * (1.0 - intensity)) + (g_main as f32 * intensity)) as u8;
                    rgba[idx + 2] = ((rgba[idx + 2] as f32 * (1.0 - intensity)) + (b_main as f32 * intensity)) as u8;
                }

                // Center active transmitter core (radius <= 6.2)
                if dist <= 6.2 {
                    let core_t = ((6.2 - dist) / 1.5).clamp(0.0, 1.0);
                    rgba[idx] = ((rgba[idx] as f32 * (1.0 - core_t)) + (r_core as f32 * core_t)) as u8;
                    rgba[idx + 1] = ((rgba[idx + 1] as f32 * (1.0 - core_t)) + (g_core as f32 * core_t)) as u8;
                    rgba[idx + 2] = ((rgba[idx + 2] as f32 * (1.0 - core_t)) + (b_core as f32 * core_t)) as u8;
                    rgba[idx + 3] = 255;
                }
            }
        }
    }

    IconData {
        rgba,
        width: size,
        height: size,
    }
}

pub struct SystemTray {
    pub actions: Arc<Mutex<VecDeque<TrayAction>>>,
    last_state: (bool, bool),
    #[cfg(target_os = "windows")]
    hwnd_shared: Arc<Mutex<usize>>,
}

impl SystemTray {
    pub fn new() -> Self {
        let actions = Arc::new(Mutex::new(VecDeque::new()));
        #[cfg(target_os = "windows")]
        let hwnd_shared = Arc::new(Mutex::new(0usize));

        #[cfg(target_os = "windows")]
        {
            let actions_clone = Arc::clone(&actions);
            let hwnd_clone = Arc::clone(&hwnd_shared);
            win32_tray::spawn_tray_thread(actions_clone, hwnd_clone);
        }

        Self {
            actions,
            last_state: (false, false),
            #[cfg(target_os = "windows")]
            hwnd_shared,
        }
    }

    pub fn poll_action(&self) -> Option<TrayAction> {
        self.actions.lock().unwrap().pop_front()
    }

    /// Updates the tray icon if hotspot state or traffic state changed.
    pub fn update_status(&mut self, hotspot_on: bool, has_traffic: bool) -> Option<Arc<IconData>> {
        if self.last_state == (hotspot_on, has_traffic) {
            return None;
        }
        self.last_state = (hotspot_on, has_traffic);
        let icon = create_cyberpunk_icon(hotspot_on, has_traffic);

        #[cfg(target_os = "windows")]
        {
            let hwnd = *self.hwnd_shared.lock().unwrap();
            if hwnd != 0 {
                win32_tray::update_tray_icon(hwnd, &icon, hotspot_on);
            }
        }

        Some(Arc::new(icon))
    }
}

#[cfg(target_os = "windows")]
impl Drop for SystemTray {
    fn drop(&mut self) {
        let hwnd = *self.hwnd_shared.lock().unwrap();
        if hwnd != 0 {
            win32_tray::remove_tray_icon(hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_tray {
    use super::{create_cyberpunk_icon, TrayAction};
    use eframe::egui::IconData;
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::sync::{Arc, Mutex, OnceLock};

    const WM_USER: u32 = 0x0400;
    const WM_TRAYICON: u32 = WM_USER + 42;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_LBUTTONDBLCLK: u32 = 0x0203;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WM_COMMAND: u32 = 0x0111;
    const WM_DESTROY: u32 = 0x0002;

    const NIM_ADD: u32 = 0x00000000;
    const NIM_MODIFY: u32 = 0x00000001;
    const NIM_DELETE: u32 = 0x00000002;
    const NIF_MESSAGE: u32 = 0x00000001;
    const NIF_ICON: u32 = 0x00000002;
    const NIF_TIP: u32 = 0x00000004;

    const TPM_BOTTOMALIGN: u32 = 0x0020;
    const TPM_LEFTALIGN: u32 = 0x0000;
    const TPM_RETURNCMD: u32 = 0x0100;
    const TPM_NONOTIFY: u32 = 0x0080;
    const MF_STRING: u32 = 0x00000000;
    const MF_SEPARATOR: u32 = 0x00000800;

    const ID_SHOW: usize = 1001;
    const ID_TOGGLE_HS: usize = 1002;
    const ID_INSPECTOR: usize = 1003;
    const ID_EXIT: usize = 1004;

    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct MSG {
        hwnd: *mut c_void,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: POINT,
    }

    #[repr(C)]
    struct WNDCLASSW {
        style: u32,
        lpfn_wnd_proc: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: *mut c_void,
        h_icon: *mut c_void,
        h_cursor: *mut c_void,
        hbr_background: *mut c_void,
        lpsz_menu_name: *const u16,
        lpsz_class_name: *const u16,
    }

    #[repr(C)]
    struct ICONINFO {
        f_icon: i32,
        x_hotspot: u32,
        y_hotspot: u32,
        hbm_mask: *mut c_void,
        hbm_color: *mut c_void,
    }

    #[repr(C)]
    struct NOTIFYICONDATAW {
        cb_size: u32,
        h_wnd: *mut c_void,
        u_id: u32,
        u_flags: u32,
        u_callback_message: u32,
        h_icon: *mut c_void,
        sz_tip: [u16; 128],
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> u16;
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
        fn DefWindowProcW(hWnd: *mut c_void, Msg: u32, wParam: usize, lParam: isize) -> isize;
        fn GetMessageW(lpMsg: *mut MSG, hWnd: *mut c_void, wMsgFilterMin: u32, wMsgFilterMax: u32) -> i32;
        fn TranslateMessage(lpMsg: *const MSG) -> i32;
        fn DispatchMessageW(lpMsg: *const MSG) -> isize;
        fn CreateIconIndirect(piconinfo: *const ICONINFO) -> *mut c_void;
        fn DestroyIcon(hIcon: *mut c_void) -> i32;
        fn CreatePopupMenu() -> *mut c_void;
        fn AppendMenuW(hMenu: *mut c_void, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> i32;
        fn TrackPopupMenu(
            hMenu: *mut c_void,
            uFlags: u32,
            x: i32,
            y: i32,
            nReserved: i32,
            hWnd: *mut c_void,
            prcRect: *const c_void,
        ) -> i32;
        fn DestroyMenu(hMenu: *mut c_void) -> i32;
        fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        fn SetForegroundWindow(hWnd: *mut c_void) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateBitmap(nWidth: i32, nHeight: i32, nPlanes: u32, nBitCount: u32, lpBits: *const c_void) -> *mut c_void;
        fn DeleteObject(ho: *mut c_void) -> i32;
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
    }

    static GLOBAL_ACTIONS: OnceLock<Arc<Mutex<VecDeque<TrayAction>>>> = OnceLock::new();

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn icon_data_to_hicon(icon: &IconData) -> *mut c_void {
        let mut bgra = Vec::with_capacity(icon.rgba.len());
        for chunk in icon.rgba.chunks_exact(4) {
            bgra.push(chunk[2]); // B
            bgra.push(chunk[1]); // G
            bgra.push(chunk[0]); // R
            bgra.push(chunk[3]); // A
        }
        let hbm_color = unsafe {
            CreateBitmap(
                icon.width as i32,
                icon.height as i32,
                1,
                32,
                bgra.as_ptr() as *const c_void,
            )
        };
        let mask_bytes = vec![0u8; ((icon.width * icon.height) / 8) as usize];
        let hbm_mask = unsafe {
            CreateBitmap(
                icon.width as i32,
                icon.height as i32,
                1,
                1,
                mask_bytes.as_ptr() as *const c_void,
            )
        };
        let ii = ICONINFO {
            f_icon: 1,
            x_hotspot: 0,
            y_hotspot: 0,
            hbm_mask,
            hbm_color,
        };
        let hicon = unsafe { CreateIconIndirect(&ii) };
        unsafe {
            DeleteObject(hbm_color);
            DeleteObject(hbm_mask);
        }
        hicon
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: *mut c_void,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if msg == WM_TRAYICON {
            let event = (lparam as u32) & 0xFFFF;
            if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                if let Some(q) = GLOBAL_ACTIONS.get() {
                    q.lock().unwrap().push_back(TrayAction::ShowWindow);
                }
                return 0;
            } else if event == WM_RBUTTONUP {
                unsafe {
                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    SetForegroundWindow(hwnd);

                    let hmenu = CreatePopupMenu();
                    let s_show = to_wide("Show NetMonitor");
                    let s_toggle = to_wide("Toggle Hotspot");
                    let s_insp = to_wide("Traffic Inspector");
                    let s_exit = to_wide("Exit");

                    AppendMenuW(hmenu, MF_STRING, ID_SHOW, s_show.as_ptr());
                    AppendMenuW(hmenu, MF_STRING, ID_TOGGLE_HS, s_toggle.as_ptr());
                    AppendMenuW(hmenu, MF_STRING, ID_INSPECTOR, s_insp.as_ptr());
                    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
                    AppendMenuW(hmenu, MF_STRING, ID_EXIT, s_exit.as_ptr());

                    let cmd = TrackPopupMenu(
                        hmenu,
                        TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
                        pt.x,
                        pt.y,
                        0,
                        hwnd,
                        std::ptr::null(),
                    ) as usize;
                    DestroyMenu(hmenu);

                    if let Some(q) = GLOBAL_ACTIONS.get() {
                        match cmd {
                            ID_SHOW => q.lock().unwrap().push_back(TrayAction::ShowWindow),
                            ID_TOGGLE_HS => q.lock().unwrap().push_back(TrayAction::ToggleHotspot),
                            ID_INSPECTOR => q.lock().unwrap().push_back(TrayAction::OpenInspector),
                            ID_EXIT => q.lock().unwrap().push_back(TrayAction::Exit),
                            _ => {}
                        }
                    }
                }
                return 0;
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    pub fn spawn_tray_thread(
        actions: Arc<Mutex<VecDeque<TrayAction>>>,
        hwnd_shared: Arc<Mutex<usize>>,
    ) {
        let _ = GLOBAL_ACTIONS.set(actions);
        std::thread::spawn(move || unsafe {
            let class_name = to_wide("NetMonitorTrayClass");
            let hinstance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                style: 0,
                lpfn_wnd_proc: wnd_proc,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: hinstance,
                h_icon: std::ptr::null_mut(),
                h_cursor: std::ptr::null_mut(),
                hbr_background: std::ptr::null_mut(),
                lpsz_menu_name: std::ptr::null(),
                lpsz_class_name: class_name.as_ptr(),
            };
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );
            if hwnd.is_null() {
                return;
            }

            *hwnd_shared.lock().unwrap() = hwnd as usize;

            let initial_icon = create_cyberpunk_icon(false, false);
            let hicon = icon_data_to_hicon(&initial_icon);

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.h_wnd = hwnd;
            nid.u_id = 1;
            nid.u_flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.u_callback_message = WM_TRAYICON;
            nid.h_icon = hicon;

            let tip = to_wide("CORE :: SYSTEM NetMonitor");
            for (i, &ch) in tip.iter().take(127).enumerate() {
                nid.sz_tip[i] = ch;
            }

            Shell_NotifyIconW(NIM_ADD, &nid);
            DestroyIcon(hicon);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
    }

    pub fn update_tray_icon(hwnd: usize, icon: &IconData, hotspot_on: bool) {
        unsafe {
            let hicon = icon_data_to_hicon(icon);
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.h_wnd = hwnd as *mut c_void;
            nid.u_id = 1;
            nid.u_flags = NIF_ICON | NIF_TIP;
            nid.h_icon = hicon;

            let tip_str = if hotspot_on {
                "CORE :: SYSTEM [Hotspot ACTIVE]"
            } else {
                "CORE :: SYSTEM [Hotspot Standby]"
            };
            let tip = to_wide(tip_str);
            for (i, &ch) in tip.iter().take(127).enumerate() {
                nid.sz_tip[i] = ch;
            }

            Shell_NotifyIconW(NIM_MODIFY, &nid);
            DestroyIcon(hicon);
        }
    }

    pub fn remove_tray_icon(hwnd: usize) {
        unsafe {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.h_wnd = hwnd as *mut c_void;
            nid.u_id = 1;
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }
}
