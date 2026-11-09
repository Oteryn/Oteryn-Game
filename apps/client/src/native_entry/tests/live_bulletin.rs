use super::test_support::{Stack, admitted_frames};
use crate::{ClientBootstrap, bulletin::Browser, reentry::Reentry};
use std::time::{Duration, Instant};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const DOCUMENT: &str = r#"{"news":[{"title":"Operator notice","body":"Published after the client release"}],"events":[{"year":2027,"month":1,"day":12,"title":"Winter gathering","body":"Meet at the town square"}]}"#;
#[test]
fn live_operator_bulletin_crosses_gateway_tls_session_and_calendar_caller() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let mut frames = admitted_frames();
                frames[0] =
                    oteryn_protocol_oteryn::bulletin::append_to_acceptance(&frames[0], DOCUMENT)?;
                let stack = Stack::start_with_frames(vec![], frames)?;
                let client = ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = Reentry::new(client);
                assert!(entry.request());
                let deadline = Instant::now() + Duration::from_secs(10);
                loop {
                    if let Some(result) = entry.poll() {
                        let _play = result?;
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("bulletin admission timed out".into());
                    }
                    std::thread::yield_now();
                }
                let document = entry.take_bulletin().ok_or("server bulletin was lost")?;
                let mut browser = Browser::new(2027, 1, r#"{"news":[],"events":[]}"#)?;
                browser.replace(&document)?;
                assert!(
                    browser
                        .news()
                        .contains("Published after the client release")
                );
                assert!(browser.month().contains("12: Winter gathering"));
                assert!(browser.month().contains("Meet at the town square"));
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "live bulletin scenario failed")?
        .map_err(Into::into)
}
