# Draft: Introduce fyi-cli to the Alaveteli project

**Status:** Ready for maintainer review/posting. Do not auto-post.

**Suggested venue:** GitHub Discussion or Issue on
[mysociety/alaveteli](https://github.com/mysociety/alaveteli) (prefer Discussion if that is the
project’s preference for tooling announcements).

**Suggested title:** `Third-party multi-instance Alaveteli client: fyi-cli (intro + good-citizen defaults)`

---

Hello Alaveteli maintainers and community,

I’m the maintainer of **[fyi-cli](https://github.com/edithatogo/fyi-cli)** — an open-source,
privacy-focused multi-instance FOI/OIA client and MCP server that speaks the Alaveteli-style
HTTP/JSON and public feed surfaces.

### Why I’m writing

1. **Introduction** — fyi-cli started as a local tracker for [FYI.org.nz](https://fyi.org.nz) and
   is expanding carefully to other Alaveteli deployments (AU RightToKnow, UK WhatDoTheyKnow,
   and several community-tier instances). We want to be transparent rather than a silent bulk
   consumer of shared infrastructure.
2. **Good-citizen defaults** — Discovery and archive paths use a contactable User-Agent, check
   robots.txt, back off on `429`/`5xx`, and prefer checkpointed small windows. Live tests are
   opt-in only. Our etiquette notes live in:
   https://github.com/edithatogo/fyi-cli/blob/master/docs/upstream-relations.md
3. **Capability findings** — While building a multi-instance catalog we observed that deployed
   instances differ in which API/feed features are effectively available. If useful, we can
   share a compact matrix and help document a recommended health/capability probe path.
4. **Optional listing** — If Alaveteli maintains (or would welcome) a short list of known
   third-party clients/tools, we would be glad to be included with a one-line description and
   link. No obligation — happy to take a “please don’t list us / please do” either way.

### What fyi-cli is / is not

- **Is:** offline-first request tracker, CLI + MCP tools, optional faithful WARC/WACZ capture,
  Tor/proxy support, multi-instance configuration.
- **Is not:** an official mySociety product, a replacement for the Alaveteli web UI, or a
  justification for unbounded bulk scraping.

Happy to adjust defaults or User-Agent format if you have preferences for third-party clients.
Contact: GitHub issues on https://github.com/edithatogo/fyi-cli or replies on this thread.

Thanks for maintaining Alaveteli and the broader FOI infrastructure.

---

## Optional short note for individual instance operators

**Subject:** Heads-up: multi-instance open-source client (fyi-cli) may use public feeds/API

Hi — just a courtesy note that [fyi-cli](https://github.com/edithatogo/fyi-cli) can talk to your
Alaveteli instance via documented public JSON/feeds for personal request tracking and optional
archival capture. Defaults aim to be polite (User-Agent, robots.txt, backoff). If you have
preferred rate limits, User-Agent format, or would rather we keep your instance out of the
default catalog, please say so and we will honour it.
