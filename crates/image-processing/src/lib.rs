use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    imageops::FilterType,
    DynamicImage, GenericImageView, ImageEncoder, ImageFormat,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ImageProcessingPreset {
    pub enabled: bool,
    pub format: Option<ImageOutputFormat>,
    pub quality: Option<u8>,
    pub resize: Option<ResizeOptions>,
    pub thumbnail: Option<ThumbnailOptions>,
    pub preserve_original: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageOutputFormat {
    Original,
    Jpeg,
    Png,
    Webp,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ResizeOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mode: ResizeMode,
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            mode: ResizeMode::Fit,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResizeMode {
    Fit,
    Fill,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ThumbnailOptions {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub format: Option<ImageOutputFormat>,
    pub quality: Option<u8>,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 320,
            height: 320,
            format: None,
            quality: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub metadata: JsonValue,
    pub original: Option<OriginalImage>,
    pub thumbnail: Option<ThumbnailImage>,
}

#[derive(Debug, Clone)]
pub struct OriginalImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ThumbnailImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageProcessingError {
    #[error("invalid image transformations: {0}")]
    InvalidTransformations(String),
    #[error("image decode failed: {0}")]
    Decode(String),
    #[error("image encode failed: {0}")]
    Encode(String),
}

pub fn process_image(
    bytes: &[u8],
    source_mime: &str,
    source_extension: &str,
    transformations: &JsonValue,
) -> Result<Option<ProcessedImage>, ImageProcessingError> {
    let preset: ImageProcessingPreset = serde_json::from_value(transformations.clone())
        .map_err(|e| ImageProcessingError::InvalidTransformations(e.to_string()))?;
    if !preset.enabled || !source_mime.starts_with("image/") {
        return Ok(None);
    }

    validate_dimensions(preset.resize.as_ref())?;
    if let Some(thumbnail) = &preset.thumbnail {
        if thumbnail.enabled && (thumbnail.width == 0 || thumbnail.height == 0) {
            return Err(ImageProcessingError::InvalidTransformations(
                "thumbnail width and height must be greater than zero".into(),
            ));
        }
    }

    let decoded =
        image::load_from_memory(bytes).map_err(|e| ImageProcessingError::Decode(e.to_string()))?;
    let (source_width, source_height) = decoded.dimensions();
    let resized = apply_resize(decoded, preset.resize.as_ref());
    let (width, height) = resized.dimensions();
    let output_format = choose_format(preset.format, source_mime, source_extension);
    let quality = preset.quality.unwrap_or(82).clamp(1, 100);
    let output_bytes = encode_image(&resized, output_format, quality)?;

    let thumbnail = match preset.thumbnail.as_ref().filter(|t| t.enabled) {
        Some(options) => {
            let thumb_image =
                resized.resize_to_fill(options.width, options.height, FilterType::Lanczos3);
            let format = choose_format(
                options.format.or(preset.format),
                source_mime,
                source_extension,
            );
            let quality = options
                .quality
                .or(preset.quality)
                .unwrap_or(82)
                .clamp(1, 100);
            let bytes = encode_image(&thumb_image, format, quality)?;
            Some(ThumbnailImage {
                size: bytes.len() as u64,
                bytes,
                mime_type: mime_for_format(format).to_string(),
                extension: extension_for_format(format).to_string(),
                width: options.width,
                height: options.height,
            })
        }
        None => None,
    };

    let original = preset.preserve_original.then(|| OriginalImage {
        bytes: bytes.to_vec(),
        mime_type: source_mime.to_string(),
        extension: source_extension.to_string(),
        size: bytes.len() as u64,
    });

    Ok(Some(ProcessedImage {
        metadata: json!({
            "image": {
                "source": {
                    "mimeType": source_mime,
                    "extension": source_extension,
                    "size": bytes.len(),
                    "width": source_width,
                    "height": source_height
                },
                "output": {
                    "mimeType": mime_for_format(output_format),
                    "extension": extension_for_format(output_format),
                    "size": output_bytes.len(),
                    "width": width,
                    "height": height,
                    "quality": quality
                },
                "thumbnail": thumbnail.as_ref().map(|t| json!({
                    "mimeType": t.mime_type,
                    "extension": t.extension,
                    "size": t.size,
                    "width": t.width,
                    "height": t.height
                })),
                "preservedOriginal": preset.preserve_original
            }
        }),
        bytes: output_bytes,
        mime_type: mime_for_format(output_format).to_string(),
        extension: extension_for_format(output_format).to_string(),
        original,
        thumbnail,
    }))
}

pub fn validate_transformations_json(value: &JsonValue) -> Result<(), ImageProcessingError> {
    let preset: ImageProcessingPreset = serde_json::from_value(value.clone())
        .map_err(|e| ImageProcessingError::InvalidTransformations(e.to_string()))?;
    validate_dimensions(preset.resize.as_ref())?;
    if let Some(thumbnail) = &preset.thumbnail {
        if thumbnail.enabled && (thumbnail.width == 0 || thumbnail.height == 0) {
            return Err(ImageProcessingError::InvalidTransformations(
                "thumbnail width and height must be greater than zero".into(),
            ));
        }
    }
    Ok(())
}

fn validate_dimensions(resize: Option<&ResizeOptions>) -> Result<(), ImageProcessingError> {
    let Some(resize) = resize else {
        return Ok(());
    };
    if resize.width.unwrap_or(1) == 0 || resize.height.unwrap_or(1) == 0 {
        return Err(ImageProcessingError::InvalidTransformations(
            "resize width and height must be greater than zero".into(),
        ));
    }
    if resize.width.is_none() && resize.height.is_none() {
        return Err(ImageProcessingError::InvalidTransformations(
            "resize requires width or height".into(),
        ));
    }
    Ok(())
}

fn apply_resize(image: DynamicImage, resize: Option<&ResizeOptions>) -> DynamicImage {
    let Some(resize) = resize else {
        return image;
    };
    let (width, height) = image.dimensions();
    let target_width = resize.width.unwrap_or(width);
    let target_height = resize.height.unwrap_or(height);
    match resize.mode {
        ResizeMode::Fit => image.resize(target_width, target_height, FilterType::Lanczos3),
        ResizeMode::Fill => image.resize_to_fill(target_width, target_height, FilterType::Lanczos3),
    }
}

fn choose_format(
    requested: Option<ImageOutputFormat>,
    source_mime: &str,
    source_extension: &str,
) -> ImageOutputFormat {
    match requested.unwrap_or(ImageOutputFormat::Original) {
        ImageOutputFormat::Original => match source_mime {
            "image/jpeg" => ImageOutputFormat::Jpeg,
            "image/png" => ImageOutputFormat::Png,
            "image/webp" => ImageOutputFormat::Webp,
            _ => match source_extension {
                "jpg" | "jpeg" => ImageOutputFormat::Jpeg,
                "png" => ImageOutputFormat::Png,
                "webp" => ImageOutputFormat::Webp,
                _ => ImageOutputFormat::Jpeg,
            },
        },
        format => format,
    }
}

fn encode_image(
    image: &DynamicImage,
    format: ImageOutputFormat,
    quality: u8,
) -> Result<Vec<u8>, ImageProcessingError> {
    let mut out = Vec::new();
    match format {
        ImageOutputFormat::Original => unreachable!("original format must be resolved first"),
        ImageOutputFormat::Jpeg => {
            let rgb = image.to_rgb8();
            let mut encoder = JpegEncoder::new_with_quality(&mut out, quality);
            encoder
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| ImageProcessingError::Encode(e.to_string()))?;
        }
        ImageOutputFormat::Png => {
            let rgba = image.to_rgba8();
            PngEncoder::new(&mut out)
                .write_image(
                    &rgba,
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ImageProcessingError::Encode(e.to_string()))?;
        }
        ImageOutputFormat::Webp => {
            let rgba = image.to_rgba8();
            WebPEncoder::new_lossless(&mut out)
                .encode(
                    &rgba,
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ImageProcessingError::Encode(e.to_string()))?;
        }
    }
    Ok(out)
}

fn mime_for_format(format: ImageOutputFormat) -> &'static str {
    match format {
        ImageOutputFormat::Original => "application/octet-stream",
        ImageOutputFormat::Jpeg => "image/jpeg",
        ImageOutputFormat::Png => "image/png",
        ImageOutputFormat::Webp => "image/webp",
    }
}

fn extension_for_format(format: ImageOutputFormat) -> &'static str {
    match format {
        ImageOutputFormat::Original => "bin",
        ImageOutputFormat::Jpeg => "jpg",
        ImageOutputFormat::Png => "png",
        ImageOutputFormat::Webp => "webp",
    }
}

#[allow(dead_code)]
fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}
