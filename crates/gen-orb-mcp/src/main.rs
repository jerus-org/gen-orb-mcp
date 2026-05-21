use anyhow::Result;
use clap::Parser;
use gen_orb_mcp::Cli;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // tracing_subscriber::init() calls LogTracer::init() automatically when
    // the tracing-log feature is active (unified via dependency tree).
    // Calling it manually beforehand causes a SetLoggerError panic.
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
