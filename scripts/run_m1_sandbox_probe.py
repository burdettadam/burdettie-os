#!/usr/bin/env python3
"""Bound and validate a source-owned native sandbox probe; never qualify a backend.

The C fixture can also run directly in a guest without Python. `inspect` checks
its captured stdout and separately observed exit status; image/build provenance
must accompany that capture. Synthetic parser tests are not enforcement evidence.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import subprocess
import sys
import time

LOG_LIMIT = 65536
BINARY_LIMIT = 4 * 1024 * 1024
NONCE = re.compile(r"[A-Za-z0-9_-]{1,64}\Z")
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
CASES = (
    ("allow_read", "read"), ("allow_write", "write"), ("allow_create", "write"),
    ("deny_read_open", "read"), ("deny_write_open", "write"),
    ("deny_create_open", "write"), ("preopened_read", "pread"),
    ("preopened_write", "pwrite"),
)
ROLES = ("worker", "descendant")
STATUSES = {
    "blocked_setup", "baseline_incomplete", "baseline_failed", "probe_incomplete",
    "blocked_policy", "interrupted", "passed_limited_probe", "coverage_gap",
}


class InvalidReport(ValueError):
    """A capture cannot support even the limited observation it reports."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InvalidReport(message)


def keys(value: object, expected: set[str], context: str) -> None:
    require(type(value) is dict and set(value) == expected, f"{context}: wrong fields")


def integer(value: object, low: int, high: int, context: str) -> int:
    require(type(value) is int and low <= value <= high, f"{context}: invalid integer")
    return value


def unique_pairs(pairs: list[tuple[str, object]]) -> dict:
    value = {}
    for key, item in pairs:
        require(key not in value, f"duplicate JSON field: {key}")
        value[key] = item
    return value


def allowed(case: dict, index: int, seed_bytes: int) -> bool:
    return (case["open_rc"] == 0 and case["open_errno"] == 0
            and case["io_rc"] == (seed_bytes if index in (0, 3, 6) else 1)
            and case["io_errno"] == 0 and case["content_ok"] == 1
            and case["sync_errno"] == 0 and case["close_errno"] == 0)


def denied(case: dict, index: int) -> bool:
    # Darwin EPERM is 1. A failed open says nothing about a read/write syscall
    # that was not attempted; retained descriptors must deny the actual I/O.
    common = (case["content_ok"] == 0 and case["sync_errno"] == 0
              and case["close_errno"] == 0)
    if index < 6:
        return (common and case["open_rc"] == -1 and case["open_errno"] == 1
                and case["io_rc"] == -2 and case["io_errno"] == 0)
    return (common and case["open_rc"] == 0 and case["open_errno"] == 0
            and case["io_rc"] == -1 and case["io_errno"] == 1)


def complete(phase: dict) -> bool:
    return (phase["received"] == 1 and phase["worker_exit"] == 0
            and phase["worker_signal"] == 0 and phase["setup_errno"] == 0
            and len(phase["roles"]) == 2 and phase["descendant_pid"] > 0
            and phase["descendant_exit"] == 0)


def validate_phase(phase: dict, parent: int, name: str) -> None:
    keys(phase, {"received", "worker_pid", "worker_exit", "worker_signal", "native_api",
                 "profile_rc", "profile_errno", "setup_errno", "descendant_pid",
                 "descendant_exit", "roles"}, name)
    for field in ("received", "native_api"):
        integer(phase[field], 0, 1, f"{name}.{field}")
    for field in ("worker_pid", "descendant_pid"):
        integer(phase[field], 0, 2**31 - 1, f"{name}.{field}")
        require(phase[field] != parent, f"{name}.{field}: parent reused as child")
    for field in ("worker_exit", "descendant_exit"):
        integer(phase[field], -2, 255, f"{name}.{field}")
    integer(phase["worker_signal"], 0, 127, f"{name}.worker_signal")
    integer(phase["profile_rc"], -2, 0, f"{name}.profile_rc")
    for field in ("profile_errno", "setup_errno"):
        integer(phase[field], 0, 4095, f"{name}.{field}")
    require(type(phase["roles"]) is list and len(phase["roles"]) <= 2, f"{name}: invalid roles")
    if phase["received"]:
        require(phase["worker_pid"] > 0, f"{name}: missing actual worker PID")
    if phase["descendant_pid"]:
        require(phase["worker_pid"] > 0 and phase["descendant_pid"] != phase["worker_pid"],
                f"{name}: invalid descendant PID")
    if phase["roles"]:
        require(phase["received"] == 1, f"{name}: roles without received child report")
    for role_index, role in enumerate(phase["roles"]):
        keys(role, {"role", "cases"}, name)
        require(role["role"] == ROLES[role_index], f"{name}: wrong or duplicate role")
        require(type(role["cases"]) is list and len(role["cases"]) == len(CASES),
                f"{name}: incomplete case list")
        for index, case in enumerate(role["cases"]):
            keys(case, {"case", "operation", "open_rc", "open_errno", "io_rc", "io_errno",
                        "content_ok", "sync_errno", "close_errno"}, name)
            require((case["case"], case["operation"]) == CASES[index], f"{name}: wrong case/operation")
            integer(case["open_rc"], -2, 0, "open_rc")
            integer(case["io_rc"], -2, 128, "io_rc")
            integer(case["content_ok"], 0, 1, "content_ok")
            for field in ("open_errno", "io_errno", "sync_errno", "close_errno"):
                integer(case[field], 0, 4095, field)


def validate_report(raw: bytes, nonce: str, actual_exit: int, expected_pid: int | None = None) -> dict:
    require(type(raw) is bytes and 0 < len(raw) <= LOG_LIMIT, "empty or oversized stdout")
    require(type(nonce) is str and NONCE.fullmatch(nonce) is not None, "invalid nonce")
    integer(actual_exit, -127, 255, "actual exit")
    try:
        report = json.loads(raw.decode("utf-8", errors="strict"), object_pairs_hook=unique_pairs)
    except (UnicodeError, ValueError) as error:
        raise InvalidReport(f"invalid JSON capture: {error}") from error
    keys(report, {"schema_version", "scope", "nonce", "status", "exit_code", "parent_pid",
                  "process_group", "seed_bytes", "denial_errno", "baseline_pass",
                  "forbidden_files_unchanged", "setup_errno", "interrupted_signal",
                  "qualified_capabilities", "baseline", "restricted"}, "report")
    require(type(report["schema_version"]) is int and report["schema_version"] == 1, "wrong schema")
    require(report["scope"] == "native_puredarwin_file_probe", "wrong scope")
    require(report["nonce"] == nonce, "nonce mismatch")
    require(type(report["status"]) is str and report["status"] in STATUSES, "unknown status")
    integer(report["exit_code"], 0, 2, "exit_code")
    require(report["exit_code"] == actual_exit, "observed exit does not match report")
    parent = integer(report["parent_pid"], 1, 2**31 - 1, "parent_pid")
    require(type(report["process_group"]) is int and report["process_group"] == parent,
            "probe is not its own process group")
    if expected_pid is not None:
        require(type(expected_pid) is int and parent == expected_pid, "actual parent PID mismatch")
    seed_bytes = len(f"Burdettie sandbox probe nonce={nonce}\n".encode("ascii"))
    require(type(report["seed_bytes"]) is int and report["seed_bytes"] == seed_bytes, "wrong sentinel length")
    require(type(report["denial_errno"]) is int and report["denial_errno"] == 1, "wrong denial errno")
    for field in ("baseline_pass", "forbidden_files_unchanged"):
        integer(report[field], 0, 1, field)
    integer(report["setup_errno"], 0, 4095, "setup_errno")
    integer(report["interrupted_signal"], 0, 127, "interrupted_signal")
    require(report["qualified_capabilities"] == [], "a probe cannot qualify backend capabilities")
    baseline, restricted = report["baseline"], report["restricted"]
    validate_phase(baseline, parent, "baseline")
    validate_phase(restricted, parent, "restricted")
    baseline_ok = complete(baseline) and all(
        allowed(case, index, seed_bytes) for role in baseline["roles"]
        for index, case in enumerate(role["cases"]))
    require(report["baseline_pass"] == int(baseline_ok), "baseline summary disagrees with operations")
    if baseline["roles"]:
        require(baseline["native_api"] == 0 and baseline["profile_rc"] == -2
                and baseline["profile_errno"] == 0, "baseline applied a policy")
    failures = []
    observations = {}
    for role in restricted["roles"]:
        outcomes = {case["case"]: (allowed(case, i, seed_bytes) if i < 3 else denied(case, i))
                    for i, case in enumerate(role["cases"])}
        observations[role["role"]] = outcomes
        failures.extend(f"{role['role']}.{name}" for name, passed in outcomes.items() if not passed)
    if not report["forbidden_files_unchanged"]:
        failures.append("forbidden_file_content_or_absence")
    restricted_complete = complete(restricted)
    limited_pass = restricted_complete and not failures
    status = report["status"]
    if status in ("passed_limited_probe", "coverage_gap"):
        require(baseline_ok and restricted_complete, "complete baseline and descendant evidence required")
        require(restricted["native_api"] == 1 and restricted["profile_rc"] == 0
                and restricted["profile_errno"] == 0, "native profile was not successfully installed")
        require(report["setup_errno"] == 0 and report["interrupted_signal"] == 0, "interrupted or failed setup")
        require((status == "passed_limited_probe") == limited_pass, "claimed result disagrees with actual operations")
        require(actual_exit == (0 if limited_pass else 1), "wrong limited-result exit")
    else:
        require(actual_exit == 2, "blocked/incomplete execution must exit nonzero")
        if status == "blocked_policy":
            require(baseline_ok and restricted["received"] == 1 and restricted["worker_exit"] == 2
                    and restricted["worker_signal"] == 0 and restricted["setup_errno"] == 0
                    and restricted["profile_rc"] == -1 and restricted["profile_errno"] > 0
                    and not restricted["roles"] and restricted["descendant_pid"] == 0
                    and restricted["descendant_exit"] == -2, "policy block lacks child/exit binding")
        elif status == "baseline_failed":
            require(complete(baseline) and not baseline_ok and not restricted["received"], "wrong baseline failure")
        elif status == "baseline_incomplete":
            require(not complete(baseline) and not restricted["received"], "wrong incomplete baseline")
        elif status == "probe_incomplete":
            require(baseline_ok and not restricted_complete, "wrong incomplete probe")
        elif status == "blocked_setup":
            require(report["setup_errno"] > 0 and not baseline["received"] and not restricted["received"],
                    "setup block lacks setup failure")
        elif status == "interrupted":
            require(report["interrupted_signal"] > 0, "missing interruption")
    return {"report": report, "assessment": {
        "selected_guest_qualified": False, "qualified_capabilities": [],
        "baseline_operations_observed": sum(len(role["cases"]) for role in baseline["roles"]),
        "restricted_operations_observed": sum(len(role["cases"]) for role in restricted["roles"]),
        "required_operation_outcomes": observations,
        "failed_required_observations": failures if restricted["roles"] else [],
        "scope": "captured_direct_file_operations_and_fork_inheritance_only",
    }}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def child_limits() -> None:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_FSIZE, (LOG_LIMIT, LOG_LIMIT))


def stop_owned_group(process: subprocess.Popen) -> bool:
    # poll() returning None leaves this child unreaped, so its PID cannot be
    # recycled before killpg. Once reaped, never signal that PGID again. This
    # return value observes ONLY the leader's exit, not descendant termination.
    for signum, seconds in ((signal.SIGTERM, 2), (signal.SIGKILL, 2)):
        if process.poll() is not None:
            return True
        try:
            os.killpg(process.pid, signum)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=seconds)
            return True
        except subprocess.TimeoutExpired:
            pass
    return False


def run_probe(executable: Path, expected_sha256: str, output: Path, nonce: str,
              timeout: float = 30) -> dict:
    if not output.is_absolute() or not executable.is_absolute():
        raise ValueError("executable and new output directory must be absolute")
    if not SHA256.fullmatch(expected_sha256) or not NONCE.fullmatch(nonce):
        raise ValueError("expected executable SHA256 and ASCII nonce are required")
    if not 0.05 <= timeout <= 30:
        raise ValueError("timeout must be between 0.05 and 30 seconds")
    output.mkdir(mode=0o700)  # Never merge with or clean an existing experiment.
    started = time.monotonic()
    receipt = {"schema_version": 1, "scope": "native_sandbox_probe_capture",
               "status": "blocked_executable", "exit_code": 2,
               "selected_guest_qualified": False, "qualified_capabilities": [],
               "nonce": nonce, "expected_executable_sha256": expected_sha256,
               "original_executable": str(executable), "timeout_seconds": timeout,
               "log_limit_bytes": LOG_LIMIT, "artifacts_retained": True,
               "descendant_termination": "unverified"}
    snapshot, stdout, stderr = (output / name for name in ("probe", "stdout.log", "stderr.log"))
    process = None
    try:
        source_fd = os.open(executable, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(source_fd, "rb") as source, snapshot.open("xb") as destination:
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise ValueError("executable input is not a regular file")
            data = source.read(BINARY_LIMIT + 1)
            if len(data) > BINARY_LIMIT:
                raise ValueError("executable exceeds the 4 MiB fixture bound")
            destination.write(data)
            destination.flush()
            os.fsync(destination.fileno())
        observed = sha256_file(snapshot)
        receipt["snapshot_sha256"] = observed
        if observed != expected_sha256:
            raise ValueError("executable snapshot SHA256 mismatch")
        snapshot.chmod(0o500)
        argv = [str(snapshot), str(output / "fixtures"), nonce]
        receipt["argv"] = argv
        receipt["status"] = "launch_failed"
        with stdout.open("xb") as out, stderr.open("xb") as err:
            process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                       env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
                                       start_new_session=True, preexec_fn=child_limits)
            receipt["parent_pid"] = receipt["process_group"] = process.pid
            deadline = time.monotonic() + timeout
            stop_reason = None
            while process.poll() is None:
                if stdout.stat().st_size >= LOG_LIMIT or stderr.stat().st_size >= LOG_LIMIT:
                    stop_reason = "output_limit"
                    break
                if time.monotonic() >= deadline:
                    stop_reason = "timed_out"
                    break
                time.sleep(0.01)
            if stop_reason:
                receipt["leader_exit_observed"] = stop_owned_group(process)
                receipt["status"] = stop_reason
            elif stdout.stat().st_size >= LOG_LIMIT or stderr.stat().st_size >= LOG_LIMIT:
                receipt["status"] = "output_limit"
            else:
                receipt["status"] = "invalid_report"
            receipt["actual_exit"] = process.poll()
        if receipt["status"] == "invalid_report":
            validation = validate_report(stdout.read_bytes(), nonce, receipt["actual_exit"], process.pid)
            receipt["validation"] = validation
            receipt["status"] = validation["report"]["status"]
            receipt["exit_code"] = validation["report"]["exit_code"]
    except (OSError, ValueError, InvalidReport) as error:
        receipt["error"] = str(error)
        if process is not None and process.poll() is None:
            receipt["leader_exit_observed"] = stop_owned_group(process)
            receipt["actual_exit"] = process.poll()
    finally:
        if process is not None:
            receipt["leader_exit_observed"] = process.poll() is not None
        receipt["elapsed_seconds"] = round(time.monotonic() - started, 6)
        for label, path in (("stdout", stdout), ("stderr", stderr)):
            if path.exists():
                receipt[label + "_bytes"] = path.stat().st_size
                receipt[label + "_sha256"] = sha256_file(path)
        with (output / "result.json").open("x", encoding="utf-8") as result:
            json.dump(receipt, result, indent=2, sort_keys=True)
            result.write("\n")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("run", help="run one exact executable in a new retained directory")
    run.add_argument("--executable", required=True, type=Path)
    run.add_argument("--sha256", required=True)
    run.add_argument("--output", required=True, type=Path)
    run.add_argument("--nonce", required=True)
    run.add_argument("--timeout", default=30, type=float)
    inspect = commands.add_parser("inspect", help="validate guest stdout and a separately observed guest exit")
    inspect.add_argument("--stdout", required=True, type=Path)
    inspect.add_argument("--exit-code", required=True, type=int)
    inspect.add_argument("--nonce", required=True)
    inspect.add_argument("--parent-pid", type=int)
    args = parser.parse_args()
    try:
        if args.command == "run":
            receipt = run_probe(args.executable, args.sha256, args.output, args.nonce, args.timeout)
            code = receipt["exit_code"]
        else:
            with args.stdout.open("rb") as stream:
                raw = stream.read(LOG_LIMIT + 1)
            receipt = validate_report(raw, args.nonce, args.exit_code, args.parent_pid)
            code = receipt["report"]["exit_code"]
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return code
    except (OSError, ValueError) as error:
        print(f"blocked: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
