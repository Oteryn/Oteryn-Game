use super::*;

#[test]
fn one_request_per_think_is_accepted_and_a_second_is_refused() {
    let route = [RouteStep::new(1, 1)];
    assert!(build_path_proposal(PathRequest::new(provenance(), 1, 1, 1), &route).is_ok());
    assert_eq!(
        build_path_proposal(PathRequest::new(provenance(), 1, 2, 1), &route),
        Err(AiError::BootstrapPathRequestLimit)
    );
}
