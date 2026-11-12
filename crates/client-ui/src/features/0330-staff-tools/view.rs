pub fn panel_text(summary: &str, feedback: &str) -> String {
    lfmt!(
        "Staff tools\n{summary}\n{feedback}",
        feedback = feedback,
        summary = summary
    )
}
