# MacReceiver (macOS Component)

Lightweight, native macOS receiver daemon that listens for input events over UDP and injects them natively into macOS with sub-millisecond latency.

---

## 🛠️ How to Compile & Run on your Mac Mini

### Step 1: Copy this folder to your Mac Mini
You can transfer the `mac-receiver` directory via AirDrop, USB flash drive, Git, or `scp`:
```bash
# Example using scp from your PC (or just copy folder)
scp -r mac-receiver user@<mac-ip>:~/mac-receiver
```

### Step 2: Build the native binary
Open Terminal on your Mac Mini, navigate to the folder, and run:
```bash
cd ~/mac-receiver
swift build -c release
```
*Note: If Xcode Command Line Tools are not installed, macOS will automatically prompt you with `xcode-select --install`. Click Install.*

The compiled binary will be located at:
```bash
.build/release/MacReceiver
```

### Step 3: Grant macOS Accessibility Permission
1. Run the app:
   ```bash
   ./.build/release/MacReceiver
   ```
2. On the first run, macOS will prompt you for Accessibility permission.
3. Open **System Settings $\rightarrow$ Privacy & Security $\rightarrow$ Accessibility** and toggle **ON** for **Terminal** (or `MacReceiver`).
4. Restart `MacReceiver`.

---

## ⚙️ Command-Line Options

```bash
# Standard run (default port 12345)
./.build/release/MacReceiver

# Custom port
./.build/release/MacReceiver --port 12345

# Swap Command (⌘) and Control (⌃) so Ctrl+C / Ctrl+V works with Windows muscle memory
./.build/release/MacReceiver --swap-cmd-ctrl

# Adjust mouse sensitivity (e.g. 1.2x)
./.build/release/MacReceiver --sensitivity 1.2
```

---

## 🚀 Optional: Run on Mac Startup (Launchd Daemon)
To have `MacReceiver` start automatically whenever your Mac Mini boots:
```bash
cp .build/release/MacReceiver /usr/local/bin/
```
And add it to your Login Items under **System Settings $\rightarrow$ General $\rightarrow$ Login Items**.
