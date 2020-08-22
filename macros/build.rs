fn main() {
    println!("cargo:rerun-if-env-changed=ANTLOG_FILTER");
    println!("cargo:rerun-if-changed=build.rs");
}
