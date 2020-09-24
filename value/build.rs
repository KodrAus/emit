extern crate version_check as rustc;

fn main() {
    if rustc::is_feature_flaggable().unwrap_or(false) {
        println!("cargo:rustc-cfg=value_bag_const_type_id");
    }
}
