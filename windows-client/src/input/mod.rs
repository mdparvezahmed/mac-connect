#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_ESCAPE, VK_F12, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_MENU, VK_PAUSE,
    VK_RCONTROL, VK_RMENU, VK_RWIN, VK_SCROLL, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, SetCursorPos,
    SetWindowsHookExW, ShowCursor, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP,
};

use crate::network::NetworkClient;
use crate::protocol::{ButtonAction, KeyAction, ModifierFlags, MouseButtonId, PacketSerializer};

#[derive(Clone)]
pub struct InputManager {
    is_locked: Arc<AtomicBool>,
    network: NetworkClient,
    thread_id: Arc<RwLock<Option<u32>>>,
}

static mut GLOBAL_INPUT_MGR: Option<InputManager> = None;
static mut KEYBOARD_HOOK: HHOOK = std::ptr::null_mut();
static mut MOUSE_HOOK: HHOOK = std::ptr::null_mut();

// Internal state tracking
static mut LOCK_CENTER_X: i32 = 0;
static mut LOCK_CENTER_Y: i32 = 0;
static mut IS_CTRL_DOWN: bool = false;
static mut IS_ALT_DOWN: bool = false;
static mut IS_SHIFT_DOWN: bool = false;
static mut IS_WIN_DOWN: bool = false;

impl InputManager {
    pub fn new(network: NetworkClient) -> Self {
        Self {
            is_locked: Arc::new(AtomicBool::new(false)),
            network,
            thread_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.is_locked.load(Ordering::Relaxed)
    }

    pub fn set_locked(&self, locked: bool) {
        let old_val = self.is_locked.swap(locked, Ordering::SeqCst);
        if old_val == locked {
            return;
        }

        unsafe {
            if locked {
                // Get current cursor location in real screen pixels
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                LOCK_CENTER_X = pt.x;
                LOCK_CENTER_Y = pt.y;

                // Hide cursor
                ShowCursor(0);
            } else {
                // Show cursor
                ShowCursor(1);

                // Reset internal key tracker
                IS_CTRL_DOWN = false;
                IS_ALT_DOWN = false;
                IS_SHIFT_DOWN = false;
                IS_WIN_DOWN = false;

                // Send reset modifiers packet to Mac
                let reset_pkt = PacketSerializer::reset_modifiers();
                self.network.send(&reset_pkt);
            }
        }
    }

    pub fn start_hooks(&self) {
        unsafe {
            GLOBAL_INPUT_MGR = Some(self.clone());
        }

        let mgr_clone = self.clone();
        std::thread::spawn(move || {
            unsafe {
                let tid = windows_sys::Win32::System::Threading::GetCurrentThreadId();
                *mgr_clone.thread_id.write() = Some(tid);

                let h_instance = std::ptr::null_mut();
                KEYBOARD_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keyboard_proc), h_instance, 0);
                MOUSE_HOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(ll_mouse_proc), h_instance, 0);

                if KEYBOARD_HOOK.is_null() || MOUSE_HOOK.is_null() {
                    eprintln!("Failed to install low-level input hooks!");
                }

                let mut msg = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    DispatchMessageW(&msg);
                }

                if !KEYBOARD_HOOK.is_null() {
                    UnhookWindowsHookEx(KEYBOARD_HOOK);
                    KEYBOARD_HOOK = std::ptr::null_mut();
                }
                if !MOUSE_HOOK.is_null() {
                    UnhookWindowsHookEx(MOUSE_HOOK);
                    MOUSE_HOOK = std::ptr::null_mut();
                }
            }
        });
    }
}

// Low-level Keyboard Hook Callback
unsafe extern "system" fn ll_keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(KEYBOARD_HOOK, code, wparam, lparam);
    }

    if let Some(ref mgr) = GLOBAL_INPUT_MGR {
        let is_locked = mgr.is_locked.load(Ordering::Relaxed);
        let kbd = *(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode as u16;

        let is_down = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => true,
            WM_KEYUP | WM_SYSKEYUP => false,
            _ => false,
        };

        // Track modifiers explicitly so we never depend on async Windows OS state
        match vk {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => IS_CTRL_DOWN = is_down,
            VK_MENU | VK_LMENU | VK_RMENU => IS_ALT_DOWN = is_down,
            VK_SHIFT => IS_SHIFT_DOWN = is_down,
            VK_LWIN | VK_RWIN => IS_WIN_DOWN = is_down,
            _ => {}
        }

        // Host Escape Keys:
        // 1. Escape key (VK_ESCAPE) -> Unlocks immediately
        // 2. Ctrl + Alt combo -> Unlocks immediately
        // 3. F12 or Pause or ScrollLock -> Unlocks immediately
        let is_host_escape = (IS_CTRL_DOWN && IS_ALT_DOWN)
            || vk == VK_ESCAPE
            || vk == VK_F12
            || vk == VK_PAUSE
            || vk == VK_SCROLL;

        if is_host_escape && is_down {
            if is_locked {
                mgr.set_locked(false);
                return 1; // Suppress host escape key from triggering Windows OS actions
            }
        }

        // If not locked, let ALL keys pass cleanly to Windows!
        if !is_locked {
            return CallNextHookEx(KEYBOARD_HOOK, code, wparam, lparam);
        }

        // When locked, stream keystrokes to Mac (Swapped: Alt is Command ⌘, Win is Option ⌥)
        let mut modifiers = ModifierFlags::new();
        modifiers.set_shift(IS_SHIFT_DOWN);
        modifiers.set_ctrl(IS_CTRL_DOWN);
        modifiers.set_alt(IS_WIN_DOWN);   // Win key becomes Option (⌥)
        modifiers.set_win(IS_ALT_DOWN);   // Alt key becomes Command (⌘)

        let action = if is_down { KeyAction::KeyDown } else { KeyAction::KeyUp };
        let pkt = PacketSerializer::key_event(vk, action, modifiers);
        mgr.network.send(&pkt);

        // Suppress key from triggering local Windows actions while focused on Mac
        return 1;
    }

    CallNextHookEx(KEYBOARD_HOOK, code, wparam, lparam)
}

// Low-level Mouse Hook Callback
unsafe extern "system" fn ll_mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(MOUSE_HOOK, code, wparam, lparam);
    }

    if let Some(ref mgr) = GLOBAL_INPUT_MGR {
        let is_locked = mgr.is_locked.load(Ordering::Relaxed);

        // If not locked, let ALL mouse events pass directly to Windows with zero interference!
        if !is_locked {
            return CallNextHookEx(MOUSE_HOOK, code, wparam, lparam);
        }

        let mouse_data = *(lparam as *const MSLLHOOKSTRUCT);
        let msg = wparam as u32;

        // Skip synthetic / injected mouse movements (e.g. from our own SetCursorPos)
        if (mouse_data.flags & 1) != 0 {
            return CallNextHookEx(MOUSE_HOOK, code, wparam, lparam);
        }

        match msg {
            WM_MOUSEMOVE => {
                let dx = mouse_data.pt.x - LOCK_CENTER_X;
                let dy = mouse_data.pt.y - LOCK_CENTER_Y;

                if dx != 0 || dy != 0 {
                    let pkt = PacketSerializer::mouse_move_relative(dx, dy);
                    mgr.network.send(&pkt);

                    // Re-center mouse cursor back to lock point
                    SetCursorPos(LOCK_CENTER_X, LOCK_CENTER_Y);
                }
                return 1;
            }

            WM_LBUTTONDOWN => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Left, ButtonAction::Press);
                mgr.network.send(&pkt);
                return 1;
            }
            WM_LBUTTONUP => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Left, ButtonAction::Release);
                mgr.network.send(&pkt);
                return 1;
            }

            WM_RBUTTONDOWN => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Right, ButtonAction::Press);
                mgr.network.send(&pkt);
                return 1;
            }
            WM_RBUTTONUP => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Right, ButtonAction::Release);
                mgr.network.send(&pkt);
                return 1;
            }

            WM_MBUTTONDOWN => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Middle, ButtonAction::Press);
                mgr.network.send(&pkt);
                return 1;
            }
            WM_MBUTTONUP => {
                let pkt = PacketSerializer::mouse_button(MouseButtonId::Middle, ButtonAction::Release);
                mgr.network.send(&pkt);
                return 1;
            }

            WM_MOUSEWHEEL => {
                let raw_delta = (mouse_data.mouseData >> 16) as i16 as i32;
                let pkt = PacketSerializer::mouse_wheel(0, raw_delta);
                mgr.network.send(&pkt);
                return 1;
            }

            WM_MOUSEHWHEEL => {
                let raw_delta = (mouse_data.mouseData >> 16) as i16 as i32;
                let pkt = PacketSerializer::mouse_wheel(raw_delta, 0);
                mgr.network.send(&pkt);
                return 1;
            }

            _ => {}
        }
    }

    CallNextHookEx(MOUSE_HOOK, code, wparam, lparam)
}
