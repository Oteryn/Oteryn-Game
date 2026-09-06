//! Both raw and re-signed structural fuzzing of the unchanged evidence parser.
//! Re-signing is a test-input transformation, not a production integrity bypass.
#![no_main]
use libfuzzer_sys::fuzz_target;
use oteryn_game_server::content::{EvidenceLimits,StagedArtifact};
use sha2::{Digest,Sha256};
use std::sync::OnceLock;
fn u32_at(bytes:&[u8],index:usize)->Option<usize> {
    Some(u32::from_be_bytes(bytes.get(index..index.checked_add(4)?)?.try_into().ok()?) as usize)
}
fuzz_target!(|data: &[u8]| {
    static LIMITS:OnceLock<EvidenceLimits>=OnceLock::new();
    let limits=LIMITS.get_or_init(||EvidenceLimits::new("evidence:audit-fuzz",1_048_576,16,524_288,4096,4096,256,512,4096,4096,8192).expect("fixed audit limits valid"));
    let _=StagedArtifact::stage(data,limits);
    if data.len()<56 || data.len()>1_048_576 {return;}
    let mut candidate=data.to_vec();let end=candidate.len()-32;
    candidate[20..24].copy_from_slice(&(end as u32).to_be_bytes());
    let count=u16::from_be_bytes([candidate[14],candidate[15]]) as usize;
    if count<=16 {
        for index in 0..count {
            let table=24+index*48;
            if table+48>end {break;}
            if let (Some(start),Some(size))=(u32_at(&candidate,table+4),u32_at(&candidate,table+8)) {
                if let Some(stop)=start.checked_add(size) {
                    if start>=24+count*48 && stop<=end {
                        let digest=Sha256::digest(&candidate[start..stop]);
                        candidate[table+16..table+48].copy_from_slice(&digest);
                    }
                }
            }
        }
    }
    let digest=Sha256::digest(&candidate[..end]);candidate[end..].copy_from_slice(&digest);
    let _=StagedArtifact::stage(&candidate,limits);
});
