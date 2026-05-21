use anyhow::Result;
use clap::Parser;
use gen_orb_mcp::Cli;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // Bridge `log` crate records (used by pcu and git2_credentials) into the
    // tracing subscriber so they appear when RUST_LOG is set.
    tracing_log::LogTracer::init()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gen_orb_mcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    cli.run()
}
