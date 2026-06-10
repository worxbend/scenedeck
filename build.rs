//! Build script for GTK resources.
//!
//! Cargo runs `build.rs` before compiling the crate. Here we call
//! `glib-compile-resources` so GTK can load the app's SVG icon from an
//! embedded GResource instead of loose files on disk.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let target = out_dir.join("scenedeck.gresource");

    let status = Command::new("glib-compile-resources")
        .arg("resources/scenedeck.gresource.xml")
        .arg("--target")
        .arg(&target)
        .arg("--sourcedir")
        .arg("resources")
        .status()
        .expect("failed to run glib-compile-resources");

    if !status.success() {
        panic!("glib-compile-resources failed with status {status}");
    }

    println!("cargo:rerun-if-changed=resources/scenedeck.gresource.xml");
    println!("cargo:rerun-if-changed=resources/icons/scalable/apps/io.scenedeck.app.svg");
}
