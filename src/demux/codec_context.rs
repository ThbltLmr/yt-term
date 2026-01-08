use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi;
use std::ptr;

const AV_INPUT_BUFFER_PADDING_SIZE: usize = 64;

/// Creates an H.264 video decoder from avcC extradata.
///
/// # Safety
/// This function uses raw FFmpeg FFI calls to allocate and configure codec contexts.
/// The caller must ensure ffmpeg has been initialized via `ffmpeg::init()`.
pub unsafe fn create_h264_decoder(
    avcc_extradata: &[u8],
) -> Result<ffmpeg::decoder::Video, ffmpeg::Error> {
    // Allocate AVCodecParameters
    let params = ffi::avcodec_parameters_alloc();
    if params.is_null() {
        return Err(ffmpeg::Error::Unknown);
    }

    // Set video codec parameters
    (*params).codec_type = ffi::AVMediaType::AVMEDIA_TYPE_VIDEO;
    (*params).codec_id = ffi::AVCodecID::AV_CODEC_ID_H264;
    (*params).width = 640;
    (*params).height = 360;
    (*params).format = ffi::AVPixelFormat::AV_PIX_FMT_YUV420P as i32;

    // Allocate and copy avcC extradata with required padding
    let extradata_size = avcc_extradata.len();
    let extradata = ffi::av_malloc(extradata_size + AV_INPUT_BUFFER_PADDING_SIZE);
    if extradata.is_null() {
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::Unknown);
    }

    // Copy extradata and zero the padding
    ptr::copy_nonoverlapping(avcc_extradata.as_ptr(), extradata as *mut u8, extradata_size);
    ptr::write_bytes(
        (extradata as *mut u8).add(extradata_size),
        0,
        AV_INPUT_BUFFER_PADDING_SIZE,
    );

    (*params).extradata = extradata as *mut u8;
    (*params).extradata_size = extradata_size as i32;

    // Find the H.264 decoder
    let codec = ffi::avcodec_find_decoder(ffi::AVCodecID::AV_CODEC_ID_H264);
    if codec.is_null() {
        ffi::av_free(extradata);
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::DecoderNotFound);
    }

    // Allocate codec context
    let ctx = ffi::avcodec_alloc_context3(codec);
    if ctx.is_null() {
        ffi::av_free(extradata);
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::Unknown);
    }

    // Copy parameters to context
    let ret = ffi::avcodec_parameters_to_context(ctx, params);
    if ret < 0 {
        ffi::avcodec_free_context(&mut (ctx as *mut _));
        ffi::av_free(extradata);
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::from(ret));
    }

    // Open the decoder
    let ret = ffi::avcodec_open2(ctx, codec, ptr::null_mut());
    if ret < 0 {
        ffi::avcodec_free_context(&mut (ctx as *mut _));
        ffi::av_free(extradata);
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::from(ret));
    }

    // Clean up parameters (extradata ownership transferred to context)
    // Note: Don't free extradata here - it's now owned by the context
    (*params).extradata = ptr::null_mut();
    (*params).extradata_size = 0;
    ffi::avcodec_parameters_free(&mut (params as *mut _));

    // Wrap in safe Context type and get video decoder
    let context = ffmpeg::codec::context::Context::wrap(ctx, None);
    context.decoder().video()
}

/// Creates an AAC audio decoder.
///
/// # Safety
/// This function uses raw FFmpeg FFI calls to allocate and configure codec contexts.
/// The caller must ensure ffmpeg has been initialized via `ffmpeg::init()`.
pub unsafe fn create_aac_decoder() -> Result<ffmpeg::decoder::Audio, ffmpeg::Error> {
    // Allocate AVCodecParameters
    let params = ffi::avcodec_parameters_alloc();
    if params.is_null() {
        return Err(ffmpeg::Error::Unknown);
    }

    // Set audio codec parameters
    (*params).codec_type = ffi::AVMediaType::AVMEDIA_TYPE_AUDIO;
    (*params).codec_id = ffi::AVCodecID::AV_CODEC_ID_AAC;
    (*params).sample_rate = 44100;
    (*params).format = ffi::AVSampleFormat::AV_SAMPLE_FMT_FLTP as i32;

    // Set stereo channel layout
    ffi::av_channel_layout_default(&mut (*params).ch_layout, 2);

    // Find the AAC decoder
    let codec = ffi::avcodec_find_decoder(ffi::AVCodecID::AV_CODEC_ID_AAC);
    if codec.is_null() {
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::DecoderNotFound);
    }

    // Allocate codec context
    let ctx = ffi::avcodec_alloc_context3(codec);
    if ctx.is_null() {
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::Unknown);
    }

    // Copy parameters to context
    let ret = ffi::avcodec_parameters_to_context(ctx, params);
    if ret < 0 {
        ffi::avcodec_free_context(&mut (ctx as *mut _));
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::from(ret));
    }

    // Open the decoder
    let ret = ffi::avcodec_open2(ctx, codec, ptr::null_mut());
    if ret < 0 {
        ffi::avcodec_free_context(&mut (ctx as *mut _));
        ffi::avcodec_parameters_free(&mut (params as *mut _));
        return Err(ffmpeg::Error::from(ret));
    }

    // Clean up parameters
    ffi::avcodec_parameters_free(&mut (params as *mut _));

    // Wrap in safe Context type and get audio decoder
    let context = ffmpeg::codec::context::Context::wrap(ctx, None);
    context.decoder().audio()
}
