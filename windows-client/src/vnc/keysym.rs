#![allow(dead_code)]

//! KeySym definitions and Windows Virtual Key to X11/RFB KeySym translation

pub const XK_BACKSPACE: u32 = 0xFF08;
pub const XK_TAB: u32       = 0xFF09;
pub const XK_RETURN: u32    = 0xFF0D;
pub const XK_ESCAPE: u32    = 0xFF1B;
pub const XK_DELETE: u32    = 0xFFFF;
pub const XK_HOME: u32      = 0xFF50;
pub const XK_LEFT: u32      = 0xFF51;
pub const XK_UP: u32        = 0xFF52;
pub const XK_RIGHT: u32     = 0xFF53;
pub const XK_DOWN: u32      = 0xFF54;
pub const XK_PAGE_UP: u32   = 0xFF55;
pub const XK_PAGE_DOWN: u32 = 0xFF56;
pub const XK_END: u32       = 0xFF57;

pub const XK_SHIFT_L: u32   = 0xFFE1;
pub const XK_SHIFT_R: u32   = 0xFFE2;
pub const XK_CONTROL_L: u32 = 0xFFE3;
pub const XK_CONTROL_R: u32 = 0xFFE4;
pub const XK_CAPS_LOCK: u32 = 0xFFE5;
pub const XK_META_L: u32    = 0xFFE7;
pub const XK_ALT_L: u32     = 0xFFE9; // Option on macOS
pub const XK_ALT_R: u32     = 0xFFEA;
pub const XK_SUPER_L: u32   = 0xFFEB; // Command (⌘) on macOS
pub const XK_SUPER_R: u32   = 0xFFEC;

pub const XK_F1: u32        = 0xFFBE;
pub const XK_F2: u32        = 0xFFBF;
pub const XK_F3: u32        = 0xFFC0;
pub const XK_F4: u32        = 0xFFC1;
pub const XK_F5: u32        = 0xFFC2;
pub const XK_F6: u32        = 0xFFC3;
pub const XK_F7: u32        = 0xFFC4;
pub const XK_F8: u32        = 0xFFC5;
pub const XK_F9: u32        = 0xFFC6;
pub const XK_F10: u32       = 0xFFC7;
pub const XK_F11: u32       = 0xFFC8;
pub const XK_F12: u32       = 0xFFC9;

/// Maps a Windows Virtual Key code (`VK_*`) and shift state to an RFB/X11 KeySym
pub fn map_vk_to_keysym(vk: u16, is_shift: bool) -> Option<u32> {
    match vk {
        // Letters A-Z
        0x41..=0x5A => {
            let base = if is_shift { 0x41 } else { 0x61 };
            Some(base + (vk as u32 - 0x41))
        }
        // Top Row Digits & Symbols
        0x30 => Some(if is_shift { b')' as u32 } else { b'0' as u32 }),
        0x31 => Some(if is_shift { b'!' as u32 } else { b'1' as u32 }),
        0x32 => Some(if is_shift { b'@' as u32 } else { b'2' as u32 }),
        0x33 => Some(if is_shift { b'#' as u32 } else { b'3' as u32 }),
        0x34 => Some(if is_shift { b'$' as u32 } else { b'4' as u32 }),
        0x35 => Some(if is_shift { b'%' as u32 } else { b'5' as u32 }),
        0x36 => Some(if is_shift { b'^' as u32 } else { b'6' as u32 }),
        0x37 => Some(if is_shift { b'&' as u32 } else { b'7' as u32 }),
        0x38 => Some(if is_shift { b'*' as u32 } else { b'8' as u32 }),
        0x39 => Some(if is_shift { b'(' as u32 } else { b'9' as u32 }),

        // Standard whitespace & control
        0x08 => Some(XK_BACKSPACE),
        0x09 => Some(XK_TAB),
        0x0D => Some(XK_RETURN),
        0x1B => Some(XK_ESCAPE),
        0x20 => Some(b' ' as u32),

        // Navigation
        0x21 => Some(XK_PAGE_UP),
        0x22 => Some(XK_PAGE_DOWN),
        0x23 => Some(XK_END),
        0x24 => Some(XK_HOME),
        0x25 => Some(XK_LEFT),
        0x26 => Some(XK_UP),
        0x27 => Some(XK_RIGHT),
        0x28 => Some(XK_DOWN),
        0x2E => Some(XK_DELETE),

        // Modifiers (Swapped: Alt is Command ⌘, Win is Option ⌥)
        0x10 | 0xA0 => Some(XK_SHIFT_L),
        0xA1        => Some(XK_SHIFT_R),
        0x11 | 0xA2 => Some(XK_CONTROL_L),
        0xA3        => Some(XK_CONTROL_R),
        0x12 | 0xA4 => Some(XK_SUPER_L), // Alt Key -> Command (⌘) on Mac
        0xA5        => Some(XK_SUPER_R),
        0x5B        => Some(XK_ALT_L),   // Windows Key -> Option (⌥) on Mac
        0x5C        => Some(XK_ALT_R),
        0x14        => Some(XK_CAPS_LOCK),

        // Function Keys
        0x70 => Some(XK_F1),
        0x71 => Some(XK_F2),
        0x72 => Some(XK_F3),
        0x73 => Some(XK_F4),
        0x74 => Some(XK_F5),
        0x75 => Some(XK_F6),
        0x76 => Some(XK_F7),
        0x77 => Some(XK_F8),
        0x78 => Some(XK_F9),
        0x79 => Some(XK_F10),
        0x7A => Some(XK_F11),
        0x7B => Some(XK_F12),

        // Punctuation / OEM Keys (US Keyboard)
        0xBA => Some(if is_shift { b':' as u32 } else { b';' as u32 }),
        0xBB => Some(if is_shift { b'+' as u32 } else { b'=' as u32 }),
        0xBC => Some(if is_shift { b'<' as u32 } else { b',' as u32 }),
        0xBD => Some(if is_shift { b'_' as u32 } else { b'-' as u32 }),
        0xBE => Some(if is_shift { b'>' as u32 } else { b'.' as u32 }),
        0xBF => Some(if is_shift { b'?' as u32 } else { b'/' as u32 }),
        0xC0 => Some(if is_shift { b'~' as u32 } else { b'`' as u32 }),
        0xDB => Some(if is_shift { b'{' as u32 } else { b'[' as u32 }),
        0xDC => Some(if is_shift { b'|' as u32 } else { b'\\' as u32 }),
        0xDD => Some(if is_shift { b'}' as u32 } else { b']' as u32 }),
        0xDE => Some(if is_shift { b'"' as u32 } else { b'\'' as u32 }),

        // Numpad
        0x60 => Some(b'0' as u32),
        0x61 => Some(b'1' as u32),
        0x62 => Some(b'2' as u32),
        0x63 => Some(b'3' as u32),
        0x64 => Some(b'4' as u32),
        0x65 => Some(b'5' as u32),
        0x66 => Some(b'6' as u32),
        0x67 => Some(b'7' as u32),
        0x68 => Some(b'8' as u32),
        0x69 => Some(b'9' as u32),
        0x6A => Some(b'*' as u32),
        0x6B => Some(b'+' as u32),
        0x6D => Some(b'-' as u32),
        0x6E => Some(b'.' as u32),
        0x6F => Some(b'/' as u32),

        _ => None,
    }
}
