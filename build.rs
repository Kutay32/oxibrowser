use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => setup_macos_bundle(),
        "ios" => setup_ios_build(),
        _ => {}
    }

    println!("cargo:rerun-if-changed=assets/Info.plist");
    println!("cargo:rerun-if-changed=assets/Info-iOS.plist");
    println!("cargo:rerun-if-changed=assets/oxibrowser.entitlements");
}

fn setup_macos_bundle() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Copy Info.plist to output directory
    let plist_src = manifest_dir.join("assets/Info.plist");
    if plist_src.exists() {
        let plist_dst = out_dir.join("Info.plist");
        fs::copy(&plist_src, &plist_dst).expect("Failed to copy Info.plist");
        println!("cargo:warning=Info.plist copied to {}", plist_dst.display());
    } else {
        println!("cargo:warning=Info.plist not found at {}", plist_src.display());
    }

    // Copy entitlements
    let entitlements_src = manifest_dir.join("assets/oxibrowser.entitlements");
    if entitlements_src.exists() {
        let entitlements_dst = out_dir.join("oxibrowser.entitlements");
        fs::copy(&entitlements_src, &entitlements_dst).expect("Failed to copy entitlements");
        println!("cargo:warning=Entitlements copied to {}", entitlements_dst.display());
    }

    // Set macOS deployment target
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");
}

fn setup_ios_build() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Copy iOS Info.plist
    let plist_src = manifest_dir.join("assets/Info-iOS.plist");
    if plist_src.exists() {
        let plist_dst = out_dir.join("Info-iOS.plist");
        fs::copy(&plist_src, &plist_dst).expect("Failed to copy iOS Info.plist");
    }

    // iOS requires specific linker flags
    println!("cargo:rustc-link-lib=framework=UIKit");
    println!("cargo:rustc-link-lib=framework=WebKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
}
