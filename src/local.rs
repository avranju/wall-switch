use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use rand::seq::IndexedRandom;
use walkdir::WalkDir;

use crate::WallSwitcher;

#[derive(Args, Debug)]
pub struct LocalArgs {
    /// Paths to the images to be used for setting the wallpaper
    #[clap(short, long, required = true, value_name = "IMAGE_PATHS")]
    image_paths: Vec<PathBuf>,

    /// Interval in seconds to change the wallpaper
    #[clap(short('n'), long, default_value = "3600", value_name = "INTERVAL")]
    interval_in_secs: u64,

    /// Transition type
    #[clap(
        short('t'),
        long,
        default_value = "random",
        value_name = "TRANSITION_TYPE"
    )]
    transition_type: String,

    /// Transition duration in seconds
    #[clap(
        short('d'),
        long,
        default_value = "3",
        value_name = "TRANSITION_DURATION_SECS"
    )]
    transition_duration_secs: u32,

    /// Resize strategy to pass to `awww img --resize`
    #[clap(long, value_name = "RESIZE", value_parser = ["no", "crop", "fit", "stretch"])]
    resize: Option<String>,
}

impl LocalArgs {
    pub(crate) fn interval_in_secs(&self) -> u64 {
        self.interval_in_secs
    }
}

pub(crate) struct LocalWallSwitcher {
    args: LocalArgs,
    images: Vec<PathBuf>,
}

impl LocalWallSwitcher {
    pub(crate) fn new(args: LocalArgs) -> Self {
        Self {
            args,
            images: Vec::new(),
        }
    }
}

impl WallSwitcher for LocalWallSwitcher {
    fn init(&mut self) -> Result<()> {
        // Discover all available images
        self.images = discover_images(&self.args.image_paths)?;

        if self.images.is_empty() {
            anyhow::bail!("No images found in the specified paths");
        }

        println!(
            "Starting wallpaper switcher with {} images",
            self.images.len()
        );
        Ok(())
    }

    fn switch(&mut self) {
        change_wallpaper_once(&self.images, &self.args);
    }
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

/// Perform one wallpaper change cycle: query current, pick a different random image, and set it
fn change_wallpaper_once(images: &[PathBuf], args: &LocalArgs) {
    // Get current wallpaper
    let current_wallpaper = match crate::get_current_wallpaper() {
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
            if let Err(e) = crate::set_wallpaper(
                &new_wallpaper,
                &args.transition_type,
                args.transition_duration_secs,
                args.resize.as_deref(),
            ) {
                eprintln!("Error setting wallpaper: {}", e);
            }
        } else {
            println!("Selected image is the same as current, skipping change");
        }
    } else {
        eprintln!("Warning: Could not select a random image");
    }
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
