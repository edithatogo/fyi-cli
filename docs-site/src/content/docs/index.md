---
title: FYI CLI Documentation
description: Privacy-focused official information request and management tool.
template: splash
hero:
  tagline: A secure, privacy-first cli tool and daemon for managing Alaveteli-based official information requests (FYI.org.nz).
  image:
    file: ../../assets/hero-icon.svg
  actions:
    - text: Read the Guides
      link: /guides/security/
      icon: right-arrow
      variant: primary
    - text: Rust Core Migration
      link: /guides/rust-migration/
      icon: external
---

## Key Features

- **Tor Network Integration**: Route all client API requests natively through Tor (`arti`) for absolute privacy.
- **Secure Keyring**: Never store API credentials in plaintext. Integrated directly with native system keyring services.
- **Zeroize Memory Scrubbing**: Cryptographically clean security credentials from RAM when not active.
- **Model Context Protocol**: Seamlessly interface with LLM developer workflows via a native MCP JSON-RPC server.
- **Terminal UI Dashboard**: Rich multi-tab visualization for tracking official request lifecycles.
