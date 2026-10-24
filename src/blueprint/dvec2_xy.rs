use glam::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct XY {
    x: f64,
    y: f64,
}

pub fn serialize<S>(v: &DVec2, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    XY { x: v.x, y: v.y }.serialize(serializer)
}
pub fn deserialize<'de, D>(deserializer: D) -> Result<DVec2, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let xy = XY::deserialize(deserializer)?;
    Ok(DVec2::new(xy.x, xy.y))
}
