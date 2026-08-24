#!/usr/bin/bash -p
set -euo pipefail

readonly TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
readonly PROJECT_ROOT="$(cd -- "$TOOL_DIR/../.." && pwd -P)"
readonly DOWNLOADS="$PROJECT_ROOT/chain/.cache/downloads"
readonly NPM_CACHE="$PROJECT_ROOT/chain/.cache/npm"

readonly ZOMBIENET_REVISION=a7c434271f094320d17cf94f7a2f95fdef417379
readonly ZOMBIENET_ARCHIVE="$DOWNLOADS/zombienet-$ZOMBIENET_REVISION.tar.gz"
readonly ZOMBIENET_ARCHIVE_SIZE=13578426
readonly ZOMBIENET_ARCHIVE_SHA256=cf46203b6c32c219d8d546d30a9069944b800d3daf4f19787dca72e5300b21e2
readonly ZOMBIENET_ROOT="zombienet-$ZOMBIENET_REVISION"
readonly PACKAGE_LOCK_RELATIVE=javascript/package-lock.json
readonly PACKAGE_LOCK_SHA256=7d6f8da6cd967dc84706d6c657259db22b965702e280a855d8620d19fa43afbe
readonly ZOMBIENET_CLI_VERSION=1.3.138
readonly TOML_GIT_COMMIT=5e17114f1af5b5b70e4f2ec10cd007623c928988
readonly TOML_INSTALLED_TREE_SHA256=2faea9de33ef0b6a95e7823a17c8beddb514f10873b8988fa65755ecff9114c7
readonly NPM_CACHE_SNAPSHOT_MAX_ENTRIES=10000
readonly NPM_CACHE_SNAPSHOT_MAX_BYTES=268435456
readonly NPM_CACHE_SNAPSHOT_MAX_DEPTH=32
readonly NPM_CACHE_SNAPSHOT_MAX_PATH_BYTES=512

readonly NODE_ARCHIVE="$DOWNLOADS/node-v22.23.1-linux-x64.tar.xz"
readonly NODE_ARCHIVE_SIZE=31068444
readonly NODE_ARCHIVE_SHA256=9749e988f437343b7fa832c69ded82a312e41a03116d766797ac14f6f9eee578
readonly NODE_ROOT=node-v22.23.1-linux-x64
readonly NODE_VERSION=v22.23.1
readonly NPM_VERSION=10.9.8
readonly NODE_SYMLINKS=$'bin/corepack\t../lib/node_modules/corepack/dist/corepack.js\nbin/npm\t../lib/node_modules/npm/bin/npm-cli.js\nbin/npx\t../lib/node_modules/npm/bin/npx-cli.js'

readonly SHA256SUM=/usr/lib/cargo/bin/coreutils/sha256sum
readonly STAT=/usr/lib/cargo/bin/coreutils/stat
readonly ENV=/usr/lib/cargo/bin/coreutils/env
readonly PYTHON=/usr/bin/python3.14
readonly GIT=/usr/bin/git
readonly GIT_SHA256=5516c9f362c29376ab9a499a33082f9f611941d8c75930c880e30ad109e39c9a

die() {
    printf 'materialize-zombienet: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' \
        'usage: materialize-zombienet.sh --output-dir ABSOLUTE_EMPTY_PRIVATE_DIR' \
        'prints: <output>/zombienet/javascript/packages/cli/dist/cli.js' \
        'pinned Node: <output>/node/bin/node' >&2
    exit 2
}

sha256_file() {
    local digest
    digest="$($SHA256SUM -- "$1")" || return
    printf '%s\n' "${digest%% *}"
}

tree_sha256() {
    local root=$1 manifest path relative digest result
    [[ -d "$root" && ! -L "$root" ]] || die "tree root is missing or symbolic: $root"
    [[ -z "$(/usr/bin/find "$root" -type l -print -quit)" ]] ||
        die "tree contains a symbolic link: $root"
    [[ -z "$(/usr/bin/find "$root" ! -type d ! -type f -print -quit)" ]] ||
        die "tree contains a nonregular entry: $root"
    manifest="$(/usr/lib/cargo/bin/coreutils/mktemp "$BUILD_HOME/.tree-manifest.XXXXXX")"
    while IFS= read -r -d '' path; do
        relative="${path#"$root"/}"
        [[ "$relative" != "$path" && "$relative" != /* &&
            "$relative" != *'/../'* && "$relative" != '../'* &&
            "$relative" != *$'\n'* ]] || die 'unsafe tree path'
        printf '%s' "$relative" | /usr/bin/iconv -f UTF-8 -t UTF-8 >/dev/null 2>&1 ||
            die 'non-UTF-8 tree path'
        digest="$(sha256_file "$path")"
        printf '%s  %s\n' "$digest" "$relative" >>"$manifest"
    done < <(/usr/bin/find "$root" -type f -print0 | /usr/lib/cargo/bin/coreutils/sort -z)
    result="$(sha256_file "$manifest")"
    /usr/bin/gnurm -f -- "$manifest"
    printf '%s\n' "$result"
}

verify_npm_cache_tree() {
    local cache=$1 label=$2
    [[ "$cache" == /* && "$cache" != / && -d "$cache" && ! -L "$cache" &&
        "$($STAT -Lc '%F' -- "$cache")" == directory ]] ||
        die "$label root is missing, symbolic, or nonabsolute"
    [[ -z "$(/usr/bin/find "$cache" -type l -print -quit)" ]] ||
        die "$label contains a symbolic link"
    [[ -z "$(/usr/bin/find "$cache" ! -type d ! -type f -print -quit)" ]] ||
        die "$label contains a nonregular entry"
    [[ -d "$cache/_cacache/content-v2" && -d "$cache/_cacache/index-v5" &&
        -n "$(/usr/bin/find "$cache/_cacache/content-v2" -type f -print -quit)" ]] ||
        die "$label lacks its content-addressed closure"
}

snapshot_npm_cache() {
    local source=$1 destination=$2
    if ! "$PYTHON" -I -S - "$source" "$destination" \
        "$NPM_CACHE_SNAPSHOT_MAX_ENTRIES" "$NPM_CACHE_SNAPSHOT_MAX_BYTES" \
        "$NPM_CACHE_SNAPSHOT_MAX_DEPTH" "$NPM_CACHE_SNAPSHOT_MAX_PATH_BYTES" <<'PY'
import hashlib
import os
import pathlib
import stat
import sys

(
    source_path,
    destination_path,
    max_entries_text,
    max_bytes_text,
    max_depth_text,
    max_path_bytes_text,
) = sys.argv[1:]
max_entries = int(max_entries_text)
max_bytes = int(max_bytes_text)
max_depth = int(max_depth_text)
max_path_bytes = int(max_path_bytes_text)
limits = {"entries": 0, "bytes": 0}
identity_fields = (
    "st_dev", "st_ino", "st_mode", "st_nlink", "st_uid", "st_gid",
    "st_size", "st_mtime_ns", "st_ctime_ns",
)


def identity(value):
    return tuple(getattr(value, field) for field in identity_fields)


def names(directory_fd):
    with os.scandir(directory_fd) as entries:
        result = sorted(entry.name for entry in entries)
    for name in result:
        if (
            not name
            or name in (".", "..")
            or "/" in name
            or "\x00" in name
            or name.encode("utf-8", "strict").decode("utf-8", "strict") != name
        ):
            raise RuntimeError("unsafe cache entry name")
    return result


def digest_fd(descriptor):
    os.lseek(descriptor, 0, os.SEEK_SET)
    digest = hashlib.sha256()
    while chunk := os.read(descriptor, 1024 * 1024):
        digest.update(chunk)
    os.lseek(descriptor, 0, os.SEEK_SET)
    return digest.digest()


def copy_file(source_directory_fd, destination_directory_fd, name, initial):
    if initial.st_size < 0 or limits["bytes"] + initial.st_size > max_bytes:
        raise RuntimeError("cache snapshot byte bound exceeded")
    source_fd = os.open(
        name,
        os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK,
        dir_fd=source_directory_fd,
    )
    destination_fd = -1
    try:
        opened = os.fstat(source_fd)
        if identity(opened) != identity(initial) or not stat.S_ISREG(opened.st_mode):
            raise RuntimeError("cache file identity drift before copy")
        destination_fd = os.open(
            name,
            os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW,
            0o600,
            dir_fd=destination_directory_fd,
        )
        source_digest = hashlib.sha256()
        total = 0
        while chunk := os.read(source_fd, 1024 * 1024):
            source_digest.update(chunk)
            total += len(chunk)
            view = memoryview(chunk)
            while view:
                written = os.write(destination_fd, view)
                if written <= 0:
                    raise RuntimeError("short cache snapshot write")
                view = view[written:]
        os.fsync(destination_fd)
        destination = os.fstat(destination_fd)
        if (
            not stat.S_ISREG(destination.st_mode)
            or stat.S_IMODE(destination.st_mode) != 0o600
            or total != opened.st_size
            or destination.st_size != total
            or digest_fd(destination_fd) != source_digest.digest()
            or digest_fd(source_fd) != source_digest.digest()
            or identity(os.fstat(source_fd)) != identity(opened)
            or identity(
                os.stat(name, dir_fd=source_directory_fd, follow_symlinks=False)
            )
            != identity(opened)
        ):
            raise RuntimeError("cache file identity drift after copy")
        limits["bytes"] += total
    finally:
        if destination_fd >= 0:
            os.close(destination_fd)
        os.close(source_fd)


def copy_directory(source_fd, destination_fd, depth=0, prefix=""):
    initial = os.fstat(source_fd)
    if not stat.S_ISDIR(initial.st_mode):
        raise RuntimeError("cache directory is nonregular")
    initial_names = names(source_fd)
    for name in initial_names:
        relative = name if not prefix else f"{prefix}/{name}"
        entry_depth = depth + 1
        limits["entries"] += 1
        if limits["entries"] > max_entries:
            raise RuntimeError("cache snapshot entry bound exceeded")
        if entry_depth > max_depth:
            raise RuntimeError("cache snapshot depth bound exceeded")
        if len(relative.encode("utf-8", "strict")) > max_path_bytes:
            raise RuntimeError("cache snapshot path-length bound exceeded")
        entry = os.stat(name, dir_fd=source_fd, follow_symlinks=False)
        if stat.S_ISDIR(entry.st_mode):
            child_source_fd = os.open(
                name,
                os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW,
                dir_fd=source_fd,
            )
            child_destination_fd = -1
            try:
                if identity(os.fstat(child_source_fd)) != identity(entry):
                    raise RuntimeError("cache directory identity drift before copy")
                os.mkdir(name, 0o700, dir_fd=destination_fd)
                child_destination_fd = os.open(
                    name,
                    os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW,
                    dir_fd=destination_fd,
                )
                copy_directory(child_source_fd, child_destination_fd, entry_depth, relative)
                if (
                    identity(os.fstat(child_source_fd)) != identity(entry)
                    or identity(os.stat(name, dir_fd=source_fd, follow_symlinks=False))
                    != identity(entry)
                ):
                    raise RuntimeError("cache directory identity drift after copy")
            finally:
                if child_destination_fd >= 0:
                    os.close(child_destination_fd)
                os.close(child_source_fd)
        elif stat.S_ISREG(entry.st_mode):
            copy_file(source_fd, destination_fd, name, entry)
        else:
            raise RuntimeError("cache snapshot source contains a symbolic or special entry")
    if names(source_fd) != initial_names or identity(os.fstat(source_fd)) != identity(initial):
        raise RuntimeError("cache directory inventory drift after copy")


source_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW
source_fd = os.open(source_path, source_flags)
destination_parent = pathlib.Path(destination_path).parent
destination_name = pathlib.Path(destination_path).name
parent_fd = os.open(destination_parent, source_flags)
destination_fd = -1
try:
    opened_root = os.fstat(source_fd)
    path_root = os.stat(source_path, follow_symlinks=False)
    if identity(opened_root) != identity(path_root) or not stat.S_ISDIR(opened_root.st_mode):
        raise RuntimeError("cache snapshot root identity mismatch")
    if not destination_name or destination_name in (".", ".."):
        raise RuntimeError("unsafe cache snapshot destination")
    os.mkdir(destination_name, 0o700, dir_fd=parent_fd)
    destination_fd = os.open(destination_name, source_flags, dir_fd=parent_fd)
    copy_directory(source_fd, destination_fd)
    if (
        identity(os.fstat(source_fd)) != identity(opened_root)
        or identity(os.stat(source_path, follow_symlinks=False)) != identity(opened_root)
        or not stat.S_ISDIR(os.fstat(destination_fd).st_mode)
        or stat.S_IMODE(os.fstat(destination_fd).st_mode) != 0o700
    ):
        raise RuntimeError("cache snapshot root identity drift")
finally:
    if destination_fd >= 0:
        os.close(destination_fd)
    os.close(parent_fd)
    os.close(source_fd)
PY
    then
        die 'same-descriptor npm cache snapshot failed'
    fi
}

verify_toml_lock() {
    "$PYTHON" -I -S - "$PACKAGE_LOCK" "$TOML_GIT_COMMIT" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
commit = sys.argv[2]
lock = json.loads(path.read_bytes())
entry = lock.get("packages", {}).get("node_modules/toml")
expected = {
    "version": "3.0.0",
    "resolved": f"git+ssh://git@github.com/pepoviola/toml-node.git#{commit}",
    "license": "MIT",
}
git_entries = sorted(
    (name, value.get("resolved"))
    for name, value in lock.get("packages", {}).items()
    if isinstance(value, dict)
    and isinstance(value.get("resolved"), str)
    and value["resolved"].startswith(("git+", "git://", "ssh://"))
)
if (
    entry != expected
    or "integrity" in entry
    or git_entries != [("node_modules/toml", expected["resolved"])]
):
    raise SystemExit("package-lock toml Git dependency identity mismatch")
PY
}

verify_toml_install() {
    local installed="$JAVASCRIPT/node_modules/toml" actual
    actual="$(tree_sha256 "$installed")"
    [[ "$actual" == "$TOML_INSTALLED_TREE_SHA256" ]] ||
        die 'installed toml Git dependency tree hash mismatch'
}

verify_regular_archive() {
    local path=$1 expected_size=$2 expected_sha256=$3 label=$4
    [[ -f "$path" && ! -L "$path" ]] || die "$label archive is missing or symbolic"
    [[ "$($STAT -Lc '%s' -- "$path")" == "$expected_size" ]] ||
        die "$label archive size mismatch"
    [[ "$(sha256_file "$path")" == "$expected_sha256" ]] ||
        die "$label archive hash mismatch"
}

read -r -d '' ARCHIVE_CHECKER <<'PY' || true
import os
import pathlib
import hashlib
import stat
import sys
import tarfile

(
    archive,
    destination,
    expected_root,
    links_text,
    expected_size_text,
    expected_sha256,
) = sys.argv[1:]
expected_size = int(expected_size_text)
expected_links = {}
if links_text:
    for line in links_text.splitlines():
        relative, target = line.split("\t", 1)
        expected_links[relative] = target

destination_path = pathlib.Path(destination)
if not destination_path.is_dir() or any(destination_path.iterdir()):
    raise SystemExit("extraction destination is not an empty directory")

def identity(info):
    return (
        info.st_dev,
        info.st_ino,
        info.st_mode,
        info.st_size,
        info.st_mtime_ns,
        info.st_ctime_ns,
    )


def hash_open_file(subject):
    subject.seek(0)
    digest = hashlib.sha256()
    while chunk := subject.read(1024 * 1024):
        digest.update(chunk)
    subject.seek(0)
    return digest.hexdigest()


flags = os.O_RDONLY | os.O_CLOEXEC
if hasattr(os, "O_NOFOLLOW"):
    flags |= os.O_NOFOLLOW
descriptor = os.open(archive, flags)
with os.fdopen(descriptor, "rb", closefd=True) as archive_file:
    initial = os.fstat(archive_file.fileno())
    if not stat.S_ISREG(initial.st_mode) or initial.st_size != expected_size:
        raise SystemExit("opened archive identity or size mismatch")
    if hash_open_file(archive_file) != expected_sha256:
        raise SystemExit("opened archive hash mismatch")

    with tarfile.open(fileobj=archive_file, mode="r:*") as source:
        members = source.getmembers()
        if not members:
            raise SystemExit("archive is empty")
        seen = set()
        actual_links = {}
        root_directory_seen = False
        for member in members:
            name = member.name
            pure = pathlib.PurePosixPath(name)
            if (not name or "\x00" in name or pure.is_absolute() or
                    any(part in ("", ".", "..") for part in pure.parts) or
                    pure.parts[0] != expected_root):
                raise SystemExit("archive member escapes the exact root")
            if name in seen:
                raise SystemExit("archive contains a duplicate member")
            seen.add(name)
            if tuple(pure.parts) == (expected_root,) and member.isdir():
                root_directory_seen = True
            if member.isdir() or member.isreg():
                continue
            if member.issym():
                relative = pathlib.PurePosixPath(*pure.parts[1:]).as_posix()
                actual_links[relative] = member.linkname
                target = pathlib.PurePosixPath(member.linkname)
                if target.is_absolute():
                    raise SystemExit("archive symlink target is absolute")
                resolved = pathlib.PurePosixPath(*pure.parts[:-1], *target.parts)
                stack = []
                for part in resolved.parts:
                    if part in ("", "."):
                        continue
                    if part == "..":
                        if not stack:
                            raise SystemExit("archive symlink target escapes")
                        stack.pop()
                    else:
                        stack.append(part)
                if not stack or stack[0] != expected_root:
                    raise SystemExit("archive symlink target escapes the exact root")
                continue
            raise SystemExit("archive contains a hardlink or special member")
        if not root_directory_seen:
            raise SystemExit("archive exact root directory is missing")
        if actual_links != expected_links:
            raise SystemExit("archive symlink inventory mismatch")
        source.extractall(destination_path, members=members, filter="data")

    if identity(os.fstat(archive_file.fileno())) != identity(initial):
        raise SystemExit("opened archive changed during extraction")
    if hash_open_file(archive_file) != expected_sha256:
        raise SystemExit("opened archive bytes changed during extraction")

extracted_root = destination_path / expected_root
if not extracted_root.is_dir() or extracted_root.is_symlink():
    raise SystemExit("extracted exact root is invalid")
for relative, expected_target in expected_links.items():
    link = extracted_root / relative
    if not link.is_symlink() or os.readlink(link) != expected_target:
        raise SystemExit("extracted symlink inventory mismatch")
PY
readonly ARCHIVE_CHECKER

extract_checked_archive() {
    local archive=$1 destination=$2 expected_root=$3 expected_links=$4
    local expected_size=$5 expected_sha256=$6
    "$PYTHON" -I -S -c "$ARCHIVE_CHECKER" \
        "$archive" "$destination" "$expected_root" "$expected_links" \
        "$expected_size" "$expected_sha256" ||
        die "safe extraction rejected ${archive##*/}"
}

[[ $# -eq 2 && "$1" == --output-dir ]] || usage
output_dir=$2
[[ "$output_dir" == /* && "$output_dir" != / && "$output_dir" != */../* &&
    "$output_dir" != */./* && "$output_dir" != */.. && "$output_dir" != */. ]] ||
    die 'output directory must be a normalized absolute path'
[[ -d "$output_dir" && ! -L "$output_dir" ]] ||
    die 'output directory must already exist as a real directory'
canonical_output="$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$output_dir")" ||
    die 'cannot canonicalize output directory'
[[ "$canonical_output" == "$output_dir" ]] || die 'output directory is not canonical'
readonly OUTPUT_DIR=$canonical_output
unset output_dir canonical_output

umask 077
[[ "$($STAT -Lc '%u:%a:%F' -- "$OUTPUT_DIR")" == "$EUID:700:directory" ]] ||
    die 'output directory must be owned by this user with mode 0700'
[[ -z "$(/usr/bin/find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -print -quit)" ]] ||
    die 'output directory must be empty'
[[ -d "$NPM_CACHE" && ! -L "$NPM_CACHE" ]] || die 'local npm cache is unavailable'
verify_npm_cache_tree "$NPM_CACHE" 'local npm cache'

verify_regular_archive "$ZOMBIENET_ARCHIVE" "$ZOMBIENET_ARCHIVE_SIZE" \
    "$ZOMBIENET_ARCHIVE_SHA256" Zombienet
verify_regular_archive "$NODE_ARCHIVE" "$NODE_ARCHIVE_SIZE" \
    "$NODE_ARCHIVE_SHA256" Node

zombienet_stage="$OUTPUT_DIR/.extract-zombienet"
node_stage="$OUTPUT_DIR/.extract-node"
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$zombienet_stage" "$node_stage"
extract_checked_archive "$ZOMBIENET_ARCHIVE" "$zombienet_stage" "$ZOMBIENET_ROOT" '' \
    "$ZOMBIENET_ARCHIVE_SIZE" "$ZOMBIENET_ARCHIVE_SHA256"
extract_checked_archive "$NODE_ARCHIVE" "$node_stage" "$NODE_ROOT" "$NODE_SYMLINKS" \
    "$NODE_ARCHIVE_SIZE" "$NODE_ARCHIVE_SHA256"
/usr/bin/gnumv -- "$zombienet_stage/$ZOMBIENET_ROOT" "$OUTPUT_DIR/zombienet"
/usr/bin/gnumv -- "$node_stage/$NODE_ROOT" "$OUTPUT_DIR/node"
/usr/lib/cargo/bin/coreutils/rmdir -- "$zombienet_stage" "$node_stage"

readonly ZOMBIENET="$OUTPUT_DIR/zombienet"
readonly NODE_HOME="$OUTPUT_DIR/node"
readonly NODE="$NODE_HOME/bin/node"
readonly NPM="$NODE_HOME/bin/npm"
readonly NPM_CLI="$NODE_HOME/lib/node_modules/npm/bin/npm-cli.js"
readonly JAVASCRIPT="$ZOMBIENET/javascript"
readonly PACKAGE_LOCK="$ZOMBIENET/$PACKAGE_LOCK_RELATIVE"
readonly CLI="$JAVASCRIPT/packages/cli/dist/cli.js"

[[ -x "$NODE" && -f "$NODE" && ! -L "$NODE" ]] || die 'pinned Node executable is invalid'
[[ -L "$NPM" && "$(/usr/lib/cargo/bin/coreutils/readlink -- "$NPM")" == '../lib/node_modules/npm/bin/npm-cli.js' ]] ||
    die 'pinned npm symlink is invalid'
[[ -f "$NPM_CLI" && ! -L "$NPM_CLI" ]] || die 'pinned npm CLI is missing or symbolic'
[[ -f "$GIT" && -x "$GIT" && ! -L "$GIT" &&
    "$(/usr/lib/cargo/bin/coreutils/realpath -e -- "$GIT")" == "$GIT" &&
    "$(sha256_file "$GIT")" == "$GIT_SHA256" ]] ||
    die 'pinned Git executable identity mismatch'
[[ "$($NODE --version)" == "$NODE_VERSION" ]] || die 'pinned Node version mismatch'
node_link_count="$(/usr/bin/find "$NODE_HOME" -type l -print | /usr/lib/cargo/bin/coreutils/wc -l)"
[[ "$node_link_count" == 3 ]] || die 'extracted Node symlink count mismatch'
unset node_link_count
[[ -f "$PACKAGE_LOCK" && ! -L "$PACKAGE_LOCK" &&
    "$(sha256_file "$PACKAGE_LOCK")" == "$PACKAGE_LOCK_SHA256" ]] ||
    die 'Zombienet package lock hash mismatch'
verify_toml_lock

readonly BUILD_HOME="$OUTPUT_DIR/.home"
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$BUILD_HOME"
readonly BUILD_LOGS="$BUILD_HOME/logs"
/usr/lib/cargo/bin/coreutils/mkdir -m 0700 -- "$BUILD_LOGS"
readonly PRIVATE_NPM_CACHE="$BUILD_HOME/npm-cache"
readonly BUILD_PATH="$NODE_HOME/bin:/usr/bin:/bin"
readonly USER_NPMRC="$BUILD_HOME/user.npmrc"
readonly GLOBAL_NPMRC="$BUILD_HOME/global.npmrc"
: >"$USER_NPMRC"
: >"$GLOBAL_NPMRC"
[[ -f "$USER_NPMRC" && ! -L "$USER_NPMRC" && ! -s "$USER_NPMRC" &&
    -f "$GLOBAL_NPMRC" && ! -L "$GLOBAL_NPMRC" && ! -s "$GLOBAL_NPMRC" &&
    "$($STAT -Lc '%d:%i' -- "$USER_NPMRC")" != \
    "$($STAT -Lc '%d:%i' -- "$GLOBAL_NPMRC")" ]] ||
    die 'npm configuration inputs are not distinct empty regular files'
snapshot_npm_cache "$NPM_CACHE" "$PRIVATE_NPM_CACHE"
verify_npm_cache_tree "$PRIVATE_NPM_CACHE" 'private npm cache snapshot'

run_npm() {
    "$ENV" -i HOME="$BUILD_HOME" PATH="$BUILD_PATH" LC_ALL=C LANG=C TZ=UTC \
        npm_config_cache="$PRIVATE_NPM_CACHE" npm_config_offline=true npm_config_audit=false \
        npm_config_fund=false npm_config_update_notifier=false \
        npm_config_loglevel=error \
        npm_config_logs_dir="$BUILD_LOGS" npm_config_logs_max=0 \
        npm_config_userconfig="$USER_NPMRC" npm_config_globalconfig="$GLOBAL_NPMRC" \
        npm_config_git="$GIT" \
        GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
        GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=/nonexistent SSH_ASKPASS=/nonexistent \
        GIT_SSH=/nonexistent GIT_SSH_COMMAND=/nonexistent GIT_PROXY_COMMAND=/nonexistent \
        GIT_EXEC_PATH=/nonexistent GIT_ALLOW_PROTOCOL= GIT_PROTOCOL_FROM_USER=0 \
        GIT_NO_REPLACE_OBJECTS=1 GIT_NO_LAZY_FETCH=1 GIT_OPTIONAL_LOCKS=0 \
        "$NODE" "$NPM_CLI" "$@"
}

run_node() {
    "$ENV" -i HOME="$BUILD_HOME" PATH="$BUILD_PATH" LC_ALL=C LANG=C TZ=UTC \
        "$NODE" "$@"
}

[[ "$(run_npm --version)" == "$NPM_VERSION" ]] || die 'pinned npm version mismatch'
(
    cd -- "$JAVASCRIPT"
    run_npm cache verify --offline --no-audit --no-fund >&2
    verify_npm_cache_tree "$PRIVATE_NPM_CACHE" 'private npm cache after verification'
    run_npm ci --offline --ignore-scripts --no-audit --no-fund >&2
    verify_npm_cache_tree "$PRIVATE_NPM_CACHE" 'private npm cache after install'
    verify_toml_install
    run_node -e '
const fs = require("node:fs");
for (const directory of process.argv.slice(1)) {
  fs.rmSync(directory, { recursive: true, force: true });
}
' \
        "$JAVASCRIPT/packages/utils/dist" \
        "$JAVASCRIPT/packages/orchestrator/dist" \
        "$JAVASCRIPT/packages/cli/dist"
    for workspace in utils orchestrator cli; do
        run_node "$JAVASCRIPT/node_modules/typescript/bin/tsc" \
            --project "$JAVASCRIPT/packages/$workspace/tsconfig.json" >&2
    done
    run_node -e '
const fs = require("node:fs");
fs.cpSync(process.argv[1], process.argv[2], {
  recursive: true,
  force: false,
  errorOnExist: true,
});
' \
        "$JAVASCRIPT/packages/orchestrator/src/providers/podman/resources/configs" \
        "$JAVASCRIPT/packages/orchestrator/dist/providers/podman/resources/configs"
    verify_toml_install
    verify_npm_cache_tree "$PRIVATE_NPM_CACHE" 'private npm cache after build'
)

[[ -f "$CLI" && ! -L "$CLI" ]] || die 'built Zombienet CLI is missing or symbolic'
[[ "$(sha256_file "$PACKAGE_LOCK")" == "$PACKAGE_LOCK_SHA256" ]] ||
    die 'Zombienet package lock changed during materialization'
cli_version="$($ENV -i HOME="$BUILD_HOME" PATH="$BUILD_PATH" LC_ALL=C LANG=C TZ=UTC \
    "$NODE" "$CLI" version)" || die 'built Zombienet CLI version command failed'
[[ "$cli_version" == "$ZOMBIENET_CLI_VERSION" ]] || die 'built Zombienet CLI version mismatch'

printf '%s\n' "$CLI"
