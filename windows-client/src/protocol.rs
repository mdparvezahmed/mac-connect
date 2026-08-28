#![allow(dead_code)]

use byteorder::{LittleEndian, WriteBytesExt};

pub const MAGIC0: u8 = 0x4D; // 'M'
pub const MAGIC1: u8 = 0x43; // 'C'

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    MouseMoveRelative = 0x01,
    MouseMoveAbsolute = 0x02,
    MouseButton       = 0x03,
    MouseWheel        = 0x04,
    KeyEvent          = 0x05,
    Ping              = 0x06,
    Pong              = 0x07,
    ResetModifiers    = 0x08,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonId {
    Left    = 0,
    Right   = 1,
    Middle  = 2,
    Back    = 3,
    Forward = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    Release = 0,
    Press   = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    KeyUp   = 0,
    KeyDown = 1,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ModifierFlags(pub u8);

impl ModifierFlags {
    pub const SHIFT: u8 = 1 << 0;
    pub const CTRL:  u8 = 1 << 1;
    pub const ALT:   u8 = 1 << 2;
    pub const WIN:   u8 = 1 << 3;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn set_shift(&mut self, on: bool) {
        if on { self.0 |= Self::SHIFT; } else { self.0 &= !Self::SHIFT; }
    }

    pub fn set_ctrl(&mut self, on: bool) {
        if on { self.0 |= Self::CTRL; } else { self.0 &= !Self::CTRL; }
    }

    pub fn set_alt(&mut self, on: bool) {
        if on { self.0 |= Self::ALT; } else { self.0 &= !Self::ALT; }
    }

    pub fn set_win(&mut self, on: bool) {
        if on { self.0 |= Self::WIN; } else { self.0 &= !Self::WIN; }
    }
}

pub struct PacketSerializer;

impl PacketSerializer {
    pub fn mouse_move_relative(dx: i32, dy: i32) -> Vec<u8> {
        let mut buf = Vec::with_capacity(11);
        buf.push(MAGIC0);
        buf.push(MAGIC1);
        buf.push(PacketType::MouseMoveRelative as u8);
        buf.write_i32::<LittleEndian>(dx).unwrap();
        buf.write_i32::<LittleEndian>(dy).unwrap();
        buf
    }

    pub fn mouse_move_absolute(norm_x: f32, norm_y: f32) -> Vec<u8> {
        let mut buf = Vec::with_capacity(11);
        buf.push(MAGIC0);
        buf.push(MAGIC1);
        buf.push(PacketType::MouseMoveAbsolute as u8);
        buf.write_f32::<LittleEndian>(norm_x).unwrap();
        buf.write_f32::<LittleEndian>(norm_y).unwrap();
        buf
    }

    pub fn mouse_button(button: MouseButtonId, action: ButtonAction) -> Vec<u8> {
        vec![
            MAGIC0,
            MAGIC1,
            PacketType::MouseButton as u8,
            button as u8,
            action as u8,
        ]
    }

    pub fn mouse_wheel(delta_x: i32, delta_y: i32) -> Vec<u8> {
        let mut buf = Vec::with_capacity(11);
        buf.push(MAGIC0);
        buf.push(MAGIC1);
        buf.push(PacketType::MouseWheel as u8);
        buf.write_i32::<LittleEndian>(delta_x).unwrap();
        buf.write_i32::<LittleEndian>(delta_y).unwrap();
        buf
    }

    pub fn key_event(win_vk: u16, action: KeyAction, modifiers: ModifierFlags) -> Vec<u8> {
        let mut buf = Vec::with_capacity(7);
        buf.push(MAGIC0);
        buf.push(MAGIC1);
        buf.push(PacketType::KeyEvent as u8);
        buf.write_u16::<LittleEndian>(win_vk).unwrap();
        buf.push(action as u8);
        buf.push(modifiers.0);
        buf
    }

    pub fn ping(timestamp: u64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(11);
        buf.push(MAGIC0);
        buf.push(MAGIC1);
        buf.push(PacketType::Ping as u8);
        buf.write_u64::<LittleEndian>(timestamp).unwrap();
        buf
    }

    pub fn reset_modifiers() -> Vec<u8> {
        vec![
            MAGIC0,
            MAGIC1,
            PacketType::ResetModifiers as u8,
        ]
    }
}
