# yt-term
Real time video streaming terminal player based on [yt-dlp](https://github.com/yt-dlp/yt-dlp), [ffmpeg](https://www.ffmpeg.org), and the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).

You can find a list of supported sites in the [yt-dlp documentation](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md).

## Disclaimer
This is a hobby project, meant to be a proof of concept and to try out the Kitty graphics protocol. While I am sure it can be improved in many ways, I am not going to maintain it. Feel free to fork / copy it if you want to use it or improve it.

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
| Option | Description | Default |
|--------|-------------|---------|
| `-u`, `--url` | The URL of the video to play | https://www.youtube.com/watch?v=dQw4w9WgXcQ |

