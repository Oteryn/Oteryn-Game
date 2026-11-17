//! T153/T154: only explicitly approved, displayed support evidence crosses this boundary.
pub const CAPABILITY_SUPPORT_INTAKE_V1: u32 = 462;
pub const CRASH: u32 = 6144;
pub const SURVEY: u32 = 6145;
pub const MAX_PREVIEW_BYTES: usize = 4096;
pub const MAX_REPLY_BYTES: usize = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSubmission;
pub fn decode_submission(payload: &[u8]) -> Result<&str, InvalidSubmission> {
    if !(2..=MAX_PREVIEW_BYTES + 1).contains(&payload.len()) || payload[0] != 1 {
        return Err(InvalidSubmission);
    }
    let text = std::str::from_utf8(&payload[1..]).map_err(|_| InvalidSubmission)?;
    if text.contains('\0') {
        return Err(InvalidSubmission);
    }
    Ok(text)
}
pub fn encode_submission(preview: &str, consent: bool) -> Result<Vec<u8>, InvalidSubmission> {
    if !consent || preview.is_empty() || preview.len() > MAX_PREVIEW_BYTES || preview.contains('\0')
    {
        return Err(InvalidSubmission);
    }
    let mut payload = Vec::with_capacity(preview.len() + 1);
    payload.push(1);
    payload.extend_from_slice(preview.as_bytes());
    Ok(payload)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn support_intake_requires_explicit_consent_and_bounded_exact_utf8_preview()
    -> Result<(), InvalidSubmission> {
        let text = "Reference: 1\nAnswer: Easy";
        assert_eq!(decode_submission(&encode_submission(text, true)?)?, text);
        assert!(encode_submission(text, false).is_err());
        for payload in [
            vec![],
            vec![1],
            vec![0, b'x'],
            vec![1, 0xff],
            vec![1, 0],
            vec![1; MAX_PREVIEW_BYTES + 2],
        ] {
            assert!(decode_submission(&payload).is_err());
        }
        assert!(encode_submission(&"x".repeat(MAX_PREVIEW_BYTES), true).is_ok());
        assert!(encode_submission(&"x".repeat(MAX_PREVIEW_BYTES + 1), true).is_err());
        Ok(())
    }
}
