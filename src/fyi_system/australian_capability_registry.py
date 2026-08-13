"""Immutable trust anchors for Australian capability contract validation.

These values are packaged with the validator and are deliberately independent of
the caller-supplied repository root containing candidate contracts and schemas.
"""

from __future__ import annotations

from dataclasses import dataclass
from types import MappingProxyType
from typing import TYPE_CHECKING, Final

if TYPE_CHECKING:
    from collections.abc import Mapping


@dataclass(frozen=True, slots=True)
class AdapterProvenancePin:
    """Externally registered provenance for the inspected capture adapter."""

    repository_revision: str
    module_path: str
    module_sha256: str


PLATFORM_SCHEMA_SHA256: Final = "00172f93df5f38e1358b96ab0df046da59587eba4880d8ad90d0c4393e6786d4"
JURISDICTION_SCHEMA_SHA256: Final = (
    "3f1fbfba541524ba60e1f4c4ad1f2a6b3ab668de83b084b87e50cabc60031d8d"
)

REGISTERED_ADAPTER_PROVENANCE: Final = AdapterProvenancePin(
    repository_revision="2db4813da80d8c61f145c24b22ff4392830bcad5",
    module_path="src/fyi_system/archive_capture.py",
    module_sha256="750074ccf35c9a7ed4df70fbf1ceb53f65a7919d2d8afdeb10047d0029f9937b",
)

REQUIRED_ACTIVATION_PREREQUISITES: Final = (
    "authentic_authority_pin",
    "source_rights_review",
    "effective_date_pin",
    "separate_capture_authorization",
)
REQUIRED_PROHIBITIONS: Final = (
    "endpoint_access",
    "source_retrieval",
    "credential_use",
    "capture",
    "legal_claims",
    "cross_jurisdiction_fallback",
    "implicit_activation",
)

# These contract-only records have no approved source evidence. Any future source
# must first be added to this external registry through a separately reviewed change.
REGISTERED_AUTHENTIC_SOURCE_EVIDENCE: Final[Mapping[str, tuple[str, ...]]] = MappingProxyType(
    {
        "AU-ACT": (),
        "AU-NT": (),
        "AU-QLD": (),
        "AU-SA": (),
        "AU-TAS": (),
        "AU-VIC": (),
        "AU-WA": (),
    },
)
