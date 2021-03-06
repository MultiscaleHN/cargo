extern crate hamcrest;
extern crate balertest;

use balertest::support::{project, execs};
use hamcrest::assert_that;

#[test]
fn read_env_vars_for_config() {
    let p = project("foo")
        .file("Baler.toml", r#"
            [package]
            name = "foo"
            authors = []
            version = "0.0.0"
            build = "build.rs"
        "#)
        .file("src/lib.rs", "")
        .file("build.rs", r#"
            use std::env;
            fn main() {
                assert_eq!(env::var("NUM_JOBS").unwrap(), "100");
            }
        "#);

    assert_that(p.baler_process("build").env("CARGO_BUILD_JOBS", "100"),
                execs().with_status(0));
}
