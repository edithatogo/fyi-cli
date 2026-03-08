# Product Definition

## Overview

A privacy-focused information retrieval and request management system that leverages the FYI.org.nz API for official information requests, with comprehensive privacy features including proxy/TOR support, multi-account management, and public information aggregation.

---

## Vision

To create a privacy-focused information retrieval and request management system that maximizes the capabilities of the FYI.org.nz API while ensuring user anonymity through proxy and TOR integration.

---

## Target Users

- **Journalists and Researchers:** Individuals who need to make official information requests while maintaining privacy
- **Privacy Advocates:** Users who prioritize anonymity when accessing government information
- **Investigative Professionals:** Users who need to aggregate information from multiple sources securely
- **Civil Society Organizations:** Groups monitoring government transparency and accountability

---

## Core Goals

1. **Privacy-First Communication:** Enable users to interact with FYI.org.nz without exposing their identity or location
2. **Automated Request Management:** Streamline the process of sending, tracking, and receiving official information requests
3. **Multi-Source Intelligence:** Aggregate data from FYI.org.nz, other accounts, and publicly available sources
4. **Secure Data Handling:** Ensure all retrieved information is stored and processed securely

---

## Design Stance

- **Human in the loop** for submission
- **Public web artifacts** treated as source-of-truth hints, not a privileged API
- **Operator clarity** over automation cleverness
- **Every important state transition** should be recoverable from SQLite + snapshots

---

## Key Features

### FYI.org.nz API Integration
- Full API coverage for sending official information requests
- Automated retrieval of request responses and status updates
- Request tracking and history management
- Batch request processing for efficiency

### Privacy & Anonymity
- TOR network integration for anonymous communication
- Configurable proxy support (HTTP, HTTPS, SOCKS)
- IP rotation and request throttling to avoid detection
- No persistent user identification storage

### Multi-Account Management
- Support for multiple FYI.org.nz accounts
- Account switching and rotation
- Consolidated view of requests across accounts
- Account-specific configuration and preferences

### Public Information Aggregation
- Integration with publicly available data sources
- Automated data correlation and linking
- Search and filtering across aggregated data
- Export capabilities for research and reporting

### Security Features
- End-to-end encryption for stored data
- Secure credential management
- Audit logging for compliance
- Data retention policies

---

## Success Metrics

- Number of successful API requests processed
- Request response rate and turnaround time
- User anonymity preservation (no IP leaks)
- Data accuracy and completeness
- System uptime and reliability

---

## Constraints & Considerations

- **Legal Compliance:** Must operate within New Zealand law and FYI.org.nz terms of service
- **Rate Limiting:** Respect API rate limits to avoid service disruption
- **Ethical Use:** System should not be used for harassment or abuse of official information processes
- **Data Privacy:** Handle all retrieved information according to privacy best practices
