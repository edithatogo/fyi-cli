use serde::{Deserialize, Serialize};

/// Represents the direction of correspondence (message) on a request.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CorrespondenceDirection {
    Request,
    Response,
}

/// Represents an Alaveteli request object (Read/Write API contract).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlaveteliRequest {
    pub id: i64,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Represents correspondence (message) on a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlaveteliCorrespondence {
    pub direction: CorrespondenceDirection,
    pub body: String,
    pub sent_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
}

/// Payload for creating a new request (Write API).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateRequestPayload {
    pub title: String,
    pub body: String,
    pub external_user_name: String,
    pub external_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

/// Response returned after successfully creating a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateRequestResponse {
    pub id: i64,
    pub url: String,
}

/// Payload for adding correspondence to an existing request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddCorrespondencePayload {
    pub direction: CorrespondenceDirection,
    pub body: String,
    pub sent_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// General action response for adding correspondence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrespondenceResponse {
    pub success: bool,
}

/// Payload for updating a request's state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateRequestStatePayload {
    pub state: String,
}

/// Response returned after updating a request's state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateRequestStateResponse {
    pub updated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_request_serialization_roundtrip() {
        let original = AlaveteliRequest {
            id: 42,
            title: "Information Request".to_string(),
            body: "Give details".to_string(),
            user_name: Some("Alice".to_string()),
            status: Some("successful".to_string()),
            created_at: Some("2026-06-15T00:00:00Z".to_string()),
            updated_at: Some("2026-06-15T01:00:00Z".to_string()),
            url: Some("https://fyi.org.nz/request/42".to_string()),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        };

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AlaveteliRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_request_deserialization_with_missing_fields() {
        let json_data = json!({
            "id": 101,
            "title": "Minimal Request",
            "body": "No extras here"
        });

        let request: AlaveteliRequest = serde_json::from_value(json_data).unwrap();
        assert_eq!(request.id, 101);
        assert_eq!(request.title, "Minimal Request");
        assert_eq!(request.body, "No extras here");
        assert!(request.user_name.is_none());
        assert!(request.status.is_none());
        assert!(request.created_at.is_none());
        assert!(request.tags.is_none());
    }

    #[test]
    fn test_direction_serialization() {
        let dir_req = CorrespondenceDirection::Request;
        let dir_res = CorrespondenceDirection::Response;

        assert_eq!(serde_json::to_value(dir_req).unwrap(), json!("request"));
        assert_eq!(serde_json::to_value(dir_res).unwrap(), json!("response"));

        let dec_req: CorrespondenceDirection = serde_json::from_value(json!("request")).unwrap();
        let dec_res: CorrespondenceDirection = serde_json::from_value(json!("response")).unwrap();

        assert_eq!(dec_req, CorrespondenceDirection::Request);
        assert_eq!(dec_res, CorrespondenceDirection::Response);
    }

    #[test]
    fn test_correspondence_serialization_roundtrip() {
        let corr = AlaveteliCorrespondence {
            direction: CorrespondenceDirection::Response,
            body: "This is the response from the ministry.".to_string(),
            sent_at: "2026-06-15T00:05:00Z".to_string(),
            state: Some("successful".to_string()),
            attachments: Some(vec!["doc1.pdf".to_string()]),
        };

        let serialized = serde_json::to_string(&corr).unwrap();
        let deserialized: AlaveteliCorrespondence = serde_json::from_str(&serialized).unwrap();
        assert_eq!(corr, deserialized);
    }

    #[tokio::test]
    async fn test_wiremock_get_request() {
        let mock_server = MockServer::start().await;

        let mock_request = AlaveteliRequest {
            id: 999,
            title: "Mocked Request".to_string(),
            body: "Testing with wiremock".to_string(),
            user_name: Some("Testy McTest".to_string()),
            status: Some("waiting_response".to_string()),
            created_at: Some("2026-06-15T00:10:00Z".to_string()),
            updated_at: None,
            url: Some("https://example.com/request/999".to_string()),
            tags: None,
        };

        Mock::given(method("GET"))
            .and(path("/api/v2/request/999.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&mock_request))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/v2/request/999.json", mock_server.uri()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let request: AlaveteliRequest = response.json().await.unwrap();
        assert_eq!(request, mock_request);
    }

    #[tokio::test]
    async fn test_wiremock_create_request() {
        let mock_server = MockServer::start().await;

        let payload = CreateRequestPayload {
            title: "OIA Request".to_string(),
            body: "Give details".to_string(),
            external_user_name: "John".to_string(),
            external_url: "https://mycopy.com".to_string(),
            tags: Some("oia test".to_string()),
        };

        let expected_response = CreateRequestResponse {
            id: 123,
            url: "https://fyi.org.nz/request/123".to_string(),
        };

        Mock::given(method("POST"))
            .and(path("/api/v2/request"))
            .and(body_json(&payload))
            .respond_with(ResponseTemplate::new(201).set_body_json(&expected_response))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/v2/request", mock_server.uri()))
            .json(&payload)
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let resp_body: CreateRequestResponse = response.json().await.unwrap();
        assert_eq!(resp_body, expected_response);
    }
}
