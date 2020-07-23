/*!
Implementation details for log macro expansion.

This crate is not intended to be consumed directly.
*/

#![feature(min_specialization)] // required to accept `T: Sized + 'static || str`
#![feature(extern_types)] // could be replaced by empty enums

mod capture;

#[doc(hidden)]
pub mod __private {
    pub use crate::capture::__PrivateLogCapture;
    pub use log::kv::Value;
}

#[cfg(test)]
mod tests {
    use crate::__private::*;
    use std::fmt;

    #[test]
    fn capture_default() {
        struct SomeType;

        impl fmt::Display for SomeType {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "some type")
            }
        }

        // Capture an arbitrary `Display`
        let _ = SomeType.__private_log_capture_with_default();

        // Capture a structured number
        assert_eq!(
            Some(42u64),
            42u64.__private_log_capture_with_default().to_u64()
        );

        // Capture a borrowed (non-static) string
        let v: &str = &String::from("a string");
        assert_eq!(
            Some("a string"),
            v.__private_log_capture_with_default().to_borrowed_str()
        );
    }
}
