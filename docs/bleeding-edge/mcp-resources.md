# MCP resources (local corpus)

Status: **experimental foundation** (MCP server surface + docs). The `fyi-mcp`
server can expose local SQLite-backed FOI data as [MCP resources](https://modelcontextprotocol.io/)
in addition to tools.

> **Disclaimer:** Resources reflect the **local** operator database only. They
> are not a live scrape of Alaveteli sites. Treat contents as sensitive case
> material (names, request bodies, authorities).

## Goals

- Let MCP-capable clients browse authorities and requests without inventing
  tool-call sequences for simple read paths.
- Use stable `fyi://` URIs so prompts and clients can bookmark documents.
- Keep MIME type `application/json` for structured consumption.

## Feature flag

| Flag | String id | Default |
|------|-----------|---------|
| `FeatureFlag::McpResources` | `mcp_resources` | off in `FeatureSet` (server may still implement handlers) |

Library-level gating lives in `fyi_core::features`. Wire flags into the MCP
binary when productizing; handlers already exist for list/read.

## URI catalog

| URI | Name | Description |
|-----|------|-------------|
| `fyi://authorities` | Public authorities | Imported authority records (`slug`, `name`, `url`) |
| `fyi://requests` | Request index | Local requests: `id`, `title`, `status`, per-item `uri` |
| `fyi://requests/{id}` | Request document | Full single request row as JSON |

List responses follow MCP `resources/list` shape (JSON):

```json
{
  "resources": [
    {
      "uri": "fyi://authorities",
      "name": "Public authorities",
      "description": "Imported public authority records used for FOI routing.",
      "mimeType": "application/json"
    },
    {
      "uri": "fyi://requests",
      "name": "Request index",
      "description": "Index of locally tracked FYI/Alaveteli requests.",
      "mimeType": "application/json"
    }
  ]
}
```

When the DB is available, `list_mcp_resources` also appends one resource entry
per known request (`fyi://requests/{id}`) so clients can deep-link.

## Read semantics

Implementation: `list_mcp_resources` / `read_mcp_resource` in
`crates/fyi-mcp/src/main.rs`.

| URI | Behaviour on read |
|-----|-------------------|
| `fyi://authorities` | Ensures authorities table exists; returns JSON array of authorities |
| `fyi://requests` | Returns pretty-printed index of up to 500 requests |
| `fyi://requests/{id}` | Parses `{id}` as `i64`; returns full request or not-found error |

Content items are MCP resource contents with `uri`, `mimeType`, and `text`
(JSON string body).

### Example index document

```json
[
  {
    "id": 1,
    "title": "Elective surgery waitlists",
    "status": "submitted",
    "uri": "fyi://requests/1"
  }
]
```

## Tools vs resources

| Concern | Prefer |
|---------|--------|
| Browse / attach context to a chat | **Resources** (`fyi://…`) |
| Create/update requests, import authorities, run monitors | **Tools** (existing MCP tools) |
| Idempotent read of large lists | Resources or list tools (both exist) |

Resources are read-only. Mutations remain tool-only.

## Security notes

- Do not publish an unauthenticated MCP endpoint on a public network.
- Resource text may include personal information from request titles/bodies.
- Prefer localhost transports for operator desktops.
- Align with `docs/SECURITY_CONFIG.md` for API keys and session handling.

## Client usage sketch

1. `resources/list` → discover `fyi://requests`.
2. `resources/read` with `fyi://requests` → pick an id.
3. `resources/read` with `fyi://requests/42` → attach full JSON to the model context.
4. Use drafting or follow-up **tools** if the client needs to mutate state.

## Related

- [README.md](./README.md) — experimental matrix
- `crates/fyi-mcp/README.md` — server overview
- `fyi_core::db` — request/authority persistence
- Offline PWA design may cache the same shapes client-side (see `offline-pwa-design.md`)
