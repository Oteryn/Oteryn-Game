use oteryn_world_vfx_real_content::content::{Viewport, qualify_fixture_decode};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("real-content-visual-slice-error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(std::env::args().skip(1))?;
    let viewport = if args.full_fixture {
        None
    } else {
        Some(Viewport::THAIS_DENSE)
    };
    let evidence = qualify_fixture_decode(
        &args.asset_zip,
        &args.assets_dir,
        &args.fixture_dir,
        viewport,
        args.cache_sheets,
    )?;
    let encoded = serde_json::to_string_pretty(&evidence)
        .map_err(|error| format!("serialize public-safe decode evidence: {error}"))?;
    println!("{encoded}");
    Ok(())
}

struct Args {
    asset_zip: PathBuf,
    assets_dir: PathBuf,
    fixture_dir: PathBuf,
    cache_sheets: usize,
    full_fixture: bool,
}

impl Args {
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut asset_zip = None;
        let mut assets_dir = None;
        let mut fixture_dir = None;
        let mut cache_sheets = 32_usize;
        let mut full_fixture = false;
        let mut iter = arguments.peekable();
        while let Some(argument) = iter.next() {
            match argument.as_str() {
                "--asset-zip" => asset_zip = Some(required_path(&mut iter, "--asset-zip")?),
                "--assets-dir" => assets_dir = Some(required_path(&mut iter, "--assets-dir")?),
                "--fixture-dir" => fixture_dir = Some(required_path(&mut iter, "--fixture-dir")?),
                "--cache-sheets" => {
                    let value = iter
                        .next()
                        .ok_or_else(|| "--cache-sheets requires an integer".to_owned())?;
                    cache_sheets = value
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --cache-sheets value {value:?}: {error}"))?;
                    if cache_sheets == 0 {
                        return Err("--cache-sheets must be at least one".to_owned());
                    }
                }
                "--full-fixture" => full_fixture = true,
                "-h" | "--help" => return Err(usage()),
                other => return Err(format!("unknown argument {other:?}\n{}", usage())),
            }
        }

        Ok(Self {
            asset_zip: asset_zip.ok_or_else(|| format!("missing --asset-zip\n{}", usage()))?,
            assets_dir: assets_dir.ok_or_else(|| format!("missing --assets-dir\n{}", usage()))?,
            fixture_dir: fixture_dir.ok_or_else(|| format!("missing --fixture-dir\n{}", usage()))?,
            cache_sheets,
            full_fixture,
        })
    }
}

fn required_path(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<PathBuf, String> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}

fn usage() -> String {
    concat!(
        "usage: oteryn-world-vfx-real-content ",
        "--asset-zip <exact-15.32.zip> ",
        "--assets-dir <extracted-assets-dir> ",
        "--fixture-dir <verified-thais-z7-export> ",
        "[--cache-sheets <N>] [--full-fixture]\n",
        "default selection: dense Thais viewport x=32400..32439 y=32239..32268 floor=-7"
    )
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_defaults_to_bounded_dense_viewport() -> Result<(), String> {
        let args = Args::parse(
            [
                "--asset-zip",
                "15.32.zip",
                "--assets-dir",
                "assets",
                "--fixture-dir",
                "fixture",
            ]
            .into_iter()
            .map(str::to_owned),
        )?;
        assert_eq!(args.cache_sheets, 32);
        assert!(!args.full_fixture);
        Ok(())
    }

    #[test]
    fn parser_rejects_zero_cache() {
        let result = Args::parse(
            [
                "--asset-zip",
                "15.32.zip",
                "--assets-dir",
                "assets",
                "--fixture-dir",
                "fixture",
                "--cache-sheets",
                "0",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        assert!(result.is_err());
    }
}
