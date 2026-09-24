# Dependency refresh for #44

This update moves the service to the current dependency generations used by the Rust gRPC and OpenTelemetry ecosystem.

Highlights:
- Tonic / tonic-reflection / tonic-prost: 0.14.6
- Prost: 0.14.4
- OpenTelemetry stack: 0.32.x
- tracing-opentelemetry: 0.33.0
- tonic-tracing-opentelemetry: 0.38.0
- Shuttle runtime: 0.57.0
- gh-workflow: 0.9.0
- protoc is vendored for reproducible clean-machine builds
- the legacy Hyper 0.14 + Tower server adapter was removed in favor of Tonic's current transport API

Regression coverage starts the production Tonic router on an ephemeral loopback port and verifies CRUD, batch semantics, NotFound behavior, reflection v1/v1alpha, and graceful shutdown.

Validation target:

```sh
cargo test --all-features --workspace
cargo +nightly fmt --check
cargo +nightly clippy --all-features --workspace -- -D warnings
```

No Honeycomb credentials are required for the tests.
