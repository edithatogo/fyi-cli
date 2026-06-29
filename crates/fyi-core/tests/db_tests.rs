use fyi_core::api::{AlaveteliCorrespondence, AlaveteliRequest, CorrespondenceDirection};
use fyi_core::db::{DbPool, SyncStatus};

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

#[tokio::test]
async fn test_insert_request_marks_dirty_and_tracks_fields() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let request = AlaveteliRequest {
        id: 10,
        title: "Initial title".to_string(),
        body: "Initial body".to_string(),
        user_name: Some("Jane".to_string()),
        status: Some("draft".to_string()),
        created_at: Some("2026-06-30T00:00:00Z".to_string()),
        updated_at: Some("2026-06-30T00:00:00Z".to_string()),
        url: Some("https://fyi.org.nz/request/10".to_string()),
        tags: Some(vec!["oia".to_string()]),
    };

    db.insert_request(&request)
        .await
        .expect("Failed to insert request");

    let metadata = db
        .get_request_sync_metadata(10)
        .await
        .expect("Failed to read sync metadata")
        .expect("metadata should be present");

    assert_eq!(metadata.sync_status, SyncStatus::Dirty);
    assert_eq!(metadata.conflict_version, 0);

    let changes = db
        .list_unsynced_field_changes(10)
        .await
        .expect("Failed to list field changes");
    let fields = changes
        .iter()
        .map(|change| change.field_name.as_str())
        .collect::<Vec<_>>();

    assert!(fields.contains(&"title"));
    assert!(fields.contains(&"body"));
    assert!(fields.contains(&"status"));
}

#[tokio::test]
async fn test_update_request_tracks_only_changed_fields() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let mut request = AlaveteliRequest {
        id: 11,
        title: "Original title".to_string(),
        body: "Original body".to_string(),
        user_name: None,
        status: Some("draft".to_string()),
        created_at: Some("2026-06-30T00:00:00Z".to_string()),
        updated_at: Some("2026-06-30T00:00:00Z".to_string()),
        url: None,
        tags: None,
    };

    db.upsert_synced_request(&request, request.updated_at.as_deref())
        .await
        .expect("Failed to insert clean request");

    request.title = "Updated title".to_string();
    request.status = Some("waiting_response".to_string());
    request.updated_at = Some("2026-06-30T01:00:00Z".to_string());

    let changed = db
        .update_request(&request)
        .await
        .expect("Failed to update request");
    assert!(changed);

    let changes = db
        .list_unsynced_field_changes(11)
        .await
        .expect("Failed to list field changes");
    let fields = changes
        .iter()
        .map(|change| change.field_name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(changes.len(), 3);
    assert!(fields.contains(&"title"));
    assert!(fields.contains(&"status"));
    assert!(fields.contains(&"updated_at"));
}

#[tokio::test]
async fn test_mark_request_clean_updates_global_sync_status() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let request = AlaveteliRequest {
        id: 12,
        title: "Cleanable request".to_string(),
        body: "Body".to_string(),
        user_name: None,
        status: None,
        created_at: None,
        updated_at: Some("2026-06-30T00:00:00Z".to_string()),
        url: None,
        tags: None,
    };

    db.insert_request(&request)
        .await
        .expect("Failed to insert request");
    assert_eq!(
        db.get_global_sync_status()
            .await
            .expect("Failed to get global sync status")
            .dirty,
        1
    );

    db.mark_request_clean(12, request.updated_at.as_deref())
        .await
        .expect("Failed to mark request clean");

    let metadata = db
        .get_request_sync_metadata(12)
        .await
        .expect("Failed to read sync metadata")
        .expect("metadata should be present");
    let global = db
        .get_global_sync_status()
        .await
        .expect("Failed to get global sync status");
    let changes = db
        .list_unsynced_field_changes(12)
        .await
        .expect("Failed to list field changes");

    assert_eq!(metadata.sync_status, SyncStatus::Clean);
    assert_eq!(metadata.remote_request_id, None);
    assert_eq!(global.total, 1);
    assert_eq!(global.clean, 1);
    assert!(changes.is_empty());
}

#[tokio::test]
async fn test_outgoing_queue_records_pending_and_confirmed_submissions() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let request = AlaveteliRequest {
        id: 13,
        title: "Queued request".to_string(),
        body: "Body".to_string(),
        user_name: None,
        status: Some("draft".to_string()),
        created_at: None,
        updated_at: Some("2026-06-30T00:00:00Z".to_string()),
        url: None,
        tags: None,
    };

    db.insert_request(&request)
        .await
        .expect("Failed to insert request");
    let queue_id = db
        .enqueue_request_submission(&request)
        .await
        .expect("Failed to enqueue request");
    let pending = db
        .list_pending_outgoing_queue(10)
        .await
        .expect("Failed to list pending queue");

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, queue_id);
    assert_eq!(pending[0].request_id, 13);

    db.mark_submission_confirmed(queue_id, 13, 1300, request.updated_at.as_deref())
        .await
        .expect("Failed to confirm submission");

    let metadata = db
        .get_request_sync_metadata(13)
        .await
        .expect("Failed to read sync metadata")
        .expect("metadata should be present");
    let pending = db
        .list_pending_outgoing_queue(10)
        .await
        .expect("Failed to list pending queue");

    assert_eq!(metadata.remote_request_id, Some(1300));
    assert_eq!(metadata.sync_status, SyncStatus::Clean);
    assert!(pending.is_empty());
}

#[tokio::test]
async fn test_outgoing_queue_depth_includes_failed_items() {
    let db = DbPool::new_in_memory()
        .await
        .expect("Failed to initialize in-memory DB");
    db.run_migrations().await.expect("Failed to run migrations");

    let request = AlaveteliRequest {
        id: 14,
        title: "Failing request".to_string(),
        body: "Body".to_string(),
        user_name: None,
        status: Some("draft".to_string()),
        created_at: None,
        updated_at: None,
        url: None,
        tags: None,
    };

    db.insert_request(&request)
        .await
        .expect("Failed to insert request");
    let queue_id = db
        .enqueue_request_submission(&request)
        .await
        .expect("Failed to enqueue request");
    db.mark_submission_failed(queue_id, 14, 3, "boom")
        .await
        .expect("Failed to mark queue item failed");

    let depth = db
        .get_outgoing_queue_depth()
        .await
        .expect("Failed to read queue depth");
    let metadata = db
        .get_request_sync_metadata(14)
        .await
        .expect("Failed to read sync metadata")
        .expect("metadata should be present");

    assert_eq!(depth.pending, 0);
    assert_eq!(depth.failed, 1);
    assert_eq!(metadata.sync_status, SyncStatus::Dirty);
}
