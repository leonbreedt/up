use crate::{TestApp, TestError, TestUser};
use up_server::api::v1::projects::{CreateProject, Project};

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn viewer_can_list_projects_they_are_assigned_to() {
    let (_app, client) = TestApp::start_and_connect(TestUser::Viewer).await;

    let projects: Vec<Project> = client
        .get("/api/v1/projects")
        .await
        .expect("failed to list projects");

    assert_eq!(1, projects.len());
    assert_eq!("3S4F26A88N9PPAWD1PYDDFDR04", projects[0].id.to_string());
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn viewer_cant_create_projects() {
    let (_app, client) = TestApp::start_and_connect(TestUser::Viewer).await;

    let request = CreateProject {
        account_id: "09WDY5H2KX9V6RSV6VC8T01AJC".parse().unwrap(),
        name: "test project".to_string(),
    };

    let result = client
        .post::<CreateProject, Project>("/api/v1/projects", request)
        .await;
    if let Err(TestError::RequestError(e)) = result {
        assert_eq!(403, e.status().unwrap().as_u16());
    } else {
        panic!("expected viewer not to be able to create a project");
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn member_cant_create_projects() {
    let (_app, client) = TestApp::start_and_connect(TestUser::Member).await;

    let request = CreateProject {
        account_id: "09WDY5H2KX9V6RSV6VC8T01AJC".parse().unwrap(),
        name: "test project".to_string(),
    };

    let result = client
        .post::<CreateProject, Project>("/api/v1/projects", request)
        .await;
    if let Err(TestError::RequestError(e)) = result {
        assert_eq!(403, e.status().unwrap().as_u16());
    } else {
        panic!("expected member not to be able to create a project");
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn administrator_can_create_projects() {
    let (_app, client) = TestApp::start_and_connect(TestUser::Administrator).await;

    let request = CreateProject {
        account_id: "09WDY5H2KX9V6RSV6VC8T01AJC".parse().unwrap(),
        name: "test project".to_string(),
    };

    let project: Project = client.post("/api/v1/projects", request).await.unwrap();

    assert_eq!("test project", project.name);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub async fn viewer_cant_see_projects_in_accounts_not_assigned_to() {
    let (_app, client) = TestApp::start_and_connect(TestUser::NoAccountViewer).await;

    let projects: Vec<Project> = client
        .get("/api/v1/projects")
        .await
        .expect("failed to list projects");

    assert_eq!(0, projects.len());
}
