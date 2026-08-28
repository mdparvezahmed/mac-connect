# MacConnect Client (Windows Component)

High-performance, low-latency Windows application written in Rust to display the Mac Mini's HDMI output (via USB capture card) and stream keyboard, mouse, and touchpad inputs to the Mac Mini when focused.

---

## 🚀 How to Build & Run on Windows

### Prerequisites
You already have **Rust & Cargo** installed!

### Step 1: Compile the executable
Open PowerShell or Command Prompt in the `windows-client` directory:
```powershell
cd d:\Code\mac-connect\windows-client
cargo build --release
```

The compiled standalone executable will be located at:
```powershell
.\target\release\mac-connect-client.exe
```

### Step 2: Running the Client
Run the application:
```powershell
cargo run --release
```
or double-click `target\release\mac-connect-client.exe`.

---

## 🎮 How to Use

1. **Video Feed**:
   - Plug your USB HDMI Capture Card into your PC.
   - Connect the HDMI cable from your Mac Mini into the Capture Card.
   - Select your capture card from the **Capture Device** dropdown at the top.

2. **Connect to Mac Mini**:
   - Enter your Mac Mini's local IP address (e.g. `192.168.1.50`) and Port (`12345`).
   - The status indicator at the top will turn **Green (Connected)** once `MacReceiver` responds.

3. **Take Control**:
   - **Click anywhere inside the video window** to lock your cursor and take full control of your Mac Mini.
   - You can use your laptop's keyboard, touchpad (with 2-finger scrolling), and mouse.

4. **Return to Windows (Host Key)**:
   - Press **`Ctrl + Alt`** or **`F12`** at any time to instantly release the cursor and return control back to your Windows PC.

5. **Shortcuts**:
   - **`F11`**: Toggle Fullscreen mode (fills the entire screen with Mac's display).
   - **`F10`**: Show / Hide top settings menu bar.
   - **`Ctrl + Alt`** or **`F12`**: Unlock cursor & return focus to Windows.
