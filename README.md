# 🖥️ MacConnect

High-performance remote control suite for your **Mac Mini** from your **Windows PC/Laptop**.

- **Primary / Default Mode**: 🌐 **Native macOS VNC (Wireless)** – Connects directly to Apple's built-in Screen Sharing service with zero cables required.
- **Hardware Mode**: 🔌 **USB Video Capture Card (HDMI)** – Zero-latency HDMI capture card pipeline.

---

## ⚡ Quick Start: Native macOS VNC Mode (Default)

### 1. One-time Setup on Mac Mini
1. Open **System Settings $\rightarrow$ General $\rightarrow$ Sharing**.
2. Toggle **ON** **Screen Sharing**.
3. Click the **(i)** info icon $\rightarrow$ toggle **ON** **"VNC viewers may control screen with password"** $\rightarrow$ set a password.
4. Note your Mac Mini's local IP address (e.g. `192.168.1.50`).

### 2. Connect from Windows Laptop
1. Launch the compiled executable:
   ```powershell
   d:\Code\mac-connect\windows-client\target\release\mac-connect-client.exe
   ```
2. Enter your Mac Mini's **IP Address** and **VNC Password**.
3. Click **🟢 Connect**.
4. Press **`F11`** for borderless fullscreen display.

---

## 🎮 Shortcuts & Controls

- **`Click inside Screen`**: Locks focus to the Mac (mouse & keystrokes stream directly to Mac).
- **`Esc`** or **`Ctrl + Alt`** or **`F12`**: **Instantly unlocks** mouse & keyboard back to Windows.
- **`F11`**: Toggle borderless fullscreen display.
- **`F10`**: Show / hide the top settings bar.
- **`Alt + C` / `Alt + V`**: Native Mac Copy / Paste (`⌘C` / `⌘V`).
- **`2-Finger Touchpad Scroll`**: Seamless continuous macOS scrolling.
