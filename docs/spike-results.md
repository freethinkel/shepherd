# Spike results: libghostty surfaces over herdr

Date: 2026-10-10. Plan: `docs/superpowers/plans/2026-10-10-shepherd-foundation.md`. herdr 0.9.3 (protocol 22),
ghostty `efb3523`, macOS 27.0 (Darwin 27), Apple Silicon.

"Verified" means seen in this session (test output or a screenshot of the running app). "Pending" means it needs
a person at the keyboard; how to run it is at the end.

## 1. A libghostty surface inside a GPUI (zui) window

**Verified.** One `ShepherdSurfaceView` (our `NSView`) per pane, added to GPUI's content view below zui's
`GPUIOverlayView` (`window.enable_scene_overlay()`), positioned from a `gpui::canvas` paint callback. Ghostty draws
into it with the user's own Ghostty config (font, theme, colors). Two panes of one tab sit side by side at
herdr's split; a split made in herdr appears in the window without a restart.

**Pending:** keyboard focus moving between surfaces by click, typing in latin and Russian layouts, ctrl / alt
keys, mouse selection and wheel scrolling, full-screen apps (vim, Claude Code), and whether GPUI's view swallows
Cmd key equivalents (Cmd+C / Cmd+V) before the surface sees them. Known gaps by design: no IME composition
(CJK, dead keys) and no paste (the read-clipboard callback returns false) until plan 2; no `mouseMoved`
tracking area, so hover-only mouse reporting does not reach apps.

**Decision:** keep the approach.

## 2. `herdr terminal session control` driven by the shim

**Verified.** Against a scratch tab: typed input reaches the pane (`echo shepherd-$((6*7))` prints `shepherd-42`),
ANSI frames come back (colors intact), EOF on the shim's stdin ends the session, closing the tab in herdr ends the
shim in under a second, and a pane resized by the window (a split halving it) reflows inside herdr, so
`terminal.resize` from SIGWINCH works. The app's event loop (subscribe, coalesce 30 ms, refetch
`session.snapshot`) keeps up with a live split.

**Pending:** typing latency compared with the herdr TUI; scrollback (Ghostty's own vs herdr's
`terminal.scroll`; the shim never sends `terminal.scroll`, so wheel scrolling in a shell shows Ghostty's local
scrollback of re-encoded frames only); behavior over ssh (remote machines are plan 2).

**Decision:** keep the JSON shim; switch to herdr's binary client protocol only if latency is noticeable.

## 3. Sizing when another client shows the same tab

**Pending.** Not run: it needs the herdr TUI open on the same tab at a different size while Shepherd is
attached. The question to answer: does the last resizer win, do they fight, and does herdr letterbox?

## 4. Building libghostty with Zig at a pinned commit

**Failed on this machine; worked around.** Zig 0.15.2 (the version ghostty `efb3523` requires) cannot link its
own build runner on macOS 27: `libSystem.tbd` in both the Xcode 27 SDK and the Command Line Tools 26.5 SDK
(updated 2026-09-21) lists only `arm64e` (and `arm64e.x1`) targets, so every libc symbol is undefined.
`SDKROOT` does not help because Zig asks `xcrun --sdk macosx --show-sdk-path`, which ignores it; a PATH
`xcrun` shim pointing at the 26.5 SDK fails the same way.

Workaround: `make ghostty GHOSTTY_PREBUILT=<dir>` copies a `libghostty.dylib` built earlier from the same commit
with the same flags (here `flutter_ghostty`'s, built 2026-09-05 against SDK 26.5), then re-ids and ad-hoc signs
it. Linking, `ghostty_init` and rendering work with it.

**Decision:** keep `GHOSTTY_PREBUILT` until either an SDK with an `arm64` slice is available to Zig 0.15.2, or a
ghostty commit that builds with a newer Zig (Homebrew has 0.17.0) is pinned. Bundling the dylib into
`Shepherd.app` is plan 2.

## How to run the pending checks

```sh
make run                                   # shows herdr's focused tab
SHEPHERD_TAB=<tab_id> make run              # or pin a tab (e.g. a scratch one) without moving herdr's focus
```

Launch it from a terminal outside herdr, or from inside herdr with another tab pinned: a Shepherd started in a
herdr pane shows that pane as a placeholder instead of mirroring itself.

# GPUI terminal (plan 2a)

Date: 2026-10-10. Each pane is a `PaneTerminal`: `herdr terminal session control` → Zeron's `alacritty_terminal`
emulator (no local scrollback) → Zeron's grid element, in Geist Mono with Zeron's palette.

**Verified** (screenshots of the running app on a scratch tab, plus the test suite):
- Two panes of one tab render side by side; ANSI 16 colors, bold, underline and backgrounds show; `eza` icons
  render once installed Nerd Fonts are set as fallbacks (Geist Mono has none).
- The pane resizes herdr's view to the measured grid: the first frame at 80×24 is redrawn at the window's size.
- Sessions: frames in, input / resize / scroll / mouse lines out, end reasons, and child cleanup (tests against a
  fake `herdr`, one live echo through a real herdr).
- Wheel deltas accumulate across trackpad ticks (unit test).

**Pending a person at the keyboard:** typing (Cyrillic, ctrl, alt, arrows), wheel scrolling through herdr's
history, vim arrows in normal and insert mode (does herdr re-encode cursor keys for the pane's mode?), drag
selection with Cmd+C and Cmd+V, and whether pasted text shows stray `ESC[200~` in apps without bracketed paste.
