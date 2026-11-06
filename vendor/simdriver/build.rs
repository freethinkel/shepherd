use std::path::PathBuf;
use std::process::Command;

const CORE_SIMULATOR: &str =
    "/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator";

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let helpers = manifest.join("helpers");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("helpers");
    std::fs::create_dir_all(&out).unwrap();

    println!("cargo:rerun-if-changed=helpers");

    // (name, extra frameworks)
    let helper_specs: [(&str, &[&str]); 3] = [
        ("sdframe", &["CoreGraphics", "IOSurface", "Accelerate"]),
        ("sdhid", &["CoreGraphics"]),
        ("sdax", &["AppKit"]),
    ];
    for (name, extra) in helper_specs {
        let tmp = out.join(format!("{name}.tmp"));
        let mut cmd = Command::new("clang");
        cmd.args(["-fobjc-arc", "-O2", "-Wall", "-Wno-unused-value"])
            .args(["-framework", "Foundation"]);
        for fw in extra {
            cmd.args(["-framework", fw]);
        }
        cmd.arg("-I")
            .arg(helpers.join("private-headers"))
            .arg(helpers.join(name).join("main.m"))
            .arg(CORE_SIMULATOR)
            .arg("-o")
            .arg(&tmp);
        let status = cmd.status().expect("failed to run clang");
        assert!(status.success(), "clang failed building {name}");
        // Build under a temp name then rename so a running binary is never overwritten in place.
        std::fs::rename(&tmp, out.join(name)).unwrap();
    }

    // sdaxsettings runs inside the simulator (`simctl spawn`), so it's built for it, both Macs' arches.
    let sdk = Command::new("xcrun").args(["--sdk", "iphonesimulator", "--show-sdk-path"]).output().expect("failed to run xcrun");
    assert!(sdk.status.success(), "no iOS Simulator SDK for sdaxsettings");
    let tmp = out.join("sdaxsettings.tmp");
    let status = Command::new("xcrun")
        .args(["--sdk", "iphonesimulator", "clang", "-O2", "-arch", "arm64", "-arch", "x86_64", "-mios-simulator-version-min=15.0"])
        .arg("-isysroot")
        .arg(String::from_utf8_lossy(&sdk.stdout).trim())
        .args(["-framework", "CoreFoundation"])
        .arg(helpers.join("sdaxsettings/main.m"))
        .arg("-o")
        .arg(&tmp)
        .status()
        .expect("failed to run clang");
    assert!(status.success(), "clang failed building sdaxsettings");
    std::fs::rename(&tmp, out.join("sdaxsettings")).unwrap();
}
