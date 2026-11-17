"""Small, bounded evidence primitives for the local M1 handoff tools.

These checks detect mistakes/drift in an already trusted checkout. They do not
protect against hostile code running as the same user or authenticate receipts.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import stat


class EvidenceError(ValueError):
    """Malformed, changed, or incompletely retained evidence."""


def require(condition: object, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hash_value(value: object) -> str:
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value),
            "expected a lowercase SHA-256 digest")
    return value


def unique_object(pairs: list) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(raw: bytes, limit: int) -> dict:
    require(0 < len(raw) <= limit, "JSON input exceeds bounds or is empty")
    def invalid_constant(value):
        raise EvidenceError("non-finite JSON number: " + value)
    result = json.loads(raw.decode("utf-8", "strict"), object_pairs_hook=unique_object,
                        parse_constant=invalid_constant)
    require(isinstance(result, dict), "JSON evidence must be an object")
    return result


def exact_keys(value: object, keys: tuple | set, label: str) -> None:
    require(isinstance(value, dict) and set(value) == set(keys),
            "unexpected fields in " + label)


def read_regular(path: Path, limit: int) -> bytes:
    path = Path(path)
    before_path = path.lstat()
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    fd = os.open(path, flags)
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= limit,
                "input must be a nonempty bounded regular file")
        raw = stream.read(before.st_size + 1)
        after = os.fstat(stream.fileno())
    after_path = path.lstat()
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns", "st_mode")
    identity = lambda st: tuple(getattr(st, name) for name in fields)
    require(len(raw) == before.st_size and
            identity(before_path) == identity(before) == identity(after) == identity(after_path),
            "input changed or was replaced during read")
    return raw


def publish(path: Path, payload: bytes) -> None:
    """Publish only after a complete write, fsync and close, without overwrite.

    The .partial file remains deliberately. Linking is the commit point; no
    fallible cleanup runs afterwards. We do not claim that a post-link directory
    fsync succeeded or guarantee survival of power loss at that commit point.
    """
    path = Path(path).absolute()
    require(path.parent.is_dir() and path.parent.resolve(strict=True) == path.parent,
            "output parent must already exist and be canonical")
    require(not path.is_relative_to("/nix/store"), "evidence must remain outside the Nix store")
    require(not os.path.lexists(path), "output must be a new path")
    partial = path.with_name(path.name + ".partial")
    fd = os.open(partial, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as stream:
        count = stream.write(payload)
        require(count == len(payload), "short evidence write")
        stream.flush()
        os.fsync(stream.fileno())
    # Persist the partial directory entry before atomic exclusive publication.
    dirfd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(dirfd)
    finally:
        os.close(dirfd)
    os.link(partial, path, follow_symlinks=False)


def encoded(value: dict) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, allow_nan=False) + "\n").encode("utf-8")
