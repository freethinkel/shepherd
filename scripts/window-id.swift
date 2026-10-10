// Prints the window number of the first on-screen window of the process named in argv[1], optionally with the
// title argv[2] (from Collie's scripts/window-id.swift).
import CoreGraphics
let owner = CommandLine.arguments[1]
let title = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : nil
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
for w in list where (w[kCGWindowOwnerName as String] as? String) == owner && (w[kCGWindowLayer as String] as? Int) == 0 {
    if let title, (w[kCGWindowName as String] as? String) != title { continue }
    print(w[kCGWindowNumber as String] as? Int ?? 0)
    break
}
