#!/usr/bin/env python3
"""Verify the fixed CubiKan runtime artifact and deployment-anchor contract."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import pathlib
import re
import socket
import struct
import sys
import tomllib
import urllib.parse
from typing import Any, NoReturn

ANCHOR_PATH = "chain/artifacts/local-deployment-anchor-v1.json"
CONFIG_PATH = "chain/config/cubikan-local.json"
METADATA_PATH = "chain/metadata/cubikan-runtime-v1.scale"
WASM_PATH = "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm"
CONFIG_SHA256 = "dc7945fbeed5b18d21c1839f8f4f5ab13a1660ca956a3513b8a9946bab6334c7"
CONFIG_SIZE = 1_278_643
METADATA_SHA256 = "171a323b1e6bf0122e549eecd5f5932e672a3e0835f32edf0b8808cfefd97302"
METADATA_SIZE = 63_327
WASM_SHA256 = "640cc616674fe7393fc93928904f0fd92d77571209c8200f08b8da6290c6a275"
WASM_SIZE = 637_930
RUNTIME_CODE_BLAKE2_256 = "e95e40bb618591b98b315b7901f3586ee5899f8bf26bda01401601c4f86b8a00"
RELAY_GENESIS_HASH = "0xeb2ada687ce553d3b9d695afd5d9d0a9c44a0b82e1f6eb823ac87e81638200f0"
PARACHAIN_GENESIS_HASH = "0x627f53b3abc01130ec273ef85759f90779e8497614a428a66d862a624ee01a17"
DEPLOYMENT_INPUT = b"CubiKan local deployment v1\n"
DEPLOYMENT_ID = hashlib.sha256(DEPLOYMENT_INPUT).hexdigest()
HASH_RE = re.compile(r"^0x[0-9a-f]{64}$")
MAX_RPC_MESSAGE_BYTES = 64 * 1024 * 1024
PARACHAIN_RPC_URL = "ws://127.0.0.1:9988/"
PARACHAIN_RPC_ROLE = "parachain-collator-alice-rpc"
PARACHAIN_BLOCK_0 = "$parachain_block_0_hash"

EXPECTED_RUNTIME_APIS = [
    ["0xab3c0572291feb8b", 2],
    ["0xea93e3f16f3d6962", 3],
    ["0x37e397fc7c91f5e4", 2],
    ["0x37c8bb1350a9a2a8", 4],
    ["0xbc9d89904f5b923f", 1],
    ["0xdf6acb689907609b", 5],
    ["0xd7bdd8a272ca0d65", 2],
    ["0x075f8cd374350e84", 1],
    ["0xd2bc9897eed08f15", 3],
    ["0xfbc577b9d747efd6", 1],
    ["0xf78b278be53f454c", 2],
    ["0xa2ddb6a58477bf63", 1],
    ["0xdd718d5cc53262d4", 1],
    ["0x04e70521a0d3d2f8", 2],
    ["0x40fe3ad401f8959a", 6],
    ["0xccd9de6396c899ca", 1],
    ["0xf3ff14d5ab527059", 3],
]


def parachain_provenance(method: str, params: list[Any]) -> dict[str, Any]:
    return {
        "endpoint_role": PARACHAIN_RPC_ROLE,
        "method": method,
        "params": params,
        "rpc_url": PARACHAIN_RPC_URL,
    }


EXPECTED_STATE_RECORDS = {
    "deployment_id": {
        "provenance": parachain_provenance(
            "state_getStorage",
            [
                "0x2609aef1a1450f1b658394d12c417d249cc1dff5d6cf049a569219f2724d2e09",
                PARACHAIN_BLOCK_0,
            ],
        ),
        "scale_hex": "0x" + DEPLOYMENT_ID,
        "value": "0x" + DEPLOYMENT_ID,
    },
    "event_schema_version": {
        "provenance": parachain_provenance(
            "state_getStorage",
            [
                "0x2609aef1a1450f1b658394d12c417d24be4eb7443b9de1b5f041137c841f02ac",
                PARACHAIN_BLOCK_0,
            ],
        ),
        "scale_hex": "0x0100",
        "value": 1,
    },
    "pallet_storage_version": {
        "provenance": parachain_provenance(
            "state_getStorage",
            [
                "0x2609aef1a1450f1b658394d12c417d241601562ebcdff856cb2f34e65f3b2659",
                PARACHAIN_BLOCK_0,
            ],
        ),
        "scale_hex": "0x0100",
        "value": 1,
    },
    "para_id": {
        "provenance": parachain_provenance(
            "state_getStorage",
            [
                "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f",
                PARACHAIN_BLOCK_0,
            ],
        ),
        "scale_hex": "0xe8030000",
        "value": 1000,
    },
}

RUNTIME_VERSION_PROVENANCE = parachain_provenance(
    "state_getRuntimeVersion", [PARACHAIN_BLOCK_0]
)
COLLATOR_ACCOUNTS = [
    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
]
SUBMITTER_ACCOUNTS = [
    "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
    "5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy",
]
ENDOWED_BALANCE = 1 << 60


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def pairs_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def regular_file(root: pathlib.Path, relative: str) -> pathlib.Path:
    path = root / relative
    if path.is_symlink() or not path.is_file():
        fail(f"missing or symbolic regular file: {relative}")
    try:
        path.resolve(strict=True).relative_to(root)
    except ValueError:
        fail(f"artifact escapes project root: {relative}")
    return path


def optional_regular_file(root: pathlib.Path, relative: str) -> pathlib.Path | None:
    path = root / relative
    if not path.exists() and not path.is_symlink():
        return None
    return regular_file(root, relative)


def load_json(path: pathlib.Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=pairs_without_duplicates)
    if not isinstance(value, dict):
        fail(f"JSON root is not an object: {path}")
    return value


def digest(path: pathlib.Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            hasher.update(chunk)
    return hasher.hexdigest()


def exact_keys(value: dict[str, Any], expected: set[str], label: str) -> None:
    actual = set(value)
    if actual != expected:
        fail(f"{label} key mismatch: missing={sorted(expected - actual)} extra={sorted(actual - expected)}")


def strictly_equal(actual: Any, expected: Any) -> bool:
    """Compare decoded JSON without Python's bool/int/float coercions."""
    if type(actual) is not type(expected):
        return False
    if isinstance(expected, dict):
        return set(actual) == set(expected) and all(
            strictly_equal(actual[key], value) for key, value in expected.items()
        )
    if isinstance(expected, list):
        return len(actual) == len(expected) and all(
            strictly_equal(actual_value, expected_value)
            for actual_value, expected_value in zip(actual, expected, strict=True)
        )
    return actual == expected


def expect_hash(value: Any, label: str) -> str:
    if not isinstance(value, str) or HASH_RE.fullmatch(value) is None or value == "0x" + "0" * 64:
        fail(f"{label} is not a nonzero lowercase 32-byte hash")
    return value[2:]


def check_artifact(
    root: pathlib.Path,
    entry: dict[str, Any],
    expected_path: str,
    expected_sha256: str,
    expected_size: int,
    label: str,
    expected_provenance: dict[str, Any] | None = None,
) -> pathlib.Path:
    keys = {"path", "sha256", "size"}
    if expected_provenance is not None:
        keys.add("provenance")
    exact_keys(entry, keys, label)
    if entry.get("path") != expected_path:
        fail(f"{label} path mismatch")
    if expected_provenance is not None and not strictly_equal(
        entry.get("provenance"), expected_provenance
    ):
        fail(f"{label} provenance mismatch")
    expected = entry.get("sha256")
    size = entry.get("size")
    if expected != expected_sha256:
        fail(f"{label} SHA-256 differs from the locked release identity")
    if type(size) is not int or size != expected_size:
        fail(f"{label} size differs from the locked release identity")
    path = regular_file(root, expected_path)
    if path.stat().st_size != size:
        fail(f"{label} size mismatch")
    if digest(path) != expected:
        fail(f"{label} SHA-256 mismatch")
    return path


def check_fixed_anchor(anchor: dict[str, Any]) -> None:
    exact_keys(
        anchor,
        {"artifacts", "deployment", "format", "namespace", "parachain_genesis", "relay_genesis", "runtime", "status"},
        "anchor",
    )
    if anchor["format"] != "cubikan-local-deployment-anchor-v1":
        fail("anchor format mismatch")
    if anchor["namespace"] != "polkadot-sdk-parachain":
        fail("anchor namespace mismatch")
    if anchor["status"] != "resolved":
        fail("deployment anchor is not the resolved release contract")

    deployment = anchor["deployment"]
    exact_keys(
        deployment,
        {
            "deployment_id",
            "deployment_id_derivation",
            "event_schema_version",
            "pallet_storage_version",
            "para_id",
            "source",
            "state_records",
        },
        "deployment",
    )
    expected_deployment = {
        "deployment_id": "0x" + DEPLOYMENT_ID,
        "deployment_id_derivation": {
            "algorithm": "sha256",
            "input_utf8": DEPLOYMENT_INPUT.decode("utf-8"),
        },
        "event_schema_version": 1,
        "pallet_storage_version": 1,
        "para_id": 1000,
        "source": "decoded-pallet-cubikan-genesis-state",
        "state_records": EXPECTED_STATE_RECORDS,
    }
    if not strictly_equal(deployment, expected_deployment):
        fail("deployment identity or provenance mismatch")

    runtime = anchor["runtime"]
    exact_keys(
        runtime,
        {
            "apis",
            "authoring_version",
            "code",
            "impl_name",
            "impl_version",
            "provenance",
            "spec_name",
            "spec_version",
            "state_version",
            "system_version",
            "transaction_version",
        },
        "runtime",
    )
    expected_runtime = {
        "authoring_version": 1,
        "impl_name": "cubikan-runtime",
        "impl_version": 0,
        "spec_name": "cubikan-runtime",
        "spec_version": 1,
        "state_version": 1,
        "system_version": 1,
        "transaction_version": 1,
    }
    apis = runtime.get("apis")
    if not isinstance(apis, list):
        fail("runtime API inventory is not a list")
    for index, api in enumerate(apis):
        if (
            not isinstance(api, list)
            or len(api) != 2
            or not isinstance(api[0], str)
            or re.fullmatch(r"0x[0-9a-f]{16}", api[0]) is None
            or type(api[1]) is not int
            or api[1] <= 0
        ):
            fail(f"runtime API inventory entry {index} is malformed")
    if len({tuple(api) for api in apis}) != len(apis):
        fail("runtime API inventory contains a duplicate")
    if not strictly_equal(apis, EXPECTED_RUNTIME_APIS):
        fail("runtime API inventory differs from the locked ordered release inventory")
    for key, expected in expected_runtime.items():
        if not strictly_equal(runtime.get(key), expected):
            fail(f"runtime {key} mismatch")
    if not strictly_equal(runtime["provenance"], RUNTIME_VERSION_PROVENANCE):
        fail("runtime version provenance mismatch")
    if set(runtime["code"]) != {"blake2_256", "provenance"}:
        fail("runtime code key mismatch")
    if not strictly_equal(runtime["code"]["provenance"], {
        "endpoint_role": "parachain-collator-alice-rpc",
        "method": "state_getStorage",
        "params": ["0x3a636f6465", "$parachain_block_0_hash"],
        "rpc_url": "ws://127.0.0.1:9988/",
    }):
        fail("runtime code provenance mismatch")
    if runtime["code"]["blake2_256"] != "0x" + RUNTIME_CODE_BLAKE2_256:
        fail("runtime code hash differs from the locked release identity")

    for label, expected in (
        (
            "relay_genesis",
            {
                "block_number": 0,
                "hash": RELAY_GENESIS_HASH,
                "provenance": {
                    "endpoint_role": "relay-validator-alice-rpc",
                    "method": "chain_getBlockHash",
                    "params": [0],
                    "rpc_url": "ws://127.0.0.1:9944/",
                },
            },
        ),
        (
            "parachain_genesis",
            {
                "block_number": 0,
                "hash": PARACHAIN_GENESIS_HASH,
                "provenance": {
                    "endpoint_role": "parachain-collator-alice-rpc",
                    "method": "chain_getBlockHash",
                    "params": [0],
                    "rpc_url": "ws://127.0.0.1:9988/",
                },
            },
        ),
    ):
        entry = anchor[label]
        exact_keys(entry, {"block_number", "hash", "provenance"}, label)
        for key, expected_value in expected.items():
            if not strictly_equal(entry[key], expected_value):
                fail(f"{label} {key} mismatch")


def check_runtime_genesis_contract(config: dict[str, Any], resolved: bool) -> None:
    runtime_genesis = config.get("genesis", {}).get("runtimeGenesis")
    if not isinstance(runtime_genesis, dict):
        fail("chain spec runtimeGenesis is missing")
    exact_keys(runtime_genesis, {"code", "patch"}, "chain spec runtimeGenesis")
    patch = runtime_genesis.get("patch")
    if not isinstance(patch, dict):
        fail("chain spec runtimeGenesis patch is missing")
    exact_keys(
        patch,
        {"balances", "collatorSelection", "cubikan", "parachainInfo", "session"},
        "chain spec runtimeGenesis patch",
    )
    code = runtime_genesis.get("code", "missing")
    if resolved:
        if not isinstance(code, str) or re.fullmatch(r"0x(?:[0-9a-f]{2})+", code) is None:
            fail("resolved chain spec lacks exact lowercase nonempty System :code bytes")
    elif code is not None:
        fail("bootstrap chain spec must not fabricate runtime code")

    if not strictly_equal(patch.get("parachainInfo"), {"parachainId": 1000}):
        fail("chain spec ParaId mismatch")
    expected_balances = [
        [account, ENDOWED_BALANCE]
        for account in COLLATOR_ACCOUNTS + SUBMITTER_ACCOUNTS
    ]
    if not strictly_equal(patch.get("balances"), {"balances": expected_balances}):
        fail("chain spec endowed balances are not exact")
    if not strictly_equal(patch.get("collatorSelection"), {
        "candidacyBond": 16_000_000_000,
        "invulnerables": COLLATOR_ACCOUNTS,
    }):
        fail("chain spec collator identities or candidacy bond mismatch")
    cubikan = patch.get("cubikan")
    if not strictly_equal(cubikan, {
        "authorizedSubmitters": SUBMITTER_ACCOUNTS,
        "deploymentId": list(bytes.fromhex(DEPLOYMENT_ID)),
        "eventSchemaVersion": 1,
        "palletStorageVersion": 1,
    }):
        fail("chain spec Cubikan genesis contract mismatch")
    expected_session_keys = [
        [account, account, {"aura": account}] for account in COLLATOR_ACCOUNTS
    ]
    if not strictly_equal(patch.get("session"), {"keys": expected_session_keys}):
        fail("chain spec session keys are not exact or role-distinct")
    if set(COLLATOR_ACCOUNTS) & set(SUBMITTER_ACCOUNTS):
        fail("chain spec collator and submitter roles overlap")


def check_chain_spec_envelope(config: dict[str, Any], *, bootstrap: bool) -> None:
    """Bind the fixed local chain identity in bootstrap and resolved specs."""
    required_keys = {
        "bootNodes",
        "chainType",
        "codeSubstitutes",
        "genesis",
        "id",
        "name",
        "para_id",
        "properties",
        "protocolId",
        "relay_chain",
        "telemetryEndpoints",
    }
    if bootstrap:
        required_keys |= {"format", "status"}
    exact_keys(config, required_keys, "chain spec")
    genesis = config.get("genesis")
    if not isinstance(genesis, dict):
        fail("chain spec genesis is missing")
    exact_keys(genesis, {"runtimeGenesis"}, "chain spec genesis")
    expected = {
        "name": "CubiKan Local",
        "id": "cubikan-local",
        "chainType": "Local",
        "relay_chain": "rococo-local",
        "para_id": 1000,
        "protocolId": "cubikan-local-v1",
        "bootNodes": [],
        "telemetryEndpoints": None,
        "codeSubstitutes": {},
    }
    for key, value in expected.items():
        if not strictly_equal(config.get(key), value):
            fail(f"chain spec {key} mismatch")
    if not strictly_equal(config.get("properties"), {
        "ss58Format": 42,
        "tokenDecimals": 12,
        "tokenSymbol": "UNIT",
    }):
        fail("chain properties mismatch")


def check_bootstrap_config(config: dict[str, Any]) -> None:
    if config.get("status") != "awaiting-runtime-wasm":
        fail("bootstrap chain spec status mismatch")
    if config.get("format") != "cubikan-local-chain-spec-bootstrap-v1":
        fail("bootstrap chain spec format mismatch")
    check_chain_spec_envelope(config, bootstrap=True)
    check_runtime_genesis_contract(config, resolved=False)


def genesis_code(config: dict[str, Any]) -> bytes:
    """Decode the exact System `:code` bytes represented by a plain or raw spec."""
    genesis = config.get("genesis")
    if not isinstance(genesis, dict):
        fail("chain spec genesis is missing")
    runtime_genesis = genesis.get("runtimeGenesis")
    encoded: Any = None
    if isinstance(runtime_genesis, dict):
        encoded = runtime_genesis.get("code")
    raw = genesis.get("raw")
    if encoded is None and isinstance(raw, dict):
        top = raw.get("top")
        if isinstance(top, dict):
            encoded = top.get("0x3a636f6465")
    if not isinstance(encoded, str) or re.fullmatch(r"0x(?:[0-9a-f]{2})+", encoded) is None:
        fail("chain spec does not encode exact lowercase nonempty System :code bytes")
    return bytes.fromhex(encoded[2:])


def receive_exact(connection: socket.socket, length: int) -> bytes:
    chunks: list[bytes] = []
    remaining = length
    while remaining:
        chunk = connection.recv(remaining)
        if not chunk:
            fail("live RPC WebSocket closed before a complete response")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def send_websocket_frame(connection: socket.socket, opcode: int, payload: bytes) -> None:
    if len(payload) > MAX_RPC_MESSAGE_BYTES:
        fail("live RPC request exceeds the fixed message bound")
    header = bytearray([0x80 | opcode])
    if len(payload) < 126:
        header.append(0x80 | len(payload))
    elif len(payload) <= 0xFFFF:
        header.append(0x80 | 126)
        header.extend(struct.pack(">H", len(payload)))
    else:
        header.append(0x80 | 127)
        header.extend(struct.pack(">Q", len(payload)))
    mask = b"\x43\x75\x62\x69"
    header.extend(mask)
    masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    connection.sendall(header + masked)


def receive_websocket_message(connection: socket.socket) -> bytes:
    message = bytearray()
    message_opcode: int | None = None
    while True:
        first, second = receive_exact(connection, 2)
        final = bool(first & 0x80)
        opcode = first & 0x0F
        if first & 0x70:
            fail("live RPC WebSocket used an unsupported reserved bit")
        if second & 0x80:
            fail("live RPC server sent an illegally masked frame")
        length = second & 0x7F
        if length == 126:
            length = struct.unpack(">H", receive_exact(connection, 2))[0]
        elif length == 127:
            length = struct.unpack(">Q", receive_exact(connection, 8))[0]
            if length >> 63:
                fail("live RPC WebSocket frame has an invalid length")
        if length > MAX_RPC_MESSAGE_BYTES or len(message) + length > MAX_RPC_MESSAGE_BYTES:
            fail("live RPC response exceeds the fixed message bound")
        payload = receive_exact(connection, length)
        if opcode == 0x8:
            fail("live RPC WebSocket returned a close frame before its response")
        if opcode == 0x9:
            if not final or length > 125:
                fail("live RPC WebSocket returned an invalid ping frame")
            send_websocket_frame(connection, 0xA, payload)
            continue
        if opcode == 0xA:
            if not final or length > 125:
                fail("live RPC WebSocket returned an invalid pong frame")
            continue
        if opcode in (0x1, 0x2):
            if message_opcode is not None:
                fail("live RPC WebSocket interleaved data messages")
            message_opcode = opcode
        elif opcode == 0x0:
            if message_opcode is None:
                fail("live RPC WebSocket returned an orphan continuation")
        else:
            fail("live RPC WebSocket returned an unsupported opcode")
        message.extend(payload)
        if final:
            if message_opcode != 0x1:
                fail("live RPC WebSocket response was not UTF-8 JSON text")
            return bytes(message)


def websocket_rpc(url: str, endpoint_role: str, method: str, params: list[Any]) -> Any:
    parsed = urllib.parse.urlsplit(url)
    if (
        parsed.scheme != "ws"
        or parsed.hostname != "127.0.0.1"
        or parsed.port not in (9944, 9988)
        or parsed.path != "/"
        or parsed.query
        or parsed.fragment
        or parsed.username is not None
        or parsed.password is not None
    ):
        fail(f"{endpoint_role} is not an exact permitted loopback WebSocket URL")
    host_header = f"127.0.0.1:{parsed.port}"
    key = base64.b64encode(b"CubiKanRpcProof1").decode("ascii")
    expected_accept = base64.b64encode(
        hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()
    ).decode("ascii")
    request = (
        "GET / HTTP/1.1\r\n"
        f"Host: {host_header}\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n\r\n"
    ).encode("ascii")
    try:
        with socket.create_connection(("127.0.0.1", parsed.port), timeout=5.0) as connection:
            connection.settimeout(10.0)
            connection.sendall(request)
            response = bytearray()
            while b"\r\n\r\n" not in response:
                if len(response) >= 16 * 1024:
                    fail(f"{endpoint_role} WebSocket handshake exceeds the fixed bound")
                response.extend(receive_exact(connection, 1))
            header_bytes, trailing = bytes(response).split(b"\r\n\r\n", 1)
            if trailing:
                fail(f"{endpoint_role} sent data before its WebSocket handshake completed")
            try:
                header_lines = header_bytes.decode("ascii").split("\r\n")
            except UnicodeDecodeError:
                fail(f"{endpoint_role} returned a non-ASCII WebSocket handshake")
            if header_lines[0] != "HTTP/1.1 101 Switching Protocols":
                fail(f"{endpoint_role} rejected the WebSocket handshake")
            headers: dict[str, str] = {}
            for line in header_lines[1:]:
                if ":" not in line:
                    fail(f"{endpoint_role} returned a malformed WebSocket header")
                name, value = line.split(":", 1)
                name = name.strip().lower()
                if name in headers:
                    fail(f"{endpoint_role} returned a duplicate WebSocket header")
                headers[name] = value.strip()
            if headers.get("upgrade", "").lower() != "websocket":
                fail(f"{endpoint_role} did not confirm the WebSocket upgrade")
            if "upgrade" not in {
                token.strip().lower() for token in headers.get("connection", "").split(",")
            }:
                fail(f"{endpoint_role} did not confirm its upgraded connection")
            if headers.get("sec-websocket-accept") != expected_accept:
                fail(f"{endpoint_role} WebSocket accept identity mismatch")
            payload = json.dumps(
                {"id": 1, "jsonrpc": "2.0", "method": method, "params": params},
                ensure_ascii=True,
                separators=(",", ":"),
                sort_keys=True,
            ).encode("ascii")
            send_websocket_frame(connection, 0x1, payload)
            response_payload = receive_websocket_message(connection)
    except (ConnectionError, OSError, TimeoutError) as error:
        fail(f"{endpoint_role} live RPC unavailable: {error}")
    try:
        decoded = json.loads(response_payload, object_pairs_hook=pairs_without_duplicates)
    except (UnicodeDecodeError, json.JSONDecodeError):
        fail(f"{endpoint_role} returned malformed JSON-RPC")
    if not isinstance(decoded, dict) or set(decoded) != {"id", "jsonrpc", "result"}:
        fail(f"{endpoint_role} returned an unexpected JSON-RPC envelope")
    if (
        not strictly_equal(decoded["id"], 1)
        or decoded["jsonrpc"] != "2.0"
        or decoded["result"] is None
    ):
        fail(f"{endpoint_role} returned an invalid JSON-RPC result")
    return decoded["result"]


def verify_live_rpc(anchor: dict[str, Any], metadata: bytes, code: bytes) -> None:
    for label in ("relay_genesis", "parachain_genesis"):
        entry = anchor[label]
        provenance = entry["provenance"]
        result = websocket_rpc(
            provenance["rpc_url"],
            provenance["endpoint_role"],
            provenance["method"],
            provenance["params"],
        )
        if result != entry["hash"]:
            fail(f"{label} live RPC value mismatch")
    parachain_hash = anchor["parachain_genesis"]["hash"]
    for label, entry in anchor["deployment"]["state_records"].items():
        provenance = entry["provenance"]
        params = [
            parachain_hash if value == PARACHAIN_BLOCK_0 else value
            for value in provenance["params"]
        ]
        result = websocket_rpc(
            provenance["rpc_url"],
            provenance["endpoint_role"],
            provenance["method"],
            params,
        )
        if result != entry["scale_hex"]:
            fail(f"deployment {label} live RPC value mismatch")
    runtime = anchor["runtime"]
    provenance = runtime["provenance"]
    runtime_version = websocket_rpc(
        provenance["rpc_url"],
        provenance["endpoint_role"],
        provenance["method"],
        [
            parachain_hash if value == PARACHAIN_BLOCK_0 else value
            for value in provenance["params"]
        ],
    )
    if not isinstance(runtime_version, dict):
        fail("runtime version live RPC result is not an object")
    expected_runtime_version = {
        "authoringVersion": runtime["authoring_version"],
        "implName": runtime["impl_name"],
        "implVersion": runtime["impl_version"],
        "specName": runtime["spec_name"],
        "specVersion": runtime["spec_version"],
        "systemVersion": runtime["system_version"],
        "transactionVersion": runtime["transaction_version"],
    }
    for key, expected in expected_runtime_version.items():
        if not strictly_equal(runtime_version.get(key), expected):
            fail(f"runtime version {key} live RPC value mismatch")
    if not strictly_equal(runtime_version.get("stateVersion"), runtime["state_version"]):
        fail("runtime version stateVersion live RPC value mismatch")
    if not strictly_equal(runtime_version.get("apis"), runtime["apis"]):
        fail("runtime version live RPC API inventory mismatch")
    for label, entry, expected in (
        ("runtime code", anchor["runtime"]["code"], "0x" + code.hex()),
        ("metadata", anchor["artifacts"]["metadata"], "0x" + metadata.hex()),
    ):
        provenance = entry["provenance"]
        params = [
            parachain_hash if value == PARACHAIN_BLOCK_0 else value
            for value in provenance["params"]
        ]
        result = websocket_rpc(
            provenance["rpc_url"],
            provenance["endpoint_role"],
            provenance["method"],
            params,
        )
        if result != expected:
            fail(f"{label} live RPC value mismatch")


def verify(root: pathlib.Path, mode: str) -> None:
    """Verify committed semantics; only the explicit live mode opens RPC sockets."""
    anchor_path = regular_file(root, ANCHOR_PATH)
    config_path = regular_file(root, CONFIG_PATH)
    anchor = load_json(anchor_path)
    config = load_json(config_path)
    check_fixed_anchor(anchor)

    pins_path = regular_file(root, "chain/pins.toml")
    pins = tomllib.loads(pins_path.read_text(encoding="utf-8"))
    artifact_pins = pins.get("runtime_artifacts")
    if not isinstance(artifact_pins, dict):
        fail("pins.toml lacks runtime_artifacts")
    exact_keys(
        artifact_pins,
        {
            "anchor_path",
            "anchor_sha256",
            "bootstrap_state",
            "chain_spec_path",
            "chain_spec_sha256",
            "metadata_path",
            "metadata_sha256",
            "runtime_code_blake2_256",
            "runtime_wasm_path",
            "runtime_wasm_sha256",
        },
        "runtime_artifacts pins",
    )
    fixed_pin_values = {
        "anchor_path": ANCHOR_PATH,
        "anchor_sha256": digest(anchor_path),
        "chain_spec_path": CONFIG_PATH,
        "chain_spec_sha256": CONFIG_SHA256,
    }
    for key, expected in fixed_pin_values.items():
        if artifact_pins.get(key) != expected:
            fail(f"runtime_artifacts.{key} mismatch")

    artifacts = anchor["artifacts"]
    exact_keys(artifacts, {"chain_spec", "metadata", "runtime_wasm"}, "artifacts")
    chain_spec_path = check_artifact(
        root,
        artifacts["chain_spec"],
        CONFIG_PATH,
        CONFIG_SHA256,
        CONFIG_SIZE,
        "chain spec",
    )
    if chain_spec_path != config_path:
        fail("chain spec path identity mismatch")
    if config.get("status") is not None or config.get("format") == "cubikan-local-chain-spec-bootstrap-v1":
        fail("resolved anchor still points to a bootstrap chain spec")
    check_chain_spec_envelope(config, bootstrap=False)
    check_runtime_genesis_contract(config, resolved=True)
    relay_hash = expect_hash(anchor["relay_genesis"]["hash"], "relay genesis hash")
    para_hash = expect_hash(anchor["parachain_genesis"]["hash"], "parachain genesis hash")
    if relay_hash == para_hash:
        fail("relay and parachain genesis hashes must be distinct semantic identities")
    code_hash = expect_hash(anchor["runtime"]["code"]["blake2_256"], "runtime code hash")
    metadata = check_artifact(
        root,
        artifacts["metadata"],
        METADATA_PATH,
        METADATA_SHA256,
        METADATA_SIZE,
        "metadata",
        {
            "endpoint_role": "parachain-collator-alice-rpc",
            "method": "state_getMetadata",
            "params": ["$parachain_block_0_hash"],
            "rpc_url": "ws://127.0.0.1:9988/",
        },
    )
    wasm = check_artifact(
        root,
        artifacts["runtime_wasm"],
        WASM_PATH,
        WASM_SHA256,
        WASM_SIZE,
        "runtime Wasm",
        {
            "builder": "substrate-wasm-builder",
            "profile": "release",
            "source_path": (
                "chain/target/release/wbuild/cubikan-runtime/"
                "cubikan_runtime.compact.compressed.wasm"
            ),
        },
    )
    if mode in ("locked", "live"):
        source_path = artifacts["runtime_wasm"]["provenance"]["source_path"]
        source_wasm = regular_file(root, source_path)
        if source_wasm.stat().st_size != wasm.stat().st_size or digest(source_wasm) != digest(wasm):
            fail("committed runtime Wasm differs from the canonical release build output")
    if metadata.stat().st_size < 4 or metadata.read_bytes()[:4] != b"meta":
        fail("metadata does not have the SCALE metadata magic")
    wasm_bytes = wasm.read_bytes()
    code_bytes = genesis_code(config)
    if wasm_bytes != code_bytes:
        fail("committed runtime artifact differs from chain-spec System :code bytes")
    if hashlib.blake2b(code_bytes, digest_size=32).hexdigest() != code_hash:
        fail("chain-spec System :code does not match the pinned block-zero code hash")
    resolved_pins = {
        "bootstrap_state": "resolved",
        "metadata_path": METADATA_PATH,
        "metadata_sha256": METADATA_SHA256,
        "runtime_wasm_path": WASM_PATH,
        "runtime_wasm_sha256": WASM_SHA256,
        "runtime_code_blake2_256": RUNTIME_CODE_BLAKE2_256,
    }
    for key, expected in resolved_pins.items():
        if artifact_pins.get(key) != expected:
            fail(f"runtime_artifacts.{key} mismatch")
    if mode == "live":
        verify_live_rpc(anchor, metadata.read_bytes(), code_bytes)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("live", "locked", "test-static"))
    parser.add_argument("root")
    args = parser.parse_args()
    try:
        root = pathlib.Path(args.root).resolve(strict=True)
        verify(root, mode=args.mode)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
        print(f"verify-runtime-artifacts: {error}", file=sys.stderr)
        return 1
    print("verify-runtime-artifacts: fixed artifact contract verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
