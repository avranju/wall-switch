mod local;
mod unsplash;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::sleep;
use tracing::{error, info};

/// Common options shared across all wallpaper providers
#[derive(clap::Args, Debug, Clone)]
pub(crate) struct CommonArgs {
    /// Interval in seconds to change the wallpaper
    #[clap(short('n'), long, default_value = "3600", value_name = "INTERVAL")]
    pub(crate) interval_in_secs: u64,

    /// Transition type
    #[clap(
        short('t'),
        long,
        default_value = "random",
        value_name = "TRANSITION_TYPE"
    )]
    pub(crate) transition_type: String,

    /// Transition duration in seconds
    #[clap(
        short('d'),
        long,
        default_value = "3",
        value_name = "TRANSITION_DURATION_SECS"
    )]
    pub(crate) transition_duration_secs: u32,

    /// Resize strategy to pass to `awww img --resize`
    #[clap(long, value_name = "RESIZE", value_parser = ["no", "crop", "fit", "stretch"])]
    pub(crate) resize: Option<String>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    common: CommonArgs,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Use local image folders as the wallpaper source
    Local(local::LocalArgs),
    /// Download random wallpapers from Unsplash
    Unsplash(unsplash::UnsplashArgs),
}

#[async_trait]
pub(crate) trait WallSwitcher {
    async fn init(&mut self) -> Result<()>;
    async fn switch(&mut self);
}

#[derive(Debug, Deserialize)]
struct AwwwDisplay {
    displaying: Option<AwwwDisplaying>,
}

#[derive(Debug, Deserialize)]
struct AwwwDisplaying {
    image: Option<PathBuf>,
}

/// Query the current wallpaper using `awww query --json`
pub(crate) fn get_current_wallpaper() -> Result<Option<PathBuf>> {
    let output = Command::new("awww")
        .arg("query")
        .arg("--json")
        .output()
        .context("Failed to execute awww query --json command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("awww query --json failed: {}", error_msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let data: HashMap<String, Vec<AwwwDisplay>> = serde_json::from_str(&stdout)
        .context("Failed to parse awww query --json output")?;

    Ok(data
        .values()
        .flat_map(|displays| displays.iter())
        .find_map(|display| display.displaying.as_ref()?.image.clone()))
}

/// Set wallpaper using `awww img`
pub(crate) fn set_wallpaper(
    image_path: &PathBuf,
    transition_type: &str,
    transition_duration_secs: u32,
    resize: Option<&str>,
) -> Result<()> {
    info!("Setting wallpaper to: {}", image_path.display());

    let mut command = Command::new("awww");
    command
        .arg("img")
        .arg("--transition-type")
        .arg(transition_type)
        .arg("--transition-duration")
        .arg(format!("{}", transition_duration_secs));

    if let Some(resize) = resize {
        command.arg("--resize").arg(resize);
    }

    let output = command
        .arg(image_path)
        .output()
        .context("Failed to execute awww img command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("awww img failed: {}", error_msg);
    }

    info!("Wallpaper changed successfully");
    Ok(())
}

async fn run_wall_switcher<T: WallSwitcher>(
    mut wall_switcher: T,
    common: &CommonArgs,
) -> Result<()> {
    wall_switcher.init().await?;

    info!("Changing wallpaper every {} seconds", common.interval_in_secs);

    // Create SIGUSR1 signal listener
    let mut sigusr1_stream =
        signal(SignalKind::user_defined1()).context("Failed to register SIGUSR1 handler")?;

    // Do an initial change once at startup
    wall_switcher.switch().await;

    // Main event loop: wait for either interval or SIGUSR1, then change wallpaper
    loop {
        info!("Waiting {} seconds until next change... (send SIGUSR1 to change immediately)", common.interval_in_secs);

        let sleep_fut = sleep(Duration::from_secs(common.interval_in_secs));
        tokio::pin!(sleep_fut);

        tokio::select! {
            _ = &mut sleep_fut => {
                info!("Interval expired, changing wallpaper...");
            }
            _ = sigusr1_stream.recv() => {
                info!("Received SIGUSR1 signal, changing wallpaper immediately...");
            }
        }

        wall_switcher.switch().await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt::init();

    // Pre-flight check: ensure `awww` is installed
    if Command::new("awww").arg("--version").output().is_err() {
        error!("The 'awww' binary was not found in your PATH. Please install it first.");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Local(args) => {
            let wall_switcher = local::LocalWallSwitcher::new(args, cli.common.clone());
            run_wall_switcher(wall_switcher, &cli.common).await
        }
        Commands::Unsplash(args) => {
            let wall_switcher = unsplash::UnsplashWallSwitcher::new(args, cli.common.clone());
            run_wall_switcher(wall_switcher, &cli.common).await
        }
    }
}
