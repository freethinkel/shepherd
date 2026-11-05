# simdriver

Embed a live iOS simulator in your app — or drive one from an AI agent — on macOS.
simdriver streams the simulator's screen as BGRA frames and injects touch, gesture and
keyboard input through the simulator's own private services. No screen recording, no
Appium, no computer use: your process talks to a small, safe driver and never loads a
private framework itself.

```rust,ignore
let driver = simdriver::Driver::start(Default::default(), |event| match event {
    simdriver::Event::Frame(f) => draw(f),                  // BGRA bytes, latest wins
    simdriver::Event::State(s) => println!("{s:?}"),        // live / restarting / …
    _ => {}
})?;
driver.pointer(simdriver::Phase::Down, 0.5, 0.5, Default::default()); // 0..1 in the box
```

An agent layer on top (`simdriver-agent` / the same package in Node) drives the screen
by meaning: it snapshots the accessibility tree, hands out stable element refs (`e4`)
with an annotated screenshot, and runs multi-step flows — validated taps, fills that
report whether they took, alerts, gestures — stopping at the first surprise.

- **Rust crate** `simdriver` (the reference API), **C ABI** (`sd_*`, header included),
  **Node addon** `simdriver` (Electron main, Node, Bun).
- Works with what Xcode gives you: `xcrun simctl`, the framebuffer's IOSurface, the
  `dtuhidd` digitizer service, and the accessibility bridge. Nothing else to install.

## Requirements

- macOS on Apple Silicon, Xcode 27.x with an iOS simulator runtime.
- A booted simulator (booting is the host's explicit call: `simdriver::boot(udid)`).
- For the Node package: Node 20+ / Bun / Electron's main process.

## The host contract

Draw the frames you receive; forward the input that lands in your box, with positions
normalized to **0..1 inside it**. Everything else — device points, edge zones, scale
factors, HID codes, process supervision, gesture interpretation — is the driver's.

Driver states: `starting → live`, `input-displaced` (someone else took the touch
input — frames keep streaming; your next touch reclaims it), `restarting` (the
simulator rebooted; frames resume), `stopped`. `stop()` is terminal and idempotent;
helpers never outlive the driver. One live driver per device per process.

Performance, measured on an iPhone 18 Pro simulator driving a CSS animation at
603×1311 (scale 2): ~60 fps, frame-gap p50 16.7 ms / p99 23 ms, tap → first frame
~9 ms median. The integration tests assert ≥ 55 fps and p99 ≤ 34 ms on an otherwise
quiet machine.

## Quickstart: Bun or Node

```js
import { Driver } from 'simdriver';

const driver = await Driver.start({}, (e) => {
  if (e.type === 'frame') myCanvasDraw(e.width, e.height, e.data); // BGRA Buffer
});
driver.pointer('down', 0.5, 0.5);
driver.pointer('up', 0.5, 0.5);

// The agent layer, same package:
const agent = driver.agent();
const snap = await agent.snapshot();
console.log(snap.text);                 // "e1 button \"General\" …"
const report = await agent.steps([
  { action: 'tap', target: 'General' },
  { action: 'fill', target: 'Search', value: 'keyboard' },
  { action: 'key', name: 'enter' },
]);
```

Every action, option, and error code is listed in
[`docs/simdriver-controls.md`](docs/simdriver-controls.md).

## Quickstart: the command line

`cargo run -p simdriver-cli --` (or install the `simdriver` binary) drives a booted
simulator one command at a time — the agent layer, with its refs remembered between
calls in `~/.cache/simdriver/`:

```sh
simdriver devices
simdriver agent snapshot --image shot.png   # elements numbered e1, e2… + annotated shot
simdriver screenshot shot.png               # plain PNG of the screen, no annotations
simdriver agent tap e4                      # validated against the live screen first
simdriver agent steps '[{"action":"tap","target":"Log in"},{"action":"waitFor","text":"Welcome"}]'
simdriver agent record -- tap "Save"        # contact sheet + timed frames of what the tap sets off
simdriver set permission grant photos com.example.app
simdriver reset app-data com.example.app
```

`--udid` picks the simulator (default: the only booted one), `--json` on the listing
commands prints machine-readable output. Run `simdriver` with no arguments for everything.

## Quickstart: Electron

`examples/electron` — driver in the main process, frames to the renderer over
`postMessage`, input back over IPC:

```js
const { Driver } = require('simdriver');
driver = await Driver.start({ udid }, (e) => {
  if (e.type === 'frame') win.webContents.postMessage('frame', { width: e.width, height: e.height, data: e.data });
});
```

Run it: `cd examples/electron && bun install && bun start` (or `npm start`).

## Quickstart: Rust

`examples/rust-minimal` — a winit window painting frames through softbuffer, mouse and
wheel forwarded as normalized input:

```sh
cargo run --release -p rust-minimal -- --seconds 5    # prints fps every second
```

The full Rust API: `Driver::{start, pointer, scroll, touch2, tap, long_press, drag,
key, key_combo, text, button, capture, accessibility, screen_points, stats, stop}`,
plus `list_devices`, `boot`, `shutdown`, `open_url`, `launch`, `terminate`,
`set_location`, `set_appearance`, `grant_permission`, `clipboard_read`,
`clipboard_write`.

## C ABI

`crates/simdriver/include/simdriver.h` (regenerate with
`cargo run -p simdriver --example gen_header`); link the cdylib or staticlib. Every
entry point is NULL-safe and panic-caught; `sd_last_error()` is thread-local. The C
smoke test in `tests/c_abi.rs` compiles and runs against the real cdylib.

## Xcode compatibility

The private surfaces simdriver uses (framebuffer, `dtuhidd`, accessibility) drift
between Xcode builds; every call is guarded so an unknown build fails with a clear
error instead of crashing.

| Xcode | Frames | Input | Accessibility | Notes |
| --- | --- | --- | --- | --- |
| 27.0 | ✓ | ✓ | ✓ | The development target; everything is tested here. |
| 27.1 | ✓ | ✓ | ✓ | Simulator pasteboard is broken (`simctl pbcopy` never lands), so non-ASCII `fill`/`text` reports `verified: false`. |

## Known gaps

- **Non-ASCII text on Xcode 27.1** (above): the pasteboard path can't land; ASCII
  typing is unaffected.
- **Keyboard events and "Connect Hardware Keyboard":** `key()`/`text()` send HID
  keyboard events, which need the simulator's *Connect Hardware Keyboard* setting on;
  behaviour with it off is still being characterised.
- **Two-finger input only.** Three- and five-finger gestures need more contacts in the
  HID helper; tracked as a later goal. Physical devices, Android, foldables: out of
  scope for v1.
- **The 60 fps bar is a quiet-machine bar.** At host load 20+ from unrelated apps,
  single stalls of 35–90 ms can appear; the tests re-run on a quiet machine rather
  than loosen.

## Attribution

The private-framework declarations and the DTUHID transport shapes are vendored from
Meta's [facebook/idb](https://github.com/facebook/idb) (MIT) — see `NOTICE`. This
project is not affiliated with or endorsed by Apple or Meta. License: MIT.
