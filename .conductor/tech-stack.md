# Technology Stack

## Overview

This project supports two technology stacks:
1. **Rust Stack (Active)**: The current primary implementation under active development/migration
2. **Python Stack (Legacy)**: The initial, production-ready implementation being replaced

---

## Rust Stack (Active)

The active stack uses Rust for enhanced performance, memory safety, and single-binary deployment.

### Core Language
**Python 3.10+**
- Rapid development and prototyping
- Excellent library ecosystem for API integration
- Strong support for data processing and automation

### Dependencies
- **feedparser>=6.0.11**: RSS/Atom feed parsing
- **requests>=2.32.0**: HTTP client for API calls
- **jinja2>=3.1.4**: Template engine for document generation

### Data Storage
**SQLite**
- File-based, no server required
- ACID compliance
- Easy backup and portability

### Quality Gates
- **pytest>=8.0.0**: Testing framework
- **pytest-cov>=5.0.0**: Coverage reporting
- **ruff>=0.6.0**: Fast Python linter
- **mypy>=1.11.0**: Static type checking

### Project Structure
```
src/fyi_system/          # Main Python package
tests/                   # Test suite
pyproject.toml           # Project configuration
```

### CLI Entry Point
```bash
fyi-system = "fyi_system.cli:main"
```

---

## Python Stack (Legacy)

The legacy implementation uses Python for rapid development and deployment.

### Core Language
**Rust (Latest Stable)**
- Memory safety without garbage collection
- Zero-cost abstractions for high performance
- Single binary deployment (no runtime dependencies)
- Excellent for system-level operations (TOR, proxy, networking)

### CLI Framework
**Clap 4+**
- Industry-standard Rust CLI framework
- Derive macros for declarative CLI definition
- Automatic help message generation
- Shell completion generation (bash, zsh, fish, PowerShell)

### Async Runtime
**Tokio**
- High-performance async runtime
- Multi-threaded scheduler for concurrent operations
- Non-blocking I/O for API requests and network operations

### HTTP Client
**reqwest**
- Modern async HTTP client
- Built-in connection pooling
- Excellent proxy support (HTTP, HTTPS, SOCKS)
- HTTP/2 support

### TOR Integration
**Stem (via subprocess) OR arti (native Rust)**
- **Option 1: Stem** - Mature Python library, call via subprocess
- **Option 2: arti** - Native Rust TOR implementation (growing, but less mature)
- Custom TOR Circuit Manager for circuit rotation and identity management

### Serialization
**serde + serde_json**
- Fast serialization/deserialization
- Type-safe JSON handling
- Support for multiple formats (JSON, TOML, YAML)

### Configuration
**config + dotenvy**
- Multi-format configuration (TOML, YAML, JSON, env vars)
- .env file support
- Hierarchical configuration

### Data Storage
**SQLite with sqlx OR rusqlite**
- **sqlx:** Async SQL with compile-time query verification
- **rusqlite:** Synchronous, simpler, well-tested
- File-based storage (no server required)

### Error Handling
**thiserror + anyhow**
- Clean error enums with thiserror
- Application-level error handling with anyhow
- Context-rich error messages

### Logging
**tracing + tracing-subscriber**
- Structured logging with spans
- Async-aware tracing
- Multiple output formats (JSON, pretty)

### Security
**rustls + ring + zeroize**
- **rustls:** Pure Rust TLS implementation
- **ring:** Cryptographic primitives
- **zeroize:** Secure memory clearing for sensitive data

### CLI Output
**indicatif + dialoguer + comfy-table**
- Progress bars and spinners
- Interactive prompts and confirmations
- Beautiful table output

### Development Tools
- **rustfmt:** Official Rust formatter
- **clippy:** Rust linter
- **cargo-audit:** Security vulnerability scanning

### Build & Package Management
**Cargo**
- Built-in package manager and build system
- Workspace support for multi-crate projects
- Feature flags for optional functionality

### Deployment
**Docker + Docker Compose**
- Containerized deployment
- Multi-stage builds for minimal image size
- cargo-chef for Docker layer caching

---

## Architecture Pattern

**Clean Architecture / Hexagonal Architecture**
- Clear separation of concerns
- Domain logic isolated from infrastructure
- Easy to test and maintain
- API adapters for FYI.org.nz and other sources

### Project Structure (Rust)
```
.
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── cli/                 # CLI commands and subcommands
│   ├── core/                # Domain logic, entities, value objects
│   ├── adapters/            # External API integrations
│   │   ├── fyi/            # FYI.org.nz API client
│   │   └── public/         # Public data source adapters
│   ├── infrastructure/      # Database, caching, TOR management
│   │   ├── tor/
│   │   ├── proxy/
│   │   └── db/
│   ├── mcp/                 # MCP server implementation
│   │   ├── tools.rs
│   │   └── resources.rs
│   └── config/              # Configuration and settings
├── tests/                   # Integration tests
└── docs/                    # Documentation
```

---

## Key Architectural Decisions

### Async-First Design
- All I/O operations are async (Python: asyncio, Rust: Tokio)
- Concurrent API requests for performance
- Non-blocking TOR circuit management

### Privacy-by-Design
- TOR integration at the network layer
- Proxy abstraction for flexibility
- Zero-knowledge architecture where possible
- Secure memory handling for credentials (Rust)

### CLI-First with MCP Server
- Primary interface is CLI
- MCP server mode for AI assistant integration
- Both modes share the same core logic

### Modular Structure
- Clean separation between CLI, MCP, and core logic
- Easy to extract FYI.org.nz client as separate crate/package
- Plugin architecture for additional data sources

---

## Future Considerations

### Potential Crate Split (Rust)
- **fyi-client:** Generic FYI.org.nz API client library
- **fyi-mcp:** MCP server implementation
- **fyi-cli:** CLI application

This allows:
- Reuse of the API client in other projects
- Independent versioning
- Clearer API boundaries

### Scaling Options
- **PostgreSQL:** If SQLite becomes a bottleneck
- **Redis:** For caching and session management
- **Message Queue:** NATS or Redis streams for event-driven architecture

---

## Migration Path (Python → Rust)

1. **Phase 1**: Extract FYI.org.nz API client as separate module
2. **Phase 2**: Implement Rust CLI skeleton with Clap
3. **Phase 3**: Port core domain logic to Rust
4. **Phase 4**: Implement MCP server in Rust
5. **Phase 5**: Add TOR/proxy support with arti
6. **Phase 6**: Full migration and deprecation of Python version
