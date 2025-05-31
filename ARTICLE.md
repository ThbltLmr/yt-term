# Streaming YouTube videos in the terminal with the Kitty graphics protocol

For decades, I have been looking for a blazingly slow, low-quality, feature-poor video player that would let me watch Minecraft Let's play videos and NordVPN ads directly in my terminal.

So when I saw the following line in the Ghostty documentation, I envisioned a way to make my dreams come true:

> Kitty graphics protocol: Ghostty supports the Kitty graphics protocol, which allows terminal applications to render images directly in the terminal

The [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) lets you display images in the terminal by sending image data (in RGB, RGBA, or PNG format) in an escape sequence, alongside properties that tell the terminal how to display the image. This means if we get a YouTube video in the form of a stream of RGB frames, we can use the protocol to display them directly in our terminal.

My initial high-level plan was the following:
- Get video data from YouTube
- Convert it to RGB frames
- Encode these frames according to the Kitty Graphics protocol
- Send them to the terminal at the right interval to match the original video's framerate
- (Optional) get audio data from YouTube and play it at the same time

## Getting a stream of RGB frames with yt-dlp and ffmpeg

(Un)surprinsingly, getting the actual video data for a YouTube video is not trivial. Could it be that YouTube does not want you to watch their content in a client that they don't make money from? Fortunately, programmers much more talented than myself have already tackled this issue in the `yt-dlp` project, which lets you download YouTube videos from their url in a variety of formats (with different resolutions, framerates, encoding and so on).

Since YouTube does not have video streams directly in RGB format, we need to convert the data we get from `yt-dlp`. The easiest solution I found to do so was to pipe the `stdout` from `yt-dlp` directly into the `stdin` of `ffmpeg`. The output of `ffmpeg` then becomes a stream of RGB frames, which is exactly what we need.

```rust
// src/video/streamer.rs
pub fn stream(&self) -> Res<()> {
    let frame_size = self.width * self.height * 3;
    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o", // output to stdout
            "-",
            "--no-part",
            "-f",
            format!("bestvideo[height={}][width={}]", self.height, self.width).as_str(),
            &self.url,
        ])
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start yt-dlp process");

    let yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

    let mut ffmpeg_process = Command::new("ffmpeg")
        .args(["-i", "pipe:0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .stdin(Stdio::from(yt_dlp_stdout))
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start ffmpeg process");
        let mut ffmpeg_stdout = ffmpeg_process
        .stdout
        .take()
        .expect("Failed to get ffmpeg stdout");
    let mut accumulated_data = Vec::new();

    // 32KB chunks, chunks that yt-dlp outputs
    let yt_dlp_chunk_size = 32768;
    let mut read_buffer = vec![0u8; yt_dlp_chunk_size];
        loop {
        match ffmpeg_stdout.read(&mut read_buffer) {
            Ok(0) => {
                self.streaming_done_tx.send(()).unwrap();
                break;
            }
            Ok(bytes_read) => {
                accumulated_data.extend_from_slice(&read_buffer[0..bytes_read]);
                    while accumulated_data.len() >= frame_size {
                    let frame_data = accumulated_data.drain(0..frame_size).collect::<Vec<u8>>();
                    let frame = Frame::new(frame_data);
                        self.rgb_buffer.lock().unwrap().push_el(frame);
                }
            }
            Err(e) => {
                eprintln!("Error reading from ffmpeg: {}", e);
                break;
            }
        }
    }
    if !accumulated_data.is_empty() {
        println!("Leftover incomplete data: {} bytes", accumulated_data.len());
    }
    let _ = ffmpeg_process.wait();
    let _ = yt_dlp_process.wait();
    Ok(())
}
```
I moved this `yt-dlp` -> `ffmpeg` pipeline into its own thread, and save the resulting frames into a buffer so we can encode them to match the Kitty graphics protocol.

## Encoding RBG frames for the Kitty graphics protocol

The Kitty graphics protocol expects the following escape sequence to display an image.

```
<ESC>_G<control_data>;<payload><ESC>\
```

Let's break it down:

### Control data

Before sending the terminal some image data, we need to provide it with more information about the image and its properties. That's the role of the control data, which is a series of key-value pairs. You can find a full reference [here](https://sw.kovidgoyal.net/kitty/graphics-protocol/#control-data-reference). For our use case, all we need are the following:

`f=24,s={image width},v={image height},t=d,a=T`

This will tell the terminal to expect RGB data (`f=24`) directly encoded in the payload (`t=d`) and to display it instantly (`a=T`), alongside the image dimensions.

My function to handle control data looks like this: 

```rust
fn encode_control_data(&self, control_data: HashMap<String, String>) -> Vec<u8> {
    let mut encoded_data = Vec::new();
    for (key, value) in control_data {
        encoded_data.push(format!("{}={}", key, value));
    }

    encoded_data.join(",").as_bytes().to_vec()
}
```

### Payload

The payload itself is simply the RBG data, encoded in base 64:

---
- Read about the protocol in Ghostty docs- Missing link: the one part of my usual workflow missing from terminal applications is YouTube
- Getting RGB frames from YouTube with yt-dlp and ffmpeg
- Displaying RBG frames with the protocol (control data and base 64 encoding)- Storing RBG and encoding frames in queues
- Handling framerate (40 ms intervals + skipping frames to maintain fps)
- Audio samples intro- Getting audio samples from yt-dlp and ffmpeg
- Outputting the audio to PulseAudio
- Syncing audio and video with ready queues

- Shutdown with channels


### The Kitty graphics protocol
The Kitty graphics protocol allows you to send images to the terminal, which can then be displayed in a terminal window.
Images can be sent in three different formats: RGB, RGBA, and PNG. They can be sent to the terminal with a specific escape sequence:
```
<ESC>_G<control_data>;<base64_encoded_image_or_image_path><ESC>\
```
Where the `control_data` is a string of comma-separated key-value pairs that specify the image format, width, height, and other properties. The `base64_encoded_image_or_image_path` is the image data or a path to an image file encoded in base64.

I chose to use the RGB format as it is the most straightfoward to implement. By default, I also chose to use a 640x360 resolution, which is high enough for most videos to be watchable and low enough to not cause performance issues.

Other properties needed in the `control_data` are the action we want to perform (in this case, display the image), and the transmission medium (which indicates where the image is coming from - in this case, the data is directly encoded in the escape code).

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
To add audio capabilities, similar to video, we use `yt-dlp` and `ffmpeg`. `yt-dlp` extracts the best audio stream from the given URL, and `ffmpeg` transcodes it into a raw audio format: 16-bit signed little-endian (s16le), 2 channels (stereo), and a 48kHz sample rate. This raw audio data is then streamed into an `audio_buffer`, which is a `RingBuffer<Sample>`.

For playback, we leverage `simple-pulse`, a Rust wrapper for PulseAudio. An `AudioAdapter` is responsible for reading `Sample`s from the `audio_buffer` and playing them through PulseAudio at the appropriate `sample_interval` (which is 1000ms / 48000 samples/sec = 20.83 microseconds per sample, but effectively we process them in chunks).

### Synchronizing audio and video with 'ready for display' queues
To ensure that audio and video playback remain synchronized, we introduce a queuing mechanism. Raw video frames are initially stored in `raw_video_buffer` and then encoded video frames (Kitty graphics protocol) are stored in `encoded_video_buffer`. Similarly, raw audio samples are stored in `audio_buffer`.

However, the `AudioAdapter` and `TerminalAdapter` (for video display) do not directly consume from these initial buffers. Instead, we have two intermediary "ready for display" queues: `ready_audio_buffer` and `ready_video_buffer`.

The main loop constantly monitors the `audio_buffer` and `encoded_video_buffer`. It waits until both of these buffers have accumulated at least one second's worth of data. Once this condition is met (checked via `has_one_second_ready()` on the `RingBuffer`s), one second of data is atomically moved from `audio_buffer` into `ready_audio_buffer` and from `encoded_video_buffer` into `ready_video_buffer` (using `queue_one_second_into()`).

Both the `AudioAdapter` and `TerminalAdapter` then read and play/display content from their respective `ready_` buffers. This ensures that audio and video are processed and presented in synchronized one-second chunks, preventing one stream from lagging significantly behind the other.

This strategy allows us to maintain a consistent playback experience, even if there are slight variations in the processing speed of audio and video streams.

### Shutdown and cleanup
To ensure that resources are properly released and the terminal is reset to its original state, we implement a graceful shutdown mechanism.

This is done by using channels so each thread can signal to downstrean thread that it is done processing. The main thread waits for all threads to finish before exiting.
