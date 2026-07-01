use clap::Parser;
use tracing::error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = loadsmith_cli::Cli::parse();

    let ctx = loadsmith_cli::Context::new(
        std::env::current_dir().expect("failed to determine working directory"),
    );

    if let Err(err) = cli.run(ctx).await {
        error!("{err}");
        std::process::exit(1);
    }
}
