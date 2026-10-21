//! Lexical bounds used by progression input models.
const MAX_REVISION_BYTES: usize = 128;

pub(crate) fn valid_revision(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= MAX_REVISION_BYTES
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}
