# Wall Switch

A simple Rust CLI tool that automatically switches your wallpaper at specified intervals using random selection from local image folders or from Unsplash.

## Prerequisites

- [awww](https://codeberg.org/LGFae/awww) - Wayland wallpaper daemon
- Rust toolchain (for building from source)

## Installation

```bash
git clone <repository-url>
cd wall-switch
cargo build --release
```

The binary will be available at `target/release/wall-switch`.

## Usage

```bash
# Basic usage - change wallpaper every hour from a single folder
./wall-switch local -i /path/to/your/wallpapers

# Multiple image directories
./wall-switch local -i /path/to/wallpapers1 -i /path/to/wallpapers2

# Custom interval (30 seconds for testing)
./wall-switch local -i /path/to/wallpapers --interval-in-secs 30

# Using Unsplash for random wallpapers
./wall-switch unsplash --api-key YOUR_UNSPLASH_API_KEY
```

### Global Options

These options are shared across all wallpaper providers and are specified at the `wall-switch` level:

- `--interval-in-secs` (`-n`) - Time interval between wallpaper changes in seconds (default: 3600)
- `--transition-type` (`-t`) - Transition type forwarded to `awww img --transition-type` (default: `random`)
- `--transition-duration-secs` (`-d`) - Transition duration in seconds (default: 3)
- `--resize` - Resize strategy forwarded to `awww img --resize` (`no`, `crop`, `fit`, or `stretch`)

### Local Provider

```bash
./wall-switch local -i ~/Pictures/Wallpapers --interval-in-secs 1800
```

#### Local Options

- `--image-paths` (`-i`) - Path to folder containing images (can be specified multiple times, required)

### Unsplash Provider

```bash
./wall-switch unsplash --api-key YOUR_UNSPLASH_API_KEY
```

#### Unsplash Options

- `--api-key` (`-a`) - Unsplash API access key (also available via `UNSPLASH_API_KEY` environment variable)
- `--width` (`-w`) - Desired wallpaper width in pixels (default: 1920)
- `--height` (`-H`) - Desired wallpaper height in pixels (default: 1080)

## Features

- **Local Provider**: Recursive discovery of images in all subdirectories of specified paths
- **Smart Selection**: Never repeats the current wallpaper (local provider)
- **Unsplash Integration**: Fetch random high-quality wallpapers from Unsplash with dynamic resizing
- **Format Support**: jpg, jpeg, png, gif, bmp, webp, tiff, tif

## Example

```bash
# Start the wallpaper switcher with local images
./wall-switch local -i ~/Pictures/Wallpapers -i ~/Downloads/Backgrounds --interval-in-secs 1800

# Output:
# Discovered 42 images
# Starting wallpaper switcher with 42 images
# Changing wallpaper every 1800 seconds
# Current wallpaper: /home/user/Pictures/current.jpg
# Setting wallpaper to: /home/user/Pictures/Wallpapers/sunset.jpg
# Wallpaper changed successfully
# Waiting 1800 seconds until next change...

# Start the wallpaper switcher with Unsplash
./wall-switch unsplash --api-key abc123 --width 2560 --height 1440 --interval-in-secs 600

# Output:
# Starting Unsplash wallpaper switcher (2560x1440, interval=600s)
# Changing wallpaper every 600 seconds
# Downloading image from: https://images.unsplash.com/...?w=2560&h=1440&fit=crop
# Saved image to: /tmp/unsplash_wall_1718000000.jpg
# Setting wallpaper to: /tmp/unsplash_wall_1718000000.jpg
# Wallpaper changed successfully
# Waiting 600 seconds until next change...
```

## Systemd User Service

You can run wall-switch as a user-mode systemd service so it starts automatically and restarts on failure.

1. Copy the sample unit file and edit it to match your setup:

   ```bash
   cp wall-switch.service ~/.config/systemd/user/wall-switch.service
   ```

2. Edit the file:
   - Replace `/usr/local/bin/wall-switch` with the path to your built binary
   - Replace `UNSPLASH_API_KEY=<your-api-key>` with your actual Unsplash API key
   - Adjust the command arguments (`unsplash`, `local`, etc.) to match your chosen provider

3. Enable and start the service:

   ```bash
   systemctl --user daemon-reload
   systemctl --user enable --now wall-switch.service
   ```

4. Check status:

   ```bash
   systemctl --user status wall-switch.service
   ```

## License

This project is open source and available under the [MIT License](LICENSE).
