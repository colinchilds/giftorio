use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_S};
use crate::image_utils::{animation_info, resize_dimensions, AnimationInfo};
use crate::models::{ImageRotation::*, *};
use crate::progress::set_progress;
use glam::{uvec2, DVec2, UVec2};
use image::{imageops, AnimationDecoder, ImageDecoder};
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[inline(always)]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub struct FrameData<'a> {
    args: &'a BlueprintArgs,
    frame_i: u32,
    curr_n_ms: u32,
    fps_timer: f64,
    frames: image::Frames<'a>,
    in_dim: UVec2,
    in_n_frames: u32,
    is_first: bool,
    samp_i: u32,
    out_dim_raw: DVec2,
    out_n_frames: u32,
    prev_frame: Option<(image::Frame, u32)>,
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
    pub fn new(image_data: &'a [u8], args: &'a BlueprintArgs) -> Result<Self, JsValue> {
        let frame_data = get_frames(image_data, &args)?;
        let in_dim = frame_data.dimensions();
        let n_frames = frame_data.n_frames();
        let scale_factor = (args.max_size as f64 / in_dim.x as f64)
            .min(args.max_size as f64 / in_dim.y as f64)
            .min(1.0);

        fn expected_output_frames(n_ms: u32, fps: u32) -> u32 {
            ((n_ms as u64 * fps as u64) / 1000) as u32 + 1
        }

        Ok(Self {
            out_n_frames: if matches!(args.mode, Mode::Static { .. }) {
                1
            } else {
                expected_output_frames(
                    frame_data.total_duration_ms.as_millis() as u32,
                    args.target_fps.max(1),
                )
            },
            args,
            frame_i: 0,
            curr_n_ms: 0,
            fps_timer: now_ms(),
            frames: frame_data.frames,
            in_dim,
            in_n_frames: n_frames,
            is_first: true,
            samp_i: 0,
            out_dim_raw: scale_factor * DVec2::from(in_dim),
            prev_frame: None,
        })
    }
}
impl Iterator for FrameData<'_> {
    type Item = Result<image::DynamicImage, JsValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut frame: Option<image::Frame> = None;

        if self.frame_i == 0 {
            self.fps_timer = now_ms();
            if matches!(self.args.mode, Mode::Static { .. }) {
                frame = match self.frames.next()? {
                    Ok(frame) => Some(frame),
                    Err(e) => return Some(Err(JsValue::from_str(&format!("Decode error: {e}")))),
                }
            }
        }

        loop {
            if self.prev_frame.is_some() {
                if self.out_t_ms() <= self.curr_n_ms {
                    frame = Some(self.prev_frame.clone().unwrap().0);
                    self.samp_i += 1;
                }
            }

            // Flush any remaining frames
            if let Some(frame) = frame {
                return Some(self.convert_frame(frame));
            }

            // Only get the next frame if we're past the current output time
            if self.out_t_ms() >= self.curr_n_ms {
                if let Some(frame) = match self.frames.next() {
                    Some(frame) => match frame {
                        Ok(frame) => Some(frame),
                        Err(e) => {
                            return Some(Err(JsValue::from_str(&format!("Decode error: {e}"))))
                        }
                    },
                    None => return None,
                } {
                    let (ms, _) = frame.delay().numer_denom_ms(); // Delay is duration of the frame
                    let delay = if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms };

                    self.curr_n_ms += delay;
                    self.prev_frame = Some((frame, self.curr_n_ms));
                    self.frame_i += 1;
                }
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
        img = match self.args.sampling_filter {
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
        img = match self.args.flipped_axes {
            FlippedAxes::X => img.fliph(),
            FlippedAxes::Y => img.flipv(),
            FlippedAxes::Both => img.flipv().fliph(),
            _ => img,
        }; // Flip before rotate
        img = match self.args.image_rotation {
            Deg90 => img.rotate90(),
            Deg180 => img.rotate180(),
            Deg270 => img.rotate270(),
            _ => img,
        };
        Ok(img)
    }
}

pub struct ImageFrameData<'a> {
    pub frames: image::Frames<'a>,
    dimensions: UVec2,
    n_frames: u32,
    total_duration_ms: Duration,
}
impl ImageFrameData<'_> {
    pub fn dimensions(&self) -> UVec2 {
        self.dimensions
    }
    pub fn n_frames(&self) -> u32 {
        self.n_frames
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
pub fn get_frames<'a>(data: &'a [u8], args: &BlueprintArgs) -> Result<ImageFrameData<'a>, JsValue> {
    let cursor = std::io::Cursor::new(data);
    let info: AnimationInfo;
    let dim: UVec2;
    let image_type = args.image_metadata.image_type.as_ref();

    Ok(ImageFrameData {
        frames: match image_type.expect("image type required").as_str() {
            "gif" => {
                let decoder = image::codecs::gif::GifDecoder::new(cursor).unwrap_throw();
                dim = UVec2::from(decoder.dimensions());
                info = animation_info(&data)?;
                decoder.into_frames()
            }
            "webp" => {
                let decoder = image::codecs::webp::WebPDecoder::new(cursor).unwrap_throw();
                dim = UVec2::from(decoder.dimensions());
                info = animation_info(&data)?;
                decoder.into_frames()
            }
            _ => {
                let frames = 1;
                let use_combs = matches!(args.mode, Mode::Static { combs: true, .. });
                let duration = Duration::from_millis(if use_combs { 1000 } else { 0 });
                dim = UVec2::from(args.image_metadata.image_size.expect("No image size"));
                info = AnimationInfo { frames, duration };
                image::Frames::new(Box::new(std::iter::once(Ok(image::Frame::new(
                    image::RgbaImage::from_raw(dim.x, dim.y, data.to_vec())
                        .expect("buffer length must be width * height * 4"),
                ))))) // bruh
            }
        },
        dimensions: dim,
        n_frames: info.frames,
        total_duration_ms: info.duration,
    })
}

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
