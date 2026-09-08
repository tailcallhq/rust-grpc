# Dependency update (#44)

The application now uses Tonic 0.14.6 / Prost 0.14.4, OpenTelemetry 0.32,
tracing-opentelemetry 0.33, tonic-tracing-opentelemetry 0.38, Shuttle 0.57,
and gh-workflow 0.9. Cargo.lock was refreshed with `cargo update`.

- Tonic now owns the HTTP/2 listener; the old Hyper 0.14 server adapter is removed.
- Both v1 and v1alpha reflection are registered, preserving older clients.
- Protobuf code and the reflection descriptor are generated together through
  tonic-prost-build, with tonic-prost providing the generated runtime codec.
- OTLP uses SpanExporter and SdkTracerProvider builders, explicit TLS roots,
  and the same Honeycomb endpoint and team metadata. Shuttle's default tracing
  setup is disabled so it does not install a competing global subscriber.
- Unused direct dependencies were removed: shuttle-axum, hyper, hyper-util,
  http-body-util, tower, prost-types, and opentelemetry-http. gh-workflow is
  needed only as a dev dependency. Any remaining versions in the lockfile are
  transitive requirements rather than application imports.
- The workflow generator uses the new named-step API and normalizes serializer
  whitespace. In CI it verifies the committed workflow instead of rewriting it.

## Reproduce

Install Rust and protoc, then run:

```sh
cargo test --locked --all-features --workspace
cargo +nightly fmt --all -- --check
cargo +nightly clippy --locked --all-features --workspace --all-targets -- -D warnings
CI=true cargo test --locked --test ci
```

Tests start the production Tonic router on an ephemeral loopback socket. A
real generated client exercises CRUD, batch ordering/deduplication, absent IDs,
NotFound status codes, and both reflection protocols, followed by graceful
shutdown. They require neither a Shuttle account nor Honeycomb credentials.

The accompanying MP4 replays the actual local test output. Live Shuttle
deployment and export to Honeycomb were not exercised.
