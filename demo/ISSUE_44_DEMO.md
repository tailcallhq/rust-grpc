# Issue #44 Demo (Dependency Update)

This PR updates outdated dependencies and verifies the project still builds/tests.

Validation commands:

```bash
cargo update
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

See `issue-44-demo.gif` for a short visual walkthrough.
