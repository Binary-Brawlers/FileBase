use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct VideoProcessingPreset {
    pub enabled: bool,
    pub thumbnail: Option<VideoThumbnailOptions>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct VideoTransformations {
    pub video: VideoProcessingPreset,
}

pub fn parse_video_preset(
    transformations: &JsonValue,
) -> Result<VideoProcessingPreset, VideoProcessingError> {
    let parsed: VideoTransformations = serde_json::from_value(transformations.clone())
        .map_err(|e| VideoProcessingError::InvalidTransformations(e.to_string()))?;
    Ok(parsed.video)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct VideoThumbnailOptions {
    pub enabled: bool,
    pub at_seconds: f64,
    pub width: u32,
    pub format: VideoThumbnailFormat,
    pub quality: u8,
}

impl Default for VideoThumbnailOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            at_seconds: 1.0,
            width: 640,
            format: VideoThumbnailFormat::Jpeg,
            quality: 82,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoThumbnailFormat {
    Jpeg,
    Png,
    Webp,
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub duration_seconds: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub frame_rate: Option<f64>,
    pub bit_rate: Option<i64>,
    pub container: Option<String>,
    pub size_bytes: Option<i64>,
}

impl VideoMetadata {
    pub fn to_json(&self) -> JsonValue {
        json!({
            "durationSeconds": self.duration_seconds,
            "width": self.width,
            "height": self.height,
            "codec": self.codec,
            "frameRate": self.frame_rate,
            "bitRate": self.bit_rate,
            "container": self.container,
            "size": self.size_bytes
        })
    }
}

#[derive(Debug, Clone)]
pub struct VideoThumbnail {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum VideoProcessingError {
    #[error("video processor unavailable: {0}")]
    Unavailable(String),
    #[error("video command failed: {0}")]
    Command(String),
    #[error("invalid video transformations: {0}")]
    InvalidTransformations(String),
    #[error("invalid video processor output: {0}")]
    InvalidOutput(String),
}

pub fn validate_video_transformations_json(value: &JsonValue) -> Result<(), VideoProcessingError> {
    let preset = parse_video_preset(value)?;
    let Some(thumbnail) = &preset.thumbnail else {
        return Ok(());
    };
    if !thumbnail.enabled {
        return Ok(());
    }
    if thumbnail.width == 0 || thumbnail.width > 4096 {
        return Err(VideoProcessingError::InvalidTransformations(
            "video thumbnail width must be between 1 and 4096".into(),
        ));
    }
    if !thumbnail.at_seconds.is_finite() || thumbnail.at_seconds < 0.0 {
        return Err(VideoProcessingError::InvalidTransformations(
            "video thumbnail at_seconds must be zero or greater".into(),
        ));
    }
    if !(1..=100).contains(&thumbnail.quality) {
        return Err(VideoProcessingError::InvalidTransformations(
            "video thumbnail quality must be between 1 and 100".into(),
        ));
    }
    Ok(())
}

pub async fn binary_available(binary: &Path) -> bool {
    Command::new(binary)
        .arg("-version")
        .output()
        .await
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub async fn probe(ffprobe: &Path, input: &Path) -> Result<VideoMetadata, VideoProcessingError> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .output()
        .await
        .map_err(|e| VideoProcessingError::Unavailable(e.to_string()))?;
    if !output.status.success() {
        return Err(VideoProcessingError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let parsed: JsonValue = serde_json::from_slice(&output.stdout)
        .map_err(|e| VideoProcessingError::InvalidOutput(e.to_string()))?;
    let format = &parsed["format"];
    let video_stream = parsed["streams"].as_array().and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream["codec_type"].as_str() == Some("video"))
    });

    let duration_seconds = video_stream
        .and_then(|stream| stream["duration"].as_str())
        .and_then(parse_f64)
        .or_else(|| format["duration"].as_str().and_then(parse_f64));
    let width = video_stream
        .and_then(|stream| stream["width"].as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let height = video_stream
        .and_then(|stream| stream["height"].as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let codec = video_stream
        .and_then(|stream| stream["codec_name"].as_str())
        .map(str::to_string);
    let frame_rate = video_stream
        .and_then(|stream| stream["avg_frame_rate"].as_str())
        .and_then(parse_frame_rate);
    let bit_rate = video_stream
        .and_then(|stream| stream["bit_rate"].as_str())
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| {
            format["bit_rate"]
                .as_str()
                .and_then(|v| v.parse::<i64>().ok())
        });
    let container = format["format_name"].as_str().map(str::to_string);
    let size_bytes = format["size"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok());

    Ok(VideoMetadata {
        duration_seconds,
        width,
        height,
        codec,
        frame_rate,
        bit_rate,
        container,
        size_bytes,
    })
}

pub async fn generate_thumbnail(
    ffmpeg: &Path,
    input: &Path,
    options: &VideoThumbnailOptions,
) -> Result<VideoThumbnail, VideoProcessingError> {
    let (codec, mime_type, extension) = match options.format {
        VideoThumbnailFormat::Jpeg => ("mjpeg", "image/jpeg", "jpg"),
        VideoThumbnailFormat::Png => ("png", "image/png", "png"),
        VideoThumbnailFormat::Webp => ("libwebp", "image/webp", "webp"),
    };
    let filter = format!("scale={}:-2", options.width);
    let output = Command::new(ffmpeg)
        .arg("-v")
        .arg("error")
        .arg("-ss")
        .arg(format!("{:.3}", options.at_seconds))
        .arg("-i")
        .arg(input)
        .arg("-frames:v")
        .arg("1")
        .arg("-vf")
        .arg(filter)
        .arg("-f")
        .arg("image2pipe")
        .arg("-vcodec")
        .arg(codec)
        .arg("pipe:1")
        .output()
        .await
        .map_err(|e| VideoProcessingError::Unavailable(e.to_string()))?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err(VideoProcessingError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let decoded = image::load_from_memory(&output.stdout)
        .map_err(|e| VideoProcessingError::InvalidOutput(e.to_string()))?;
    Ok(VideoThumbnail {
        width: decoded.width(),
        height: decoded.height(),
        bytes: output.stdout,
        mime_type: mime_type.to_string(),
        extension: extension.to_string(),
    })
}

fn parse_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn parse_frame_rate(value: &str) -> Option<f64> {
    let (numerator, denominator) = value.split_once('/')?;
    let numerator = numerator.parse::<f64>().ok()?;
    let denominator = denominator.parse::<f64>().ok()?;
    if denominator == 0.0 {
        return None;
    }
    let rate = numerator / denominator;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fractional_frame_rates() {
        assert_eq!(parse_frame_rate("30000/1001"), Some(29.97002997002997));
        assert_eq!(parse_frame_rate("25/1"), Some(25.0));
        assert_eq!(parse_frame_rate("0/0"), None);
        assert_eq!(parse_frame_rate("not-a-rate"), None);
    }

    #[test]
    fn validates_video_thumbnail_options() {
        assert!(validate_video_transformations_json(&json!({
            "video": {
                "enabled": true,
                "thumbnail": { "enabled": true, "width": 640, "at_seconds": 1.5 }
            }
        }))
        .is_ok());
        assert!(validate_video_transformations_json(&json!({
            "video": {
                "enabled": true,
                "thumbnail": { "enabled": true, "width": 0 }
            }
        }))
        .is_err());
        assert!(validate_video_transformations_json(&json!({
            "video": {
                "enabled": true,
                "thumbnail": { "enabled": true, "at_seconds": -1.0 }
            }
        }))
        .is_err());
        assert!(
            !parse_video_preset(&json!({ "enabled": true }))
                .expect("parse legacy shape")
                .enabled
        );
    }

    #[test]
    fn serializes_metadata() {
        let metadata = VideoMetadata {
            duration_seconds: Some(12.5),
            width: Some(1920),
            height: Some(1080),
            codec: Some("h264".to_string()),
            frame_rate: Some(30.0),
            bit_rate: Some(4_000_000),
            container: Some("mov,mp4,m4a,3gp,3g2,mj2".to_string()),
            size_bytes: Some(1_000_000),
        };
        let value = metadata.to_json();
        assert_eq!(value["durationSeconds"], 12.5);
        assert_eq!(value["width"], 1920);
        assert_eq!(value["codec"], "h264");
    }
}
