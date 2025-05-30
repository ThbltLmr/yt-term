# yt-term
Real time streaming terminal player based on [yt-dlp](https://github.com/yt-dlp/yt-dlp).
You can find a list of supported sites in the [yt-dlp documentation](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md).

## Dependencies
This program uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) to extract data from video websites, and [ffmpeg](https://www.ffmpeg.org) to pre-encode frames to RGB.
Please note this also requires a terminal that supports the Kitty graphics protocol, e.g. [Kitty](https://sw.kovidgoyal.net/kitty/), [Ghostty](https://ghostty.org/).

## What it this repository?
This is an experiment to stream video in a terminal using the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/#). 
It is not meant to be a full-featured video player. I have not tested in any terminal other than Ghostty, and I have not tested with various websites and video formats. 

## How it works

### The Kitty graphics protocol
The Kitty graphics protocol allows you to send images to the terminal, which can then be displayed in a terminal window.
Images can be sent in three different formats: RGB, RGBA, and PNG. They can be sent to the terminal with a specific escape sequence:
```
<ESC>_G<control_data>;<base64_encoded_image_or_image_path><ESC>\
```
Where the `control_data` is a string of comma-separated key-value pairs that specify the image format, width, height, and other properties. The `base64_encoded_image_or_image_path` is the image data or a path to an image file encoded in base64.

I chose to use the RGB format as it is the most straightfoward to implement. By default, I also chose to use a 640x360 resolution, which is high enough for most videos and low enough to not cause performance issues.

Finally, the last properties needed in the `control_data` are the action we want to perform (in this case, display the image), and the transmission medium (which indicates where the image is coming from - in this case, the data is directly encoded in the escape code).

```rust
let encoded_control_data = self.encode_control_data(HashMap::from([
    ("f".into(), "24".into()), // RGB format
    ("s".into(), format!("{}", self.width).into()), // image width
    ("v".into(), format!("{}", self.height).into()), // image height
    ("t".into(), "d".into()), // transmission medium
    ("a".into(), "T".into()), // action to perform (T for display)
    ("X".into(), format!("{}", x_offset).into()), // x offset to center the image
    ("Y".into(), format!("{}", y_offset).into()), // y offset to center the image
]));
```

### Getting RGB frames for a video
To get a stream of RGB frames from a video, we use a combination of `yt-dlp` and `ffmpeg`.

We start `yt-dlp` to extract the video stream, with the right width and height, and set it up to output to `stdout`. We can then pipe it to `ffmpeg`'s `stdin`, which will decode the video and convert it to RGB frames.

We then store these RGB frames in a first buffer, from which we can read to encode them to the Kitty graphics protocol and store them in a second queue.

### Displaying the frames in the terminal
To display the video frames in the terminal, we need to read from the second queue, and send the encoded frames to the terminal at the right time.
Most YouTube videos in 360p have a frame rate of 25 FPS, meaning that we need to send a frame every 40 milliseconds.

```rust
if last_frame_time.elapsed() >= self.frame_interval {
    let encoded_frame = self.encoded_buffer.lock().unwrap().get_el();
    if let Some(frame) = encoded_frame {
        last_frame_time = std::time::Instant::now();
        self.display_frame(frame)?;
    }
}
```

Another important aspect is to clear the terminal before each frame. We can achieve this by sending a corresponding escape sequence before each frame.

```rust
fn display_frame(&self, frame: Frame) -> Res<()> {
    let mut stdout = io::stdout();
    let reset_cursor = b"\x1B[H";
    let mut buffer = vec![];
    buffer.extend_from_slice(reset_cursor);
    buffer.extend_from_slice(&frame.data);
    stdout.write_all(&buffer)?;
    stdout.flush()?;
    Ok(())
}
```

Finally, to avoid overlapping the frames with logs from the program (or any other processes), we can use an alternate screen. The `ScreenGuard` struct does this by switching to the alternate screen when it is created and switching back to the main screen when it is dropped.

```rust
pub struct ScreenGuard {}

impl ScreenGuard {
    pub fn new() -> Res<Self> {
        let mut stdout = std::io::stdout();
        let alternate_screen = b"\x1B[?1049h";

        stdout.write_all(alternate_screen)?;
        stdout.flush()?;
        Ok(ScreenGuard {})
    }
}

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();

        let mut buffer = vec![];
        let reset = b"\x1B[?1049l";
        let clear = b"\x1b[2J";
        let cursor = b"\x1b[H";
        buffer.extend_from_slice(reset);
        buffer.extend_from_slice(clear);
        buffer.extend_from_slice(cursor);

        stdout.write_all(&buffer).unwrap();
        stdout.flush().unwrap();
    }
}
```

### Frame rate
Since we aim for 25 FPS, we need to ensure that we send a frame every 40 milliseconds. This is done by checking the elapsed time since the last frame was sent and only sending a new frame if enough time has passed.

```rust
if last_frame_time.elapsed() >= self.frame_interval {
    let encoded_frame = self.encoded_buffer.lock().unwrap().get_el();
    if let Some(frame) = encoded_frame {
        last_frame_time = std::time::Instant::now();
        self.display_frame(frame)?;
    }
}
```
I ran into issues where the frame would take 50-60 ms to display, which would cause the frame rate to drop around 20 FPS. To mitigate this, we can check that the elapsed time since last frame is not too much greater than 40 ms, and if it is, we can skip the frame to avoid lagging behind.

This makes the video playback a bit choppy, but it ensures that we don't fall too far behind the video stream.


```rust
if last_frame_time.elapsed() >= self.frame_interval {
    let encoded_frame = self.encoded_buffer.lock().unwrap().get_el();
    if let Some(frame) = encoded_frame {
        if total_frames_counter > 0
            && last_frame_time.elapsed()
                > self.frame_interval + Duration::from_millis(2) // If there was over 42 ms since the last frame, we skip this frame
        {
            last_frame_time += self.frame_interval;
            total_frames_counter += 1;
            continue;
        }

        last_frame_time = std::time::Instant::now();
        total_frames_counter += 1;
        self.display_frame(frame)?;
    }
}
```

### Adding audio

### Synchronizing audio and video with 'ready for display' queues
