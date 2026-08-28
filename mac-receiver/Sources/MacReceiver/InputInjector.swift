import Foundation
import CoreGraphics
import ApplicationServices

public class InputInjector {
    private let keyMapper: KeyMapper
    private var isLeftDown = false
    private var isRightDown = false
    private var isMiddleDown = false
    private var mouseSensitivity: Double = 1.0

    public init(keyMapper: KeyMapper, mouseSensitivity: Double = 1.0) {
        self.keyMapper = keyMapper
        self.mouseSensitivity = mouseSensitivity
    }

    public func setSensitivity(_ val: Double) {
        self.mouseSensitivity = max(0.1, min(5.0, val))
    }

    /// Moves cursor by relative offset (dx, dy)
    public func moveMouseRelative(dx: Int32, dy: Int32) {
        let currentPos = getCurrentMousePosition()
        let scaledDx = Double(dx) * mouseSensitivity
        let scaledDy = Double(dy) * mouseSensitivity

        let mainBounds = CGDisplayBounds(CGMainDisplayID())
        let newX = max(mainBounds.minX, min(mainBounds.maxX - 1, currentPos.x + CGFloat(scaledDx)))
        let newY = max(mainBounds.minY, min(mainBounds.maxY - 1, currentPos.y + CGFloat(scaledDy)))
        let newLocation = CGPoint(x: newX, y: newY)

        let eventType: CGEventType
        let mouseButton: CGMouseButton
        if isLeftDown {
            eventType = .leftMouseDragged
            mouseButton = .left
        } else if isRightDown {
            eventType = .rightMouseDragged
            mouseButton = .right
        } else if isMiddleDown {
            eventType = .otherMouseDragged
            mouseButton = .center
        } else {
            eventType = .mouseMoved
            mouseButton = .left
        }

        if let event = CGEvent(mouseEventSource: nil, mouseType: eventType, mouseCursorPosition: newLocation, mouseButton: mouseButton) {
            event.setIntegerValueField(.mouseEventDeltaX, value: Int64(dx))
            event.setIntegerValueField(.mouseEventDeltaY, value: Int64(dy))
            event.post(tap: .cghidEventTap)
        }
    }

    /// Moves cursor to absolute normalized screen coordinates (0.0 to 1.0)
    public func moveMouseAbsolute(normX: Float, normY: Float) {
        let mainBounds = CGDisplayBounds(CGMainDisplayID())
        let targetX = mainBounds.minX + CGFloat(normX) * mainBounds.width
        let targetY = mainBounds.minY + CGFloat(normY) * mainBounds.height
        let targetLocation = CGPoint(x: targetX, y: targetY)

        let eventType: CGEventType = isLeftDown ? .leftMouseDragged : (isRightDown ? .rightMouseDragged : .mouseMoved)
        let mouseButton: CGMouseButton = isLeftDown ? .left : (isRightDown ? .right : .left)

        if let event = CGEvent(mouseEventSource: nil, mouseType: eventType, mouseCursorPosition: targetLocation, mouseButton: mouseButton) {
            event.post(tap: .cghidEventTap)
        }
    }

    /// Injects mouse button click / release
    public func handleMouseButton(button: MouseButtonId, action: ButtonAction) {
        let currentPos = getCurrentMousePosition()
        let isDown = (action == .press)

        let eventType: CGEventType
        let cgButton: CGMouseButton

        switch button {
        case .left:
            isLeftDown = isDown
            eventType = isDown ? .leftMouseDown : .leftMouseUp
            cgButton = .left
        case .right:
            isRightDown = isDown
            eventType = isDown ? .rightMouseDown : .rightMouseUp
            cgButton = .right
        case .middle:
            isMiddleDown = isDown
            eventType = isDown ? .otherMouseDown : .otherMouseUp
            cgButton = .center
        case .back:
            eventType = isDown ? .otherMouseDown : .otherMouseUp
            cgButton = .center
        case .forward:
            eventType = isDown ? .otherMouseDown : .otherMouseUp
            cgButton = .center
        }

        if let event = CGEvent(mouseEventSource: nil, mouseType: eventType, mouseCursorPosition: currentPos, mouseButton: cgButton) {
            if button == .back {
                event.setIntegerValueField(.mouseEventNumber, value: 3)
            } else if button == .forward {
                event.setIntegerValueField(.mouseEventNumber, value: 4)
            }
            event.post(tap: .cghidEventTap)
        }
    }

    /// Injects smooth touchpad pixel scrolling
    public func handleMouseWheel(deltaX: Int32, deltaY: Int32) {
        // Create continuous pixel scroll event
        // Note: deltaY > 0 means scroll up (standard)
        if let event = CGEvent(scrollWheelEvent2Source: nil,
                               units: .pixel,
                               wheelCount: 2,
                               wheel1: deltaY,
                               wheel2: deltaX,
                               wheel3: 0) {
            event.post(tap: .cghidEventTap)
        }
    }

    /// Injects keyboard events
    public func handleKeyEvent(winVk: UInt16, action: KeyAction, modifiers: ModifierFlags) {
        guard let macKey = keyMapper.mapWindowsVkToMacKeycode(winVk) else {
            return
        }

        let isDown = (action == .keyDown)
        if let event = CGEvent(keyboardEventSource: nil, virtualKey: macKey, keyDown: isDown) {
            var flags: CGEventFlags = []
            if modifiers.contains(.shift) { flags.insert(.maskShift) }
            if modifiers.contains(.ctrl) {
                if keyMapper.swapCmdAndCtrl {
                    flags.insert(.maskCommand)
                } else {
                    flags.insert(.maskControl)
                }
            }
            if modifiers.contains(.alt) { flags.insert(.maskAlternate) }
            if modifiers.contains(.win) {
                if keyMapper.swapCmdAndCtrl {
                    flags.insert(.maskControl)
                } else {
                    flags.insert(.maskCommand)
                }
            }
            event.flags = flags
            event.post(tap: .cghidEventTap)
        }
    }

    /// Resets all modifier keys and mouse button states
    public func resetStates() {
        isLeftDown = false
        isRightDown = false
        isMiddleDown = false
    }

    private func getCurrentMousePosition() -> CGPoint {
        if let event = CGEvent(source: nil) {
            return event.location
        }
        let bounds = CGDisplayBounds(CGMainDisplayID())
        return CGPoint(x: bounds.midX, y: bounds.midY)
    }
}
