// Shepherd's 3D device view: t3code's viewer (./t3code, MIT) in a web view over the Device tab. Frames come from
// Shepherd over its `shepherd-device` scheme, a foldable's hinge in their headers; touches, keys and hinge moves go back
// over the IPC channel.
import { createDuoPinch } from "./t3code/duoControl.ts";
import { createDuoViewer } from "./t3code/duoViewer.ts";
import { resolveDeviceModelId } from "./t3code/model.ts";
import { createPhoneInteraction } from "./t3code/phoneInteraction.ts";
import { bindPhoneTrackpad } from "./t3code/phoneTrackpad.ts";
import { createPhoneViewer } from "./t3code/phoneViewer.ts";
import { resolveDeviceShape } from "./t3code/shapeProfile.ts";
import type { DevicePlatform, DeviceScreenSize } from "./types.ts";

declare global {
  interface Window {
    ipc: { postMessage: (message: string) => void };
    shepherdReset: () => void;
  }
}

/** A foldable's hinge as Shepherd sends it (`crate::device::Fold`). */
type Fold = {
  duo: boolean;
  angle: number;
  stand: "laptop" | "tent" | null;
  orientation: DeviceScreenSize["orientation"];
};

const params = new URLSearchParams(location.search);
const platform = (params.get("platform") === "android" ? "android" : "ios") as DevicePlatform;
const name = params.get("name") ?? "";
document.body.style.background = params.get("bg") ?? "transparent";

const post = (message: object) => window.ipc.postMessage(JSON.stringify(message));

const canvas = document.createElement("canvas");
canvas.style.cssText = "position:fixed;inset:0;width:100%;height:100%;touch-action:none;outline:none";
canvas.tabIndex = 0;
document.body.append(canvas);
const surface = (width: number, height: number) => Object.assign(document.createElement("canvas"), { width, height });
const source = surface(900, 1950);

const modelId = resolveDeviceModelId(platform, name);
const model = modelId ? { id: modelId, url: `model/${modelId}` } : null;
// Shepherd shows the page once the device is drawn as it'll stay: its model in (or given up on) and a frame on it
let modelSettled = !modelId;
let framed = false;
let announced = false;
const settle = () => {
  if (announced || !modelSettled || !framed) return;
  announced = true;
  // not after a frame: WebKit runs no animation frames for a hidden page, and Shepherd hides it until this
  post({ ready: true });
};
const modelOptions = {
  onUnavailable: () => post({ note: "3D view unavailable" }),
  onModelError: (cause: unknown) => {
    post({ note: `Device model: ${String(cause)}` });
    modelSettled = true;
    settle();
  },
  onModelReady: () => {
    modelSettled = true;
    settle();
  },
};

// iPhone Duo folds in t3code's own viewer, its two screens each on a canvas of their own, in the frame's raw way round
// (Shepherd sends the inner one turned upright; the viewer turns it itself)
const duoSources = { 1: surface(1398, 2034), 3: surface(2007, 2853) };
const duo =
  modelId === "iphone-duo" && model
    ? createDuoViewer({
        canvas,
        sources: duoSources,
        model,
        ...modelOptions,
        onOrientationRequested: (orientation) => post({ hold: orientation }),
      })
    : null;
const phone = duo
  ? null
  : createPhoneViewer({
      canvas,
      source,
      ...modelOptions,
      profile: resolveDeviceShape({ platform, name, portraitAspect: 0.46 }),
      model,
    });
const viewer = (duo ?? phone)!;
window.shepherdReset = () => viewer.resetPose();

const resize = () => viewer.resize(innerWidth, innerHeight, devicePixelRatio);
addEventListener("resize", resize);
resize();

let duoScreen: DeviceScreenSize | null = null;
const screenKey = (s: DeviceScreenSize | null) => JSON.stringify(s);

/** Draws a Duo frame on its screen's canvas, the inner one turned back, and tells the viewer which screen is live. */
function showDuo(bitmap: ImageBitmap, screenId: 1 | 3, fold: Fold | null) {
  const inner = screenId === 3;
  const target = duoSources[screenId];
  const [width, height] = inner ? [bitmap.height, bitmap.width] : [bitmap.width, bitmap.height];
  if (target.width !== width || target.height !== height) Object.assign(target, { width, height });
  const context = target.getContext("2d")!;
  context.save();
  if (inner) {
    // a quarter turn back, counterclockwise: the frame's top-left lands bottom-left
    context.translate(0, height);
    context.rotate(-Math.PI / 2);
  }
  context.drawImage(bitmap, 0, 0);
  context.restore();
  const next: DeviceScreenSize = {
    width,
    height,
    orientation: fold?.orientation ?? "portrait",
    screenId,
    supportsHingeAngle: true,
    supportsPhysicalOrientation: false,
    hingeAngle: fold?.angle ?? (inner ? 180 : 0),
    hingePose: fold?.stand ?? null,
  };
  if (screenKey(next) !== screenKey(duoScreen)) duo!.setScreen((duoScreen = next));
  duo!.frameUpdated(screenId);
}

// the latest frame only: each request waits for one newer than the last
async function pump() {
  let seq = 0;
  for (;;) {
    try {
      const response = await fetch(`frame?after=${seq}`);
      if (response.status !== 200) continue;
      seq = Number(response.headers.get("x-seq") ?? seq);
      const fold = JSON.parse(response.headers.get("x-fold") ?? "null") as Fold | null;
      const bitmap = await createImageBitmap(await response.blob());
      if (duo) {
        showDuo(bitmap, response.headers.get("x-screen") === "1" ? 1 : 3, fold);
      } else {
        if (source.width !== bitmap.width || source.height !== bitmap.height)
          Object.assign(source, { width: bitmap.width, height: bitmap.height });
        source.getContext("2d")!.drawImage(bitmap, 0, 0);
        phone!.frameUpdated();
        // an Android foldable's own scene, folded as far as its hinge
        phone!.setFoldAngle(fold && !fold.duo ? fold.angle : null);
      }
      bitmap.close();
      framed = true;
      settle();
    } catch {
      await new Promise((r) => setTimeout(r, 500));
    }
  }
}
void pump();

const interaction = createPhoneInteraction({
  screenPoint: (point, captured) => viewer.screenPoint(point.x, point.y, captured),
  touch: (phase, point) => post({ touch: phase, x: point.x, y: point.y }),
  orbit: viewer.orbit,
  zoomBy: () => {},
  onInteractionActive: viewer.setInteractionActive,
});
// a pinch over the Duo opens or closes its hinge, as t3code's
const pinch = duo
  ? createDuoPinch({
      angle: () => duoScreen?.hingeAngle ?? 180,
      contains: (x, y) => !!duoScreen && duo.beginHinge(x, y),
      change: (angle) => {
        duo.setHingePreview(angle, true);
        if (angle !== null) post({ hinge: angle });
      },
    })
  : undefined;
const at = (e: PointerEvent) => {
  const rect = canvas.getBoundingClientRect();
  return { x: (e.clientX - rect.left) / rect.width, y: (e.clientY - rect.top) / rect.height };
};
canvas.addEventListener("pointerdown", (e) => {
  canvas.focus();
  canvas.setPointerCapture(e.pointerId);
  // the screen takes a touch; anywhere else, or another button, turns the device
  interaction.begin(e.pointerId, at(e), e.button !== 0);
});
canvas.addEventListener("pointermove", (e) => interaction.move(e.pointerId, at(e)));
for (const type of ["pointerup", "pointercancel", "lostpointercapture"] as const)
  canvas.addEventListener(type, (e) => interaction.end(e.pointerId));
canvas.addEventListener("contextmenu", (e) => e.preventDefault());
bindPhoneTrackpad(
  canvas,
  interaction,
  pinch && {
    begin(x, y) {
      interaction.end();
      return pinch.begin(x, y);
    },
    move: pinch.move,
    end: pinch.end,
  },
);

// W3C key names to GPUI's, which Shepherd's sessions take
const KEYS: Record<string, string> = {
  Enter: "enter",
  Backspace: "backspace",
  Delete: "delete",
  Tab: "tab",
  Escape: "escape",
  ArrowLeft: "left",
  ArrowRight: "right",
  ArrowUp: "up",
  ArrowDown: "down",
  Home: "home",
  End: "end",
  PageUp: "pageup",
  PageDown: "pagedown",
};
addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey) return;
  const key = KEYS[e.key];
  if (key) post({ key });
  else if (e.key.length === 1) post({ text: e.key });
  else return;
  e.preventDefault();
});
