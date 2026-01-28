# Streaming YouTube in the terminal because I can
Real time video streaming terminal player based on [yt-dlp](https://github.com/yt-dlp/yt-dlp), [ffmpeg](https://www.ffmpeg.org), and the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).

**Disclaimer: This is a hobby project.** I wanted to learn more about Rust and video formats, and I did. It's not meant to be a legitimate alternative to an actual YouTube client or any other video streaming platform.

## Demo

https://github.com/user-attachments/assets/f102be1d-9e0f-42da-bdc8-012b09d3045f





## Dependencies
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) to extract data from video websites
- [ffmpeg](https://www.ffmpeg.org/) for video and audio decoding
- a terminal that supports the Kitty graphics protocol, e.g. [Kitty](https://sw.kovidgoyal.net/kitty/), [Ghostty](https://ghostty.org/)

## Usage

1. Clone the repository
2. Run or build with `cargo`

Run without arguments to start the TUI. Alternatively, you can pass the `-u` or `--url` option to play a specific video, or `-s` or `--search` to search YouTube and play the first result.

```bash
git clone git@github.com:ThbltLmr/yt-term.git  # or use HTTPS or the GitHub CLI
cd yt-term
cargo build --release # or cargo run
```

## FAQ

### 1. Is this an actual useful program?

No.

### 2. Should I use this?

Probably not.

### 3. Does it technically work?

Yes, kind of.

### 4. Why does it not work for this video / my terminal / my OS / etc?

This is a Rust terminal program built by a TypeScript web developer, what did you expect?

### 5. I read the source code and now my eyes are bleeding

First, that is not a question. Also, see question 4.
