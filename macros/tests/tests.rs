/// we don't import anything here because we're testing that it works in a context with no previous
/// imports
#[test]
fn wraps_result() {
    #[macros::wrap_result("std::process::exit(0)")]
    fn wraps(owo: &'static str) -> Result<(), failure::Error> {
        failure::bail!(owo);
    }
    wraps("blep");
}
