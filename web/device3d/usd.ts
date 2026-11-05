// Apple's AR models (the USDZ files apple.com serves for its product pages), turned into what t3code's viewer
// expects of its GLBs: the body alone, the screen a flat mesh named `device-screen`, centered, 2.2 units tall and
// facing +Z. t3code does this offline in Blender (scripts/convert-device-model.py); this is the same math.
import { BufferGeometry, Euler, Float32BufferAttribute, Group, Matrix3, Matrix4, Mesh, Quaternion, Vector3, type Object3D } from "three";
import { toCreasedNormals } from "three/addons/utils/BufferGeometryUtils.js";
import { unzipSync, zipSync } from "three/addons/libs/fflate.module.js";
import { USDLoader } from "three/addons/loaders/USDLoader.js";
import { USDComposer } from "three/addons/loaders/usd/USDComposer.js";

// three's composer knows `xformOp:translate:pivot` but no other suffixed op (`xformOp:translate:rotatePivot`,
// `…:scalePivot`: Maya's pivots, which Apple's models use), and skips them: a part rotated about its pivot lands
// rotated about the origin instead, off the device. USD reads any `xformOp:<kind>:<suffix>` as `<kind>`; so does this,
// for prims that have one, handing the rest to three's own.
const applyTransform = USDComposer.prototype.applyTransform;
USDComposer.prototype.applyTransform = function (obj: Object3D, fields: Record<string, any>, attrs: Record<string, any> = {}) {
  const data = { ...fields, ...attrs };
  const order: string[] = data["xformOpOrder"] ?? [];
  if (!order.some((op) => op.split(":").length > 2 && !op.endsWith("xformOp:translate:pivot")))
    return applyTransform.call(this, obj, fields, attrs);
  const matrix = new Matrix4();
  const step = new Matrix4();
  for (const raw of order) {
    const inverse = raw.startsWith("!invert!");
    const op = inverse ? raw.slice("!invert!".length) : raw;
    const v = data[op];
    if (v === undefined) continue;
    const rad = (deg: number) => (deg * Math.PI) / 180;
    switch (op.split(":")[1]) {
      case "translate":
        step.makeTranslation(v[0], v[1], v[2]);
        break;
      case "scale":
        typeof v === "number" ? step.makeScale(v, v, v) : step.makeScale(v[0], v[1], v[2]);
        break;
      case "rotateX":
        step.makeRotationX(rad(v));
        break;
      case "rotateY":
        step.makeRotationY(rad(v));
        break;
      case "rotateZ":
        step.makeRotationZ(rad(v));
        break;
      case "rotateXYZ":
        step.makeRotationFromEuler(new Euler(rad(v[0]), rad(v[1]), rad(v[2]), "ZYX"));
        break;
      case "orient":
        step.makeRotationFromQuaternion(new Quaternion(v[0], v[1], v[2], v[3]));
        break;
      case "transform":
        step.set(v[0], v[4], v[8], v[12], v[1], v[5], v[9], v[13], v[2], v[6], v[10], v[14], v[3], v[7], v[11], v[15]);
        break;
      default:
        continue;
    }
    if (inverse) step.invert();
    matrix.multiply(step);
  }
  obj.matrix.copy(matrix);
  obj.matrix.decompose(obj.position, obj.quaternion, obj.scale);
};

/** Which node is the body and which its screen, per model (from t3code's models/sources.json). */
const NODES: Record<string, { body: string; screen: string }> = {
  "iphone-18-pro": { body: "UBGArkKGrAMRRnj", screen: "EHjrGDHNHzgNjtE" },
  "iphone-18-pro-max": { body: "wUOhcMgiBmCgGaw", screen: "EHjrGDHNHzgNjtE" },
  "ipad-pro-13-m5": { body: "zRrSLDpdYmKeRJQ", screen: "lsDiIbtoSGSmWWZ" },
};

/**
 * Apple's root layer pulls its geometry in through a payload, which three's USDC parser skips: the layer that holds
 * the meshes goes first in the archive, which makes it the root.
 */
export function parseAppleUsdz(bytes: Uint8Array) {
  const zip = unzipSync(bytes);
  const names = Object.keys(zip);
  const nested = names.filter((n) => n.endsWith(".usdc") && n.includes("/"));
  const ordered: Record<string, Uint8Array> = {};
  for (const name of [...nested, ...names.filter((n) => !nested.includes(n))]) ordered[name] = zip[name];
  const archive = zipSync(ordered, { level: 0 });
  return new USDLoader().parse(archive.buffer as ArrayBuffer);
}

/** Least squares of `rows` (each [u, v, 1]) against `points`: how a point moves per unit of u and of v. */
function uvAxes(rows: number[][], points: Vector3[]) {
  const ata = new Matrix3().set(0, 0, 0, 0, 0, 0, 0, 0, 0);
  const atp = [new Vector3(), new Vector3(), new Vector3()];
  const e = ata.elements;
  rows.forEach((r, k) => {
    for (let i = 0; i < 3; i++) {
      for (let j = 0; j < 3; j++) e[j * 3 + i] += r[i] * r[j];
      atp[i].addScaledVector(points[k], r[i]);
    }
  });
  const inv = ata.clone().invert().elements;
  const solve = (i: number) =>
    new Vector3()
      .addScaledVector(atp[0], inv[0 * 3 + i])
      .addScaledVector(atp[1], inv[1 * 3 + i])
      .addScaledVector(atp[2], inv[2 * 3 + i]);
  return { right: solve(0).normalize(), up: solve(1).normalize() };
}

export function normalizeAppleModel(root: Object3D, id: string) {
  const nodes = NODES[id];
  if (!nodes) throw new Error(`No nodes known for model ${id}`);
  root.updateMatrixWorld(true);
  const body = root.getObjectByName(nodes.body);
  if (!body) throw new Error(`Model ${id} has no body ${nodes.body}`);
  const meshes: Mesh[] = [];
  body.traverse((o) => {
    if (o instanceof Mesh) meshes.push(o);
  });
  const screen = meshes.find((m) => m.name === nodes.screen);
  if (!screen) throw new Error(`Model ${id} has no screen ${nodes.screen}`);
  // the screen's authored UVs give the device's front and its portrait axes
  const position = screen.geometry.getAttribute("position");
  const uv = screen.geometry.getAttribute("uv");
  const points: Vector3[] = [];
  const rows: number[][] = [];
  for (let i = 0; i < position.count; i++) {
    points.push(new Vector3().fromBufferAttribute(position, i).applyMatrix4(screen.matrixWorld));
    rows.push([uv.getX(i), uv.getY(i), 1]);
  }
  const { right, up } = uvAxes(rows, points);
  const normal = new Vector3().crossVectors(right, up).normalize();
  const rot = new Matrix4().makeBasis(right, up, normal).transpose();
  const turned = points.map((p) => p.clone().applyMatrix4(rot));
  const lo = turned.reduce((a, p) => a.min(p), turned[0].clone());
  const hi = turned.reduce((a, p) => a.max(p), turned[0].clone());
  const center = lo.clone().add(hi).multiplyScalar(0.5);
  const scale = 2.2 / (hi.y - lo.y);
  const transform = new Matrix4()
    .makeTranslation(0, 0, 0.043)
    .multiply(new Matrix4().makeScale(scale, scale, scale))
    .multiply(new Matrix4().makeTranslation(-center.x, -center.y, -center.z))
    .multiply(rot);
  const out = new Group();
  for (const mesh of meshes) {
    mesh.geometry = mesh.geometry.clone().applyMatrix4(new Matrix4().multiplyMatrices(transform, mesh.matrixWorld));
    mesh.position.set(0, 0, 0);
    mesh.quaternion.identity();
    mesh.scale.set(1, 1, 1);
    out.add(mesh);
  }
  // flat at its own z, so the viewer's projection plane is the screen's
  const z = screen.geometry.getAttribute("position");
  let zSum = 0;
  for (let i = 0; i < z.count; i++) zSum += z.getZ(i);
  const zMean = zSum / z.count;
  for (let i = 0; i < z.count; i++) z.setZ(i, zMean);
  z.needsUpdate = true;
  screen.name = "device-screen";
  // Apple's meshes come without normals; the loader's own average them across hard edges too, which streaks a
  // bevel's light over the faces beside it. Edges sharper than 30° keep their faces' own.
  for (const mesh of meshes) mesh.geometry = toCreasedNormals(mesh.geometry, Math.PI / 6);
  return out;
}

/**
 * iPhone Duo, the rig t3code's viewer (duoScene.ts) folds: Apple's model in its own centimeters, stood up with the
 * inner screen to +Z and the camera end up, its halves as `left-half` and `right-half` hinged on the Y axis through the
 * origin, and its displays named. Apple's lies open flat; the inner display, its bezel and the hinge span both halves,
 * so each is cut in two at the hinge. t3code's GLB holds the same, from serve-sim's conversion.
 */
const DUO = {
  left: "upTUAKvMVkPOMKq",
  right: "SiftyleUEEZwLhF",
  cover: "hhgAIoCGsHXeDPY",
  across: { JnJdTkxbQgUtLwU: "inner-bezel", UXtsBZYlaUvHoEh: "inner-display", MvKPXGSdYDVvSpk: "hinge", xdyyaajWsatVNxN: "hinge-cap" } as Record<string, string>,
};

/** `mesh`'s triangles either side of x = 0, each side its own mesh. */
function cutAtHinge(mesh: Mesh, name: string): Mesh[] {
  const geometry = mesh.geometry.index ? mesh.geometry.toNonIndexed() : mesh.geometry;
  const position = geometry.getAttribute("position");
  const names = Object.keys(geometry.attributes);
  const sides = [0, 1].map(() => Object.fromEntries(names.map((n) => [n, [] as number[]])));
  for (let t = 0; t < position.count; t += 3) {
    const side = position.getX(t) + position.getX(t + 1) + position.getX(t + 2) < 0 ? 0 : 1;
    for (const n of names) {
      const a = geometry.getAttribute(n);
      for (let v = t; v < t + 3; v++) for (let c = 0; c < a.itemSize; c++) sides[side][n].push(a.array[v * a.itemSize + c]);
    }
  }
  return sides.map((side, i) => {
    const half = new BufferGeometry();
    for (const n of names) half.setAttribute(n, new Float32BufferAttribute(side[n], geometry.getAttribute(n).itemSize));
    const piece = new Mesh(half, mesh.material);
    piece.name = `${name}-${i === 0 ? "left" : "right"}`;
    return piece;
  });
}

export function normalizeAppleDuo(root: Object3D) {
  root.updateMatrixWorld(true);
  const left = root.getObjectByName(DUO.left);
  const right = root.getObjectByName(DUO.right);
  if (!left || !right) throw new Error("iPhone Duo model has no halves");
  // flat on its back, screen up (+Y), the camera end toward -Z: a quarter turn about X stands it up
  const stand = new Matrix4().makeRotationX(Math.PI / 2);
  const baked = (mesh: Mesh) => {
    mesh.geometry = mesh.geometry.clone().applyMatrix4(new Matrix4().multiplyMatrices(stand, mesh.matrixWorld));
    mesh.position.set(0, 0, 0);
    mesh.quaternion.identity();
    mesh.scale.set(1, 1, 1);
    return mesh;
  };
  const halves = [new Group(), new Group()];
  halves[0].name = "left-half";
  halves[1].name = "right-half";
  const meshes = (o: Object3D) => {
    const found: Mesh[] = [];
    o.traverse((m) => {
      if (m instanceof Mesh) found.push(m);
    });
    return found;
  };
  for (const [i, half] of [left, right].entries()) for (const mesh of meshes(half)) halves[i].add(baked(mesh));
  for (const mesh of meshes(root)) {
    const name = DUO.across[mesh.name];
    if (!name) continue;
    cutAtHinge(baked(mesh), name).forEach((piece, i) => halves[i].add(piece));
  }
  const cover = halves[0].getObjectByName(DUO.cover);
  if (!cover) throw new Error("iPhone Duo model has no cover display");
  cover.name = "cover-display";
  for (const half of halves)
    for (const mesh of meshes(half)) mesh.geometry = toCreasedNormals(mesh.geometry, Math.PI / 6);
  const duo = new Group();
  duo.name = "iphone-duo";
  duo.add(...halves);
  return duo;
}
