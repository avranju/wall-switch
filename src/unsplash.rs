use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Args;
use serde::Deserialize;

use crate::WallSwitcher;

const UNSPLASH_API_BASE: &str = "https://api.unsplash.com";

// ── CLI arguments ────────────────────────────────────────────────────────────

#[derive(Args, Debug)]
pub struct UnsplashArgs {
    /// Unsplash API access key
    #[clap(short, long, env = "UNSPLASH_API_KEY", value_name = "ACCESS_KEY")]
    api_key: String,

    /// Desired wallpaper width in pixels
    #[clap(short('w'), long, default_value = "1920", value_name = "WIDTH")]
    width: u32,

    /// Desired wallpaper height in pixels
    #[clap(short('H'), long, default_value = "1080", value_name = "HEIGHT")]
    height: u32,
}

// ── Unsplash API response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PhotoResponse {
    urls: PhotoUrls,
}

#[derive(Debug, Deserialize)]
struct PhotoUrls {
    raw: String,
}

// ── WallSwitcher implementation ──────────────────────────────────────────────

pub(crate) struct UnsplashWallSwitcher {
    args: UnsplashArgs,
    common: crate::CommonArgs,
    client: reqwest::Client,
}

impl UnsplashWallSwitcher {
    pub(crate) fn new(args: UnsplashArgs, common: crate::CommonArgs) -> Self {
        Self {
            args,
            common,
            client: reqwest::Client::new(),
        }
    }

    /// Build the Imgix URL with dynamic resizing query parameters.
    fn resize_url(&self, raw_url: &str) -> String {
        format!(
            "{}?w={}&h={}&fit=crop",
            raw_url, self.args.width, self.args.height
        )
    }

    /// Download an image from the given URL and save it to a temp file.
    /// Returns the path to the downloaded file.
    async fn download_image(&self, url: &str) -> Result<PathBuf> {
        println!("Downloading image from: {}", url);

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to download image from Unsplash")?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "Image download failed with status {}: {}",
                resp.status(),
                resp.text().await?
            );
        }

        let bytes = resp.bytes().await.context("Failed to read image bytes")?;

        // Generate a unique filename using timestamp and random suffix
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("unsplash_wall_{}.jpg", timestamp);
        let path = std::env::temp_dir().join(filename);

        std::fs::write(&path, &bytes).context("Failed to save downloaded image to temp file")?;

        println!("Saved image to: {}", path.display());
        Ok(path)
    }

    /// Fetch a random photo from Unsplash using the /photos/random endpoint.
    /// Returns the raw URL of the photo.
    async fn fetch_random_photo(&self) -> Result<String> {
        let url = format!("{}/photos/random", UNSPLASH_API_BASE);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Client-ID {}", self.args.api_key))
            .header("Accept-Version", "v1")
            .send()
            .await
            .context("Failed to send request to Unsplash API")?;

        // Check for rate limiting (HTTP 429)
        if resp.status() == 429 {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60);

            let message = format!(
                "Rate limited by Unsplash API. Retry-After: {} seconds. Skipping this cycle.",
                retry_after
            );
            eprintln!("{}", message);
            anyhow::bail!("rate_limited: {}", message);
        }

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            let error: Result<ErrorResponse, _> = serde_json::from_str(&error_text);
            let msg = match error {
                Ok(err) => err.message.unwrap_or_else(|| "Unknown error".to_string()),
                Err(_) => error_text,
            };
            anyhow::bail!("Unsplash API error (HTTP {}): {}", status, msg);
        }

        let photo: PhotoResponse = resp
            .json()
            .await
            .context("Failed to parse Unsplash API response")?;

        if photo.urls.raw.is_empty() {
            anyhow::bail!("Received empty raw URL from Unsplash API");
        }

        Ok(photo.urls.raw)
    }
}

#[async_trait]
impl WallSwitcher for UnsplashWallSwitcher {
    async fn init(&mut self) -> Result<()> {
        println!(
            "Starting Unsplash wallpaper switcher ({}x{}, interval={}s)",
            self.args.width, self.args.height, self.common.interval_in_secs
        );
        Ok(())
    }

    async fn switch(&mut self) {
        if let Err(e) = self.switch_once().await {
            if e.to_string().starts_with("rate_limited:") {
                // Rate limiting is expected — just print and continue
                return;
            }
            eprintln!("Error switching wallpaper: {}", e);
        }
    }
}

impl UnsplashWallSwitcher {
    async fn switch_once(&mut self) -> Result<()> {
        // Fetch a random photo
        let raw_url = self.fetch_random_photo().await?;

        // Build the resized URL using Unsplash's dynamic resizing (Imgix)
        let resized_url = self.resize_url(&raw_url);

        // Download the image to temp
        let image_path = self.download_image(&resized_url).await?;

        // Set the wallpaper
        crate::set_wallpaper(
            &image_path,
            &self.common.transition_type,
            self.common.transition_duration_secs,
            self.common.resize.as_deref(),
        )?;

        Ok(())
    }
}
