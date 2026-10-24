use crate::constants::DEFAULT_FRAME_DELAY;
use crate::image_utils::{AnimationInfo, animation_info, resize_dimensions};
use crate::models::{ImageRotation::*, *};
use crate::progress::set_progress;
use crate::util::{ResultJsExt, format_duration, js_num, now_ms, wasm_shared_memory};
use glam::{DVec2, UVec2, uvec2};
use image::{AnimationDecoder, ImageDecoder, imageops};
use std::{pin::Pin, time::Duration};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub type FrameCallback = Box<
    dyn for<'a> Fn(
            Result<&'a image::DynamicImage, &'a JsValue>,
        ) -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'a>>
        + 'static,
>;

pub struct FrameData<'a> {
    args: &'a BlueprintArgs,
    curr_n_us: u64,
    fps_timer: f64,
    frame_decoder: FrameDecoder<'a>,
    in_dim: UVec2,
    in_frame_i: u32,
    in_n_frames: u32,
    is_first: bool,
    on_frame: Option<FrameCallback>,
    out_dim_raw: DVec2,
    out_frame_i: usize,
    out_n_frames: u32,
    prev_frame: Option<FrameWithTimestamp>,
}

impl FrameData<'_> {
    /// Returns the dimensions of the output frames.
    ///
    /// Note: This is truncated from the raw dimensions instead of rounded.
    pub fn out_dim(&self) -> UVec2 {
        let dim = self.out_dim_raw();
        match self.args.image_rotation {
            Deg90 | Deg270 => uvec2(dim.y, dim.x),
            _ => dim,
        }
    }
    fn out_dim_raw(&self) -> UVec2 {
        let (w, h) = resize_dimensions(
            self.in_dim.x,
            self.in_dim.y,
            self.out_dim_raw.x.round() as u32,
            self.out_dim_raw.y.round() as u32,
            false,
        );
        uvec2(w, h)
    }
    pub fn curr_duration_us(&self) -> u64 {
        self.frame_decoder.curr_n_us
    }
    pub fn curr_total_frames(&self) -> u32 {
        self.in_frame_i
    }
    pub fn total_frames(&self) -> u32 {
        self.out_n_frames
    }
}
impl<'a> FrameData<'a> {
    pub fn new(
        image_data: &'a [u8],
        args: &'a BlueprintArgs,
        on_frame: Option<FrameCallback>,
        js_next_frame: Option<js_sys::Function<JsVideoFrameCallback>>,
    ) -> Result<Self, JsValue> {
        let frame_data = get_frames(image_data, args, js_next_frame)?;
        let in_dim = frame_data.dimensions();
        let n_frames = frame_data.n_frames();
        let scale_factor = (args.max_size as f64 / in_dim.x as f64)
            .min(args.max_size as f64 / in_dim.y as f64)
            .min(1.0);

        fn expected_output_frames(n_ms: u32, fps: u32) -> u32 {
            ((n_ms as u64 * fps as u64) / 1000) as u32 + 1
        }

        Ok({
            let mut obj = Self {
                out_n_frames: match args.mode {
                    Mode::Static { .. } => 1,
                    _ => expected_output_frames(
                        frame_data.info.duration.as_millis() as u32,
                        args.target_fps.max(1),
                    ),
                },
                args,
                curr_n_us: 0,
                fps_timer: now_ms(),
                frame_decoder: frame_data.frame_decoder,
                in_dim,
                in_frame_i: 0,
                in_n_frames: n_frames,
                is_first: true,
                on_frame,
                out_dim_raw: scale_factor * DVec2::from(in_dim),
                out_frame_i: 0,
                prev_frame: None,
            };
            obj.frame_decoder.dim = obj.out_dim_raw();
            obj
        })
    }

    pub async fn next(&mut self) -> Result<Option<image::DynamicImage>, JsValue> {
        let mut frame: Option<image::Frame> = None;
        let use_js_decode = matches!(self.frame_decoder.state, FrameDecoderState::JsFrames { .. });

        if matches!(self.args.mode, Mode::Static { .. }) {
            if let Some(FrameWithTimestamp(f, timestamp)) = self.frame_decoder.next_frame().await? {
                frame = Some(f);
                self.curr_n_us = timestamp;
            } else {
                return Ok(None);
            }
        }
        if self.out_frame_i == 0 {
            self.fps_timer = now_ms();
        }

        loop {
            if self.out_t_us() <= self.curr_n_us
                && let Some(prev) = &self.prev_frame
            {
                frame = Some(prev.0.clone());
                self.out_frame_i += 1;
            }

            // Flush any remaining frames
            if let Some(frame) = frame {
                let frame = self.convert_frame(frame);
                if !use_js_decode && let Some(callback) = &self.on_frame {
                    callback(frame.as_ref()).await?;
                }
                let frame = frame.map(|mut frame| {
                    use crate::blueprint::models::{FlippedAxes::*, ImageRotation::*};
                    frame = match self.args.flipped_axes {
                        X => frame.fliph(),
                        Y => frame.flipv(),
                        Both => frame.flipv().fliph(),
                        _ => frame,
                    };
                    frame = match self.args.image_rotation {
                        Deg90 => frame.rotate90(),
                        Deg180 => frame.rotate180(),
                        Deg270 => frame.rotate270(),
                        _ => frame,
                    };
                    frame
                });
                return Ok(Some(frame?));
            }

            // Only get the next frame if we're past the current output time
            if self.out_t_us() >= self.curr_n_us {
                let frame = self.frame_decoder.next_frame().await?;
                let Some(FrameWithTimestamp(frame, timestamp)) = frame else {
                    return Ok(None);
                };

                self.curr_n_us = timestamp;
                self.prev_frame = Some(FrameWithTimestamp(frame, self.curr_n_us));
                self.in_frame_i += 1;
            }

            if self.is_first {
                self.is_first = false;
            } else {
                let duration: f64;
                let curr_time: f64;
                let progress: f64;

                if use_js_decode {
                    duration = self.args.image_metadata.duration.unwrap();
                    progress = (self.out_frame_i as f64 / self.args.target_fps as f64) / duration;
                    curr_time = self.curr_n_us as f64 / 1000.0 / 1000.0;
                } else {
                    duration = (self.out_n_frames * 1000) as f64 / self.args.target_fps as f64;
                    progress = self.out_frame_i as f64 / self.out_n_frames as f64;
                    curr_time = progress * duration;
                }

                set_progress(
                    0.10,
                    0.90,
                    progress,
                    &format!(
                        "Processing frame {}, {} / {}  ({:.1} FPS)",
                        self.out_frame_i,
                        format_duration((curr_time * 1000.0) as u128),
                        format_duration((duration * 1000.0) as u128),
                        1000.0 * self.out_frame_i as f64 / (now_ms() - self.fps_timer)
                    ),
                );
            }
        }
    }

    //// Returns the current output time in microseconds.
    fn out_t_us(&self) -> u64 {
        (self.out_frame_i as u64 * 1000 * 1000) / self.args.target_fps as u64
    }

    /// Converts the input frame to the target output using the specified settings.
    pub fn convert_frame(&self, frame: image::Frame) -> Result<image::DynamicImage, JsValue> {
        let mut img = image::DynamicImage::ImageRgba8(frame.into_buffer());
        let out_dim = self.out_dim_raw.round().as_uvec2();

        // Apply filters
        for &f in self.args.image_filters.iter() {
            img = match f {
                ImageFilter::Blur(sigma) => img.blur(sigma),
                ImageFilter::Brightness(value) => img.brighten(value),
                ImageFilter::Contrast(c) => img.adjust_contrast(c),
                ImageFilter::HueRotate(value) => img.huerotate(value),
                ImageFilter::Unsharpen(sigma, threshold) => img.unsharpen(sigma, threshold),
            };
        }

        if self.args.grayscale_bits > 0 {
            img = image::DynamicImage::ImageLuma8(img.to_luma8());
        }
        img = match self.args.resampling_filter {
            ResamplingFilter::Thumbnail => img.thumbnail(out_dim.x, out_dim.y),
            filter => img.resize(
                out_dim.x,
                out_dim.y,
                match filter {
                    ResamplingFilter::Catrom => imageops::FilterType::CatmullRom,
                    ResamplingFilter::Gaussian => imageops::FilterType::Gaussian,
                    ResamplingFilter::Lanczos3 => imageops::FilterType::Lanczos3,
                    ResamplingFilter::Nearest => imageops::FilterType::Nearest,
                    ResamplingFilter::Triangle => imageops::FilterType::Triangle,
                    _ => unreachable!(),
                },
            ),
        };

        Ok(img)
    }
}

pub struct ImageFrameData<'a> {
    frame_decoder: FrameDecoder<'a>,
    dimensions: UVec2,
    info: AnimationInfo,
}
impl ImageFrameData<'_> {
    pub fn dimensions(&self) -> UVec2 {
        self.dimensions
    }
    pub fn n_frames(&self) -> u32 {
        self.info.frames
    }
}

/// Decodes the provided image data into a vector of frames based on the image type.
///
/// Supported types: "gif" and "webp".
///
/// # Arguments
///
/// * `image_data` - A byte slice containing the image data.
/// * `image_type` - The type of the image ("gif" or "webp").
///
/// # Returns
///
/// An iterator of `Frame` objects or a JavaScript error.
pub fn get_frames<'a>(
    data: &'a [u8],
    args: &BlueprintArgs,
    js_next_frame: Option<js_sys::Function<JsVideoFrameCallback>>,
) -> Result<ImageFrameData<'a>, JsValue> {
    let cursor = std::io::Cursor::new(data);
    let info: AnimationInfo;
    let dimensions: UVec2;
    let state: FrameDecoderState<'a>;

    match args.image_metadata.mime_type.as_str() {
        "image/gif" => {
            let decoder = image::codecs::gif::GifDecoder::new(cursor).map_js_err("decode err")?;
            dimensions = UVec2::from(decoder.dimensions());
            info = animation_info(data)?;
            state = FrameDecoderState::NativeFrames(decoder.into_frames());
        }
        "image/webp" => {
            let decoder = image::codecs::webp::WebPDecoder::new(cursor).map_js_err("decode err")?;
            dimensions = UVec2::from(decoder.dimensions());
            info = animation_info(data)?;
            state = FrameDecoderState::NativeFrames(decoder.into_frames());
        }
        t if t.starts_with("image/") => {
            let use_combs = matches!(args.mode, Mode::Static { combs: true, .. });
            info = AnimationInfo {
                frames: 1,
                duration: Duration::from_millis(if use_combs { 1000 } else { 0 }),
                timestamps_ms: Some(vec![0]),
            };
            dimensions = UVec2::from(args.image_metadata.dimensions.expect("No image size"));
            let frames = image::Frames::new(Box::new(std::iter::once(Ok(image::Frame::new(
                image::RgbaImage::from_raw(dimensions.x, dimensions.y, data.to_vec())
                    .expect("buffer length must be width * height * 4"),
            ))))); // bruh
            state = FrameDecoderState::NativeFrames(frames);
        }
        t if t.starts_with("video/") => {
            info = AnimationInfo {
                frames: 1,
                duration: Duration::from_secs_f64(
                    (args.image_metadata.duration).expect("Duration must be provided for video"),
                ),
                timestamps_ms: None,
            };
            dimensions = UVec2::from(args.image_metadata.dimensions.expect("No dimensions"));
            let get_frame = js_next_frame.expect("JS next frame must be provided for videos");
            state = FrameDecoderState::JsFrames(get_frame);
        }
        t => return Err(JsValue::from_str(&format!("Unsupported image type: {t}"))),
    };

    Ok(ImageFrameData {
        frame_decoder: FrameDecoder::new(state, dimensions),
        info,
        dimensions,
    })
}

enum FrameDecoderState<'a> {
    NativeFrames(image::Frames<'a>),
    JsFrames(js_sys::Function<JsVideoFrameCallback>),
}

#[derive(Clone)]
struct FrameWithTimestamp(image::Frame, u64);
struct FrameDecoder<'a> {
    curr_n_us: u64,
    dim: UVec2,
    state: FrameDecoderState<'a>,
}
impl<'a> FrameDecoder<'a> {
    fn new(state: FrameDecoderState<'a>, dim: UVec2) -> FrameDecoder<'a> {
        FrameDecoder {
            curr_n_us: 0,
            dim,
            state,
        }
    }

    async fn next_frame(&mut self) -> Result<Option<FrameWithTimestamp>, JsValue> {
        Ok(match &mut self.state {
            FrameDecoderState::NativeFrames(frames) => {
                frames.next().transpose().map_js_err("image")?.map(|f| {
                    let delay = f.delay();
                    let output = FrameWithTimestamp(f, self.curr_n_us);
                    let delay: std::time::Duration = if delay.numer_denom_ms().0 == 0 {
                        DEFAULT_FRAME_DELAY
                    } else {
                        delay.into()
                    };
                    self.curr_n_us += delay.as_micros() as u64;
                    output
                })
            }
            FrameDecoderState::JsFrames(get_next_frame) => {
                let in_buf = vec![0u8; (self.dim.element_product() * 4) as usize];

                let args_obj = js_sys::Object::new();
                let mem = wasm_shared_memory(in_buf.as_slice())?;
                let dim = js_sys::Array::of2(&js_num(self.dim.x), &js_num(self.dim.y));

                js_sys::Reflect::set(&args_obj, &"inBuffer".into(), &mem.into())?;
                js_sys::Reflect::set(&args_obj, &"dimensions".into(), &dim)?;

                let promise = get_next_frame.call(&JsValue::NULL, (&args_obj.into(),))?;
                let result = JsFuture::from(promise).await?;
                let info: JsVideoFrameData = serde_wasm_bindgen::from_value(result)?;

                if info.is_finished {
                    return Ok(None);
                }

                let img_buf = image::RgbaImage::from_raw(self.dim.x, self.dim.y, in_buf);
                let img_buf = img_buf.expect("Invalid RGBA frame dimensions");

                Some(FrameWithTimestamp(
                    image::Frame::new(img_buf),
                    (info.media_time.unwrap() * 1000.0 * 1000.0) as u64,
                ))
            }
        })
    }
}

pub type JsVideoFrameCallback = fn(JsValue) -> js_sys::Promise<JsValue>;

/// Converts an RGB pixel to a single 24 bit integer (inside a u32, I know...).
///
/// Note: This is a fallback implementation for when the SIMD-accelerated version is not available.
///
/// # Arguments
///
/// * `r` - Red channel (0–255).
/// * `g` - Green channel (0–255).
/// * `b` - Blue channel (0–255).
///
/// # Returns
///
/// An integer representing the RGB value.
#[inline(always)]
#[allow(dead_code)]
pub fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
