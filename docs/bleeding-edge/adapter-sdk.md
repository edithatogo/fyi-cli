# Community jurisdiction adapter SDK

Status: **experimental foundation** (library + docs). Third-party FOI portals
that are not first-party in the embedded catalog can be described via the
`CommunityJurisdictionAdapter` trait in `fyi-core`.

> **Disclaimer:** Adapter registration does not imply legal endorsement of a
> jurisdiction’s FOI regime, nor production support for the remote site. Community
> adapters default to `InstanceStatus::Community` and may lack write or
> prefilled-URL capabilities.

## Goals

- Let contributors package a jurisdiction as code without forking the catalog TOML.
- Keep network I/O out of the trait (catalog mapping only).
- Validate required fields before instances enter any in-memory registry.

## Core types

| Type | Module | Role |
|------|--------|------|
| `AdapterDescriptor` | `fyi_core::adapter` | id, name, author, version, description |
| `AdapterValidation` | `fyi_core::adapter` | `ok` + human messages |
| `CommunityJurisdictionAdapter` | `fyi_core::adapter` | trait: `descriptor`, `to_instance`, `draft_scaffold`, `validate` |
| `StubCommunityAdapter` | `fyi_core::adapter` | documented example (“Pacific FOI”) |
| `register_adapter_instance` | `fyi_core::adapter` | validate many adapters → `Vec<Instance>` or errors |

## Trait contract

```rust
pub trait CommunityJurisdictionAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn to_instance(&self) -> Instance;
    fn draft_scaffold(&self, authority_name: &str) -> String { /* default */ }
    fn validate(&self) -> AdapterValidation { /* default checks */ }
}
```

Default `validate` checks:

- non-empty `instance.id`, `base_url`, `country`, `foi_law.law_name`
- `base_url` starts with `http://` or `https://`

Default `draft_scaffold` produces a short opening paragraph using the law name.

Live Alaveteli HTTP remains on `fyi_core::jurisdiction::FoiProvider` — adapters
only describe **catalog** metadata and local draft scaffolding.

## Example (stub)

```rust
use fyi_core::adapter::{
    CommunityJurisdictionAdapter, StubCommunityAdapter, register_adapter_instance,
};

let adapter = StubCommunityAdapter::example_pacific();
assert!(adapter.validate().ok);
let instance = adapter.to_instance();
assert_eq!(instance.id, "example-pacific-foi");

let registered = register_adapter_instance(&[Box::new(adapter)]).unwrap();
assert_eq!(registered.len(), 1);
assert_eq!(registered[0].status, fyi_core::jurisdiction::InstanceStatus::Community);
```

## Implementing a real community adapter

1. Define a struct holding site URL, locale, `FoiLaw`, and `Capabilities`.
2. Implement `CommunityJurisdictionAdapter::descriptor` and `to_instance`.
3. Optionally override `draft_scaffold` for jurisdiction-specific letter openings.
4. Call `validate` / `register_adapter_instance` before merging into operator config.
5. Gate exposure behind feature flag `community_adapters` (`fyi_core::features`).

### Capability honesty

Set capability bits to match **observed** site behaviour:

| Field | Meaning |
|-------|---------|
| `read` | Public request pages / feeds readable |
| `write` | Programmatic create/update supported |
| `attachments` | File upload on create |
| `batch` | Bulk operations |
| `feeds` | Atom/RSS or similar |
| `search` | Site search usable |
| `prefilled_url` | Deep-link with query prefill |
| `health` | Health/ping endpoint known |

Prefer under-claiming (`write: false`) until integration tests prove otherwise.

## Feature flag

| Flag | String id | Default |
|------|-----------|---------|
| `FeatureFlag::CommunityAdapters` | `community_adapters` | off |

See [README.md](./README.md) for the full experimental vs foundation matrix.

## Non-goals (this phase)

- Dynamic plugin loading (`.so` / WASM).
- Automatic discovery of community adapters from the network.
- Shipping community adapters in the embedded `instances.toml` without review.
- Legal advice embedded in scaffolds.

## Related modules

- `fyi_core::jurisdiction` — `Instance`, `FoiLaw`, `Capabilities`, registry
- `fyi_core::drafting` — uses instance law metadata for prompts
- `fyi_core::federation` — federated summaries can include community instances once registered
- `docs/multi-jurisdiction-security-checklist.md` — threat considerations for new sites
