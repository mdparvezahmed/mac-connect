import Foundation

/// Binary Packet Protocol for MacConnect
/// Magic bytes: 'M', 'C' (0x4D, 0x43)
public enum PacketType: UInt8 {
    case mouseMoveRelative = 0x01
    case mouseMoveAbsolute = 0x02
    case mouseButton       = 0x03
    case mouseWheel        = 0x04
    case keyEvent          = 0x05
    case ping              = 0x06
    case pong              = 0x07
    case resetModifiers    = 0x08
}

public enum MouseButtonId: UInt8 {
    case left    = 0
    case right   = 1
    case middle  = 2
    case back    = 3
    case forward = 4
}

public enum ButtonAction: UInt8 {
    case release = 0
    case press   = 1
}

public enum KeyAction: UInt8 {
    case keyUp   = 0
    case keyDown = 1
}

public struct ModifierFlags: OptionSet {
    public let rawValue: UInt8
    public init(rawValue: UInt8) { self.rawValue = rawValue }

    public static let shift = ModifierFlags(rawValue: 1 << 0)
    public static let ctrl  = ModifierFlags(rawValue: 1 << 1)
    public static let alt   = ModifierFlags(rawValue: 1 << 2)
    public static let win   = ModifierFlags(rawValue: 1 << 3) // Mapped to Cmd
}

public enum IncomingPacket {
    case mouseMoveRelative(dx: Int32, dy: Int32)
    case mouseMoveAbsolute(normX: Float, normY: Float)
    case mouseButton(button: MouseButtonId, action: ButtonAction)
    case mouseWheel(deltaX: Int32, deltaY: Int32)
    case keyEvent(winVk: UInt16, action: KeyAction, modifiers: ModifierFlags)
    case ping(timestamp: UInt64)
    case resetModifiers
}

public struct PacketParser {
    public static let magic0: UInt8 = 0x4D // 'M'
    public static let magic1: UInt8 = 0x43 // 'C'

    public static func parse(data: Data) -> IncomingPacket? {
        guard data.count >= 3 else { return nil }

        let bytes = [UInt8](data)
        guard bytes[0] == magic0 && bytes[1] == magic1 else { return nil }

        guard let packetType = PacketType(rawValue: bytes[2]) else { return nil }

        switch packetType {
        case .mouseMoveRelative:
            guard data.count >= 11 else { return nil }
            let dx = data.subdata(in: 3..<7).withUnsafeBytes { $0.load(as: Int32.self) }
            let dy = data.subdata(in: 7..<11).withUnsafeBytes { $0.load(as: Int32.self) }
            return .mouseMoveRelative(dx: dx, dy: dy)

        case .mouseMoveAbsolute:
            guard data.count >= 11 else { return nil }
            let normX = data.subdata(in: 3..<7).withUnsafeBytes { $0.load(as: Float.self) }
            let normY = data.subdata(in: 7..<11).withUnsafeBytes { $0.load(as: Float.self) }
            return .mouseMoveAbsolute(normX: normX, normY: normY)

        case .mouseButton:
            guard data.count >= 5 else { return nil }
            guard let btn = MouseButtonId(rawValue: bytes[3]),
                  let action = ButtonAction(rawValue: bytes[4]) else { return nil }
            return .mouseButton(button: btn, action: action)

        case .mouseWheel:
            guard data.count >= 11 else { return nil }
            let dx = data.subdata(in: 3..<7).withUnsafeBytes { $0.load(as: Int32.self) }
            let dy = data.subdata(in: 7..<11).withUnsafeBytes { $0.load(as: Int32.self) }
            return .mouseWheel(deltaX: dx, deltaY: dy)

        case .keyEvent:
            guard data.count >= 7 else { return nil }
            let winVk = data.subdata(in: 3..<5).withUnsafeBytes { $0.load(as: UInt16.self) }
            guard let action = KeyAction(rawValue: bytes[5]) else { return nil }
            let modifiers = ModifierFlags(rawValue: bytes[6])
            return .keyEvent(winVk: winVk, action: action, modifiers: modifiers)

        case .ping:
            guard data.count >= 11 else { return nil }
            let ts = data.subdata(in: 3..<11).withUnsafeBytes { $0.load(as: UInt64.self) }
            return .ping(timestamp: ts)

        case .pong:
            return nil

        case .resetModifiers:
            return .resetModifiers
        }
    }

    public static func createPong(timestamp: UInt64) -> Data {
        var data = Data([magic0, magic1, PacketType.pong.rawValue])
        var ts = timestamp
        data.append(UnsafeBufferPointer(start: &ts, count: 1))
        return data
    }
}
