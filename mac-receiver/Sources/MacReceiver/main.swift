import Foundation

struct AppConfig {
    var port: UInt16 = 12345
    var swapCmdAndCtrl: Bool = false
    var sensitivity: Double = 1.0
}

func parseCommandLineArgs() -> AppConfig {
    var config = AppConfig()
    let args = CommandLine.arguments

    var i = 1
    while i < args.count {
        let arg = args[i]
        switch arg {
        case "--port", "-p":
            if i + 1 < args.count, let p = UInt16(args[i + 1]) {
                config.port = p
                i += 1
            }
        case "--swap-cmd-ctrl":
            config.swapCmdAndCtrl = true
        case "--sensitivity", "-s":
            if i + 1 < args.count, let sens = Double(args[i + 1]) {
                config.sensitivity = sens
                i += 1
            }
        case "--help", "-h":
            print("""
            MacReceiver - Low Latency Input Receiver for macOS
            
            Usage:
              MacReceiver [options]

            Options:
              --port, -p <port>         UDP port to listen on (default: 12345)
              --swap-cmd-ctrl           Swap Command and Control keys (for PC muscle memory)
              --sensitivity, -s <val>   Mouse sensitivity multiplier (default: 1.0)
              --help, -h                Show this help message
            """)
            exit(0)
        default:
            print("Unknown argument: \(arg). Use --help for usage.")
        }
        i += 1
    }
    return config
}

let config = parseCommandLineArgs()

print("""
=============================================================
🍎 MacReceiver - Ultra Low-Latency Input Receiver
=============================================================
• Listening Port:       \(config.port) (UDP)
• Key Mapping:          \(config.swapCmdAndCtrl ? "Swapped (Ctrl ⇄ Cmd)" : "Standard (Win ➔ Cmd, Ctrl ➔ Ctrl)")
• Mouse Sensitivity:    \(config.sensitivity)x
=============================================================
""")

// Check Accessibility permission
PermissionChecker.checkAccessibility(prompt: true)

// Initialize KeyMapper & InputInjector
let keyMapper = KeyMapper(swapCmdAndCtrl: config.swapCmdAndCtrl)
let inputInjector = InputInjector(keyMapper: keyMapper, mouseSensitivity: config.sensitivity)

// Start UDP Server
let server = NetworkServer(port: config.port, inputInjector: inputInjector)
server.start()

// Handle termination signals
signal(SIGINT) { _ in
    print("\n👋 Stopping MacReceiver...")
    exit(0)
}
signal(SIGTERM) { _ in
    print("\n👋 Stopping MacReceiver...")
    exit(0)
}

// Keep main thread alive
RunLoop.main.run()
