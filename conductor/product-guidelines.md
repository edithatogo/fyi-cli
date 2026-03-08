# Product Guidelines

## Prose Style & Tone

### Voice
- **Professional yet accessible:** Write clearly and concisely, avoiding unnecessary jargon
- **Transparent:** Be upfront about system capabilities and limitations
- **Privacy-conscious:** Emphasize security and anonymity in all user-facing communications
- **Neutral and objective:** Maintain an impartial tone suitable for research and journalism contexts

### Documentation Standards
- Use active voice for instructions and procedures
- Include code examples for all technical implementations
- Provide context and rationale for architectural decisions
- Maintain changelog for all significant updates

## Branding Principles

### Core Values
1. **Privacy First:** All design decisions prioritize user anonymity and data protection
2. **Transparency:** The system's operations should be auditable and understandable
3. **Reliability:** Consistent performance builds trust with users
4. **Ethical Use:** Promote responsible use of official information processes

### Visual Identity (Future)
- Clean, minimal interface design
- High contrast for accessibility
- Clear status indicators for request states
- Privacy status prominently displayed (TOR active, proxy status, etc.)

## User Experience Principles

### Privacy UX
- **Visible Privacy Indicators:** Always show current anonymity status (TOR/proxy active, IP leak protection)
- **One-Click Privacy:** Make privacy features easy to enable and verify
- **Graceful Degradation:** If TOR/proxy fails, alert user and pause operations safely
- **No Dark Patterns:** Never nudge users toward less private options

### Information Architecture
- **Request-Centric Design:** Organize interface around the request lifecycle
- **Multi-Account Clarity:** Clear visual separation between different accounts
- **Search-First:** Powerful search and filtering as primary navigation method
- **Export-Ready:** Easy export of data in research-friendly formats (CSV, JSON, PDF)

### Interaction Design
- **Batch Operations:** Support bulk actions for efficiency
- **Progressive Disclosure:** Show advanced options only when needed
- **Confirmation for Destructive Actions:** Protect against accidental data loss
- **Keyboard Shortcuts:** Power user features for frequent operations

## Accessibility Guidelines

### WCAG Compliance
- Target WCAG 2.1 AA compliance
- Sufficient color contrast ratios (4.5:1 minimum)
- Keyboard navigable interface
- Screen reader compatible

### Inclusive Design
- Support multiple languages (starting with English, with i18n architecture)
- Timezone-aware date/time display
- Configurable date formats (NZ and international)

## Security Guidelines

### Data Handling
- **Encryption at Rest:** All stored data encrypted using industry-standard algorithms
- **Encryption in Transit:** TLS 1.3 for all network communications
- **Minimal Data Retention:** Store only what's necessary, delete when no longer needed
- **Secure Deletion:** Proper cryptographic erasure for sensitive data

### Authentication & Authorization
- **Credential Isolation:** Never store credentials in plaintext
- **Session Management:** Secure session handling with appropriate timeouts
- **Multi-Factor Support:** Where applicable, support MFA for account access

### API Integration Best Practices
- **Rate Limiting:** Implement client-side rate limiting to respect API constraints
- **Error Handling:** Graceful handling of API errors with retry logic
- **Response Validation:** Validate all API responses before processing
- **Version Awareness:** Handle API versioning and deprecation notices

## Code Quality Standards

### Testing Requirements
- Unit test coverage: >80%
- Integration tests for all API endpoints
- Security testing for privacy features
- Performance testing under load

### Code Review Guidelines
- All changes require peer review
- Security-sensitive changes require additional review
- Document all public APIs and interfaces
- Maintain backward compatibility where possible

## Compliance & Ethics

### Legal Considerations
- Comply with New Zealand Official Information Act
- Respect FYI.org.nz terms of service
- Follow applicable data protection regulations
- Maintain audit trails for accountability

### Ethical Use Policy
- System should not facilitate harassment
- No automated mass-requesting that could burden agencies
- Respect legitimate agency response timelines
- Support responsible journalism and research practices
