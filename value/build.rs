use std::env;
use std::process::Command;
use std::str::{self, FromStr};

#[path = "src/internal/cast/primitive.rs"]
mod primitive;

fn main() {
    let minor = match rustc_minor_version() {
        Some(minor) => minor,
        None => return,
    };

    // If the Rust version is at least 1.47.0 then we can use type ids at compile time
    if minor >= 47 {
        println!("cargo:rustc-cfg=const_type_id");
    }

    // Generate sorted type id lookup
    primitive::generate();

    println!("cargo:rustc-cfg=srcbuild");
    println!("cargo:rerun-if-changed=build.rs");
}

// From the `serde` build script
fn rustc_minor_version() -> Option<u32> {
    let rustc = match env::var_os("RUSTC") {
        Some(rustc) => rustc,
        None => return None,
    };

    let output = match Command::new(rustc).arg("--version").output() {
        Ok(output) => output,
        Err(_) => return None,
    };

    let version = match str::from_utf8(&output.stdout) {
        Ok(version) => version,
        Err(_) => return None,
    };

    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }

    let next = match pieces.next() {
        Some(next) => next,
        None => return None,
    };

    u32::from_str(next).ok()
}
