//! Build script for browser-ui
//! 
//! Ensures the WebView subprocess binary is available.

use std::env;
use std::path::PathBuf;

fn main() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    
    // Path to subprocess binary
    let subprocess_name = if cfg!(target_os = "windows") {
        "rusty-browser-webview.exe"
    } else {
        "rusty-browser-webview"
    };

    // Look in target directory
    let target_dir = PathBuf::from(&manifest_dir)
        .join("..")
        .join("..")
        .join("target")
        .join(&profile);
    
    let subprocess_path = target_dir.join(&subprocess_name);

    if subprocess_path.exists() {
        println!("cargo:rustc-env=WEBVIEW_SUBPROCESS_PATH={}", subprocess_path.display());
        println!("cargo:warning=WebView subprocess found: {}", subprocess_path.display());
    } else {
        println!("cargo:warning=WebView subprocess not found at: {}", subprocess_path.display());
        println!("cargo:warning=Run: cargo build -p rusty-browser-webview");
        println!("cargo:warning=Browser will run in UI-only mode until subprocess is available");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
