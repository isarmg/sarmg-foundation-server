"""Machine-enforced Sarmg Foundation profile and consumer policy."""

from .policy import (
    ConformanceError,
    generate_consumer_matrix,
    load_product_manifest,
    verify_consumer_registry,
    verify_foundation,
    verify_manifest,
    verify_release,
    verify_schema,
    verify_source,
    verify_web,
)

__all__ = [
    "ConformanceError",
    "generate_consumer_matrix",
    "load_product_manifest",
    "verify_consumer_registry",
    "verify_foundation",
    "verify_manifest",
    "verify_release",
    "verify_schema",
    "verify_source",
    "verify_web",
]
