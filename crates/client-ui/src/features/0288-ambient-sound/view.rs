pub fn status(playing: bool) -> &'static str {
    if playing {
        tr!("ambience on")
    } else {
        tr!("ambience off")
    }
}
