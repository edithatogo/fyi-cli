//! Pinned upstream authority-import API, selected explicitly by the caller.
//!
//! This API uses an authority key and does not represent requester sessions.

use super::*;
use crate::api::CorrespondenceDirection;
use read_contract::{
    parse_upstream_authority_request, safe_source_error, upstream_authority_request_url,
    validate_authority_key,
};

fn invalid() -> anyhow::Error {
    SourceReadError::new(SourceFailureKind::Validation).into()
}

fn malformed() -> anyhow::Error {
    SourceReadError::new(SourceFailureKind::MalformedPayload).into()
}

impl SyncClient {
    fn stock_url(&self, path: &str, key: &str) -> Result<Url> {
        validate_authority_key(key)?;
        self.base_url.join(path).map_err(|_| invalid())
    }

    fn stock_form(&self, url: Url, fields: &[(&str, &str)]) -> RequestBuilder {
        let mut encoded = url.clone();
        {
            let mut pairs = encoded.query_pairs_mut();
            for (name, value) in fields {
                pairs.append_pair(name, value);
            }
        }
        let body = encoded.query().unwrap_or_default().to_owned();
        self.http
            .post(url)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
    }

    async fn stock_send(&self, request: RequestBuilder) -> Result<GuardedResponse> {
        let response = self
            .send_guarded_with_limits_using(
                request,
                None,
                false,
                "alaveteli_authority_write",
                |request| self.execute_stock_request(request),
            )
            .await
            .map_err(safe_source_error)?;
        ensure_success(response, "authority endpoint")
            .await
            .map_err(safe_source_error)
    }

    async fn execute_stock_request(
        &self,
        request: reqwest::Request,
    ) -> Result<(reqwest::Response, Url)> {
        #[cfg(feature = "sdk-regression-fixture")]
        if self.sdk_fixture_transport.is_some() {
            return self.execute_guarded_request(request).await;
        }
        let http = self.evidence_http.as_ref().ok_or_else(invalid)?;
        let response = http
            .execute(request)
            .await
            .map_err(|error| error.without_url())?;
        let url = response.url().clone();
        Ok((response, url))
    }

    fn stock_result(&self, response: &GuardedResponse, key: &str) -> Result<Value> {
        let value: Value = serde_json::from_slice(&response.body).map_err(|_| malformed())?;
        if value.get("errors").is_some() {
            return Err(malformed());
        }
        let raw_url = value
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(malformed)?;
        let url = Url::parse(raw_url).map_err(|_| malformed())?;
        // Stock constructs an http URL even when the API is served over https.
        // The URL is returned as data, never fetched or used for a redirect.
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str() != self.base_url.host_str()
            || url.port() != self.base_url.port()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || raw_url.contains(key)
        {
            return Err(malformed());
        }
        Ok(value)
    }

    /// Import a request using the pinned stock authority API.
    pub async fn stock_create_request(
        &self,
        key: &str,
        payload: &CreateRequestPayload,
    ) -> Result<CreateRequestResponse> {
        if payload
            .tags
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            return Err(invalid());
        }
        let url = self.stock_url("api/v2/request.json", key)?;
        let json = serde_json::json!({"title":payload.title, "body":payload.body, "external_user_name":payload.external_user_name, "external_url":payload.external_url}).to_string();
        let response = self
            .stock_send(self.stock_form(url, &[("k", key), ("request_json", &json)]))
            .await?;
        let value = self.stock_result(&response, key)?;
        let id = value
            .get("id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(malformed)?;
        Ok(CreateRequestResponse {
            id,
            url: value["url"].as_str().ok_or_else(malformed)?.to_owned(),
        })
    }

    /// Import correspondence, preserving the existing attachment checks.
    pub async fn stock_add_correspondence<P: AsRef<Path>>(
        &self,
        key: &str,
        request_id: i64,
        payload: &AddCorrespondencePayload,
        attachments: &[P],
    ) -> Result<CorrespondenceResponse> {
        if request_id <= 0
            || (payload.direction == CorrespondenceDirection::Request
                && (!attachments.is_empty() || payload.state.is_some()))
        {
            return Err(invalid());
        }
        if let Some(state) = &payload.state {
            validate_request_state(state).map_err(|_| invalid())?;
        }
        let url = self.stock_url(&format!("api/v2/request/{request_id}.json"), key)?;
        let json = serde_json::json!({"direction":payload.direction, "body":payload.body, "sent_at":payload.sent_at}).to_string();
        let request = if attachments.is_empty() {
            let mut fields = vec![("k", key), ("correspondence_json", json.as_str())];
            if let Some(state) = &payload.state {
                fields.push(("state", state));
            }
            self.stock_form(url, &fields)
        } else {
            let mut form = reqwest::multipart::Form::new()
                .text("k", key.to_owned())
                .text("correspondence_json", json);
            if let Some(state) = &payload.state {
                form = form.text("state", state.clone());
            }
            form = Self::add_attachment_parts(form, attachments, true)
                .await
                .map_err(|_| invalid())?;
            self.http.post(url).multipart(form)
        };
        let response = self.stock_send(request).await?;
        self.stock_result(&response, key)?;
        Ok(CorrespondenceResponse { success: true })
    }

    /// Preflight the expected state before the stock authority state update.
    /// The pinned upstream server does not guarantee an atomic compare-and-set.
    pub async fn stock_update_state_if_current(
        &self,
        key: &str,
        request_id: i64,
        expected: &str,
        payload: &UpdateRequestStatePayload,
    ) -> Result<UpdateRequestStateResponse> {
        validate_request_state(expected).map_err(|_| invalid())?;
        validate_request_state(&payload.state).map_err(|_| invalid())?;
        let get_url = upstream_authority_request_url(&self.base_url, request_id, key)?;
        let response = self
            .send_guarded_with_limits_using(
                self.http
                    .get(get_url.clone())
                    .header(header::ACCEPT_ENCODING, "identity"),
                None,
                false,
                "alaveteli_authority_request",
                |request| self.execute_stock_request(request),
            )
            .await
            .map_err(safe_source_error)?;
        if response.final_url != get_url {
            return Err(invalid());
        }
        let response = ensure_success(response, "authority request endpoint")
            .await
            .map_err(safe_source_error)?;
        let current = parse_upstream_authority_request(&response.body, request_id)?;
        if current.status.as_deref() != Some(expected) {
            return Err(invalid());
        }
        let etag = response.headers.get(header::ETAG).cloned();
        let url = self.stock_url(&format!("api/v2/request/{request_id}/update.json"), key)?;
        let mut request = self.stock_form(url, &[("k", key), ("state", &payload.state)]);
        if let Some(etag) = etag {
            request = request.header(header::IF_MATCH, etag);
        }
        let response = self.stock_send(request).await?;
        self.stock_result(&response, key)?;
        Ok(UpdateRequestStateResponse { updated: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stock_writes_reject_opaque_clients_without_a_no_redirect_transport() {
        let client = SyncClient::with_http_client("https://source.invalid", Client::new()).unwrap();
        let payload = CreateRequestPayload {
            title: "Synthetic".into(),
            body: "Synthetic".into(),
            external_user_name: "Synthetic".into(),
            external_url: "https://source.invalid/external".into(),
            tags: None,
        };
        let error = client
            .stock_create_request("synthetic-key", &payload)
            .await
            .unwrap_err();
        assert_eq!(
            source_failure_kind(&error),
            Some(SourceFailureKind::Validation)
        );
        assert!(!error.to_string().contains("synthetic-key"));
    }
}
