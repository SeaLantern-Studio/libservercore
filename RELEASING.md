# Releasing

This repository is intended for crates.io publication.

## Publish Order

Current publish order matters:

1. `sl-server-core-taxonomy`
2. `sl-server-flavor-core`

`sl-server-flavor-core` depends on `sl-server-core-taxonomy`, so dry-run and publish checks for the flavor crate will fail until the taxonomy crate version is available on crates.io.

## Checklist

1. Update versions in the affected crate manifests.
2. Update `CHANGELOG.md`.
3. Run `cargo test`.
4. Run `cargo package -p <crate> --allow-dirty` for a local package sanity check if needed.
5. Run `cargo publish --dry-run -p sl-server-core-taxonomy`.
6. Publish `sl-server-core-taxonomy`.
7. Wait for crates.io index visibility.
8. Run `cargo publish --dry-run -p sl-server-flavor-core`.
9. Publish `sl-server-flavor-core`.

