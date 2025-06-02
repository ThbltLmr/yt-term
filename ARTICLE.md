# Streaming YouTube videos in the terminal with the Kitty graphics protocol

## What is the Kitty graphics protocol?
- Graphics protocol implemented in a few terminal emulators to display rgb, rgba or png images
- Explanation on prefix + control data + payload + suffix

## Getting video data for a YouTube video with yt-dlp and ffmpeg
- yt-dlp output to stdout, piped to ffmpeg stdin
- store the output in chunks on 32kb
- gets us a queue of RGB frames

## Encoding frames to be diplayed

## Managing the frame rate

## Improving the display

## Getting audio data

## Synchronizing audio and video

## Shutting down the program at the end of the video

## Conclusion and demo
