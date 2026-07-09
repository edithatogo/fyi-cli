# External registry follow-ups (#26 Smithery, #32 GitHub MCP)

Operator playbook for the two remaining **blocked-external** issues after Conductor
tracks were archived.

## #26 — Smithery live score and indexing refresh

### Current verified state (2026-07-09)

| Check | Result |
|-------|--------|
| Registry namespace API | `edithatogo/fyi-mcp` present (`isDeployed: true`, homepage GitHub) |
| Public page | https://smithery.ai/server/@edithatogo/fyi-mcp returns 200 |
| **Score field** | Still **`null`** in `registry.smithery.ai` for this and sibling edithatogo servers |
| useCount | 0 |

A null score is common for local/stdio servers that have not yet been exercised
through Smithery’s scoring pipeline (or when scoring has not been backfilled).

### Repo actions completed to maximise score eligibility

Smithery quality dimensions (tool descriptions, annotations, resources, metadata,
instructions, prompts, minimal config) are addressed by:

| Dimension | Repo evidence |
|-----------|----------------|
| Tool descriptions / parameters / annotations | `crates/fyi-mcp` tools/list + `enrich_tool_definitions` (PR #128, #130, #132) |
| Resources | `resources/list` + `resources/read` (`fyi://…`) |
| Server instructions | `initialize.instructions` (this follow-up PR) |
| Prompts | `prompts/list` + `prompts/get` (this follow-up PR) |
| Server metadata | Root [`smithery.yaml`](../smithery.yaml) (this follow-up PR) |
| Config UX | No required secrets; optional `DATABASE_URL` / `FYI_MCP_EPHEMERAL` only |
| Official registry coherence | `server.json` → `io.github.edithatogo/fyi-mcp` @ 0.1.2 |

### Operator steps (Smithery dashboard / CLI)

1. Open https://smithery.ai → namespace **edithatogo** → server **fyi-mcp**.
2. Trigger **re-index / rescan** if the UI exposes it (paste GitHub URL if adding fresh).
3. Optional CLI (authenticated):
   ```bash
   npx @smithery/cli mcp search fyi-mcp
   npx @smithery/cli auth login   # if required
   ```
4. After rescan, re-check:
   ```bash
   curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq '.servers[] | select(.slug=="fyi-mcp")'
   ```
5. If score remains null after rescan + usage: capture exact dashboard screenshot/API
   snippet and leave issue open with evidence (Smithery-side scoring gap).

### When to close #26

Close when **at least one** is true:

- API/page reports a **non-null numeric score**, **or**
- Smithery documents that local/stdio listings intentionally keep `score: null` and
  metadata visibility criteria are met (description + tools visible).

---

## #32 — GitHub MCP Registry onboarding (`github.com/mcp`)

### Current verified state (2026-07-09)

| Check | Result |
|-------|--------|
| Official OSS MCP Registry | `io.github.edithatogo/fyi-mcp` **0.1.2** `active` + `isLatest: true` |
| `https://github.com/mcp?q=fyi-mcp` | No curated server card for our package (search UI only) |
| Direct paths | `github.com/mcp/io.github.edithatogo/fyi-mcp` → **404** |

### Upstream process (evidence)

GitHub discussion [github/github-mcp-server#1257](https://github.com/github/github-mcp-server/discussions/1257)
(collaborator **trent-j**, May–Jun 2026):

- OSS registry **version sync** exists **after** a server is onboarded.
- **Initial onboarding remains manual curation**.
- Servers already on the official registry should be ready; product team performs listing.

Prerequisite we already satisfy: published on the open-source registry
([publish guide](https://github.com/modelcontextprotocol/registry/blob/main/docs/guides/publishing/publish-server.md)).

### Operator steps (onboarding request)

1. Confirm OSS registry still lists latest:
   ```bash
   curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .
   ```
2. Post or comment on a **github/github-mcp-server** discussion (or GitHub Support)
   requesting curation of:
   - **Name:** `io.github.edithatogo/fyi-mcp`
   - **Repo:** https://github.com/edithatogo/fyi-cli
   - **Version:** 0.1.2 (active/latest on OSS registry)
   - **Description:** Local FOI/OIA Alaveteli request tracker MCP server
3. After onboard, verify:
   - Search: https://github.com/mcp?q=fyi-mcp
   - Direct listing path (once known) returns 200 and shows 0.1.2+

### When to close #32

Close when `fyi-mcp` / `io.github.edithatogo/fyi-mcp` is **searchable and listed** on
https://github.com/mcp with a version ≥ 0.1.2.

---

## Cross-links

- Matrix: [`docs/registry-distribution-matrix.md`](registry-distribution-matrix.md)
- Official `server.json`: [`server.json`](../server.json)
- Smithery metadata: [`smithery.yaml`](../smithery.yaml)
- Glama (already live): https://glama.ai/mcp/servers/edithatogo/fyi-cli
