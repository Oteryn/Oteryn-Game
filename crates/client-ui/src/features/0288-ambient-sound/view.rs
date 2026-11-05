pub fn status(playing: bool) -> &'static str {
    if playing {
        "ambience on"
    } else {
        "ambience off"
    }
}
