#!/usr/bin/env python3
"""Read-only preflight for the M1 PureDarwin spike; does not build or boot."""

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path

PUREDARWIN_REVISION = "0163b2db225b85e3bf96fc25646ea290105ac298"
MAX_COMMAND_OUTPUT_BYTES = 1024 * 1024


def command_output(argv):
    try:
        result = subprocess.run(argv, capture_output=True, text=True, errors="replace",
                                timeout=15, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {"available": False, "error": type(exc).__name__}
    if len(result.stdout.encode("utf-8")) > MAX_COMMAND_OUTPUT_BYTES:
        return {"available": False, "exit_code": result.returncode,
                "error": "output_limit_exceeded"}
    # Validation must see every line. In particular, a mismatched submodule may
    # appear after a long list of otherwise matching entries.
    return {"available": result.returncode == 0, "exit_code": result.returncode,
            "stdout": result.stdout}


def collect_report(source=None):
    blockers = []
    system, machine = platform.system(), platform.machine()
    if system != "Linux" or machine != "x86_64":
        blockers.append("The selected reproducibility host is x86_64 Linux.")
    versions = {}
    for name in ("git", "nix", "qemu-system-x86_64", "qemu-img", "cargo", "rustc", "clang"):
        found = shutil.which(name)
        versions[name] = command_output([found, "--version"]) if found else {"available": False}
        if "stdout" in versions[name]:
            # Version text is presentation only, never repository validation.
            versions[name]["stdout"] = versions[name]["stdout"].strip()[:512]
    for required in ("git", "nix"):
        if not versions[required]["available"]:
            blockers.append(f"Required build tool is missing or unusable: {required}.")
    checkout = {"supplied": source is not None}
    if source is None:
        blockers.append("No pinned PureDarwin checkout supplied.")
    elif not source.is_dir():
        blockers.append("PureDarwin source path is not an existing directory.")
    else:
        git = ["git", "--no-optional-locks", "-c", "core.fsmonitor=false", "-C", str(source)]
        root = command_output(git + ["rev-parse", "--show-toplevel"])
        checkout["exact_repository_root"] = root["available"] and bool(root.get("stdout", "").strip()) and (
            Path(root["stdout"].strip()).resolve() == source.resolve()
        )
        if not checkout["exact_repository_root"]:
            blockers.append("PureDarwin source path must be the exact repository root.")
        head = command_output(git + ["rev-parse", "HEAD"])
        checkout["revision"] = head.get("stdout", "").strip() if head["available"] else None
        if checkout["revision"] != PUREDARWIN_REVISION:
            blockers.append("PureDarwin checkout does not match the selected revision.")
        clean = command_output(git + ["status", "--porcelain", "--untracked-files=all",
                                      "--ignore-submodules=none"])
        checkout["clean"] = clean["available"] and not clean.get("stdout")
        if not checkout["clean"]:
            blockers.append("PureDarwin source is modified or status could not be read.")
        submodules = command_output(git + ["submodule", "status", "--recursive"])
        checkout["submodules_match"] = submodules["available"] and not any(
            line.startswith(("-", "+", "U")) for line in submodules.get("stdout", "").splitlines()
        )
        if not checkout["submodules_match"]:
            blockers.append("PureDarwin submodules are unavailable or do not match the checkout.")
        lock = source / "flake.lock"
        if lock.is_file():
            checkout["flake_lock_sha256"] = hashlib.sha256(lock.read_bytes()).hexdigest()
        else:
            blockers.append("PureDarwin flake.lock is missing.")
    return {
        "schema_version": 1,
        "target": "puredarwin-x86_64-q35-tcg",
        "candidate_revision": PUREDARWIN_REVISION,
        "host": {"os": system, "architecture": machine},
        "tools": versions,
        "kvm_available": os.path.exists("/dev/kvm") and os.access("/dev/kvm", os.R_OK | os.W_OK),
        "checkout": checkout,
        "status": "blocked" if blockers else "ready_for_spike",
        "blockers": blockers,
        "build_executed": False,
        "boot_executed": False,
        "notes": ["QEMU and OVMF are supplied by the pinned Nix build; PATH probes are informational.",
                  "KVM is optional; the selected baseline uses QEMU TCG.",
                  "Ready means preflight only; image build, runtime ABI, persistence and hardware remain unproven."]
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="Existing PureDarwin checkout; never downloaded")
    args = parser.parse_args()
    report = collect_report(args.source)
    print(json.dumps(report, indent=2))
    return 2 if report["status"] == "blocked" else 0


if __name__ == "__main__":
    raise SystemExit(main())
