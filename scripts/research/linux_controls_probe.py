#!/usr/bin/env python3
"""Report narrow Linux mechanism observations; never certify a launch backend.

Uses source-owned files/memory and disposable child processes. It does not mount
filesystems, change host policy, start services, contact a network, or execute
generated machine code. Exit 77 from a probe means unavailable/blocked.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import socket
import subprocess
import tempfile


def run(argv: list[str]) -> dict:
    result = subprocess.run(argv, capture_output=True, text=True, timeout=15)
    return {
        "argv": argv,
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
    }


def unix_socket_probe(kind: int, path: Path) -> dict:
    """Use only a private source-owned Unix socket and one fixed byte."""
    stage = "socket"
    try:
        with socket.socket(socket.AF_UNIX, kind) as server:
            server.settimeout(2)
            stage = "bind"
            server.bind(str(path))
            if kind == socket.SOCK_STREAM:
                stage = "listen"
                server.listen(1)
            with socket.socket(socket.AF_UNIX, kind) as client:
                client.settimeout(2)
                stage = "connect"
                client.connect(str(path))
                stage = "roundtrip"
                client.send(b"p")
                if kind == socket.SOCK_STREAM:
                    with server.accept()[0] as connection:
                        received = connection.recv(1)
                else:
                    received = server.recv(1)
                return {"type": kind, "stage": stage, "result": received == b"p"}
    except OSError as error:
        return {"type": kind, "stage": stage, "result": False,
                "errno": error.errno, "error": str(error)}


def socketpair_probe() -> dict:
    try:
        left, right = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with left, right:
            left.settimeout(2)
            right.settimeout(2)
            left.send(b"p")
            return {"type": "socketpair", "result": right.recv(1) == b"p"}
    except OSError as error:
        return {"type": "socketpair", "result": False,
                "errno": error.errno, "error": str(error)}


def main() -> None:
    source = Path(__file__).with_suffix(".c")
    status = Path("/proc/self/status").read_text()
    report = {
        "scope": "mechanism_probe_not_backend_qualification",
        "kernel": platform.release(),
        "architecture": platform.machine(),
        "uid": os.getuid(),
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "parent_status": {
            key: value.strip()
            for key, value in (line.split(":", 1) for line in status.splitlines())
            if key in {"NoNewPrivs", "Seccomp", "Seccomp_filters", "CapEff", "CapBnd"}
        },
        "cgroup_controllers_visible": Path("/sys/fs/cgroup/cgroup.controllers").exists(),
        "tools": {name: shutil.which(name) for name in ["cc", "bwrap", "systemctl", "unshare"]},
        "commands": [],
    }
    with tempfile.TemporaryDirectory(prefix="burdettie-linux-controls-") as name:
        scratch = Path(name)
        binary = scratch / "probe"
        compile_result = run([
            "cc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
            str(source), "-o", str(binary),
        ])
        report["commands"].append(compile_result)
        if compile_result["returncode"]:
            print(json.dumps(report, indent=2))
            raise SystemExit(1)
        allowed = scratch / "allowed"
        denied = scratch / "denied"
        allowed.write_text("allowed fixture\n")
        denied.write_text("denied fixture\n")
        report["unix_socket_probes"] = [
            unix_socket_probe(socket.SOCK_STREAM, scratch / "stream"),
            unix_socket_probe(socket.SOCK_DGRAM, scratch / "datagram"),
            socketpair_probe(),
        ]
        for args in [["landlock", str(allowed), str(denied)], ["seccomp"],
                     ["memory"], ["executable-memory"]]:
            report["commands"].append(run([str(binary), *args]))
        if shutil.which("unshare"):
            for extra in [[], ["--mount", "--propagation", "unchanged"],
                          ["--uts"], ["--ipc"], ["--net"], ["--pid", "--fork"]]:
                report["commands"].append(run([
                    "unshare", "--user", "--map-root-user", *extra, "/bin/true",
                ]))
        if shutil.which("bwrap"):
            report["commands"].append(run(["bwrap", "--version"]))
        if shutil.which("systemctl"):
            report["commands"].append(run(["systemctl", "--user", "is-system-running"]))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
