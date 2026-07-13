# External registry follow-ups (#26 Smithery, #32 GitHub MCP)

Operator playbook for the two **external-registry** issues after Conductor
tracks were archived.

## Fresh verification — 2026-07-11

The live endpoints were rechecked before changing issue state:

- Smithery namespace `edithatogo/fyi-mcp`: `isDeployed=true`, `remote=false`,
  `useCount=0`, `score=null`.
- Smithery detail: 14 tools, 2 resources, and 3 prompts.
- Official MCP Registry: `io.github.edithatogo/fyi-mcp` version `0.1.2`,
  `status=active`, `isLatest=true`.
- GitHub curated search `https://github.com/mcp?q=fyi-mcp`: page loads but has
  no server card; direct path `github.com/mcp/io.github.edithatogo/fyi-mcp`
  returns 404.

## #26 — Smithery live score and indexing refresh

### Current verified state (2026-07-11, post republish)

| Check | Result |
|-------|--------|
| Registry namespace API | `edithatogo/fyi-mcp` present (`isDeployed: true`, `remote: false`, homepage GitHub) |
| Public page | https://smithery.ai/server/@edithatogo/fyi-mcp returns 200 |
| Live binary surface | **14 tools**, **2 resources** (`fyi://…`), **3 prompts**, `initialize.instructions` (~966 chars) |
| Registry detail scan | Intermittently **14 / 2 / 3** with rich tool descriptions (`firstDescLen≈355`); sometimes lags at stale **12 / 0 / 0** |
| **Score field** | Still **`null`** for this and **all** sibling `edithatogo/*` local servers |
| useCount | 0 |
| Scored comparison | Remote high-traffic servers (e.g. `github`, `vercel/grep`) report non-null `score` + `useCount` ≫ 0 |

**Conclusion (operator):** Repo + MCPB publish have maximised score *eligibility*. A null
list-endpoint score with `useCount: 0` and `remote: false` is consistent across the
namespace and is **not** fixed by further code-only changes. Residual is Smithery-side
scoring / usage backfill for local stdio MCPB packages.

### Repo actions completed to maximise score eligibility

Smithery quality dimensions (tool descriptions, annotations, resources, metadata,
instructions, prompts, minimal config) are addressed by:

| Dimension | Repo evidence |
|-----------|----------------|
| Tool descriptions / parameters / annotations | `crates/fyi-mcp` tools/list + `enrich_tool_definitions` (PR #128, #130, #132) |
| Resources | `resources/list` + `resources/read` (`fyi://…`) |
| Server instructions | `initialize.instructions` (PR #133) |
| Prompts | `prompts/list` + `prompts/get` (PR #133) |
| Server metadata | Root [`smithery.yaml`](../smithery.yaml) (PR #133) |
| Config UX | No required secrets; optional `DATABASE_URL` / `FYI_MCP_EPHEMERAL` only |
| Official registry coherence | `server.json` → `io.github.edithatogo/fyi-mcp` @ 0.1.2 |
| MCPB manifest export | [`scripts/export_mcp_manifest.py`](../scripts/export_mcp_manifest.py) → [`packaging/mcpb/fyi-mcp/manifest.json`](../packaging/mcpb/fyi-mcp/manifest.json) |

### Operator / automation steps (Smithery)

**Rebuild + republish MCPB (stdio) so Smithery rescans tools/resources/prompts:**

```bash
cargo build -p fyi-mcp --release
uv run python scripts/export_mcp_manifest.py
# stage packaging/mcpb/fyi-mcp/manifest.json + target/release/fyi-mcp.exe into an .mcpb zip
# (Windows): Compress-Archive then rename to .mcpb
smithery auth login   # once
smithery mcp publish ./target/mcpb/fyi-mcp-smithery-refresh.mcpb -n edithatogo/fyi-mcp
```

**Done 2026-07-09 (authenticated publishes, status SUCCESS):**

| Release id | Notes |
|------------|--------|
| `8828d4a6-8474-4534-9e91-f06aeafa0710` | First rich MCPB republish |
| `9219599b-af42-4a24-b509-4cd2c9e5d42f` | Re-export + republish after scan lag (14/2/3 in package) |

Dashboard: https://smithery.ai/server/@edithatogo/fyi-mcp/releases

1. Re-check (poll a few times; detail cache can lag):
   ```bash
   curl -sS "https://registry.smithery.ai/servers/@edithatogo/fyi-mcp" | jq '{tools:(.tools|length),resources:(.resources|length),prompts:(.prompts|length)}'
   curl -sS "https://registry.smithery.ai/servers?namespace=edithatogo" | jq '.servers[] | select(.slug=="fyi-mcp") | {score,useCount,remote}'
   ```
2. Optional: exercise once via Smithery UI / client if score stays null after usage.
3. **Close criteria** below — do not block on infinite republish loops.

### When to close #26

Close when **at least one** is true:

- API/page reports a **non-null numeric score**, **or**
- Maintainer judgement that listing + capability scan evidence is complete and score remains
  null solely as a Smithery local/stdio scoring gap (same null for all `edithatogo/*`
  zero-use local servers; remote packages show scores).

---

## #32 — GitHub MCP Registry onboarding (`github.com/mcp`)

### Current verified state (2026-07-11)

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

### Operator steps (onboarding request) — **filed**

1. Confirm OSS registry still lists latest:
   ```bash
   curl -sS "https://registry.modelcontextprotocol.io/v0/servers?search=fyi-mcp" | jq .
   ```
2. **Done 2026-07-09 — requests filed:**
   - Comment on [discussion #1257](https://github.com/github/github-mcp-server/discussions/1257#discussioncomment-17584387)
   - Dedicated Q&A: [discussion #2844](https://github.com/github/github-mcp-server/discussions/2844)
   - Same-pattern bug cross-link: [modelcontextprotocol/registry#1107](https://github.com/modelcontextprotocol/registry/issues/1107#issuecomment-4924522420)
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
