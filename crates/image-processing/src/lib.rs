use base64::Engine as _;
use font8x8::{UnicodeFonts, BASIC_FONTS};
use image::{
    codecs::{avif::AvifEncoder, jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    imageops::{self, FilterType},
    DynamicImage, GenericImageView, ImageEncoder, Rgba, RgbaImage,
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
    pub thumbnails: Vec<ThumbnailOptions>,
    pub watermark: Option<WatermarkOptions>,
    pub url_transforms: UrlTransforms,
    pub preserve_original: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct UrlTransforms {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageOutputFormat {
    Original,
    Jpeg,
    Png,
    Webp,
    Avif,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkKind {
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    #[default]
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WatermarkOptions {
    pub enabled: bool,
    #[serde(rename = "type")]
    pub kind: WatermarkKind,
    pub text: Option<String>,
    pub image: Option<String>,
    pub position: WatermarkPosition,
    pub opacity: f32,
    pub margin: u32,
    pub scale: f32,
    pub width_percent: f32,
    pub color: String,
    pub apply_to_thumbnail: bool,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            kind: WatermarkKind::Text,
            text: None,
            image: None,
            position: WatermarkPosition::default(),
            opacity: 0.7,
            margin: 16,
            scale: 2.0,
            width_percent: 20.0,
            color: "ffffff".to_string(),
            apply_to_thumbnail: false,
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
    pub thumbnails: Vec<ThumbnailImage>,
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
    if source_mime == "image/avif" {
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
    validate_extra_thumbnails(&preset.thumbnails)?;
    if let Some(watermark) = &preset.watermark {
        validate_watermark(watermark)?;
    }

    let decoded =
        image::load_from_memory(bytes).map_err(|e| ImageProcessingError::Decode(e.to_string()))?;
    let (source_width, source_height) = decoded.dimensions();
    let resized = apply_resize(decoded, preset.resize.as_ref());

    let active_watermark = preset.watermark.as_ref().filter(|w| w.enabled);
    let watermarked = match active_watermark {
        Some(watermark) => Some(apply_watermark(&resized, watermark)?),
        None => None,
    };
    let output_image = watermarked.as_ref().unwrap_or(&resized);
    let (width, height) = output_image.dimensions();

    let output_format = choose_format(preset.format, source_mime, source_extension);
    let quality = preset.quality.unwrap_or(82).clamp(1, 100);
    let output_bytes = encode_image(output_image, output_format, quality)?;

    let thumbnail = match preset.thumbnail.as_ref().filter(|t| t.enabled) {
        Some(options) => Some(render_thumbnail(
            &resized,
            options,
            preset.format,
            preset.quality,
            source_mime,
            source_extension,
            active_watermark,
        )?),
        None => None,
    };

    let mut extra_thumbnails = Vec::new();
    for options in &preset.thumbnails {
        let duplicate = thumbnail.as_ref().is_some_and(|primary| {
            primary.width == options.width && primary.height == options.height
        }) || extra_thumbnails.iter().any(|existing: &ThumbnailImage| {
            existing.width == options.width && existing.height == options.height
        });
        if duplicate {
            continue;
        }
        extra_thumbnails.push(render_thumbnail(
            &resized,
            options,
            preset.format,
            preset.quality,
            source_mime,
            source_extension,
            active_watermark,
        )?);
    }

    let original = preset.preserve_original.then(|| OriginalImage {
        bytes: bytes.to_vec(),
        mime_type: source_mime.to_string(),
        extension: source_extension.to_string(),
        size: bytes.len() as u64,
    });

    let watermark_metadata = active_watermark.map(|watermark| {
        json!({
            "type": match watermark.kind {
                WatermarkKind::Text => "text",
                WatermarkKind::Image => "image",
            },
            "position": watermark.position,
            "opacity": watermark.opacity,
            "applyToThumbnail": watermark.apply_to_thumbnail
        })
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
                "thumbnail": thumbnail.as_ref().map(thumbnail_json),
                "thumbnails": thumbnail
                    .iter()
                    .chain(extra_thumbnails.iter())
                    .map(thumbnail_json)
                    .collect::<Vec<_>>(),
                "watermark": watermark_metadata,
                "preservedOriginal": preset.preserve_original
            }
        }),
        bytes: output_bytes,
        mime_type: mime_for_format(output_format).to_string(),
        extension: extension_for_format(output_format).to_string(),
        original,
        thumbnail,
        thumbnails: extra_thumbnails,
    }))
}

fn thumbnail_json(thumbnail: &ThumbnailImage) -> JsonValue {
    json!({
        "mimeType": thumbnail.mime_type,
        "extension": thumbnail.extension,
        "size": thumbnail.size,
        "width": thumbnail.width,
        "height": thumbnail.height
    })
}

fn render_thumbnail(
    source: &DynamicImage,
    options: &ThumbnailOptions,
    fallback_format: Option<ImageOutputFormat>,
    fallback_quality: Option<u8>,
    source_mime: &str,
    source_extension: &str,
    watermark: Option<&WatermarkOptions>,
) -> Result<ThumbnailImage, ImageProcessingError> {
    let mut thumb_image =
        source.resize_to_fill(options.width, options.height, FilterType::Lanczos3);
    if let Some(watermark) = watermark.filter(|w| w.apply_to_thumbnail) {
        if let Ok(watermarked) = apply_watermark_to_image(&thumb_image, watermark) {
            thumb_image = DynamicImage::ImageRgba8(watermarked);
        }
    }
    let format = choose_format(
        options.format.or(fallback_format),
        source_mime,
        source_extension,
    );
    let quality = options
        .quality
        .or(fallback_quality)
        .unwrap_or(82)
        .clamp(1, 100);
    let bytes = encode_image(&thumb_image, format, quality)?;
    Ok(ThumbnailImage {
        size: bytes.len() as u64,
        bytes,
        mime_type: mime_for_format(format).to_string(),
        extension: extension_for_format(format).to_string(),
        width: options.width,
        height: options.height,
    })
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
    validate_extra_thumbnails(&preset.thumbnails)?;
    if let Some(watermark) = &preset.watermark {
        validate_watermark(watermark)?;
    }
    Ok(())
}

const MAX_EXTRA_THUMBNAILS: usize = 5;
const MAX_THUMBNAIL_DIMENSION: u32 = 4096;

fn validate_extra_thumbnails(thumbnails: &[ThumbnailOptions]) -> Result<(), ImageProcessingError> {
    if thumbnails.len() > MAX_EXTRA_THUMBNAILS {
        return Err(ImageProcessingError::InvalidTransformations(format!(
            "at most {MAX_EXTRA_THUMBNAILS} additional thumbnails are supported"
        )));
    }
    for (index, thumbnail) in thumbnails.iter().enumerate() {
        if thumbnail.width == 0
            || thumbnail.height == 0
            || thumbnail.width > MAX_THUMBNAIL_DIMENSION
            || thumbnail.height > MAX_THUMBNAIL_DIMENSION
        {
            return Err(ImageProcessingError::InvalidTransformations(format!(
                "thumbnails[{index}] width and height must be between 1 and {MAX_THUMBNAIL_DIMENSION}"
            )));
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

fn validate_watermark(watermark: &WatermarkOptions) -> Result<(), ImageProcessingError> {
    if !watermark.enabled {
        return Ok(());
    }
    if !(0.0..=1.0).contains(&watermark.opacity) {
        return Err(ImageProcessingError::InvalidTransformations(
            "watermark opacity must be between 0 and 1".into(),
        ));
    }
    if watermark.margin > 5000 {
        return Err(ImageProcessingError::InvalidTransformations(
            "watermark margin must be 5000 pixels or less".into(),
        ));
    }
    match watermark.kind {
        WatermarkKind::Text => {
            let text = watermark.text.as_deref().unwrap_or_default().trim();
            if text.is_empty() {
                return Err(ImageProcessingError::InvalidTransformations(
                    "text watermark requires a non-empty text value".into(),
                ));
            }
            if !(0.1..=32.0).contains(&watermark.scale) {
                return Err(ImageProcessingError::InvalidTransformations(
                    "text watermark scale must be between 0.1 and 32".into(),
                ));
            }
            parse_hex_color(&watermark.color)?;
        }
        WatermarkKind::Image => {
            let data = watermark.image.as_deref().unwrap_or_default();
            if data.trim().is_empty() {
                return Err(ImageProcessingError::InvalidTransformations(
                    "image watermark requires an image value".into(),
                ));
            }
            if !(1.0..=100.0).contains(&watermark.width_percent) {
                return Err(ImageProcessingError::InvalidTransformations(
                    "image watermark width_percent must be between 1 and 100".into(),
                ));
            }
            decode_watermark_source(data)?;
        }
    }
    Ok(())
}

fn apply_watermark(
    base: &DynamicImage,
    watermark: &WatermarkOptions,
) -> Result<DynamicImage, ImageProcessingError> {
    apply_watermark_to_image(base, watermark).map(DynamicImage::ImageRgba8)
}

fn apply_watermark_to_image(
    base: &DynamicImage,
    watermark: &WatermarkOptions,
) -> Result<RgbaImage, ImageProcessingError> {
    let overlay = match watermark.kind {
        WatermarkKind::Text => render_text_watermark(watermark)?,
        WatermarkKind::Image => render_image_watermark(base, watermark)?,
    };
    if overlay.width() == 0 || overlay.height() == 0 {
        return Ok(base.to_rgba8());
    }

    let mut canvas = base.to_rgba8();
    let (canvas_width, canvas_height) = canvas.dimensions();
    let (overlay_width, overlay_height) = overlay.dimensions();
    let margin = watermark.margin as i64;
    let x = match watermark.position {
        WatermarkPosition::TopLeft
        | WatermarkPosition::CenterLeft
        | WatermarkPosition::BottomLeft => margin,
        WatermarkPosition::TopCenter
        | WatermarkPosition::Center
        | WatermarkPosition::BottomCenter => (canvas_width as i64 - overlay_width as i64) / 2,
        WatermarkPosition::TopRight
        | WatermarkPosition::CenterRight
        | WatermarkPosition::BottomRight => canvas_width as i64 - overlay_width as i64 - margin,
    };
    let y = match watermark.position {
        WatermarkPosition::TopLeft | WatermarkPosition::TopCenter | WatermarkPosition::TopRight => {
            margin
        }
        WatermarkPosition::CenterLeft
        | WatermarkPosition::Center
        | WatermarkPosition::CenterRight => (canvas_height as i64 - overlay_height as i64) / 2,
        WatermarkPosition::BottomLeft
        | WatermarkPosition::BottomCenter
        | WatermarkPosition::BottomRight => canvas_height as i64 - overlay_height as i64 - margin,
    };
    let x = x.clamp(0, (canvas_width.saturating_sub(1)) as i64);
    let y = y.clamp(0, (canvas_height.saturating_sub(1)) as i64);

    let mut overlay = overlay;
    for pixel in overlay.pixels_mut() {
        let alpha = f32::from(pixel[3]) * watermark.opacity;
        pixel[3] = alpha.round().clamp(0.0, 255.0) as u8;
    }

    imageops::overlay(&mut canvas, &overlay, x, y);
    Ok(canvas)
}

fn render_text_watermark(watermark: &WatermarkOptions) -> Result<RgbaImage, ImageProcessingError> {
    let text = watermark.text.as_deref().unwrap_or_default();
    let scale = watermark.scale.round().max(1.0) as u32;
    let color = parse_hex_color(&watermark.color)?;
    let glyph_width = 8 * scale;
    let glyph_height = 8 * scale;
    let characters = text.chars().count().max(1) as u32;
    let width = glyph_width * characters;
    let mut canvas = RgbaImage::new(width.max(1), glyph_height);

    for (index, character) in text.chars().enumerate() {
        let glyph = BASIC_FONTS.get(character).or_else(|| BASIC_FONTS.get('?'));
        let Some(glyph) = glyph else {
            continue;
        };
        let origin_x = index as u32 * glyph_width;
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..8 {
                if bits & (1 << column) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = origin_x + column * scale + dx;
                        let y = row as u32 * scale + dy;
                        if x < width && y < glyph_height {
                            canvas.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
                        }
                    }
                }
            }
        }
    }

    Ok(canvas)
}

fn render_image_watermark(
    base: &DynamicImage,
    watermark: &WatermarkOptions,
) -> Result<RgbaImage, ImageProcessingError> {
    let decoded = decode_watermark_source(watermark.image.as_deref().unwrap_or_default())?;
    let (base_width, _) = base.dimensions();
    let target_width = ((base_width as f32) * watermark.width_percent / 100.0).round() as u32;
    let target_width = target_width.clamp(1, base_width.max(1));
    let (source_width, source_height) = decoded.dimensions();
    if source_width == 0 || source_height == 0 {
        return Err(ImageProcessingError::InvalidTransformations(
            "watermark image has no pixels".into(),
        ));
    }
    let target_height = ((target_width as f64) * (source_height as f64) / (source_width as f64))
        .round()
        .max(1.0) as u32;
    Ok(decoded
        .resize_exact(target_width, target_height, FilterType::Lanczos3)
        .to_rgba8())
}

fn decode_watermark_source(source: &str) -> Result<DynamicImage, ImageProcessingError> {
    let encoded = source
        .split_once("base64,")
        .map(|(_, payload)| payload)
        .unwrap_or(source)
        .trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| {
            ImageProcessingError::InvalidTransformations(format!(
                "watermark image must be base64 encoded: {e}"
            ))
        })?;
    image::load_from_memory(&bytes).map_err(|e| {
        ImageProcessingError::InvalidTransformations(format!("watermark image is invalid: {e}"))
    })
}

fn parse_hex_color(value: &str) -> Result<[u8; 3], ImageProcessingError> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ImageProcessingError::InvalidTransformations(
            "watermark color must be a 6 digit hex value".into(),
        ));
    }
    let red = u8::from_str_radix(&value[0..2], 16).unwrap_or(0);
    let green = u8::from_str_radix(&value[2..4], 16).unwrap_or(0);
    let blue = u8::from_str_radix(&value[4..6], 16).unwrap_or(0);
    Ok([red, green, blue])
}

#[derive(Debug, Clone)]
pub struct TransformedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
}

pub fn transform_image(
    bytes: &[u8],
    source_mime: &str,
    source_extension: &str,
    transformations: &JsonValue,
) -> Result<Option<TransformedImage>, ImageProcessingError> {
    let preset: ImageProcessingPreset = serde_json::from_value(transformations.clone())
        .map_err(|e| ImageProcessingError::InvalidTransformations(e.to_string()))?;
    if !source_mime.starts_with("image/") || source_mime == "image/avif" {
        return Ok(None);
    }
    validate_dimensions(preset.resize.as_ref())?;
    if let Some(watermark) = &preset.watermark {
        validate_watermark(watermark)?;
    }

    let decoded =
        image::load_from_memory(bytes).map_err(|e| ImageProcessingError::Decode(e.to_string()))?;
    let resized = apply_resize(decoded, preset.resize.as_ref());
    let output = match preset.watermark.as_ref().filter(|w| w.enabled) {
        Some(watermark) => apply_watermark(&resized, watermark)?,
        None => resized,
    };
    let (width, height) = output.dimensions();
    let format = choose_format(preset.format, source_mime, source_extension);
    let quality = preset.quality.unwrap_or(82).clamp(1, 100);
    let bytes = encode_image(&output, format, quality)?;

    Ok(Some(TransformedImage {
        bytes,
        mime_type: mime_for_format(format).to_string(),
        extension: extension_for_format(format).to_string(),
        width,
        height,
    }))
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
            "image/avif" => ImageOutputFormat::Avif,
            _ => match source_extension {
                "jpg" | "jpeg" => ImageOutputFormat::Jpeg,
                "png" => ImageOutputFormat::Png,
                "webp" => ImageOutputFormat::Webp,
                "avif" => ImageOutputFormat::Avif,
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
        ImageOutputFormat::Avif => {
            let rgba = image.to_rgba8();
            let encoder = AvifEncoder::new_with_speed_quality(&mut out, 4, quality);
            encoder
                .write_image(
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
        ImageOutputFormat::Avif => "image/avif",
    }
}

fn extension_for_format(format: ImageOutputFormat) -> &'static str {
    match format {
        ImageOutputFormat::Original => "bin",
        ImageOutputFormat::Jpeg => "jpg",
        ImageOutputFormat::Png => "png",
        ImageOutputFormat::Webp => "webp",
        ImageOutputFormat::Avif => "avif",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, 120, 255])
        });
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(
                image.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .expect("encode sample png");
        out
    }

    #[test]
    fn encodes_avif_output() {
        let source = sample_png(48, 32);
        let result = process_image(
            &source,
            "image/png",
            "png",
            &json!({ "enabled": true, "format": "avif", "quality": 70 }),
        )
        .expect("process image")
        .expect("processing enabled");
        assert_eq!(result.mime_type, "image/avif");
        assert_eq!(result.extension, "avif");
        assert_eq!(&result.bytes[4..8], b"ftyp");
        assert_eq!(result.metadata["image"]["output"]["mimeType"], "image/avif");
    }

    #[test]
    fn applies_text_watermark() {
        let source = sample_png(80, 80);
        let result = process_image(
            &source,
            "image/png",
            "png",
            &json!({
                "enabled": true,
                "format": "png",
                "watermark": {
                    "enabled": true,
                    "type": "text",
                    "text": "FileBase",
                    "position": "bottom_right",
                    "opacity": 0.5,
                    "scale": 1.0
                }
            }),
        )
        .expect("process image")
        .expect("processing enabled");
        assert_eq!(result.metadata["image"]["watermark"]["type"], "text");
        assert_eq!(
            result.metadata["image"]["watermark"]["position"],
            "bottom_right"
        );
    }

    #[test]
    fn applies_image_watermark() {
        let source = sample_png(60, 60);
        let watermark = sample_png(16, 16);
        let encoded = base64::engine::general_purpose::STANDARD.encode(&watermark);
        let result = process_image(
            &source,
            "image/png",
            "png",
            &json!({
                "enabled": true,
                "format": "png",
                "watermark": {
                    "enabled": true,
                    "type": "image",
                    "image": format!("data:image/png;base64,{encoded}"),
                    "position": "center",
                    "width_percent": 25.0
                }
            }),
        )
        .expect("process image")
        .expect("processing enabled");
        assert_eq!(result.metadata["image"]["watermark"]["type"], "image");
    }

    #[test]
    fn generates_extra_thumbnail_sizes() {
        let source = sample_png(120, 80);
        let result = process_image(
            &source,
            "image/png",
            "png",
            &json!({
                "enabled": true,
                "format": "png",
                "thumbnail": { "enabled": true, "width": 32, "height": 32, "format": "png" },
                "thumbnails": [
                    { "width": 64, "height": 64, "format": "png" },
                    { "width": 32, "height": 32, "format": "png" }
                ]
            }),
        )
        .expect("process image")
        .expect("processing enabled");
        assert_eq!(result.thumbnails.len(), 1);
        assert_eq!(result.thumbnails[0].width, 64);
        let thumbnails = result.metadata["image"]["thumbnails"]
            .as_array()
            .expect("thumbnails metadata");
        assert_eq!(thumbnails.len(), 2);
    }

    #[test]
    fn transforms_on_demand() {
        let source = sample_png(100, 50);
        let result = transform_image(
            &source,
            "image/png",
            "png",
            &json!({
                "enabled": false,
                "format": "jpeg",
                "quality": 55,
                "resize": { "width": 40, "height": 20, "mode": "fill" }
            }),
        )
        .expect("transform image")
        .expect("image transformed");
        assert_eq!(result.mime_type, "image/jpeg");
        assert_eq!(result.extension, "jpg");
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 20);
    }

    #[test]
    fn rejects_too_many_extra_thumbnails() {
        let error = validate_transformations_json(&json!({
            "enabled": true,
            "thumbnails": [
                { "width": 10, "height": 10 },
                { "width": 11, "height": 11 },
                { "width": 12, "height": 12 },
                { "width": 13, "height": 13 },
                { "width": 14, "height": 14 },
                { "width": 15, "height": 15 }
            ]
        }))
        .expect_err("too many thumbnails rejected");
        assert!(error.to_string().contains("at most"));
    }

    #[test]
    fn rejects_empty_text_watermark() {
        let error = validate_transformations_json(&json!({
            "enabled": true,
            "watermark": { "enabled": true, "type": "text", "text": "  " }
        }))
        .expect_err("invalid watermark rejected");
        assert!(error.to_string().contains("non-empty"));
    }
}
