pub use emit_ct::*;

#[doc(hidden)]
pub use emit_rt::*;

#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.pass("tests/ui/pass/*.rs");
        t.compile_fail("tests/ui/fail/*.rs");
    }
}
