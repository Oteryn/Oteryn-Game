/// Minimal FND-02 accepted + core join snapshot fixture. Like the refusal fixture below,
/// this is test-only wire data; the client still decodes it through the production session.
pub(crate) fn admitted_frames() -> Vec<Vec<u8>> {
    fn bytes(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut out = vec![tag, value.len() as u8];
        out.extend_from_slice(value);
        out
    }
    let id = [
        1, 147, 79, 16, 124, 0, 112, 1, 128, 91, 59, 17, 34, 51, 68, 1,
    ];
    let mut accepted = bytes(10, &id);
    accepted.extend(bytes(18, &id));
    accepted.extend(bytes(26, &id));
    accepted.extend([32, 1, 40, 0, 48, 1, 56, 1, 64, 1, 72, 1]);
    let mut accepted_frame = vec![8, 2];
    accepted_frame.extend(bytes(34, &accepted));
    let mut spatial = bytes(10, &[17; 32]);
    spatial.extend(bytes(18, &[8, 0, 16, 0, 24, 0]));
    let mut domain = vec![8, 1, 16, 1, 24, 1];
    domain.extend(bytes(34, &spatial));
    let mut body = bytes(10, &domain);
    body.extend(bytes(10, &[8, 2, 16, 1, 24, 1, 34, 0]));
    let begin = vec![8, 1, 16, 1, 24, body.len() as u8, 32, 0];
    let mut chunk = vec![8, 1];
    chunk.extend(bytes(26, &body));
    let mut frames = vec![accepted_frame];
    for (kind, payload) in [(11, begin), (12, chunk), (13, vec![8, 1])] {
        let mut frame = vec![8, kind, 16, 1];
        frame.extend(bytes(34, &payload));
        frames.push(frame);
    }
    frames
}
