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

We then store these RGB frames in a buffer, from which we can read to encode them to the Kitty graphics protocol.
