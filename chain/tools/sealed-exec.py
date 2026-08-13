"""Copy one reviewed ELF into a sealed memfd and execute only those bytes."""

from __future__ import annotations

import errno
import fcntl
import hashlib
import os
import stat
import sys


CHUNK_SIZE = 1024 * 1024
REQUIRED_SEALS = (
    fcntl.F_SEAL_WRITE
    | fcntl.F_SEAL_GROW
    | fcntl.F_SEAL_SHRINK
    | fcntl.F_SEAL_SEAL
)


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"sealed-exec: {message}")


def parse_nonnegative_decimal(value: str, label: str) -> int:
    if not value or not value.isascii() or not value.isdecimal():
        fail(f"{label} is not canonical decimal")
    parsed = int(value, 10)
    if str(parsed) != value:
        fail(f"{label} is not canonical decimal")
    return parsed


def parse_sha256(value: str) -> str:
    if len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
        fail("expected SHA-256 is not lowercase hexadecimal")
    return value


def source_identity(metadata: os.stat_result) -> tuple[int, int, int, int, int]:
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_size,
        metadata.st_mtime_ns,
        metadata.st_ctime_ns,
    )


def open_source(mode: str, subject: str) -> tuple[int, str]:
    if mode in {"exec-path", "probe-path"}:
        if not subject.startswith("/") or "\x00" in subject:
            fail("source path is not absolute and NUL-free")
        before = os.lstat(subject)
        if not stat.S_ISREG(before.st_mode):
            fail("source path is not a regular file")
        descriptor = os.open(subject, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
        opened = os.fstat(descriptor)
        if source_identity(before) != source_identity(opened):
            os.close(descriptor)
            fail("source path changed while opening")
        return descriptor, subject

    if mode == "exec-fd":
        descriptor_number = parse_nonnegative_decimal(subject, "source descriptor")
        if descriptor_number <= 2:
            fail("source descriptor must be greater than standard I/O")
        descriptor = os.dup(descriptor_number)
        os.set_inheritable(descriptor, False)
        return descriptor, f"fd:{descriptor_number}"

    fail("unsupported mode")


def materialize_sealed(
    source_descriptor: int,
    expected_size: int,
    expected_sha256: str,
) -> int:
    before = os.fstat(source_descriptor)
    if not stat.S_ISREG(before.st_mode):
        fail("opened source is not a regular file")
    if before.st_size != expected_size:
        fail("opened source size mismatch")
    os.lseek(source_descriptor, 0, os.SEEK_SET)

    sealed_descriptor = os.memfd_create("cubikan-sealed-exec-v1", os.MFD_ALLOW_SEALING)
    digest = hashlib.sha256()
    copied = 0
    while True:
        chunk = os.read(source_descriptor, CHUNK_SIZE)
        if not chunk:
            break
        digest.update(chunk)
        copied += len(chunk)
        view = memoryview(chunk)
        while view:
            written = os.write(sealed_descriptor, view)
            if written <= 0:
                fail("short write while materializing memfd")
            view = view[written:]

    after = os.fstat(source_descriptor)
    if source_identity(before) != source_identity(after):
        fail("source changed while materializing memfd")
    if copied != expected_size or digest.hexdigest() != expected_sha256:
        fail("materialized bytes differ from the reviewed identity")
    if copied < 4:
        fail("reviewed executable is shorter than an ELF header")

    os.lseek(sealed_descriptor, 0, os.SEEK_SET)
    if os.read(sealed_descriptor, 4) != b"\x7fELF":
        fail("reviewed executable is not ELF")
    os.fchmod(sealed_descriptor, 0o500)
    fcntl.fcntl(sealed_descriptor, fcntl.F_ADD_SEALS, REQUIRED_SEALS)
    actual_seals = fcntl.fcntl(sealed_descriptor, fcntl.F_GET_SEALS)
    if actual_seals != REQUIRED_SEALS:
        fail("memfd does not carry the complete required seal set")

    try:
        os.pwrite(sealed_descriptor, b"\x00", 0)
    except OSError as error:
        if error.errno != errno.EPERM:
            fail(f"post-seal write failed with unexpected errno {error.errno}")
    else:
        fail("post-seal write unexpectedly succeeded")

    os.lseek(sealed_descriptor, 0, os.SEEK_SET)
    sealed_digest = hashlib.sha256()
    sealed_size = 0
    while True:
        chunk = os.read(sealed_descriptor, CHUNK_SIZE)
        if not chunk:
            break
        sealed_digest.update(chunk)
        sealed_size += len(chunk)
    if sealed_size != expected_size or sealed_digest.hexdigest() != expected_sha256:
        fail("sealed memfd identity changed after sealing")
    os.lseek(sealed_descriptor, 0, os.SEEK_SET)
    return sealed_descriptor


def main(arguments: list[str]) -> None:
    if len(arguments) < 4:
        fail("usage: MODE SUBJECT SIZE SHA256 [-- ARG0 ARG ...]")
    mode, subject, size_text, digest_text, *remaining = arguments
    expected_size = parse_nonnegative_decimal(size_text, "expected size")
    expected_sha256 = parse_sha256(digest_text)
    source_descriptor, _source_label = open_source(mode, subject)
    try:
        sealed_descriptor = materialize_sealed(
            source_descriptor,
            expected_size,
            expected_sha256,
        )
    finally:
        os.close(source_descriptor)

    if mode == "probe-path":
        if remaining:
            fail("probe mode accepts no execution arguments")
        print("sealed-exec: seal-set=write,grow,shrink,seal post-seal-write=denied")
        os.close(sealed_descriptor)
        return

    if not remaining or remaining[0] != "--" or len(remaining) < 2:
        fail("execution mode requires -- followed by argv[0]")
    execution_argv = remaining[1:]
    os.execve(sealed_descriptor, execution_argv, dict(os.environ))


if __name__ == "__main__":
    main(sys.argv[1:])
