use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use rand::seq::IndexedRandom;
use walkdir::WalkDir;

use crate::WallSwitcher;
use tracing::{error, info, warn};

#[derive(Args, Debug)]
pub struct LocalArgs {
    /// Paths to the images to be used for setting the wallpaper
    #[clap(short, long, required = true, value_name = "IMAGE_PATHS")]
    image_paths: Vec<PathBuf>,
}

pub(crate) struct LocalWallSwitcher {
    args: LocalArgs,
    common: crate::CommonArgs,
    images: Vec<PathBuf>,
}

impl LocalWallSwitcher {
    pub(crate) fn new(args: LocalArgs, common: crate::CommonArgs) -> Self {
        Self {
            args,
            common,
            images: Vec::new(),
        }
    }
}

#[async_trait]
impl WallSwitcher for LocalWallSwitcher {
    async fn init(&mut self) -> Result<()> {
        // Discover all available images
        self.images = discover_images(&self.args.image_paths)?;

        if self.images.is_empty() {
            anyhow::bail!("No images found in the specified paths");
        }

        info!(
            "Starting wallpaper switcher with {} images",
            self.images.len()
        );
        Ok(())
    }

    async fn switch(&mut self) {
        change_wallpaper_once(&self.images, &self.common);
    }
}

/// Recursively discover all image files from the given folder paths
fn discover_images(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut images = Vec::new();

    // Common image extensions to look for
    let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif"];

    for path in paths {
        if !path.exists() {
            warn!("Path does not exist: {}", path.display());
            continue;
        }

        if !path.is_dir() {
            warn!("Path is not a directory: {}", path.display());
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
                    error!("Error accessing file: {}", e);
                }
            }
        }
    }

    info!("Discovered {} images", images.len());
    Ok(images)
}

/// Perform one wallpaper change cycle: query current, pick a different random image, and set it
fn change_wallpaper_once(images: &[PathBuf], common: &crate::CommonArgs) {
    // Get current wallpaper
    let current_wallpaper = match crate::get_current_wallpaper() {
        Ok(current) => {
            if let Some(ref path) = current {
                info!("Current wallpaper: {}", path.display());
            }
            current
        }
        Err(e) => {
            warn!("Could not query current wallpaper: {}", e);
            None
        }
    };

    // Select a new random wallpaper
    if let Some(new_wallpaper) = select_random_image(images, current_wallpaper.as_ref()) {
        // Only change if it's different from current (extra safety check)
        if current_wallpaper.as_ref() != Some(&new_wallpaper) {
            if let Err(e) = crate::set_wallpaper(
                &new_wallpaper,
                &common.transition_type,
                common.transition_duration_secs,
                common.resize.as_deref(),
            ) {
                error!("Error setting wallpaper: {}", e);
            }
        } else {
            info!("Selected image is the same as current, skipping change");
        }
    } else {
        warn!("Could not select a random image");
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
