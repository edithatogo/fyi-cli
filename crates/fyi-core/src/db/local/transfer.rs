//! Versioned private transfer of local tracking state, independent of acquisition.
use super::provenance::{FeedEvent, RequestSnapshot};
use super::requests::{
    get_request_in_connection, parsed_timestamp, validate_status, TrackedRequest,
};
use super::{required_nullable, LocalAuthority, LocalError, MAX_LOCAL_INPUT_BYTES};
use crate::db::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::{Connection, FromRow, SqliteConnection};
use std::collections::{HashMap, HashSet};

pub const LOCAL_TRANSFER_FORMAT: &str = "fyi-local-requests";
pub const LOCAL_TRANSFER_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalRequestEvent {
    pub id: i64,
    pub tracked_request_id: i64,
    pub kind: String,
    #[serde(deserialize_with = "required_nullable")]
    pub old_status: Option<String>,
    pub new_status: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalSequences {
    pub tracked_requests: i64,
    pub local_request_events: i64,
    pub feed_events: i64,
    pub request_snapshots: i64,
}

impl LocalSequences {
    fn entries(&self) -> [(&'static str, i64); 4] {
        [
            ("tracked_requests", self.tracked_requests),
            ("local_request_events", self.local_request_events),
            ("feed_events", self.feed_events),
            ("request_snapshots", self.request_snapshots),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocalTransferDocument {
    pub format: String,
    pub schema_version: u32,
    pub authorities: Vec<LocalAuthority>,
    pub requests: Vec<TrackedRequest>,
    pub events: Vec<LocalRequestEvent>,
    pub feed_events: Vec<FeedEvent>,
    pub request_snapshots: Vec<RequestSnapshot>,
    pub sequences: LocalSequences,
}

#[derive(Debug, Clone)]
pub enum LocalImport {
    Versioned(LocalTransferDocument),
    Legacy(Vec<TrackedRequest>),
}

/// Deserialize directly into typed records, preserving duplicate-key rejection.
pub fn parse_local_import(bytes: &[u8]) -> Result<LocalImport, LocalError> {
    if bytes.len() > MAX_LOCAL_INPUT_BYTES {
        return Err(LocalError::InvalidInput("request import exceeds 16 MiB"));
    }
    let invalid = || LocalError::InvalidInput("invalid local request transfer document");
    match bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
    {
        Some(b'{') => {
            let document: LocalTransferDocument =
                serde_json::from_slice(bytes).map_err(|_| invalid())?;
            if document.format != LOCAL_TRANSFER_FORMAT
                || document.schema_version != LOCAL_TRANSFER_VERSION
            {
                return Err(LocalError::InvalidInput(
                    "unsupported local request transfer version",
                ));
            }
            Ok(LocalImport::Versioned(document))
        }
        Some(b'[') => Ok(LocalImport::Legacy(
            serde_json::from_slice(bytes).map_err(|_| invalid())?,
        )),
        _ => Err(invalid()),
    }
}

/// Produce a deterministic document that the bounded importer can read again.
pub fn encode_local_export(document: &LocalTransferDocument) -> Result<Vec<u8>, LocalError> {
    document.validate()?;
    let mut bytes = serde_json::to_vec_pretty(document)
        .map_err(|_| LocalError::InvalidInput("unable to encode local request export"))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_LOCAL_INPUT_BYTES {
        return Err(LocalError::InvalidInput("request export exceeds 16 MiB"));
    }
    Ok(bytes)
}

fn positive_unique(ids: impl Iterator<Item = i64>) -> Result<HashSet<i64>, LocalError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id <= 0 || !seen.insert(id) {
            return Err(LocalError::InvalidInput(
                "invalid or duplicate local transfer ID",
            ));
        }
    }
    Ok(seen)
}

fn validate_requests(requests: &[TrackedRequest]) -> Result<HashSet<i64>, LocalError> {
    let ids = positive_unique(requests.iter().map(|row| row.id))?;
    for row in requests {
        if row.authority_slug.trim().is_empty()
            || row.authority_slug.len() > 1024
            || row.authority_slug.chars().any(char::is_control)
            || row.fyi_request_id.is_some_and(|id| id <= 0)
        {
            return Err(LocalError::InvalidInput("invalid local request identity"));
        }
    }
    // Historical text and nullable timestamp/status fields are preserved verbatim.
    // Registration's stricter input policy is not applied retroactively to a backup.
    Ok(ids)
}

fn validate_history(
    requests: &[TrackedRequest],
    events: &[LocalRequestEvent],
) -> Result<(), LocalError> {
    positive_unique(events.iter().map(|event| event.id))?;
    let requests: HashMap<_, _> = requests
        .iter()
        .map(|request| (request.id, request))
        .collect();
    let mut ordered: Vec<_> = events.iter().collect();
    ordered.sort_by_key(|event| event.id);
    let mut last: HashMap<i64, &str> = HashMap::new();
    for event in ordered {
        if !requests.contains_key(&event.tracked_request_id)
            || parsed_timestamp(Some(&event.occurred_at)).is_none()
        {
            return Err(LocalError::InvalidInput(
                "invalid local history reference or timestamp",
            ));
        }
        validate_status(&event.new_status)?;
        match event.kind.as_str() {
            "created"
                if event.old_status.is_none() && !last.contains_key(&event.tracked_request_id) => {}
            "status_changed" if event.old_status.as_deref() != Some(event.new_status.as_str()) => {
                if last
                    .get(&event.tracked_request_id)
                    .is_some_and(|previous| event.old_status.as_deref() != Some(*previous))
                {
                    return Err(LocalError::InvalidInput(
                        "inconsistent local status history",
                    ));
                }
            }
            _ => return Err(LocalError::InvalidInput("invalid local history transition")),
        }
        last.insert(event.tracked_request_id, &event.new_status);
    }
    if last
        .into_iter()
        .any(|(id, status)| requests[&id].status.as_deref() != Some(status))
    {
        return Err(LocalError::InvalidInput(
            "local request status disagrees with its history",
        ));
    }
    Ok(())
}

impl LocalTransferDocument {
    fn validate(&self) -> Result<(), LocalError> {
        if self.format != LOCAL_TRANSFER_FORMAT || self.schema_version != LOCAL_TRANSFER_VERSION {
            return Err(LocalError::InvalidInput(
                "unsupported local request transfer version",
            ));
        }
        let mut authorities = HashSet::new();
        for authority in &self.authorities {
            authority.validate()?;
            if !authorities.insert(authority.slug.as_str()) {
                return Err(LocalError::InvalidInput("duplicate local authority slug"));
            }
        }
        let ids = validate_requests(&self.requests)?;
        if self
            .requests
            .iter()
            .any(|row| !authorities.contains(row.authority_slug.as_str()))
        {
            return Err(LocalError::InvalidInput(
                "local request authority is absent from the document",
            ));
        }
        validate_history(&self.requests, &self.events)?;
        positive_unique(self.feed_events.iter().map(|row| row.id))?;
        positive_unique(self.request_snapshots.iter().map(|row| row.id))?;
        if self.feed_events.iter().any(|row| {
            row.tracked_request_id.is_none_or(|id| !ids.contains(&id))
                || row.request_id_guess.is_some_and(|id| id <= 0)
        }) {
            return Err(LocalError::InvalidInput("invalid linked feed reference"));
        }
        let remote_ids: HashSet<_> = self
            .requests
            .iter()
            .filter_map(|row| row.fyi_request_id)
            .collect();
        if self
            .request_snapshots
            .iter()
            .any(|row| !remote_ids.contains(&row.fyi_request_id))
        {
            return Err(LocalError::InvalidInput(
                "invalid linked snapshot reference",
            ));
        }
        let maxima = [
            self.requests.iter().map(|row| row.id).max().unwrap_or(0),
            self.events.iter().map(|row| row.id).max().unwrap_or(0),
            self.feed_events.iter().map(|row| row.id).max().unwrap_or(0),
            self.request_snapshots
                .iter()
                .map(|row| row.id)
                .max()
                .unwrap_or(0),
        ];
        if self
            .sequences
            .entries()
            .iter()
            .zip(maxima)
            .any(|((_, value), maximum)| *value < maximum)
        {
            return Err(LocalError::InvalidInput(
                "invalid local allocation high-water mark",
            ));
        }
        Ok(())
    }
}

async fn read_requests(
    connection: &mut SqliteConnection,
) -> Result<Vec<TrackedRequest>, LocalError> {
    Ok(sqlx::query_as(
        "SELECT id, authority_slug, title, body, tags, fyi_url, fyi_request_id,
                status, created_at, updated_at, last_seen_at, last_event_title
         FROM tracked_requests ORDER BY id",
    )
    .fetch_all(connection)
    .await?)
}

async fn read_events(
    connection: &mut SqliteConnection,
) -> Result<Vec<LocalRequestEvent>, LocalError> {
    Ok(sqlx::query_as(
        "SELECT id, tracked_request_id, kind, old_status, new_status, occurred_at
         FROM local_request_events ORDER BY id",
    )
    .fetch_all(connection)
    .await?)
}

async fn read_sequences(connection: &mut SqliteConnection) -> Result<LocalSequences, LocalError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT name, seq FROM sqlite_sequence WHERE name IN
         ('tracked_requests', 'local_request_events', 'feed_events', 'request_snapshots')",
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut seen = HashSet::new();
    let mut result = LocalSequences::default();
    for (name, value) in rows {
        if value < 0 || !seen.insert(name.clone()) {
            return Err(LocalError::Schema);
        }
        match name.as_str() {
            "tracked_requests" => result.tracked_requests = value,
            "local_request_events" => result.local_request_events = value,
            "feed_events" => result.feed_events = value,
            "request_snapshots" => result.request_snapshots = value,
            _ => return Err(LocalError::Schema),
        }
    }
    let maxima: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COALESCE(MAX(id), 0) FROM tracked_requests),
                (SELECT COALESCE(MAX(id), 0) FROM local_request_events),
                (SELECT COALESCE(MAX(id), 0) FROM feed_events),
                (SELECT COALESCE(MAX(id), 0) FROM request_snapshots)",
    )
    .fetch_one(connection)
    .await?;
    if result.tracked_requests < maxima.0
        || result.local_request_events < maxima.1
        || result.feed_events < maxima.2
        || result.request_snapshots < maxima.3
    {
        return Err(LocalError::Schema);
    }
    Ok(result)
}

impl DbPool {
    /// Export only local tracking and linked evidence from one consistent read view.
    pub async fn export_local_requests(&self) -> Result<LocalTransferDocument, LocalError> {
        let mut transaction = self.pool().begin().await?;
        let result = async {
            super::schema::validate_local_references(&mut transaction).await?;
            let authorities =
                sqlx::query_as("SELECT slug, name, url FROM authorities ORDER BY slug")
                    .fetch_all(&mut *transaction)
                    .await?;
            let requests = read_requests(&mut transaction).await?;
            let events = read_events(&mut transaction).await?;
            let feed_events = sqlx::query_as(
                "SELECT f.id, f.feed_url, f.event_id, f.title, f.link, f.published,
                        f.summary, f.request_id_guess, f.tracked_request_id, f.raw_json, f.seen_at
                 FROM feed_events f JOIN tracked_requests r ON r.id = f.tracked_request_id
                 ORDER BY f.id",
            )
            .fetch_all(&mut *transaction)
            .await?;
            let request_snapshots = sqlx::query_as(
                "SELECT s.id, s.fyi_request_id, s.source_url, s.raw_json, s.fetched_at
                 FROM request_snapshots s WHERE EXISTS
                 (SELECT 1 FROM tracked_requests r WHERE r.fyi_request_id = s.fyi_request_id)
                 ORDER BY s.id",
            )
            .fetch_all(&mut *transaction)
            .await?;
            let document = LocalTransferDocument {
                format: LOCAL_TRANSFER_FORMAT.to_owned(),
                schema_version: LOCAL_TRANSFER_VERSION,
                authorities,
                requests,
                events,
                feed_events,
                request_snapshots,
                sequences: read_sequences(&mut transaction).await?,
            };
            document.validate()?;
            Ok(document)
        }
        .await;
        match result {
            Ok(document) => {
                transaction.commit().await?;
                Ok(document)
            }
            Err(error) => {
                transaction.rollback().await?;
                Err(error)
            }
        }
    }

    /// Import atomically; identical rows never churn timestamps or allocation state.
    pub async fn import_local_requests(
        &self,
        input: &LocalImport,
        replace: bool,
    ) -> Result<usize, LocalError> {
        match input {
            LocalImport::Versioned(document) => document.validate()?,
            LocalImport::Legacy(requests) => {
                validate_requests(requests)?;
            }
        }
        let mut connection = self.pool().acquire().await?;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await?;
        let result = import_in_connection(&mut transaction, input, replace).await;
        match result {
            Ok(count) => {
                transaction.commit().await?;
                Ok(count)
            }
            Err(error) => {
                transaction.rollback().await?;
                Err(error)
            }
        }
    }
}

async fn import_in_connection(
    connection: &mut SqliteConnection,
    input: &LocalImport,
    replace: bool,
) -> Result<usize, LocalError> {
    let (requests, native) = match input {
        LocalImport::Versioned(document) => (document.requests.as_slice(), Some(document)),
        LocalImport::Legacy(requests) => (requests.as_slice(), None),
    };
    let events: &[LocalRequestEvent] = native.map_or(&[], |document| document.events.as_slice());
    let previous_requests = read_requests(connection).await?;
    let previous_events = read_events(connection).await?;
    let prior_events: HashMap<_, _> = previous_events
        .iter()
        .map(|event| (event.id, event))
        .collect();
    // Retained event IDs identify immutable history, even during a full replacement.
    for event in events {
        if prior_events
            .get(&event.id)
            .is_some_and(|previous| **previous != *event)
        {
            return Err(LocalError::InvalidInput(
                "existing local history ID conflicts",
            ));
        }
    }
    read_sequences(connection).await?;
    if replace {
        validate_replacement(
            connection,
            requests,
            &previous_requests,
            &previous_events,
            native.is_none(),
        )
        .await?;
    }
    if let Some(document) = native {
        for authority in &document.authorities {
            import_authority(connection, authority, replace).await?;
        }
    }
    if replace {
        let wanted_events: HashSet<_> = events.iter().map(|event| event.id).collect();
        for event in &previous_events {
            if !wanted_events.contains(&event.id) {
                sqlx::query("DELETE FROM local_request_events WHERE id = ?")
                    .bind(event.id)
                    .execute(&mut *connection)
                    .await?;
            }
        }
        let wanted_requests: HashSet<_> = requests.iter().map(|request| request.id).collect();
        for request in &previous_requests {
            if !wanted_requests.contains(&request.id) {
                sqlx::query("DELETE FROM tracked_requests WHERE id = ?")
                    .bind(request.id)
                    .execute(&mut *connection)
                    .await?;
            }
        }
    }
    for request in requests {
        import_request(connection, request, replace).await?;
    }
    for event in events {
        if !prior_events.contains_key(&event.id) {
            sqlx::query(
                "INSERT INTO local_request_events
                 (id, tracked_request_id, kind, old_status, new_status, occurred_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(event.id)
            .bind(event.tracked_request_id)
            .bind(&event.kind)
            .bind(&event.old_status)
            .bind(&event.new_status)
            .bind(&event.occurred_at)
            .execute(&mut *connection)
            .await?;
        }
    }
    if let Some(document) = native {
        for event in &document.feed_events {
            import_feed_event(connection, event).await?;
        }
        for snapshot in &document.request_snapshots {
            import_snapshot(connection, snapshot).await?;
        }
        advance_sequences(connection, &document.sequences).await?;
    }
    let final_requests = read_requests(connection).await?;
    let final_events = read_events(connection).await?;
    validate_history(&final_requests, &final_events)?;
    super::schema::validate_local_references(connection).await?;
    read_sequences(connection).await?;
    Ok(requests.len())
}

async fn validate_replacement(
    connection: &mut SqliteConnection,
    incoming: &[TrackedRequest],
    previous: &[TrackedRequest],
    previous_events: &[LocalRequestEvent],
    legacy: bool,
) -> Result<(), LocalError> {
    let linked_local: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT tracked_request_id FROM feed_events WHERE tracked_request_id IS NOT NULL",
    )
    .fetch_all(&mut *connection)
    .await?;
    let linked_remote: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT s.fyi_request_id FROM request_snapshots s WHERE EXISTS
         (SELECT 1 FROM tracked_requests r WHERE r.fyi_request_id = s.fyi_request_id)",
    )
    .fetch_all(&mut *connection)
    .await?;
    if legacy
        && (!previous_events.is_empty() || !linked_local.is_empty() || !linked_remote.is_empty())
    {
        return Err(LocalError::InvalidInput(
            "legacy array replacement would discard local provenance",
        ));
    }
    let before: HashMap<_, _> = previous
        .iter()
        .map(|request| (request.id, request))
        .collect();
    let after: HashMap<_, _> = incoming
        .iter()
        .map(|request| (request.id, request))
        .collect();
    for id in linked_local {
        let old = before.get(&id).ok_or(LocalError::Schema)?;
        let new = after.get(&id).ok_or(LocalError::InvalidInput(
            "replacement would orphan a linked feed",
        ))?;
        if old.fyi_request_id != new.fyi_request_id {
            return Err(LocalError::InvalidInput(
                "replacement would rebind a linked feed",
            ));
        }
    }
    let retained_remote: HashSet<_> = incoming
        .iter()
        .filter_map(|request| request.fyi_request_id)
        .collect();
    if linked_remote.iter().any(|id| !retained_remote.contains(id)) {
        return Err(LocalError::InvalidInput(
            "replacement would detach a linked snapshot",
        ));
    }
    Ok(())
}

async fn import_authority(
    connection: &mut SqliteConnection,
    authority: &LocalAuthority,
    replace: bool,
) -> Result<(), LocalError> {
    let previous: Option<LocalAuthority> =
        sqlx::query_as("SELECT slug, name, url FROM authorities WHERE slug = ?")
            .bind(&authority.slug)
            .fetch_optional(&mut *connection)
            .await?;
    match previous {
        Some(previous) if previous == *authority => return Ok(()),
        Some(_) if !replace => {
            return Err(LocalError::InvalidInput(
                "existing local authority conflicts",
            ))
        }
        Some(_) => {
            sqlx::query("UPDATE authorities SET name = ?, url = ? WHERE slug = ?")
                .bind(&authority.name)
                .bind(&authority.url)
                .bind(&authority.slug)
                .execute(connection)
                .await?;
        }
        None => {
            sqlx::query("INSERT INTO authorities (slug, name, url) VALUES (?, ?, ?)")
                .bind(&authority.slug)
                .bind(&authority.name)
                .bind(&authority.url)
                .execute(connection)
                .await?;
        }
    }
    Ok(())
}

async fn import_request(
    connection: &mut SqliteConnection,
    request: &TrackedRequest,
    replace: bool,
) -> Result<(), LocalError> {
    let previous = get_request_in_connection(connection, request.id).await?;
    if previous.as_ref() == Some(request) {
        return Ok(());
    }
    if previous.is_some() && !replace {
        return Err(LocalError::InvalidInput(
            "existing local request ID conflicts",
        ));
    }
    if previous.is_some() {
        sqlx::query(
            "UPDATE tracked_requests SET authority_slug = ?, title = ?, body = ?, tags = ?,
             fyi_url = ?, fyi_request_id = ?, status = ?, created_at = ?, updated_at = ?,
             last_seen_at = ?, last_event_title = ? WHERE id = ?",
        )
        .bind(&request.authority_slug)
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.tags)
        .bind(&request.fyi_url)
        .bind(request.fyi_request_id)
        .bind(&request.status)
        .bind(&request.created_at)
        .bind(&request.updated_at)
        .bind(&request.last_seen_at)
        .bind(&request.last_event_title)
        .bind(request.id)
        .execute(connection)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO tracked_requests
             (id, authority_slug, title, body, tags, fyi_url, fyi_request_id, status,
              created_at, updated_at, last_seen_at, last_event_title)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(request.id)
        .bind(&request.authority_slug)
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.tags)
        .bind(&request.fyi_url)
        .bind(request.fyi_request_id)
        .bind(&request.status)
        .bind(&request.created_at)
        .bind(&request.updated_at)
        .bind(&request.last_seen_at)
        .bind(&request.last_event_title)
        .execute(connection)
        .await?;
    }
    Ok(())
}

async fn import_feed_event(
    connection: &mut SqliteConnection,
    event: &FeedEvent,
) -> Result<(), LocalError> {
    let previous: Option<FeedEvent> = sqlx::query_as(
        "SELECT id, feed_url, event_id, title, link, published, summary, request_id_guess,
                tracked_request_id, raw_json, seen_at FROM feed_events WHERE id = ?",
    )
    .bind(event.id)
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(previous) = previous {
        return if previous == *event {
            Ok(())
        } else {
            Err(LocalError::InvalidInput(
                "existing feed evidence ID conflicts",
            ))
        };
    }
    sqlx::query(
        "INSERT INTO feed_events
         (id, feed_url, event_id, title, link, published, summary, request_id_guess,
          tracked_request_id, raw_json, seen_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.id)
    .bind(&event.feed_url)
    .bind(&event.event_id)
    .bind(&event.title)
    .bind(&event.link)
    .bind(&event.published)
    .bind(&event.summary)
    .bind(event.request_id_guess)
    .bind(event.tracked_request_id)
    .bind(&event.raw_json)
    .bind(&event.seen_at)
    .execute(connection)
    .await?;
    Ok(())
}

async fn import_snapshot(
    connection: &mut SqliteConnection,
    snapshot: &RequestSnapshot,
) -> Result<(), LocalError> {
    let previous: Option<RequestSnapshot> = sqlx::query_as(
        "SELECT id, fyi_request_id, source_url, raw_json, fetched_at FROM request_snapshots WHERE id = ?",
    ).bind(snapshot.id).fetch_optional(&mut *connection).await?;
    if let Some(previous) = previous {
        return if previous == *snapshot {
            Ok(())
        } else {
            Err(LocalError::InvalidInput(
                "existing snapshot evidence ID conflicts",
            ))
        };
    }
    sqlx::query(
        "INSERT INTO request_snapshots (id, fyi_request_id, source_url, raw_json, fetched_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(snapshot.id)
    .bind(snapshot.fyi_request_id)
    .bind(&snapshot.source_url)
    .bind(&snapshot.raw_json)
    .bind(&snapshot.fetched_at)
    .execute(connection)
    .await?;
    Ok(())
}

async fn advance_sequences(
    connection: &mut SqliteConnection,
    incoming: &LocalSequences,
) -> Result<(), LocalError> {
    for (name, value) in incoming.entries() {
        let previous: Option<i64> =
            sqlx::query_scalar("SELECT seq FROM sqlite_sequence WHERE name = ?")
                .bind(name)
                .fetch_optional(&mut *connection)
                .await?;
        if value > previous.unwrap_or(0) {
            if previous.is_some() {
                sqlx::query("UPDATE sqlite_sequence SET seq = ? WHERE name = ?")
                    .bind(value)
                    .bind(name)
                    .execute(&mut *connection)
                    .await?;
            } else {
                sqlx::query("INSERT INTO sqlite_sequence (name, seq) VALUES (?, ?)")
                    .bind(name)
                    .bind(value)
                    .execute(&mut *connection)
                    .await?;
            }
        }
    }
    Ok(())
}
