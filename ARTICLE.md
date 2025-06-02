# Streaming YouTube videos in the terminal with the Kitty graphics protocol

I recently started using the terminal emulator [Ghostty](https://ghostty.org) and the following line in the documentation peeked my interest:

> Kitty graphics protocol: Ghostty supports the Kitty graphics protocol, which allows terminal applications to render images directly in the terminal.

In this article, we will learn what the Kitty graphics protocol is, and attempt to use it to stream a YouTube video directly in the terminal.

## What is the Kitty graphics protocol?
The [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol) is a specification allowing client programs running into terminal emulators to display images using RBG, RGBA or PNG format. While initially developed for [Kitty](https://sw.kovidgoyal.net/kitty/), it has been implemented in other terminals like Ghostty and WezTerm. All the client program has to do is send a sequence to `STDOUT` with the right escape characters and encoding.

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
<ESC>_Gf=24,s=<image width>,v=<image height>,a=T,t=d;<base64_encoded_pixels><ESC>\ # Sending the RGB data directly in the payload
<ESC>_Gf=24,s=<image width>,v=<image height>,a=T;t=f<base64_encoded_file_path><ESC>\ # Sending the path to a file containing RGB data
<ESC>_Gf=100,a=T;t=f<base64_encoded_file_path><ESC>\ # Sending the path to a PNG file; width and height are not necessary as they will be in the PNG metadata
```

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
