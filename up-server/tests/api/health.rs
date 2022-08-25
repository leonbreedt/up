use crate::{TestApp, TestUser};

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn health_check() {
    let (_, client) = TestApp::start_and_connect(TestUser::Anonymous).await;

    let response = client
        .get_string("/health")
        .await
        .expect("failed to perform health check");

    assert_eq!("UP", response);
}
