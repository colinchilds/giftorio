use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_S};
use crate::image_utils::{AnimationInfo, animation_info, resize_dimensions};
use crate::macros::log;
use crate::models::{ImageRotation::*, *};
use crate::progress::set_progress;
use crate::util::{MapJsErr, js_num};
use glam::{DVec2, UVec2, uvec2};
use image::{AnimationDecoder, ImageDecoder, imageops};
use std::{pin::Pin, time::Duration};
use wasm_bindgen::prelude::*;

#[inline(always)]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub type FrameCallback = Box<
    dyn for<'a> Fn(
            Result<&'a image::DynamicImage, &'a JsValue>,
        ) -> Pin<Box<dyn Future<Output = Result<(), JsValue>> + 'a>>
        + 'static,
>;

pub struct FrameData<'a> {
    args: &'a BlueprintArgs,
    curr_n_ms: u32,
    fps_timer: f64,
    frame_i: usize,
    frame_decoder: FrameDecoder<'a>,
    in_dim: UVec2,
    in_n_frames: u32,
    is_first: bool,
    on_frame: Option<FrameCallback>,
    out_dim_raw: DVec2,
    out_n_frames: u32,
    prev_frame: Option<(image::Frame, u32)>,
    samp_i: u32,
}

impl FrameData<'_> {
    /// Returns the dimensions of the output frames.
    ///
    /// Note: This is truncated from the raw dimensions instead of rounded.
    pub fn dimensions(&self) -> UVec2 {
        let (w, h) = resize_dimensions(
            self.in_dim.x,
            self.in_dim.y,
            self.out_dim_raw.x.round() as u32,
            self.out_dim_raw.y.round() as u32,
            false,
        );
        if matches!(self.args.image_rotation, Deg90 | Deg270) {
            uvec2(h, w)
        } else {
            uvec2(w, h)
        }
    }
    pub fn curr_duration_ms(&self) -> u32 {
        self.curr_n_ms
    }
    pub fn curr_total_frames(&self) -> u32 {
        self.samp_i
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

        Ok(Self {
            out_n_frames: match args.mode {
                Mode::Static { .. } => 1,
                _ => expected_output_frames(
                    frame_data.info.duration.as_millis() as u32,
                    args.target_fps.max(1),
                ),
            },
            args,
            curr_n_ms: 0,
            fps_timer: now_ms(),
            frame_i: 0,
            frame_decoder: frame_data.frame_decoder,
            in_dim,
            in_n_frames: n_frames,
            is_first: true,
            on_frame,
            out_dim_raw: scale_factor * DVec2::from(in_dim),
            prev_frame: None,
            samp_i: 0,
        })
    }

    pub async fn next(&mut self) -> Result<Option<image::DynamicImage>, JsValue> {
        let mut frame: Option<image::Frame> = None;

        if self.frame_i == 0 {
            self.fps_timer = now_ms();
            if matches!(self.args.mode, Mode::Static { .. }) {
                frame = match self.frame_decoder.next_frame().await? {
                    Some(frame) => Some(frame.map_js_err()?),
                    None => return Ok(None),
                }
            }
        }

        loop {
            if self.prev_frame.is_some() && self.out_t_ms() <= self.curr_n_ms {
                frame = Some(self.prev_frame.clone().unwrap().0);
                self.samp_i += 1;
            }

            // Flush any remaining frames
            if let Some(frame) = frame {
                let frame = self.convert_frame(frame);

                if let Some(callback) = &self.on_frame {
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
            if self.out_t_ms() >= self.curr_n_ms {
                let frame = match self.frame_decoder.next_frame().await? {
                    Some(frame) => match frame {
                        Ok(frame) => frame,
                        Err(e) => return Err(JsValue::from_str(&format!("Decode err: {e}"))),
                    },
                    None => return Ok(None),
                };

                let (ms, _) = frame.delay().numer_denom_ms(); // Delay is duration of the frame
                let delay = if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms };

                self.curr_n_ms += delay;
                self.prev_frame = Some((frame, self.curr_n_ms));
                self.frame_i += 1;
            }

            if self.is_first {
                self.is_first = false;
            } else {
                set_progress(
                    0.10,
                    0.90,
                    self.frame_i as f64 / self.in_n_frames as f64,
                    &format!(
                        "Processing frame {} / {}  ({:.1} FPS)",
                        self.frame_i,
                        self.in_n_frames,
                        1000.0 * self.frame_i as f64 / (now_ms() - self.fps_timer)
                    ),
                );
            }
        }
    }
}
impl FrameData<'_> {
    //// Returns the current output time in milliseconds.
    fn out_t_ms(&self) -> u32 {
        (self.samp_i * MS_PER_S as u32) / self.args.target_fps
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
    pub frame_decoder: FrameDecoder<'a>,
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

    Ok(ImageFrameData {
        frame_decoder: match args.image_metadata.mime_type.as_str() {
            "image/gif" => {
                let decoder = image::codecs::gif::GifDecoder::new(cursor).map_js_err()?;
                dimensions = UVec2::from(decoder.dimensions());
                info = animation_info(data)?;
                FrameDecoder::new(decoder.into_frames())
            }
            "image/webp" => {
                let decoder = image::codecs::webp::WebPDecoder::new(cursor).map_js_err()?;
                dimensions = UVec2::from(decoder.dimensions());
                info = animation_info(data)?;
                FrameDecoder::new(decoder.into_frames())
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
                FrameDecoder::new(frames)
            }
            t if t.starts_with("video/") => {
                log!("yesssssssssssss video");
                info = AnimationInfo {
                    frames: 1,
                    duration: args
                        .image_metadata
                        .duration
                        .map(Duration::from_secs_f64)
                        .expect("Duration must be provided when decoding unknown video"),
                    timestamps_ms: Some(vec![0]),
                };

                dimensions = UVec2::from(args.image_metadata.dimensions.expect("No dimensions"));
                FrameDecoder {
                    js_next_frame,
                    frames: None,
                }
            }
            t => return Err(JsValue::from_str(&format!("Unsupported image type: {t}"))),
        },
        info,
        dimensions,
    })
}

pub struct FrameDecoder<'a> {
    js_next_frame: Option<js_sys::Function<JsVideoFrameCallback>>,
    frames: Option<image::Frames<'a>>,
}
impl<'a> FrameDecoder<'a> {
    pub fn new(frames: image::Frames<'a>) -> Self {
        Self {
            frames: Some(frames),
            js_next_frame: None,
        }
    }
    async fn next_frame(&mut self) -> Result<Option<image::ImageResult<image::Frame>>, JsValue> {
        if let Some(ref mut frames) = self.frames {
            Ok(frames.next())
        } else if let Some(js_next_frame) = &self.js_next_frame {
            #[derive(serde::Deserialize)]
            struct JsVideoFrameData {
                #[serde(rename = "isFinished")]
                pub is_finished: bool,
            }

            let mem = wasm_bindgen::memory();
            let args = (&mem, &js_num(25), &js_num(25), &js_num(25), &js_num(25));

            let promise = js_next_frame.call(&JsValue::NULL, args).map_js_err()?;
            let js_frame = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_js_err()?;
            let js_frame: JsVideoFrameData =
                serde_wasm_bindgen::from_value(js_frame).map_js_err()?;

            if js_frame.is_finished {
                return Ok(None);
            }
            Ok(None)
        } else {
            Ok(None)
        }
    }
}

pub type JsVideoFrameCallback = fn(
    JsValue,        // mem
    js_sys::Number, // ptr
    js_sys::Number, // len
    js_sys::Number, // width
    js_sys::Number, // height
) -> js_sys::Promise<JsValue>;

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
