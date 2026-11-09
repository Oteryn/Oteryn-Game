//! Shared exact device selection used by the native output builder.
pub fn select_audio_device(requested: &str, names: &[String]) -> Result<Option<usize>, String> {
    if requested.is_empty() {
        return Ok(None);
    }
    names
        .iter()
        .position(|name| name == requested)
        .map(Some)
        .ok_or_else(|| format!("Audio output device is unavailable: {requested}"))
}
