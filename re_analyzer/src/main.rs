//! Re: Analyzer main entry point.

use anyhow::Ok;
use re_analyzer::{meta, resolve::Resolver};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let metadata = re_analyzer::run_metadata()?;
    let mut metadata: meta::Metadata = serde_json::from_str(&metadata)?;

    let mut resolver = Resolver::new(metadata.packages, metadata.resolve);
    let resolved_packages = resolver.resolve();
    metadata.packages = resolved_packages;

    Ok(())
}
