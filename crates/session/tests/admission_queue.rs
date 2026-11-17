use oteryn_session::{
    Admission, AdmissionQueue, CharacterId, Session, SessionError, admission_queue, read_frame,
    write_frame,
};
use std::time::Duration;
#[test]
fn server_queue_metadata_survives_the_real_session_admission_reader()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| -> Result<(), String> {
        let run = || -> Result<(), Box<dyn std::error::Error>> {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
            runtime.block_on(async {
                let (client, mut server) = tokio::io::duplex(4096);
                let queue = AdmissionQueue { position: 7, retry_after_ms: 2000 };
                let frame = admission_queue::encode_queue_refusal(queue)?;
                let peer = tokio::spawn(async move {
                    let _bootstrap = read_frame(&mut server).await?;
                    write_frame(&mut server, &frame).await?;
                    Ok::<_, SessionError>(())
                });
                let result = Session::admit(client, Admission {
                    appearance_items: None, schema_revision: 1, character_id: CharacterId::decode(&[0x01,0x89,0x0a,0x5d,0xac,0x96,0x77,0x4b,0xbc,0xce,0xb3,0x02,0x09,0x9a,0x80,0x57])?,
                    admission_material: b"fixture-grant", client_build_id: "oteryn-client/test", supported_capabilities: &[], deadline: Duration::from_secs(1),
                }).await;
                peer.await??;
                assert!(matches!(result, Err(SessionError::AdmissionQueued { code:1115, queue: observed }) if observed == queue));
                Ok::<_, Box<dyn std::error::Error>>(())
            })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "admission queue scenario thread failed")?.map_err(Into::into)
}
