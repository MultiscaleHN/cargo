use baler;
use baler::util::{CliResult, CliError, Config};
use baler::util::important_paths::{find_root_manifest_for_wd};

#[derive(Deserialize)]
pub struct LocateProjectFlags {
    flag_manifest_path: Option<String>,
}

pub const USAGE: &'static str = "
Print a JSON representation of a Baler.toml file's location

Usage:
    baler locate-project [options]

Options:
    --manifest-path PATH    Path to the manifest to locate
    -h, --help              Print this message
";

#[derive(Serialize)]
pub struct ProjectLocation {
    root: String
}

pub fn execute(flags: LocateProjectFlags,
               config: &Config) -> CliResult {
    let root = find_root_manifest_for_wd(flags.flag_manifest_path, config.cwd())?;

    let string = root.to_str()
                      .ok_or_else(|| "Your project path contains \
                                             characters not representable in \
                                             Unicode".into())
                      .map_err(|e| CliError::new(e, 1))?;

    let location = ProjectLocation { root: string.to_string() };
    baler::print_json(&location);
    Ok(())
}
