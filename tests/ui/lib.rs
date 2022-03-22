#[cfg(test)]
mod tests {
    #[test]
    fn ui() {
        let t = trybuild::TestCases::new();
        t.pass("pass/*.rs");
        t.compile_fail("fail/*.rs");
    }
}
