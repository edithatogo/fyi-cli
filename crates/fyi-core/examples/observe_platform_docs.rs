//! Maintenance-only source observation; no source is contacted without `--live`.

use fyi_core::sync::documentation::{observe_documentation, SOURCES};
use std::path::PathBuf;

fn usage() {
    println!("Usage: observe_platform_docs --live --source SOURCE_ID --output FRESH_DIRECTORY");
    println!("One anonymous documentation GET; private local evidence only. Sources:");
    for source in SOURCES {
        println!("  {}", source.source_id);
    }
}

fn arguments(args: impl IntoIterator<Item = String>) -> Option<(String, PathBuf)> {
    let mut args = args.into_iter();
    let (mut live, mut source, mut output) = (false, None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--live" if !live => live = true,
            "--source" if source.is_none() => source = Some(args.next()?),
            "--output" if output.is_none() => output = Some(PathBuf::from(args.next()?)),
            _ => return None,
        }
    }
    if live {
        Some((source?, output?))
    } else {
        None
    }
}

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["--help"] || args == ["-h"] {
        usage();
        return;
    }
    let Some((source, output)) = arguments(args) else {
        usage();
        std::process::exit(2);
    };
    match observe_documentation(&source, &output, true).await {
        Ok(observation) => println!("{observation}"),
        Err(_) => {
            // Never echo a response, server header, arbitrary input, credential
            // or transport error chain. A directory without observation.json
            // is incomplete evidence and cannot advance a reviewed baseline.
            eprintln!("Documentation observation failed; check authorization, source, fresh output directory, and network policy.");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_source_and_output_are_required_and_duplicate_flags_fail_closed() {
        for args in [
            vec![],
            vec!["--source", "muckrock_client", "--output", "capture"],
            vec!["--live"],
            vec!["--live", "--source", "muckrock_client"],
            vec![
                "--live",
                "--source",
                "muckrock_client",
                "--output",
                "capture",
                "--live",
            ],
            vec![
                "--live",
                "--source",
                "muckrock_client",
                "--output",
                "capture",
                "--url",
                "https://example.invalid",
            ],
        ] {
            assert!(arguments(args.into_iter().map(str::to_owned)).is_none());
        }
        let parsed = arguments(
            [
                "--live",
                "--source",
                "muckrock_client",
                "--output",
                "capture",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(parsed, ("muckrock_client".into(), PathBuf::from("capture")));
    }
}
