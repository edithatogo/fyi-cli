# Technology Stack

## Core Language
**Rust (Latest Stable)**
- Memory safety without garbage collection
- Zero-cost abstractions for high performance
- Single binary deployment (no runtime dependencies)
- Excellent for system-level operations (TOR, proxy, networking)
- Strong type system catches errors at compile time

## CLI Framework
**Clap 4+**
- Industry-standard Rust CLI framework
- Derive macros for declarative CLI definition
- Automatic help message generation
- Subcommand support for complex CLI structures
- Shell completion generation (bash, zsh, fish, PowerShell)

## Async Runtime
**Tokio**
- High-performance async runtime
- Multi-threaded scheduler for concurrent operations
- Excellent ecosystem integration
- Non-blocking I/O for API requests and network operations

## HTTP Client
**reqwest**
- Modern async HTTP client
- Built-in connection pooling
- Excellent proxy support (HTTP, HTTPS, SOCKS)
- HTTP/2 support
- Type-safe request/response handling

## TOR Integration
**Stem (via Python subprocess) OR arti (native Rust)**
- **Option 1: Stem** - Mature Python library, call via subprocess
- **Option 2: arti** - Native Rust TOR implementation (growing, but less mature)
- **Custom TOR Circuit Manager:** Built in-house for circuit rotation and identity management
- **Proxy Chain Support:** Multiple proxy providers with automatic failover

## Serialization
**serde + serde_json**
- Fast serialization/deserialization
- Type-safe JSON handling
- Derive macros for boilerplate reduction
- Support for multiple formats (JSON, TOML, YAML)

## Configuration
**config + dotenvy**
- Multi-format configuration (TOML, YAML, JSON, env vars)
- .env file support
- Hierarchical configuration
- Type-safe configuration structs

## Data Storage
**SQLite with sqlx OR rusqlite**
- **sqlx:** Async SQL with compile-time query verification
- **rusqlite:** Synchronous, simpler, well-tested
- File-based storage (no server required)
- ACID compliance for data integrity
- Easy migration to PostgreSQL if scaling needed

## Error Handling
**thiserror + anyhow**
- **thiserror:** For library error types (clean error enums)
- **anyhow:** For application-level error handling
- Context-rich error messages
- Easy error propagation with `?` operator

## Logging
**tracing + tracing-subscriber**
- Structured logging with spans
- Async-aware tracing
- Multiple output formats (JSON, pretty)
- Log levels and filtering
- Performance profiling capabilities

## Testing
**Built-in Rust test framework + tokio-test**
- Unit tests with `#[test]` attribute
- Integration tests in `tests/` directory
- Async test support with tokio-test
- Mocking with mockall or wiremock for HTTP

## Security
**ring OR rustls**
- **rustls:** Pure Rust TLS implementation (preferred for security audits)
- **ring:** Cryptographic primitives
- Secure credential storage
- Encryption at rest for sensitive data

**zeroize**
- Secure memory clearing for sensitive data
- Prevents credential leakage

## CLI Output
**indicatif + dialoguer**
- **indicatif:** Progress bars and spinners
- **dialoguer:** Interactive prompts and confirmations
- **comfy-table:** Beautiful table output

## Documentation
**rustdoc**
- Built-in documentation generation
- Markdown support in doc comments
- `cargo doc` for HTML documentation

## Development Tools
**rustfmt**
- Official Rust formatter
- Consistent code style

**clippy**
- Rust linter
- Catches common mistakes and improves code quality

**cargo-audit**
- Security vulnerability scanning for dependencies
- Regular dependency auditing

## MCP Server Framework
**MCP Rust SDK** (or custom implementation)
- Model Context Protocol server implementation
- Tool definitions and handlers
- Resource and prompt management

## Build & Package Management
**Cargo**
- Built-in package manager and build system
- Workspace support for multi-crate projects
- Feature flags for optional functionality

## Deployment & Containerization
**Docker + Docker Compose**
- Containerized deployment
- Multi-stage builds for minimal image size
- TOR and proxy configuration in containers

**cargo-chef**
- Docker layer caching for faster builds

## Additional Crates
- **tokio-retry** or **backoff:** Retry logic with exponential backoff
- **uuid:** Unique identifier generation
- **chrono** or **time:** Date/time handling
- **regex:** Pattern matching
- **url:** URL parsing and manipulation
- **base64:** Encoding/decoding
- **hex:** Hex encoding/decoding
- **sha2:** SHA-2 hash functions
- **hmac:** HMAC for message authentication

## Architecture Pattern
**Clean Architecture / Hexagonal Architecture**
- Clear separation of concerns
- Domain logic isolated from infrastructure
- Easy to test and maintain
- API adapters for FYI.org.nz and other sources

## Project Structure
```
.
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── cli/                 # CLI commands and subcommands
│   │   ├── mod.rs
│   │   ├── commands/        # Command implementations
│   │   └── args.rs          # Clap argument definitions
│   ├── core/                # Domain logic, entities, value objects
│   │   ├── mod.rs
│   │   ├── entities/
│   │   └── services/
│   ├── adapters/            # External API integrations
│   │   ├── mod.rs
│   │   ├── fyi/            # FYI.org.nz API client
│   │   └── public/         # Public data source adapters
│   ├── infrastructure/      # Database, caching, TOR management
│   │   ├── mod.rs
│   │   ├── tor/
│   │   ├── proxy/
│   │   └── db/
│   ├── mcp/                 # MCP server implementation
│   │   ├── mod.rs
│   │   ├── tools.rs
│   │   └── resources.rs
│   └── config/              # Configuration and settings
│       └── mod.rs
├── tests/                   # Integration tests
└── docs/                    # Documentation
```

## Key Architectural Decisions

### Async-First Design
- All I/O operations use tokio async runtime
- Concurrent API requests for performance
- Non-blocking TOR circuit management

### Privacy-by-Design
- TOR integration at the network layer
- Proxy abstraction for flexibility
- Zero-knowledge architecture where possible
- Secure memory handling for credentials

### CLI-First with MCP Server
- Primary interface is CLI (Clap)
- MCP server mode for AI assistant integration
- Both modes share the same core logic

### Modular Structure
- Clean separation between CLI, MCP, and core logic
- Easy to extract FYI.org.nz client as separate crate
- Plugin architecture for additional data sources

## Future Considerations

### Potential Crate Split
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
