use fyi_core::api::{AlaveteliCorrespondence, AlaveteliRequest, CorrespondenceDirection};
use fyi_core::db::DbPool;

#[tokio::test]
async fn test_db_pool_and_migrations() {
    // 1. Initialize an in-memory SQLite database
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");

    // 2. Run migrations
    db.run_migrations().await.expect("Failed to run migrations");

    // 3. Insert and verify AlaveteliRequest
    let request = AlaveteliRequest {
        id: 1,
        title: "Test OIA Request".to_string(),
        body: "Please provide the information.".to_string(),
        user_name: Some("Jane Doe".to_string()),
        status: Some("waiting_response".to_string()),
        created_at: Some("2026-06-15T00:00:00Z".to_string()),
        updated_at: Some("2026-06-15T01:00:00Z".to_string()),
        url: Some("https://fyi.org.nz/request/1".to_string()),
        tags: Some(vec!["test".to_string(), "oia".to_string()]),
    };

    db.insert_request(&request)
        .await
        .expect("Failed to insert request");

    let retrieved = db
        .get_request(1)
        .await
        .expect("Failed to query request")
        .expect("Request should exist");

    assert_eq!(retrieved, request);

    // 4. Insert AlaveteliCorrespondence and verify
    let corr1 = AlaveteliCorrespondence {
        direction: CorrespondenceDirection::Request,
        body: "First message body".to_string(),
        sent_at: "2026-06-15T00:01:00Z".to_string(),
        state: Some("sent".to_string()),
        attachments: Some(vec!["doc1.pdf".to_string()]),
    };

    let corr2 = AlaveteliCorrespondence {
        direction: CorrespondenceDirection::Response,
        body: "Response message body".to_string(),
        sent_at: "2026-06-15T00:02:00Z".to_string(),
        state: Some("received".to_string()),
        attachments: None,
    };

    db.insert_correspondence(1, &corr1)
        .await
        .expect("Failed to insert correspondence 1");
    db.insert_correspondence(1, &corr2)
        .await
        .expect("Failed to insert correspondence 2");

    let correspondences = db
        .get_correspondence_for_request(1)
        .await
        .expect("Failed to retrieve correspondences");

    assert_eq!(correspondences.len(), 2);
    assert_eq!(correspondences[0], corr1);
    assert_eq!(correspondences[1], corr2);
}

#[tokio::test]
async fn test_request_not_found() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let result = db.get_request(999).await.expect("Query failed");
    assert!(result.is_none());

    let corrs = db
        .get_correspondence_for_request(999)
        .await
        .expect("Query failed");
    assert!(corrs.is_empty());
}
