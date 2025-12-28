use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use rand::seq::IndexedRandom;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Paths to the images to be used for setting the wallpaper
    #[clap(short, long, required = true, value_name = "IMAGE_PATHS")]
    image_paths: Vec<PathBuf>,

    /// Interval in seconds to change the wallpaper
    #[clap(short('n'), long, default_value = "3600", value_name = "INTERVAL")]
    interval_in_secs: u64,

    /// Transition type
    #[clap(short('t'), long, default_value = "random", value_name="TRANSITION_TYPE")]
    transition_type: String,

    /// Transition duration in seconds
    #[clap(short('d'), long, default_value = "3", value_name="TRANSITION_DURATION_SECS")]
    transition_duration_secs: u32,
}

/// Recursively discover all image files from the given folder paths
fn discover_images(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut images = Vec::new();

    // Common image extensions to look for
    let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif"];

    for path in paths {
        if !path.exists() {
            eprintln!("Warning: Path does not exist: {}", path.display());
            continue;
        }

        if !path.is_dir() {
            eprintln!("Warning: Path is not a directory: {}", path.display());
            continue;
        }

        // Walk through the directory recursively
        for entry in WalkDir::new(path).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // Check if it's a file and has an image extension
                    if path.is_file()
                        && let Some(extension) = path.extension()
                        && let Some(ext_str) = extension.to_str()
                    {
                        let ext_lower = ext_str.to_lowercase();
                        if image_extensions.contains(&ext_lower.as_str()) {
                            images.push(path.to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error accessing file: {}", e);
                }
            }
        }
    }

    println!("Discovered {} images", images.len());
    Ok(images)
}

/// Query the current wallpaper using `swww query`
fn get_current_wallpaper() -> Result<Option<PathBuf>> {
    let output = Command::new("swww")
        .arg("query")
        .output()
        .context("Failed to execute swww query command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("swww query failed: {}", error_msg);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse output like: "DP-1: 2560x1080, scale: 2, currently displaying: image: /path/to/image.jpg"
    for line in stdout.lines() {
        if let Some(image_part) = line.split("currently displaying: image: ").nth(1) {
            return Ok(Some(PathBuf::from(image_part.trim())));
        }
    }

    Ok(None)
}
/// Perform one wallpaper change cycle: query current, pick a different random image, and set it
fn change_wallpaper_once(images: &[PathBuf], cli: &Cli) {
    // Get current wallpaper
    let current_wallpaper = match get_current_wallpaper() {
        Ok(current) => {
            if let Some(ref path) = current {
                println!("Current wallpaper: {}", path.display());
            }
            current
        }
        Err(e) => {
            eprintln!("Warning: Could not query current wallpaper: {}", e);
            None
        }
    };

    // Select a new random wallpaper
    if let Some(new_wallpaper) = select_random_image(images, current_wallpaper.as_ref()) {
        // Only change if it's different from current (extra safety check)
        if current_wallpaper.as_ref() != Some(&new_wallpaper) {
            if let Err(e) = set_wallpaper(&new_wallpaper, cli) {
                eprintln!("Error setting wallpaper: {}", e);
            }
        } else {
            println!("Selected image is the same as current, skipping change");
        }
    } else {
        eprintln!("Warning: Could not select a random image");
    }
}


/// Set wallpaper using `swww img`
fn set_wallpaper(image_path: &PathBuf, cli: &Cli) -> Result<()> {
    println!("Setting wallpaper to: {}", image_path.display());

    let output = Command::new("swww")
        .arg("img")
        .arg("--transition-type")
        .arg(&cli.transition_type)
        .arg("--transition-duration")
        .arg(format!("{}", cli.transition_duration_secs))
        .arg(image_path)
        .output()
        .context("Failed to execute swww img command")?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("swww img failed: {}", error_msg);
    }

    println!("Wallpaper changed successfully");
    Ok(())
}

/// Select a random image that's different from the current wallpaper
fn select_random_image(images: &[PathBuf], current: Option<&PathBuf>) -> Option<PathBuf> {
    let mut rng = rand::rng();

    // If there's only one image or no current wallpaper, just pick randomly
    if images.len() <= 1 || current.is_none() {
        return images.choose(&mut rng).cloned();
    }

    let current = current.unwrap();

    // Filter out the current wallpaper and pick from the rest
    let candidates: Vec<&PathBuf> = images.iter().filter(|&img| img != current).collect();

    // If all images are the same as current (shouldn't happen), just return current
    if candidates.is_empty() {
        return Some(current.clone());
    }

    candidates.choose(&mut rng).map(|&img| img.clone())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Discover all available images
    let images = discover_images(&cli.image_paths)?;

    if images.is_empty() {
        anyhow::bail!("No images found in the specified paths");
    }

    println!("Starting wallpaper switcher with {} images", images.len());
    println!("Changing wallpaper every {} seconds", cli.interval_in_secs);

    // Create SIGUSR1 signal listener
    let mut sigusr1_stream = signal(SignalKind::user_defined1())
        .context("Failed to register SIGUSR1 handler")?;

    // Do an initial change once at startup
    change_wallpaper_once(&images, &cli);

    // Main event loop: wait for either interval or SIGUSR1, then change wallpaper
    loop {
        println!(
            "Waiting {} seconds until next change... (send SIGUSR1 to change immediately)",
            cli.interval_in_secs
        );

        let sleep_fut = sleep(Duration::from_secs(cli.interval_in_secs));
        tokio::pin!(sleep_fut);

        tokio::select! {
            _ = &mut sleep_fut => {
                println!("Interval expired, changing wallpaper...");
            }
            _ = sigusr1_stream.recv() => {
                println!("Received SIGUSR1 signal, changing wallpaper immediately...");
            }
        }

        change_wallpaper_once(&images, &cli);
    }
}
