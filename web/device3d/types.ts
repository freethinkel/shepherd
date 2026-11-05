// The few t3code contract types the ported viewer reads.
export type DevicePlatform = "ios" | "android";

export interface DeviceScreenSize {
  readonly width: number;
  readonly height: number;
  readonly orientation: "portrait" | "portrait_upside_down" | "landscape_left" | "landscape_right";
  // iPhone Duo's, from Shepherd's frame headers
  readonly screenId?: number;
  readonly supportsHingeAngle?: boolean;
  readonly supportsPhysicalOrientation?: boolean;
  readonly hingeAngle?: number;
  readonly hingePose?: "closed" | "book" | "open" | "laptop" | "tent" | null;
}
