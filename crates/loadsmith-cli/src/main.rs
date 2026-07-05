use anyhow::Context;
use clap::Parser;
use loadsmith::{
    registry::local::LocalRegistry,
    thunderstore::{ThunderstoreRegistry, sqlite::SqliteIndex},
};
use tracing::{error, warn};
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_DIRECTIVE: &str = "info";
const VERBOSE_DIRECTIVE: &str = "trace,h2=info,hyper_util=info,reqwest=info";

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        error!("{err:?}");
        std::process::exit(1);
    }
}

async fn try_main() -> anyhow::Result<()> {
    let cli = loadsmith_cli::Cli::parse();

    let rust_log_set = std::env::var("RUST_LOG").is_ok();

    let filter = if rust_log_set {
        EnvFilter::from_default_env()
    } else if cli.verbose {
        EnvFilter::new(VERBOSE_DIRECTIVE)
    } else {
        EnvFilter::new(DEFAULT_DIRECTIVE)
    };

    let indicatif_layer = IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time()
                .with_writer(indicatif_layer.get_stderr_writer()),
        )
        .with(indicatif_layer)
        .try_init()
        .context("failed to initialise logging")?;

    if cli.verbose && rust_log_set {
        warn!("RUST_LOG is set, verbose flag will be ignored");
    }

    cli.run(create_context()?).await
}

fn create_context() -> anyhow::Result<loadsmith_cli::Context> {
    let home_dir = dirs_next::home_dir()
        .context("failed to determine home directory")?
        .join(".loadsmith");

    std::fs::create_dir_all(&home_dir).context("failed to create home directory")?;

    let http = reqwest::Client::new();
    let thunderstore = thunderstore::Client::builder()
        .with_client(http.clone())
        .build()
        .context("failed to initialise thunderstore client")?;
    let index = SqliteIndex::open(thunderstore.clone(), home_dir.join("index.db"))
        .context("failed to open index")?;

    let mut registry_set = loadsmith::registry::RegistrySet::new();
    registry_set.add("local", LocalRegistry::new());
    registry_set.add(
        "thunderstore",
        ThunderstoreRegistry::sqlite(thunderstore.clone(), index.clone()),
    );

    let working_dir = std::env::current_dir().context("failed to determine working directory")?;

    Ok(loadsmith_cli::Context::new(
        http,
        thunderstore,
        registry_set,
        index,
        working_dir,
        home_dir,
    ))
}
