# 🖥️ MacConnect

Zero-latency hardware & network bridge to control your **Mac Mini** from your **Windows PC/Laptop** using a USB Video Capture Card for instant video display and UDP for keyboard, mouse, and trackpad input streaming.

```
d:\Code\mac-connect\
├── windows-client/          # Windows Client App (Rust + egui/DirectShow/MediaFoundation)
└── mac-receiver/            # macOS Receiver Daemon (Native Swift + CoreGraphics CGEvent)
```

---

## ⚡ Quick Start Guide

### 1. Setup Mac Mini (`mac-receiver/`)
1. Copy the `mac-receiver` folder to your Mac Mini.
2. In Terminal on your Mac:
   ```bash
   cd ~/mac-receiver
   swift build -c release
   ./.build/release/MacReceiver
   ```
3. When prompted, enable **Accessibility Permission** under **System Settings $\rightarrow$ Privacy & Security $\rightarrow$ Accessibility**.
4. Note your Mac Mini's local IP address (e.g. `192.168.1.50`).

### 2. Setup Windows Laptop (`windows-client/`)
1. Connect HDMI cable from Mac Mini to your USB Capture Card.
2. Plug the USB Capture Card into your Windows PC.
3. Launch the compiled client executable:
   ```powershell
   d:\Code\mac-connect\windows-client\target\release\mac-connect-client.exe
   ```
4. Select your **Capture Device** from the top dropdown.
5. Enter your Mac Mini's IP address.

---

## 🎮 How to Control

- **Click inside the video window**: Locks your mouse cursor inside the Mac screen and activates input streaming.
- **`Ctrl + Alt`** or **`F12`**: Instantly releases cursor focus back to Windows.
- **`F11`**: Toggle borderless fullscreen display.
- **`F10`**: Show / hide the top settings bar.

---

## ⌨️ Default Key Mappings

| Windows Key | Mac Mini Mapping |
| :--- | :--- |
| **Windows Key** | **Command (`⌘`)** |
| **Alt Key** | **Option (`⌥`)** |
| **Ctrl Key** | **Control (`⌃`)** |
| **2-Finger Touchpad Scroll** | **macOS Continuous Pixel Scroll** |
| **F1 – F12** | **Mac Function Keys (`F1 – F12`)** |

*(To swap `Ctrl` and `Cmd` for PC muscle memory copy/paste, run `./.build/release/MacReceiver --swap-cmd-ctrl` on your Mac).*
