//! Audit-only positive corpus generator in a disposable integration-test target.
#[test]
fn create_actual_positive_content_corpus()->Result<(),Box<dyn std::error::Error>> {
    use oteryn_game_server::content::{compile,synthetic_vsl_fixture,CompileTarget,EvidenceLimits,StagedArtifact};
    let limits=EvidenceLimits::new("evidence:audit-fuzz",1_048_576,16,524_288,4096,4096,256,512,4096,4096,8192)?;
    let graph=synthetic_vsl_fixture(&limits)?;let artifacts=compile(&graph,&limits,CompileTarget::Evidence)?;
    let directory=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus/content");
    std::fs::create_dir_all(&directory)?;
    for (name,bytes) in [("server",artifacts.server_artifact),("client",artifacts.client_artifact)] {
        StagedArtifact::stage(&bytes,&limits)?;std::fs::write(directory.join(name),bytes)?;
    }
    Ok(())
}
