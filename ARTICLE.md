# Streaming YouTube videos in the terminal with the Kitty graphics protocol

## What is the Kitty graphics protocol?
- Graphics protocol implemented in a few terminal emulators to display rgb, rgba or png images
- Explanation on prefix + control data + payload + suffix

## Handling asynchronous encoding and display
- we want to encode the data and display it as we go
- need mulitple threads with shared data
- use shared memory controlled with mutexes
- schema with how it works

## Getting video data for a YouTube video with yt-dlp and ffmpeg
- yt-dlp output to stdout, piped to ffmpeg stdin
- store the output in chunks on 32kb
- gets us a queue of RGB frames

## Encoding frames to be diplayed
- which control data do we need
- encode the control data once
- read from queue of rgb frames
- encode each frame in base64
- send to the terminal the whole thing
- now we have a separate queue of frames ready to be displayed

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

## Synchronizing audio and video

## Shutting down the program at the end of the video

## Conclusion and demo
