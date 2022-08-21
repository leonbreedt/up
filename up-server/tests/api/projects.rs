use crate::TestApp;
use std::time::Duration;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn create_project() {
    let test_app = TestApp::new().await;

    // TODO: make API calls with real JWTs

    std::thread::sleep(Duration::from_secs(10));
}
