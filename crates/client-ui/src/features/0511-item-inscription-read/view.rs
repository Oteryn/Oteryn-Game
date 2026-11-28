use super::Known;

/// The attribution line accompanies the selected item's text in the writing window.
pub fn attribution_text(known: &Known, selected_handle: u64) -> Option<String> {
    let Known::Read {
        handle,
        attribution: Some(attribution),
        ..
    } = known
    else {
        return None;
    };
    if *handle != selected_handle {
        return None;
    }
    let date = crate::house::format_ownership_date(attribution.written_at_unix_ms);
    Some(match &attribution.writer_name {
        Some(writer) => format!("{writer} · {date}"),
        None => date,
    })
}
