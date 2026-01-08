# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build              # Build debug
cargo build --release    # Build release
cargo test               # Run all tests
cargo test test_name     # Run a single test
cargo run -- --url <youtube-url>     # Play a video by URL
cargo run -- --search "query"        # Play first YouTube search result
```

## External Dependencies

Requires installed on the system:
- **yt-dlp**: Downloads/streams video data
- **ffmpeg** (with dev libraries): Video/audio decoding
- **PulseAudio dev libs** (Linux): Audio output via cpal

Arch Linux: `pacman -S yt-dlp ffmpeg`
Ubuntu: `apt install yt-dlp ffmpeg libpulse-dev libavutil-dev`

## Architecture

Real-time terminal video player using Kitty graphics protocol. Four-thread pipeline with mpsc channels:

```
yt-dlp process (spawned)
        ↓
[Demultiplexer] -- parses MP4 boxes as they stream in
    ↓           ↓
[Audio]      [Video]
    ↓           ↓
[cpal]    [Kitty protocol → stdout]
```

### Module Overview

- **src/demux/**: MP4 parsing and ffmpeg decoding (~75% of codebase)
  - `demultiplexer.rs`: Spawns yt-dlp, parses ftyp/moov/mdat boxes, decodes H264→RGB and AAC→PCM
  - `codec_context.rs`: Unsafe FFI to construct ffmpeg decoders from avcC/codec parameters
  - `get_moov_box.rs`: Parses MP4 metadata structure (trak, mdia, stsd, avcC)
  - `get_sample_map.rs`: Builds ordered queue of audio/video sample locations

- **src/video/**: Terminal rendering
  - `encoder.rs`: Converts RGB frames to Kitty graphics protocol (base64 + escape sequences)
  - `adapter.rs`: Writes frames to stdout with timing sync

- **src/audio/**: Audio playback
  - `adapter.rs`: Feeds decoded samples to cpal output stream

- **src/helpers/**: CLI args, terminal screen guard, common types

### Key Patterns

- All inter-thread communication via `std::sync::mpsc` channels
- Message enums with `Done` variant for clean shutdown signaling
- `recv_timeout(16ms)` polling for graceful termination
- Video uses format 18 from yt-dlp: H264 640x360 + AAC-LC audio
- Alternate screen mode (`\x1B[?1049h`) for clean terminal experience

### Codec Context Initialization

`src/demux/codec_context.rs` contains unsafe FFI functions that construct ffmpeg decoders directly from codec parameters extracted from the MP4 moov box. The H.264 video decoder is initialized with avcC extradata, and the AAC audio decoder with sample rate/channel configuration. This approach avoids needing external sample files.
