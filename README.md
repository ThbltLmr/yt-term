# yt-term
Real time video streaming terminal player based on [yt-dlp](https://github.com/yt-dlp/yt-dlp), [ffmpeg](https://www.ffmpeg.org), and the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).

## Disclaimer
This is a hobby project, meant to try out the Kitty graphics protocol. This is not meant to be an actual YouTube client. In fact, it will probably not even for work some videos that have non-supported formats.

## Dependencies
This program requires:
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) to extract data from video websites
- a terminal that supports the Kitty graphics protocol, e.g. [Kitty](https://sw.kovidgoyal.net/kitty/), [Ghostty](https://ghostty.org/)

## Usage

### Cloning the repository and building with Cargo
```bash
git clone git@github.com:ThbltLmr/yt-term.git  # or use HTTPS or the GitHub CLI
cd yt-term
cargo build --release
```

### Downloading the pre-built binary
You can download the pre-built binary from the [releases page](https://github.com/ThbltLmr/yt-term/releases).

### Running the program
You can run the program with the following command:
```bash 

./target/release/yt-term [options] # if using Cargo
./yt-term [options] # if using the pre-built binary
```

### Options
| Option | Description |
|--------|-------------|
| `-u`, `--url` | The URL of the video to play |
| `-s`, `--search` | A search query; the tool will play the first result for that query on YouTube |

