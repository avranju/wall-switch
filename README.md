# Wall Switch

A simple Rust CLI tool that automatically switches your wallpaper at specified intervals using random selection from your image collections.

## Prerequisites

- [swww](https://github.com/Horus645/swww) - Wayland wallpaper daemon
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
./wall-switch --image-paths /path/to/your/wallpapers

# Multiple image directories
./wall-switch --image-paths /path/to/wallpapers1 --image-paths /path/to/wallpapers2

# Custom interval (30 seconds for testing)
./wall-switch --image-paths /path/to/wallpapers --interval-in-secs 30
```

### Options

- `--image-paths` (`-i`) - Path to folder containing images (can be specified multiple times)
- `--interval-in-secs` - Time interval between wallpaper changes in seconds (default: 3600)

## Features

- **Recursive Discovery**: Finds images in all subdirectories of specified paths
- **Smart Selection**: Never repeats the current wallpaper
- **Format Support**: jpg, jpeg, png, gif, bmp, webp, tiff, tif

## Example

```bash
# Start the wallpaper switcher
./wall-switch -i ~/Pictures/Wallpapers -i ~/Downloads/Backgrounds --interval-in-secs 1800

# Output:
# Discovered 42 images
# Starting wallpaper switcher with 42 images
# Changing wallpaper every 1800 seconds
# Current wallpaper: /home/user/Pictures/current.jpg
# Setting wallpaper to: /home/user/Pictures/Wallpapers/sunset.jpg
# Wallpaper changed successfully
# Waiting 1800 seconds until next change...
```

## License

This project is open source and available under the [MIT License](LICENSE).
