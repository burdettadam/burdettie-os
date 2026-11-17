#!/usr/bin/env python3
"""Check a retained thread-state report and its separately observed exit code.

This is a consistency check, not authenticated execution or guest qualification.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import sys

from m1_evidence import EvidenceError, exact_keys, load_json, read_regular, require

MAX_REPORT = 4096
EXPECTED = {
    "schema_version": 1,
    "workers_requested": 6,
    "workers_started": 6,
    "workers_joined": 6,
    "rounds": 192,
    "tls_checks": 280,
    "errno_checks": 256,
    "errno_address_checks": 384,
    "once_calls": 1,
    "destructors": 6,
    "join_checks": 6,
    "pthread_errors": 0,
    "failures": 0,
}


def validate(raw: bytes, exit_code: int, platform: str, architecture: str) -> dict:
    """Reject incomplete, forged-shape, contradictory or wrong-target reports."""
    require(type(exit_code) is int and exit_code == 0, "observed execution did not exit zero")
    require(platform in ("darwin", "linux"), "unsupported expected platform")
    require(architecture in ("arm64", "x86_64"), "unsupported expected architecture")
    report = load_json(raw, MAX_REPORT)
    exact_keys(report, set(EXPECTED) | {"scope", "platform", "architecture", "status"}, "thread-state report")
    require(report["scope"] == "m1_pthread_state_execution", "wrong thread-state scope")
    require(report["platform"] == platform and report["architecture"] == architecture,
            "thread-state platform or architecture mismatch")
    require(report["status"] == "pass", "thread-state operation did not pass")
    for name, expected in EXPECTED.items():
        require(type(report[name]) is int and report[name] == expected,
                "thread-state counter mismatch: " + name)
    return report


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--exit-code", type=int, required=True)
    parser.add_argument("--platform", choices=("linux", "darwin"), required=True)
    parser.add_argument("--architecture", choices=("x86_64", "arm64"), required=True)
    args = parser.parse_args(argv)
    try:
        validate(read_regular(args.report, MAX_REPORT), args.exit_code, args.platform, args.architecture)
    except (EvidenceError, OSError, ValueError, TypeError) as error:
        print("Thread-state report rejected: " + str(error), file=sys.stderr)
        return 1
    print("Thread-state report and observed exit are consistent; no selected-guest qualification.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
