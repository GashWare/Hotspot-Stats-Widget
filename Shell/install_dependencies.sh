#!/usr/bin/env bash
# Installation script for Linux dependencies (Arch, Debian, Kali, Ubuntu)
set -e

echo "Detecting package manager..."
if command -v apt-get &>/dev/null; then
    echo "Detected Debian / Ubuntu / Kali system."
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        cargo \
        rustc \
        pkg-config \
        libssl-dev \
        libpcap-dev \
        libx11-dev \
        libxcursor-dev \
        libxrandr-dev \
        libxi-dev \
        libxkbcommon-dev \
        libwayland-dev \
        network-manager \
        iptables \
        python3 \
        python3-pip \
        python3-tk
    echo "Installing Python dependencies..."
    pip3 install --break-system-packages customtkinter psutil scapy || pip3 install customtkinter psutil scapy
elif command -v pacman &>/dev/null; then
    echo "Detected Arch Linux system."
    sudo pacman -S --needed --noconfirm \
        base-devel \
        rust \
        cargo \
        pkgconf \
        openssl \
        libpcap \
        libx11 \
        libxcursor \
        libxrandr \
        libxi \
        libxkbcommon \
        wayland \
        networkmanager \
        iptables \
        python \
        python-pip \
        tk
    echo "Installing Python dependencies..."
    pip install --break-system-packages customtkinter psutil scapy || pip install customtkinter psutil scapy
else
    echo "Unsupported distribution package manager. Please manually install: cargo, rustc, libpcap-dev, X11/Wayland dev libraries, network-manager, iptables, and python3."
fi

echo "Dependencies installed successfully!"
