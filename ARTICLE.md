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
