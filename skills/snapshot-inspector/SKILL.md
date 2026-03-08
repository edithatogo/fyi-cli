# Snapshot Inspector

Use this skill when a tracked FYI request has one or more stored `.json` snapshots and the operator needs a quick view of:
- current described state
- likely attachments
- likely event/message history
- missing data that still needs manual review

## Inputs
- tracked request ID
- latest stored snapshot JSON

## Outputs
- concise operator summary
- attachment list with names/URLs where present
- event list ordered by timestamp when possible
- caveats where FYI JSON structure was ambiguous
