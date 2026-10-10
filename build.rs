use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/ghostty");
    let header = dir.join("ghostty.h");
    assert!(header.exists(), "run `make ghostty` first: {} is missing", header.display());
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=ghostty");
    // ponytail: rpath to vendor/ for dev builds; the app bundle (plan 2) copies the dylib into Frameworks.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .allowlist_function("ghostty_.*")
        .allowlist_type("ghostty_.*")
        .allowlist_var("GHOSTTY_.*")
        .prepend_enum_name(false)
        .generate()
        .expect("bindgen ghostty.h")
        .write_to_file(out.join("ghostty.rs"))
        .unwrap();
}
