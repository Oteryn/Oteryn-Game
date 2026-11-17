use super::*;
#[test]
fn gap_world_channel_list() -> Result<(), oteryn_session::SessionError> {
    for premium in [false, true] {
        let mut bytes = vec![10, 38, 10, 16];
        bytes.extend([1; 16]);
        bytes.extend([18, 16]);
        bytes.extend([2; 16]);
        bytes.extend([24, u8::from(premium)]);
        assert_eq!(
            decode(&bytes)?,
            ServedChannel {
                world_id: [1; 16],
                channel_id: [2; 16],
                premium_only: premium
            }
        );
        // Fields may arrive in a different protobuf order.
        let reordered = [&bytes[..2], &bytes[38..], &bytes[20..38], &bytes[2..20]].concat();
        assert_eq!(decode(&reordered)?, decode(&bytes)?);
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
        let mut duplicate = bytes.clone();
        duplicate[20] = 10;
        assert!(decode(&duplicate).is_err());
        bytes.extend([24, 1]);
        assert!(decode(&bytes).is_err());
    }
    Ok(())
}
