use baler::ops;
use baler::util::{CliResult, Config};

#[derive(Deserialize)]
pub struct Options {
    flag_bin: Vec<String>,
    flag_root: Option<String>,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,

    arg_spec: String,
}

pub const USAGE: &'static str = "
Remove a Rust binary

Usage:
    baler uninstall [options] <spec>
    baler uninstall (-h | --help)

Options:
    -h, --help                Print this message
    --root DIR                Directory to uninstall packages from
    --bin NAME                Only uninstall the binary NAME
    -v, --verbose ...         Use verbose output (-vv very verbose/build.rs output)
    -q, --quiet               Less output printed to stdout
    --color WHEN              Coloring: auto, always, never
    --frozen                  Require Cargo.lock and cache are up to date
    --locked                  Require Cargo.lock is up to date

The argument SPEC is a package id specification (see `baler help pkgid`) to
specify which crate should be uninstalled. By default all binaries are
uninstalled for a crate but the `--bin` and `--example` flags can be used to
only uninstall particular binaries.
";

pub fn execute(options: Options, config: &Config) -> CliResult {
    config.configure(options.flag_verbose,
                     options.flag_quiet,
                     &options.flag_color,
                     options.flag_frozen,
                     options.flag_locked)?;

    let root = options.flag_root.as_ref().map(|s| &s[..]);
    ops::uninstall(root, &options.arg_spec, &options.flag_bin, config)?;
    Ok(())
}

