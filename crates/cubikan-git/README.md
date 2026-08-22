# cubikan-git

`cubikan-git` resolves one caller-supplied, full Git commit object ID into the
provider-neutral CubiKan reference tuple
`(git.commit.sha1|git.commit.sha256, scope, object-id)`.

The adapter deliberately exposes only `GitCommitResolver`,
`GitReferenceError`, and the `cubikan-core::ExternalReference` returned by
`resolve_commit`. It does not expose repository paths, Git output, configuration,
credentials, source content, blame, author, committer, signer, object-database
handles, or process authority. A commit reference is correlation evidence only;
it is not authorship, causality, verification, satisfaction, or provider
attestation.

Resolution is read-only and fail-closed. One canonical absolute Git executable
is used for every bounded argv-only process. Git must be at least 2.45 and support
the global `--no-lazy-fetch` option and `rev-parse --show-object-format`. The
repository must be its exact canonical worktree root with an ordinary contained
`.git/objects` store. Alternate, replacement, promisor, partial-clone, dangerous
configuration, inherited authority, noncommit, abbreviated, normalized, and
wrong-algorithm inputs are rejected without fetching or invoking a shell.

The Sprint 11 local-chain journey owns finalized submission and verified SQLite
rebuild evidence. This crate never simulates either authority.
