# CubiKan foundation snapshot v1

This fixture freezes the T-1101 dependency and build boundary so later source
work cannot rewrite the foundation proof. `inventory.toml` is the authoritative,
declarative reconstruction map. Its `source` paths are relative to this
directory; its `target` paths are relative to a new, empty materialization root.

The canonical payload contains the exact root and chain workspace manifests and
lockfiles, both Rust toolchain files, root ignore and automation configuration,
the reviewed rusqlite patch, every root member manifest, and the complete pallet
and runtime source present at the T-1101 boundary. The pallet snapshot includes
the predeclared T-1102 `types`, `conformance`, and model-test modules together
with the `serde` and `serde_json` development dependencies.

Files below `support/root-stubs/` are intentionally tiny build probes, not
product-source snapshots. They make the exact root manifests and lockfile
independently usable for offline `cargo tree` and `cargo check` after the live
root crates evolve. Chain sources are canonical snapshot bytes and support the
native check, release build, and compressed Wasm identity proof.

`payload/chain/runtime/proof/` contains the 288-byte compressed Wasm that the
snapshot build produced. It is an immutable reference subject only: the
verifier deletes any materialized build output, rebuilds into a fresh target
directory, and compares the new artifact to these recorded bytes and digest.

No payload file is literally named `Cargo.toml` or `Cargo.lock`. A consumer
materializes those names only in its temporary output directory according to the
inventory. For each `[[files]]` entry it must verify `bytes` and `sha256` before
copying `source` to `target`. It must also verify and copy each `[[trees]]` entry;
the rusqlite tree is repository-owned and separately reconstructed from its
pinned registry archive and patch.

After materialization, `Cargo.toml` and `chain/Cargo.toml` are separate workspace
roots with separate lockfiles. Run the declarative proof commands recorded in
`inventory.toml` with the pinned Cargo home, `CARGO_NET_OFFLINE=true`, and an
empty task-owned target directory. Normalize only the temporary materialization
prefix to `<PROJECT>` before hashing feature-tree output.

The inventory deliberately does not hash itself. Its complete bytes are pinned
by the repository verifier, while every other regular file in this fixture is
listed inside it.
