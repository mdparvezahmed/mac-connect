import Foundation
import Carbon

public class KeyMapper {
    public var swapCmdAndCtrl: Bool = false

    public init(swapCmdAndCtrl: Bool = false) {
        self.swapCmdAndCtrl = swapCmdAndCtrl
    }

    /// Translates Windows Virtual Key code to macOS Virtual Key code (CGKeyCode)
    public func mapWindowsVkToMacKeycode(_ vk: UInt16) -> CGKeyCode? {
        // Direct Windows VK Mapping
        switch vk {
        // Letters (VK_A = 0x41 .. VK_Z = 0x5A)
        case 0x41: return 0x00 // A
        case 0x42: return 0x0B // B
        case 0x43: return 0x08 // C
        case 0x44: return 0x02 // D
        case 0x45: return 0x0E // E
        case 0x46: return 0x03 // F
        case 0x47: return 0x05 // G
        case 0x48: return 0x04 // H
        case 0x49: return 0x22 // I
        case 0x4A: return 0x26 // J
        case 0x4B: return 0x28 // K
        case 0x4C: return 0x25 // L
        case 0x4D: return 0x2E // M
        case 0x4E: return 0x2D // N
        case 0x4F: return 0x1F // O
        case 0x50: return 0x23 // P
        case 0x51: return 0x0C // Q
        case 0x52: return 0x0F // R
        case 0x53: return 0x01 // S
        case 0x54: return 0x11 // T
        case 0x55: return 0x20 // U
        case 0x56: return 0x09 // V
        case 0x57: return 0x0D // W
        case 0x58: return 0x07 // X
        case 0x59: return 0x10 // Y
        case 0x5A: return 0x06 // Z

        // Numbers top row (VK_0 = 0x30 .. VK_9 = 0x39)
        case 0x30: return 0x1D // 0
        case 0x31: return 0x12 // 1
        case 0x32: return 0x13 // 2
        case 0x33: return 0x14 // 3
        case 0x34: return 0x15 // 4
        case 0x35: return 0x17 // 5
        case 0x36: return 0x16 // 6
        case 0x37: return 0x1A // 7
        case 0x38: return 0x1C // 8
        case 0x39: return 0x19 // 9

        // Function Keys (VK_F1 = 0x70 .. VK_F12 = 0x7B)
        case 0x70: return 0x7A // F1
        case 0x71: return 0x78 // F2
        case 0x72: return 0x63 // F3
        case 0x73: return 0x76 // F4
        case 0x74: return 0x60 // F5
        case 0x75: return 0x61 // F6
        case 0x76: return 0x62 // F7
        case 0x77: return 0x64 // F8
        case 0x78: return 0x65 // F9
        case 0x79: return 0x6D // F10
        case 0x7A: return 0x67 // F11
        case 0x7B: return 0x6F // F12

        // Modifiers
        case 0x10, 0xA0: return 0x38 // Shift (Left)
        case 0xA1:       return 0x3C // Shift (Right)

        case 0x11, 0xA2: // Control (Left)
            return swapCmdAndCtrl ? 0x37 /* Command */ : 0x3B /* Control */
        case 0xA3:       // Control (Right)
            return swapCmdAndCtrl ? 0x36 /* Command */ : 0x3E /* Control */

        case 0x12, 0xA4: return 0x37 // Alt / Option (Left) -> Mapped to Mac Command (⌘)
        case 0xA5:       return 0x36 // Alt / Option (Right) -> Mapped to Mac Command (⌘)

        case 0x5B:       return 0x3A // Left Win Key -> Mapped to Mac Option (⌥)
        case 0x5C:       return 0x3D // Right Win Key -> Mapped to Mac Option (⌥)

        case 0x14: return 0x39 // Caps Lock

        // Whitespace & Navigation
        case 0x0D: return 0x24 // Return / Enter
        case 0x09: return 0x30 // Tab
        case 0x20: return 0x31 // Space
        case 0x08: return 0x33 // Backspace (Mac Delete)
        case 0x1B: return 0x35 // Escape

        case 0x2E: return 0x75 // Forward Delete
        case 0x24: return 0x73 // Home
        case 0x23: return 0x77 // End
        case 0x21: return 0x74 // Page Up
        case 0x22: return 0x79 // Page Down

        // Arrow Keys
        case 0x25: return 0x7B // Left Arrow
        case 0x27: return 0x7C // Right Arrow
        case 0x28: return 0x7D // Down Arrow
        case 0x26: return 0x7E // Up Arrow

        // Punctuation & Symbols (OEM keys on US standard keyboards)
        case 0xBA, 0x3B: return 0x29 // Semicolon / Colon
        case 0xBB, 0x3D: return 0x18 // Equal / Plus
        case 0xBC, 0x2C: return 0x2B // Comma / Less Than
        case 0xBD, 0x2D: return 0x1B // Minus / Underscore
        case 0xBE, 0x2E: return 0x2F // Period / Greater Than
        case 0xBF, 0x2F: return 0x2C // Slash / Question Mark
        case 0xC0, 0x60: return 0x32 // Grave / Tilde (` / ~)
        case 0xDB, 0x5B: return 0x21 // Left Bracket / Brace ([ / {)
        case 0xDC, 0x5C: return 0x2A // Backslash / Pipe (\ / |)
        case 0xDD, 0x5D: return 0x1E // Right Bracket / Brace (] / })
        case 0xDE, 0x27: return 0x27 // Quote / Double Quote (' / ")

        // Numpad
        case 0x60: return 0x52 // Num 0
        case 0x61: return 0x53 // Num 1
        case 0x62: return 0x54 // Num 2
        case 0x63: return 0x55 // Num 3
        case 0x64: return 0x56 // Num 4
        case 0x65: return 0x57 // Num 5
        case 0x66: return 0x58 // Num 6
        case 0x67: return 0x59 // Num 7
        case 0x68: return 0x5B // Num 8
        case 0x69: return 0x5C // Num 9
        case 0x6A: return 0x43 // Num Multiply (*)
        case 0x6B: return 0x45 // Num Add (+)
        case 0x6D: return 0x4E // Num Subtract (-)
        case 0x6E: return 0x41 // Num Decimal (.)
        case 0x6F: return 0x4B // Num Divide (/)

        default:
            return nil
        }
    }
}
