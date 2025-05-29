# yt-term
Real time streaming terminal player based on [yt-dlp](https://github.com/yt-dlp/yt-dlp).
You can find a list of supported sites in the [yt-dlp documentation](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md).

## Dependencies
This program uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) to extract data from video websites, and [ffmpeg](https://www.ffmpeg.org) to pre-encode frames to RGB.
Please note this also requires a terminal that supports the Kitty graphics protocol, e.g. [Kitty](https://sw.kovidgoyal.net/kitty/), [Ghostty](https://ghostty.org/).

## What it this project?
This is an experiment to stream video in a terminal using the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/#). 
It is not meant to be a full-featured video player. I have not tested in any terminal other than Ghostty, and I have not tested with various websites and video formats. 
