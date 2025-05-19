use clap::Parser;
use futures::executor::block_on;
use reqwest;
use serde_json::Value;
use std::collections::VecDeque;
use std::error::Error;
use std::io::{BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use url::Url;

// Maximum number of frames to store in the buffer
const MAX_BUFFER_SIZE: usize = 100;

// Command line arguments structure
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// YouTube URL to play
    #[clap(short, long)]
    url: String,

    /// Video quality (e.g., "high", "medium", "low")
    #[clap(short, long, default_value = "medium")]
    quality: String,

    /// Output width in pixels
    #[clap(short, long, default_value = "640")]
    width: u32,

    /// Output height in pixels
    #[clap(short, long, default_value = "480")]
    height: u32,

    /// Frames per second
    #[clap(short, long, default_value = "30")]
    fps: u32,
}

// Structure to hold video frames
struct VideoFrame {
    data: Vec<u8>,
    timestamp: u64,
}

// Main video buffer
struct VideoBuffer {
    frames: VecDeque<VideoFrame>,
}

impl VideoBuffer {
    fn new() -> Self {
        VideoBuffer {
            frames: VecDeque::with_capacity(MAX_BUFFER_SIZE),
        }
    }

    fn push_frame(&mut self, frame: VideoFrame) {
        if self.frames.len() >= MAX_BUFFER_SIZE {
            // If buffer is full, remove the oldest frame
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    fn get_frame(&mut self) -> Option<VideoFrame> {
        self.frames.pop_front()
    }

    fn len(&self) -> usize {
        self.frames.len()
    }
}

// Custom error type for our application
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Create a shared buffer for frames
    let buffer = Arc::new(Mutex::new(VideoBuffer::new()));

    // Clone the buffer reference for the producer thread
    let producer_buffer = Arc::clone(&buffer);

    // Start the producer thread to fetch and process video frames
    let producer_handle = thread::spawn(move || {
        if let Err(e) = fetch_youtube_video(
            args.url,
            args.quality,
            args.width,
            args.height,
            args.fps,
            producer_buffer,
        ) {
            eprintln!("Error in producer thread: {}", e);
        }
    });

    // Clone the buffer reference for the consumer thread
    let consumer_buffer = Arc::clone(&buffer);

    // Start the consumer thread to display frames
    let consumer_handle = thread::spawn(move || {
        display_frames(consumer_buffer, args.fps);
    });

    // Wait for both threads to finish
    producer_handle.join().unwrap();
    consumer_handle.join().unwrap();

    Ok(())
}

fn extract_video_id(youtube_url: &str) -> Result<String> {
    let url = Url::parse(youtube_url)?;

    // Extract video ID from various YouTube URL formats
    if let Some(host) = url.host_str() {
        if host.contains("youtube.com") {
            // youtube.com/watch?v=VIDEO_ID format
            if let Some(video_id) = url
                .query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.to_string())
            {
                return Ok(video_id);
            }
        } else if host.contains("youtu.be") {
            // youtu.be/VIDEO_ID format
            if let Some(path) = url.path().strip_prefix('/') {
                return Ok(path.to_string());
            }
        }
    }

    Err("Could not extract YouTube video ID".into())
}

async fn get_video_info(video_id: &str) -> Result<Value> {
    // This is a simplified approach and might need adjustments as YouTube's API changes
    let info_url = format!(
        "https://www.youtube.com/get_video_info?video_id={}&el=embedded&ps=default&eurl=&gl=US&hl=en",
        video_id
    );

    let response = reqwest::get(&info_url).await?;
    let body = response.text().await?;

    // Parse the response which is URL encoded
    let parsed: serde_urlencoded::de::Deserializer<'_, serde_urlencoded::de::UrlEncodedValue> =
        serde_urlencoded::Deserializer::new(body.as_str());
    let parsed_map: std::collections::HashMap<String, String> =
        serde::Deserialize::deserialize(parsed)?;

    // The player response contains streaming info
    if let Some(player_response) = parsed_map.get("player_response") {
        let json: Value = serde_json::from_str(player_response)?;
        return Ok(json);
    }

    Err("Could not parse video info".into())
}

fn find_best_format(json: &Value, quality: &str) -> Result<String> {
    // Extract streaming URLs based on quality preference
    let formats = json
        .get("streamingData")
        .and_then(|data| data.get("formats"))
        .ok_or("No streaming formats found")?;

    // Find suitable format based on quality preference
    if let Some(formats) = formats.as_array() {
        let quality_rank = match quality {
            "high" => 2,
            "medium" => 1,
            _ => 0, // "low" or anything else
        };

        // Sort formats by quality and get the best one according to preference
        let mut sorted_formats: Vec<_> = formats.iter().collect();
        sorted_formats.sort_by(|a, b| {
            let a_height = a.get("height").and_then(|h| h.as_u64()).unwrap_or(0);
            let b_height = b.get("height").and_then(|h| h.as_u64()).unwrap_or(0);

            if quality_rank == 2 {
                // high quality: descending
                b_height.cmp(&a_height)
            } else if quality_rank == 1 {
                // medium quality: middle
                let a_diff = if a_height > 480 {
                    a_height - 480
                } else {
                    480 - a_height
                };
                let b_diff = if b_height > 480 {
                    b_height - 480
                } else {
                    480 - b_height
                };
                a_diff.cmp(&b_diff)
            } else {
                // low quality: ascending
                a_height.cmp(&b_height)
            }
        });

        // Get the URL of the selected format
        if let Some(best_format) = sorted_formats.first() {
            if let Some(url) = best_format.get("url").and_then(|u| u.as_str()) {
                return Ok(url.to_string());
            }
        }
    }

    Err("Could not find suitable video format".into())
}

fn fetch_youtube_video(
    url: String,
    quality: String,
    width: u32,
    height: u32,
    fps: u32,
    buffer: Arc<Mutex<VideoBuffer>>,
) -> Result<()> {
    println!("Fetching video from: {}", url);

    // Extract video ID from URL
    let video_id = extract_video_id(&url)?;
    println!("Extracted video ID: {}", video_id);

    // Get video info and streaming URL
    let video_info = block_on(get_video_info(&video_id))?;
    let video_url = find_best_format(&video_info, &quality)?;

    println!("Got video URL, now processing with ffmpeg");

    // Use ffmpeg to decode the video into raw frames
    let mut ffmpeg_process = Command::new("ffmpeg")
        .args([
            "-i",
            &video_url,
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            &format!("{}x{}", width, height),
            "-r",
            &fps.to_string(),
            "-", // Output to stdout
        ])
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = ffmpeg_process
        .stdout
        .take()
        .expect("Failed to open ffmpeg stdout");
    let mut reader = BufReader::new(stdout);

    // Calculate the size of each frame in bytes
    let frame_size = (width * height * 3) as usize; // RGB24 = 3 bytes per pixel
    let mut frame_data = vec![0u8; frame_size];
    let mut timestamp: u64 = 0;

    // Read frames from ffmpeg and add them to the buffer
    loop {
        match reader.read_exact(&mut frame_data) {
            Ok(_) => {
                let frame = VideoFrame {
                    data: frame_data.clone(),
                    timestamp,
                };

                // Acquire the lock and push the frame to the buffer
                let mut video_buffer = buffer.lock().unwrap();
                video_buffer.push_frame(frame);

                // Print buffer status
                println!(
                    "Producer: Added frame {}. Buffer size: {}",
                    timestamp,
                    video_buffer.len()
                );

                // Release the lock
                drop(video_buffer);

                timestamp += 1;
            }
            Err(_) => break,
        }
    }

    println!("Video processing complete");
    Ok(())
}

fn display_frames(buffer: Arc<Mutex<VideoBuffer>>, fps: u32) {
    // Calculate frame duration based on fps
    let frame_duration = Duration::from_millis(1000 / fps as u64);

    // Start consuming frames
    loop {
        // Try to get a frame from the buffer
        let frame = {
            let mut video_buffer = buffer.lock().unwrap();
            video_buffer.get_frame()
        };

        // If we got a frame, display it
        if let Some(frame) = frame {
            // Convert the frame to a format suitable for terminal display
            let base64_frame = base64::encode(&frame.data);

            // Display the frame using Kitty graphics protocol
            display_kitty_image(&base64_frame, frame.timestamp);

            // Wait for the next frame duration
            thread::sleep(frame_duration);
        } else {
            // If no frames are available, wait a bit before trying again
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn display_kitty_image(base64_data: &str, timestamp: u64) {
    // This is a simplified implementation of the Kitty graphics protocol
    // Full implementation would need to follow the protocol specification:
    // https://sw.kovidgoyal.net/kitty/graphics-protocol/

    // Format: ESC + _ + G + <payload> + ESC + \
    // a=T: transmit directly
    // f=24: RGB data format (24 bits per pixel)
    // s=<width>: width of the image
    // v=<height>: height of the image
    // m=1: compressed base64 data follows
    println!("\x1b_Ga=T,f=24,s=100,v=100,m=1;{}\x1b\\", base64_data);
    println!("Consumer: Displayed frame {}", timestamp);
}
