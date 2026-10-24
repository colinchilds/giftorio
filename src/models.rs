use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintArgs {
    pub custom_height: Option<u32>,
    pub custom_width: Option<u32>,
    pub display_margin_y: u32,
    pub flipped_axes: FlippedAxes,
    pub grayscale_bits: u32,
    pub image_filters: Vec<ImageFilter>,
    pub image_metadata: ImageMetadata,
    pub image_rotation: ImageRotation,
    pub include_power_poles: bool,
    pub max_group_size: Option<u32>,
    pub max_size: u32,
    pub mode: Mode,
    pub name: Option<String>,
    pub output_format: OutputFormat,
    pub power_pole_name: Option<String>,
    pub power_pole_quality: Option<Arc<str>>,
    pub prefer_green_wires: bool,
    pub prefer_horizontal_wires: bool,
    pub resampling_filter: ResamplingFilter,
    pub signal_compression: Option<SignalCompression>,
    pub signal_sorting: SignalSorting,
    pub target_fps: u32,
    pub timing_signals: TimingSignals,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimingSignals {
    pub f: Arc<crate::blueprint::models::SignalID>,
    pub s: Arc<crate::blueprint::models::SignalID>,
    pub t: Arc<crate::blueprint::models::SignalID>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ImageFilter {
    Blur(f32),
    Brightness(i32),
    Contrast(f32),
    HueRotate(i32),
    Unsharpen(f32, i32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub dimensions: Option<(u32, u32)>,
    pub duration: Option<f64>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageRotation {
    #[serde(rename = "none")]
    Deg0 = 0,
    Deg90 = 90,
    Deg180 = 180,
    Deg270 = 270,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlippedAxes {
    None = 0,
    X = 1,
    Y = 2,
    Both = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Full,
    Lamps,
    LampGrid,
    #[serde(rename = "staticImage")]
    Static {
        #[serde(rename = "useConstantCombinators")]
        const_cb: bool,
        #[serde(rename = "useCombinators")]
        combs: bool,
        #[serde(rename = "useTimer")]
        timer: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalSorting {
    None,
    Compression,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResamplingFilter {
    Catrom,
    Gaussian,
    Lanczos3,
    Nearest,
    Thumbnail,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalCompression {
    Delta,
    Temporal { window: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    Blueprint,
    Json,
    RawCompressed,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsVideoFrameData {
    #[serde(default)]
    pub is_finished: bool,
    #[serde(default)]
    pub media_time: Option<f64>,
}
