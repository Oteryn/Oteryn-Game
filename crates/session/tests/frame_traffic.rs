//! Its own test binary: the counters are process-wide, so no other test may run beside this one.
use oteryn_session::{FrameCount, frame_traffic, read_frame, write_frame};

#[test]
fn one_frame_each_way_is_counted_once() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        assert_eq!(
            frame_traffic(),
            (FrameCount::default(), FrameCount::default())
        );
        let (mut client, mut server) = tokio::io::duplex(64);
        write_frame(&mut client, b"hello").await?;
        assert_eq!(read_frame(&mut server).await?, b"hello");
        // Both ends live in this process: the frame is one write and one read of 4 + 5 bytes.
        let one = FrameCount {
            frames: 1,
            bytes: 9,
        };
        assert_eq!(frame_traffic(), (one, one));
        write_frame(&mut server, b"ok!").await?;
        assert_eq!(read_frame(&mut client).await?, b"ok!");
        let two = FrameCount {
            frames: 2,
            bytes: 16,
        };
        assert_eq!(frame_traffic(), (two, two));
        Ok(())
    })
}
