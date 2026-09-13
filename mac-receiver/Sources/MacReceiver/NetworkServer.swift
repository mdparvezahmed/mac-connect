import Foundation

public class NetworkServer {
    private let port: UInt16
    private let inputInjector: InputInjector
    private var isRunning = false
    private var socketFd: Int32 = -1
    private var workerThread: Thread?
    private var loggedFirstPacket = false
    private var loggedFirstPong = false
    private var loggedBadPacket = false

    public init(port: UInt16, inputInjector: InputInjector) {
        self.port = port
        self.inputInjector = inputInjector
    }

    public func start() {
        guard !isRunning else { return }

        // Create UDP socket
        socketFd = socket(AF_INET, SOCK_DGRAM, 0)
        guard socketFd >= 0 else {
            print("❌ Failed to create UDP socket: \(errno)")
            return
        }

        // Set reuse address
        var reuse = 1
        setsockopt(socketFd, SOL_SOCKET, SO_REUSEADDR, &reuse, socklen_t(MemoryLayout<Int32>.size))

        // Bind to 0.0.0.0:<port>
        var addr = sockaddr_in()
        addr.sin_family = sa_family_t(AF_INET)
        addr.sin_port = port.bigEndian
        addr.sin_addr.s_addr = INADDR_ANY.bigEndian

        let bindResult = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                bind(socketFd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }

        guard bindResult == 0 else {
            print("❌ Failed to bind to UDP port \(port): \(errno)")
            close(socketFd)
            return
        }

        isRunning = true
        print("🚀 MacReceiver UDP server listening on port \(port)")

        // Start listening thread
        workerThread = Thread { [weak self] in
            self?.receiveLoop()
        }
        workerThread?.name = "MacReceiver.UDPWorker"
        workerThread?.qualityOfService = .userInteractive
        workerThread?.start()
    }

    public func stop() {
        isRunning = false
        if socketFd >= 0 {
            close(socketFd)
            socketFd = -1
        }
    }

    /// Renders a client address as "ip:port" for logging.
    private func describe(_ addr: sockaddr_in) -> String {
        var copy = addr
        var text = [CChar](repeating: 0, count: Int(INET_ADDRSTRLEN))
        inet_ntop(AF_INET, &copy.sin_addr, &text, socklen_t(INET_ADDRSTRLEN))
        return "\(String(cString: text)):\(UInt16(bigEndian: copy.sin_port))"
    }

    private func receiveLoop() {
        var buffer = [UInt8](repeating: 0, count: 1024)
        var clientAddr = sockaddr_in()
        var addrLen = socklen_t(MemoryLayout<sockaddr_in>.size)

        while isRunning {
            let bytesRead = withUnsafeMutablePointer(to: &clientAddr) {
                $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                    recvfrom(socketFd, &buffer, buffer.count, 0, $0, &addrLen)
                }
            }

            guard bytesRead > 0 else {
                if !isRunning { break }
                continue
            }

            if !loggedFirstPacket {
                loggedFirstPacket = true
                print("\u{1F4E5} First packet received from \(describe(clientAddr)) (\(bytesRead) bytes)")
            }

            let data = Data(bytes: buffer, count: bytesRead)
            guard let packet = PacketParser.parse(data: data) else {
                if !loggedBadPacket {
                    loggedBadPacket = true
                    print("\u{26A0}\u{FE0F} Unrecognised packet from \(describe(clientAddr)), ignoring")
                }
                continue
            }

            // Dispatch event to InputInjector
            switch packet {
            case .mouseMoveRelative(let dx, let dy):
                inputInjector.moveMouseRelative(dx: dx, dy: dy)

            case .mouseMoveAbsolute(let normX, let normY):
                inputInjector.moveMouseAbsolute(normX: normX, normY: normY)

            case .mouseButton(let button, let action):
                inputInjector.handleMouseButton(button: button, action: action)

            case .mouseWheel(let deltaX, let deltaY):
                inputInjector.handleMouseWheel(deltaX: deltaX, deltaY: deltaY)

            case .keyEvent(let winVk, let action, let modifiers):
                inputInjector.handleKeyEvent(winVk: winVk, action: action, modifiers: modifiers)

            case .ping(let timestamp):
                // Send pong reply back to client
                let pongData = PacketParser.createPong(timestamp: timestamp)
                let sent = pongData.withUnsafeBytes { rawBuffer in
                    withUnsafePointer(to: &clientAddr) {
                        $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                            sendto(socketFd, rawBuffer.baseAddress, pongData.count, 0, $0, addrLen)
                        }
                    }
                }

                // Report the first reply either way. A ping that arrives but
                // whose pong cannot leave the machine is the signature of the
                // Local Network privacy gate, and is otherwise invisible.
                if !loggedFirstPong {
                    loggedFirstPong = true
                    if sent < 0 {
                        print("\u{274C} Ping from \(describe(clientAddr)) arrived but the pong failed to send (errno \(errno))")
                    } else {
                        print("\u{1F517} Linked - answered ping from \(describe(clientAddr))")
                    }
                }

            case .resetModifiers:
                inputInjector.resetStates()
            }
        }
    }
}
