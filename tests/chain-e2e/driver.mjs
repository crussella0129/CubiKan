import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import * as fs from "node:fs";
import * as fsp from "node:fs/promises";
import net from "node:net";
import { createRequire, syncBuiltinESMExports } from "node:module";
import path from "node:path";
import process from "node:process";
import { Writable } from "node:stream";
import { pathToFileURL } from "node:url";

const SYSTEM_EVENTS_KEY =
  "0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
const RUNTIME_CODE_KEY = "0x3a636f6465";
const PHASE_TIMEOUT_MS = 180_000;
const PROCESS_OUTPUT_LIMIT = 1_048_576;
const MAX_AUDIT_ARTIFACT_BYTES = 4_194_304;
const MAX_RAW_CHAIN_SPEC_BYTES = 8_388_608;
const MAX_AUDIT_TOTAL_BYTES = 25_165_824;
const MAX_NODE_LOG_BYTES = 1_048_576;
const MAX_NODE_LOG_TOTAL_BYTES = 4_194_304;
const ARCHIVE_PROBE_BLOCK_CONCURRENCY = 4;
const CENSUS_BLOCK_CONCURRENCY = 4;
const FINALIZED_OBSERVATION_CONCURRENCY = 4;
const SNAPSHOT_READ_CONCURRENCY = 4;
const JOURNEY_PHASES = new Set([
  "module-imports",
  "zombienet-start",
  "api-connect",
  "node-evidence",
  "normalizer-matrix",
  "pre-mutation-readiness",
  "initial-submissions",
  "initial-submission-m01",
  "initial-submission-m02",
  "initial-submission-m03",
  "initial-submission-m04",
  "initial-submission-m05",
  "initial-submission-m06",
  "initial-submission-m07",
  "collator-stop",
  "survivor-submissions",
  "survivor-submission-m08",
  "survivor-submission-m09",
  "survivor-submission-m10",
  "survivor-submission-m11",
  "survivor-submission-m12",
  "survivor-submission-m13",
  "survivor-submission-m14",
  "survivor-submission-m15",
  "survivor-submission-m16",
  "survivor-submission-m17",
  "survivor-submission-m18",
  "survivor-submission-m19",
  "survivor-submission-m20",
  "survivor-submission-m21",
  "finality-stability",
  "collator-restart-spawn",
  "collator-restart-native-readiness",
  "collator-restart-primary-listener",
  "collator-restart-api-reconnect",
  "collator-restart-relay-api",
  "collator-restart-convergence",
  "collator-restart-evidence",
  "archive-and-observations",
  "parachain-census",
  "projection-snapshot-uninterrupted",
  "projection-snapshot-fresh-pair",
  "projection-snapshot-rebuild-a",
  "projection-snapshot-rebuild-b",
  "relay-resume",
  "relay-resume-api-reconnect",
  "relay-resume-convergence",
  "relay-census",
  "network-stop",
  "post-stop-verification",
  "audit-materialization",
  "evidence-publication",
  "complete",
]);
const SPEC_CANONICALIZATION =
  "remove-one-top-level-bootNodes-recursive-key-sort-compact-json-v1";
const SENSITIVE_PAYLOAD_KEYS = [
  "credential",
  "credentials",
  "mnemonic",
  "passphrase",
  "password",
  "private_key",
  "private_locator",
  "prompt",
  "provider_secret",
  "secret",
  "seed",
  "source_body",
  "token",
  "transcript",
];
const SENSITIVE_PAYLOAD_KEY_ALTERNATION = SENSITIVE_PAYLOAD_KEYS.join("|");
const FORBIDDEN_TEXT_PATTERNS = [
  /(?:^|[^A-Z0-9_])(?:AWS_SECRET_ACCESS_KEY|GIT_ASKPASS|GIT_CONFIG_GLOBAL|GIT_CONFIG_SYSTEM|SSH_AUTH_SOCK|HTTP_PROXY|HTTPS_PROXY|ALL_PROXY|NO_PROXY)=/giu,
  new RegExp(
    `(?:^|[,{])[ \\t\\r\\n]*"(?:${SENSITIVE_PAYLOAD_KEY_ALTERNATION})"[ \\t\\r\\n]*:`,
    "giu",
  ),
  new RegExp(
    `(?:^|[^a-z0-9_])(?:${SENSITIVE_PAYLOAD_KEY_ALTERNATION})[ \\t]*[:=]`,
    "giu",
  ),
];

function installBoundedNodeLogCapture(networkRoot) {
  const mutableFs = createRequire(import.meta.url)("node:fs");
  const originalCreateWriteStream = mutableFs.createWriteStream;
  const sizes = new Map();
  const identities = new Map();
  const hashes = new Map();
  let totalBytes = 0;
  let discardedBytes = 0;
  let overflowed = false;
  let captureError = null;
  let allowFailureCleanupUnlink = false;
  let restored = false;
  const activeStreams = new Set();

  class BoundedNodeLogStream extends Writable {
    constructor(filePath, options = {}) {
      super(options);
      this.path = path.resolve(String(filePath));
      const flags = options?.flags ?? "w";
      requireCondition(flags === "w" || flags === "a", `unsupported node log flags ${flags}`);
      const canonicalParent = mutableFs.realpathSync(path.dirname(this.path));
      requireCondition(
        canonicalParent === networkRoot || canonicalParent.startsWith(`${networkRoot}${path.sep}`),
        `node log parent escaped the private network root: ${this.path}`,
      );
      const openFlags =
        mutableFs.constants.O_WRONLY |
        mutableFs.constants.O_CREAT |
        mutableFs.constants.O_CLOEXEC |
        mutableFs.constants.O_NOFOLLOW |
        (flags === "a" ? mutableFs.constants.O_APPEND : mutableFs.constants.O_TRUNC);
      this.fd = mutableFs.openSync(this.path, openFlags, 0o600);
      mutableFs.fchmodSync(this.fd, 0o600);
      const descriptor = mutableFs.fstatSync(this.fd, { bigint: true });
      const pathIdentity = mutableFs.lstatSync(this.path, { bigint: true });
      requireCondition(
        descriptor.isFile() &&
          pathIdentity.isFile() &&
          !pathIdentity.isSymbolicLink() &&
          descriptor.nlink === 1n &&
          descriptor.uid === BigInt(process.geteuid()) &&
          (descriptor.mode & 0o7777n) === 0o600n &&
          descriptor.dev === pathIdentity.dev &&
          descriptor.ino === pathIdentity.ino,
        `node log is not one owner-only no-follow regular file: ${this.path}`,
      );
      const initialSize = safeNumber(descriptor.size, `node log size ${this.path}`);
      const priorSize = sizes.get(this.path) ?? 0;
      const stableIdentity = {
        device: canonicalIdentityNumber(descriptor.dev, `node log device ${this.path}`),
        inode: canonicalIdentityNumber(descriptor.ino, `node log inode ${this.path}`),
        mode: (safeNumber(descriptor.mode, `node log mode ${this.path}`) & 0o7777)
          .toString(8)
          .padStart(4, "0"),
        link_count: safeNumber(descriptor.nlink, `node log link count ${this.path}`),
        owner_uid: safeNumber(descriptor.uid, `node log owner ${this.path}`),
      };
      const priorIdentity = identities.get(this.path);
      requireCondition(
        initialSize === priorSize &&
          initialSize <= MAX_NODE_LOG_BYTES &&
          (flags === "a" || priorSize === 0) &&
          (priorIdentity === undefined ||
            JSON.stringify(priorIdentity) === JSON.stringify(stableIdentity)),
        `node log reopen identity/size drifted: ${this.path}`,
      );
      if (priorIdentity === undefined) identities.set(this.path, stableIdentity);
      if (!hashes.has(this.path)) {
        requireCondition(initialSize === 0, `node log hash state lacks prior bytes: ${this.path}`);
        hashes.set(this.path, createHash("sha256"));
      }
      sizes.set(this.path, initialSize);
      if (flags === "a" && priorSize === 0) totalBytes += initialSize;
      this.bytesWritten = 0;
      this.fdClosed = false;
      activeStreams.add(this);
      this.once("close", () => activeStreams.delete(this));
      this.on("error", (error) => {
        if (captureError === null) captureError = error;
      });
    }

    _write(chunk, encoding, callback) {
      try {
        const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk, encoding);
        const current = sizes.get(this.path) ?? 0;
        const permitted = Math.max(
          0,
          Math.min(
            bytes.length,
            MAX_NODE_LOG_BYTES - current,
            MAX_NODE_LOG_TOTAL_BYTES - totalBytes,
          ),
        );
        if (permitted > 0) {
          const written = mutableFs.writeSync(this.fd, bytes, 0, permitted);
          requireCondition(written === permitted, `short bounded node log write: ${this.path}`);
          sizes.set(this.path, current + written);
          hashes.get(this.path).update(bytes.subarray(0, written));
          totalBytes += written;
          this.bytesWritten += written;
        }
        if (permitted !== bytes.length) {
          overflowed = true;
          discardedBytes += bytes.length - permitted;
        }
        callback();
      } catch (error) {
        callback(error);
      }
    }

    _final(callback) {
      try {
        if (!this.fdClosed) {
          mutableFs.fsyncSync(this.fd);
          let descriptor = mutableFs.fstatSync(this.fd, { bigint: true });
          let pathIdentity = null;
          try {
            pathIdentity = mutableFs.lstatSync(this.path, { bigint: true });
          } catch (error) {
            if (error?.code !== "ENOENT") throw error;
            descriptor = mutableFs.fstatSync(this.fd, { bigint: true });
          }
          const stableIdentity = identities.get(this.path);
          const pathStillBound = pathIdentity !== null;
          requireCondition(
            descriptor.isFile() &&
              descriptor.dev.toString(10) === stableIdentity.device &&
              descriptor.ino.toString(10) === stableIdentity.inode &&
              safeNumber(descriptor.size, `final node log size ${this.path}`) ===
                sizes.get(this.path) &&
              descriptor.uid === BigInt(process.geteuid()) &&
              (descriptor.mode & 0o7777n) === 0o600n &&
              (pathStillBound
                ? pathIdentity.isFile() &&
                  !pathIdentity.isSymbolicLink() &&
                  pathIdentity.dev === descriptor.dev &&
                  pathIdentity.ino === descriptor.ino &&
                  descriptor.nlink === 1n
                : allowFailureCleanupUnlink && descriptor.nlink === 0n),
            `node log object changed before close: ${this.path}`,
          );
          mutableFs.closeSync(this.fd);
          this.fdClosed = true;
        }
        callback();
      } catch (error) {
        callback(error);
      }
    }

    _destroy(error, callback) {
      try {
        if (!this.fdClosed) {
          mutableFs.closeSync(this.fd);
          this.fdClosed = true;
        }
        callback(error);
      } catch (closeError) {
        callback(error ?? closeError);
      }
    }
  }

  mutableFs.createWriteStream = (filePath, options) => {
    const resolved = path.resolve(String(filePath));
    if (resolved.startsWith(`${networkRoot}${path.sep}`) && resolved.endsWith(".log"))
      return new BoundedNodeLogStream(resolved, options);
    return originalCreateWriteStream.call(mutableFs, filePath, options);
  };
  syncBuiltinESMExports();
  return {
    async quiesce() {
      const streams = [...activeStreams];
      await Promise.all(
        streams.map(
          (stream) =>
            new Promise((resolve) => {
              stream.once("close", resolve);
            }),
        ),
      );
      requireCondition(activeStreams.size === 0, "bounded node log streams did not quiesce");
    },
    allowFailureCleanupUnlink() {
      allowFailureCleanupUnlink = true;
    },
    assertHealthy() {
      requireCondition(
        !overflowed && discardedBytes === 0 && captureError === null,
        "bounded node log capture overflowed or failed its integrity checks",
      );
    },
    evidence() {
      requireCondition(captureError === null, "bounded node log capture failed before evidence");
      const files = [...sizes.entries()]
        .sort(([left], [right]) => left.localeCompare(right, "en"))
        .map(([filePath, size]) => {
          const symbolic = mutableFs.lstatSync(filePath, { bigint: true });
          const identity = identities.get(filePath);
          const fd = mutableFs.openSync(
            filePath,
            mutableFs.constants.O_RDONLY |
              mutableFs.constants.O_CLOEXEC |
              mutableFs.constants.O_NOFOLLOW,
          );
          let bytes;
          try {
            const descriptor = mutableFs.fstatSync(fd, { bigint: true });
            requireCondition(
              symbolic.isFile() &&
                !symbolic.isSymbolicLink() &&
                descriptor.dev.toString(10) === identity.device &&
                descriptor.ino.toString(10) === identity.inode &&
                symbolic.dev === descriptor.dev &&
                symbolic.ino === descriptor.ino &&
                safeNumber(descriptor.size, `retained node log size ${filePath}`) === size &&
                descriptor.nlink === 1n &&
                descriptor.uid === BigInt(process.geteuid()) &&
                (descriptor.mode & 0o7777n) === 0o600n,
              `bounded node log object drifted: ${filePath}`,
            );
            bytes = mutableFs.readFileSync(fd);
          } finally {
            mutableFs.closeSync(fd);
          }
          const retainedSha256 = sha256(bytes);
          requireCondition(
            bytes.length === size && retainedSha256 === hashes.get(filePath).copy().digest("hex"),
            `bounded node log bytes differ from captured writes: ${filePath}`,
          );
          return { file_path: filePath, size, ...identity, sha256: retainedSha256 };
        });
      return {
        format: "cubikan-bounded-node-log-capture-v1",
        per_file_limit_bytes: MAX_NODE_LOG_BYTES,
        aggregate_limit_bytes: MAX_NODE_LOG_TOTAL_BYTES,
        total_bytes: totalBytes,
        discarded_bytes: discardedBytes,
        overflowed,
        files,
      };
    },
    restore() {
      if (restored) return;
      mutableFs.createWriteStream = originalCreateWriteStream;
      syncBuiltinESMExports();
      restored = true;
    },
  };
}

function captureProcessDiagnostics(limit) {
  const originalStdoutWrite = process.stdout.write.bind(process.stdout);
  const originalStderrWrite = process.stderr.write.bind(process.stderr);
  const records = [];
  let totalBytes = 0;
  let discardedBytes = 0;
  let overflowed = false;
  let restored = false;
  const intercept = (stream) => (chunk, encoding, callback) => {
    const actualEncoding = typeof encoding === "string" ? encoding : undefined;
    const actualCallback =
      typeof encoding === "function" ? encoding : typeof callback === "function" ? callback : null;
    const bytes = Buffer.isBuffer(chunk)
      ? Buffer.from(chunk)
      : chunk instanceof Uint8Array
        ? Buffer.from(chunk)
        : Buffer.from(String(chunk), actualEncoding ?? "utf8");
    if (totalBytes + bytes.length > limit) {
      overflowed = true;
      discardedBytes += bytes.length;
      fail("orchestrator diagnostics exceeded the retained bound");
    }
    if (bytes.length > 0) records.push({ sequence: records.length, stream, bytes });
    totalBytes += bytes.length;
    if (actualCallback) queueMicrotask(actualCallback);
    return true;
  };
  process.stdout.write = intercept("stdout");
  process.stderr.write = intercept("stderr");
  return {
    evidence(fixture) {
      const aggregate = Buffer.concat(records.map((record) => record.bytes));
      const stdout = Buffer.concat(
        records.filter((record) => record.stream === "stdout").map((record) => record.bytes),
      );
      const stderr = Buffer.concat(
        records.filter((record) => record.stream === "stderr").map((record) => record.bytes),
      );
      const scannedTexts = [aggregate, stdout, stderr].map((bytes) => bytes.toString("utf8"));
      const forbiddenHits = [
        ...new Set(
          scannedTexts.flatMap((text) =>
            FORBIDDEN_TEXT_PATTERNS.flatMap((pattern) => text.match(pattern) ?? []),
          ),
        ),
      ].sort();
      const nonloopbackHits = [
        ...new Set(
          scannedTexts
            .map((text) => firstDisallowedUrl(text, fixture))
            .filter((value) => value !== null),
        ),
      ].sort();
      requireCondition(
        !overflowed &&
          discardedBytes === 0 &&
          forbiddenHits.length === 0 &&
          nonloopbackHits.length === 0,
        "orchestrator diagnostics overflowed or contain a public URL, hostile helper, or secret marker",
      );
      return {
        format: "cubikan-bounded-orchestrator-diagnostics-v1",
        limit_bytes: limit,
        total_bytes: totalBytes,
        discarded_bytes: discardedBytes,
        overflowed,
        aggregate_sha256: sha256(aggregate),
        stdout_sha256: sha256(stdout),
        stderr_sha256: sha256(stderr),
        forbidden_hits: forbiddenHits,
        nonloopback_hits: nonloopbackHits,
        records: records.map((record) => ({
          sequence: record.sequence,
          stream: record.stream,
          byte_length: record.bytes.length,
          bytes_hex: `0x${record.bytes.toString("hex")}`,
        })),
      };
    },
    restore() {
      if (restored) return;
      process.stdout.write = originalStdoutWrite;
      process.stderr.write = originalStderrWrite;
      restored = true;
    },
  };
}

function installTerminationPhaseMarker(filePath) {
  let phase = "module-imports";
  let installed = true;
  const temporaryPath = `${filePath}.tmp`;
  const directoryPath = path.dirname(filePath);
  const validateMarker = (candidate, expectedSize) => {
    const marker = fs.lstatSync(candidate, { bigint: true });
    requireCondition(
      marker.isFile() &&
        !marker.isSymbolicLink() &&
        marker.nlink === 1n &&
        marker.uid === BigInt(process.geteuid()) &&
        (marker.mode & 0o7777n) === 0o600n &&
        marker.size > 0n &&
        marker.size <= 64n &&
        marker.size === BigInt(expectedSize),
      "journey phase marker is not one exact owner-only regular file",
    );
    return marker;
  };
  const publish = (next) => {
    requireCondition(JOURNEY_PHASES.has(next), `unknown journey phase ${next}`);
    const bytes = Buffer.from(`${next}\n`, "ascii");
    if (fs.existsSync(filePath)) validateMarker(filePath, fs.lstatSync(filePath).size);
    requireCondition(!fs.existsSync(temporaryPath), "journey phase temporary already exists");
    const descriptor = fs.openSync(
      temporaryPath,
      fs.constants.O_WRONLY |
        fs.constants.O_CREAT |
        fs.constants.O_EXCL |
        fs.constants.O_CLOEXEC |
        fs.constants.O_NOFOLLOW,
      0o600,
    );
    try {
      let offset = 0;
      while (offset < bytes.length) {
        const written = fs.writeSync(
          descriptor,
          bytes,
          offset,
          bytes.length - offset,
          null,
        );
        requireCondition(written > 0, "journey phase marker write made no progress");
        offset += written;
      }
      fs.fsyncSync(descriptor);
      const opened = fs.fstatSync(descriptor, { bigint: true });
      const named = validateMarker(temporaryPath, bytes.length);
      requireCondition(
        opened.dev === named.dev && opened.ino === named.ino,
        "journey phase temporary identity drifted",
      );
    } finally {
      fs.closeSync(descriptor);
    }
    fs.renameSync(temporaryPath, filePath);
    validateMarker(filePath, bytes.length);
    const directory = fs.openSync(
      directoryPath,
      fs.constants.O_RDONLY |
        fs.constants.O_DIRECTORY |
        fs.constants.O_CLOEXEC |
        fs.constants.O_NOFOLLOW,
    );
    try {
      fs.fsyncSync(directory);
    } finally {
      fs.closeSync(directory);
    }
    phase = next;
  };
  requireCondition(
    !fs.existsSync(filePath) && !fs.existsSync(temporaryPath),
    "journey phase marker paths were not fresh",
  );
  publish(phase);
  const onSigterm = () => {
    const marker = Buffer.from(`run-four-node-journey: last-phase=${phase}\n`, "ascii");
    try {
      fs.writeSync(2, marker);
    } catch {}
    process.exit(143);
  };
  process.once("SIGTERM", onSigterm);
  return {
    set(next) {
      publish(next);
    },
    remove() {
      if (!installed) return;
      process.removeListener("SIGTERM", onSigterm);
      installed = false;
    },
  };
}

function fail(message) {
  throw new Error(message);
}

function requireCondition(condition, message) {
  if (!condition) fail(message);
}

function parseArguments(argv) {
  const expected = [
    "--fixture",
    "--evidence",
    "--work-root",
    "--config",
    "--zombienet-root",
  ];
  requireCondition(argv.length === expected.length * 2, "invalid driver argv length");
  const parsed = {};
  for (let index = 0; index < expected.length; index += 1) {
    const flag = expected[index];
    requireCondition(argv[index * 2] === flag, `expected ${flag}`);
    const value = argv[index * 2 + 1];
    requireCondition(
      path.isAbsolute(value) && !value.includes("\0") && path.normalize(value) === value,
      `${flag} must be one normalized absolute path`,
    );
    parsed[flag.slice(2).replaceAll("-", "_")] = value;
  }
  return parsed;
}

function sortedValue(value) {
  if (Array.isArray(value)) return value.map(sortedValue);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, sortedValue(value[key])]),
    );
  }
  return value;
}

function canonicalBytes(value) {
  return Buffer.from(JSON.stringify(sortedValue(value)), "utf8");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function canonicalSha256(value) {
  return sha256(canonicalBytes(value));
}

function codecJson(value) {
  if (value === null || value === undefined) return null;
  if (typeof value.toJSON === "function") return value.toJSON();
  return value;
}

function exactUnsignedBigInt(value, label) {
  const decimal =
    typeof value === "number" && Number.isSafeInteger(value) && value >= 0
      ? String(value)
      : typeof value === "string" && /^(?:0|[1-9][0-9]*)$/.test(value)
        ? value
        : null;
  requireCondition(decimal !== null, `${label} is not a canonical unsigned integer`);
  return BigInt(decimal);
}

class ScaleCursor {
  constructor(bytes, label) {
    this.bytes = bytes;
    this.label = label;
    this.offset = 0;
  }

  take(length) {
    requireCondition(
      Number.isSafeInteger(length) && length >= 0 && this.offset + length <= this.bytes.length,
      `${this.label} SCALE payload is truncated`,
    );
    const value = this.bytes.subarray(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }

  u8() {
    return this.take(1)[0];
  }

  u16() {
    return this.take(2).readUInt16LE(0);
  }

  u64() {
    return this.take(8).readBigUInt64LE(0).toString(10);
  }

  compactLength() {
    const first = this.u8();
    const mode = first & 3;
    if (mode === 0) return first >>> 2;
    if (mode === 1) {
      const value = ((this.u8() << 8) | first) >>> 2;
      requireCondition(value >= 64, `${this.label} has a noncanonical compact length`);
      return value;
    }
    if (mode === 2) {
      const rest = this.take(3);
      const encoded = first | (rest[0] << 8) | (rest[1] << 16) | (rest[2] << 24);
      const value = encoded >>> 2;
      requireCondition(value >= 16_384, `${this.label} has a noncanonical compact length`);
      return value;
    }
    fail(`${this.label} uses an over-bound compact length`);
  }

  text(maximum = 256) {
    const length = this.compactLength();
    requireCondition(length > 0 && length <= maximum, `${this.label} text length is invalid`);
    return new TextDecoder("utf-8", { fatal: true }).decode(this.take(length));
  }

  uuid() {
    const hex = this.take(16).toString("hex");
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  vector(parse, maximum) {
    const length = this.compactLength();
    requireCondition(length <= maximum, `${this.label} vector length is over bound`);
    return Array.from({ length }, () => parse());
  }

  finish() {
    requireCondition(this.offset === this.bytes.length, `${this.label} SCALE payload has trailing bytes`);
  }
}

function parseExternalReference(cursor) {
  return {
    namespace: cursor.text(64),
    scope: cursor.text(),
    value: cursor.text(),
  };
}

function parseDefinitionKey(cursor) {
  return { id: cursor.text(64), version: cursor.u64() };
}

function parseRelationshipKey(cursor) {
  return {
    definition: parseDefinitionKey(cursor),
    source_id: cursor.uuid(),
    target_id: cursor.uuid(),
  };
}

function parseAssociationKey(cursor) {
  const unitId = cursor.uuid();
  const subjectTag = cursor.u8();
  requireCondition(subjectTag === 0 || subjectTag === 1, "association subject tag is invalid");
  return {
    unit_id: unitId,
    subject:
      subjectTag === 0
        ? { type: "whole_unit" }
        : { type: "revision", revision: cursor.u64() },
    reference: parseExternalReference(cursor),
  };
}

function parseOptionText(cursor) {
  const tag = cursor.u8();
  requireCondition(tag === 0 || tag === 1, "optional text tag is invalid");
  return tag === 0 ? null : cursor.text();
}

function parseAcceptedPayload(bytes, expectedOperation) {
  const cursor = new ScaleCursor(bytes, `accepted ${expectedOperation}`);
  const tag = cursor.u8();
  const expectedTags = {
    create_intent_unit: 0,
    transition_intent_unit: 1,
    complete_intent_unit: 2,
    create_relationship_definition: 3,
    create_relationship: 4,
    delete_relationship: 5,
    record_association: 6,
    revoke_association: 7,
  };
  requireCondition(expectedTags[expectedOperation] === tag, `accepted payload tag mismatches ${expectedOperation}`);
  let variant;
  let effect;
  let payload;
  if (tag === 0) {
    requireCondition(cursor.u16() === 1, "create event command schema version drifted");
    const unitId = cursor.uuid();
    const origin = parseExternalReference(cursor);
    const species = cursor.text();
    const workflow = {
      id: cursor.text(),
      phases: cursor.vector(() => cursor.text(), 32),
      initial_phase: cursor.text(),
      edges: cursor.vector(() => ({ from: cursor.text(), to: cursor.text() }), 128),
      completion_phases: cursor.vector(() => cursor.text(), 32),
    };
    variant = "unit_created";
    payload = {
      command_schema_version: 1,
      id: unitId,
      origin,
      species,
      workflow,
    };
    effect = { type: variant, unit_id: unitId, committed_revision: "0" };
  } else if (tag === 1) {
    const unitId = cursor.uuid();
    const committedRevision = cursor.u64();
    const from = cursor.text();
    const to = cursor.text();
    variant = "unit_transitioned";
    payload = { unit_id: unitId, committed_revision: committedRevision, from, to };
    effect = { type: variant, unit_id: unitId, committed_revision: committedRevision };
  } else if (tag === 2) {
    const unitId = cursor.uuid();
    const committedRevision = cursor.u64();
    const phase = cursor.text();
    variant = "unit_completed";
    payload = { unit_id: unitId, committed_revision: committedRevision, phase };
    effect = { type: variant, unit_id: unitId, committed_revision: committedRevision };
  } else if (tag === 3) {
    const definition = parseDefinitionKey(cursor);
    requireCondition(cursor.u8() === 0, "relationship direction is not Directed");
    const sourceSpecies = parseOptionText(cursor);
    const targetSpecies = parseOptionText(cursor);
    const selfPolicyTag = cursor.u8();
    const cyclePolicyTag = cursor.u8();
    requireCondition(selfPolicyTag <= 1 && cyclePolicyTag <= 1, "relationship policy tag is invalid");
    variant = "relationship_definition_created";
    payload = {
      definition,
      direction: "directed",
      source_species: sourceSpecies,
      target_species: targetSpecies,
      self_policy: selfPolicyTag === 0 ? "allow" : "reject",
      cycle_policy: cyclePolicyTag === 0 ? "allow" : "reject",
    };
    effect = { type: variant, definition };
  } else if (tag === 4 || tag === 5) {
    const relationship = parseRelationshipKey(cursor);
    variant = tag === 4 ? "relationship_created" : "relationship_deleted";
    payload = { relationship };
    effect = { type: variant, relationship };
  } else {
    const association = parseAssociationKey(cursor);
    variant = tag === 6 ? "association_recorded" : "association_revoked";
    payload = { association };
    effect = { type: variant, association };
  }
  cursor.finish();
  return { tag, variant, payload, effect };
}

function expectedAcceptedPayload(mutation, lifecyclePhases) {
  const operation = mutation.request.operation;
  if (mutation.operation === "create_intent_unit") {
    const unit = operation.intent_unit;
    lifecyclePhases.set(unit.id, operation.workflow.initial_phase);
    return {
      command_schema_version: 1,
      id: unit.id,
      origin: unit.origin,
      species: unit.species,
      workflow: operation.workflow,
    };
  }
  if (mutation.operation === "transition_intent_unit") {
    const from = lifecyclePhases.get(operation.id);
    requireCondition(typeof from === "string", `${mutation.id} has no modeled source phase`);
    lifecyclePhases.set(operation.id, operation.target);
    return {
      unit_id: operation.id,
      committed_revision: (BigInt(operation.expected_revision) + 1n).toString(10),
      from,
      to: operation.target,
    };
  }
  if (mutation.operation === "complete_intent_unit") {
    const phase = lifecyclePhases.get(operation.id);
    requireCondition(typeof phase === "string", `${mutation.id} has no modeled completion phase`);
    return {
      unit_id: operation.id,
      committed_revision: (BigInt(operation.expected_revision) + 1n).toString(10),
      phase,
    };
  }
  if (mutation.operation === "create_relationship_definition") {
    return {
      definition: operation.definition,
      direction: "directed",
      source_species: operation.source_species ?? null,
      target_species: operation.target_species ?? null,
      self_policy: operation.self_policy,
      cycle_policy: operation.cycle_policy,
    };
  }
  if (["create_relationship", "delete_relationship"].includes(mutation.operation)) {
    return { relationship: operation.relationship };
  }
  if (["record_association", "revoke_association"].includes(mutation.operation)) {
    return { association: operation.association };
  }
  fail(`unmapped accepted payload operation ${mutation.operation}`);
}

function safeNumber(value, label) {
  const number = Number(value);
  requireCondition(Number.isSafeInteger(number), `${label} is not a safe integer`);
  return number;
}

function canonicalIdentityNumber(value, label) {
  requireCondition(typeof value === "bigint" && value >= 0n, `${label} is not an unsigned integer`);
  return value.toString(10);
}

function timeParts(nanoseconds, label) {
  const billion = 1_000_000_000n;
  const seconds = nanoseconds / billion;
  const remainder = nanoseconds % billion;
  return {
    seconds: safeNumber(seconds, `${label} seconds`),
    nanoseconds: safeNumber(remainder, `${label} nanoseconds`),
  };
}

function identityFromStat(stat) {
  const modified = timeParts(stat.mtimeNs, "mtime");
  const changed = timeParts(stat.ctimeNs, "ctime");
  return {
    device: canonicalIdentityNumber(stat.dev, "device"),
    inode: canonicalIdentityNumber(stat.ino, "inode"),
    size: safeNumber(stat.size, "size"),
    mode: (safeNumber(stat.mode, "mode") & 0o7777).toString(8).padStart(4, "0"),
    link_count: safeNumber(stat.nlink, "link count"),
    modified_seconds: modified.seconds,
    modified_nanoseconds: modified.nanoseconds,
    changed_seconds: changed.seconds,
    changed_nanoseconds: changed.nanoseconds,
  };
}

async function openedFileEvidence(filePath, maximumBytes = null) {
  const symbolic = await fsp.lstat(filePath, { bigint: true });
  requireCondition(!symbolic.isSymbolicLink(), `symbolic input rejected: ${filePath}`);
  const pathBefore = identityFromStat(symbolic);
  if (maximumBytes !== null) {
    requireCondition(
      Number.isSafeInteger(maximumBytes) &&
        maximumBytes > 0 &&
        pathBefore.size <= maximumBytes,
      `file exceeds the retained bound: ${filePath}`,
    );
  }
  const handle = await fsp.open(filePath, "r");
  try {
    const descriptorBefore = identityFromStat(await handle.stat({ bigint: true }));
    const bytes = await readExactAt(handle, pathBefore.size, filePath);
    const extra = Buffer.alloc(1);
    const extraRead = await handle.read(extra, 0, 1, pathBefore.size);
    requireCondition(extraRead.bytesRead === 0, `file grew while reading ${filePath}`);
    const descriptorAfter = identityFromStat(await handle.stat({ bigint: true }));
    const pathAfter = identityFromStat(await fsp.lstat(filePath, { bigint: true }));
    const identities = [pathBefore, descriptorBefore, descriptorAfter, pathAfter];
    requireCondition(
      identities.every((identity) => JSON.stringify(identity) === JSON.stringify(pathBefore)),
      `file identity changed while reading ${filePath}`,
    );
    requireCondition(bytes.length === pathBefore.size, `short read for ${filePath}`);
    return {
      evidence: {
        path_before: pathBefore,
        descriptor_before: descriptorBefore,
        descriptor_after: descriptorAfter,
        path_after: pathAfter,
        bytes_read: bytes.length,
        sha256: sha256(bytes),
        regular_file: symbolic.isFile(),
        symbolic_link: symbolic.isSymbolicLink(),
      },
      bytes,
    };
  } finally {
    await handle.close();
  }
}

async function openedOwnerBootstrapFile(filePath, maximumBytes, label, allowEmpty = false) {
  const symbolic = await fsp.lstat(filePath, { bigint: true });
  requireCondition(
    symbolic.isFile() &&
      !symbolic.isSymbolicLink() &&
      symbolic.uid === BigInt(process.geteuid()) &&
      symbolic.nlink === 1n &&
      (symbolic.mode & 0o7777n) === 0o600n &&
      (allowEmpty || symbolic.size > 0n) &&
      symbolic.size <= BigInt(maximumBytes),
    `${label} is not one nonempty owner-only bounded regular file`,
  );
  return await openedFileEvidence(filePath, maximumBytes);
}

function decodeExactLowerHexExport(bytes, label) {
  const text = bytes.toString("ascii");
  requireCondition(
    bytes.length >= 4 &&
      text.length === bytes.length &&
      /^0x(?:[0-9a-f]{2})+$/.test(text),
    `${label} is not exact lowercase 0x-prefixed even-length hex without whitespace`,
  );
  const decoded = Buffer.from(text.slice(2), "hex");
  requireCondition(
    `0x${decoded.toString("hex")}` === text,
    `${label} did not round-trip through its exact hex encoding`,
  );
  return decoded;
}

async function openedProcExecutableEvidence(pid, expectedPath, maximumBytes) {
  const procExecutable = procPath(pid, "exe");
  const linkBefore = await fsp.readlink(procExecutable);
  requireCondition(linkBefore === expectedPath, `process ${pid} executable link drifted`);
  const pathBeforeStat = await fsp.stat(procExecutable, { bigint: true });
  const pathBefore = identityFromStat(pathBeforeStat);
  requireCondition(
    pathBefore.size > 0 && pathBefore.size <= maximumBytes,
    `process ${pid} executable exceeds its retained bound`,
  );
  const handle = await fsp.open(procExecutable, "r");
  try {
    const descriptorBefore = identityFromStat(await handle.stat({ bigint: true }));
    const bytes = await readExactAt(handle, pathBefore.size, procExecutable);
    const extra = Buffer.alloc(1);
    const extraRead = await handle.read(extra, 0, 1, pathBefore.size);
    requireCondition(extraRead.bytesRead === 0, `process ${pid} executable grew while reading`);
    const descriptorAfter = identityFromStat(await handle.stat({ bigint: true }));
    const pathAfter = identityFromStat(await fsp.stat(procExecutable, { bigint: true }));
    const linkAfter = await fsp.readlink(procExecutable);
    requireCondition(
      linkAfter === linkBefore &&
        [descriptorBefore, descriptorAfter, pathAfter].every(
          (identity) => JSON.stringify(identity) === JSON.stringify(pathBefore),
        ),
      `process ${pid} executable identity changed while reading`,
    );
    return {
      evidence: {
        path_before: pathBefore,
        descriptor_before: descriptorBefore,
        descriptor_after: descriptorAfter,
        path_after: pathAfter,
        bytes_read: bytes.length,
        sha256: sha256(bytes),
        regular_file: pathBeforeStat.isFile(),
        symbolic_link: true,
      },
      bytes,
    };
  } finally {
    await handle.close();
  }
}

async function readExactAt(handle, size, label) {
  const bytes = Buffer.alloc(size);
  let offset = 0;
  while (offset < bytes.length) {
    const result = await handle.read(bytes, offset, bytes.length - offset, offset);
    requireCondition(result.bytesRead > 0, `${label} reached EOF before its retained size`);
    offset += result.bytesRead;
  }
  return bytes;
}

async function readBoundedSpecialFile(filePath, maximumBytes) {
  const handle = await fsp.open(filePath, "r");
  const chunks = [];
  let total = 0;
  try {
    while (true) {
      const chunk = Buffer.alloc(Math.min(65_536, maximumBytes + 1 - total));
      const { bytesRead } = await handle.read(chunk, 0, chunk.length, null);
      if (bytesRead === 0) break;
      total += bytesRead;
      requireCondition(total <= maximumBytes, `${filePath} exceeded its retained read bound`);
      chunks.push(chunk.subarray(0, bytesRead));
    }
    return Buffer.concat(chunks, total);
  } finally {
    await handle.close();
  }
}

async function sqliteSidecarAbsence(database) {
  const observed = {};
  for (const suffix of ["-journal", "-shm", "-wal"]) {
    let errno = null;
    try {
      await fsp.lstat(`${database}${suffix}`);
    } catch (error) {
      if (error?.code === "ENOENT") errno = "ENOENT";
      else throw error;
    }
    requireCondition(errno === "ENOENT", `${database}${suffix} remained after the local process exited`);
    observed[suffix] = errno;
  }
  return observed;
}

async function readUniqueJson(filePath) {
  const bytes = (await openedFileEvidence(filePath)).bytes;
  const text = bytes.toString("utf8");
  // JSON.parse itself is not a duplicate-key oracle. The fixture is pinned by
  // the Rust duplicate-key visitor; generated raw specs are checked below by a
  // lexical parser before their canonical genesis hashes are admitted.
  return { bytes, value: JSON.parse(text) };
}

function rejectDuplicateJsonKeys(text, label) {
  const stack = [];
  let index = 0;
  let expectingKey = false;
  let pendingKey = null;
  const whitespace = /\s/;
  const readString = () => {
    const start = index;
    index += 1;
    let escaped = false;
    while (index < text.length) {
      const character = text[index];
      if (!escaped && character === '"') {
        index += 1;
        return JSON.parse(text.slice(start, index));
      }
      if (!escaped && character === "\\") escaped = true;
      else escaped = false;
      index += 1;
    }
    fail(`${label} has an unterminated JSON string`);
  };
  while (index < text.length) {
    const character = text[index];
    if (whitespace.test(character)) {
      index += 1;
      continue;
    }
    if (character === '"') {
      const value = readString();
      let lookahead = index;
      while (lookahead < text.length && whitespace.test(text[lookahead])) lookahead += 1;
      if (stack.at(-1)?.type === "object" && text[lookahead] === ":") {
        const keys = stack.at(-1).keys;
        requireCondition(!keys.has(value), `${label} contains duplicate key ${value}`);
        keys.add(value);
        pendingKey = value;
        expectingKey = false;
      }
      continue;
    }
    if (character === "{") {
      stack.push({ type: "object", keys: new Set() });
      expectingKey = true;
    } else if (character === "[") stack.push({ type: "array" });
    else if (character === "}" || character === "]") stack.pop();
    else if (character === "," && stack.at(-1)?.type === "object") expectingKey = true;
    else if (character === ":") {
      requireCondition(pendingKey !== null || !expectingKey, `${label} malformed object member`);
      pendingKey = null;
    }
    index += 1;
  }
  requireCondition(stack.length === 0, `${label} has unbalanced JSON containers`);
}

async function withTimeout(label, milliseconds, operation) {
  let timer;
  try {
    return await Promise.race([
      operation(),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`${label} timed out`)), milliseconds);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function runProcess(executable, args, options = {}) {
  const input = options.input ?? Buffer.alloc(0);
  const limit = options.limit ?? PROCESS_OUTPUT_LIMIT;
  const label = options.label ?? executable;
  return await new Promise((resolve, reject) => {
      const child = spawn("/usr/bin/setsid", [executable, ...args], {
        cwd: options.cwd,
        env: options.env,
        stdio: ["pipe", "pipe", "pipe"],
      });
      const stdout = [];
      const stderr = [];
      let stdoutBytes = 0;
      let stderrBytes = 0;
      let failure = null;
      let killTimer = null;
      let forceReturnTimer = null;
      let settled = false;
      const settleReject = (error) => {
        if (settled) return;
        settled = true;
        reject(error);
      };
      const settleResolve = (value) => {
        if (settled) return;
        settled = true;
        resolve(value);
      };
      const terminate = (reason) => {
        if (failure === null) failure = reason;
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch (error) {
          if (error?.code !== "ESRCH") failure = `${failure}; TERM failed: ${error}`;
        }
        if (!killTimer) {
          killTimer = setTimeout(() => {
            try {
              process.kill(-child.pid, "SIGKILL");
            } catch (error) {
              if (error?.code !== "ESRCH") failure = `${failure}; KILL failed: ${error}`;
            }
          }, 250);
        }
        if (!forceReturnTimer) {
          forceReturnTimer = setTimeout(() => {
            child.stdin.destroy();
            child.stdout.destroy();
            child.stderr.destroy();
            try {
              child.kill("SIGKILL");
            } catch {}
            settleReject(new Error(`${failure}; retained pipes did not close after termination`));
          }, 5_000);
        }
      };
      const deadlineTimer = setTimeout(
        () => terminate(`${label} timed out`),
        options.timeout ?? PHASE_TIMEOUT_MS,
      );
      const retain = (chunks, chunk, current) => {
        const remaining = Math.max(0, limit - current);
        if (remaining > 0) chunks.push(chunk.subarray(0, remaining));
        return current + chunk.length;
      };
      child.stdout.on("data", (chunk) => {
        stdoutBytes = retain(stdout, chunk, stdoutBytes);
        if (stdoutBytes > limit) terminate(`${label} stdout exceeded ${limit} bytes`);
      });
      child.stderr.on("data", (chunk) => {
        stderrBytes = retain(stderr, chunk, stderrBytes);
        if (stderrBytes > limit) terminate(`${label} stderr exceeded ${limit} bytes`);
      });
      child.once("error", (error) => {
        clearTimeout(deadlineTimer);
        if (killTimer) clearTimeout(killTimer);
        if (forceReturnTimer) clearTimeout(forceReturnTimer);
        settleReject(error);
      });
      child.once("close", async (code, signal) => {
        clearTimeout(deadlineTimer);
        if (killTimer) clearTimeout(killTimer);
        if (forceReturnTimer) clearTimeout(forceReturnTimer);
        const groupLive = () => {
          try {
            process.kill(-child.pid, 0);
            return true;
          } catch (error) {
            if (error?.code === "ESRCH") return false;
            throw error;
          }
        };
        if (groupLive()) {
          failure ??= `${label} exited with live process-group members`;
          try {
            process.kill(-child.pid, "SIGKILL");
          } catch (error) {
            if (error?.code !== "ESRCH") failure = `${failure}; final KILL failed: ${error}`;
          }
          try {
            await waitFor(() => !groupLive(), `${label} process-group reap`, 5_000, 10);
          } catch (error) {
            failure = `${failure}; ${error.message}`;
          }
        }
        if (failure !== null) return settleReject(new Error(failure));
        settleResolve({
          code,
          signal,
          stdout: Buffer.concat(stdout),
          stderr: Buffer.concat(stderr),
        });
      });
      child.stdin.end(input);
    });
}

function procPath(pid, member) {
  return `/proc/${pid}/${member}`;
}

async function procStatWithState(pid) {
  const text = await fsp.readFile(procPath(pid, "stat"), "utf8");
  const close = text.lastIndexOf(")");
  requireCondition(close > 0, `malformed /proc/${pid}/stat`);
  const fields = text.slice(close + 2).trim().split(/\s+/);
  return {
    state: fields[0],
    parent_pid: Number(fields[1]),
    process_group: Number(fields[2]),
    session_id: Number(fields[3]),
    start_time_ticks: Number(fields[19]),
  };
}

async function procStat(pid) {
  const { state: _state, ...generation } = await procStatWithState(pid);
  return generation;
}

function sameProcessGeneration(left, right) {
  return (
    left.parent_pid === right.parent_pid &&
    left.process_group === right.process_group &&
    left.session_id === right.session_id &&
    left.start_time_ticks === right.start_time_ticks
  );
}

async function processGenerationExited(pid, observedGeneration) {
  try {
    const before = await procStatWithState(pid);
    if (!sameProcessGeneration(before, observedGeneration) || before.state !== "Z") return false;
    const argvBefore = await fsp.readFile(procPath(pid, "cmdline"));
    const commBefore = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
    const after = await procStatWithState(pid);
    const argvAfter = await fsp.readFile(procPath(pid, "cmdline"));
    const commAfter = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
    return (
      sameProcessGeneration(before, after) &&
      after.state === "Z" &&
      argvBefore.length === 0 &&
      argvAfter.length === 0 &&
      commBefore === commAfter
    );
  } catch (error) {
    if (!["ENOENT", "ESRCH"].includes(error?.code)) throw error;
    return !(await processGenerationLive(pid, observedGeneration.start_time_ticks));
  }
}

async function processGenerationExitedWithin(pid, observedGeneration) {
  for (let retry = 0; retry < 8; retry += 1) {
    if (await processGenerationExited(pid, observedGeneration)) return true;
    await new Promise((resolve) => setTimeout(resolve, 1));
  }
  return processGenerationExited(pid, observedGeneration);
}

async function procObjectIdentity(pid, member = "") {
  const stat = await fsp.stat(procPath(pid, member), { bigint: true });
  return {
    device: canonicalIdentityNumber(stat.dev, "proc device"),
    inode: canonicalIdentityNumber(stat.ino, "proc inode"),
  };
}

async function processGenerationLive(pid, startTimeTicks) {
  try {
    return (await procStat(pid)).start_time_ticks === startTimeTicks;
  } catch (error) {
    if (["ENOENT", "ESRCH"].includes(error?.code)) return false;
    throw error;
  }
}

function parseStatusIds(value, pid, name) {
  const fields = value?.split(/\s+/) ?? [];
  requireCondition(
    fields.length === 4 && fields.every((field) => /^(?:0|[1-9][0-9]*)$/.test(field)),
    `process ${pid} status has malformed ${name}`,
  );
  const ids = fields.map(Number);
  requireCondition(
    ids.every((id) => Number.isSafeInteger(id) && id >= 0 && id <= 0xffff_ffff),
    `process ${pid} status has out-of-range ${name}`,
  );
  return ids;
}

async function processPrivilegeStatus(pid) {
  const bytes = await readBoundedSpecialFile(procPath(pid, "status"), 65_536);
  const fields = new Map();
  for (const line of bytes.toString("utf8").split("\n")) {
    const separator = line.indexOf(":");
    if (separator <= 0) continue;
    const name = line.slice(0, separator);
    if (
      ![
        "Uid",
        "Gid",
        "NoNewPrivs",
        "CapInh",
        "CapPrm",
        "CapEff",
        "CapBnd",
        "CapAmb",
      ].includes(name)
    )
      continue;
    requireCondition(!fields.has(name), `process ${pid} status duplicates ${name}`);
    fields.set(name, line.slice(separator + 1).trim());
  }
  const capabilities = {
    ambient: fields.get("CapAmb"),
    bounding: fields.get("CapBnd"),
    effective: fields.get("CapEff"),
    inheritable: fields.get("CapInh"),
    permitted: fields.get("CapPrm"),
  };
  requireCondition(
    /^(?:0|1)$/.test(fields.get("NoNewPrivs") ?? "") &&
      Object.values(capabilities).every(
        (value) => typeof value === "string" && /^[0-9a-f]{16}$/.test(value),
      ),
    `process ${pid} status lacks canonical privilege fields`,
  );
  return {
    no_new_privileges: fields.get("NoNewPrivs") === "1",
    capabilities,
    status_uids: parseStatusIds(fields.get("Uid"), pid, "Uid"),
    status_gids: parseStatusIds(fields.get("Gid"), pid, "Gid"),
  };
}

async function processPrivilegeEvidence(pid) {
  const status = await processPrivilegeStatus(pid);
  requireCondition(
    status.no_new_privileges &&
      Object.values(status.capabilities).every((value) => value === "0000000000000000"),
    `process ${pid} retained privileges or capabilities`,
  );
  return {
    no_new_privileges: status.no_new_privileges,
    capabilities: status.capabilities,
  };
}

function exactNamespaceIdentity(value, kind) {
  return new RegExp(`^${kind}:\\[[1-9][0-9]*\\]$`).test(value);
}

async function pvfProcessSecurityEvidence(pid, workerFixture) {
  const zero = "0000000000000000";
  const expectedFull = workerFixture.nested_namespace_capability_mask;
  requireCondition(
    /^[0-9a-f]{16}$/.test(expectedFull) && expectedFull !== zero,
    "PVF nested namespace capability-mask fixture is not canonical",
  );
  const orchestratorUserNamespaceBefore = await fsp.readlink("/proc/self/ns/user");
  const orchestratorMountNamespaceBefore = await fsp.readlink("/proc/self/ns/mnt");
  const userNamespaceBefore = await fsp.readlink(procPath(pid, "ns/user"));
  const mountNamespaceBefore = await fsp.readlink(procPath(pid, "ns/mnt"));
  const uidMapBefore = await readBoundedSpecialFile(procPath(pid, "uid_map"), 4096);
  const gidMapBefore = await readBoundedSpecialFile(procPath(pid, "gid_map"), 4096);
  const statusBefore = await processPrivilegeStatus(pid);
  const statusAfter = await processPrivilegeStatus(pid);
  const uidMapAfter = await readBoundedSpecialFile(procPath(pid, "uid_map"), 4096);
  const gidMapAfter = await readBoundedSpecialFile(procPath(pid, "gid_map"), 4096);
  const userNamespaceAfter = await fsp.readlink(procPath(pid, "ns/user"));
  const mountNamespaceAfter = await fsp.readlink(procPath(pid, "ns/mnt"));
  const orchestratorUserNamespaceAfter = await fsp.readlink("/proc/self/ns/user");
  const orchestratorMountNamespaceAfter = await fsp.readlink("/proc/self/ns/mnt");
  requireCondition(
    exactNamespaceIdentity(orchestratorUserNamespaceBefore, "user") &&
      exactNamespaceIdentity(orchestratorMountNamespaceBefore, "mnt") &&
      exactNamespaceIdentity(userNamespaceBefore, "user") &&
      exactNamespaceIdentity(mountNamespaceBefore, "mnt") &&
      orchestratorUserNamespaceBefore === orchestratorUserNamespaceAfter &&
      orchestratorMountNamespaceBefore === orchestratorMountNamespaceAfter &&
      userNamespaceBefore === userNamespaceAfter &&
      mountNamespaceBefore === mountNamespaceAfter &&
      uidMapBefore.equals(uidMapAfter) &&
      gidMapBefore.equals(gidMapAfter) &&
      JSON.stringify(statusBefore) === JSON.stringify(statusAfter),
    `PVF worker ${pid} security state changed during capture`,
  );
  const outerCapabilities = Object.values(statusBefore.capabilities).every(
    (value) => value === zero,
  );
  const nestedCapabilities =
    statusBefore.capabilities.ambient === zero &&
    statusBefore.capabilities.inheritable === zero &&
    statusBefore.capabilities.bounding === expectedFull &&
    statusBefore.capabilities.effective === expectedFull &&
    statusBefore.capabilities.permitted === expectedFull;
  const outerProfile =
    statusBefore.no_new_privileges &&
    outerCapabilities &&
    userNamespaceBefore === orchestratorUserNamespaceBefore &&
    mountNamespaceBefore === orchestratorMountNamespaceBefore &&
    statusBefore.status_uids.every((id) => id === 0) &&
    statusBefore.status_gids.every((id) => id === 0) &&
    uidMapBefore.length > 0 &&
    gidMapBefore.length > 0;
  const nestedProfile =
    statusBefore.no_new_privileges &&
    nestedCapabilities &&
    userNamespaceBefore !== orchestratorUserNamespaceBefore &&
    mountNamespaceBefore !== orchestratorMountNamespaceBefore &&
    statusBefore.status_uids.every((id) => id === 0) &&
    statusBefore.status_gids.every((id) => id === 0) &&
    uidMapBefore.length === 0 &&
    gidMapBefore.length === 0;
  requireCondition(
    outerProfile !== nestedProfile,
    `PVF worker ${pid} lacked one exact outer or nested sandbox profile`,
  );
  return {
    profile: nestedProfile ? "nested-unmapped-user-mount" : "outer-zero-capability",
    no_new_privileges: statusBefore.no_new_privileges,
    capabilities: statusBefore.capabilities,
    status_uids: statusBefore.status_uids,
    status_gids: statusBefore.status_gids,
    uid_map_before: uidMapBefore.toString("utf8"),
    uid_map_after: uidMapAfter.toString("utf8"),
    gid_map_before: gidMapBefore.toString("utf8"),
    gid_map_after: gidMapAfter.toString("utf8"),
    user_namespace_before: userNamespaceBefore,
    user_namespace_after: userNamespaceAfter,
    mount_namespace_before: mountNamespaceBefore,
    mount_namespace_after: mountNamespaceAfter,
    orchestrator_user_namespace: orchestratorUserNamespaceBefore,
    orchestrator_mount_namespace: orchestratorMountNamespaceBefore,
  };
}

async function stableProcessAncestry() {
  const self = await procStat(process.pid);
  const ancestors = new Map();
  let nextPid = self.parent_pid;
  while (nextPid !== 0) {
    requireCondition(
      Number.isSafeInteger(nextPid) && nextPid > 0 && !ancestors.has(nextPid),
      "driver process ancestry is invalid or cyclic",
    );
    requireCondition(ancestors.size < 64, "driver process ancestry exceeded its bound");
    const generation = await procStat(nextPid);
    ancestors.set(nextPid, generation);
    nextPid = generation.parent_pid;
  }
  requireCondition(ancestors.has(1), "driver process ancestry did not terminate at PID 1");
  return { self, generations: [...ancestors.entries()] };
}

async function nodeProcessInventory(fixture, repoRoot) {
  const expectedNamespaceInitArgv = [
    "/usr/bin/bash",
    "chain/tools/run-zombienet-e2e.sh",
    "--config",
    "chain/config/zombienet.toml",
    "--relay-validators",
    "2",
    "--collators",
    "2",
    "--loopback-only",
  ];
  const nodeAssets = new Map();
  for (const node of fixture.topology.nodes) {
    const pin = fixture.pinned_inputs.find((candidate) => candidate.path === node.binary);
    requireCondition(pin, `node asset pin is missing for ${node.role}`);
    const absolute = path.join(repoRoot, node.binary);
    nodeAssets.set(absolute, { size: pin.size, sha256: pin.sha256 });
  }
  const assetDigestsBySize = new Map();
  for (const asset of nodeAssets.values()) {
    const digests = assetDigestsBySize.get(asset.size) ?? new Set();
    digests.add(asset.sha256);
    assetDigestsBySize.set(asset.size, digests);
  }
  const ancestryBefore = await stableProcessAncestry();
  const ancestors = new Map(ancestryBefore.generations);
  const namespaceInitGeneration = ancestors.get(1);
  const namespaceInitArgvBytes = await fsp.readFile(procPath(1, "cmdline"));
  const namespaceInitArgv = splitNul(namespaceInitArgvBytes);
  const namespaceInitComm = (await fsp.readFile(procPath(1, "comm"), "utf8")).trim();
  const namespaceInitAfter = await procStat(1);
  requireCondition(
    namespaceInitGeneration.parent_pid === 0 &&
      namespaceInitGeneration.process_group === 0 &&
      namespaceInitGeneration.session_id === 0 &&
      namespaceInitGeneration.start_time_ticks === namespaceInitAfter.start_time_ticks &&
      namespaceInitComm === "bash" &&
      JSON.stringify(namespaceInitArgv) === JSON.stringify(expectedNamespaceInitArgv),
    "PID 1 was not the exact stable namespace init",
  );
  const records = [];
  const entries = await fsp.readdir("/proc", { withFileTypes: true });
  const pids = entries
    .filter((entry) => entry.isDirectory() && /^[1-9][0-9]*$/.test(entry.name))
    .map((entry) => Number(entry.name))
    .sort((left, right) => left - right);
  for (const pid of pids) {
    let observedGeneration = null;
    try {
      const before = await procStatWithState(pid);
      observedGeneration = before;
      const argvBytes = await fsp.readFile(procPath(pid, "cmdline"));
      if (!isCompleteProcVector(argvBytes)) {
        requireCondition(
          await processGenerationExitedWithin(pid, before),
          `live process ${pid} exposed an incomplete cmdline during node inventory`,
        );
        continue;
      }
      const argv = splitNul(argvBytes);
      const comm = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
      const nodeShaped =
        nodeAssets.has(argv[0]) ||
        comm === "memfd:cubikan-s" ||
        ["polkadot", "polkadot-parachain", "polkadot-omni", "polkadot-omni-node"].includes(
          comm,
        ) ||
        (argv.some((argument) => argument === "--chain") &&
          argv.some((argument) => argument === "--base-path") &&
          argv.some((argument) => ["--validator", "--collator"].includes(argument)));
      let executable;
      try {
        executable = await fsp.readlink(procPath(pid, "exe"));
      } catch (error) {
        if (error?.code !== "EACCES") throw error;
        const expectedAncestor = ancestors.get(pid);
        const after = await procStat(pid);
        const argvAfterBytes = await fsp.readFile(procPath(pid, "cmdline"));
        const commAfter = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
        const exactNamespaceInit =
          pid !== 1 ||
          (before.parent_pid === 0 &&
            before.process_group === 0 &&
            before.session_id === 0 &&
            comm === "bash" &&
            JSON.stringify(argv) === JSON.stringify(expectedNamespaceInitArgv));
        requireCondition(
          expectedAncestor !== undefined &&
            !nodeShaped &&
            before.parent_pid === expectedAncestor.parent_pid &&
            before.process_group === expectedAncestor.process_group &&
            before.session_id === expectedAncestor.session_id &&
            before.start_time_ticks === expectedAncestor.start_time_ticks &&
            before.parent_pid === after.parent_pid &&
            before.process_group === after.process_group &&
            before.session_id === after.session_id &&
            before.start_time_ticks === after.start_time_ticks &&
            argvBytes.equals(argvAfterBytes) &&
            comm === commAfter &&
            exactNamespaceInit,
          `inaccessible process ${pid} was not an exact stable non-node harness ancestor`,
        );
        continue;
      }
      let candidate =
        executable === "/memfd:cubikan-sealed-exec-v1 (deleted)" ||
        nodeAssets.has(executable) ||
        nodeShaped;
      if (!candidate) {
        const executableStat = await fsp.stat(procPath(pid, "exe"), { bigint: true });
        const executableSize = safeNumber(executableStat.size, `process ${pid} executable size`);
        const expectedDigests = assetDigestsBySize.get(executableSize);
        if (expectedDigests) {
          const executableBytes = await readBoundedSpecialFile(
            procPath(pid, "exe"),
            executableSize,
          );
          candidate =
            executableBytes.length === executableSize &&
            expectedDigests.has(sha256(executableBytes));
        }
      }
      const after = await procStat(pid);
      const executableAfter = await fsp.readlink(procPath(pid, "exe"));
      requireCondition(
        before.start_time_ticks === after.start_time_ticks &&
          before.parent_pid === after.parent_pid &&
          executable === executableAfter,
        `node process ${pid} generation changed during /proc inventory`,
      );
      if (!candidate) continue;
      records.push({
        pid,
        parent_pid: before.parent_pid,
        start_time_ticks: before.start_time_ticks,
        proc_exe_link: executable,
        proc_cmdline_sha256: sha256(argvBytes),
      });
    } catch (error) {
      if (!["ENOENT", "ESRCH"].includes(error?.code)) throw error;
      requireCondition(
        observedGeneration === null || (await processGenerationExited(pid, observedGeneration)),
        `live process ${pid} became uninspectable during /proc inventory`,
      );
    }
  }
  const ancestryAfter = await stableProcessAncestry();
  requireCondition(
    JSON.stringify(ancestryBefore) === JSON.stringify(ancestryAfter),
    "driver process ancestry changed during /proc inventory",
  );
  return records;
}

async function pvfWorkerProcessInventory(
  workerFixture,
  parentRoles,
  roleContexts,
  knownReparentedGenerations = null,
) {
  const workerPaths = new Map(
    workerFixture.assets.map((asset) => [asset.materialized_path, asset.name]),
  );
  const workerComms = new Set(workerFixture.assets.map((asset) => asset.name.slice(0, 15)));
  const workerSubcommands = new Set(["prepare-worker", "execute-worker"]);
  const captured = new Map();
  const entries = await fsp.readdir("/proc", { withFileTypes: true });
  const pids = entries
    .filter((entry) => entry.isDirectory() && /^[1-9][0-9]*$/.test(entry.name))
    .map((entry) => Number(entry.name))
    .sort((left, right) => left - right);
  pid_loop: for (const pid of pids) {
    let procHandle = null;
    let exeHandle = null;
    let observedGeneration = null;
    try {
      let before = await procStatWithState(pid);
      observedGeneration = before;
      let argvBytes = await fsp.readFile(procPath(pid, "cmdline"));
      let comm = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
      let classificationAfter = await procStatWithState(pid);
      let classificationArgvAfter = await fsp.readFile(procPath(pid, "cmdline"));
      let classificationCommAfter = (
        await fsp.readFile(procPath(pid, "comm"), "utf8")
      ).trim();
      if (classificationAfter.state === "Z") {
        requireCondition(
          sameProcessGeneration(before, classificationAfter) &&
            comm === classificationCommAfter &&
            (await processGenerationExited(pid, before)),
          `process ${pid} did not remain an exact exited generation during PVF classification`,
        );
        continue;
      }
      let classificationStable =
        before.state !== "Z" &&
        sameProcessGeneration(before, classificationAfter) &&
        isCompleteProcVector(argvBytes) &&
        isCompleteProcVector(classificationArgvAfter) &&
        argvBytes.equals(classificationArgvAfter) &&
        comm === classificationCommAfter;
      for (let retry = 0; !classificationStable && retry < 8; retry += 1) {
        await new Promise((resolve) => setTimeout(resolve, 1));
        const retryBefore = await procStatWithState(pid);
        observedGeneration = retryBefore;
        const retryArgv = await fsp.readFile(procPath(pid, "cmdline"));
        const retryComm = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
        const retryAfter = await procStatWithState(pid);
        const retryArgvAfter = await fsp.readFile(procPath(pid, "cmdline"));
        const retryCommAfter = (
          await fsp.readFile(procPath(pid, "comm"), "utf8")
        ).trim();
        if (retryAfter.state === "Z") {
          requireCondition(
            sameProcessGeneration(retryBefore, retryAfter) &&
              retryComm === retryCommAfter &&
              (await processGenerationExited(pid, retryBefore)),
            `process ${pid} did not remain an exact exited generation during PVF classification retry`,
          );
          continue pid_loop;
        }
        classificationStable =
          retryBefore.state !== "Z" &&
          sameProcessGeneration(retryBefore, retryAfter) &&
          isCompleteProcVector(retryArgv) &&
          isCompleteProcVector(retryArgvAfter) &&
          retryArgv.equals(retryArgvAfter) &&
          retryComm === retryCommAfter;
        if (classificationStable) {
          before = retryBefore;
          argvBytes = retryArgv;
          comm = retryComm;
          classificationAfter = retryAfter;
          classificationArgvAfter = retryArgvAfter;
          classificationCommAfter = retryCommAfter;
        }
      }
      if (
        !classificationStable &&
        observedGeneration !== null &&
        (await processGenerationExited(pid, observedGeneration))
      ) {
        continue;
      }
      requireCondition(
        classificationStable &&
          before.state !== "Z" &&
          sameProcessGeneration(before, classificationAfter) &&
          argvBytes.equals(classificationArgvAfter) &&
          comm === classificationCommAfter,
        `process ${pid} did not reach a stable PVF worker classification`,
      );
      const argv = splitNul(argvBytes);
      const workerShaped =
        workerPaths.has(argv[0]) || workerComms.has(comm) || workerSubcommands.has(argv[1]);
      if (!workerShaped) continue;
      procHandle = await fsp.open(procPath(pid, ""), "r");
      exeHandle = await fsp.open(procPath(pid, "exe"), "r");
      const procDescriptorBefore = identityFromStat(await procHandle.stat({ bigint: true }));
      const exeDescriptorBefore = identityFromStat(await exeHandle.stat({ bigint: true }));
      const executable = await fsp.readlink(procPath(pid, "exe"));
      const workerName = workerPaths.get(executable);
      requireCondition(workerName !== undefined, `PVF-shaped process ${pid} used an unknown executable`);
      const expectedSubcommand =
        workerName === "polkadot-prepare-worker" ? "prepare-worker" : "execute-worker";
      requireCondition(
        argv[0] === executable &&
          comm === workerName.slice(0, 15) &&
          argv[1] === expectedSubcommand,
        `PVF-shaped process ${pid} did not match its exact executable, comm, and subcommand`,
      );
      const environmentBytes = await fsp.readFile(procPath(pid, "environ"));
      if (!isCompleteProcVector(environmentBytes)) {
        requireCondition(
          await processGenerationExitedWithin(pid, before),
          `live PVF worker ${pid} exposed an incomplete environment`,
        );
        continue;
      }
      const environmentEntries = splitNul(environmentBytes);
      const environment = {};
      for (const entry of environmentEntries) {
        const separator = entry.indexOf("=");
        requireCondition(separator > 0, `PVF worker ${pid} has malformed environment`);
        const name = entry.slice(0, separator);
        requireCondition(!(name in environment), `PVF worker ${pid} has duplicate environment`);
        environment[name] = entry.slice(separator + 1);
      }
      const security = await pvfProcessSecurityEvidence(pid, workerFixture);
      const procBefore = await procObjectIdentity(pid);
      const exeBefore = await procObjectIdentity(pid, "exe");
      const after = await procStatWithState(pid);
      if (after.state === "Z") {
        requireCondition(
          await processGenerationExited(pid, before),
          `PVF worker ${pid} did not remain an exact exited generation`,
        );
        continue;
      }
      const argvAfterBytes = await fsp.readFile(procPath(pid, "cmdline"));
      const commAfter = (await fsp.readFile(procPath(pid, "comm"), "utf8")).trim();
      const environmentAfterBytes = await fsp.readFile(procPath(pid, "environ"));
      const executableAfter = await fsp.readlink(procPath(pid, "exe"));
      if (
        !isCompleteProcVector(argvAfterBytes) ||
        !isCompleteProcVector(environmentAfterBytes)
      ) {
        requireCondition(
          await processGenerationExitedWithin(pid, before),
          `live PVF worker ${pid} exposed an incomplete process vector`,
        );
        continue;
      }
      const procAfter = await procObjectIdentity(pid);
      const exeAfter = await procObjectIdentity(pid, "exe");
      const procDescriptorAfter = identityFromStat(await procHandle.stat({ bigint: true }));
      const exeDescriptorAfter = identityFromStat(await exeHandle.stat({ bigint: true }));
      const finalStat = await procStatWithState(pid);
      if (finalStat.state === "Z") {
        requireCondition(
          await processGenerationExited(pid, before),
          `PVF worker ${pid} did not remain an exact exited generation after capture`,
        );
        continue;
      }
      requireCondition(
        sameProcessGeneration(before, after) &&
          sameProcessGeneration(before, finalStat) &&
          argvBytes.equals(argvAfterBytes) &&
          comm === commAfter &&
          environmentBytes.equals(environmentAfterBytes) &&
          executable === executableAfter &&
          procBefore.device === procAfter.device &&
          procBefore.inode === procAfter.inode &&
          exeBefore.device === exeAfter.device &&
          exeBefore.inode === exeAfter.inode &&
          procDescriptorBefore.device === procBefore.device &&
          procDescriptorBefore.inode === procBefore.inode &&
          procDescriptorAfter.device === procAfter.device &&
          procDescriptorAfter.inode === procAfter.inode &&
          exeDescriptorBefore.device === exeBefore.device &&
          exeDescriptorBefore.inode === exeBefore.inode &&
          exeDescriptorAfter.device === exeAfter.device &&
          exeDescriptorAfter.inode === exeAfter.inode,
        `PVF worker ${pid} generation changed during /proc inventory`,
      );
      captured.set(pid, {
        kind: workerName === "polkadot-prepare-worker" ? "prepare" : "execute",
        pid,
        parent_pid: before.parent_pid,
        start_time_ticks: before.start_time_ticks,
        proc_exe_link: executable,
        proc_comm: comm,
        argv,
        proc_cmdline_sha256: sha256(argvBytes),
        environment: sortedValue(environment),
        proc_environ_entries: environmentEntries,
        proc_environ_sha256: sha256(environmentBytes),
        security,
        proc_directory_device_before: procBefore.device,
        proc_directory_inode_before: procBefore.inode,
        proc_directory_device_after: procAfter.device,
        proc_directory_inode_after: procAfter.inode,
        proc_directory_descriptor_device_before: procDescriptorBefore.device,
        proc_directory_descriptor_inode_before: procDescriptorBefore.inode,
        proc_directory_descriptor_device_after: procDescriptorAfter.device,
        proc_directory_descriptor_inode_after: procDescriptorAfter.inode,
        executable_device_before: exeBefore.device,
        executable_inode_before: exeBefore.inode,
        executable_device_after: exeAfter.device,
        executable_inode_after: exeAfter.inode,
        executable_descriptor_device_before: exeDescriptorBefore.device,
        executable_descriptor_inode_before: exeDescriptorBefore.inode,
        executable_descriptor_device_after: exeDescriptorAfter.device,
        executable_descriptor_inode_after: exeDescriptorAfter.inode,
      });
    } catch (error) {
      if (!["ENOENT", "ESRCH"].includes(error?.code)) throw error;
      requireCondition(
        observedGeneration === null || (await processGenerationExited(pid, observedGeneration)),
        `live process ${pid} became uninspectable during PVF worker inventory`,
      );
    } finally {
      await Promise.allSettled([procHandle?.close(), exeHandle?.close()]);
    }
  }
  const records = [];
  for (const raw of captured.values()) {
    let ancestor = raw;
    let knownReparentedGeneration = null;
    const ancestry = new Set([raw.pid]);
    while (!parentRoles.has(ancestor.parent_pid)) {
      const parentWorker = captured.get(ancestor.parent_pid);
      if (parentWorker === undefined) {
        knownReparentedGeneration = knownReparentedGenerations?.get(
          `${raw.kind}:${raw.pid}:${raw.start_time_ticks}`,
        );
        const knownAncestorGeneration = knownReparentedGenerations?.get(
          `${ancestor.kind}:${ancestor.pid}:${ancestor.start_time_ticks}`,
        );
        requireCondition(
          knownReparentedGeneration !== undefined &&
            (ancestor.parent_pid === 1 ||
              knownAncestorGeneration?.parent_pid === ancestor.parent_pid),
          `PVF worker ${raw.pid} has an unowned parent`,
        );
        break;
      }
      requireCondition(
        parentWorker.kind === raw.kind && !ancestry.has(parentWorker.pid),
        `PVF worker ${raw.pid} has invalid same-executable ancestry`,
      );
      ancestry.add(parentWorker.pid);
      ancestor = parentWorker;
    }
    const role = knownReparentedGeneration?.role ?? parentRoles.get(ancestor.parent_pid);
    const processClass =
      knownReparentedGeneration?.process_class ??
      (raw.pid === ancestor.pid ? "supervisor" : "job");
    const supervisorPid = knownReparentedGeneration?.supervisor_pid ?? ancestor.pid;
    const subcommand = `${raw.kind}-worker`;
    requireCondition(
      raw.argv[0] === raw.proc_exe_link && raw.argv[1] === subcommand,
      `PVF worker ${raw.pid} executable/subcommand drifted`,
    );
    let index = 2;
    requireCondition(
      raw.argv[index] === "--node-impl-version" &&
        raw.argv[index + 1] === workerFixture.node_impl_version,
      `PVF worker ${raw.pid} node implementation version drifted`,
    );
    index += 2;
    requireCondition(
      raw.argv.length === index + 4 &&
        raw.argv[index] === "--socket-path" &&
        raw.argv[index + 2] === "--worker-dir-path",
      `PVF worker ${raw.pid} argv shape drifted`,
    );
    const socketPath = raw.argv[index + 1];
    const workerDirectory = raw.argv[index + 3];
    requireCondition(
      new RegExp(`^/tmp/pvf-host-${raw.kind}-[A-Za-z0-9]{10}$`).test(socketPath),
      `PVF worker ${raw.pid} socket path grammar drifted`,
    );
    const roleContext = roleContexts.get(role);
    const dataRoot = roleContext?.data_root;
    const databasePath =
      roleContext === undefined
        ? null
        : path.join(
            roleContext.data_root,
            "chains",
            roleContext.chain_spec_id,
            ...workerFixture.database_path_components,
          );
    const artifactsCachePath = databasePath === null ? null : path.join(databasePath, "pvf-artifacts");
    const workerDirectoryName = path.basename(workerDirectory);
    requireCondition(
      dataRoot !== undefined &&
        path.isAbsolute(workerDirectory) &&
        path.normalize(workerDirectory) === workerDirectory &&
        path.dirname(workerDirectory) === artifactsCachePath &&
        new RegExp(`^worker-dir-${raw.kind}-[A-Za-z0-9]+$`).test(workerDirectoryName),
      `PVF worker ${raw.pid} directory escaped its role relay cache`,
    );
    requireCondition(
      Object.keys(raw.environment).length === 1 &&
        typeof raw.environment.RUST_LOG === "string" &&
        raw.environment.RUST_LOG.length <= 4096,
      `PVF worker ${raw.pid} environment is not exactly bounded RUST_LOG`,
    );
    const record = {
      role,
      process_class: processClass,
      supervisor_pid: supervisorPid,
      socket_path: socketPath,
      worker_directory: workerDirectory,
      database_path: databasePath,
      artifacts_cache_path: artifactsCachePath,
      ...raw,
    };
    if (knownReparentedGeneration !== null) {
      const { parent_pid: _knownParentPid, ...knownComparable } = knownReparentedGeneration;
      const { parent_pid: _currentParentPid, ...currentComparable } = record;
      requireCondition(
        canonicalBytes(knownComparable).equals(canonicalBytes(currentComparable)),
        `PVF worker ${raw.pid} did not match its exact retained generation after reparenting`,
      );
    }
    records.push(record);
  }
  records.sort((left, right) => left.pid - right.pid);
  return records;
}

async function pvfHostPathInventory(workerFixture) {
  const records = [];
  const prefixName = path.basename(workerFixture.host_path_prefix);
  const topLevel = (await fsp.readdir("/tmp", { withFileTypes: true }))
    .filter((entry) => entry.name.startsWith(prefixName))
    .sort((left, right) => left.name.localeCompare(right.name, "en"));
  requireCondition(
    topLevel.every((entry) => /^pvf-host-(?:prepare|execute)-[A-Za-z0-9]{10}$/.test(entry.name)),
    "PVF host inventory contains a malformed top-level path",
  );
  const walk = async (absolute) => {
    requireCondition(
      records.length < workerFixture.maximum_observed_host_paths,
      "PVF host path inventory exceeded its retained bound",
    );
    let handle = null;
    let capturedInitialIdentity = false;
    try {
      const beforeStat = await fsp.lstat(absolute, { bigint: true });
      const before = identityFromStat(beforeStat);
      capturedInitialIdentity = true;
      const fileType = beforeStat.isDirectory()
        ? "directory"
        : beforeStat.isSocket()
          ? "socket"
          : beforeStat.isFile()
            ? "regular"
            : beforeStat.isSymbolicLink()
              ? "symbolic-link"
              : "special";
      requireCondition(
        !beforeStat.isSymbolicLink() && ["directory", "socket", "regular"].includes(fileType),
        `PVF host path is symbolic or special: ${absolute}`,
      );
      let descriptorBefore = null;
      let descriptorAfter = null;
      if (beforeStat.isDirectory() || beforeStat.isFile()) {
        handle = await fsp.open(
          absolute,
          fs.constants.O_RDONLY |
            fs.constants.O_CLOEXEC |
            fs.constants.O_NOFOLLOW |
            fs.constants.O_NONBLOCK,
        );
        descriptorBefore = identityFromStat(await handle.stat({ bigint: true }));
      }
      let children = [];
      if (beforeStat.isDirectory())
        children = (await fsp.readdir(`/proc/self/fd/${handle.fd}`)).sort((left, right) =>
          left.localeCompare(right, "en"),
        );
      if (handle) descriptorAfter = identityFromStat(await handle.stat({ bigint: true }));
      const afterStat = await fsp.lstat(absolute, { bigint: true });
      const after = identityFromStat(afterStat);
      requireCondition(
        JSON.stringify(before) === JSON.stringify(after),
        `PVF host path identity changed during inventory: ${absolute}`,
      );
      requireCondition(
        descriptorBefore === null ||
          (JSON.stringify(descriptorBefore) === JSON.stringify(before) &&
            JSON.stringify(descriptorAfter) === JSON.stringify(after)),
        `PVF host path descriptor diverged during inventory: ${absolute}`,
      );
      records.push({
        path: absolute,
        file_type: fileType,
        mode: before.mode,
        device: before.device,
        inode: before.inode,
        owner_uid: safeNumber(beforeStat.uid, "PVF host path owner"),
        mode_after: after.mode,
        device_after: after.device,
        inode_after: after.inode,
        descriptor_device_before: descriptorBefore?.device ?? null,
        descriptor_inode_before: descriptorBefore?.inode ?? null,
        descriptor_device_after: descriptorAfter?.device ?? null,
        descriptor_inode_after: descriptorAfter?.inode ?? null,
      });
      for (const child of children) await walk(path.join(absolute, child));
    } catch (error) {
      if (!capturedInitialIdentity && ["ENOENT", "ENOTDIR"].includes(error?.code)) return;
      throw error;
    } finally {
      await handle?.close().catch(() => {});
    }
  };
  for (const entry of topLevel) await walk(path.join("/tmp", entry.name));
  records.sort((left, right) => left.path.localeCompare(right.path, "en"));
  return records;
}

async function pvfUnixSocketInventory(workerFixture) {
  const text = (await readBoundedSpecialFile("/proc/net/unix", 1_048_576)).toString("utf8");
  const records = [];
  for (const line of text.split("\n").slice(1).filter(Boolean)) {
    const fields = line.trim().split(/\s+/);
    requireCondition(fields.length >= 7, "malformed /proc/net/unix row");
    const socketPath = fields[7] ?? "";
    if (!socketPath.startsWith(workerFixture.host_path_prefix)) continue;
    requireCondition(
      /^\/tmp\/pvf-host-(?:prepare|execute)-[A-Za-z0-9]{10}$/.test(socketPath),
      `PVF Unix socket path grammar drifted: ${socketPath}`,
    );
    requireCondition(
      records.length < workerFixture.maximum_observed_unix_sockets,
      "PVF Unix socket inventory exceeded its retained bound",
    );
    requireCondition(/^(?:0|[1-9][0-9]*)$/.test(fields[6]), "PVF Unix socket inode is not canonical");
    records.push({
      path: socketPath,
      socket_type: fields[4],
      state: fields[5],
      inode: fields[6],
      raw_line: line,
    });
  }
  records.sort((left, right) => left.raw_line.localeCompare(right.raw_line, "en"));
  return records;
}

async function createPvfRuntimeMonitor(workerFixture, parentRoles, roleContexts) {
  const generations = new Map();
  const hostPaths = new Map();
  const unixSockets = new Map();
  const maximumByRoleKind = Object.fromEntries(
    workerFixture.execution_sides.flatMap((side) => {
      const role = side.split(".")[0];
      return [[`${role}.execute`, 0], [`${role}.prepare`, 0]];
    }),
  );
  const maximumJobChildrenBySupervisor = {};
  let sampleCount = 0;
  let maximumTotalWorkers = 0;
  let maximumPrepareWorkers = 0;
  let maximumExecuteWorkers = 0;
  let overflowed = false;
  let failure = null;
  let timer = null;
  let inFlight = Promise.resolve();
  let stopped = false;
  let quiescePromise = null;

  const sample = async (retain, knownReparentedGenerations = null) => {
    const [processes, paths, sockets] = await Promise.all([
      pvfWorkerProcessInventory(
        workerFixture,
        parentRoles,
        roleContexts,
        knownReparentedGenerations,
      ),
      pvfHostPathInventory(workerFixture),
      pvfUnixSocketInventory(workerFixture),
    ]);
    if (!retain) return { processes, paths, sockets };
    sampleCount += 1;
    const prepareCount = processes.filter((record) => record.kind === "prepare").length;
    const executeCount = processes.filter((record) => record.kind === "execute").length;
    maximumTotalWorkers = Math.max(maximumTotalWorkers, processes.length);
    maximumPrepareWorkers = Math.max(maximumPrepareWorkers, prepareCount);
    maximumExecuteWorkers = Math.max(maximumExecuteWorkers, executeCount);
    for (const key of Object.keys(maximumByRoleKind)) {
      const [role, kind] = key.split(".");
      const count = processes.filter(
        (record) =>
          record.role === role && record.kind === kind && record.process_class === "supervisor",
      ).length;
      maximumByRoleKind[key] = Math.max(maximumByRoleKind[key], count);
      requireCondition(
        count <= workerFixture.maximum_supervisors_per_role_kind,
        `PVF ${key} supervisor pool exceeded its cap`,
      );
    }
    for (const supervisor of processes.filter((record) => record.process_class === "supervisor")) {
      const key = `${supervisor.role}.${supervisor.kind}.${supervisor.pid}`;
      const count = processes.filter(
        (record) =>
          record.process_class === "job" && record.supervisor_pid === supervisor.pid,
      ).length;
      maximumJobChildrenBySupervisor[key] = Math.max(
        maximumJobChildrenBySupervisor[key] ?? 0,
        count,
      );
      requireCondition(
        count <= workerFixture.maximum_job_children_per_supervisor,
        `PVF ${key} job children exceeded the subordinate cap`,
      );
    }
    requireCondition(
      processes.length <= workerFixture.maximum_total_worker_processes,
      "aggregate PVF supervisor/job process inventory exceeded exact caps",
    );
    for (const record of processes) {
      const key = `${record.role}:${record.kind}:${record.pid}:${record.start_time_ticks}`;
      generations.set(key, record);
    }
    for (const record of paths)
      hostPaths.set(`${record.path}:${record.device}:${record.inode}`, record);
    for (const record of sockets) unixSockets.set(record.raw_line, record);
    requireCondition(
      generations.size <= workerFixture.maximum_observed_generations &&
        hostPaths.size <= workerFixture.maximum_observed_host_paths &&
        unixSockets.size <= workerFixture.maximum_observed_unix_sockets,
      "PVF runtime monitor exceeded a retained evidence bound",
    );
    return { processes, paths, sockets };
  };
  const schedule = () => {
    if (stopped) return;
    timer = setTimeout(() => {
      inFlight = inFlight
        .then(() => sample(true))
        .catch((error) => {
          overflowed = true;
          failure ??= error;
        })
        .finally(schedule);
    }, workerFixture.monitor_interval_milliseconds);
  };
  await sample(true);
  schedule();
  const quiesce = () => {
    quiescePromise ??= (async () => {
      stopped = true;
      if (timer) clearTimeout(timer);
      await inFlight;
      if (failure) throw failure;
      await sample(true);
    })();
    return quiescePromise;
  };
  return {
    async sampleNow() {
      await inFlight;
      if (failure) throw failure;
      await sample(true);
    },
    async quiesce() {
      await quiesce();
    },
    async stopAndEvidence() {
      await quiesce();
      const knownReparentedGenerations = new Map(
        [...generations.values()].map((record) => [
          `${record.kind}:${record.pid}:${record.start_time_ticks}`,
          record,
        ]),
      );
      requireCondition(
        knownReparentedGenerations.size === generations.size,
        "retained PVF generation identities collided",
      );
      let postStop = null;
      await waitFor(
        async () => {
          postStop = await sample(false, knownReparentedGenerations);
          return (
            postStop.processes.length === 0 &&
            postStop.paths.length === 0 &&
            postStop.sockets.length === 0
          );
        },
        "PVF process/path/socket teardown",
        30_000,
        workerFixture.monitor_interval_milliseconds,
      );
      requireCondition(
        postStop !== null &&
          postStop.processes.length === 0 &&
          postStop.paths.length === 0 &&
          postStop.sockets.length === 0,
        "PVF process/path/socket teardown did not reach an exact empty inventory",
      );
      return {
        interval_milliseconds: workerFixture.monitor_interval_milliseconds,
        sample_count: sampleCount,
        maximum_total_workers: maximumTotalWorkers,
        maximum_prepare_workers: maximumPrepareWorkers,
        maximum_execute_workers: maximumExecuteWorkers,
        maximum_by_role_kind: sortedValue(maximumByRoleKind),
        maximum_job_children_by_supervisor: sortedValue(maximumJobChildrenBySupervisor),
        observed_generations: [...generations.values()].sort((left, right) =>
          `${left.role}:${left.kind}:${left.pid}:${left.start_time_ticks}`.localeCompare(
            `${right.role}:${right.kind}:${right.pid}:${right.start_time_ticks}`,
            "en",
          ),
        ),
        observed_host_paths: [...hostPaths.values()].sort((left, right) =>
          left.path.localeCompare(right.path, "en"),
        ),
        observed_unix_sockets: [...unixSockets.values()].sort((left, right) =>
          left.raw_line.localeCompare(right.raw_line, "en"),
        ),
        post_stop_processes: postStop.processes,
        post_stop_host_paths: postStop.paths,
        post_stop_unix_sockets: postStop.sockets,
        overflowed,
      };
    },
    async cancel() {
      stopped = true;
      if (timer) clearTimeout(timer);
      await inFlight;
    },
  };
}

async function waitFor(predicate, label, timeout = 60_000, interval = 25) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, interval));
  }
  fail(`${label} timed out`);
}

function isCompleteProcVector(bytes) {
  return bytes.length > 0 && bytes.at(-1) === 0;
}

function splitNul(bytes) {
  requireCondition(isCompleteProcVector(bytes), "proc vector lacks terminal NUL");
  return bytes
    .subarray(0, bytes.length - 1)
    .toString("utf8")
    .split("\0");
}

function flagValues(argv, flag) {
  const values = [];
  for (let index = 0; index + 1 < argv.length; index += 1) {
    if (argv[index] === flag) values.push(argv[index + 1]);
  }
  return values;
}

function splitNodeArgv(argv) {
  const separator = argv.indexOf("--");
  return separator < 0
    ? { primary: argv, relay: null }
    : { primary: argv.slice(0, separator), relay: argv.slice(separator + 1) };
}

async function procSeals(pid, executableHandle, executableIdentity) {
  requireCondition(
    Number.isInteger(executableHandle.fd) && executableHandle.fd > 2,
    `node ${pid} executable descriptor is invalid`,
  );
  const descriptorIdentity = identityFromStat(
    await executableHandle.stat({ bigint: true }),
  );
  requireCondition(
    descriptorIdentity.device === executableIdentity.device &&
      descriptorIdentity.inode === executableIdentity.inode,
    `node ${pid} executable descriptor identity drifted before seal inspection`,
  );
  const probeSource = `import errno, fcntl, os, stat, sys
path, expected_device, expected_inode = sys.argv[1:]
descriptor = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
try:
    opened = os.fstat(descriptor)
    if not stat.S_ISREG(opened.st_mode):
        raise SystemExit(10)
    if opened.st_dev != int(expected_device) or opened.st_ino != int(expected_inode):
        raise SystemExit(11)
    required = fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL
    if fcntl.fcntl(descriptor, fcntl.F_GET_SEALS) != required:
        raise SystemExit(12)
    try:
        writable = os.open(path, os.O_RDWR | os.O_CLOEXEC)
    except OSError as error:
        if error.errno not in {errno.EPERM, errno.EACCES, errno.ETXTBSY}:
            raise SystemExit(13)
    else:
        try:
            os.pwrite(writable, b"\\x00", 0)
        except OSError as error:
            if error.errno != errno.EPERM:
                raise SystemExit(14)
        else:
            raise SystemExit(15)
        finally:
            os.close(writable)
    print("seal-mask=0f write=denied")
finally:
    os.close(descriptor)`;
  const probe = await runProcess(
    "/usr/bin/python3.14",
    [
      "-I",
      "-S",
      "-c",
      probeSource,
      procPath(pid, "exe"),
      executableIdentity.device,
      executableIdentity.inode,
    ],
    {
      cwd: process.env.PWD,
      env: {
        HOME: "/home/charles",
        LANG: "C",
        LC_ALL: "C",
        PATH: "/usr/bin:/bin",
        PWD: process.env.PWD,
        SHLVL: "0",
        TZ: "UTC",
      },
      label: `node ${pid} sealed executable probe`,
      limit: 4096,
      timeout: 5_000,
    },
  );
  requireCondition(
    probe.code === 0 &&
      probe.signal === null &&
      probe.stdout.equals(Buffer.from("seal-mask=0f write=denied\n")) &&
      probe.stderr.length === 0,
    `node ${pid} sealed executable probe rejected its exact memfd`,
  );
  const descriptorIdentityAfter = identityFromStat(
    await executableHandle.stat({ bigint: true }),
  );
  requireCondition(
    JSON.stringify(descriptorIdentityAfter) === JSON.stringify(descriptorIdentity),
    `node ${pid} executable descriptor identity drifted during seal inspection`,
  );
  return {
    seals: ["seal", "shrink", "grow", "write"],
    post_seal_write_denied: true,
  };
}

async function rpcGenesisHash(api) {
  return (await api.rpc.chain.getBlockHash(0)).toHex();
}

async function rpcFinalized(api) {
  const hash = (await api.rpc.chain.getFinalizedHead()).toHex();
  const header = await api.rpc.chain.getHeader(hash);
  return { hash, number: header.number.toNumber(), header: codecJson(header) };
}

async function rpcBest(api) {
  const header = await api.rpc.chain.getHeader();
  return {
    hash: header.hash.toHex(),
    number: header.number.toNumber(),
    header: codecJson(header),
  };
}

function conciseHead(head) {
  return { number: head.number, hash: head.hash };
}

async function relayStateAt(relayApi, paraRegistry, relayHash, policy, label) {
  const [activeConfig, registeredParas, paraHeadOption] = await withTimeout(
    label,
    policy.rpc_timeout_milliseconds,
    () =>
      Promise.all([
        relayApi.query.configuration.activeConfig.at(relayHash),
        relayApi.query.paras.parachains.at(relayHash),
        relayApi.query.paras.heads.at(relayHash, policy.para_id),
      ]),
  );
  const schedulerCores = safeNumber(
    activeConfig.schedulerParams.numCores.toString(),
    "active scheduler core count",
  );
  const paraRegistered = registeredParas.some(
    (paraId) => safeNumber(paraId.toString(), "registered para ID") === policy.para_id,
  );
  requireCondition(paraRegistered, "configured parachain is absent from the relay finalized state");
  requireCondition(
    schedulerCores === policy.expected_scheduler_cores,
    "active relay scheduler core count differs from the readiness policy",
  );
  requireCondition(
    paraHeadOption.isSome,
    "configured parachain has no head in the relay finalized state",
  );
  const headData = Buffer.from(paraHeadOption.unwrap().toU8a(true));
  requireCondition(
    headData.length > 0 && headData.length <= 4096,
    "relay parachain head data is empty or over bound",
  );
  const header = paraRegistry.createType("Header", headData);
  return {
    relay_para_header: {
      number: header.number.toNumber(),
      hash: header.hash.toHex(),
      head_data_sha256: sha256(headData),
    },
    scheduler_cores: schedulerCores,
    active_config_scale_sha256: sha256(Buffer.from(activeConfig.toU8a())),
  };
}

async function waitForPreMutationReadiness(
  relayApiA,
  relayApiB,
  apiA,
  apiB,
  policy,
) {
  const started = Date.now();
  const deadline = started + policy.timeout_milliseconds;
  let sampleCount = 0;
  let baseline = null;
  let lastObserved = { phase: "baseline" };
  while (Date.now() < deadline && baseline === null) {
    sampleCount += 1;
    const [relayA, relayB, collatorA, collatorB] = await withTimeout(
      `pre-mutation baseline sample ${sampleCount} head RPCs`,
      policy.rpc_timeout_milliseconds,
      () =>
        Promise.all([
          rpcFinalized(relayApiA),
          rpcFinalized(relayApiB),
          rpcFinalized(apiA),
          rpcFinalized(apiB),
        ]),
    );
    lastObserved = {
      phase: "baseline",
      relay_a_finalized: conciseHead(relayA),
      relay_b_finalized: conciseHead(relayB),
      collator_a_finalized: conciseHead(collatorA),
      collator_b_finalized: conciseHead(collatorB),
    };
    if (relayA.number === relayB.number && relayA.hash === relayB.hash) {
      const relayState = await relayStateAt(
        relayApiA,
        apiA.registry,
        relayA.hash,
        policy,
        `pre-mutation baseline sample ${sampleCount} relay-state RPCs`,
      );
      baseline = {
        relay_finalized: conciseHead(relayA),
        relay_para_header: relayState.relay_para_header,
        collator_a_finalized: conciseHead(collatorA),
        collator_b_finalized: conciseHead(collatorB),
        para_registered: true,
        relay_endpoints_equal: true,
        scheduler_cores: relayState.scheduler_cores,
        active_config_scale_sha256: relayState.active_config_scale_sha256,
      };
      break;
    }
    const remaining = deadline - Date.now();
    if (remaining > 0) {
      await new Promise((resolve) =>
        setTimeout(resolve, Math.min(policy.poll_interval_milliseconds, remaining)),
      );
    }
  }
  requireCondition(
    baseline !== null,
    `pre-mutation baseline relay convergence timed out after ${sampleCount} samples; last_observed=${JSON.stringify(lastObserved)}`,
  );
  while (Date.now() < deadline) {
    sampleCount += 1;
    const [relayA, relayB, collatorA, collatorB, bestA, bestB] = await withTimeout(
      `pre-mutation readiness sample ${sampleCount} head RPCs`,
      policy.rpc_timeout_milliseconds,
      () =>
        Promise.all([
          rpcFinalized(relayApiA),
          rpcFinalized(relayApiB),
          rpcFinalized(apiA),
          rpcFinalized(apiB),
          rpcBest(apiA),
          rpcBest(apiB),
        ]),
    );
    const relayEqual = relayA.number === relayB.number && relayA.hash === relayB.hash;
    const collatorEqual =
      collatorA.number === collatorB.number && collatorA.hash === collatorB.hash;
    const relayProgressMinimum =
      baseline.relay_finalized.number + policy.required_progress_blocks;
    const paraProgressMinimum =
      baseline.relay_para_header.number + policy.required_progress_blocks;
    const collatorProgressMinimum =
      Math.max(
        baseline.collator_a_finalized.number,
        baseline.collator_b_finalized.number,
      ) + policy.required_progress_blocks;
    const gapA = bestA.number - collatorA.number;
    const gapB = bestB.number - collatorB.number;
    lastObserved = {
      baseline,
      relay_a_finalized: conciseHead(relayA),
      relay_b_finalized: conciseHead(relayB),
      collator_a_finalized: conciseHead(collatorA),
      collator_b_finalized: conciseHead(collatorB),
      collator_a_best: { ...conciseHead(bestA), finalized_gap: gapA },
      collator_b_best: { ...conciseHead(bestB), finalized_gap: gapB },
      relay_progress_minimum: relayProgressMinimum,
      para_progress_minimum: paraProgressMinimum,
      collator_progress_minimum: collatorProgressMinimum,
    };
    if (relayEqual) {
      const relayState = await relayStateAt(
        relayApiA,
        apiA.registry,
        relayA.hash,
        policy,
        `pre-mutation readiness sample ${sampleCount} relay-state RPCs`,
      );
      const relayParaHeader = relayState.relay_para_header;
      lastObserved = {
        ...lastObserved,
        para_registered: true,
        scheduler_cores: relayState.scheduler_cores,
        relay_para_header: relayState.relay_para_header,
      };
      if (
        collatorEqual &&
        relayA.number >= relayProgressMinimum &&
        relayParaHeader.number >= policy.minimum_finalized_number &&
        relayParaHeader.number >= paraProgressMinimum &&
        collatorA.number >= policy.minimum_finalized_number &&
        collatorA.number >= collatorProgressMinimum &&
        relayParaHeader.number <= collatorA.number &&
        relayParaHeader.number <= bestA.number &&
        relayParaHeader.number <= bestB.number &&
        gapA >= 0 &&
        gapB >= 0 &&
        gapA <= policy.maximum_best_finalized_gap &&
        gapB <= policy.maximum_best_finalized_gap
      ) {
        const [historicalA, historicalB, atA, atB] =
          await withTimeout(
            `pre-mutation readiness sample ${sampleCount} parachain-history RPCs`,
            policy.rpc_timeout_milliseconds,
            () =>
              Promise.all([
                apiA.rpc.chain.getBlockHash(relayParaHeader.number),
                apiB.rpc.chain.getBlockHash(relayParaHeader.number),
                apiA.at(collatorA.hash),
                apiB.at(collatorB.hash),
              ]),
          );
        const [authoritiesCodecA, authoritiesCodecB] = await withTimeout(
          `pre-mutation readiness sample ${sampleCount} Aura runtime API RPCs`,
          policy.rpc_timeout_milliseconds,
          () => Promise.all([atA.call.auraApi.authorities(), atB.call.auraApi.authorities()]),
        );
        requireCondition(
          authoritiesCodecA.length <= policy.expected_distinct_aura_authorities &&
            authoritiesCodecB.length <= policy.expected_distinct_aura_authorities,
          "Aura runtime API authority result exceeds the readiness evidence bound",
        );
        const historicalBlockHashes = {
          collator_a: historicalA.toHex(),
          collator_b: historicalB.toHex(),
        };
        const authoritiesA = authoritiesCodecA.map((authority) => authority.toHex());
        const authoritiesB = authoritiesCodecB.map((authority) => authority.toHex());
        const authoritiesEqual = JSON.stringify(authoritiesA) === JSON.stringify(authoritiesB);
        const distinctAuthorityCount = new Set(authoritiesA).size;
        lastObserved = {
          ...lastObserved,
          historical_block_hashes: historicalBlockHashes,
          authority_count_a: authoritiesA.length,
          authority_count_b: authoritiesB.length,
          authorities_equal: authoritiesEqual,
          distinct_authority_count: distinctAuthorityCount,
        };
        if (
          historicalBlockHashes.collator_a === relayParaHeader.hash &&
          historicalBlockHashes.collator_b === relayParaHeader.hash &&
          authoritiesEqual &&
          authoritiesA.length === policy.expected_distinct_aura_authorities &&
          distinctAuthorityCount === policy.expected_distinct_aura_authorities &&
          Date.now() <= deadline
        ) {
          return {
            para_id: policy.para_id,
            baseline,
            elapsed_milliseconds: Date.now() - started,
            sample_count: sampleCount,
            relay_finalized: conciseHead(relayA),
            relay_para_header: relayParaHeader,
            para_registered: true,
            relay_endpoints_equal: true,
            scheduler_cores: relayState.scheduler_cores,
            active_config_scale_sha256: relayState.active_config_scale_sha256,
            collator_finalized: conciseHead(collatorA),
            collator_finalized_endpoints_equal: true,
            historical_block_hashes: historicalBlockHashes,
            collator_a_best: { ...conciseHead(bestA), finalized_gap: gapA },
            collator_b_best: { ...conciseHead(bestB), finalized_gap: gapB },
            authorities_a: authoritiesA,
            authorities_b: authoritiesB,
            authorities_equal: true,
            distinct_authority_count: distinctAuthorityCount,
          };
        }
      }
    }
    const remaining = deadline - Date.now();
    if (remaining > 0) {
      await new Promise((resolve) =>
        setTimeout(resolve, Math.min(policy.poll_interval_milliseconds, remaining)),
      );
    }
  }
  fail(
    `pre-mutation readiness timed out after ${sampleCount} samples; last_observed=${JSON.stringify(lastObserved)}`,
  );
}

async function storageHex(api, key, blockHash) {
  const value = await api.rpc.state.getStorage(key, blockHash);
  return value.isNone ? null : value.unwrap().toHex();
}

async function runtimeCodeHash(api, genesisHash) {
  const value = await storageHex(api, RUNTIME_CODE_KEY, genesisHash);
  requireCondition(typeof value === "string" && value.startsWith("0x"), "runtime :code missing");
  return sha256(Buffer.from(value.slice(2), "hex"));
}

async function chainSpecEvidence(specPath, api, observedRpcEndpoint = null) {
  const { evidence, bytes } = await openedFileEvidence(specPath, MAX_RAW_CHAIN_SPEC_BYTES);
  const text = bytes.toString("utf8");
  rejectDuplicateJsonKeys(text, specPath);
  const spec = JSON.parse(text);
  requireCondition(
    Object.prototype.hasOwnProperty.call(spec, "bootNodes") && Array.isArray(spec.bootNodes),
    `${specPath} lacks one top-level bootNodes array`,
  );
  requireCondition(
    typeof spec.id === "string" && /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(spec.id),
    `${specPath} has no canonical chain-spec id`,
  );
  const topLevelBootnodes = spec.bootNodes;
  const chainSpecId = spec.id;
  delete spec.bootNodes;
  const genesisHash = await rpcGenesisHash(api);
  return {
    raw_path: specPath,
    raw_size: bytes.length,
    raw_sha256: evidence.sha256,
    chain_spec_id: chainSpecId,
    duplicate_keys_rejected: true,
    top_level_bootnodes: topLevelBootnodes,
    removed_top_level_members: ["bootNodes"],
    canonicalization: SPEC_CANONICALIZATION,
    genesis_payload_sha256: canonicalSha256(spec),
    live_genesis_hash: genesisHash,
    live_runtime_code_sha256: await runtimeCodeHash(api, genesisHash),
    observed_rpc_endpoint: observedRpcEndpoint,
  };
}

async function nodeEvidence(roleFixture, pid, repoRoot, primaryApi, relaySide = null) {
  const procHandle = await fsp.open(procPath(pid, ""), "r");
  const exeHandle = await fsp.open(procPath(pid, "exe"), "r");
  try {
    const procDescriptorBefore = identityFromStat(await procHandle.stat({ bigint: true }));
    const exeDescriptorBefore = identityFromStat(await exeHandle.stat({ bigint: true }));
    const argvBytes = await fsp.readFile(procPath(pid, "cmdline"));
    const argv = splitNul(argvBytes);
    const environmentBytes = await fsp.readFile(procPath(pid, "environ"));
    const environmentEntries = splitNul(environmentBytes);
    const environment = {};
    for (const entry of environmentEntries) {
      const separator = entry.indexOf("=");
      requireCondition(separator > 0, `node ${roleFixture.role} has malformed environment`);
      const name = entry.slice(0, separator);
      requireCondition(!(name in environment), `node ${roleFixture.role} has duplicate environment`);
      environment[name] = entry.slice(separator + 1);
    }
    const privileges = await processPrivilegeEvidence(pid);
    const statBefore = await procStat(pid);
    const procBefore = await procObjectIdentity(pid);
    const exeLink = await fsp.readlink(procPath(pid, "exe"));
    const exeStat = await fsp.stat(procPath(pid, "exe"), { bigint: true });
    const exeBefore = await procObjectIdentity(pid, "exe");
    const exeBytes = await exeHandle.readFile();
    const sealed = await procSeals(pid, exeHandle, exeBefore);
    const split = splitNodeArgv(argv);
    const primaryChain = flagValues(split.primary, "--chain");
    requireCondition(primaryChain.length === 1, `${roleFixture.role} primary chain path is not exact`);
    const relayChain = split.relay ? flagValues(split.relay, "--chain") : [];
    const primarySpec = await chainSpecEvidence(primaryChain[0], primaryApi);
    const livePeerId = (await primaryApi.rpc.system.localPeerId()).toString();
    requireCondition(
      livePeerId === roleFixture.peer_id,
      `${roleFixture.role} live system_localPeerId differs from its independent fixture`,
    );
    const relaySpec = relaySide
      ? await chainSpecEvidence(
          (() => {
            requireCondition(relayChain.length === 1, `${roleFixture.role} relay chain path is not exact`);
            return relayChain[0];
          })(),
          relaySide.api,
          relaySide.endpoint,
        )
      : null;
    const statAfter = await procStat(pid);
    const procAfter = await procObjectIdentity(pid);
    const exeAfter = await procObjectIdentity(pid, "exe");
    const procDescriptorAfter = identityFromStat(await procHandle.stat({ bigint: true }));
    const exeDescriptorAfter = identityFromStat(await exeHandle.stat({ bigint: true }));
    requireCondition(
      statBefore.parent_pid === process.pid && statAfter.parent_pid === statBefore.parent_pid,
      `${roleFixture.role} parent process identity diverged`,
    );
    requireCondition(
      procDescriptorBefore.device === procBefore.device &&
        procDescriptorBefore.inode === procBefore.inode &&
        procDescriptorAfter.device === procAfter.device &&
        procDescriptorAfter.inode === procAfter.inode,
      `${roleFixture.role} retained /proc directory identity diverged`,
    );
    requireCondition(
      exeDescriptorBefore.device === exeBefore.device &&
        exeDescriptorBefore.inode === exeBefore.inode &&
        exeDescriptorAfter.device === exeAfter.device &&
        exeDescriptorAfter.inode === exeAfter.inode,
      `${roleFixture.role} retained executable identity diverged`,
    );
    const primaryDataDirectories = flagValues(split.primary, "--base-path");
    const relayDataDirectories = split.relay ? flagValues(split.relay, "--base-path") : [];
    requireCondition(
      primaryDataDirectories.length === 1 &&
        relayDataDirectories.length === (split.relay ? 1 : 0),
      `${roleFixture.role} base-path ownership is not exact`,
    );
    const dataDirectories = await Promise.all(
      [
        ["primary", primaryDataDirectories[0]],
        ...(split.relay ? [["relay-side", relayDataDirectories[0]]] : []),
      ].map(async ([side, directoryPath]) => {
        const identity = identityFromStat(await fsp.stat(directoryPath, { bigint: true }));
        requireCondition(identity.mode === "0700", `${roleFixture.role} ${side} base path is not private`);
        return {
          side,
          path: directoryPath,
          mode: identity.mode,
          device: identity.device,
          inode: identity.inode,
        };
      }),
    );
    return {
    role: roleFixture.role,
    kind: roleFixture.kind,
    generated_name: roleFixture.generated_name,
    dev_seed: roleFixture.dev_seed,
    node_key: roleFixture.node_key,
    peer_id: livePeerId,
    pid,
    parent_pid: statBefore.parent_pid,
    reviewed_source_asset: path.join(repoRoot, roleFixture.binary),
    argv,
    proc_cmdline_sha256: sha256(argvBytes),
    environment: sortedValue(environment),
    proc_environ_entries: environmentEntries,
    proc_environ_sha256: sha256(environmentBytes),
    privileges,
    proc_exe_link: exeLink,
    proc_exe_size: safeNumber(exeStat.size, "proc exe size"),
    proc_exe_sha256: sha256(exeBytes),
    proc_exe_mode: (safeNumber(exeStat.mode, "proc exe mode") & 0o7777)
      .toString(8)
      .padStart(4, "0"),
    proc_exe_seals: sealed.seals,
    post_seal_write_denied: sealed.post_seal_write_denied,
    process_start_time_ticks_before: statBefore.start_time_ticks,
    process_start_time_ticks_after: statAfter.start_time_ticks,
    proc_directory_device_before: procBefore.device,
    proc_directory_inode_before: procBefore.inode,
    proc_directory_device_after: procAfter.device,
    proc_directory_inode_after: procAfter.inode,
      proc_directory_descriptor_device_before: procDescriptorBefore.device,
      proc_directory_descriptor_inode_before: procDescriptorBefore.inode,
      proc_directory_descriptor_device_after: procDescriptorAfter.device,
      proc_directory_descriptor_inode_after: procDescriptorAfter.inode,
      executable_device_before: exeBefore.device,
      executable_inode_before: exeBefore.inode,
      executable_device_after: exeAfter.device,
      executable_inode_after: exeAfter.inode,
      executable_descriptor_device_before: exeDescriptorBefore.device,
      executable_descriptor_inode_before: exeDescriptorBefore.inode,
      executable_descriptor_device_after: exeDescriptorAfter.device,
      executable_descriptor_inode_after: exeDescriptorAfter.inode,
    authenticated_archive_node_evidence: roleFixture.archive,
    data_directory: primaryDataDirectories[0],
    data_directory_mode: dataDirectories[0].mode,
    data_directories: dataDirectories,
    listener_addresses: roleFixture.listeners,
    primary_bootnodes: flagValues(split.primary, "--bootnodes"),
    relay_side_bootnodes: split.relay ? flagValues(split.relay, "--bootnodes") : [],
    archive_flags: roleFixture.archive
      ? ["--blocks-pruning", "archive", "--state-pruning", "archive"]
      : [],
    primary_runtime_sha256: primarySpec.live_runtime_code_sha256,
    primary_chain_spec: primarySpec,
    relay_side_runtime_sha256: relaySpec?.live_runtime_code_sha256 ?? null,
      relay_side_chain_spec: relaySpec,
    };
  } finally {
    await Promise.allSettled([procHandle.close(), exeHandle.close()]);
  }
}

async function exactTmpfsBindMountEvidence(root, label) {
  const mountInfoText = (
    await readBoundedSpecialFile("/proc/self/mountinfo", 1_048_576)
  ).toString("utf8");
  const mountLines = mountInfoText
    .split("\n")
    .filter((line) => line.split(" ")[4] === root);
  requireCondition(mountLines.length === 1, `${label} is not one exact mount`);
  const mountFields = mountLines[0].split(" ");
  const separator = mountFields.indexOf("-");
  requireCondition(
    separator >= 6 && mountFields.length >= separator + 4,
    `malformed ${label} mountinfo row`,
  );
  const mountOptions = mountFields[5].split(",").sort();
  const superOptions = mountFields[separator + 3].split(",").sort();
  const mount = {
    raw_mountinfo_line: mountLines[0],
    mount_id: Number(mountFields[0]),
    parent_id: Number(mountFields[1]),
    major_minor: mountFields[2],
    root: mountFields[3],
    mount_point: mountFields[4],
    mount_options: mountOptions,
    optional_fields: mountFields.slice(6, separator),
    filesystem_type: mountFields[separator + 1],
    mount_source: mountFields[separator + 2],
    super_options: superOptions,
    bind: mountFields[3] !== "/",
    read_only: mountOptions.includes("ro"),
    nodev: mountOptions.includes("nodev"),
    nosuid: mountOptions.includes("nosuid"),
    executable: !mountOptions.includes("noexec"),
  };
  requireCondition(
    mount.mount_id > 0 &&
      mount.parent_id > 0 &&
      mount.mount_point === root &&
      mount.filesystem_type === "tmpfs" &&
      mount.bind &&
      mount.read_only &&
      mount.nodev &&
      mount.nosuid &&
      mount.executable,
    `${label} is not an exact read-only executable nodev/nosuid tmpfs bind`,
  );
  return mount;
}

async function readOnlyWriteProbe(root, label) {
  let errno = null;
  const probe = path.join(root, `.driver-write-probe-${process.pid}`);
  try {
    const handle = await fsp.open(probe, "wx", 0o600);
    await handle.close();
    await fsp.unlink(probe);
  } catch (error) {
    errno = error?.code ?? null;
  }
  requireCondition(!fs.existsSync(probe), `${label} write probe left a file behind`);
  return errno;
}

async function pvfWorkerStaticEvidence(workerFixture, initialNodes) {
  const root = await fsp.realpath(workerFixture.root);
  requireCondition(root === workerFixture.root, "PVF worker root is not canonical");
  const rootStat = await fsp.lstat(root, { bigint: true });
  const rootIdentity = identityFromStat(rootStat);
  const filesystem = await fsp.statfs(root, { bigint: true });
  const filesystemMagic = filesystem.type.toString(16);
  const entries = (await fsp.readdir(root)).sort((left, right) =>
    left.localeCompare(right, "en"),
  );
  requireCondition(
    rootStat.isDirectory() &&
      !rootStat.isSymbolicLink() &&
      rootIdentity.mode === workerFixture.directory_mode &&
      filesystemMagic === workerFixture.filesystem_magic &&
      JSON.stringify(entries) ===
        JSON.stringify(workerFixture.assets.map((asset) => asset.name).sort()),
    "PVF worker root type/mode/filesystem/inventory drifted",
  );

  const mountInfoText = (
    await readBoundedSpecialFile("/proc/self/mountinfo", 1_048_576)
  ).toString("utf8");
  const mountLines = mountInfoText
    .split("\n")
    .filter((line) => line.split(" ")[4] === root);
  requireCondition(mountLines.length === 1, "PVF worker root is not one exact mount");
  const mountFields = mountLines[0].split(" ");
  const separator = mountFields.indexOf("-");
  requireCondition(separator >= 6 && mountFields.length >= separator + 4, "malformed PVF mountinfo row");
  const mountOptions = mountFields[5].split(",").sort();
  const superOptions = mountFields[separator + 3].split(",").sort();
  const mount = {
    raw_mountinfo_line: mountLines[0],
    mount_id: Number(mountFields[0]),
    parent_id: Number(mountFields[1]),
    major_minor: mountFields[2],
    root: mountFields[3],
    mount_point: mountFields[4],
    mount_options: mountOptions,
    optional_fields: mountFields.slice(6, separator),
    filesystem_type: mountFields[separator + 1],
    mount_source: mountFields[separator + 2],
    super_options: superOptions,
    bind: mountFields[3] !== "/",
    read_only: mountOptions.includes("ro"),
    nodev: mountOptions.includes("nodev"),
    nosuid: mountOptions.includes("nosuid"),
    executable: !mountOptions.includes("noexec"),
  };
  requireCondition(
    mount.mount_id > 0 &&
      mount.parent_id > 0 &&
      mount.mount_point === root &&
      mount.filesystem_type === "tmpfs" &&
      mount.bind &&
      mount.read_only &&
      mount.nodev &&
      mount.nosuid &&
      mount.executable,
    "PVF worker mount is not exact read-only executable nodev/nosuid tmpfs bind",
  );

  let writeProbeErrno = null;
  const probe = path.join(root, `.driver-write-probe-${process.pid}`);
  try {
    const handle = await fsp.open(probe, "wx", 0o600);
    await handle.close();
    await fsp.unlink(probe);
  } catch (error) {
    writeProbeErrno = error?.code ?? null;
  }
  requireCondition(
    writeProbeErrno === workerFixture.write_probe_errno && !fs.existsSync(probe),
    "PVF worker root did not produce exact EROFS write denial",
  );

  const assets = [];
  for (const expected of workerFixture.assets) {
    const opened = await openedFileEvidence(expected.materialized_path);
    const assetStat = await fsp.stat(expected.materialized_path, { bigint: true });
    requireCondition(
      opened.evidence.path_before.mode === expected.mode &&
        opened.evidence.path_before.size === expected.size &&
        opened.evidence.path_before.link_count === 1 &&
        opened.evidence.sha256 === expected.sha256 &&
        opened.evidence.regular_file &&
        !opened.evidence.symbolic_link,
      `PVF worker asset drifted: ${expected.name}`,
    );
    assets.push({
      name: expected.name,
      path: expected.materialized_path,
      owner_uid: safeNumber(assetStat.uid, "PVF worker owner"),
      read: opened.evidence,
    });
  }

  const executionSides = workerFixture.execution_sides.map((contract) => {
    const [role, side] = contract.split(".");
    const split = splitNodeArgv(initialNodes[role].argv);
    const segment = side === "primary" ? split.primary : split.relay;
    requireCondition(segment !== null, `PVF execution side is absent: ${contract}`);
    const observed = {
      role,
      side,
      workers_path_values: flagValues(segment, workerFixture.workers_path_flag),
      database_values: flagValues(segment, "--database"),
      execute_workers_max_num_values: flagValues(segment, "--execute-workers-max-num"),
      prepare_workers_soft_max_num_values: flagValues(
        segment,
        "--prepare-workers-soft-max-num",
      ),
      prepare_workers_hard_max_num_values: flagValues(
        segment,
        "--prepare-workers-hard-max-num",
      ),
    };
    requireCondition(
      JSON.stringify(observed.workers_path_values) === JSON.stringify([workerFixture.root]) &&
        observed.database_values.length === 0 &&
        JSON.stringify(observed.execute_workers_max_num_values) === JSON.stringify(["1"]) &&
        JSON.stringify(observed.prepare_workers_soft_max_num_values) === JSON.stringify(["1"]) &&
        JSON.stringify(observed.prepare_workers_hard_max_num_values) === JSON.stringify(["1"]),
      `PVF argv contract drifted for ${contract}`,
    );
    return observed;
  });
  return {
    root,
    root_identity: rootIdentity,
    root_owner_uid: safeNumber(rootStat.uid, "PVF worker root owner"),
    filesystem_magic: filesystemMagic,
    mount,
    write_probe_errno: writeProbeErrno,
    assets,
    execution_sides: executionSides,
  };
}

async function runLocal(localBinary, database, endpoint, request, signer, repoRoot) {
  const args = ["--database", database, "--rpc", endpoint];
  if (signer) args.push("--dev-signer", signer);
  const environment = {
    HOME: "/home/charles",
    LANG: "C",
    LC_ALL: "C",
    PATH: "/usr/bin:/bin",
    PWD: repoRoot,
    SHLVL: "0",
    TZ: "UTC",
  };
  const result = await runProcess(localBinary, args, {
    cwd: repoRoot,
    input: canonicalBytes(request),
    label: `cubikan-local ${request.operation.type}`,
    timeout: PHASE_TIMEOUT_MS,
    env: environment,
  });
  let failureResponseClass = null;
  try {
    if (
      result.stdout.length > 2 &&
      result.stdout[0] === 0x7b &&
      result.stdout.at(-2) === 0x7d &&
      result.stdout.at(-1) === 0x0a
    ) {
      const failureText = new TextDecoder("utf-8", { fatal: true }).decode(
        result.stdout.subarray(0, -1),
      );
      rejectDuplicateJsonKeys(failureText, `cubikan-local ${request.operation.type} failure`);
      const failureResponse = JSON.parse(failureText);
      failureResponseClass = {
        outcome: typeof failureResponse?.outcome === "string" ? failureResponse.outcome : null,
        error_code:
          typeof failureResponse?.error?.code === "string" ? failureResponse.error.code : null,
        error_field:
          typeof failureResponse?.error?.field === "string" ? failureResponse.error.field : null,
      };
    }
  } catch {
    failureResponseClass = null;
  }
  requireCondition(
    result.code === 0 && result.signal === null && result.stderr.length === 0,
    `cubikan-local failed: code=${result.code} signal=${result.signal} stdout_size=${result.stdout.length} stdout_sha256=${sha256(result.stdout)} stderr_size=${result.stderr.length} stderr_sha256=${sha256(result.stderr)} response_class=${JSON.stringify(failureResponseClass)}`,
  );
  requireCondition(
    result.stdout.length > 2 &&
      result.stdout[0] === 0x7b &&
      result.stdout.at(-2) === 0x7d &&
      result.stdout.at(-1) === 0x0a,
    "cubikan-local response is not one unframed JSON object plus one LF",
  );
  const bytes = result.stdout.subarray(0, -1);
  const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  rejectDuplicateJsonKeys(text, `cubikan-local ${request.operation.type} response`);
  const response = JSON.parse(text);
  return {
    response,
    bytes,
    stdout_bytes: result.stdout,
    executable: localBinary,
    argv: [localBinary, ...args],
    environment,
    environment_sha256: canonicalSha256(environment),
  };
}

async function waitEndpointsEqual(apiA, apiB, minimumNumber, label) {
  let result;
  await waitFor(
    async () => {
      const [a, b] = await Promise.all([rpcFinalized(apiA), rpcFinalized(apiB)]);
      if (a.number >= minimumNumber && a.number === b.number && a.hash === b.hash) {
        result = a;
        return true;
      }
      return false;
    },
    label,
    PHASE_TIMEOUT_MS,
    250,
  );
  return result;
}

async function connectPolkadotApi(ApiPromise, WsProvider, endpoint, label) {
  const provider = new WsProvider(endpoint);
  let api = null;
  try {
    api = await withTimeout(label, PHASE_TIMEOUT_MS, async () => {
      const connected = await ApiPromise.create({ provider });
      await connected.isReady;
      requireCondition(
        provider.endpoint === endpoint && provider.isConnected && connected.isConnected,
        `${label} did not establish the exact requested WebSocket transport`,
      );
      return connected;
    });
    return api;
  } catch (error) {
    if (api) await api.disconnect().catch(() => {});
    else await provider.disconnect().catch(() => {});
    throw error;
  }
}

async function connectRestartPolkadotApi(
  ApiPromise,
  WsProvider,
  restart,
  endpoint,
  label,
) {
  const provider = new WsProvider(endpoint);
  let api = null;
  try {
    api = await waitForRestartStep(restart, label, async () => {
      const connected = await ApiPromise.create({ provider });
      await connected.isReady;
      return connected;
    });
    return api;
  } catch (error) {
    if (api) await api.disconnect().catch(() => {});
    else await provider.disconnect().catch(() => {});
    throw error;
  }
}

async function observeStableFinalityInterval(api, expected, index, durationMilliseconds) {
  const started = Date.now();
  const deadline = started + durationMilliseconds;
  let samples = 0;
  while (Date.now() < deadline) {
    const observed = await withTimeout(
      `finality stability interval ${index} sample`,
      10_000,
      () => rpcFinalized(api),
    );
    requireCondition(
      observed.number === expected.number && observed.hash === expected.hash,
      `finalized head changed during stability interval ${index}`,
    );
    samples += 1;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const ended = await withTimeout(`finality stability interval ${index} end`, 10_000, () =>
    rpcFinalized(api),
  );
  requireCondition(
    ended.number === expected.number && ended.hash === expected.hash,
    `finalized head changed at stability interval ${index} boundary`,
  );
  return {
    index,
    start_number: expected.number,
    start_hash: expected.hash,
    end_number: ended.number,
    end_hash: ended.hash,
    elapsed_milliseconds: Date.now() - started,
    sample_count: samples + 1,
    unchanged: true,
  };
}

async function listenerRecordsForNode(roleFixture, pid) {
  const result = await runProcess("/usr/bin/ss", ["-H", "-lntup"], {
    label: `${roleFixture.role} listener inventory`,
    timeout: 10_000,
    env: { PATH: "/usr/bin:/bin", LANG: "C", LC_ALL: "C" },
  });
  requireCondition(result.code === 0, `${roleFixture.role} listener inventory failed`);
  const parsed = parseSsInventory(result.stdout);
  return roleFixture.listeners.map((address) => {
    const rows = parsed.filter((row) => row.local_address === address);
    requireCondition(
      rows.length === 1 && rows[0].pids.length === 1 && rows[0].pids[0] === pid,
      `listener ${address} is not uniquely owned by ${roleFixture.role}/${pid}`,
    );
    return { protocol: rows[0].protocol, address, pid, raw_line: rows[0].raw_line };
  });
}

function parseSsInventory(rawBytes) {
  const text = rawBytes.toString("utf8");
  return text
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      const columns = line.trim().split(/\s+/);
      const hasNetworkColumn = columns[0] === "tcp" || columns[0] === "udp";
      const protocol = hasNetworkColumn ? columns[0] : "tcp";
      const state = columns[hasNetworkColumn ? 1 : 0];
      const localAddress = columns[hasNetworkColumn ? 4 : 3];
      const processColumns = columns.slice(hasNetworkColumn ? 6 : 5);
      requireCondition(
        columns.length >= (hasNetworkColumn ? 7 : 6) &&
          ((protocol === "tcp" && state === "LISTEN") ||
            (protocol === "udp" && state === "UNCONN")),
        `malformed ss row: ${line}`,
      );
      requireCondition(
        /^(?:127\.0\.0\.1|\[::1\]):[1-9][0-9]{0,4}$/.test(localAddress),
        `ss exposed a non-loopback or malformed listener: ${localAddress}`,
      );
      const pidMatches = [...processColumns.join(" ").matchAll(/(?:^|,)pid=([1-9][0-9]*)(?:,|\))/g)];
      const pids = [...new Set(pidMatches.map((match) => Number(match[1])))].sort((a, b) => a - b);
      requireCondition(pids.length > 0, `ss listener lacks a process owner: ${line}`);
      return { protocol, state, local_address: localAddress, pids, raw_line: line };
    });
}

function spawnFrozenNativeNode(client, processEntry, frozenCommand) {
  const log = fs.createWriteStream(processEntry.logs, { flags: "a", mode: 0o600 });
  const spawnArgs = ["-c", ...frozenCommand];
  const child = spawn(client.command, spawnArgs, {
    env: { ...process.env },
    stdio: ["ignore", "pipe", "pipe"],
  });
  requireCondition(Number.isSafeInteger(child.pid) && child.pid > 1, "manual collator spawn failed");
  child.stdout.pipe(log, { end: false });
  child.stderr.pipe(log, { end: false });
  const closed = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) => {
      log.end(() => resolve({ code, signal }));
    });
  });
  processEntry.pid = child.pid;
  return { child, closed, spawn_executable: client.command, spawn_args: spawnArgs };
}

async function waitForRestartStep(restart, label, operation) {
  const outcome = await withTimeout(label, PHASE_TIMEOUT_MS, () =>
    Promise.race([
      Promise.resolve()
        .then(operation)
        .then((value) => ({ kind: "ready", value })),
      restart.closed.then((exit) => ({ kind: "exit", exit })),
    ]),
  );
  requireCondition(
    outcome.kind === "ready",
    `manual collator B exited before ${label}: code=${outcome.exit?.code ?? "null"} signal=${outcome.exit?.signal ?? "null"}`,
  );
  return outcome.value;
}

async function finalizedObservation(
  api,
  endpoint,
  submission,
  finalCheckpoint,
  signerAccounts,
  expectedPayload,
) {
  const blockHash = (await api.rpc.chain.getBlockHash(submission.block_number)).toHex();
  const [header, signedBlock, events] = await Promise.all([
    api.rpc.chain.getHeader(blockHash),
    api.rpc.chain.getBlock(blockHash),
    api.query.system.events.at(blockHash),
  ]);
  const extrinsics = signedBlock.block.extrinsics;
  const matchingExtrinsics = extrinsics.filter(
    (extrinsic) => extrinsic.hash.toHex() === submission.extrinsic_hash,
  );
  requireCondition(
    blockHash === submission.block_hash &&
      header.number.toNumber() === submission.block_number &&
      matchingExtrinsics.length === 1 &&
      submission.extrinsic_index < extrinsics.length &&
      extrinsics[submission.extrinsic_index].hash.toHex() === submission.extrinsic_hash,
    `${submission.id} finalized block/extrinsic join differs from its accepted coordinate`,
  );
  const acceptedInExtrinsic = [];
  for (let index = 0; index < events.length; index += 1) {
    const record = events[index];
    if (
      !record.phase.isApplyExtrinsic ||
      record.phase.asApplyExtrinsic.toNumber() !== submission.extrinsic_index ||
      record.event.section.toLowerCase() !== "cubikan" ||
      record.event.method !== "Accepted"
    ) continue;
    acceptedInExtrinsic.push({ index, record });
  }
  requireCondition(
    acceptedInExtrinsic.length === 1,
    `${submission.id} extrinsic did not emit exactly one Cubikan.Accepted event`,
  );
  const { index: acceptedIndex, record } = acceptedInExtrinsic[0];
  const data = record.event.data;
  requireCondition(data.length === 5, "Cubikan.Accepted event field count drifted");
  const deploymentId = data[0].toHex();
  const eventSchemaVersion = Number(data[1].toString());
  const globalSequence = data[2].toString();
  const signer = data[3].toHex();
  const payloadBytes = Buffer.from(data[4].toU8a());
  const decoded = parseAcceptedPayload(payloadBytes, submission.operation);
  const expectedSigner = signerAccounts[submission.signer];
  const mismatches = [
    ["event_index", acceptedIndex !== submission.event_index],
    ["topics", record.topics.length !== 0],
    ["deployment_id", deploymentId !== submission.response.coordinate.deployment_id],
    ["event_schema_version", eventSchemaVersion !== 1],
    ["global_sequence", globalSequence !== submission.response.coordinate.global_sequence],
    ["signer", signer !== expectedSigner],
    ["payload", canonicalSha256(decoded.payload) !== canonicalSha256(expectedPayload)],
    ["effect", canonicalSha256(decoded.effect) !== canonicalSha256(submission.response.effect)],
  ]
    .filter(([, mismatched]) => mismatched)
    .map(([field]) => field);
  requireCondition(
    mismatches.length === 0,
    `${submission.id} sole Accepted event mismatched ${mismatches.join(",")}`,
  );
  const acceptedEvent = {
    pallet: "Cubikan",
    event: "Accepted",
    deployment_id: deploymentId,
    event_schema_version: eventSchemaVersion,
    global_sequence: globalSequence,
    signer,
    payload_variant: decoded.variant,
    payload_scale_hex: `0x${payloadBytes.toString("hex")}`,
    payload_sha256: sha256(payloadBytes),
    payload: decoded.payload,
    effect: decoded.effect,
  };
  const effect = acceptedEvent.effect;
  return {
    endpoint,
    finalized_head_number: finalCheckpoint.number,
    finalized_head_hash: finalCheckpoint.hash,
    block_number: submission.block_number,
    block_hash: blockHash,
    extrinsic_hash: submission.extrinsic_hash,
    extrinsic_index: submission.extrinsic_index,
    event_index: submission.event_index,
    matching_extrinsic_count: matchingExtrinsics.length,
    matching_event_count: acceptedInExtrinsic.length,
    accepted_pallet: acceptedEvent.pallet,
    accepted_event: acceptedEvent.event,
    accepted_deployment_id: acceptedEvent.deployment_id,
    accepted_event_schema_version: acceptedEvent.event_schema_version,
    accepted_global_sequence: acceptedEvent.global_sequence,
    accepted_signer: acceptedEvent.signer,
    accepted_payload_variant: acceptedEvent.payload_variant,
    accepted_payload_scale_hex: acceptedEvent.payload_scale_hex,
    accepted_payload_sha256: acceptedEvent.payload_sha256,
    accepted_payload: acceptedEvent.payload,
    accepted_effect: effect,
    accepted_effect_sha256: canonicalSha256(effect),
    finalized_header_sha256: canonicalSha256(codecJson(header)),
    block_extrinsics_sha256: canonicalSha256(extrinsics.map(codecJson)),
    system_events_sha256: canonicalSha256(codecJson(events)),
  };
}

const DISPATCH_METHOD_BY_OPERATION = Object.freeze({
  create_intent_unit: "createUnit",
  transition_intent_unit: "transitionUnit",
  complete_intent_unit: "completeUnit",
  create_relationship_definition: "createRelationshipDefinition",
  create_relationship: "createRelationship",
  delete_relationship: "deleteRelationship",
  record_association: "recordAssociation",
  revoke_association: "revokeAssociation",
});

function exactDispatchMethod(operation) {
  const method = DISPATCH_METHOD_BY_OPERATION[operation];
  requireCondition(typeof method === "string", `unknown protocol operation ${operation}`);
  return method;
}

function eventPhaseEvidence(phase) {
  if (phase.isApplyExtrinsic) {
    return {
      kind: "apply_extrinsic",
      extrinsic_index: phase.asApplyExtrinsic.toNumber(),
    };
  }
  if (phase.isInitialization) return { kind: "initialization", extrinsic_index: null };
  if (phase.isFinalization) return { kind: "finalization", extrinsic_index: null };
  fail("finalized event has an unknown phase");
}

function exactCallIndexHex(callIndex) {
  requireCondition(callIndex instanceof Uint8Array, "finalized call index is not a byte vector");
  const bytes = Buffer.from(callIndex);
  requireCondition(bytes.length === 2, "finalized call index is not exactly two bytes");
  return `0x${bytes.toString("hex")}`;
}

function exactSignerAccountHex(signer) {
  requireCondition(
    signer?.type === "Id" && typeof signer.value?.toHex === "function",
    "finalized signed extrinsic does not use the exact MultiAddress Id variant",
  );
  const account = signer.value.toHex();
  requireCondition(
    /^0x[0-9a-f]{64}$/.test(account),
    "finalized signed extrinsic signer is not one canonical 32-byte account",
  );
  return account;
}

function eventDataEvidence(event) {
  const data = codecJson(event.data);
  const key = `${event.section}.${event.method}`;
  if (key === "balances.Withdraw" || key === "transactionPayment.TransactionFeePaid") {
    requireCondition(
      Array.isArray(data) && data.length > 0 && typeof event.data[0]?.toHex === "function",
      `${key} payer is not a typed account identifier`,
    );
    const payer = event.data[0].toHex();
    requireCondition(
      /^0x[0-9a-f]{64}$/.test(payer),
      `${key} payer is not one canonical 32-byte account`,
    );
    data[0] = payer;
  }
  if (key === "onDemandAssignmentProvider.SpotPriceSet") {
    requireCondition(
      Array.isArray(data) &&
        data.length === 1 &&
        event.data.length === 1 &&
        typeof event.data[0]?.toString === "function",
      "onDemandAssignmentProvider.SpotPriceSet data is not one typed price",
    );
    data[0] = nonnegativeInteger(
      event.data[0].toString(),
      "onDemandAssignmentProvider.SpotPriceSet price",
    );
  }
  if (key === "grandpa.NewAuthorities") {
    requireCondition(
      Array.isArray(data) &&
        data.length === 1 &&
        event.data.length === 1 &&
        typeof event.data[0]?.[Symbol.iterator] === "function",
      "grandpa.NewAuthorities data is not one typed authority set",
    );
    const authorities = [...event.data[0]];
    requireCondition(authorities.length === 2, "grandpa.NewAuthorities is not exact two-validator data");
    data[0] = authorities.map((authority, index) => {
      requireCondition(
        authority.length === 2 &&
          typeof authority[0]?.toHex === "function" &&
          typeof authority[1]?.toString === "function",
        `grandpa.NewAuthorities authority ${index} is not one typed ID/weight pair`,
      );
      const authorityId = authority[0].toHex();
      const authorityWeight = nonnegativeInteger(
        authority[1].toString(),
        `grandpa.NewAuthorities authority ${index} weight`,
      );
      requireCondition(
        /^0x[0-9a-f]{64}$/.test(authorityId) && authorityWeight === 1,
        `grandpa.NewAuthorities authority ${index} ID/weight drifted`,
      );
      return [authorityId, authorityWeight];
    });
  }
  return data;
}

async function endpointChainCensusBlock(api, endpoint, blockNumber) {
  const blockHash = await withTimeout(`census ${endpoint} block ${blockNumber} hash`, 30_000, () =>
    api.rpc.chain.getBlockHash(blockNumber),
  ).then((value) => value.toHex());
  const [header, signedBlock, events] = await withTimeout(
    `census ${endpoint} block ${blockNumber}`,
    30_000,
    () =>
      Promise.all([
        api.rpc.chain.getHeader(blockHash),
        api.rpc.chain.getBlock(blockHash),
        api.query.system.events.at(blockHash),
      ]),
  );
  requireCondition(
    header.number.toNumber() === blockNumber &&
      signedBlock.block.header.number.toNumber() === blockNumber &&
      signedBlock.block.header.hash.toHex() === blockHash,
    `census ${endpoint} block/header identity drifted at ${blockNumber}`,
  );
  const extrinsics = signedBlock.block.extrinsics.map((extrinsic, index) => {
    const scaleBytes = Buffer.from(extrinsic.toU8a());
    const args = extrinsic.method.args.map(codecJson);
    return {
      index,
      hash: extrinsic.hash.toHex(),
      signed: extrinsic.isSigned,
      signer: extrinsic.isSigned ? exactSignerAccountHex(extrinsic.signer) : null,
      section: extrinsic.method.section,
      method: extrinsic.method.method,
      call_index: exactCallIndexHex(extrinsic.method.callIndex),
      args,
      args_sha256: canonicalSha256(args),
      scale_hex: `0x${scaleBytes.toString("hex")}`,
      scale_sha256: sha256(scaleBytes),
    };
  });
  const eventRows = [...events].map((record, index) => {
    const scaleBytes = Buffer.from(record.toU8a());
    const data = eventDataEvidence(record.event);
    return {
      index,
      phase: eventPhaseEvidence(record.phase),
      section: record.event.section,
      method: record.event.method,
      data,
      data_sha256: canonicalSha256(data),
      topics: record.topics.map((topic) => topic.toHex()),
      scale_hex: `0x${scaleBytes.toString("hex")}`,
      scale_sha256: sha256(scaleBytes),
    };
  });
  const headerBytes = Buffer.from(header.toU8a());
  const systemEventsBytes = Buffer.from(events.toU8a());
  return {
    number: blockNumber,
    hash: blockHash,
    parent_hash: header.parentHash.toHex(),
    state_root: header.stateRoot.toHex(),
    extrinsics_root: header.extrinsicsRoot.toHex(),
    header_scale_hex: `0x${headerBytes.toString("hex")}`,
    header_sha256: sha256(headerBytes),
    extrinsics,
    system_events_scale_hex: `0x${systemEventsBytes.toString("hex")}`,
    system_events_sha256: sha256(systemEventsBytes),
    events: eventRows,
  };
}

async function endpointChainCensus(api, endpoint, finalCheckpoint, maximumRetainedBytes) {
  const blocks = [];
  let retainedBytes = 0;
  for (
    let batchStart = 0;
    batchStart <= finalCheckpoint.number;
    batchStart += CENSUS_BLOCK_CONCURRENCY
  ) {
    const batchEnd = Math.min(
      finalCheckpoint.number + 1,
      batchStart + CENSUS_BLOCK_CONCURRENCY,
    );
    const rows = await orderedParallelOperations(
      Array.from({ length: batchEnd - batchStart }, (_, offset) => () =>
        endpointChainCensusBlock(api, endpoint, batchStart + offset),
      ),
    );
    for (const row of rows) {
      retainedBytes += canonicalBytes(row).length;
      requireCondition(
        retainedBytes <= maximumRetainedBytes,
        `census ${endpoint} exceeded its incremental retained byte bound`,
      );
      blocks.push(row);
    }
  }
  requireCondition(
    blocks.at(-1)?.hash === finalCheckpoint.hash,
    `census ${endpoint} did not terminate at exact F`,
  );
  return {
    endpoint,
    range_start: 0,
    range_end: finalCheckpoint.number,
    retained_bytes: retainedBytes,
    transcript_sha256: canonicalSha256(blocks),
    blocks,
  };
}

function attestEqualEndpointCensus(endpointA, endpointB, finalCheckpoint) {
  const transcriptA = canonicalBytes(endpointA.blocks);
  const transcriptB = canonicalBytes(endpointB.blocks);
  requireCondition(
    endpointA.endpoint !== endpointB.endpoint &&
      endpointA.range_start === 0 &&
      endpointB.range_start === 0 &&
      endpointA.range_end === finalCheckpoint.number &&
      endpointB.range_end === finalCheckpoint.number &&
      endpointA.blocks.length === finalCheckpoint.number + 1 &&
      endpointB.blocks.length === endpointA.blocks.length &&
      endpointA.transcript_sha256 === sha256(transcriptA) &&
      endpointB.transcript_sha256 === sha256(transcriptB) &&
      transcriptA.equals(transcriptB),
    "dual endpoint finalized chain census diverged",
  );
  return {
    endpoint: endpointB.endpoint,
    range_start: endpointB.range_start,
    range_end: endpointB.range_end,
    block_count: endpointB.blocks.length,
    transcript_sha256: endpointB.transcript_sha256,
  };
}

function censusCategoryCounts(blocks, zeroCountKeys) {
  const counts = Object.fromEntries(
    zeroCountKeys
      .filter((key) => !["public_rpc", "secret"].includes(key))
      .map((key) => [key, 0]),
  );
  const bump = (key) => {
    requireCondition(Object.hasOwn(counts, key), `unknown census category ${key}`);
    counts[key] += 1;
  };
  const classify = (section, method, kind) => {
    const key = `${section}.${method}`.toLowerCase();
    if (key === "cubikan.replaceauthorizedsubmitters" || key === "cubikan.authorizedsubmittersreplaced")
      bump("allowlist_mutation");
    if (key === "system.newaccount" || key === "system.killedaccount") bump("account_creation");
    if (key === "session.setkeys" || key === "session.purgekeys") bump("key_import");
    if (key.includes("faucet") || key.includes("drip")) bump("faucet");
    if (
      (section.toLowerCase() === "balances" && method.toLowerCase().includes("transfer")) ||
      key === "balances.transfer"
    ) bump("transfer");
    if (
      /(?:registrar|paras|parachaininfo)/i.test(section) &&
      /(?:register|deregister|para.?id)/i.test(method)
    ) bump("para_id_registration");
    if (/coretime/i.test(section) || /coretime/i.test(method)) bump("coretime");
    if (
      (section.toLowerCase() === "system" &&
        /^(?:setcode|setcodewithoutchecks|authorizeupgrade|authorizeupgradewithoutchecks|applyauthorizedupgrade|codeupdated)$/i.test(method)) ||
      (section.toLowerCase() === "parachainsystem" &&
        /(?:upgrade|validationfunction)/i.test(method))
    ) bump("runtime_upload");
    if (/(?:deployment|deploy)/i.test(key) && section.toLowerCase() !== "cubikan") bump("deployment");
    if (/(?:release|upgradeauthorized|validationfunction)/i.test(key)) bump("release");
    if (
      /^(?:sudo|democracy|referenda|convictionvoting|council|technicalcommittee|whitelist|treasury|scheduler|preimage|proxy|multisig|utility)$/i.test(section)
    ) bump("governance");
    requireCondition(kind === "call" || kind === "event", "invalid census classifier kind");
  };
  for (const block of blocks) {
    for (const extrinsic of block.extrinsics)
      classify(extrinsic.section, extrinsic.method, "call");
    for (const event of block.events) classify(event.section, event.method, "event");
  }
  return counts;
}

async function finalizedChainCensus(apiA, apiB, fixture, submissions, finalCheckpoint) {
  const endpointRetainedLimit = fixture.audit.chain_census.maximum_retained_bytes;
  const [endpointA, endpointB] = await withTimeout(
    "dual-endpoint finalized chain census",
    300_000,
    () =>
      Promise.all([
        endpointChainCensus(apiA, "collator-a", finalCheckpoint, endpointRetainedLimit),
        endpointChainCensus(apiB, "collator-b", finalCheckpoint, endpointRetainedLimit),
      ]),
  );
  const endpointBAttestation = attestEqualEndpointCensus(
    endpointA,
    endpointB,
    finalCheckpoint,
  );
  const allowedUnsignedCalls = new Set(fixture.audit.chain_census.allowed_unsigned_calls);
  const allowedEvents = new Set(fixture.audit.chain_census.allowed_events);
  const feeEventKeys = [
    "balances.Withdraw",
    "balances.BurnedDebt",
    "transactionPayment.TransactionFeePaid",
  ];
  const eventCounts = Object.fromEntries(
    fixture.audit.chain_census.allowed_events.map((key) => [key, 0]),
  );
  const expectedByHash = new Map(submissions.map((submission) => [submission.extrinsic_hash, submission]));
  requireCondition(expectedByHash.size === submissions.length, "expected submission hash is duplicated");
  const seenExpected = new Set();
  const externalActions = [];
  const forbiddenEvents = [];
  let signedExtrinsicCount = 0;
  let unsignedInherentCount = 0;
  let acceptedEventCount = 0;
  let extrinsicSuccessCount = 0;
  let extrinsicFailedCount = 0;
  for (const block of endpointA.blocks) {
    const successByExtrinsic = Array(block.extrinsics.length).fill(0);
    const feeCountsByExtrinsic = Array.from({ length: block.extrinsics.length }, () =>
      Array(feeEventKeys.length).fill(0),
    );
    const feeAmountsByExtrinsic = Array.from({ length: block.extrinsics.length }, () =>
      Array(feeEventKeys.length).fill(null),
    );
    for (const extrinsic of block.extrinsics) {
      const call = `${extrinsic.section}.${extrinsic.method}`;
      if (extrinsic.signed) {
        signedExtrinsicCount += 1;
        const expected = expectedByHash.get(extrinsic.hash);
        if (!expected) {
          externalActions.push(`signed:${block.number}:${extrinsic.index}:${call}:${extrinsic.hash}`);
          continue;
        }
        const expectedAccount = fixture.audit.dev_signer_accounts[expected.signer];
        const mismatches = [
          ["duplicate", seenExpected.has(expected.id)],
          ["block_number", block.number !== expected.block_number],
          ["extrinsic_index", extrinsic.index !== expected.extrinsic_index],
          ["signer", extrinsic.signer !== expectedAccount],
          ["section", extrinsic.section !== "cubikan"],
          ["method", extrinsic.method !== exactDispatchMethod(expected.operation)],
        ]
          .filter(([, mismatched]) => mismatched)
          .map(([field]) => field);
        requireCondition(
          mismatches.length === 0,
          `${expected.id} census call/coordinate/signer mismatched ${mismatches.join(",")}`,
        );
        seenExpected.add(expected.id);
      } else if (allowedUnsignedCalls.has(call)) {
        unsignedInherentCount += 1;
      } else {
        externalActions.push(`unsigned:${block.number}:${extrinsic.index}:${call}:${extrinsic.hash}`);
      }
    }
    for (const event of block.events) {
      const key = `${event.section}.${event.method}`;
      if (!allowedEvents.has(key)) {
        forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
      } else {
        eventCounts[key] += 1;
      }
      if (key === "system.ExtrinsicSuccess") {
        extrinsicSuccessCount += 1;
        if (
          event.phase.kind !== "apply_extrinsic" ||
          event.phase.extrinsic_index === null ||
          event.phase.extrinsic_index >= block.extrinsics.length
        ) {
          forbiddenEvents.push(`phase:${block.number}:${event.index}:${key}`);
        } else {
          successByExtrinsic[event.phase.extrinsic_index] += 1;
        }
      }
      if (key === "system.ExtrinsicFailed") {
        extrinsicFailedCount += 1;
        forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
      }
      if (key === "cubikan.Accepted") {
        acceptedEventCount += 1;
        const expected = submissions.find(
          (submission) =>
            submission.block_number === block.number &&
            submission.event_index === event.index &&
            event.phase.kind === "apply_extrinsic" &&
            event.phase.extrinsic_index === submission.extrinsic_index,
        );
        if (!expected) forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
      }
      const feeEventIndex = feeEventKeys.indexOf(key);
      if (feeEventIndex !== -1) {
        const extrinsicIndex = event.phase.extrinsic_index;
        if (
          event.phase.kind !== "apply_extrinsic" ||
          extrinsicIndex === null ||
          extrinsicIndex >= block.extrinsics.length
        ) {
          forbiddenEvents.push(`phase:${block.number}:${event.index}:${key}`);
        } else {
          const extrinsic = block.extrinsics[extrinsicIndex];
          const expected = expectedByHash.get(extrinsic.hash);
          if (!extrinsic.signed || !expected) {
            forbiddenEvents.push(`fee_origin:${block.number}:${event.index}:${key}`);
          } else {
            feeCountsByExtrinsic[extrinsicIndex][feeEventIndex] += 1;
            const expectedPayer = fixture.audit.dev_signer_accounts[expected.signer];
            if (key === "balances.Withdraw") {
              requireCondition(
                Array.isArray(event.data) && event.data.length === 2,
                `balances.Withdraw data shape drifted at ${block.number}:${event.index}`,
              );
              const [payer, amount] = event.data;
              requireCondition(
                payer === expectedPayer,
                `balances.Withdraw payer drifted at ${block.number}:${event.index}`,
              );
              feeAmountsByExtrinsic[extrinsicIndex][feeEventIndex] = exactUnsignedBigInt(
                amount,
                `balances.Withdraw amount at ${block.number}:${event.index}`,
              );
            } else if (key === "balances.BurnedDebt") {
              requireCondition(
                Array.isArray(event.data) && event.data.length === 1,
                `balances.BurnedDebt data shape drifted at ${block.number}:${event.index}`,
              );
              feeAmountsByExtrinsic[extrinsicIndex][feeEventIndex] = exactUnsignedBigInt(
                event.data[0],
                `balances.BurnedDebt amount at ${block.number}:${event.index}`,
              );
            } else {
              requireCondition(
                Array.isArray(event.data) && event.data.length === 3,
                `transactionPayment.TransactionFeePaid data shape drifted at ${block.number}:${event.index}`,
              );
              const [payer, actualFee, tip] = event.data;
              requireCondition(
                payer === expectedPayer &&
                  exactUnsignedBigInt(
                    tip,
                    `transactionPayment.TransactionFeePaid tip at ${block.number}:${event.index}`,
                  ) === 0n,
                `transactionPayment.TransactionFeePaid payer or zero tip drifted at ${block.number}:${event.index}`,
              );
              feeAmountsByExtrinsic[extrinsicIndex][feeEventIndex] = exactUnsignedBigInt(
                actualFee,
                `transactionPayment.TransactionFeePaid actual fee at ${block.number}:${event.index}`,
              );
            }
          }
        }
      }
      if (
        key.toLowerCase() === "cubikan.authorizedsubmittersreplaced" ||
        key.toLowerCase() === "system.codeupdated" ||
        key.toLowerCase() === "system.newaccount" ||
        key.toLowerCase() === "system.killedaccount" ||
        key.toLowerCase() === "balances.transfer" ||
        (/^(?:sudo|democracy|referenda|convictionvoting|council|technicalcommittee|whitelist|treasury|scheduler|preimage|proxy|multisig|utility)$/i.test(event.section)) ||
        (event.section.toLowerCase() === "parachainsystem" &&
          /(?:upgrade|validationfunction|upwardmessagesent)/i.test(event.method))
      ) forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
    }
    for (const [extrinsicIndex, extrinsic] of block.extrinsics.entries()) {
      if (successByExtrinsic[extrinsicIndex] !== 1)
        forbiddenEvents.push(
          `success_count:${block.number}:${extrinsicIndex}:${successByExtrinsic[extrinsicIndex]}:system.ExtrinsicSuccess`,
        );
      const expectedFeeCount = extrinsic.signed && expectedByHash.has(extrinsic.hash) ? 1 : 0;
      for (const [feeEventIndex, key] of feeEventKeys.entries()) {
        if (feeCountsByExtrinsic[extrinsicIndex][feeEventIndex] !== expectedFeeCount)
          forbiddenEvents.push(
            `fee_count:${block.number}:${extrinsicIndex}:${feeCountsByExtrinsic[extrinsicIndex][feeEventIndex]}:${key}`,
          );
      }
      if (expectedFeeCount === 1) {
        const [withdrawAmount, burnedDebtAmount, actualFee] =
          feeAmountsByExtrinsic[extrinsicIndex];
        requireCondition(
          withdrawAmount !== null &&
            withdrawAmount > 0n &&
            withdrawAmount === burnedDebtAmount &&
            withdrawAmount === actualFee,
          `fee withdrawal/burn/payment identity drifted at ${block.number}:${extrinsicIndex}`,
        );
      }
    }
  }
  const expectedExtrinsicSuccessCount = endpointA.blocks.reduce(
    (count, block) => count + block.extrinsics.length,
    0,
  );
  const expectedEventCounts = {
    "balances.BurnedDebt": submissions.length,
    "balances.Withdraw": submissions.length,
    "cubikan.Accepted": submissions.length,
    "system.ExtrinsicSuccess": expectedExtrinsicSuccessCount,
    "transactionPayment.TransactionFeePaid": submissions.length,
  };
  const aggregateChecks = [
    ["seen_expected", seenExpected.size, submissions.length],
    ["signed_extrinsics", signedExtrinsicCount, submissions.length],
    ["accepted_events", acceptedEventCount, submissions.length],
    ["extrinsic_success_events", extrinsicSuccessCount, expectedExtrinsicSuccessCount],
    ["extrinsic_failed_events", extrinsicFailedCount, 0],
    ["external_actions", externalActions.length, 0],
    ["forbidden_events", forbiddenEvents.length, 0],
    ...Object.entries(expectedEventCounts).map(([key, expected]) => [
      `event_count:${key}`,
      eventCounts[key],
      expected,
    ]),
  ];
  const aggregateMismatches = aggregateChecks
    .filter(([, actual, expected]) => actual !== expected)
    .map(([field, actual, expected]) => ({ field, actual, expected }));
  const forbiddenEventHistogram = new Map();
  for (const row of forbiddenEvents) {
    const key = row.split(":").at(-1);
    forbiddenEventHistogram.set(key, (forbiddenEventHistogram.get(key) ?? 0) + 1);
  }
  const aggregateDiagnostic = {
    mismatches: aggregateMismatches,
    forbidden_event_histogram: Object.fromEntries(
      [...forbiddenEventHistogram.entries()]
        .sort(([left], [right]) => left.localeCompare(right))
        .slice(0, 16),
    ),
    external_action_examples: externalActions.slice(0, 8),
    forbidden_event_examples: forbiddenEvents.slice(0, 8),
  };
  requireCondition(
    aggregateMismatches.length === 0,
    `finalized chain census aggregate mismatched ${canonicalBytes(aggregateDiagnostic)
      .toString("utf8")
      .slice(0, 4096)}`,
  );
  const census = {
    format: "cubikan-finalized-chain-census-v1",
    range_start: fixture.audit.chain_census.range_start,
    range_end: finalCheckpoint.number,
    maximum_retained_bytes: fixture.audit.chain_census.maximum_retained_bytes,
    allowed_unsigned_calls: fixture.audit.chain_census.allowed_unsigned_calls,
    allowed_events: fixture.audit.chain_census.allowed_events,
    endpoint_a: endpointA,
    endpoint_b: endpointBAttestation,
    endpoints_equal: true,
    expected_submission_hashes: submissions.map((submission) => submission.extrinsic_hash),
    expected_signed_extrinsic_count: submissions.length,
    signed_extrinsic_count: signedExtrinsicCount,
    unsigned_inherent_count: unsignedInherentCount,
    accepted_event_count: acceptedEventCount,
    extrinsic_success_count: extrinsicSuccessCount,
    extrinsic_failed_count: extrinsicFailedCount,
    event_counts: eventCounts,
    forbidden_counts: censusCategoryCounts(endpointA.blocks, fixture.audit.zero_count_keys),
    external_actions: externalActions,
    forbidden_events: forbiddenEvents,
  };
  requireCondition(
    canonicalBytes(census).length <= fixture.audit.chain_census.maximum_retained_bytes,
    "finalized chain census exceeds its retained evidence bound",
  );
  return census;
}

function nonnegativeInteger(value, label) {
  const parsed =
    typeof value === "number" && Number.isSafeInteger(value)
      ? value
      : typeof value === "string" && /^(?:0|[1-9][0-9]*)$/.test(value)
        ? Number(value)
        : Number.NaN;
  requireCondition(Number.isSafeInteger(parsed) && parsed >= 0, `${label} is not a nonnegative integer`);
  return parsed;
}

function normalizedCodecKey(value) {
  return value.replaceAll("_", "").toLowerCase();
}

function exactLowerHexByteLength(value, label) {
  requireCondition(
    typeof value === "string" && /^0x(?:[0-9a-f]{2})*$/.test(value),
    `${label} is not canonical lowercase even-length hex`,
  );
  return (value.length - 2) / 2;
}

function inspectRelayInherentValue(value, observation, path = "$args") {
  if (Array.isArray(value)) {
    value.forEach((entry, index) =>
      inspectRelayInherentValue(entry, observation, `${path}/${index}`),
    );
    return;
  }
  if (value === null || typeof value !== "object") return;
  for (const [key, child] of Object.entries(value)) {
    const childPath = `${path}/${key}`;
    switch (normalizedCodecKey(key)) {
      case "backedcandidates":
        requireCondition(Array.isArray(child), `${childPath} is not an array`);
        observation.backed_candidate_count += child.length;
        break;
      case "paraid":
        observation.candidate_para_ids.push(nonnegativeInteger(child, childPath));
        break;
      case "newvalidationcode":
        observation.new_validation_code_field_count += 1;
        if (child !== null) observation.new_validation_code_count += 1;
        break;
      case "upwardmessages":
        requireCondition(Array.isArray(child), `${childPath} is not an array`);
        observation.upward_message_field_count += 1;
        const byteLengths = child.map((message, index) =>
          exactLowerHexByteLength(message, `${childPath}/${index}`),
        );
        const separatorIndexes = byteLengths
          .map((length, index) => (length === 0 ? index : -1))
          .filter((index) => index !== -1);
        requireCondition(
          separatorIndexes.length <= 1,
          `${childPath} contains duplicate UMP signal separators`,
        );
        const separatorIndex = separatorIndexes[0] ?? child.length;
        const signalCount = separatorIndexes.length === 0 ? 0 : child.length - separatorIndex - 1;
        requireCondition(
          separatorIndexes.length === 0 ||
            (signalCount >= 1 &&
              signalCount <= 2 &&
              byteLengths.slice(separatorIndex + 1).every((length) => length > 0)),
          `${childPath} contains a malformed UMP protocol-signal suffix`,
        );
        observation.upward_message_count += separatorIndex;
        observation.upward_signal_separator_count += separatorIndexes.length;
        observation.upward_signal_count += signalCount;
        break;
      case "horizontalmessages":
        requireCondition(Array.isArray(child), `${childPath} is not an array`);
        observation.horizontal_message_field_count += 1;
        observation.horizontal_message_count += child.length;
        break;
      case "processeddownwardmessages":
        observation.processed_downward_message_field_count += 1;
        observation.processed_downward_message_count += nonnegativeInteger(child, childPath);
        break;
      case "disputes":
        requireCondition(Array.isArray(child), `${childPath} is not an array`);
        observation.dispute_field_count += 1;
        observation.dispute_statement_count += child.length;
        break;
      default:
        break;
    }
    inspectRelayInherentValue(child, observation, childPath);
  }
}

function collectNamedIntegers(value, normalizedName, result, path = "$event") {
  if (Array.isArray(value)) {
    value.forEach((entry, index) =>
      collectNamedIntegers(entry, normalizedName, result, `${path}/${index}`),
    );
    return;
  }
  if (value === null || typeof value !== "object") return;
  for (const [key, child] of Object.entries(value)) {
    const childPath = `${path}/${key}`;
    if (normalizedCodecKey(key) === normalizedName)
      result.push(nonnegativeInteger(child, childPath));
    collectNamedIntegers(child, normalizedName, result, childPath);
  }
}

async function relayRuntimePointIdentity(api, blockHash) {
  const [runtimeHex, metadata, runtimeVersion] = await Promise.all([
    storageHex(api, RUNTIME_CODE_KEY, blockHash),
    api.rpc.state.getMetadata(blockHash),
    api.rpc.state.getRuntimeVersion(blockHash),
  ]);
  requireCondition(
    typeof runtimeHex === "string" && /^0x[0-9a-f]+$/.test(runtimeHex),
    `relay runtime :code is missing at ${blockHash}`,
  );
  const runtimeBytes = Buffer.from(runtimeHex.slice(2), "hex");
  const metadataBytes = Buffer.from(metadata.toHex().slice(2), "hex");
  return {
    block_hash: blockHash,
    runtime_spec_version: runtimeVersion.specVersion.toNumber(),
    runtime_code_sha256: sha256(runtimeBytes),
    metadata_sha256: sha256(metadataBytes),
  };
}

async function relayRuntimeEndpointIdentity(api, endpoint, rangeEndHash) {
  const genesisHash = await rpcGenesisHash(api);
  const [genesis, rangeEnd] = await Promise.all([
    relayRuntimePointIdentity(api, genesisHash),
    relayRuntimePointIdentity(api, rangeEndHash),
  ]);
  return { endpoint, genesis, range_end: rangeEnd };
}

async function finalizedRelayChainCensus(apiA, apiB, fixture, finalCheckpoint) {
  const censusFixture = fixture.audit.relay_chain_census;
  const endpointRetainedLimit = censusFixture.maximum_retained_bytes;
  const [endpointA, endpointB, runtimeA, runtimeB] = await withTimeout(
    "dual-validator finalized relay chain census",
    300_000,
    () =>
      Promise.all([
        endpointChainCensus(apiA, "relay-a", finalCheckpoint, endpointRetainedLimit),
        endpointChainCensus(apiB, "relay-b", finalCheckpoint, endpointRetainedLimit),
        relayRuntimeEndpointIdentity(apiA, "relay-a", finalCheckpoint.hash),
        relayRuntimeEndpointIdentity(apiB, "relay-b", finalCheckpoint.hash),
      ]),
  );
  const endpointBAttestation = attestEqualEndpointCensus(
    endpointA,
    endpointB,
    finalCheckpoint,
  );
  const runtimePointsEqual =
    canonicalSha256(runtimeA.genesis) === canonicalSha256(runtimeB.genesis) &&
    canonicalSha256(runtimeA.range_end) === canonicalSha256(runtimeB.range_end);
  const runtimeUnchanged =
    runtimeA.genesis.runtime_spec_version === runtimeA.range_end.runtime_spec_version &&
    runtimeA.genesis.runtime_code_sha256 === runtimeA.range_end.runtime_code_sha256 &&
    runtimeA.genesis.metadata_sha256 === runtimeA.range_end.metadata_sha256 &&
    runtimeB.genesis.runtime_spec_version === runtimeB.range_end.runtime_spec_version &&
    runtimeB.genesis.runtime_code_sha256 === runtimeB.range_end.runtime_code_sha256 &&
    runtimeB.genesis.metadata_sha256 === runtimeB.range_end.metadata_sha256;
  requireCondition(runtimePointsEqual && runtimeUnchanged, "relay runtime identity changed or diverged");

  const allowedUnsignedCalls = new Set(censusFixture.allowed_unsigned_calls);
  const allowedEvents = new Set(censusFixture.allowed_events);
  const expectedInitialSpotPrice = censusFixture.expected_initial_spot_price;
  const expectedGrandpaAuthorities = censusFixture.expected_grandpa_authorities;
  requireCondition(
    censusFixture.minimum_session_rotation_count === 2 &&
      expectedInitialSpotPrice === 10_000_000 &&
      Array.isArray(expectedGrandpaAuthorities) &&
      expectedGrandpaAuthorities.length === 2 &&
      expectedGrandpaAuthorities.every(
        (authority) =>
          Array.isArray(authority) &&
          authority.length === 2 &&
          /^0x[0-9a-f]{64}$/.test(authority[0]) &&
          authority[1] === 1,
      ) &&
      new Set(expectedGrandpaAuthorities.map(([authorityId]) => authorityId)).size === 2,
    "relay session-rotation fixture is not the exact two-validator contract",
  );
  const eventCounts = Object.fromEntries(censusFixture.allowed_events.map((key) => [key, 0]));
  const observation = {
    backed_candidate_count: 0,
    candidate_para_ids: [],
    new_validation_code_field_count: 0,
    new_validation_code_count: 0,
    upward_message_field_count: 0,
    upward_message_count: 0,
    upward_signal_separator_count: 0,
    upward_signal_count: 0,
    horizontal_message_field_count: 0,
    horizontal_message_count: 0,
    processed_downward_message_field_count: 0,
    processed_downward_message_count: 0,
    dispute_field_count: 0,
    dispute_statement_count: 0,
  };
  const eventCandidateParaIds = [];
  const historicalRootSessionIndices = [];
  const newSessionIndices = [];
  let newQueuedCount = 0;
  let grandpaNewAuthoritiesCount = 0;
  let initialSpotPrice = null;
  const externalActions = [];
  const forbiddenEvents = [];
  let signedExtrinsicCount = 0;
  let unsignedInherentCount = 0;
  let timestampInherentCount = 0;
  let paraInherentCount = 0;
  let extrinsicSuccessCount = 0;
  let extrinsicFailedCount = 0;

  for (const block of endpointA.blocks) {
    const calls = block.extrinsics.map((extrinsic) => `${extrinsic.section}.${extrinsic.method}`);
    if (block.number === 0) {
      requireCondition(
        block.extrinsics.length === 0 && block.events.length === 0,
        "relay genesis block retained an extrinsic or event",
      );
    } else {
      requireCondition(
        JSON.stringify(calls) === JSON.stringify(censusFixture.allowed_unsigned_calls),
        `relay block ${block.number} did not contain the exact ordered inherent set`,
      );
    }
    const successByExtrinsic = Array(block.extrinsics.length).fill(0);
    for (const extrinsic of block.extrinsics) {
      const call = `${extrinsic.section}.${extrinsic.method}`;
      if (extrinsic.signed) {
        signedExtrinsicCount += 1;
        externalActions.push(`signed:${block.number}:${extrinsic.index}:${call}:${extrinsic.hash}`);
      } else if (!allowedUnsignedCalls.has(call)) {
        externalActions.push(`unsigned:${block.number}:${extrinsic.index}:${call}:${extrinsic.hash}`);
      } else {
        unsignedInherentCount += 1;
      }
      requireCondition(extrinsic.signer === null, `relay extrinsic ${block.number}:${extrinsic.index} retained a signer`);
      if (call === "timestamp.set") timestampInherentCount += 1;
      if (call === "paraInherent.enter") {
        paraInherentCount += 1;
        inspectRelayInherentValue(extrinsic.args, observation);
      }
    }
    for (const event of block.events) {
      const key = `${event.section}.${event.method}`;
      if (!allowedEvents.has(key)) {
        forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
        continue;
      }
      eventCounts[key] += 1;
      if (key === "onDemandAssignmentProvider.SpotPriceSet") {
        const spotPrice =
          Array.isArray(event.data) && event.data.length === 1
            ? nonnegativeInteger(
                event.data[0],
                `onDemandAssignmentProvider.SpotPriceSet at ${block.number}:${event.index}`,
              )
            : null;
        if (
          block.number !== 1 ||
          event.index !== 0 ||
          event.phase.kind !== "initialization" ||
          event.phase.extrinsic_index !== null ||
          event.topics.length !== 0 ||
          spotPrice !== expectedInitialSpotPrice ||
          initialSpotPrice !== null
        ) {
          forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        } else {
          initialSpotPrice = spotPrice;
        }
      } else if (key === "system.ExtrinsicSuccess") {
        extrinsicSuccessCount += 1;
        if (
          event.phase.kind !== "apply_extrinsic" ||
          event.phase.extrinsic_index === null ||
          event.phase.extrinsic_index >= block.extrinsics.length
        ) {
          forbiddenEvents.push(`phase:${block.number}:${event.index}:${key}`);
        } else {
          successByExtrinsic[event.phase.extrinsic_index] += 1;
        }
      } else if (key === "system.ExtrinsicFailed") {
        extrinsicFailedCount += 1;
        forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
      } else if (key.startsWith("paraInclusion.Candidate")) {
        const paraIds = [];
        collectNamedIntegers(event.data, "paraid", paraIds);
        if (
          event.phase.kind !== "apply_extrinsic" ||
          event.phase.extrinsic_index !== 1 ||
          paraIds.length === 0 ||
          paraIds.some(
            (paraId) => paraId !== fixture.checkpoints.pre_mutation_readiness.para_id,
          )
        ) forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        eventCandidateParaIds.push(...paraIds);
      } else if (key === "historical.RootStored") {
        if (
          event.phase.kind !== "initialization" ||
          event.phase.extrinsic_index !== null ||
          !Array.isArray(event.data) ||
          event.data.length !== 1
        ) {
          forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        } else {
          historicalRootSessionIndices.push(
            nonnegativeInteger(event.data[0], `historical.RootStored at ${block.number}:${event.index}`),
          );
        }
      } else if (key === "session.NewSession") {
        if (
          event.phase.kind !== "initialization" ||
          event.phase.extrinsic_index !== null ||
          !Array.isArray(event.data) ||
          event.data.length !== 1
        ) {
          forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        } else {
          newSessionIndices.push(
            nonnegativeInteger(event.data[0], `session.NewSession at ${block.number}:${event.index}`),
          );
        }
      } else if (key === "session.NewQueued") {
        if (
          event.phase.kind !== "initialization" ||
          event.phase.extrinsic_index !== null ||
          !Array.isArray(event.data) ||
          event.data.length !== 0
        ) {
          forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        } else {
          newQueuedCount += 1;
        }
      } else if (key === "grandpa.NewAuthorities") {
        if (
          event.phase.kind !== "finalization" ||
          event.phase.extrinsic_index !== null ||
          !Array.isArray(event.data) ||
          event.data.length !== 1 ||
          canonicalSha256(event.data[0]) !== canonicalSha256(expectedGrandpaAuthorities)
        ) {
          forbiddenEvents.push(`payload:${block.number}:${event.index}:${key}`);
        } else {
          grandpaNewAuthoritiesCount += 1;
        }
      } else {
        forbiddenEvents.push(`event:${block.number}:${event.index}:${key}`);
      }
    }
    requireCondition(
      successByExtrinsic.every((count) => count === 1),
      `relay block ${block.number} lacks exactly one success event per inherent`,
    );
  }

  const candidateParaIds = [...new Set(observation.candidate_para_ids)].sort((a, b) => a - b);
  const eventParaIds = [...new Set(eventCandidateParaIds)].sort((a, b) => a - b);
  const nonGenesisBlockCount = finalCheckpoint.number;
  const expectedParaId = fixture.checkpoints.pre_mutation_readiness.para_id;
  const expectedUnsignedInherentCount =
    nonGenesisBlockCount * censusFixture.allowed_unsigned_calls.length;
  const historicalRootIndicesMatch =
    historicalRootSessionIndices.length === newSessionIndices.length &&
    historicalRootSessionIndices.every(
      (sessionIndex, index) => sessionIndex === newSessionIndices[index] + 1,
    );
  const expectedGrandpaNewAuthoritiesCount = Math.max(newSessionIndices.length - 1, 0);
  const relayAggregateChecks = [
    ["signed_extrinsics", signedExtrinsicCount, 0],
    ["unsigned_inherents", unsignedInherentCount, expectedUnsignedInherentCount],
    ["timestamp_inherents", timestampInherentCount, nonGenesisBlockCount],
    ["para_inherents", paraInherentCount, nonGenesisBlockCount],
    ["extrinsic_success_events", extrinsicSuccessCount, unsignedInherentCount],
    ["extrinsic_failed_events", extrinsicFailedCount, 0],
    [
      "initial_spot_price_set_events",
      eventCounts["onDemandAssignmentProvider.SpotPriceSet"],
      1,
    ],
    ["initial_spot_price", initialSpotPrice, expectedInitialSpotPrice],
    [
      "session_rotation_minimum",
      newSessionIndices.length >= censusFixture.minimum_session_rotation_count,
      true,
    ],
    ["historical_root_events", historicalRootSessionIndices.length, newSessionIndices.length],
    ["new_queued_events", newQueuedCount, newSessionIndices.length],
    ["historical_root_session_indices", historicalRootIndicesMatch, true],
    [
      "grandpa_new_authorities_events",
      grandpaNewAuthoritiesCount,
      expectedGrandpaNewAuthoritiesCount,
    ],
    ["backed_candidates_nonzero", observation.backed_candidate_count > 0, true],
    [
      "candidate_para_id_observations",
      observation.candidate_para_ids.length,
      observation.backed_candidate_count,
    ],
    ["candidate_para_id_cardinality", candidateParaIds.length, 1],
    [
      "candidate_para_id_exact",
      candidateParaIds.length === 1 ? candidateParaIds[0] : null,
      expectedParaId,
    ],
    ["event_para_id_cardinality", eventParaIds.length, 1],
    [
      "event_para_id_exact",
      eventParaIds.length === 1 ? eventParaIds[0] : null,
      expectedParaId,
    ],
    [
      "new_validation_code_fields",
      observation.new_validation_code_field_count,
      observation.backed_candidate_count,
    ],
    [
      "upward_message_fields",
      observation.upward_message_field_count,
      observation.backed_candidate_count,
    ],
    [
      "horizontal_message_fields",
      observation.horizontal_message_field_count,
      observation.backed_candidate_count,
    ],
    [
      "processed_downward_message_fields",
      observation.processed_downward_message_field_count,
      observation.backed_candidate_count,
    ],
    ["dispute_fields", observation.dispute_field_count, paraInherentCount],
    ["new_validation_codes", observation.new_validation_code_count, 0],
    ["upward_messages", observation.upward_message_count, 0],
    [
      "upward_signal_separators",
      observation.upward_signal_separator_count,
      observation.backed_candidate_count,
    ],
    [
      "upward_signal_minimum",
      observation.upward_signal_count >= observation.backed_candidate_count,
      true,
    ],
    [
      "upward_signal_maximum",
      observation.upward_signal_count <= observation.backed_candidate_count * 2,
      true,
    ],
    ["horizontal_messages", observation.horizontal_message_count, 0],
    ["processed_downward_messages", observation.processed_downward_message_count, 0],
    ["dispute_statements", observation.dispute_statement_count, 0],
    ["external_actions", externalActions.length, 0],
    ["forbidden_events", forbiddenEvents.length, 0],
  ];
  const relayAggregateMismatches = relayAggregateChecks
    .filter(([, actual, expected]) => actual !== expected)
    .map(([field, actual, expected]) => ({ field, actual, expected }));
  const forbiddenRelayEventHistogram = new Map();
  for (const row of forbiddenEvents) {
    const key = row.split(":").at(-1);
    forbiddenRelayEventHistogram.set(key, (forbiddenRelayEventHistogram.get(key) ?? 0) + 1);
  }
  const relayAggregateDiagnostic = {
    mismatches: relayAggregateMismatches,
    candidate_para_id_examples: candidateParaIds.slice(0, 8),
    event_para_id_examples: eventParaIds.slice(0, 8),
    allowed_event_counts: eventCounts,
    backed_candidate_count: observation.backed_candidate_count,
    upward_signal_count: observation.upward_signal_count,
    historical_root_session_index_examples: historicalRootSessionIndices.slice(0, 8),
    new_session_index_examples: newSessionIndices.slice(0, 8),
    new_queued_count: newQueuedCount,
    grandpa_new_authorities_count: grandpaNewAuthoritiesCount,
    initial_spot_price: initialSpotPrice,
    forbidden_event_histogram: Object.fromEntries(
      [...forbiddenRelayEventHistogram.entries()]
        .sort(([left], [right]) => left.localeCompare(right))
        .slice(0, 16),
    ),
    external_action_examples: externalActions.slice(0, 8),
    forbidden_event_examples: forbiddenEvents.slice(0, 8),
  };
  requireCondition(
    relayAggregateMismatches.length === 0,
    `finalized relay chain census aggregate mismatched ${canonicalBytes(relayAggregateDiagnostic)
      .toString("utf8")
      .slice(0, 4096)}`,
  );

  const census = {
    format: "cubikan-finalized-relay-chain-census-v1",
    range_start: censusFixture.range_start,
    range_end: finalCheckpoint.number,
    checkpoint_hash: finalCheckpoint.hash,
    maximum_retained_bytes: censusFixture.maximum_retained_bytes,
    minimum_session_rotation_count: censusFixture.minimum_session_rotation_count,
    expected_initial_spot_price: expectedInitialSpotPrice,
    expected_grandpa_authorities: expectedGrandpaAuthorities,
    allowed_unsigned_calls: censusFixture.allowed_unsigned_calls,
    allowed_events: censusFixture.allowed_events,
    endpoint_a: endpointA,
    endpoint_b: endpointBAttestation,
    endpoints_equal: true,
    runtime_identity: {
      endpoint_a: runtimeA,
      endpoint_b: runtimeB,
      endpoints_equal: runtimePointsEqual,
      unchanged: runtimeUnchanged,
    },
    signed_extrinsic_count: signedExtrinsicCount,
    unsigned_inherent_count: unsignedInherentCount,
    timestamp_inherent_count: timestampInherentCount,
    para_inherent_count: paraInherentCount,
    extrinsic_success_count: extrinsicSuccessCount,
    extrinsic_failed_count: extrinsicFailedCount,
    session_rotation_count: newSessionIndices.length,
    historical_root_session_indices: historicalRootSessionIndices,
    new_session_indices: newSessionIndices,
    new_queued_count: newQueuedCount,
    grandpa_new_authorities_count: grandpaNewAuthoritiesCount,
    initial_spot_price: initialSpotPrice,
    backed_candidate_count: observation.backed_candidate_count,
    candidate_para_ids: candidateParaIds,
    event_candidate_para_ids: eventParaIds,
    new_validation_code_field_count: observation.new_validation_code_field_count,
    new_validation_code_count: observation.new_validation_code_count,
    upward_message_field_count: observation.upward_message_field_count,
    upward_message_count: observation.upward_message_count,
    upward_signal_separator_count: observation.upward_signal_separator_count,
    upward_signal_count: observation.upward_signal_count,
    horizontal_message_field_count: observation.horizontal_message_field_count,
    horizontal_message_count: observation.horizontal_message_count,
    processed_downward_message_field_count: observation.processed_downward_message_field_count,
    processed_downward_message_count: observation.processed_downward_message_count,
    dispute_field_count: observation.dispute_field_count,
    dispute_statement_count: observation.dispute_statement_count,
    event_counts: eventCounts,
    forbidden_counts: censusCategoryCounts(endpointA.blocks, fixture.audit.zero_count_keys),
    external_actions: externalActions,
    forbidden_events: forbiddenEvents,
  };
  requireCondition(
    canonicalBytes(census).length <= censusFixture.maximum_retained_bytes,
    "finalized relay chain census exceeds its retained evidence bound",
  );
  return census;
}

async function runSnapshotPage({
  contract,
  localBinary,
  database,
  endpoint,
  repoRoot,
  read,
}) {
  const output = await runLocal(localBinary, database, endpoint, read.request, null, repoRoot);
  requireCondition(
    output.response.outcome === "success" &&
      output.response.result &&
      typeof output.response.result === "object",
    `${contract.label} ${read.id} did not return one successful DB-backed result`,
  );
  requireCondition(output.executable === localBinary, `${contract.label} read executable drifted`);
  return output;
}

async function orderedParallelOperations(operations) {
  const settlements = await Promise.allSettled(operations.map((operation) => operation()));
  for (const settlement of settlements) {
    if (settlement.status === "rejected") throw settlement.reason;
  }
  return settlements.map((settlement) => settlement.value);
}

async function readSnapshot({
  contract,
  fixture,
  localBinary,
  database,
  endpoint,
  repoRoot,
  replacement,
  sourceGate,
}) {
  const readRpcUrl = fixture.endpoints[contract.read_endpoint];
  requireCondition(
    typeof readRpcUrl === "string" && endpoint === readRpcUrl,
    `${contract.label} actual read endpoint does not match its pinned role`,
  );
  requireCondition(sourceGate?.authorize, `${contract.label} lacks a source gate`);
  const sourceAuthorization = sourceGate.authorize(
    contract.label,
    contract.source_endpoint,
  );
  let creationProbeErrno = null;
  try {
    await fsp.lstat(database);
  } catch (error) {
    if (error?.code === "ENOENT") creationProbeErrno = "ENOENT";
    else throw error;
  }
  const responses = [];
  const hashes = {};
  const responseBodies = {};
  const responseBodySizes = {};
  const stdoutHashes = {};
  const stdoutSizes = {};
  const readArgvs = {};
  const readEnvironments = {};
  const readEnvironmentHashes = {};
  requireCondition(
    fixture.reads.length === 12 && fixture.reads[0]?.id === "get-primary",
    `${contract.label} lacks the exact 12-page snapshot contract and serial primer`,
  );
  const outputs = new Array(fixture.reads.length);
  outputs[0] = await runSnapshotPage({
    contract,
    localBinary,
    database,
    endpoint,
    repoRoot,
    read: fixture.reads[0],
  });
  for (let start = 1; start < fixture.reads.length; start += SNAPSHOT_READ_CONCURRENCY) {
    const reads = fixture.reads.slice(start, start + SNAPSHOT_READ_CONCURRENCY);
    const settlements = await Promise.allSettled(
      reads.map((read) =>
        runSnapshotPage({ contract, localBinary, database, endpoint, repoRoot, read }),
      ),
    );
    for (let offset = 0; offset < settlements.length; offset += 1) {
      const settlement = settlements[offset];
      if (settlement.status === "rejected") throw settlement.reason;
      outputs[start + offset] = settlement.value;
    }
  }
  for (let index = 0; index < fixture.reads.length; index += 1) {
    const read = fixture.reads[index];
    const output = outputs[index];
    responses.push(output.response);
    responseBodies[read.id] = `0x${output.bytes.toString("hex")}`;
    responseBodySizes[read.id] = output.bytes.length;
    hashes[read.id] = sha256(output.bytes);
    stdoutHashes[read.id] = sha256(output.stdout_bytes);
    stdoutSizes[read.id] = output.stdout_bytes.length;
    readArgvs[read.id] = output.argv;
    readEnvironments[read.id] = output.environment;
    readEnvironmentHashes[read.id] = output.environment_sha256;
  }
  const projectionRead = await openedFileEvidence(database);
  const sidecarAbsence = await sqliteSidecarAbsence(database);
  const stat = projectionRead.evidence.path_before;
  const results = responses.map((response) => response.result);
  const checkpoints = results.map((result) => result.checkpoint);
  requireCondition(
    checkpoints.every(
      (checkpoint) => canonicalSha256(checkpoint) === canonicalSha256(checkpoints[0]),
    ),
    `${contract.label} DB-backed read checkpoints diverged`,
  );
  const responseCheckpoint = checkpoints[0];
  requireCondition(
    responseCheckpoint &&
      Number.isSafeInteger(Number(responseCheckpoint.block_number)) &&
      typeof responseCheckpoint.block_hash === "string",
    `${contract.label} lacks a valid DB-backed checkpoint`,
  );

  const coordinateBySequence = new Map();
  const collectCoordinates = (value) => {
    if (Array.isArray(value)) {
      for (const child of value) collectCoordinates(child);
    } else if (value && typeof value === "object") {
      for (const [key, child] of Object.entries(value)) {
        if (/^(?:last|created|deleted|revoked)_coordinate$/.test(key)) {
          const sequence = String(child?.global_sequence ?? "");
          requireCondition(/^[1-9][0-9]*$/.test(sequence), `${contract.label} has malformed coordinate`);
          const prior = coordinateBySequence.get(sequence);
          requireCondition(
            prior === undefined || canonicalSha256(prior) === canonicalSha256(child),
            `${contract.label} has conflicting coordinate for global sequence ${sequence}`,
          );
          coordinateBySequence.set(sequence, child);
        }
        collectCoordinates(child);
      }
    }
  };
  results.forEach(collectCoordinates);
  const coordinates = [...coordinateBySequence.values()].sort(
    (left, right) => Number(left.global_sequence) - Number(right.global_sequence),
  );
  const units = [results[0].intent_unit, results[1].intent_unit];
  requireCondition(units.every((unit) => unit && typeof unit === "object"), "unit reads were absent");
  const history = units.flatMap((unit) => unit.history);
  const origins = units.map((unit) => unit.origin);
  const definitions = [results[4].definition];
  const edges = results[5].items;
  const projections = [...results[6].items, ...results[7].items];
  const provenance = [...results[8].items, ...results[9].items];
  const sections = {
    checkpoint: [responseCheckpoint],
    coordinates,
    definitions,
    edges,
    history,
    origins,
    pages: results,
    projections,
    provenance,
    units,
  };
  for (const [name, expected] of Object.entries(fixture.semantic_projection.section_counts)) {
    requireCondition(sections[name]?.length === expected, `${contract.label} ${name} count drifted`);
  }
  const associationKey = provenance
    .map((item) => item.key)
    .find(
      (key) =>
        key?.unit_id === "01890f47-0ed1-7c68-a7c5-02f69b5a5fe3" &&
        key?.subject?.type === "revision" &&
        key?.subject?.revision === "7" &&
        key?.reference?.namespace === "git.commit.sha1" &&
        key?.reference?.scope === "workspace/cubikan" &&
        key?.reference?.value === "15f37f16e70096c2624ca92fed9d55750640ec88",
    );
  requireCondition(associationKey, `${contract.label} DB reads lack the T-1114 association`);
  const exactAssociationCoreJson = JSON.stringify({
    unit_id: associationKey.unit_id,
    subject: { kind: "revision", revision: Number(associationKey.subject.revision) },
    reference: associationKey.reference,
  });
  const genesisHashes = new Set(coordinates.map((coordinate) => coordinate.parachain_genesis_hash));
  const deploymentIds = new Set(coordinates.map((coordinate) => coordinate.deployment_id));
  requireCondition(
    genesisHashes.size === 1 && deploymentIds.size === 1,
    `${contract.label} DB-backed coordinate identity diverged`,
  );
  const attestationIdentity = {
    parachain_genesis_hash: [...genesisHashes][0],
    deployment_id: [...deploymentIds][0],
    checkpoint: responseCheckpoint,
  };
  return {
    label: contract.label,
    source_endpoint: contract.source_endpoint,
    read_endpoint: contract.read_endpoint,
    read_rpc_url: readRpcUrl,
    read_executable: localBinary,
    read_argvs: readArgvs,
    read_environments: readEnvironments,
    read_environment_sha256s: readEnvironmentHashes,
    projection_path: database,
    projection_file_mode: stat.mode,
    projection_file_device: stat.device,
    projection_file_inode: stat.inode,
    projection_file_size: stat.size,
    projection_file_sha256: projectionRead.evidence.sha256,
    sqlite_sidecar_absence: sidecarAbsence,
    fresh_database: contract.fresh_database,
    database_deleted_before_build: contract.database_deleted_before_build,
    creation_probe_errno: contract.fresh_database ? creationProbeErrno : null,
    replacement,
    full_stream_attested: true,
    source_eligible_at_open: sourceAuthorization.gate_open,
    source_gate_sequence: sourceAuthorization.sequence,
    checkpoint_number: Number(responseCheckpoint.block_number),
    checkpoint_hash: responseCheckpoint.block_hash,
    read_ids: fixture.reads.map((read) => read.id),
    read_responses: Object.fromEntries(
      fixture.reads.map((read, index) => [read.id, responses[index]]),
    ),
    read_response_body_hex: responseBodies,
    read_response_body_sizes: responseBodySizes,
    read_response_sha256s: hashes,
    read_stdout_sizes: stdoutSizes,
    read_stdout_sha256s: stdoutHashes,
    sections,
    semantic_sha256: canonicalSha256(sections),
    attestation_identity_sha256: canonicalSha256(attestationIdentity),
    exact_association_core_json: exactAssociationCoreJson,
  };
}

async function primeSnapshotDatabase({
  contract,
  fixture,
  localBinary,
  database,
  endpoint,
  repoRoot,
  sourceGate,
}) {
  const readRpcUrl = fixture.endpoints[contract.read_endpoint];
  requireCondition(
    typeof readRpcUrl === "string" && endpoint === readRpcUrl,
    `${contract.label} actual read endpoint does not match its pinned role`,
  );
  requireCondition(sourceGate?.authorize, `${contract.label} lacks a source gate`);
  sourceGate.authorize(contract.label, contract.source_endpoint);
  let creationProbeErrno = null;
  try {
    await fsp.lstat(database);
  } catch (error) {
    if (error?.code === "ENOENT") creationProbeErrno = "ENOENT";
    else throw error;
  }
  requireCondition(
    creationProbeErrno === "ENOENT" &&
      fixture.reads.length === 12 &&
      fixture.reads[0]?.id === "get-primary",
    `${contract.label} did not begin with an absent database and exact get-primary primer`,
  );
  const output = await runSnapshotPage({
    contract,
    localBinary,
    database,
    endpoint,
    repoRoot,
    read: fixture.reads[0],
  });
  const checkpoint = output.response.result.checkpoint;
  requireCondition(
    checkpoint &&
      Number.isSafeInteger(Number(checkpoint.block_number)) &&
      typeof checkpoint.block_hash === "string",
    `${contract.label} primer lacks a valid DB-backed checkpoint`,
  );
  return checkpoint;
}

async function replacementSnapshot(options) {
  const database = options.database;
  requireCondition(!fs.existsSync(database), `${options.contract.label} database unexpectedly exists`);
  const primingContract = {
    ...options.contract,
    label: `${options.contract.label}-predelete`,
    database_deleted_before_build: false,
  };
  const primedCheckpoint = await primeSnapshotDatabase({ ...options, contract: primingContract });
  const pathBefore = identityFromStat(await fsp.lstat(database, { bigint: true }));
  const retained = await fsp.open(database, "r");
  try {
    const descriptorBefore = identityFromStat(await retained.stat({ bigint: true }));
    const retainedBytes = await readExactAt(retained, descriptorBefore.size, `${options.contract.label} pre-delete database`);
    requireCondition(
      canonicalSha256(pathBefore) === canonicalSha256(descriptorBefore) &&
        retainedBytes.length === descriptorBefore.size,
      `${options.contract.label} pre-delete descriptor did not match its path`,
    );
    await fsp.unlink(database);
    let deletionProbeErrno = null;
    try {
      await fsp.lstat(database);
    } catch (error) {
      if (error?.code === "ENOENT") deletionProbeErrno = "ENOENT";
      else throw error;
    }
    const rebuilt = await readSnapshot({ ...options, replacement: null });
    requireCondition(
      Number(primedCheckpoint.block_number) === rebuilt.checkpoint_number &&
        primedCheckpoint.block_hash === rebuilt.checkpoint_hash,
      `${options.contract.label} rebuilt another checkpoint after its one-page primer`,
    );
    const descriptorAfter = identityFromStat(await retained.stat({ bigint: true }));
    const retainedAfterBytes = await readExactAt(
      retained,
      descriptorAfter.size,
      `${options.contract.label} retained deleted database`,
    );
    requireCondition(
      descriptorAfter.device === descriptorBefore.device &&
        descriptorAfter.inode === descriptorBefore.inode &&
        descriptorAfter.size === descriptorBefore.size &&
        descriptorBefore.link_count === 1 &&
        descriptorAfter.link_count === 0 &&
        sha256(retainedAfterBytes) === sha256(retainedBytes),
      `${options.contract.label} retained deleted database descriptor changed during rebuild`,
    );
    rebuilt.replacement = {
      predelete_device: pathBefore.device,
      predelete_inode: pathBefore.inode,
      predelete_size: pathBefore.size,
      predelete_sha256: sha256(retainedBytes),
      retained_descriptor_device_before: descriptorBefore.device,
      retained_descriptor_inode_before: descriptorBefore.inode,
      retained_descriptor_size_before: descriptorBefore.size,
      retained_descriptor_link_count_before: descriptorBefore.link_count,
      retained_descriptor_device_after: descriptorAfter.device,
      retained_descriptor_inode_after: descriptorAfter.inode,
      retained_descriptor_size_after: descriptorAfter.size,
      retained_descriptor_link_count_after: descriptorAfter.link_count,
      retained_descriptor_sha256_after: sha256(retainedAfterBytes),
      deletion_probe_errno: deletionProbeErrno,
      postcreate_device: rebuilt.projection_file_device,
      postcreate_inode: rebuilt.projection_file_inode,
      postcreate_size: rebuilt.projection_file_size,
      postcreate_sha256: rebuilt.projection_file_sha256,
    };
    return rebuilt;
  } finally {
    await retained.close();
  }
}

async function probeArchiveBlock(apiA, apiB, fixture, blockNumber) {
  const [hashA, hashB] = await orderedParallelOperations([
    async () => (await apiA.rpc.chain.getBlockHash(blockNumber)).toHex(),
    async () => (await apiB.rpc.chain.getBlockHash(blockNumber)).toHex(),
  ]);
  requireCondition(
    /^0x[0-9a-f]{64}$/.test(hashA) && hashA === hashB,
    `archive block hash diverged at ${blockNumber}`,
  );
  const rows = [];
  for (const method of fixture.archive_probe.methods_per_block) {
    const invoke = async (api, blockHash) => {
      if (method === "chain_getBlockHash") return { value: blockHash, height: blockNumber };
      if (method === "chain_getHeader") {
        const header = await api.rpc.chain.getHeader(blockHash);
        return { value: codecJson(header), height: header.number.toNumber() };
      }
      if (method === "chain_getBlock") {
        const signed = await api.rpc.chain.getBlock(blockHash);
        return { value: codecJson(signed), height: signed.block.header.number.toNumber() };
      }
      if (method === "state_getStorage:System.Events") {
        return { value: await storageHex(api, SYSTEM_EVENTS_KEY, blockHash), height: blockNumber };
      }
      if (method === "state_getStorage::code") {
        return { value: await storageHex(api, RUNTIME_CODE_KEY, blockHash), height: blockNumber };
      }
      fail(`unknown archive method ${method}`);
    };
    const [probeA, probeB] = await orderedParallelOperations([
      () => invoke(apiA, hashA),
      () => invoke(apiB, hashB),
    ]);
    const rawPresent = probeA.value !== undefined && probeA.value !== null &&
      probeB.value !== undefined && probeB.value !== null;
    const canonicalGenesisDefault =
      method === "state_getStorage:System.Events" &&
      blockNumber === 0 &&
      probeA.value === null &&
      probeB.value === null;
    const present = rawPresent || canonicalGenesisDefault;
    const canonicalA = canonicalGenesisDefault ? "0x00" : probeA.value;
    const canonicalB = canonicalGenesisDefault ? "0x00" : probeB.value;
    const isGenesisEvents = method === "state_getStorage:System.Events" && blockNumber === 0;
    requireCondition(
      present &&
        (isGenesisEvents ? canonicalGenesisDefault && !rawPresent : rawPresent) &&
        probeA.height === blockNumber &&
        probeB.height === blockNumber,
      `archive ${method} was absent or height-mismatched at ${blockNumber}`,
    );
    rows.push({
      block_number: blockNumber,
      block_hash: hashA,
      method,
      endpoint_a_observed_height: probeA.height,
      endpoint_b_observed_height: probeB.height,
      endpoint_a_block_hash: hashA,
      endpoint_b_block_hash: hashB,
      endpoint_a_sha256: canonicalSha256(canonicalA),
      endpoint_b_sha256: canonicalSha256(canonicalB),
      raw_present: rawPresent,
      canonical_genesis_default: canonicalGenesisDefault,
      present,
      equal:
        JSON.stringify(sortedValue(canonicalA)) ===
        JSON.stringify(sortedValue(canonicalB)),
    });
  }
  return rows;
}

async function probeArchive(apiA, apiB, fixture, finalCheckpoint) {
  const transcript = [];
  for (
    let batchStart = fixture.archive_probe.range_start;
    batchStart <= finalCheckpoint.number;
    batchStart += ARCHIVE_PROBE_BLOCK_CONCURRENCY
  ) {
    const batchEnd = Math.min(
      finalCheckpoint.number + 1,
      batchStart + ARCHIVE_PROBE_BLOCK_CONCURRENCY,
    );
    const rowsByBlock = await orderedParallelOperations(
      Array.from({ length: batchEnd - batchStart }, (_, offset) => () =>
        probeArchiveBlock(apiA, apiB, fixture, batchStart + offset),
      ),
    );
    for (const rows of rowsByBlock) transcript.push(...rows);
  }
  const count = (finalCheckpoint.number + 1) * fixture.archive_probe.methods_per_block.length;
  return {
    range_start: fixture.archive_probe.range_start,
    range_end: finalCheckpoint.number,
    methods_per_block: fixture.archive_probe.methods_per_block,
    system_events_storage_key: fixture.archive_probe.system_events_storage_key,
    runtime_code_storage_key: fixture.archive_probe.runtime_code_storage_key,
    probed_block_numbers: Array.from({ length: finalCheckpoint.number + 1 }, (_, index) => index),
    expected_per_endpoint: count,
    endpoint_a_probe_count: count,
    endpoint_b_probe_count: count,
    missing_count: transcript.filter((record) => !record.present).length,
    mismatch_count: transcript.filter((record) => !record.equal).length,
    completed_before_source_use: true,
    transcript,
    transcript_sha256: canonicalSha256(transcript),
  };
}

async function socketRefused(address) {
  return await new Promise((resolve) => {
    const socket = net.connect({ host: "127.0.0.1", port: Number(address.split(":").at(-1)) });
    const timer = setTimeout(() => {
      socket.destroy();
      resolve(false);
    }, 100);
    socket.once("connect", () => {
      clearTimeout(timer);
      socket.destroy();
      resolve(false);
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      resolve(error.code === "ECONNREFUSED");
    });
  });
}

async function socketInventory(fixture, nodesByRole) {
  const result = await runProcess("/usr/bin/ss", ["-H", "-lntup"], {
    label: "ss listener inventory",
    timeout: 10_000,
    env: { PATH: "/usr/bin:/bin", LANG: "C", LC_ALL: "C" },
  });
  requireCondition(result.code === 0, "ss listener inventory failed");
  const parsed = parseSsInventory(result.stdout);
  const records = [];
  const expectedAddresses = new Set();
  const expectedPids = new Set();
  for (const node of fixture.topology.nodes) {
    const pid = nodesByRole[node.role].pid;
    expectedPids.add(pid);
    for (const address of node.listeners) {
      expectedAddresses.add(address);
      const rows = parsed.filter((row) => row.local_address === address);
      requireCondition(
        rows.length === 1 && rows[0].pids.length === 1 && rows[0].pids[0] === pid,
        `listener ${address} is not uniquely owned by ${node.role}/${pid}`,
      );
      requireCondition(rows[0].protocol === "tcp", `${address} is not a TCP listener`);
      records.push({ protocol: rows[0].protocol, address, pid, raw_line: rows[0].raw_line });
    }
  }
  const unexpected = parsed
    .filter((row) => !expectedAddresses.has(row.local_address))
    .map((row) => row.raw_line);
  const unexpectedPids = [...new Set(parsed.flatMap((row) => row.pids))]
    .filter((pid) => !expectedPids.has(pid))
    .sort((left, right) => left - right);
  requireCondition(unexpected.length === 0, "unexpected listener exists in the private namespace");
  requireCondition(unexpectedPids.length === 0, "unexpected listener-owning process exists");
  return { records, raw: result.stdout, unexpected, unexpected_pids: unexpectedPids };
}

async function capturedSocketInventory(label) {
  const result = await runProcess("/usr/bin/ss", ["-H", "-lntup"], {
    label,
    timeout: 10_000,
    env: { PATH: "/usr/bin:/bin", LANG: "C", LC_ALL: "C" },
  });
  requireCondition(result.code === 0 && result.stderr.length === 0, `${label} failed`);
  return {
    command: ["/usr/bin/ss", "-H", "-lntup"],
    stdout_sha256: sha256(result.stdout),
    stdout_utf8: result.stdout.toString("utf8"),
    records: parseSsInventory(result.stdout),
  };
}

function insertBeforeFlag(values, flag, inserted, side) {
  const separator = values.indexOf("--");
  const start = side === "relay-side" ? separator + 1 : 0;
  const end = side === "relay-side" || separator < 0 ? values.length : separator;
  const relative = values.slice(start, end).indexOf(flag);
  requireCondition(relative >= 0, `${side} fixture lacks ${flag}`);
  values.splice(start + relative, 0, ...inserted);
}

function mutateFlag(values, flag, side, operation) {
  const separator = values.indexOf("--");
  const start = side === "relay-side" ? separator + 1 : 0;
  const end = side === "relay-side" || separator < 0 ? values.length : separator;
  const relative = values.slice(start, end).indexOf(flag);
  requireCondition(relative >= 0, `${side} fixture lacks ${flag}`);
  operation(start + relative);
}

function rawGeneratedArgv(node) {
  const values = node.argv.slice(1);
  const ports = {
    "relay-a": [["30333", "9944"]],
    "relay-b": [["30334", "9945"]],
    "collator-a": [["30335", "9988"], ["30337", "9990"]],
    "collator-b": [["30336", "9989"], ["30338", "9991"]],
  }[node.role];
  for (const [index, side] of ["primary", "relay-side"].entries()) {
    if (index >= ports.length) break;
    mutateFlag(values, "--experimental-rpc-endpoint", side, (position) => {
      const replacement = ["--port", ports[index][0], "--rpc-port", ports[index][1]];
      if (side === "primary")
        replacement.push("--rpc-methods", "unsafe", "--rpc-cors", "all");
      values.splice(position, 2, ...replacement);
    });
  }
  return values;
}

async function executeNormalizerRejectionMatrix(fixture, initialNodes, repoRoot, networkRoot) {
  const launcher = path.join(repoRoot, "chain/tools/zombienet-node-launcher.sh");
  const normalizer = path.join(repoRoot, "chain/tools/normalize-node-argv.sh");
  const environment = {
    CUBIKAN_ZOMBIENET_RUN_DIR: networkRoot,
    HOME: "/home/charles",
    LANG: "C",
    LC_ALL: "C",
    PATH: "/usr/bin:/bin",
    PWD: repoRoot,
    SHLVL: "0",
    TZ: "UTC",
  };
  const bases = {
    relay: rawGeneratedArgv(initialNodes["relay-a"]),
    collator: rawGeneratedArgv(initialNodes["collator-a"]),
  };
  const mutations = new Map();
  const define = (name, role, base, change, directCommand = null) => {
    const values = [...bases[base]];
    change(values);
    mutations.set(name, { role, values, directCommand });
  };
  define("unexpected-positional", "relay-a", "relay", (values) => values.push("unexpected"));
  define("unknown-flag-primary", "relay-a", "relay", (values) => values.push("--mystery"));
  define("unknown-flag-relay-side", "collator-a", "collator", (values) => values.push("--mystery"));
  define("duplicate-value-flag-primary", "relay-a", "relay", (values) => values.push("--rpc-port", "9944"));
  define("duplicate-value-flag-relay-side", "collator-a", "collator", (values) => values.push("--rpc-port", "9990"));
  define("duplicate-switch-primary", "relay-a", "relay", (values) => values.push("--validator"));
  define("duplicate-switch-relay-side", "collator-a", "collator", (values) => values.push("--no-mdns"));
  define("both-rpc-spellings-primary", "relay-a", "relay", (values) => values.push("--ws-port", "9944"));
  define("both-rpc-spellings-relay-side", "collator-a", "collator", (values) => values.push("--ws-port", "9990"));
  define("generated-structured-rpc-endpoint", "relay-a", "relay", (values) =>
    values.push(
      "--experimental-rpc-endpoint",
      "listen-addr=127.0.0.1:9944,methods=unsafe,cors=all",
    ),
  );
  define("missing-value-primary", "relay-a", "relay", (values) =>
    mutateFlag(values, "--rpc-port", "primary", (index) => values.splice(index + 1, 1)),
  );
  define("missing-value-relay-side", "collator-a", "collator", (values) =>
    mutateFlag(values, "--rpc-port", "relay-side", (index) => values.splice(index + 1, 1)),
  );
  define("missing-primary-rpc-cors", "relay-a", "relay", (values) =>
    mutateFlag(values, "--rpc-cors", "primary", (index) => values.splice(index, 2)),
  );
  define("missing-primary-rpc-methods", "relay-a", "relay", (values) =>
    mutateFlag(values, "--rpc-methods", "primary", (index) => values.splice(index, 2)),
  );
  define("wrong-primary-rpc-methods", "relay-a", "relay", (values) =>
    mutateFlag(values, "--rpc-methods", "primary", (index) => {
      values[index + 1] = "safe";
    }),
  );
  define("primary-rpc-policy-on-relay-side", "collator-a", "collator", (values) =>
    values.push("--rpc-cors", "all"),
  );
  define("missing-required-primary", "relay-a", "relay", (values) =>
    mutateFlag(values, "--rpc-port", "primary", (index) => values.splice(index, 2)),
  );
  define("missing-required-relay-side", "collator-a", "collator", (values) =>
    mutateFlag(values, "--rpc-port", "relay-side", (index) => values.splice(index, 2)),
  );
  define("nonloopback-bootnode-primary", "relay-a", "relay", (values) =>
    mutateFlag(values, "--bootnodes", "primary", (index) => {
      values[index + 1] = values[index + 1].replace("127.0.0.1", "203.0.113.1");
    }),
  );
  define("nonloopback-bootnode-relay-side", "collator-a", "collator", (values) =>
    mutateFlag(values, "--bootnodes", "relay-side", (index) => {
      values[index + 1] = values[index + 1].replace("127.0.0.1", "203.0.113.1");
    }),
  );
  define("relay-separator-on-validator", "relay-a", "relay", (values) => values.push("--"));
  define("missing-relay-separator-on-collator", "collator-a", "collator", (values) =>
    values.splice(values.indexOf("--"), 1),
  );
  define("missing-blocks-pruning-archive", "collator-a", "collator", (values) =>
    mutateFlag(values, "--blocks-pruning", "primary", (index) => values.splice(index, 2)),
  );
  define("missing-state-pruning-archive", "collator-a", "collator", (values) =>
    mutateFlag(values, "--state-pruning", "primary", (index) => values.splice(index, 2)),
  );
  define("missing-worker-path", "relay-a", "relay", (values) =>
    mutateFlag(values, "--workers-path", "primary", (index) => values.splice(index, 2)),
  );
  define("wrong-worker-path", "relay-a", "relay", (values) =>
    mutateFlag(values, "--workers-path", "primary", (index) => {
      values[index + 1] = "/tmp/pvf-workers";
    }),
  );
  define("missing-execute-worker-cap", "relay-a", "relay", (values) =>
    mutateFlag(values, "--execute-workers-max-num", "primary", (index) =>
      values.splice(index, 2),
    ),
  );
  define("wrong-soft-worker-cap", "relay-a", "relay", (values) =>
    mutateFlag(values, "--prepare-workers-soft-max-num", "primary", (index) => {
      values[index + 1] = "2";
    }),
  );
  define("duplicate-hard-worker-cap", "relay-a", "relay", (values) =>
    values.push("--prepare-workers-hard-max-num", "1"),
  );
  define("missing-relay-side-workers", "collator-a", "collator", (values) =>
    mutateFlag(values, "--workers-path", "relay-side", (index) => values.splice(index, 2)),
  );
  define("wrong-relay-side-worker-cap", "collator-a", "collator", (values) =>
    mutateFlag(values, "--prepare-workers-hard-max-num", "relay-side", (index) => {
      values[index + 1] = "0";
    }),
  );
  define("worker-path-on-collator-primary", "collator-a", "collator", (values) =>
    insertBeforeFlag(
      values,
      "--rpc-port",
      ["--workers-path", "/run/cubikan-exec/pvf-workers"],
      "primary",
    ),
  );
  define("unsafe-chain-path", "relay-a", "relay", (values) =>
    mutateFlag(values, "--chain", "primary", (index) => { values[index + 1] = "/tmp/escape.json"; }),
  );
  define("unsafe-base-path", "collator-a", "collator", (values) =>
    mutateFlag(values, "--base-path", "relay-side", (index) => { values[index + 1] = "/tmp/escape"; }),
  );
  define(
    "substituted-node-path",
    "relay-a",
    "relay",
    () => {},
    "/usr/lib/cargo/bin/coreutils/false",
  );
  define("nonregular-node-asset", "relay-a", "relay", () => {}, networkRoot);

  const evidence = [];
  for (const caseName of fixture.topology.normalizer_rejection_cases) {
    const before = await nodeProcessInventory(fixture, repoRoot);
    let result;
    let executedExecutable;
    let executedArgs;
    let executedEnvironment;
    if (caseName === "grammar-hash-mismatch") {
      const source = await fsp.readFile(normalizer, "utf8");
      const modified = source.replace(
        /readonly EXPECTED_GRAMMAR_SHA256="[0-9a-f]{64}"/,
        `readonly EXPECTED_GRAMMAR_SHA256="${"0".repeat(64)}"`,
      );
      requireCondition(modified !== source, "normalizer grammar oracle was not mutable in memory");
      executedExecutable = "/usr/bin/bash";
      executedArgs = [
          "--noprofile", "--norc", "-p", "-c", modified, normalizer,
          "__cubikan_normalizer_bound_memory_v1__", normalizer, sha256(Buffer.from(modified)),
          modified, "--verify-grammar",
        ];
      executedEnvironment = { ...environment, CUBIKAN_NORMALIZER_SANITIZED: "1" };
      result = await runProcess(
        executedExecutable,
        executedArgs,
        {
          cwd: repoRoot,
          label: caseName,
          timeout: 30_000,
          env: executedEnvironment,
        },
      );
    } else {
      const mutation = mutations.get(caseName);
      requireCondition(mutation, `normalizer case ${caseName} lacks an executable mutation`);
      executedExecutable = mutation.directCommand ? normalizer : launcher;
      executedArgs = mutation.directCommand
        ? ["--role", mutation.role, "--print0", "--", mutation.directCommand, ...mutation.values]
        : ["--role", mutation.role, "--print0", "--", ...mutation.values];
      executedEnvironment = environment;
      result = await runProcess(executedExecutable, executedArgs, {
        cwd: repoRoot,
        label: caseName,
        timeout: 60_000,
        env: environment,
      });
    }
    const after = await nodeProcessInventory(fixture, repoRoot);
    const beforeGenerations = new Set(before.map((record) => `${record.pid}:${record.start_time_ticks}`));
    const launched = after.filter(
      (record) => !beforeGenerations.has(`${record.pid}:${record.start_time_ticks}`),
    ).length;
    requireCondition(
      Number.isInteger(result.code) && result.code !== 0 && result.signal === null && launched === 0,
      `normalizer mutation ${caseName} did not reject before node launch`,
    );
    evidence.push({
      case: caseName,
      rejected: true,
      exit_code: result.code,
      launched_node_processes: launched,
      executable: executedExecutable,
      argv_sha256: canonicalSha256(executedArgs),
      environment_sha256: canonicalSha256(executedEnvironment),
      stdout_sha256: sha256(result.stdout),
      stderr_sha256: sha256(result.stderr),
      node_processes_before: before,
      node_processes_after: after,
    });
  }
  return evidence;
}

async function endpointIdentity(
  api,
  relayApi,
  node,
  checkpoint,
  projectionCheckpointSha,
) {
  const relayGenesis = await rpcGenesisHash(relayApi);
  const paraGenesis = await rpcGenesisHash(api);
  const metadata = (await api.rpc.state.getMetadata(checkpoint.hash)).toHex();
  const runtime = await storageHex(api, RUNTIME_CODE_KEY, checkpoint.hash);
  const runtimeVersion = await api.rpc.state.getRuntimeVersion(checkpoint.hash);
  const cubikanQuery = api.query.cubiKan ?? api.query.cubikan;
  requireCondition(cubikanQuery?.deploymentAnchor, "live Cubikan deployment storage is unavailable");
  const [deployment, eventSchemaVersion, palletStorageVersion] = await Promise.all([
    cubikanQuery.deploymentAnchor.at(checkpoint.hash),
    cubikanQuery.eventSchemaVersion.at(checkpoint.hash),
    cubikanQuery.palletStorageVersion.at(checkpoint.hash),
  ]);
  const deploymentId = deployment.toHex();
  const runtimeBytes = Buffer.from(runtime.slice(2), "hex");
  const runtimeCodeChainHash = api.registry.hash(runtimeBytes).toHex();
  requireCondition(
    /^0x[0-9a-f]{64}$/.test(deploymentId) &&
      /^0x[0-9a-f]{64}$/.test(runtimeCodeChainHash) &&
      typeof runtime === "string" &&
      /^0x[0-9a-f]+$/.test(runtime),
    "endpoint identity lacks live deployment/:code evidence",
  );
  return {
    relay_genesis_hash: relayGenesis,
    relay_genesis_payload_sha256: node.relay_side_chain_spec.genesis_payload_sha256,
    parachain_genesis_hash: paraGenesis,
    parachain_genesis_payload_sha256: node.primary_chain_spec.genesis_payload_sha256,
    deployment_id: deploymentId,
    checkpoint_hash: checkpoint.hash,
    runtime_spec_version: runtimeVersion.specVersion.toNumber(),
    event_schema_version: Number(eventSchemaVersion.toString()),
    pallet_storage_version: Number(palletStorageVersion.toString()),
    runtime_code_sha256: sha256(runtimeBytes),
    runtime_code_chain_hash: runtimeCodeChainHash,
    metadata_sha256: sha256(Buffer.from(metadata.slice(2), "hex")),
    projection_checkpoint_sha256: projectionCheckpointSha,
  };
}

async function externalConnectivityDenied() {
  return await new Promise((resolve) => {
    const socket = net.connect({ host: "198.51.100.1", port: 443 });
    const timer = setTimeout(() => {
      socket.destroy();
      resolve(true);
    }, 500);
    socket.once("connect", () => {
      clearTimeout(timer);
      socket.destroy();
      resolve(false);
    });
    socket.once("error", () => {
      clearTimeout(timer);
      resolve(true);
    });
  });
}

async function writeOwnerFile(filePath, bytes, maximumBytes = MAX_AUDIT_ARTIFACT_BYTES) {
  requireCondition(
    Buffer.isBuffer(bytes) &&
      Number.isSafeInteger(maximumBytes) &&
      maximumBytes > 0 &&
      maximumBytes <= MAX_RAW_CHAIN_SPEC_BYTES &&
      bytes.length <= maximumBytes,
    `audit artifact exceeds the retained bound: ${filePath}`,
  );
  await fsp.mkdir(path.dirname(filePath), { recursive: true, mode: 0o700 });
  await fsp.writeFile(filePath, bytes, { mode: 0o600, flag: "wx" });
  await fsp.chmod(filePath, 0o600);
}

async function copyOwnerFile(
  source,
  destination,
  allowEmpty = false,
  maximumBytes = MAX_AUDIT_ARTIFACT_BYTES,
) {
  const { bytes, evidence } = await openedFileEvidence(source, maximumBytes);
  requireCondition(allowEmpty || bytes.length > 0, `empty audit source ${source}`);
  await writeOwnerFile(destination, bytes, maximumBytes);
  return evidence;
}

async function waitForStableFile(filePath, label) {
  let prior = null;
  await waitFor(async () => {
    const current = identityFromStat(await fsp.stat(filePath, { bigint: true }));
    const stable = prior !== null && canonicalSha256(current) === canonicalSha256(prior);
    prior = current;
    return stable && current.size > 0;
  }, `${label} flush`, 30_000, 50);
}

function jsonLines(values) {
  return Buffer.from(values.map((value) => canonicalBytes(value).toString("utf8")).join("\n") + "\n");
}

function submissionLaneName(directory, deploymentId, signer) {
  const directoryBytes = Buffer.from(directory, "utf8");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(directoryBytes.length);
  const digest = createHash("sha256")
    .update(Buffer.from("CubiKan signer lane v1\0", "utf8"))
    .update(length)
    .update(directoryBytes)
    .update(Buffer.from(deploymentId.slice(2), "hex"))
    .update(Buffer.from(signer.slice(2), "hex"))
    .digest("hex");
  return `cubikan-submission-${digest}.lock`;
}

async function submissionLaneInventory(directory, deploymentId, signerAccounts) {
  const expectedNames = Object.entries(signerAccounts)
    .map(([signer, account]) => ({
      signer,
      account,
      name: submissionLaneName(directory, deploymentId, account),
    }))
    .sort((left, right) => left.name.localeCompare(right.name, "en"));
  const entries = (await fsp.readdir(directory)).sort((left, right) => left.localeCompare(right, "en"));
  const actualLockNames = entries.filter((name) => name.endsWith(".lock"));
  const residues = entries.filter(
    (name) =>
      name.endsWith(".journal") ||
      name.endsWith(".tmp") ||
      name.endsWith("-wal") ||
      name.endsWith("-shm") ||
      name.endsWith("-journal"),
  );
  requireCondition(
    JSON.stringify(actualLockNames) === JSON.stringify(expectedNames.map(({ name }) => name)) &&
      residues.length === 0,
    "submission-lane inventory contains a missing/extra lock or durable residue",
  );
  const locks = [];
  for (const expected of expectedNames) {
    const filePath = path.join(directory, expected.name);
    const symbolic = await fsp.lstat(filePath, { bigint: true });
    const identity = identityFromStat(symbolic);
    requireCondition(
      symbolic.isFile() &&
        !symbolic.isSymbolicLink() &&
        identity.mode === "0600" &&
        identity.link_count === 1 &&
        identity.size === 0 &&
        safeNumber(symbolic.uid, "submission lock owner") === process.getuid(),
      `submission lane ${expected.name} is not one empty owner-only regular inode`,
    );
    locks.push({
      ...expected,
      path: filePath,
      owner_uid: safeNumber(symbolic.uid, "submission lock owner"),
      identity,
    });
  }
  return {
    format: "cubikan-submission-lane-audit-v1",
    directory,
    deployment_id: deploymentId,
    locks,
    prohibited_residue_names: residues,
  };
}

function firstDisallowedUrl(text, fixture) {
  const matches =
    text.match(/(?:git\+ssh|https?|wss?|ssh|git):\/\/[^\s"'<>]+/giu) ?? [];
  const allowedHosts = new Set(fixture.audit.allowed_hosts);
  const allowedPorts = new Set(
    fixture.topology.nodes.flatMap((node) =>
      node.listeners.map((address) => address.slice(address.lastIndexOf(":") + 1)),
    ),
  );
  return matches.find((candidate) => {
    try {
      const parsed = new URL(candidate);
      const hostname = parsed.hostname.replace(/^\[|\]$/g, "");
      return (
        parsed.username !== "" ||
        parsed.password !== "" ||
        !allowedHosts.has(hostname) ||
        !allowedPorts.has(parsed.port)
      );
    } catch {
      return true;
    }
  }) ?? null;
}

async function auditTree(artifactRoot, fixture) {
  const expectedDirectories = ["", "action", "config", "fixture", "journal", "log", "socket"];
  const walkedDirectories = [];
  const walkedFiles = [];
  let symbolicLinkCount = 0;
  let nonRegularCount = 0;
  async function walk(relative) {
    const current = path.join(artifactRoot, relative);
    walkedDirectories.push(relative);
    const entries = await fsp.readdir(current, { withFileTypes: true });
    entries.sort((left, right) => left.name.localeCompare(right.name, "en"));
    for (const entry of entries) {
      const child = relative ? `${relative}/${entry.name}` : entry.name;
      if (entry.isSymbolicLink()) symbolicLinkCount += 1;
      else if (entry.isDirectory()) await walk(child);
      else if (entry.isFile()) walkedFiles.push(child);
      else nonRegularCount += 1;
    }
  }
  await walk("");
  walkedDirectories.sort();
  requireCondition(
    JSON.stringify(walkedDirectories) === JSON.stringify(expectedDirectories),
    "audit directory walk differs from the exact inventory",
  );
  const expected = fixture.audit.required_artifacts.map((artifact) => artifact.logical_path);
  requireCondition(JSON.stringify(walkedFiles) === JSON.stringify(expected), "audit file walk drifted");
  const fileObjects = new Set();
  let hardLinkAliasCount = 0;
  let scannedByteCount = 0;
  const artifacts = [];
  for (let index = 0; index < expected.length; index += 1) {
    const logicalPath = expected[index];
    const fixtureArtifact = fixture.audit.required_artifacts[index];
    const absolute = path.join(artifactRoot, logicalPath);
    const rawChainSpec = /^config\/(?:collator|relay)-[ab]\.raw\.json$/.test(logicalPath);
    const maximumBytes = rawChainSpec
      ? MAX_RAW_CHAIN_SPEC_BYTES
      : MAX_AUDIT_ARTIFACT_BYTES;
    const read = await openedFileEvidence(absolute, maximumBytes);
    const object = `${read.evidence.path_before.device}:${read.evidence.path_before.inode}`;
    if (fileObjects.has(object) || read.evidence.path_before.link_count !== 1) hardLinkAliasCount += 1;
    fileObjects.add(object);
    const text = read.bytes.toString("utf8");
    const publicUrl = firstDisallowedUrl(text, fixture);
    const nonloopbackHits = publicUrl ? [publicUrl] : [];
    const forbiddenHits = FORBIDDEN_TEXT_PATTERNS.flatMap((pattern) => text.match(pattern) ?? []);
    scannedByteCount += read.bytes.length;
    requireCondition(
      scannedByteCount <= MAX_AUDIT_TOTAL_BYTES,
      "audit tree exceeds the total retained byte bound",
    );
    artifacts.push({
      kind: fixtureArtifact.kind,
      logical_path: logicalPath,
      path: absolute,
      read: read.evidence,
      owner_only: (Number.parseInt(read.evidence.path_before.mode, 8) & 0o077) === 0,
      forbidden_hits: forbiddenHits,
      nonloopback_hits: nonloopbackHits,
    });
  }
  requireCondition(symbolicLinkCount === 0, "audit tree contains a symlink");
  requireCondition(nonRegularCount === 0, "audit tree contains a special file");
  requireCondition(hardLinkAliasCount === 0, "audit tree contains a hard-link alias");
  requireCondition(
    artifacts.every(
      (artifact) =>
        artifact.nonloopback_hits.length === 0 && artifact.forbidden_hits.length === 0,
    ),
    "audit tree contains a public URL, hostile helper, or secret-bearing marker",
  );
  const rootStat = await fsp.stat(artifactRoot, { bigint: true });
  return {
    artifact_root: artifactRoot,
    artifact_root_device: canonicalIdentityNumber(rootStat.dev, "audit root device"),
    artifact_root_inode: canonicalIdentityNumber(rootStat.ino, "audit root inode"),
    recursive_walk_complete: true,
    walked_directories: walkedDirectories,
    symbolic_link_count: symbolicLinkCount,
    hard_link_alias_count: hardLinkAliasCount,
    non_regular_count: nonRegularCount,
    artifacts,
    scanned_byte_count: scannedByteCount,
  };
}

async function atomicPublish(filePath, value) {
  const bytes = Buffer.concat([canonicalBytes(value), Buffer.from("\n")]);
  const temporary = `${filePath}.tmp-${process.pid}`;
  const handle = await fsp.open(temporary, "wx", 0o600);
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  await fsp.chmod(temporary, 0o600);
  await fsp.rename(temporary, filePath);
  const directory = await fsp.open(path.dirname(filePath), "r");
  try {
    await directory.sync();
  } finally {
    await directory.close();
  }
}

async function main() {
  const args = parseArguments(process.argv.slice(2));
  const fixturePath = await fsp.realpath(args.fixture);
  const configPath = await fsp.realpath(args.config);
  const zombienetRoot = await fsp.realpath(args.zombienet_root);
  requireCondition(fixturePath === args.fixture, "fixture path is not canonical");
  requireCondition(configPath === args.config, "config path is not canonical");
  requireCondition(zombienetRoot === args.zombienet_root, "Zombienet root is not canonical");
  const { bytes: fixtureBytes, value: fixture } = await readUniqueJson(fixturePath);
  requireCondition(fixture.format === "cubikan-chain-e2e-journey-v1", "fixture format drifted");
  const readinessPolicy = fixture.checkpoints?.pre_mutation_readiness;
  requireCondition(
    readinessPolicy?.para_id === 1000 &&
      readinessPolicy.minimum_finalized_number === 1 &&
      readinessPolicy.required_progress_blocks === 1 &&
      readinessPolicy.expected_scheduler_cores === 1 &&
      readinessPolicy.expected_distinct_aura_authorities === 2 &&
      readinessPolicy.maximum_best_finalized_gap === 8 &&
      readinessPolicy.timeout_milliseconds === PHASE_TIMEOUT_MS &&
      readinessPolicy.rpc_timeout_milliseconds === 10_000 &&
      readinessPolicy.poll_interval_milliseconds === 250 &&
      Object.keys(readinessPolicy).length === 9,
    "pre-mutation readiness policy drifted",
  );
  const repoRoot = path.dirname(path.dirname(path.dirname(fixturePath)));
  const supportedRoot = await fsp.realpath(process.env.CUBIKAN_TEST_SUPPORTED_ROOT ?? "");
  const workRoot = args.work_root;
  requireCondition(workRoot.startsWith(`${supportedRoot}/`), "work root escaped supported root");
  const terminationPhasePath = path.join(path.dirname(workRoot), ".driver.phase");
  requireCondition(
    args.evidence === path.join(path.dirname(workRoot), "evidence-v1.json") &&
      terminationPhasePath.startsWith(`${supportedRoot}${path.sep}`),
    "journey phase path escaped the exact session root",
  );
  requireCondition(process.env.CUBIKAN_ZOMBIENET_RUN_DIR === path.join(workRoot, "network"), "network dir contract drifted");
  const localBinary = await fsp.realpath(process.env.CUBIKAN_LOCAL_TEST_BINARY ?? "");
  const toolchainRoot = await fsp.realpath(fixture.launcher.materialized_toolchain_root);
  requireCondition(
    toolchainRoot === fixture.launcher.materialized_toolchain_root,
    "private materialized toolchain root is not canonical",
  );
  const toolchainRootStat = await fsp.lstat(toolchainRoot, { bigint: true });
  const toolchainRootIdentityBefore = identityFromStat(toolchainRootStat);
  const toolchainFilesystemMagic = (
    await fsp.statfs(toolchainRoot, { bigint: true })
  ).type.toString(16);
  const toolchainMountBefore = await exactTmpfsBindMountEvidence(
    toolchainRoot,
    "materialized toolchain",
  );
  const toolchainWriteProbeErrno = await readOnlyWriteProbe(
    toolchainRoot,
    "materialized toolchain",
  );
  requireCondition(
    toolchainRootStat.isDirectory() &&
      !toolchainRootStat.isSymbolicLink() &&
      toolchainRootIdentityBefore.mode === fixture.launcher.materialized_toolchain_mode &&
      toolchainFilesystemMagic === fixture.launcher.materialized_toolchain_filesystem_magic &&
      toolchainWriteProbeErrno === fixture.launcher.materialized_toolchain_write_probe_errno,
    "private materialized toolchain root identity/filesystem/write denial drifted",
  );
  const materializedNode = await fsp.realpath(fixture.launcher.materialized_node_path);
  requireCondition(
    materializedNode === fixture.launcher.materialized_node_path &&
      process.execPath === materializedNode,
    "orchestrator did not execute the exact materialized pinned Node path",
  );
  const localBinaryBefore = await openedFileEvidence(localBinary);
  const materializedNodeBefore = await openedFileEvidence(
    materializedNode,
    fixture.launcher.node_executable_size,
  );
  const procNodeBefore = await openedProcExecutableEvidence(
    process.pid,
    materializedNode,
    fixture.launcher.node_executable_size,
  );
  requireCondition(
    materializedNodeBefore.evidence.bytes_read === fixture.launcher.node_executable_size &&
      materializedNodeBefore.evidence.sha256 === fixture.launcher.node_executable_sha256 &&
      procNodeBefore.evidence.bytes_read === fixture.launcher.node_executable_size &&
      procNodeBefore.evidence.sha256 === fixture.launcher.node_executable_sha256 &&
      JSON.stringify(materializedNodeBefore.evidence.path_before) ===
        JSON.stringify(procNodeBefore.evidence.path_before),
    "materialized Node and /proc/self/exe do not match the independent executable oracle",
  );
  const networkRoot = process.env.CUBIKAN_ZOMBIENET_RUN_DIR;
  await fsp.mkdir(networkRoot, { mode: 0o700 });
  const networkRootStat = await fsp.lstat(networkRoot, { bigint: true });
  requireCondition(
    networkRootStat.isDirectory() &&
      !networkRootStat.isSymbolicLink() &&
      (safeNumber(networkRootStat.mode, "network root mode") & 0o7777) === 0o700 &&
      (await fsp.readdir(networkRoot)).length === 0,
    "network root is not one exact empty private directory",
  );

  const pins = [];
  for (const pin of fixture.pinned_inputs) {
    const pinPath = path.isAbsolute(pin.path) ? pin.path : path.join(repoRoot, pin.path);
    const read = await openedFileEvidence(pinPath);
    requireCondition(
      read.bytes.length === pin.size && read.evidence.sha256 === pin.sha256,
      `pin mismatch for ${pin.path}`,
    );
    pins.push({ path: pin.path, read: read.evidence, verified: true });
  }

  requireCondition(
    Number.isSafeInteger(fixture.launcher.bootstrap_log_max_bytes) &&
      fixture.launcher.bootstrap_log_max_bytes === 1_048_576,
    "bootstrap log retained bound drifted",
  );
  const bootstrapRoot = await fsp.realpath(path.join(workRoot, "bootstrap"));
  requireCondition(
    bootstrapRoot === path.join(workRoot, "bootstrap"),
    "bootstrap root is not the exact canonical work-root child",
  );
  const materializerLogPath = path.join(bootstrapRoot, "materializer.log");
  const genesisExportLogPath = path.join(bootstrapRoot, "genesis-export.log");
  const genesisHeadPath = await fsp.realpath(
    process.env.CUBIKAN_PARACHAIN_GENESIS_HEAD ?? "",
  );
  const genesisWasmPath = await fsp.realpath(
    process.env.CUBIKAN_PARACHAIN_GENESIS_WASM ?? "",
  );
  const chainSpecPath = await fsp.realpath(
    process.env.CUBIKAN_PARACHAIN_CHAIN_SPEC ?? "",
  );
  const parachainBinary = await fsp.realpath(
    process.env.CUBIKAN_POLKADOT_PARACHAIN ?? "",
  );
  requireCondition(
    genesisHeadPath === path.join(bootstrapRoot, "parachain-genesis-head") &&
      genesisWasmPath === path.join(bootstrapRoot, "parachain-genesis-wasm") &&
      chainSpecPath === path.join(repoRoot, "chain/config/cubikan-local.json") &&
      parachainBinary === path.join(repoRoot, "chain/.cache/downloads/polkadot-parachain"),
    "bootstrap path/environment contract drifted",
  );
  const runtimeWasmPin = fixture.pinned_inputs.find(
    (pin) => pin.path === "chain/artifacts/cubikan-runtime-v1.compact.compressed.wasm",
  );
  const parachainBinaryPin = fixture.pinned_inputs.find(
    (pin) => pin.path === "chain/.cache/downloads/polkadot-parachain",
  );
  const chainSpecPin = fixture.pinned_inputs.find(
    (pin) => pin.path === "chain/config/cubikan-local.json",
  );
  requireCondition(
    runtimeWasmPin && parachainBinaryPin && chainSpecPin,
    "bootstrap producer/spec/runtime pins are incomplete",
  );
  const materializerLogBeforeRead = await openedOwnerBootstrapFile(
    materializerLogPath,
    fixture.launcher.bootstrap_log_max_bytes,
    "materializer log",
  );
  const genesisExportLogBeforeRead = await openedOwnerBootstrapFile(
    genesisExportLogPath,
    fixture.launcher.bootstrap_log_max_bytes,
    "genesis export log",
  );
  const materializerLogBefore = materializerLogBeforeRead.evidence;
  const genesisExportLogBefore = genesisExportLogBeforeRead.evidence;
  const genesisHeadBefore = await openedOwnerBootstrapFile(
    genesisHeadPath,
    4096,
    "parachain genesis-head export",
  );
  const genesisWasmBefore = await openedOwnerBootstrapFile(
    genesisWasmPath,
    runtimeWasmPin.size * 2 + 2,
    "parachain genesis-wasm export",
  );
  const genesisHeadDecoded = decodeExactLowerHexExport(
    genesisHeadBefore.bytes,
    "parachain genesis-head export",
  );
  const genesisWasmDecoded = decodeExactLowerHexExport(
    genesisWasmBefore.bytes,
    "parachain genesis-wasm export",
  );
  requireCondition(
    genesisHeadDecoded.length === fixture.launcher.genesis_head_decoded_size &&
      sha256(genesisHeadDecoded) === fixture.launcher.genesis_head_decoded_sha256 &&
      genesisWasmDecoded.length === runtimeWasmPin.size &&
      sha256(genesisWasmDecoded) === runtimeWasmPin.sha256,
    "bootstrap genesis exports do not match the exact header/runtime oracle",
  );
  const bootstrapExportCommands = [
    [parachainBinary, "export-genesis-head", "--chain", chainSpecPath, genesisHeadPath],
    [parachainBinary, "export-genesis-wasm", "--chain", chainSpecPath, genesisWasmPath],
  ];
  const bootstrapMaterializerCommand = [
    path.join(repoRoot, "chain/tools/materialize-zombienet.sh"),
    "--output-dir",
    toolchainRoot,
  ];
  const execLine = (argv) => Buffer.from(`exec: ${argv.join(" ")}\n`, "utf8");
  const materializerExecLine = execLine(bootstrapMaterializerCommand);
  const exactExecLines = (bytes, label) => {
    requireCondition(
      bytes.length > 0 && bytes.at(-1) === 0x0a,
      `${label} is not terminal-LF-framed`,
    );
    const text = bytes.toString("utf8");
    requireCondition(
      Buffer.from(text, "utf8").equals(bytes),
      `${label} is not exact UTF-8`,
    );
    return text
      .split("\n")
      .filter((line) => line.startsWith("exec: "));
  };
  const materializerExecLines = exactExecLines(
    materializerLogBeforeRead.bytes,
    "materializer log",
  );
  const genesisExecLines = exactExecLines(
    genesisExportLogBeforeRead.bytes,
    "genesis export log",
  );
  const expectedGenesisExecLines = bootstrapExportCommands.map(
    (argv) => `exec: ${argv.join(" ")}`,
  );
  requireCondition(
    materializerLogBeforeRead.bytes
      .subarray(0, materializerExecLine.length)
      .equals(materializerExecLine) &&
      materializerExecLines.length === 1 &&
      materializerExecLines[0] === materializerExecLine.toString("utf8").trimEnd() &&
      genesisExportLogBeforeRead.bytes
        .subarray(0, execLine(bootstrapExportCommands[0]).length)
        .equals(execLine(bootstrapExportCommands[0])) &&
      genesisExecLines.length === expectedGenesisExecLines.length &&
      genesisExecLines.every(
        (line, index) => line === expectedGenesisExecLines[index],
      ),
    "bootstrap logs do not contain the exact closed execution transcript",
  );

  const orchestratorUrl = pathToFileURL(
    path.join(zombienetRoot, "javascript/packages/orchestrator/dist/index.js"),
  ).href;
  const utilsUrl = pathToFileURL(
    path.join(zombienetRoot, "javascript/packages/utils/dist/index.js"),
  ).href;
  const polkadotApiUrl = pathToFileURL(
    path.join(zombienetRoot, "javascript/node_modules/@polkadot/api/index.js"),
  ).href;
  const boundedNodeLogs = installBoundedNodeLogCapture(networkRoot);
  const diagnosticCapture = captureProcessDiagnostics(PROCESS_OUTPUT_LIMIT);
  const terminationPhase = installTerminationPhaseMarker(terminationPhasePath);
  let start;
  let readNetworkConfig;
  let ApiPromise;
  let WsProvider;
  try {
    [{ start }, { readNetworkConfig }, { ApiPromise, WsProvider }] = await Promise.all([
      import(orchestratorUrl),
      import(utilsUrl),
      import(polkadotApiUrl),
    ]);
  } catch (error) {
    boundedNodeLogs.restore();
    diagnosticCapture.restore();
    terminationPhase.remove();
    throw error;
  }
  requireCondition(
    typeof start === "function" &&
      typeof readNetworkConfig === "function" &&
      typeof ApiPromise === "function" &&
      typeof WsProvider === "function",
    "pinned Zombienet/Polkadot API missing",
  );

  let network = null;
  let networkStopped = false;
  let networkStopAttempted = false;
  let workRemoved = false;
  let relayBPaused = false;
  let manualRestart = null;
  let relaySideApiA = null;
  let relaySideApiB = null;
  let pvfRuntimeMonitor = null;
  const terminatedProcesses = [];
  try {
    terminationPhase.set("zombienet-start");
    const launchConfig = readNetworkConfig(configPath);
    network = await withTimeout("Zombienet start", 600_000, () =>
      start("", launchConfig, {
        monitor: false,
        spawnConcurrency: 1,
        inCI: false,
        dir: networkRoot,
        force: true,
        logType: "silent",
      }),
    );
    requireCondition(network?.client?.processMap, "native processMap is unavailable");
    terminationPhase.set("api-connect");
    const networkNodes = {
      "relay-a": network.nodesByName.alice,
      "relay-b": network.nodesByName.bob,
      "collator-a": network.nodesByName["alice-1"],
      "collator-b": network.nodesByName["bob-1"],
    };
    requireCondition(Object.values(networkNodes).every(Boolean), "exact four Zombienet nodes are missing");
    await Promise.all(Object.values(networkNodes).map((node) => node.connectApi()));
    const apiA = networkNodes["collator-a"].apiInstance;
    let apiB = networkNodes["collator-b"].apiInstance;
    let relayApiA = networkNodes["relay-a"].apiInstance;
    let relayApiB = networkNodes["relay-b"].apiInstance;
    relaySideApiA = await connectPolkadotApi(
      ApiPromise,
      WsProvider,
      "ws://127.0.0.1:9990",
      "collator A embedded relay-side API",
    );
    relaySideApiB = await connectPolkadotApi(
      ApiPromise,
      WsProvider,
      "ws://127.0.0.1:9991",
      "collator B embedded relay-side API",
    );

    const fixtureByRole = Object.fromEntries(fixture.topology.nodes.map((node) => [node.role, node]));
    const processNames = {
      "relay-a": "alice",
      "relay-b": "bob",
      "collator-a": "alice-1",
      "collator-b": "bob-1",
    };
    const initialNodes = {};
    terminationPhase.set("node-evidence");
    for (const role of Object.keys(processNames)) {
      const pid = network.client.processMap[processNames[role]]?.pid;
      requireCondition(Number.isSafeInteger(pid) && pid > 1, `${role} processMap PID missing`);
      initialNodes[role] = await nodeEvidence(
        fixtureByRole[role],
        pid,
        repoRoot,
        role.startsWith("collator") ? (role === "collator-a" ? apiA : apiB) : role === "relay-a" ? relayApiA : relayApiB,
        role === "collator-a"
          ? { api: relaySideApiA, endpoint: "ws://127.0.0.1:9990/" }
          : role === "collator-b"
            ? { api: relaySideApiB, endpoint: "ws://127.0.0.1:9991/" }
            : null,
      );
      terminatedProcesses.push({ pid, start_time_ticks: initialNodes[role].process_start_time_ticks_before });
    }
    const genesisHeadChainHash = apiA.registry.hash(genesisHeadDecoded).toHex();
    requireCondition(
      genesisHeadChainHash === initialNodes["collator-a"].primary_chain_spec.live_genesis_hash &&
        genesisHeadChainHash === initialNodes["collator-b"].primary_chain_spec.live_genesis_hash &&
        sha256(genesisWasmDecoded) === initialNodes["collator-a"].primary_runtime_sha256 &&
        sha256(genesisWasmDecoded) === initialNodes["collator-b"].primary_runtime_sha256,
      "bootstrap genesis exports diverged from both live collator identities",
    );
    const relaySpecs = [
      initialNodes["relay-a"].primary_chain_spec,
      initialNodes["relay-b"].primary_chain_spec,
      initialNodes["collator-a"].relay_side_chain_spec,
      initialNodes["collator-b"].relay_side_chain_spec,
    ];
    const parachainSpecs = [
      initialNodes["collator-a"].primary_chain_spec,
      initialNodes["collator-b"].primary_chain_spec,
    ];
    requireCondition(
      relaySpecs.every(
        (spec) =>
          spec.raw_size === relaySpecs[0].raw_size &&
          spec.raw_sha256 === relaySpecs[0].raw_sha256 &&
          spec.top_level_bootnodes.length === 0,
      ) &&
        parachainSpecs.every(
          (spec) =>
            spec.raw_size === parachainSpecs[0].raw_size &&
            spec.raw_sha256 === parachainSpecs[0].raw_sha256 &&
            spec.top_level_bootnodes.length === 0,
        ) &&
        relaySpecs[0].raw_sha256 !== parachainSpecs[0].raw_sha256,
      "launched relay/parachain chain-spec bytes are not exact, equal by family, and distinct across families",
    );
    const initialNodeProcesses = await nodeProcessInventory(fixture, repoRoot);
    const expectedInitialPids = Object.values(initialNodes).map((node) => node.pid).sort((a, b) => a - b);
    requireCondition(
      JSON.stringify(initialNodeProcesses.map((record) => record.pid)) ===
        JSON.stringify(expectedInitialPids) &&
        initialNodeProcesses.every((record) => record.parent_pid === process.pid),
      "private /proc inventory is not exactly the four orchestrator-owned sealed nodes",
    );
    const pvfStaticEvidence = await pvfWorkerStaticEvidence(fixture.pvf_workers, initialNodes);
    const pvfParentRoles = new Map(
      Object.values(initialNodes).map((node) => [node.pid, node.role]),
    );
    const pvfRoleContexts = new Map(
      Object.values(initialNodes).map((node) => [
        node.role,
        {
          data_root: node.data_directories.find((directory) =>
            node.role.startsWith("collator")
              ? directory.side === "relay-side"
              : directory.side === "primary",
          ).path,
          chain_spec_id: node.role.startsWith("collator")
            ? node.relay_side_chain_spec.chain_spec_id
            : node.primary_chain_spec.chain_spec_id,
        },
      ]),
    );
    pvfRuntimeMonitor = await createPvfRuntimeMonitor(
      fixture.pvf_workers,
      pvfParentRoles,
      pvfRoleContexts,
    );
    const sockets = await socketInventory(fixture, initialNodes);

    terminationPhase.set("normalizer-matrix");
    const normalizerMutations = await executeNormalizerRejectionMatrix(
      fixture,
      initialNodes,
      repoRoot,
      networkRoot,
    );

    terminationPhase.set("pre-mutation-readiness");
    const preMutationReadiness = await waitForPreMutationReadiness(
      relayApiA,
      relayApiB,
      apiA,
      apiB,
      readinessPolicy,
    );

    const projectionsRoot = path.join(workRoot, "projections");
    await fsp.mkdir(projectionsRoot, { mode: 0o700 });
    const uninterruptedDatabase = path.join(projectionsRoot, "uninterrupted.sqlite3");
    const submissions = [];
    const expectedPayloads = new Map();
    const lifecyclePhases = new Map();
    const submitMutation = async (mutation) => {
      const output = await runLocal(
        localBinary,
        uninterruptedDatabase,
        fixture.endpoints[mutation.endpoint],
        mutation.request,
        mutation.signer,
        repoRoot,
      );
      const coordinate = output.response.coordinate;
      requireCondition(output.response.outcome === "finalized_accepted", `${mutation.id} was not accepted`);
      expectedPayloads.set(mutation.id, expectedAcceptedPayload(mutation, lifecyclePhases));
      const association = output.response.effect?.association;
      submissions.push({
        id: mutation.id,
        phase: mutation.phase,
        signer: mutation.signer,
        endpoint: mutation.endpoint,
        executable: output.executable,
        argv: output.argv,
        environment: output.environment,
        environment_sha256: output.environment_sha256,
        work_category: mutation.work_category,
        operation: mutation.operation,
        request_sha256: canonicalSha256(mutation.request),
        response: output.response,
        response_body_hex: `0x${output.bytes.toString("hex")}`,
        response_body_size: output.bytes.length,
        response_sha256: sha256(output.bytes),
        response_stdout_size: output.stdout_bytes.length,
        response_stdout_sha256: sha256(output.stdout_bytes),
        result_kind: output.response.outcome,
        submission_count: 1,
        inclusion_count: 1,
        finalization_count: 1,
        accepted_event_count: 1,
        projection_applied: output.response.projection?.status === "caught_up",
        block_number: Number(coordinate.block_number),
        block_hash: coordinate.block_hash,
        extrinsic_hash: coordinate.extrinsic_hash,
        extrinsic_index: coordinate.extrinsic_index,
        event_index: coordinate.system_event_index,
        recorded_association_core_json: association
          ? JSON.stringify({
              unit_id: association.unit_id,
              subject:
                association.subject.type === "revision"
                  ? { kind: "revision", revision: Number(association.subject.revision) }
                  : { kind: "whole_unit" },
              reference: association.reference,
            })
          : null,
        endpoint_observations: [],
      });
    };

    terminationPhase.set("initial-submissions");
    for (const mutation of fixture.mutations.slice(0, 7)) {
      terminationPhase.set(`initial-submission-${mutation.id}`);
      await submitMutation(mutation);
    }
    const checkpointC = await waitEndpointsEqual(
      apiA,
      apiB,
      submissions.at(-1).block_number,
      "checkpoint C convergence",
    );
    const oldB = initialNodes["collator-b"];
    const oldBDataStat = identityFromStat(await fsp.stat(oldB.data_directory, { bigint: true }));
    const processEntryB = network.client.processMap["bob-1"];
    const frozenCommand = [...(processEntryB.cmd ?? [])];
    const frozenLogPath = processEntryB.logs;
    requireCondition(
      processEntryB.pid === oldB.pid &&
        frozenCommand.length > 0 &&
        frozenCommand.every((argument) => typeof argument === "string" && !argument.includes("\0")) &&
        path.isAbsolute(frozenLogPath),
      "collator B native process definition was not freezeable",
    );
    const configShaBefore = canonicalSha256(oldB.argv);
    const frozenCommandSha = canonicalSha256(frozenCommand);
    terminationPhase.set("collator-stop");
    const stopObservation = (async () => {
      await waitFor(
        () => processGenerationLive(oldB.pid, oldB.process_start_time_ticks_before).then((live) => !live),
        "old collator B generation exit",
      );
      let procProbeErrno = null;
      try {
        await fsp.lstat(procPath(oldB.pid, ""));
      } catch (error) {
        if (error?.code === "ENOENT") procProbeErrno = "ENOENT";
        else throw error;
      }
      requireCondition(procProbeErrno === "ENOENT", "old collator B /proc object was not absent");
      const errors = {};
      await waitFor(async () => {
        const results = await Promise.all(
          fixtureByRole["collator-b"].listeners.map(async (address) => [address, await socketRefused(address)]),
        );
        if (!results.every(([, refused]) => refused)) return false;
        for (const [address] of results) errors[address] = "ECONNREFUSED";
        return true;
      }, "collator B listener absence", 30_000, 5);
      return { listener_probe_errors: errors, proc_probe_errno: procProbeErrno };
    })();
    await Promise.all([apiB.disconnect(), relaySideApiB.disconnect()]);
    relaySideApiB = null;
    networkNodes["collator-b"].apiInstance = undefined;
    processEntryB.pid = undefined;
    requireCondition(processEntryB.pid === undefined, "collator B PID was not cleared before stop");
    const preSignalStat = await procStat(oldB.pid);
    const preSignalProc = await procObjectIdentity(oldB.pid);
    requireCondition(
      preSignalStat.start_time_ticks === oldB.process_start_time_ticks_before &&
        preSignalProc.device === oldB.proc_directory_device_after &&
        preSignalProc.inode === oldB.proc_directory_inode_after,
      "collator B process generation changed between disconnect and SIGTERM",
    );
    const stopSignalSent = process.kill(oldB.pid, "SIGTERM");
    requireCondition(stopSignalSent, "collator B SIGTERM was not delivered");
    const stoppedObservation = await stopObservation;
    const listenerProbeErrors = stoppedObservation.listener_probe_errors;
    const stoppedNodeProcesses = await nodeProcessInventory(fixture, repoRoot);
    const expectedStoppedPids = Object.values(initialNodes)
      .filter((node) => node.role !== "collator-b")
      .map((node) => node.pid)
      .sort((a, b) => a - b);
    requireCondition(
      JSON.stringify(stoppedNodeProcesses.map((record) => record.pid)) ===
        JSON.stringify(expectedStoppedPids),
      "stopped /proc inventory did not contain exactly the three survivor nodes",
    );
    await pvfRuntimeMonitor.sampleNow();
    const stoppedSockets = await capturedSocketInventory("stopped collator B socket inventory");
    requireCondition(
      stoppedSockets.records.every(
        (record) => !fixtureByRole["collator-b"].listeners.includes(record.local_address),
      ),
      "stopped collator B retained a listener in the complete ss inventory",
    );
    terminationPhase.set("survivor-submissions");
    for (const mutation of fixture.mutations.slice(7)) {
      terminationPhase.set(`survivor-submission-${mutation.id}`);
      await submitMutation(mutation);
    }
    const prePauseCandidate = await waitFor(
      async () => (await rpcFinalized(apiA)).number >= submissions.at(-1).block_number,
      "survivor candidate F",
      PHASE_TIMEOUT_MS,
      250,
    ).then(() => rpcFinalized(apiA));
    await Promise.all([
      withTimeout("pre-pause relay A API disconnect", 10_000, () => relayApiA.disconnect()),
      withTimeout("pre-pause relay B API disconnect", 10_000, () => relayApiB.disconnect()),
    ]);
    networkNodes["relay-a"].apiInstance = undefined;
    networkNodes["relay-b"].apiInstance = undefined;
    relayApiA = null;
    relayApiB = null;
    requireCondition(await networkNodes["relay-b"].pause(), "NetworkNode.pause failed for relay B");
    relayBPaused = true;
    const candidateF = await rpcFinalized(apiA);
    requireCondition(
      candidateF.number >= prePauseCandidate.number && candidateF.number >= submissions.at(-1).block_number,
      "survivor finalized head regressed while relay B was paused",
    );
    terminationPhase.set("finality-stability");
    const finalityStability = [];
    for (let index = 1; index <= fixture.checkpoints.stability_interval_count; index += 1) {
      finalityStability.push(
        await observeStableFinalityInterval(
          apiA,
          candidateF,
          index,
          fixture.checkpoints.stability_interval_milliseconds,
        ),
      );
    }
    const checkpointF = candidateF;

    terminationPhase.set("collator-restart-spawn");
    manualRestart = spawnFrozenNativeNode(network.client, processEntryB, frozenCommand);
    const restartedPid = manualRestart.child.pid;
    pvfParentRoles.set(restartedPid, "collator-b");
    requireCondition(
      processEntryB.pid === restartedPid && restartedPid !== oldB.pid,
      "collator B PID was not immediately republished as a new generation",
    );
    terminationPhase.set("collator-restart-native-readiness");
    await waitForRestartStep(manualRestart, "manual collator B native readiness", () =>
      network.client.wait_node_ready("bob-1"),
    );
    const restartedPrimaryEndpoint = new URL(fixture.endpoints["collator-b"]);
    const restartedPrimaryAddress = `${restartedPrimaryEndpoint.hostname}:${restartedPrimaryEndpoint.port}`;
    requireCondition(
      restartedPrimaryEndpoint.protocol === "ws:" &&
        restartedPrimaryEndpoint.hostname === "127.0.0.1" &&
        fixtureByRole["collator-b"].listeners.includes(restartedPrimaryAddress),
      "restarted collator B primary RPC endpoint is not one pinned loopback listener",
    );
    terminationPhase.set("collator-restart-primary-listener");
    await waitForRestartStep(manualRestart, "manual collator B primary listener", () =>
      waitFor(
        () => socketRefused(restartedPrimaryAddress).then((refused) => !refused),
        "manual collator B primary listener",
        PHASE_TIMEOUT_MS,
        25,
      ),
    );
    terminationPhase.set("collator-restart-api-reconnect");
    apiB = await connectRestartPolkadotApi(
      ApiPromise,
      WsProvider,
      manualRestart,
      fixture.endpoints["collator-b"],
      "manual collator B API reconnect",
    );
    networkNodes["collator-b"].apiInstance = apiB;
    terminationPhase.set("collator-restart-relay-api");
    relaySideApiB = await connectRestartPolkadotApi(
      ApiPromise,
      WsProvider,
      manualRestart,
      "ws://127.0.0.1:9991",
      "restarted collator B embedded relay-side API",
    );
    terminationPhase.set("collator-restart-convergence");
    const convergedF = await waitForRestartStep(
      manualRestart,
      "restarted collator B convergence",
      () => waitEndpointsEqual(apiA, apiB, checkpointF.number, "restarted B catch-up"),
    );
    requireCondition(convergedF.hash === checkpointF.hash, "restarted B converged to another F");

    terminationPhase.set("collator-restart-evidence");
    const restartedNode = await waitForRestartStep(
      manualRestart,
      "restarted collator B evidence",
      () =>
        nodeEvidence(
          fixtureByRole["collator-b"],
          restartedPid,
          repoRoot,
          apiB,
          { api: relaySideApiB, endpoint: "ws://127.0.0.1:9991/" },
        ),
    );
    const restartedListenerRecords = await listenerRecordsForNode(
      fixtureByRole["collator-b"],
      restartedPid,
    );
    const restartedNodeProcesses = await nodeProcessInventory(fixture, repoRoot);
    const expectedRestartedPids = [...expectedStoppedPids, restartedPid].sort((a, b) => a - b);
    requireCondition(
      JSON.stringify(restartedNodeProcesses.map((record) => record.pid)) ===
        JSON.stringify(expectedRestartedPids),
      "restarted /proc inventory did not contain exactly four sealed node generations",
    );
    const restartedSockets = await capturedSocketInventory("restarted node socket inventory");
    await pvfRuntimeMonitor.sampleNow();
    terminatedProcesses.push({ pid: restartedPid, start_time_ticks: restartedNode.process_start_time_ticks_before });
    const newBDataStat = identityFromStat(await fsp.stat(restartedNode.data_directory, { bigint: true }));
    terminationPhase.set("archive-and-observations");
    const archiveProbes = await probeArchive(apiA, apiB, fixture, checkpointF);
    requireCondition(archiveProbes.missing_count === 0 && archiveProbes.mismatch_count === 0, "archive probes diverged");

    for (
      let batchStart = 0;
      batchStart < submissions.length;
      batchStart += FINALIZED_OBSERVATION_CONCURRENCY
    ) {
      const batch = submissions.slice(
        batchStart,
        batchStart + FINALIZED_OBSERVATION_CONCURRENCY,
      );
      const observations = await orderedParallelOperations(
        batch.map((submission) => async () =>
          orderedParallelOperations([
            () =>
              finalizedObservation(
                apiA,
                "collator-a",
                submission,
                checkpointF,
                fixture.audit.dev_signer_accounts,
                expectedPayloads.get(submission.id),
              ),
            () =>
              finalizedObservation(
                apiB,
                "collator-b",
                submission,
                checkpointF,
                fixture.audit.dev_signer_accounts,
                expectedPayloads.get(submission.id),
              ),
          ]),
        ),
      );
      for (let offset = 0; offset < batch.length; offset += 1) {
        submissions[batchStart + offset].endpoint_observations = observations[offset];
      }
    }
    terminationPhase.set("parachain-census");
    const chainCensus = await finalizedChainCensus(
      apiA,
      apiB,
      fixture,
      submissions,
      checkpointF,
    );
    const sourceGateTranscript = [];
    let sourceGateSequence = 0;
    let sourceGateOpen = false;
    let sourceGateEvidence = null;
    const sourceGate = {
      authorize(label, sourceEndpoint) {
        requireCondition(
          ["both", "collator-a", "collator-b"].includes(sourceEndpoint),
          `${label} has an unknown source endpoint`,
        );
        if (!sourceGateOpen) {
          requireCondition(
            label === "uninterrupted" &&
              sourceEndpoint === "both" &&
              sourceGateTranscript.length === 0,
            `${label} attempted an unguarded pre-identity source open`,
          );
        } else {
          requireCondition(label !== "uninterrupted", "uninterrupted source reopened after gate");
        }
        sourceGateSequence += 1;
        const entry = {
          sequence: sourceGateSequence,
          kind: "source_open",
          label,
          source_endpoint: sourceEndpoint,
          gate_open: sourceGateOpen,
          identity_prerequisites_complete: sourceGateOpen,
          archive_prerequisites_complete: sourceGateOpen,
        };
        sourceGateTranscript.push(entry);
        return entry;
      },
      open(identityA, identityB, archiveEvidence) {
        const identityASha = canonicalSha256(identityA);
        const identityBSha = canonicalSha256(identityB);
        const archiveComplete =
          archiveEvidence.completed_before_source_use === true &&
          archiveEvidence.missing_count === 0 &&
          archiveEvidence.mismatch_count === 0 &&
          archiveEvidence.endpoint_a_probe_count === archiveEvidence.expected_per_endpoint &&
          archiveEvidence.endpoint_b_probe_count === archiveEvidence.expected_per_endpoint &&
          archiveEvidence.transcript_sha256 === canonicalSha256(archiveEvidence.transcript);
        requireCondition(
          !sourceGateOpen &&
            sourceGateTranscript.length === 1 &&
            sourceGateTranscript[0].label === "uninterrupted" &&
            !sourceGateTranscript[0].gate_open &&
            identityASha === identityBSha &&
            archiveComplete,
          "dual endpoint identity/archive prerequisites did not authorize the rebuild gate",
        );
        sourceGateSequence += 1;
        sourceGateOpen = true;
        const gateEntry = {
          sequence: sourceGateSequence,
          kind: "gate_open",
          label: "dual-endpoint-identity-archive",
          source_endpoint: null,
          gate_open: true,
          identity_prerequisites_complete: true,
          archive_prerequisites_complete: true,
        };
        sourceGateTranscript.push(gateEntry);
        sourceGateEvidence = {
          format: "cubikan-rebuild-source-gate-v1",
          gate_sequence: gateEntry.sequence,
          identity_a_sha256: identityASha,
          identity_b_sha256: identityBSha,
          archive_probes_sha256: canonicalSha256(archiveEvidence),
          transcript: sourceGateTranscript,
        };
      },
    };
    const snapshotOptions = {
      fixture,
      localBinary,
      repoRoot,
      sourceGate,
    };
    const contracts = Object.fromEntries(
      fixture.semantic_projection.snapshot_contracts.map((contract) => [contract.label, contract]),
    );
    const snapshotByLabel = {};
    terminationPhase.set("projection-snapshot-uninterrupted");
    snapshotByLabel.uninterrupted = await readSnapshot({
      ...snapshotOptions,
      contract: contracts.uninterrupted,
      database: uninterruptedDatabase,
      endpoint: fixture.endpoints["collator-a"],
      replacement: null,
    });
    const dbCheckpoint = snapshotByLabel.uninterrupted.sections.checkpoint[0];
    requireCondition(
      Number(dbCheckpoint.block_number) === checkpointF.number &&
        dbCheckpoint.block_hash === checkpointF.hash &&
        dbCheckpoint.last_global_sequence === String(fixture.mutations.length) &&
        /^0x[0-9a-f]{64}$/.test(dbCheckpoint.runtime_code_hash),
      "uninterrupted DB-backed checkpoint did not bind exact F/sequence/runtime identity",
    );
    const projectionCheckpointSha = canonicalSha256(dbCheckpoint);
    const identityA = await endpointIdentity(
      apiA,
      relaySideApiA,
      initialNodes["collator-a"],
      checkpointF,
      projectionCheckpointSha,
    );
    const identityB = await endpointIdentity(
      apiB,
      relaySideApiB,
      restartedNode,
      checkpointF,
      projectionCheckpointSha,
    );
    requireCondition(
      dbCheckpoint.runtime_code_hash === identityA.runtime_code_chain_hash &&
        identityA.deployment_id === submissions[0].response.coordinate.deployment_id &&
        canonicalSha256(identityA) === canonicalSha256(identityB),
      "DB checkpoint runtime/endpoint identity diverged",
    );
    const sourceEligibleBeforeProbes = sourceGateOpen;
    const sourceUseCountBeforeProbes = sourceGateTranscript.filter(
      (entry) => entry.kind === "source_open" && entry.source_endpoint === "collator-b",
    ).length;
    sourceGate.open(identityA, identityB, archiveProbes);
    terminationPhase.set("projection-snapshot-fresh-pair");
    const [freshA, freshB] = await orderedParallelOperations([
      () =>
        readSnapshot({
          ...snapshotOptions,
          contract: contracts["fresh-a"],
          database: path.join(projectionsRoot, "fresh-a.sqlite3"),
          endpoint: fixture.endpoints["collator-a"],
          replacement: null,
        }),
      () =>
        readSnapshot({
          ...snapshotOptions,
          contract: contracts["fresh-b"],
          database: path.join(projectionsRoot, "fresh-b.sqlite3"),
          endpoint: fixture.endpoints["collator-b"],
          replacement: null,
        }),
    ]);
    snapshotByLabel["fresh-a"] = freshA;
    snapshotByLabel["fresh-b"] = freshB;
    terminationPhase.set("projection-snapshot-rebuild-a");
    snapshotByLabel["rebuild-a"] = await replacementSnapshot({
      ...snapshotOptions,
      contract: contracts["rebuild-a"],
      database: path.join(projectionsRoot, "rebuild-a.sqlite3"),
      endpoint: fixture.endpoints["collator-a"],
    });
    terminationPhase.set("projection-snapshot-rebuild-b");
    snapshotByLabel["rebuild-b"] = await replacementSnapshot({
      ...snapshotOptions,
      contract: contracts["rebuild-b"],
      database: path.join(projectionsRoot, "rebuild-b.sqlite3"),
      endpoint: fixture.endpoints["collator-b"],
    });
    const sourceEligible = sourceGateOpen;
    const sourceUseCount = sourceGateTranscript.filter(
      (entry) => entry.kind === "source_open" && entry.source_endpoint === "collator-b",
    ).length;
    requireCondition(
      sourceGateEvidence !== null &&
        sourceGateTranscript.length === 8 &&
        sourceGateTranscript.every((entry, index) => entry.sequence === index + 1) &&
        sourceGateTranscript[1].kind === "gate_open" &&
        sourceGateTranscript.slice(2).every((entry) => entry.gate_open) &&
        sourceGateTranscript
          .filter((entry) => entry.kind === "source_open" && entry.source_endpoint === "collator-a")
          .length === 3 &&
        sourceUseCount === 3,
      "fresh/rebuild sources did not follow the exact dual identity/archive gate transcript",
    );
    const snapshots = fixture.semantic_projection.snapshot_contracts.map(
      (contract) => snapshotByLabel[contract.label],
    );
    requireCondition(
      snapshots.every(
        (snapshot) =>
          canonicalSha256(snapshot.sections.checkpoint[0]) === canonicalSha256(dbCheckpoint),
      ),
      "post-gate B snapshots diverged from the DB-backed F checkpoint",
    );

    const orchestratorArgvBytes = await fsp.readFile(procPath(process.pid, "cmdline"));
    const orchestratorArgv = splitNul(orchestratorArgvBytes);
    const orchestratorBefore = await procStat(process.pid);
    const orchestratorProcBefore = await procObjectIdentity(process.pid);
    const orchestratorPrivilegesBefore = await processPrivilegeEvidence(process.pid);
    const orchestratorExecutable = await fsp.realpath(procPath(process.pid, "exe"));
    const orchestratorProcAfter = await procObjectIdentity(process.pid);
    const orchestratorAfter = await procStat(process.pid);
    const networkStat = await fsp.stat(networkRoot, { bigint: true });

    terminationPhase.set("relay-resume");
    requireCondition(await networkNodes["relay-b"].resume(), "NetworkNode.resume failed for relay B");
    relayBPaused = false;
    terminationPhase.set("relay-resume-api-reconnect");
    const relayEndpointA = "ws://127.0.0.1:9944/";
    const relayEndpointB = "ws://127.0.0.1:9945/";
    requireCondition(
      new URL(networkNodes["relay-a"].wsUri).href === relayEndpointA &&
        new URL(networkNodes["relay-b"].wsUri).href === relayEndpointB,
      "resumed relay WebSocket endpoints drifted",
    );
    const relayReconnectResults = await Promise.allSettled([
      connectPolkadotApi(
        ApiPromise,
        WsProvider,
        relayEndpointA,
        "resumed relay A API reconnect",
      ).then((api) => {
        relayApiA = api;
        networkNodes["relay-a"].apiInstance = api;
      }),
      connectPolkadotApi(
        ApiPromise,
        WsProvider,
        relayEndpointB,
        "resumed relay B API reconnect",
      ).then((api) => {
        relayApiB = api;
        networkNodes["relay-b"].apiInstance = api;
      }),
    ]);
    const relayReconnectFailure = relayReconnectResults.find(
      (result) => result.status === "rejected",
    );
    if (relayReconnectFailure) throw relayReconnectFailure.reason;
    requireCondition(relayApiA !== null && relayApiB !== null, "resumed relay APIs are missing");
    const [relayGenesisA, relayGenesisB] = await Promise.all([
      rpcGenesisHash(relayApiA),
      rpcGenesisHash(relayApiB),
    ]);
    requireCondition(
      relayGenesisA === initialNodes["relay-a"].primary_chain_spec.live_genesis_hash &&
        relayGenesisB === initialNodes["relay-b"].primary_chain_spec.live_genesis_hash &&
        relayGenesisA === relayGenesisB,
      "resumed relay APIs do not match the audited relay genesis identity",
    );
    const relayAHeadAfterResume = await rpcFinalized(relayApiA);
    terminationPhase.set("relay-resume-convergence");
    const relayCheckpoint = await waitEndpointsEqual(
      relayApiA,
      relayApiB,
      relayAHeadAfterResume.number,
      "resumed relay B validator catch-up",
    );
    terminationPhase.set("relay-census");
    const relayChainCensus = await finalizedRelayChainCensus(
      relayApiA,
      relayApiB,
      fixture,
      relayCheckpoint,
    );
    const cleanupNodeProcesses = [...terminatedProcesses];
    await Promise.all([relaySideApiA.disconnect(), relaySideApiB.disconnect()]);
    relaySideApiA = null;
    relaySideApiB = null;
    await withTimeout("pre-stop PVF runtime monitor quiescence", 30_000, () =>
      pvfRuntimeMonitor.quiesce(),
    );
    terminationPhase.set("network-stop");
    networkStopAttempted = true;
    await withTimeout("Zombienet stop", 60_000, () => network.stop());
    networkStopped = true;
    terminationPhase.set("post-stop-verification");
    const manualRestartExit = await withTimeout(
      "manual collator B teardown/reap",
      30_000,
      () => manualRestart.closed,
    );
    requireCondition(
      manualRestartExit.code === null && manualRestartExit.signal === "SIGKILL",
      "manual collator B was not reaped through the pinned NativeClient teardown",
    );
    for (const lifetime of terminatedProcesses) {
      await waitFor(
        () => processGenerationLive(lifetime.pid, lifetime.start_time_ticks).then((live) => !live),
        `cleanup process ${lifetime.pid}`,
        30_000,
      );
    }
    const allListeners = fixture.topology.nodes.flatMap((node) => node.listeners);
    for (const address of allListeners)
      await waitFor(() => socketRefused(address), `cleanup listener ${address}`, 30_000);
    const remainingNodeProcesses = await nodeProcessInventory(fixture, repoRoot);
    const releasedSockets = await capturedSocketInventory("post-stop socket inventory");
    requireCondition(
      remainingNodeProcesses.length === 0 && releasedSockets.records.length === 0,
      "node/network teardown left a sealed node process or listener",
    );
    const pvfRuntimeEvidence = await pvfRuntimeMonitor.stopAndEvidence();
    const completePvfEvidence = {
      ...pvfStaticEvidence,
      runtime_monitor: pvfRuntimeEvidence,
    };
    await withTimeout("bounded node log quiescence", 30_000, () => boundedNodeLogs.quiesce());
    boundedNodeLogs.assertHealthy();
    const nodeLogCapture = boundedNodeLogs.evidence();
    boundedNodeLogs.restore();

    const materializerLogAfter = (
      await openedOwnerBootstrapFile(
        materializerLogPath,
        fixture.launcher.bootstrap_log_max_bytes,
        "post-run materializer log",
      )
    ).evidence;
    const genesisExportLogAfter = (
      await openedOwnerBootstrapFile(
        genesisExportLogPath,
        fixture.launcher.bootstrap_log_max_bytes,
        "post-run genesis export log",
      )
    ).evidence;
    const genesisHeadAfter = await openedOwnerBootstrapFile(
      genesisHeadPath,
      4096,
      "post-run parachain genesis-head export",
    );
    const genesisWasmAfter = await openedOwnerBootstrapFile(
      genesisWasmPath,
      runtimeWasmPin.size * 2 + 2,
      "post-run parachain genesis-wasm export",
    );
    requireCondition(
      JSON.stringify(materializerLogAfter) === JSON.stringify(materializerLogBefore) &&
        JSON.stringify(genesisExportLogAfter) === JSON.stringify(genesisExportLogBefore) &&
        JSON.stringify(genesisHeadAfter.evidence) ===
          JSON.stringify(genesisHeadBefore.evidence) &&
        JSON.stringify(genesisWasmAfter.evidence) ===
          JSON.stringify(genesisWasmBefore.evidence) &&
        genesisHeadAfter.bytes.equals(genesisHeadBefore.bytes) &&
        genesisWasmAfter.bytes.equals(genesisWasmBefore.bytes),
      "bootstrap logs or genesis exports changed while the live journey ran",
    );
    const bootstrapEvidence = {
      log_max_bytes: fixture.launcher.bootstrap_log_max_bytes,
      materializer_log: {
        path: materializerLogPath,
        before: materializerLogBefore,
        after: materializerLogAfter,
        bytes_hex: `0x${materializerLogBeforeRead.bytes.toString("hex")}`,
      },
      genesis_export_log: {
        path: genesisExportLogPath,
        before: genesisExportLogBefore,
        after: genesisExportLogAfter,
        bytes_hex: `0x${genesisExportLogBeforeRead.bytes.toString("hex")}`,
      },
      genesis_head: {
        path: genesisHeadPath,
        before: genesisHeadBefore.evidence,
        after: genesisHeadAfter.evidence,
        encoding: "lowercase-0x-hex-no-whitespace-v1",
        decoded_size: genesisHeadDecoded.length,
        decoded_sha256: sha256(genesisHeadDecoded),
        chain_hash: genesisHeadChainHash,
      },
      genesis_wasm: {
        path: genesisWasmPath,
        before: genesisWasmBefore.evidence,
        after: genesisWasmAfter.evidence,
        encoding: "lowercase-0x-hex-no-whitespace-v1",
        decoded_size: genesisWasmDecoded.length,
        decoded_sha256: sha256(genesisWasmDecoded),
      },
      export_commands: bootstrapExportCommands,
      materializer_command: bootstrapMaterializerCommand,
      chain_spec_path: chainSpecPath,
      parachain_binary_path: parachainBinary,
    };

    terminationPhase.set("audit-materialization");
    const laneInventory = await submissionLaneInventory(
      projectionsRoot,
      identityA.deployment_id,
      fixture.audit.dev_signer_accounts,
    );
    const auditRoot = path.join(workRoot, "audit");
    for (const directory of ["action", "config", "fixture", "journal", "log", "socket"])
      await fsp.mkdir(path.join(auditRoot, directory), { recursive: true, mode: 0o700 });
    await writeOwnerFile(
      path.join(auditRoot, "action/finalized-chain-census.json"),
      canonicalBytes(chainCensus),
    );
    await writeOwnerFile(
      path.join(auditRoot, "action/finalized-relay-chain-census.json"),
      canonicalBytes(relayChainCensus),
    );
    await writeOwnerFile(path.join(auditRoot, "action/reads.jsonl"), jsonLines(snapshots));
    await writeOwnerFile(path.join(auditRoot, "action/submissions.jsonl"), jsonLines(submissions));
    const copyRawSpec = async (role) => {
      const spec = initialNodes[role].primary_chain_spec;
      const copiedSource = await copyOwnerFile(
        spec.raw_path,
        path.join(auditRoot, `config/${role}.raw.json`),
        false,
        spec.raw_size,
      );
      requireCondition(
        copiedSource.bytes_read === spec.raw_size && copiedSource.sha256 === spec.raw_sha256,
        `${role} raw spec changed between live identity capture and audit copy`,
      );
    };
    await copyRawSpec("collator-a");
    await copyRawSpec("collator-b");
    await writeOwnerFile(
      path.join(auditRoot, "config/environment-inventory.json"),
      canonicalBytes(Object.fromEntries(Object.entries(initialNodes).map(([role, node]) => [role, node.environment]))),
    );
    await copyOwnerFile(configPath, path.join(auditRoot, "config/materialized-zombienet.toml"));
    const auditProcessInventory = {
      format: "cubikan-process-inventory-v1",
      orchestrator: {
        pid: process.pid,
        privileges: orchestratorPrivilegesBefore,
      },
      initial_node_processes: initialNodeProcesses,
      stopped_node_processes: stoppedNodeProcesses,
      restarted_node_processes: restartedNodeProcesses,
      remaining_node_processes: remainingNodeProcesses,
      pvf_workers: completePvfEvidence,
    };
    await writeOwnerFile(
      path.join(auditRoot, "config/process-inventory.json"),
      canonicalBytes(auditProcessInventory),
    );
    await writeOwnerFile(
      path.join(auditRoot, "config/pvf-worker-inventory.json"),
      canonicalBytes(completePvfEvidence),
    );
    await copyRawSpec("relay-a");
    await copyRawSpec("relay-b");
    await writeOwnerFile(path.join(auditRoot, "fixture/journey-v1.json"), fixtureBytes);
    await copyOwnerFile(
      path.join(repoRoot, fixture.exact_recorded_association.fixture_path),
      path.join(auditRoot, "fixture/recorded-association-v1.json"),
    );
    await writeOwnerFile(path.join(auditRoot, "journal/archive-probes.jsonl"), jsonLines(archiveProbes.transcript));
    await writeOwnerFile(
      path.join(auditRoot, "journal/finality.jsonl"),
      jsonLines(submissions.flatMap((submission) => submission.endpoint_observations)),
    );
    await writeOwnerFile(
      path.join(auditRoot, "journal/submission-lanes.json"),
      canonicalBytes(laneInventory),
    );
    for (const [role, name] of Object.entries(processNames)) {
      const source = network.client.processMap[name].logs;
      await waitForStableFile(source, `${role} node log`);
      const captured = nodeLogCapture.files.find((file) => file.file_path === source);
      const copiedSource = await copyOwnerFile(
        source,
        path.join(auditRoot, `log/${role}.log`),
      );
      requireCondition(
        captured &&
          copiedSource.bytes_read === captured.size &&
          copiedSource.sha256 === captured.sha256 &&
          copiedSource.path_before.device === captured.device &&
          copiedSource.path_before.inode === captured.inode,
        `${role} node log changed between bounded capture and audit copy`,
      );
    }
    await copyOwnerFile(
      genesisExportLogPath,
      path.join(auditRoot, "log/genesis-export.log"),
    );
    await copyOwnerFile(
      materializerLogPath,
      path.join(auditRoot, "log/materializer.log"),
    );
    const orchestratorDiagnostics = diagnosticCapture.evidence(fixture);
    const orchestratorLogEvidence = {
      format: "cubikan-orchestrator-log-v1",
      pid: process.pid,
      node_roles: Object.keys(processNames),
      diagnostics: orchestratorDiagnostics,
      normalizer_rejection_matrix: {
        case_count: normalizerMutations.length,
        transcript_sha256: canonicalSha256(normalizerMutations),
      },
    };
    await writeOwnerFile(
      path.join(auditRoot, "log/orchestrator.log"),
      canonicalBytes(orchestratorLogEvidence),
    );
    const auditListenerInventory = {
      format: "cubikan-listener-inventory-v1",
      initial: {
        command: ["/usr/bin/ss", "-H", "-lntup"],
        stdout_sha256: sha256(sockets.raw),
        stdout_utf8: sockets.raw.toString("utf8"),
        records: sockets.records,
      },
      stopped: stoppedSockets,
      restarted: restartedSockets,
      released: releasedSockets,
      pvf_unix_sockets: {
        observed: pvfRuntimeEvidence.observed_unix_sockets,
        post_stop: pvfRuntimeEvidence.post_stop_unix_sockets,
      },
    };
    await writeOwnerFile(
      path.join(auditRoot, "socket/listeners.txt"),
      canonicalBytes(auditListenerInventory),
    );
    await writeOwnerFile(
      path.join(auditRoot, "socket/pvf-unix-sockets.json"),
      canonicalBytes({
        observed: pvfRuntimeEvidence.observed_unix_sockets,
        post_stop: pvfRuntimeEvidence.post_stop_unix_sockets,
      }),
    );
    const auditFiles = await auditTree(auditRoot, fixture);
    const allNodeEvidence = [...Object.values(initialNodes), restartedNode];
    const secretEnvironmentNamesPresent = [
      ...new Set(
        [
          ...allNodeEvidence.map((node) => node.environment),
          ...pvfRuntimeEvidence.observed_generations.map((worker) => worker.environment),
        ].flatMap((environment) =>
          Object.keys(environment).filter((name) =>
            /(?:CREDENTIAL|HELPER|MNEMONIC|PASSPHRASE|PASSWORD|PRIVATE|PROXY|SECRET|SEED|TOKEN|ASKPASS|AUTH_SOCK)/i.test(name),
          ),
        ),
      ),
    ].sort();
    const publicUrls = [
      ...new Set(auditFiles.artifacts.flatMap((artifact) => artifact.nonloopback_hits)),
    ].sort();
    const externalActions = [
      ...chainCensus.external_actions,
      ...chainCensus.forbidden_events,
      ...relayChainCensus.external_actions,
      ...relayChainCensus.forbidden_events,
    ];
    const zeroCounts = Object.fromEntries(
      fixture.audit.zero_count_keys.map((key) => {
        if (key === "public_rpc") return [key, publicUrls.length + sockets.unexpected.length];
        if (key === "secret") {
          return [
            key,
            secretEnvironmentNamesPresent.length +
              auditFiles.artifacts.reduce(
                (count, artifact) => count + artifact.forbidden_hits.length,
                0,
            ),
          ];
        }
        requireCondition(
          Object.hasOwn(chainCensus.forbidden_counts, key) &&
            Object.hasOwn(relayChainCensus.forbidden_counts, key),
          `chain census pair lacks zero-count category ${key}`,
        );
        return [key, chainCensus.forbidden_counts[key] + relayChainCensus.forbidden_counts[key]];
      }),
    );
    requireCondition(
      Object.values(zeroCounts).every((count) => count === 0) &&
        secretEnvironmentNamesPresent.length === 0 &&
        publicUrls.length === 0 &&
        externalActions.length === 0,
      "derived audit counters found a public, secret-bearing, or external action",
    );

    const localBinaryAfter = await openedFileEvidence(localBinary);
    const materializedNodeAfter = await openedFileEvidence(
      materializedNode,
      fixture.launcher.node_executable_size,
    );
    const procNodeAfter = await openedProcExecutableEvidence(
      process.pid,
      materializedNode,
      fixture.launcher.node_executable_size,
    );
    const toolchainRootIdentityAfter = identityFromStat(
      await fsp.lstat(toolchainRoot, { bigint: true }),
    );
    const toolchainMountAfter = await exactTmpfsBindMountEvidence(
      toolchainRoot,
      "post-run materialized toolchain",
    );
    requireCondition(
      JSON.stringify(localBinaryAfter.evidence) === JSON.stringify(localBinaryBefore.evidence) &&
        JSON.stringify(materializedNodeAfter.evidence) ===
          JSON.stringify(materializedNodeBefore.evidence) &&
        JSON.stringify(procNodeAfter.evidence) === JSON.stringify(procNodeBefore.evidence) &&
        JSON.stringify(toolchainRootIdentityAfter) ===
          JSON.stringify(toolchainRootIdentityBefore) &&
        JSON.stringify(toolchainMountAfter) === JSON.stringify(toolchainMountBefore) &&
        JSON.stringify(await processPrivilegeEvidence(process.pid)) ===
          JSON.stringify(orchestratorPrivilegesBefore) &&
        JSON.stringify(materializedNodeAfter.evidence.path_before) ===
          JSON.stringify(procNodeAfter.evidence.path_before),
      "runtime executable identity or bytes changed across the live journey",
    );

    const stopTranscript = {
      signal: "SIGTERM",
      signal_send_succeeded: stopSignalSent,
      pid_cleared_before_signal: true,
      sent_to_pid: oldB.pid,
      sent_to_start_time_ticks: oldB.process_start_time_ticks_before,
      pre_signal_proc_device: preSignalProc.device,
      pre_signal_proc_inode: preSignalProc.inode,
      wait_observed: true,
      exit_code: null,
      termination_signal: null,
      generation_wait_observed: true,
      proc_probe_errno: stoppedObservation.proc_probe_errno,
      listener_probe_errors: listenerProbeErrors,
      observed_before_first_survivor_mutation: true,
    };
    const evidence = {
      format: fixture.launcher.evidence_format,
      version: 1,
      isolation: {
        loopback_network_namespace: true,
        private_pid_namespace: (await fsp.readlink("/proc/1/ns/pid")) === (await fsp.readlink("/proc/self/ns/pid")),
        fresh_procfs: fs.existsSync("/proc/1/stat"),
        pid_one_reaper_present: (await fsp.readFile("/proc/1/cmdline")).length > 0,
        external_connectivity_denied: await externalConnectivityDenied(),
        synthetic_chain: true,
        dev_only: true,
        supported_root: supportedRoot,
        supported_root_filesystem_magic: "ef53",
      },
      pins,
      runtime_executables: {
        local_binary: {
          path: localBinary,
          before: localBinaryBefore.evidence,
          after: localBinaryAfter.evidence,
        },
        orchestrator_node: {
          toolchain_root: toolchainRoot,
          toolchain_root_identity_before: toolchainRootIdentityBefore,
          toolchain_root_identity_after: toolchainRootIdentityAfter,
          toolchain_filesystem_magic: toolchainFilesystemMagic,
          toolchain_write_probe_errno: toolchainWriteProbeErrno,
          toolchain_mount_before: toolchainMountBefore,
          toolchain_mount_after: toolchainMountAfter,
          path: materializedNode,
          expected_size: fixture.launcher.node_executable_size,
          expected_sha256: fixture.launcher.node_executable_sha256,
          materialized_before: materializedNodeBefore.evidence,
          materialized_after: materializedNodeAfter.evidence,
          proc_exe_before: procNodeBefore.evidence,
          proc_exe_after: procNodeAfter.evidence,
        },
      },
      bootstrap: bootstrapEvidence,
      topology: {
        orchestrator: {
          pid: process.pid,
          executable: orchestratorExecutable,
          argv: orchestratorArgv,
          proc_cmdline_sha256: sha256(orchestratorArgvBytes),
          process_start_time_ticks_before: orchestratorBefore.start_time_ticks,
          process_start_time_ticks_after: orchestratorAfter.start_time_ticks,
          proc_directory_device_before: orchestratorProcBefore.device,
          proc_directory_inode_before: orchestratorProcBefore.inode,
          proc_directory_device_after: orchestratorProcAfter.device,
          proc_directory_inode_after: orchestratorProcAfter.inode,
          data_directory: networkRoot,
          data_directory_mode: (safeNumber(networkStat.mode, "network mode") & 0o7777).toString(8).padStart(4, "0"),
          listener_addresses: [],
          node_child_pids: Object.values(initialNodes).map((node) => node.pid),
          privileges: orchestratorPrivilegesBefore,
        },
        nodes: fixture.topology.nodes.map((node) => initialNodes[node.role]),
        node_processes: initialNodeProcesses,
        ss_records: sockets.records,
        unexpected_node_processes: initialNodeProcesses.filter(
          (record) => !expectedInitialPids.includes(record.pid),
        ).length,
        unexpected_listeners: sockets.unexpected,
        normalizer_mutations: normalizerMutations,
      },
      pvf_workers: {
        ...completePvfEvidence,
      },
      submissions,
      checkpoints: {
        pre_mutation: preMutationReadiness,
        c: {
          name: fixture.checkpoints.catch_up,
          number: checkpointC.number,
          hash: checkpointC.hash,
          projection_coordinate_sha256: canonicalSha256(submissions[6].response.projection.checkpoint),
          stability_intervals: [],
        },
        f: {
          name: fixture.checkpoints.final,
          number: checkpointF.number,
          hash: checkpointF.hash,
          projection_coordinate_sha256: projectionCheckpointSha,
          stability_intervals: finalityStability,
        },
        stopped_role: fixture.checkpoints.stop_role,
        survivor_role: fixture.checkpoints.survivor_role,
        stopped_at_c: true,
        survivor_finalized_mutation_ids: fixture.mutations.slice(7).map((mutation) => mutation.id),
      },
      restart: {
        role: "collator-b",
        endpoint: "collator-b",
        pid_before: oldB.pid,
        pid_after: restartedPid,
        process_start_time_ticks_after: restartedNode.process_start_time_ticks_before,
        data_directory_before: oldB.data_directory,
        data_directory_after: restartedNode.data_directory,
        data_directory_mode_before: oldB.data_directory_mode,
        data_directory_mode_after: restartedNode.data_directory_mode,
        data_directory_device_before: oldBDataStat.device,
        data_directory_inode_before: oldBDataStat.inode,
        data_directory_device_after: newBDataStat.device,
        data_directory_inode_after: newBDataStat.inode,
        config_sha256_before: configShaBefore,
        config_sha256_after: canonicalSha256(restartedNode.argv),
        frozen_command: frozenCommand,
        frozen_command_sha256: frozenCommandSha,
        frozen_log_path: frozenLogPath,
        spawn_executable: manualRestart.spawn_executable,
        spawn_args: manualRestart.spawn_args,
        teardown_exit_code: manualRestartExit.code,
        teardown_signal: manualRestartExit.signal,
        listener_records_after: restartedListenerRecords,
        archive_flags_before: oldB.archive_flags,
        archive_flags_after: restartedNode.archive_flags,
        synchronized_number: checkpointF.number,
        synchronized_hash: checkpointF.hash,
        source_eligible_before_probes: sourceEligibleBeforeProbes,
        source_use_count_before_probes: sourceUseCountBeforeProbes,
        source_eligible_after_probes: sourceEligible,
        source_use_count_after_probes: sourceUseCount,
        source_gate: {
          ...sourceGateEvidence,
          transcript_sha256: canonicalSha256(sourceGateTranscript),
        },
        identity_a: identityA,
        identity_b: identityB,
        restarted_node: restartedNode,
        stop: { transcript: stopTranscript, transcript_sha256: canonicalSha256(stopTranscript) },
        archive_probes: archiveProbes,
      },
      snapshots,
      audit: {
        loopback_only: true,
        synthetic_only: true,
        dev_only: true,
        dev_signers: fixture.audit.dev_signers,
        synthetic_origins: fixture.audit.synthetic_origins,
        allowed_hosts: fixture.audit.allowed_hosts,
        socket_addresses: allListeners,
        ...auditFiles,
        submission_lanes: laneInventory,
        chain_census: chainCensus,
        relay_chain_census: relayChainCensus,
        node_log_capture: nodeLogCapture,
        orchestrator_log: orchestratorLogEvidence,
        process_inventory: auditProcessInventory,
        listener_inventory: auditListenerInventory,
        zero_counts: zeroCounts,
        secret_environment_names_present: secretEnvironmentNamesPresent,
        public_urls: publicUrls,
        external_actions: externalActions,
      },
      cleanup: {
        node_network_cleanup_complete: true,
        orchestrator_exit_observation: "pending-parent-after-publication",
        terminated_node_processes: cleanupNodeProcesses,
        listener_addresses_released: allListeners,
        work_root_removed: true,
        remaining_node_processes: remainingNodeProcesses.map((record) => ({
          pid: record.pid,
          start_time_ticks: record.start_time_ticks,
        })),
        remaining_listeners: releasedSockets.records.map((record) => record.local_address),
      },
    };
    requireCondition(evidence.isolation.external_connectivity_denied, "external connectivity was available");
    terminationPhase.set("evidence-publication");
    await fsp.rm(workRoot, { recursive: true, force: false });
    workRemoved = true;
    requireCondition(!fs.existsSync(workRoot), "work root removal was not observed");
    await atomicPublish(args.evidence, evidence);
    terminationPhase.set("complete");
  } finally {
    if (relayBPaused && network?.nodesByName?.bob) {
      try {
        await withTimeout("failure cleanup relay B resume", 10_000, () =>
          network.nodesByName.bob.resume(),
        );
      } catch {}
    }
    for (const api of [relaySideApiA, relaySideApiB]) {
      if (api) {
        try {
          await withTimeout("failure cleanup relay-side disconnect", 10_000, () =>
            api.disconnect(),
          );
        } catch {}
      }
    }
    if (network && !networkStopped && !networkStopAttempted) {
      networkStopAttempted = true;
      try {
        await withTimeout("failure cleanup Zombienet stop", 30_000, () => network.stop());
      } catch {}
    }
    if (!workRemoved && fs.existsSync(workRoot)) {
      try {
        await withTimeout("failure cleanup node log quiescence", 10_000, () =>
          boundedNodeLogs.quiesce(),
        );
      } catch {
        boundedNodeLogs.allowFailureCleanupUnlink();
      }
      try {
        await fsp.rm(workRoot, { recursive: true, force: true });
      } catch {}
    }
    if (pvfRuntimeMonitor) {
      try {
        await withTimeout("failure cleanup PVF monitor cancel", 10_000, () =>
          pvfRuntimeMonitor.cancel(),
        );
      } catch {}
    }
    boundedNodeLogs.restore();
    diagnosticCapture.restore();
    terminationPhase.remove();
  }
}

await main();
