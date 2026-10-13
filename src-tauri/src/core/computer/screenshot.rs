use super::*;

/// Long edge we downscale screenshots to before sending to the model. Keeps the
/// image inside the range vision models handle well and makes coordinates stable.
pub(super) const TARGET_LONG_EDGE: u32 = 1280;

/// A screenshot ready for the model, plus the mapping back to the real screen.
pub struct Shot {
    pub base64: String,
    /// Dimensions of the (downscaled) image the model sees.
    pub width: u32,
    pub height: u32,
    /// Logical screen size in points (what input events use).
    pub screen_w: i32,
    pub screen_h: i32,
}

impl Shot {
    /// Map a model coordinate (in image pixels) to a logical screen point.
    pub fn to_screen(&self, x: f64, y: f64) -> (i32, i32) {
        let sx = self.screen_w as f64 / self.width.max(1) as f64;
        let sy = self.screen_h as f64 / self.height.max(1) as f64;
        ((x * sx).round() as i32, (y * sy).round() as i32)
    }
}

/// One action the agent asked us to perform. `kind` mirrors Anthropic's computer
/// tool actions we support; coordinates are in the screenshot's pixel space.
#[derive(Debug, Deserialize)]
pub struct Action {
    pub kind: String,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub scroll_direction: Option<String>,
    #[serde(default)]
    pub scroll_amount: Option<i32>,
}

/// Has the user granted us Accessibility control? On non-macOS we assume yes.
pub fn accessibility_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_client::accessibility::application_is_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Pop the system prompt asking the user to grant Accessibility (macOS).
pub fn prompt_accessibility() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Logical screen size in points.
pub(super) fn screen_size() -> Result<(i32, i32), String> {
    let enigo = Enigo::new(&EnigoSettings::default()).map_err(|e| format!("input init failed: {e}"))?;
    enigo.main_display().map_err(|e| format!("display query failed: {e}"))
}

/// Capture the screen, downscale, and return it as a model-ready PNG.
pub fn screenshot() -> Result<Shot, String> {
    let (screen_w, screen_h) = screen_size()?;

    let raw = capture_png()?;
    let img = image::load_from_memory(&raw).map_err(|e| format!("decode screenshot: {e}"))?;
    let (w0, h0) = (img.width(), img.height());

    // downscale so the long edge ≤ TARGET_LONG_EDGE (never upscale)
    let long = w0.max(h0);
    let scaled = if long > TARGET_LONG_EDGE {
        let f = TARGET_LONG_EDGE as f32 / long as f32;
        img.resize(
            (w0 as f32 * f) as u32,
            (h0 as f32 * f) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let (width, height) = (scaled.width(), scaled.height());

    let mut buf = std::io::Cursor::new(Vec::new());
    scaled
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("encode screenshot: {e}"))?;
    let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.get_ref());

    Ok(Shot { base64, width, height, screen_w, screen_h })
}

/// Grab the full screen to PNG bytes. macOS uses the built-in `screencapture`.
pub(super) fn capture_png() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let tmp = std::env::temp_dir().join(format!("bba-cu-{stamp}.png"));
        let status = Command::new("screencapture")
            .args(["-x", "-C"]) // -x: no sound, -C: capture cursor
            .arg(&tmp)
            .status()
            .map_err(|e| format!("screencapture failed: {e}"))?;
        if !status.success() || !tmp.exists() {
            return Err("screencapture produced no image".into());
        }
        let bytes = std::fs::read(&tmp).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp);
        Ok(bytes)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Screen capture is only available on macOS right now.".into())
    }
}
