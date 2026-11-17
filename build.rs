const SLINT_ENTRY: &str = "ui/flyout.slint";

fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent".into());
    if let Err(e) = slint_build::compile_with_config(SLINT_ENTRY, config) {
        panic!("{SLINT_ENTRY}: expected a successful Slint compile, actual error: {e}");
    }
}
