#!/usr/bin/env python3
"""Validate bounded Sprint 10 provider/action evidence without network access."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from copy import deepcopy
from datetime import datetime
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit


EXPECTED_ACCOUNT = "crussella0129"
EXPECTED_REPOSITORY = "crussella0129/CubiKan"
EXPECTED_BRANCH = "dev"
DERIVATIVE_SLUGS = {
    "animus-ledger",
    "cubikan-agent-ops",
    "cubikan-observatory",
    "cubikan-org-app-kit",
    "cubikan-process-studio",
    "cubikan-skill-graph",
}
REPOSITORY_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
GITHUB_API_ROOT = "https://api.github.com"
READ_ONLY_ACTION_KINDS = frozenset(
    {
        "git.remote.read",
        "git.history.read",
        "github.public_rest.repository.read",
        "github.public_rest.published_releases.read",
        "github.public_rest.deployments.read",
        "github.connector.repository.search",
        "github.connector.repositories.list",
    }
)
MUTATING_ACTION_KINDS = frozenset({"git.push"})
ACTION_KINDS = READ_ONLY_ACTION_KINDS | MUTATING_ACTION_KINDS


class EvidenceError(ValueError):
    """Raised when a bounded audit artifact is incomplete or unsafe."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def require_complete(data: dict[str, Any], field: str) -> None:
    require(data.get(field) is True, f"{field} must be the boolean true")


def parse_timestamp(value: Any, context: str) -> datetime:
    require(isinstance(value, str) and value, f"{context} must be a timestamp string")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise EvidenceError(f"{context} must be an ISO-8601 timestamp") from error
    require(parsed.tzinfo is not None, f"{context} must include an explicit UTC offset")
    return parsed


def require_observed_at(
    record: dict[str, Any], window: tuple[datetime, datetime], context: str
) -> None:
    observed_at = parse_timestamp(record.get("observed_at"), f"{context}.observed_at")
    require(
        window[0] <= observed_at <= window[1],
        f"{context}.observed_at falls outside observation_window",
    )


def git_commit_exists(root: Path, commit: str) -> bool:
    return (
        subprocess.run(
            ["git", "-C", str(root), "cat-file", "-e", f"{commit}^{{commit}}"],
            check=False,
            capture_output=True,
        ).returncode
        == 0
    )


def git_is_ancestor(root: Path, ancestor: str, descendant: str) -> bool:
    return (
        subprocess.run(
            ["git", "-C", str(root), "merge-base", "--is-ancestor", ancestor, descendant],
            check=False,
            capture_output=True,
        ).returncode
        == 0
    )


def repository_name(value: Any, context: str) -> str:
    require(isinstance(value, str), f"{context} must be a repository name string")
    require(bool(REPOSITORY_RE.fullmatch(value)), f"{context} is not owner/name: {value}")
    return value


def is_derivative_repository(value: str) -> bool:
    parts = value.casefold().split("/", 1)
    return len(parts) == 2 and parts[1] in DERIVATIVE_SLUGS


def repository_from_target(value: str) -> str | None:
    """Normalize owner/name, GitHub URLs, and SCP-like GitHub targets."""
    candidate = value.strip()
    if candidate.startswith("git@github.com:"):
        candidate = candidate.removeprefix("git@github.com:")
    elif "://" in candidate:
        parsed = urlsplit(candidate)
        if parsed.hostname is None or parsed.hostname.casefold() != "github.com":
            return None
        candidate = parsed.path.lstrip("/")
    if "@" in candidate and candidate.count("/") == 1:
        candidate = candidate.split("@", 1)[0]
    candidate = candidate.split("#", 1)[0].split("?", 1)[0].removesuffix(".git")
    return candidate if REPOSITORY_RE.fullmatch(candidate) else None


def derivative_slug_in_target(value: str) -> str | None:
    normalized = repository_from_target(value)
    if normalized and is_derivative_repository(normalized):
        return normalized.casefold().split("/", 1)[1]
    candidate = value.casefold().strip().removesuffix(".git")
    for slug in DERIVATIVE_SLUGS:
        if candidate == slug or re.search(
            rf"(?:^|[/@:]){re.escape(slug)}(?:$|[/?#@])", candidate
        ):
            return slug
    return None


def validate_public_rest_observation(
    record: Any,
    expected_endpoint: str,
    expected_status: int,
    window: tuple[datetime, datetime],
    context: str,
) -> None:
    require(isinstance(record, dict), f"{context} must be a typed observation object")
    require(
        record.get("provider") == "github-public-rest",
        f"{context}.provider must be github-public-rest",
    )
    require(record.get("method") == "GET", f"{context}.method must be GET")
    require(
        record.get("endpoint") == expected_endpoint,
        f"{context}.endpoint must be {expected_endpoint}",
    )
    status = record.get("http_status")
    require(
        type(status) is int and status == expected_status,
        f"{context}.http_status must be {expected_status}",
    )
    require_observed_at(record, window, context)


def validate_connector_observation(
    record: Any,
    repository: str,
    window: tuple[datetime, datetime],
    context: str,
) -> None:
    require(isinstance(record, dict), f"{context} must be a typed observation object")
    require(
        record.get("provider") == "github-connected-app",
        f"{context}.provider must be github-connected-app",
    )
    require(
        record.get("scope") == "connected-installation-only",
        f"{context}.scope must be connected-installation-only",
    )
    require(
        type(record.get("result_count")) is int and record["result_count"] == 0,
        f"{context}.result_count must be the integer 0",
    )
    query = record.get("query")
    source_id = record.get("source_id")
    slug = repository.split("/", 1)[1]
    expected_query = f"{slug} user:{EXPECTED_ACCOUNT} in:name"
    require(
        query == expected_query,
        f"{context}.query must be {expected_query}",
    )
    if source_id is not None:
        require(
            isinstance(source_id, str) and source_id.strip(),
            f"{context}.source_id must be non-empty when present",
        )
    require_observed_at(record, window, context)


def validate_connector_list_limitation(
    record: Any, window: tuple[datetime, datetime]
) -> None:
    context = "connector_repository_list_observation"
    require(isinstance(record, dict), f"{context} must be a typed observation object")
    require(
        record.get("provider") == "github-connected-app",
        f"{context}.provider must be github-connected-app",
    )
    require(
        record.get("scope") == "connected-installation-only",
        f"{context}.scope must be connected-installation-only",
    )
    require(
        type(record.get("result_count")) is int and record["result_count"] == 0,
        f"{context}.result_count must be the integer 0",
    )
    require(
        record.get("complete_account_inventory") is False,
        f"{context}.complete_account_inventory must be false",
    )
    source_id = record.get("source_id")
    query = record.get("query")
    require(
        (isinstance(source_id, str) and source_id.strip())
        or (isinstance(query, str) and query.strip()),
        f"{context} must include its source_id or exact query",
    )
    require_observed_at(record, window, context)


def validate_collection_observation(
    record: Any,
    expected_endpoint: str,
    window: tuple[datetime, datetime],
    context: str,
) -> list[dict[str, Any]]:
    validate_public_rest_observation(record, expected_endpoint, 200, window, context)
    require(record.get("pagination_complete") is True, f"{context}.pagination_complete must be true")
    results = record.get("results")
    require(isinstance(results, list), f"{context}.results must be a list")
    result_count = record.get("result_count")
    require(
        type(result_count) is int and result_count == len(results),
        f"{context}.result_count must equal the results list length",
    )
    return results


def validate_inventory(data: dict[str, Any]) -> tuple[int, int, int]:
    observation_window = data.get("observation_window")
    require(isinstance(observation_window, dict), "observation_window must be an object")
    window = (
        parse_timestamp(observation_window.get("started_at"), "observation_window.started_at"),
        parse_timestamp(
            observation_window.get("completed_at"), "observation_window.completed_at"
        ),
    )
    require(window[0] <= window[1], "observation_window must not run backwards")
    captured_at = parse_timestamp(data.get("captured_at"), "captured_at")
    require(
        window[0] <= captured_at <= window[1],
        "captured_at falls outside observation_window",
    )
    require(data.get("github_account") == EXPECTED_ACCOUNT, "unexpected GitHub account")
    scope = data.get("repository_check_scope")
    require(isinstance(scope, str) and scope.strip(), "repository_check_scope is required")
    folded_scope = scope.casefold()
    require("bounded" in folded_scope, "repository_check_scope must describe its bounded scope")
    require(
        "public" in folded_scope or "connected" in folded_scope,
        "repository_check_scope must identify public or connected evidence",
    )
    require(
        "complete account" not in folded_scope and "all account" not in folded_scope,
        "repository_check_scope must not claim a complete account inventory",
    )
    offline_limitation = data.get("offline_validator_limitation")
    require(
        isinstance(offline_limitation, str) and offline_limitation.strip(),
        "offline_validator_limitation is required",
    )
    folded_limitation = offline_limitation.casefold()
    require("offline" in folded_limitation, "offline_validator_limitation must name offline validation")
    require(
        "does not call" in folded_limitation or "no provider calls" in folded_limitation,
        "offline_validator_limitation must state that no provider is called",
    )
    require_complete(data, "derivative_repository_checks_complete")

    cubikan_repository = data.get("cubikan_repository")
    require(
        isinstance(cubikan_repository, dict), "cubikan_repository must be an identity object"
    )
    cubikan_name = repository_name(
        cubikan_repository.get("name_with_owner"), "cubikan_repository.name_with_owner"
    )
    require(
        cubikan_name.casefold() == EXPECTED_REPOSITORY.casefold(),
        f"unexpected CubiKan repository identity: {cubikan_name}",
    )
    require(cubikan_repository.get("exists") is True, "CubiKan repository must exist")
    validate_public_rest_observation(
        cubikan_repository.get("public_rest_observation"),
        f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}",
        200,
        window,
        "cubikan_repository.public_rest_observation",
    )

    repository_checks = data.get("derivative_repository_checks")
    require(
        isinstance(repository_checks, list),
        "derivative_repository_checks must be a bounded targeted list",
    )
    expected_names = {
        f"{EXPECTED_ACCOUNT}/{slug}".casefold() for slug in DERIVATIVE_SLUGS
    }
    checked_names: set[str] = set()
    for index, record in enumerate(repository_checks):
        require(
            isinstance(record, dict), f"derivative_repository_checks[{index}] must be an object"
        )
        name = repository_name(
            record.get("name_with_owner"),
            f"derivative_repository_checks[{index}].name_with_owner",
        )
        folded = name.casefold()
        require(
            folded in expected_names,
            f"derivative_repository_checks[{index}] is not a recommended slug: {name}",
        )
        require(folded not in checked_names, f"duplicate targeted repository check: {name}")
        checked_names.add(folded)
        require(
            record.get("found_in_observed_scopes") is False,
            f"targeted repository check found the repository in an observed scope: {name}",
        )
        validate_public_rest_observation(
            record.get("public_rest_observation"),
            f"{GITHUB_API_ROOT}/repos/{name}",
            404,
            window,
            f"derivative_repository_checks[{index}].public_rest_observation",
        )
        validate_connector_observation(
            record.get("authenticated_connector_observation"),
            name,
            window,
            f"derivative_repository_checks[{index}].authenticated_connector_observation",
        )
    require(
        checked_names == expected_names,
        "targeted repository checks must cover each of the exact six recommended slugs once",
    )

    validate_connector_list_limitation(
        data.get("connector_repository_list_observation"), window
    )
    publication_scope = repository_name(
        data.get("publication_scope_repository"), "publication_scope_repository"
    )
    require(
        publication_scope.casefold() == EXPECTED_REPOSITORY.casefold(),
        "release/deployment inventory must be explicitly CubiKan-scoped",
    )
    releases = validate_collection_observation(
        data.get("published_release_observation"),
        f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}/releases?per_page=100",
        window,
        "published_release_observation",
    )
    deployments = validate_collection_observation(
        data.get("deployment_observation"),
        f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}/deployments?per_page=100",
        window,
        "deployment_observation",
    )
    for collection_name, collection in (
        ("published_releases", releases),
        ("deployments", deployments),
    ):
        for index, record in enumerate(collection):
            require(isinstance(record, dict), f"{collection_name}[{index}] must be an object")
            target = repository_name(
                record.get("repository"), f"{collection_name}[{index}].repository"
            )
            require(
                target.casefold() == EXPECTED_REPOSITORY.casefold(),
                f"{collection_name}[{index}] targets non-CubiKan repository: {target}",
            )
            require(
                isinstance(record.get("identifier"), str) and record["identifier"],
                f"{collection_name}[{index}].identifier must be non-empty",
            )
    return len(repository_checks), len(releases), len(deployments)


def validate_actions(
    data: dict[str, Any], root: Path | None = None, tested_head: str | None = None
) -> tuple[int, int]:
    require_complete(data, "action_ledger_complete")
    require(data.get("action_scope") == "sprint-10", "action_scope must be sprint-10")
    actions = data.get("actions")
    require(isinstance(actions, list) and actions, "actions must be a non-empty complete list")

    mutations = 0
    mutation_heads: list[str] = []
    for index, action in enumerate(actions):
        require(isinstance(action, dict), f"actions[{index}] must be an object")
        kind = action.get("kind")
        target = action.get("target")
        mutation = action.get("mutation")
        require(isinstance(kind, str) and kind, f"actions[{index}].kind must be non-empty")
        require(kind in ACTION_KINDS, f"actions[{index}].kind is not allowed: {kind}")
        require(isinstance(target, str) and target, f"actions[{index}].target must be non-empty")
        require(isinstance(mutation, bool), f"actions[{index}].mutation must be boolean")

        target_repository = repository_from_target(target)
        expected_mutation = kind in MUTATING_ACTION_KINDS
        derivative_slug = derivative_slug_in_target(target)
        require(
            mutation is expected_mutation,
            f"actions[{index}] mutation must be {str(expected_mutation).lower()} for {kind}",
        )
        if derivative_slug is not None:
            require(
                not mutation,
                f"actions[{index}] records a derivative mutation for {derivative_slug}: {kind} {target}",
            )
        if not mutation:
            continue

        mutations += 1
        require(
            target_repository is not None,
            f"actions[{index}] mutation target must resolve to a GitHub owner/name",
        )
        require(
            target_repository.casefold() == EXPECTED_REPOSITORY.casefold(),
            f"actions[{index}] mutation targets non-CubiKan repository: {target}",
        )
        require(
            action.get("branch") == EXPECTED_BRANCH,
            f"actions[{index}] mutation must target branch {EXPECTED_BRANCH}",
        )
        head = action.get("head")
        require(
            isinstance(head, str) and COMMIT_RE.fullmatch(head) is not None,
            f"actions[{index}].head must be a full lowercase 40-hex commit",
        )
        mutation_heads.append(head)
        if root is not None:
            require(
                git_commit_exists(root, head),
                f"actions[{index}].head does not resolve to a commit: {head}",
            )
            require(
                tested_head is not None and git_is_ancestor(root, head, tested_head),
                f"actions[{index}].head is not on the tested candidate history: {head}",
            )
            if len(mutation_heads) > 1:
                require(
                    git_is_ancestor(root, mutation_heads[-2], head),
                    f"actions[{index}].head is out of push order: {head}",
                )
    if tested_head is not None:
        require(bool(mutation_heads), "action ledger must record the tested candidate push")
        require(
            mutation_heads[-1] == tested_head,
            "final recorded push head must equal tested_head",
        )
    return len(actions), mutations


def valid_inventory_fixture() -> dict[str, Any]:
    observed_at = "2026-08-11T19:30:00Z"
    checks = []
    for slug in sorted(DERIVATIVE_SLUGS):
        name = f"{EXPECTED_ACCOUNT}/{slug}"
        checks.append(
            {
                "name_with_owner": name,
                "found_in_observed_scopes": False,
                "public_rest_observation": {
                    "provider": "github-public-rest",
                    "method": "GET",
                    "endpoint": f"{GITHUB_API_ROOT}/repos/{name}",
                    "http_status": 404,
                    "observed_at": observed_at,
                },
                "authenticated_connector_observation": {
                    "provider": "github-connected-app",
                    "scope": "connected-installation-only",
                    "query": f"{slug} user:{EXPECTED_ACCOUNT} in:name",
                    "result_count": 0,
                    "observed_at": observed_at,
                },
            }
        )
    return {
        "captured_at": observed_at,
        "observation_window": {
            "started_at": "2026-08-11T19:00:00Z",
            "completed_at": "2026-08-11T20:00:00Z",
        },
        "github_account": EXPECTED_ACCOUNT,
        "repository_check_scope": "bounded targeted public REST and connected checks",
        "offline_validator_limitation": (
            "offline shape and consistency validation only; it does not call a provider"
        ),
        "derivative_repository_checks_complete": True,
        "cubikan_repository": {
            "name_with_owner": EXPECTED_REPOSITORY,
            "exists": True,
            "public_rest_observation": {
                "provider": "github-public-rest",
                "method": "GET",
                "endpoint": f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}",
                "http_status": 200,
                "observed_at": observed_at,
            },
        },
        "derivative_repository_checks": checks,
        "connector_repository_list_observation": {
            "provider": "github-connected-app",
            "scope": "connected-installation-only",
            "query": "list connected repositories",
            "result_count": 0,
            "complete_account_inventory": False,
            "observed_at": observed_at,
        },
        "publication_scope_repository": EXPECTED_REPOSITORY,
        "published_release_observation": {
            "provider": "github-public-rest",
            "method": "GET",
            "endpoint": f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}/releases?per_page=100",
            "http_status": 200,
            "pagination_complete": True,
            "result_count": 0,
            "results": [],
            "observed_at": observed_at,
        },
        "deployment_observation": {
            "provider": "github-public-rest",
            "method": "GET",
            "endpoint": f"{GITHUB_API_ROOT}/repos/{EXPECTED_REPOSITORY}/deployments?per_page=100",
            "http_status": 200,
            "pagination_complete": True,
            "result_count": 0,
            "results": [],
            "observed_at": observed_at,
        },
    }


def self_test() -> int:
    invalid_actions = [
        {
            "kind": "repository_creation",
            "target": "cubikan-agent-ops",
            "mutation": False,
        },
        {
            "kind": "push",
            "target": EXPECTED_REPOSITORY,
            "branch": EXPECTED_BRANCH,
            "mutation": True,
        },
        {
            "kind": "git.push",
            "target": "https://github.com/crussella0129/cubikan-observatory.git",
            "branch": EXPECTED_BRANCH,
            "mutation": True,
        },
        {
            "kind": "deploy",
            "target": "git@github.com:crussella0129/cubikan-process-studio.git",
            "mutation": False,
        },
        {
            "kind": "github.public_rest.repository.read",
            "target": EXPECTED_REPOSITORY,
            "mutation": True,
        },
    ]
    for index, action in enumerate(invalid_actions):
        try:
            validate_actions(
                {
                    "action_ledger_complete": True,
                    "action_scope": "sprint-10",
                    "actions": [action],
                }
            )
        except EvidenceError:
            continue
        print(f"audit_evidence self-test: invalid action {index} was accepted", file=sys.stderr)
        return 1

    malformed_head = {
        "kind": "git.push",
        "target": "https://github.com/crussella0129/CubiKan.git",
        "branch": EXPECTED_BRANCH,
        "head": "0" * 41,
        "mutation": True,
    }
    try:
        validate_actions(
            {
                "action_ledger_complete": True,
                "action_scope": "sprint-10",
                "actions": [malformed_head],
            }
        )
    except EvidenceError:
        pass
    else:
        print("audit_evidence self-test: malformed push head was accepted", file=sys.stderr)
        return 1

    repository_root = Path(__file__).resolve().parents[4]
    current_head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repository_root, text=True
    ).strip()
    unresolved_head = "f" * 40
    require(not git_commit_exists(repository_root, unresolved_head), "self-test SHA exists")
    try:
        validate_actions(
            {
                "action_ledger_complete": True,
                "action_scope": "sprint-10",
                "actions": [
                    {
                        "kind": "git.push",
                        "target": "https://github.com/crussella0129/CubiKan.git",
                        "branch": EXPECTED_BRANCH,
                        "head": unresolved_head,
                        "mutation": True,
                    }
                ],
            },
            repository_root,
            current_head,
        )
    except EvidenceError:
        pass
    else:
        print("audit_evidence self-test: unresolved push head was accepted", file=sys.stderr)
        return 1

    valid_read_targets = {
        "git.remote.read": EXPECTED_REPOSITORY,
        "git.history.read": EXPECTED_REPOSITORY,
        "github.public_rest.repository.read": EXPECTED_REPOSITORY,
        "github.public_rest.published_releases.read": EXPECTED_REPOSITORY,
        "github.public_rest.deployments.read": EXPECTED_REPOSITORY,
        "github.connector.repository.search": f"{EXPECTED_ACCOUNT}/cubikan-agent-ops",
        "github.connector.repositories.list": EXPECTED_ACCOUNT,
    }
    validate_actions(
        {
            "action_ledger_complete": True,
            "action_scope": "sprint-10",
            "actions": [
                *[
                    {"kind": kind, "target": target, "mutation": False}
                    for kind, target in valid_read_targets.items()
                ],
                {
                    "kind": "git.push",
                    "target": "https://github.com/crussella0129/CubiKan.git",
                    "branch": EXPECTED_BRANCH,
                    "head": "0" * 40,
                    "mutation": True,
                },
            ],
        }
    )

    validate_inventory(valid_inventory_fixture())
    invalid_inventories = []
    fabricated_source = valid_inventory_fixture()
    fabricated_source["derivative_repository_checks"][0]["public_rest_observation"] = {
        "source": "claimed GitHub response",
        "status": "404",
    }
    invalid_inventories.append(fabricated_source)
    missing_connector = valid_inventory_fixture()
    del missing_connector["derivative_repository_checks"][0][
        "authenticated_connector_observation"
    ]
    invalid_inventories.append(missing_connector)
    overstated_connector = valid_inventory_fixture()
    overstated_connector["connector_repository_list_observation"][
        "complete_account_inventory"
    ] = True
    invalid_inventories.append(overstated_connector)
    count_mismatch = valid_inventory_fixture()
    count_mismatch["published_release_observation"]["result_count"] = 1
    invalid_inventories.append(count_mismatch)

    for index, fixture in enumerate(invalid_inventories):
        try:
            validate_inventory(deepcopy(fixture))
        except EvidenceError:
            continue
        print(
            f"audit_evidence self-test: invalid inventory {index} was accepted",
            file=sys.stderr,
        )
        return 1

    print(
        "audit_evidence self-test: 5 unsafe/unknown actions, malformed and unresolved "
        "push heads, and 4 invalid evidence shapes rejected; typed offline fixture accepted"
    )
    return 0


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    if len(sys.argv) != 3:
        print("usage: audit_evidence.py PROJECT_ROOT AUDIT_EVIDENCE.json", file=sys.stderr)
        return 2

    root = Path(sys.argv[1]).resolve()
    evidence_path = Path(sys.argv[2]).resolve()
    try:
        data = json.loads(evidence_path.read_text(encoding="utf-8"))
        require(isinstance(data, dict), "audit evidence must be a JSON object")
        require(data.get("schema_version") == 1, "schema_version must be 1")
        current_head = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True
        ).strip()
        tested_head = data.get("tested_head")
        require(
            isinstance(tested_head, str) and COMMIT_RE.fullmatch(tested_head) is not None,
            "tested_head must be a full lowercase 40-hex commit",
        )
        require(git_commit_exists(root, tested_head), "tested_head does not resolve to a commit")
        require(
            git_is_ancestor(root, tested_head, current_head),
            "tested_head is not an ancestor of the current evidence checkout",
        )

        repository_checks, releases, deployments = validate_inventory(data)
        actions, mutations = validate_actions(data, root, tested_head)
    except (EvidenceError, OSError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"audit_evidence: {error}", file=sys.stderr)
        return 1

    print(
        "audit_evidence: "
        f"head={tested_head} targeted_repository_checks={repository_checks} "
        f"published_releases={releases} "
        f"deployments={deployments} actions={actions} remote_mutations={mutations}; "
        "captured observations report no derivative repository or mutation; "
        "offline shape/consistency validation only (no provider calls)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
