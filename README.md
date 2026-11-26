# 🏎️ Hotspot Stats Widget (NetMonitor)

<p align="center">
  <img src="Rust/app_icon.ico" width="96" height="96" alt="Hotspot Stats Widget Logo" />
</p>

<p align="center">
  <b>High-performance desktop system monitor, Windows Mobile Hotspot manager, and real-time AI Model Quota cockpit written in Rust using <code>eframe</code> and <code>egui</code>.</b>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Language-Rust-orange?style=flat-square&logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/GUI-Eframe%20%2F%20Egui%200.35-blue?style=flat-square" alt="egui" />
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-brightgreen?style=flat-square&logo=windows" alt="Platform" />
  <img src="https://img.shields.io/badge/License-MIT-lightgrey?style=flat-square" alt="License" />
</p>

---

## 🌟 Overview

**Hotspot Stats Widget** is a lightweight, zero-latency desktop monitor designed for power users, developers, and hardware enthusiasts. Originally conceived as a Python/Tkinter tool, it has been completely re-engineered in pure Rust to provide sub-millisecond rendering, hardware GPU acceleration, multi-adapter network management, packet-level traffic sniffing, and real-time AI quota tracking styled after classic automotive instrument clusters.

---

## 🚀 Key Features

### 1. 🏎️ Antigravity AI Model Quota ("AI Fuel Gauges")
- **Real-Time Quota Telemetry**: Queries the local Antigravity Language Server in real-time (`RetrieveUserQuotaSummary`) over high-efficiency HTTP/1.1 RPC.
- **Dual Tank Cockpit Architecture**:
  - **⚡ 5-Hour Sprint Tank**: Tracks the immediate 5-hour rolling demand window with exact percentage remaining and live reset countdown (e.g., `⏱ 4h 34m`).
  - **⛽ Weekly Reserve Tank**: Tracks total tier weekly capacity with percentage remaining and reset duration (e.g., `⏱ 4d 21h`).
  - **🤖 Claude & GPT Models (3P)**: Sub-gauge tracking third-party model limits.
- **Automotive Fuel Gauge Styling**:
  - **Ferrari Dash**: Dual analog sweeping needle dials with `E` to `F` tick marks, illuminated active arcs, needle pivot hubs, and glowing low-fuel warning LEDs.
  - **C4 Corvette Cockpit**: Dual ascending 7-segment digital LCD wedges with `[E ▪▪▪▪▪▪▪▪▪ F]` segmented bars and flashing `[LOW FUEL]` annunciator boxes.
  - **Nissan 300ZX**: Dual segmented electronic cockpit ladder gauges with authentic 80s Z31 phosphor styling.
  - **Retro Amber & VFD Green**: Classic illuminated dual fuel chambers with `E ━━ 1/2 ━━ F` graduations.
- **Low Fuel Warning Lamp**: Automatically illuminates and pulses when capacity drops below 15% (matching Antigravity's low-capacity alert indicator).
- **Zero-Config Auto-Discovery**: Automatically detects active language server ports and CSRF tokens from environment variables, daemon state files (`~/.gemini/antigravity/daemon/hub_*.json`), or cached bridges.

### 2. 📊 Hardware & System Telemetry
- **CPU & RAM**: Real-time load percentage, clock frequency (GHz), core counts, package temperatures, and top consumer processes.
- **Automated Memory Optimizer**: Built-in `Clean Working Set RAM` engine that safely trims unused process working sets on-demand, on a scheduled interval (e.g., every 15m), or automatically when RAM crosses a threshold (e.g., $\ge 80\%$).
- **GPU & Dedicated VRAM**: Direct 3D engine utilization percentage, GPU temperatures, and dedicated video RAM tracking with hardware OpenGL acceleration.
- **SWAP & Storage**: Page file memory monitoring, disk capacity, and real-time system uptime counter.

### 3. 📡 Network Throughput & Packet Inspector
- **Throughput Metrics**: Live Download and Upload speeds with fluid animated graphs, peak hold indicators, and session data counters.
- **Hotspot Traffic Isolation**: Toggle between whole-system bandwidth and dedicated hotspot client bandwidth.
- **Packet Sniffer & Inspection**: Low-level packet capture via Npcap / libpcap directly on the virtual Wi-Fi Direct adapter.
- **Integrated DNS/IP Blocker**: Block unwanted telemetry, tracking, and advertising domains before they touch your network.

### 4. 🛜 Windows Mobile Hotspot Management
- **One-Click Hotspot Activation**: Toggle the Windows Mobile Hotspot with automated administrative elevation when required.
- **Multi-Adapter Smart Switching**: Automatically identifies existing internet connections and binds the hotspot to the optimal secondary wireless adapter.
- **Auto Max Throughput & 5 GHz**: Automatically configures high-throughput 802.11ac/ax Wi-Fi Direct settings and disables power-saving sleep timeouts.
- **Client Discovery & Port Scanner**: Scans connected client devices, reveals hostnames/MACs, and runs non-destructive port scans.

### 5. 🎧 Bluetooth Device Hub
- Real-time Bluetooth adapter status, device discovery, signal RSSI meters, battery percentages, and pairing states.
- Clean connection toggling for wireless headphones and input peripherals.

### 6. 📌 Taskbar Micro-Gauges
- An independent, transparent, always-on-top companion window designed to sit docked directly above the Windows taskbar.
- Compact micro-dials for **CPU**, **RAM**, **GPU**, **VRAM**, **DL**, **UL**, and **FUEL**.
- Click the RAM micro-gauge anytime to instantly purge memory working sets.
- Hover or click the FUEL micro-gauge for instant quota diagnostics.

---

## 🎨 Theme Engine

The widget includes several authentic automotive, vintage computing, and cyberpunk themes:

| Theme | Inspiration / Visual Aesthetic |
|---|---|
| **🏎️ Nissan 300ZX** | Z31 digital electronic dashboard with dual horizontal segmented ladders |
| **🏎️ Ferrari Dash** | High-contrast racing tachometer, analog sweeping needle gauges, shift lights |
| **🏎️ Corvette Red** | 1980s C4 Corvette digital cockpit, 7-segment digits, ascending LCD wedges |
| **⚡ Retro Amber** | 1980s CRT digital dashboard with warm monochrome phosphor glow |
| **📟 VFD Green** | Vacuum Fluorescent Display (Subaru XT / retro mint green phosphor) |
| **🌐 Cyberpunk Neon** | High-energy neon cyan, magenta, and electric yellow grid styling |
| **🌧️ Matrix** | Animated falling digital rain reacting dynamically to system load |
| **🕶️ Stealth OLED** | Minimalist gunmetal and ice white for dark desktop environments |

---

## 🛠️ Architecture & Tech Stack

```
Stats Widget/
├── Rust/
│   ├── test_gui/
│   │   ├── src/
│   │   │   ├── main.rs          # Application entrypoint, event loop & UI frames
│   │   │   ├── quota.rs         # Antigravity Language Server RPC & quota poller
│   │   │   ├── skins.rs         # Automotive gauge renderers & theme skins
│   │   │   ├── stats.rs         # Hardware metrics (CPU, RAM, GPU, VRAM, Net)
│   │   │   ├── hotspot.rs       # WinRT Windows Mobile Hotspot manager
│   │   │   ├── bluetooth.rs     # Bluetooth device manager & WinRT APIs
│   │   │   ├── traffic.rs       # Npcap packet capture & firewall blocker
│   │   │   ├── memclean.rs      # Working set memory trimmer
│   │   │   ├── graph.rs         # Fluid line & tachometer graph engine
│   │   │   ├── window_fx.rs     # Win32 window docking, blur & topmost pinning
│   │   │   └── config.rs        # Serialization & runtime preferences
│   │   └── Cargo.toml           # Crate manifest & dependencies
├── PowerShell/                  # Diagnostic, tethering, and WinRT scripts
├── Python/                      # Original Python / Tkinter prototype implementation
├── Batch/                       # Build & startup automation batch files
└── Shell/                       # Linux cross-platform compilation scripts
```

- **GUI Framework**: `eframe` / `egui 0.35` (Immediate-mode GUI with OpenGL hardware acceleration).
- **System Telemetry**: `sysinfo 0.38` & Win32 Performance Counters.
- **Packet Capture**: `pcap` / native Npcap DLL bindings.
- **Windows Runtime**: WinRT APIs (`Windows.Networking.NetworkOperators`, `Windows.Devices.Bluetooth`).

---

## 💻 Building & Installation

### Prerequisites
1. **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/).
2. **C++ Build Tools**: Visual Studio Build Tools with Windows SDK (for MSVC target).
3. **Npcap**: [Npcap SDK](https://npcap.com/) (optional, required only for raw packet sniffing).

### Compilation

#### Windows (Release Build)
```powershell
cd Rust\test_gui
cargo build --release
```
The optimized executable will be located at `Rust\test_gui\target\release\hotspot_stats_widget.exe`.

#### Linux Cross-Compilation Check
```bash
cargo check --target x86_64-unknown-linux-gnu
```

---

## ⚙️ Configuration

The widget automatically saves its state to `widget_config.json` upon exit or modification. An example configuration template is available in [`widget_config.example.json`](widget_config.example.json):

```json
{
  "theme": "nissan_300zx",
  "show_model_quota": true,
  "quota_graph_style": "dash",
  "show_taskbar_gauges": true,
  "auto_clean_ram_interval_enabled": false,
  "auto_clean_ram_threshold_enabled": true,
  "auto_clean_ram_threshold_pct": 80
}
```

- **Module Reordering**: Right-click any module and select **Unlock Drag & Drop** (or hold <kbd>Alt</kbd>) to drag modules into your preferred order.
- **Module Resizing**: Drag the bottom border handle of any module vertically to customize its height.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
