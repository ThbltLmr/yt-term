# Streaming YouTube videos in the terminal with the Kitty graphics protocol

I recently started using the terminal emulator [Ghostty](https://ghostty.org) and the following line in the documentation peeked my interest:

> Kitty graphics protocol: Ghostty supports the Kitty graphics protocol, which allows terminal applications to render images directly in the terminal.

In this article, we will learn what the Kitty graphics protocol is, and attempt to use it to stream a YouTube video directly in the terminal.

## What is the Kitty graphics protocol?
The [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol) is a specification allowing client programs running into terminal emulators to display images using RBG, RGBA or PNG format. While initially developed for [Kitty](https://sw.kovidgoyal.net/kitty/), it has been implemented in other terminals like Ghostty and WezTerm. All the client program has to do is send a graphics escape code to `STDOUT` with the right escape characters and encoding.

So what does that look like? The specification tells us:

`<ESC>_G<control data>;<payload><ESC>\`

The `<ESC>_G` prefix and the `<ESC>\` suffix are the delimiters to let the terminal know where our image data starts and ends. The two interesting parts in this sequence are the `control_data` and the `payload`.

### Control data
The control data is a series of comma-separated key-value pairs. It includes some metadata about the image, such as its format, width or height, as well as some instructions for the terminal on how to display the image. You can find a full reference [here](https://sw.kovidgoyal.net/kitty/graphics-protocol/#control-data-reference).

For instance, if we just need to display some basic RGB data, we just need the following:
```
<ESC>_Gf=24,s=<image width>,v=<image height>,a=T;<payload><ESC>\
```
In this example, the `f`, `s` and `v` keys are the image metadata, and `a=T` tells the terminal we want it to display the image.

### Payload
The payload is the actual image data, encoded in base 64. It can be either a file path or the raw image data (the `t` key in the control data can be used to tell the terminal whether we're sending raw data or a file path).

```
# Sending the RGB data directly in the payload
<ESC>_Gf=24,s=<image width>,v=<image height>,a=T,t=d;<base64_encoded_pixels><ESC>\ 
# Sending the path to a file containing RGB data
<ESC>_Gf=24,s=<image width>,v=<image height>,a=T;t=f<base64_encoded_file_path><ESC>\ 
# Sending the path to a PNG file; width and height are not necessary as they will be in the PNG metadata
<ESC>_Gf=100,a=T;t=f<base64_encoded_file_path><ESC>\ 
```

## Handling parallel encoding and display
Since our goal is to stream YouTube videos, we are going to need to encode and display frames simultaneously. The approach I chose to store, encode and display frames was the following:
- Our program's state is composed of two queues: one stores the frames before we've encoded them to follow the graphics protocol, the other one stores the graphics escape codes ready to be sent to `STDOUT`;
- One thread receives data from YouTube and stores it in the first queue;
- A second thread pops each frame from this first queue, converts it to the graphics escape code to display, and stores it in the second queue;
- A third thread pops the graphics escape codes from the second queue to display them at the right frame rate. 

Since I assumed all threads would be simultaneously active with little idle time, I used Rust's `std::thread` and no async runtime. I also used mutexes to share the buffers across threads.

The flow of data in our program should look something like this:

<EXCALIDRAW>

## Getting video data for a YouTube video with yt-dlp and ffmpeg
The first step is to get the video data from YouTube into our first queue (storing frames before we've encoded them into graphics escape codes). Considering the variety of existing video formats and the complexity of the YouTube API, I cowardly decided to rely on the superior programmers at `yt-dlp` and `ffmpeg` to provide me a stream of RGB frames.

First, I picked a width and height for the frames I wanted to display. I pretended it was 2010 and went with 360p (i.e. 360 * 640), to avoid any performance issues. Therefore, I knew I was going to store chunks of 360 * 640 * 3 bytes per pixel = 691200 bytes of RGB data per frame. I could then:
- start `yt-dlp` for my YouTube video of choice, selecting a 360p format and outputting the result to `STDOUT`;
- pipe the `yt-dlp` output to `ffmpeg`;
- kindly ask `ffmpeg` to convert the data to RGB and output it to `STDOUT`

I could then read the `ffmpeg` output and store each chunk of 691kB to our first queue.

<details>
<summary>This is what the function handling this pipeline looks like</summary>

```rust
    pub fn stream(&self) -> Res<()> {
        let frame_size = self.width * self.height * 3;
        let mut yt_dlp_process = Command::new("yt-dlp")
            .args([
                "-o",
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

        let mut read_buffer = vec![0u8; frame_size];

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
</details>

## Encoding frames to be diplayed
Now that we are have a queue of RGB frames, we need to convert them to graphics escape codes matching the Kitty graphics protocol. To do so, we need some control data (which will be the same in every escape code), and we need to encode the RGB data in base64. 
Here are the control data key-value pairs that we need:
- `f=24`: to signal that we are sending RGB data;
- `s=640`: the height of the image;
- `v=360`: the width of the image;
- `t=d`: to signal that the image data will be directly in the payload;
- `a=T`: to instruct the terminal to display the frame when received.

Once we have this control data, we simply need to repeat the same few steps for each frame:
- read the RGB data from our first queue;
- encode it in base 64 (I used the base64 crate)
- return a slice with the encode prefix (`<ESC>_G`), our control data, the base 64 encoded data, and the suffix (`<ESC>\`)
- store this slice in our second queue, ready for display;

<details>
<summary>This is what my encoding class looks like</summary>

```rust
fn encode_frame(&self, encoded_control_data: Vec<u8>, frame: Frame) -> Frame {
    // Base64 encode the frame data
    let encoded_payload = self.encode_rgb(frame.data);
    let prefix = b"\x1b_G";
    let suffix = b"\x1b\\";
    let delimiter = b";";
    let mut buffer = vec![];
    buffer.extend_from_slice(prefix);
    buffer.extend_from_slice(&encoded_control_data);
    buffer.extend_from_slice(delimiter);
    buffer.extend_from_slice(&encoded_payload);
    buffer.extend_from_slice(suffix);
    Frame::new(buffer)
}
pub fn encode(&mut self) -> Res<()> {
    loop {
        let mut rgb_buffer = self.rgb_buffer.lock().unwrap();
        let x_offset = (self.term_width as usize - self.width) / 2;
        let y_offset = (self.term_height as usize - self.height) / 2;
        let encoded_control_data = self.encode_control_data(HashMap::from([
            ("f".into(), "24".into()),
            ("s".into(), format!("{}", self.width)),
            ("v".into(), format!("{}", self.height)),
            ("t".into(), "d".into()),
            ("a".into(), "T".into()),
            ("X".into(), format!("{}", x_offset)),
            ("Y".into(), format!("{}", y_offset)),
        ]));
        let frame = rgb_buffer.get_el();
        if let Some(frame) = frame {
            let encoded_frame = self.encode_frame(encoded_control_data, frame);
            let mut encoded_buffer = self.encoded_buffer.lock().unwrap();
            encoded_buffer.push_el(encoded_frame);
        } else if self.streaming_done_rx.try_recv().is_ok() {
            self.encoding_done_tx.send(()).unwrap();
            return Ok(());
        }
    }
}
fn encode_control_data(&self, control_data: HashMap<String, String>) -> Vec<u8> {
    let mut encoded_data = Vec::new();
    for (key, value) in control_data {
        encoded_data.push(format!("{}={}", key, value));
    }
    encoded_data.join(",").as_bytes().to_vec()
}
fn encode_rgb(&self, rgb: Vec<u8>) -> Vec<u8> {
    let encoded = general_purpose::STANDARD.encode(&rgb);
    encoded.as_bytes().to_vec()
}
```
</details>

## Managing the frame rate
- read from the queue of frames to be displayed
- time when each frame is displayed
- only display next frame once 40 ms have passed
- issue: what if it took 40+ ms to display the frame
- skip frames when needed

## Improving the display
- clearing the terminal and resetting cursor
- using alternate screen
- centering frames

## Getting audio data
- same logic as for video, pipe yt-dlp into ffmpeg
- no need for encoding
- output to pulseaudio with pulse crate

## Synchronizing audio and video
- we can't guarantee that both streams will start at the exact same time + one might buffer
- implement a 'ready' queue on both sides, to which we add the same time
- we add one second of content to these queues when both are ready
- we play from these queues

## Shutting down the program at the end of the video
- each producer lets consumer know when it's done
- if producer is done + queue to be consumed is empty, consumer stops and lets downstream consumer know
- once the display consumer is done, we can shutdown the program

## Conclusion and demo
- screeen capture of demo
- current memory usage
- potential for optimization
