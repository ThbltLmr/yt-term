use ffmpeg_next::{self as ffmpeg};

fn decode_frame(frame: Vec<u8>) -> Vec<u8> {
    ffmpeg::init().unwrap();

    let mut codec = ffmpeg::codec::decoder::find(ffmpeg::codec::Id::H264).unwrap();

    let decoder = ffmpeg::codec::context::Context::new_with_codec(codec).decoder();
}
