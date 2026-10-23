use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintArgs {
    pub name: Option<String>,
    #[serde(rename = "customHeight")]
    pub custom_height: Option<u32>,
    #[serde(rename = "customWidth")]
    pub custom_width: Option<u32>,
    #[serde(rename = "flippedAxes")]
    pub flipped_axes: FlippedAxes,
    #[serde(rename = "grayscaleBits")]
    pub grayscale_bits: u32,
    #[serde(rename = "imageFilters")]
    pub image_filters: Vec<ImageFilter>,
    #[serde(rename = "imageMetadata")]
    pub image_metadata: ImageMetadata,
    #[serde(rename = "imageRotation")]
    pub image_rotation: ImageRotation,
    #[serde(rename = "includePowerPoles")]
    pub include_power_poles: bool,
    #[serde(rename = "displayMarginY")]
    pub lamp_margin_y: u32,
    #[serde(rename = "maxGroupSize")]
    pub max_group_size: Option<u32>,
    #[serde(rename = "maxSize")]
    pub max_size: u32,
    #[serde(rename = "mode")]
    pub mode: Mode,
    #[serde(rename = "outputFormat")]
    pub output_format: OutputFormat,
    #[serde(rename = "powerPoleName")]
    pub power_pole_name: Option<String>,
    #[serde(rename = "powerPoleQuality")]
    pub power_pole_quality: Option<Arc<str>>,
    #[serde(rename = "useGreenLampWires")]
    pub prefer_green_wires: bool,
    #[serde(rename = "useHorizontalLampWires")]
    pub prefer_horizontal_wires: bool,
    #[serde(rename = "resamplingFilter")]
    pub sampling_filter: ResamplingFilter,
    #[serde(rename = "signalCompression")]
    pub signal_compression: Option<SignalCompression>,
    #[serde(rename = "signalSorting")]
    pub signal_sorting: SignalSorting,
    #[serde(rename = "targetFps")]
    pub target_fps: u32,

    #[serde(rename = "timingSignals")]
    pub timing_sigs: TimingSignals,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimingSignals {
    pub f: crate::blueprint::models::SignalID,
    pub s: crate::blueprint::models::SignalID,
    pub t: crate::blueprint::models::SignalID,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImageFilter {
    #[serde(rename = "blur")]
    Blur(f32),
    #[serde(rename = "brightness")]
    Brightness(i32),
    #[serde(rename = "contrast")]
    Contrast(f32),
    #[serde(rename = "hueRotate")]
    HueRotate(i32),
    #[serde(rename = "unsharpen")]
    Unsharpen(f32, i32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageMetadata {
    #[serde(rename = "imageType")]
    pub image_type: Option<String>,
    #[serde(default, rename = "imageSize")]
    pub image_size: Option<(u32, u32)>,
    #[serde(default, rename = "imageRotation")]
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageRotation {
    #[serde(rename = "none")]
    Deg0 = 0,
    #[serde(rename = "deg90")]
    Deg90 = 90,
    #[serde(rename = "deg180")]
    Deg180 = 180,
    #[serde(rename = "deg270")]
    Deg270 = 270,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FlippedAxes {
    #[serde(rename = "none")]
    None = 0,
    #[serde(rename = "x")]
    X = 1,
    #[serde(rename = "y")]
    Y = 2,
    #[serde(rename = "both")]
    Both = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "lamps")]
    Lamps,
    #[serde(rename = "lampGrid")]
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
pub enum SignalSorting {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "compression")]
    Compression,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResamplingFilter {
    #[serde(rename = "catrom")]
    Catrom,
    #[serde(rename = "gaussian")]
    Gaussian,
    #[serde(rename = "lanczos3")]
    Lanczos3,
    #[serde(rename = "nearest")]
    Nearest,
    #[serde(rename = "triangle")]
    Triangle,
    #[serde(rename = "thumbnail")]
    Thumbnail,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SignalCompression {
    #[serde(rename = "temporal")]
    Temporal { window: u32 },
    #[serde(rename = "delta")]
    Delta,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "blueprint")]
    Blueprint,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "rawCompressed")]
    RawCompressed,
}
