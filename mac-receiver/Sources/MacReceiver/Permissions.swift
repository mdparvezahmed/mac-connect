import Foundation
import ApplicationServices

public struct PermissionChecker {
    /// Checks if Accessibility permission is granted.
    /// If prompt is true and not granted, prompts the macOS System Settings dialog.
    @discardableResult
    public static func checkAccessibility(prompt: Bool = true) -> Bool {
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: prompt] as CFDictionary
        let isTrusted = AXIsProcessTrustedWithOptions(options)

        if !isTrusted {
            print("\n=============================================================")
            print("⚠️  MAC ACCESSIBILITY PERMISSION REQUIRED")
            print("=============================================================")
            print("MacReceiver needs Accessibility permission to inject keyboard")
            print("and mouse events into macOS.")
            print("")
            print("To enable:")
            print("1. Open System Settings -> Privacy & Security -> Accessibility")
            print("2. Add or toggle ON: Terminal (or the compiled MacReceiver binary)")
            print("3. Restart MacReceiver once permission is enabled.")
            print("=============================================================\n")
        }

        return isTrusted
    }
}
