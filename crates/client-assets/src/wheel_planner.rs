//! Version 1 of the Tibia Wheel planner packs 36 slice values and 24 gem bytes
//! in a single raw DEFLATE stream. Slot order is the native allocation order.
pub fn decode_allocation(compressed: &[u8]) -> Result<[u8; 36], &'static str> {
    let mut bytes = [0; 61];
    let mut decoder = flate2::Decompress::new(false);
    let status = decoder
        .decompress(compressed, &mut bytes, flate2::FlushDecompress::Finish)
        .map_err(|_| "Invalid Wheel code")?;
    if status != flate2::Status::StreamEnd
        || decoder.total_out() != 60
        || decoder.total_in() != compressed.len() as u64
    {
        return Err("Invalid Wheel code length");
    }
    if bytes[36..60].iter().any(|byte| *byte != 255) {
        return Err("This build contains Tibia gem identities unavailable on this server");
    }
    bytes[..36]
        .try_into()
        .map_err(|_| "Invalid Wheel code length")
}
