#!/usr/bin/env python3
"""Run selected unmodified Darling launcher tests; this is not a guest run.

Requires a checkout of the pinned upstream source and a native C compiler.
Builds and test filesystem writes stay in a temporary directory. The JSON
report distinguishes skipped checks and failures from a successful test exit.
"""

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import tempfile


REVISION = "90af44da21d314b9acd24de1d1272e4aba8b9cc6"
TESTS = {
    "ids": ["launcher-ids.c"],
    "env": ["launcher-env.c"],
    "prefix": ["launcher-prefix.c"],
    "mode": ["launcher-mode.c", "launcher-server.c"],
    "rootless-ns": ["launcher-server.c"],
    "server": ["launcher-server.c"],
    "prefix-state": [],
    "profile-prefix": [],
}


def run(argv, cwd, env, seconds=20):
    process = subprocess.Popen(
        argv, cwd=cwd, env=env, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, start_new_session=True,
    )
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
    return {
        "argv": argv, "exit_code": process.returncode,
        "timed_out": timed_out, "stdout": stdout, "stderr": stderr,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--cc", default="cc")
    args = parser.parse_args()
    source = args.source.resolve()
    actual = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=source, text=True,
    ).strip()
    if actual != REVISION:
        parser.error(f"expected upstream {REVISION}, found {actual}")
    subprocess.run(
        ["git", "diff", "--exit-code", "HEAD", "--", "src/startup", "tests/launcher", "src/shellspawn/shellspawn.h"],
        cwd=source, check=True, stdout=subprocess.DEVNULL,
    )
    compiler = shutil.which(args.cc)
    if compiler is None:
        parser.error(f"compiler not found: {args.cc}")
    version = subprocess.check_output([compiler, "--version"], text=True).splitlines()[0]
    report = {
        "scope": "unmodified upstream host launcher tests; no Darling guest or confinement qualification",
        "upstream_revision": actual, "kernel": platform.release(),
        "machine": platform.machine(), "compiler": version, "tests": [],
    }
    with tempfile.TemporaryDirectory(prefix="burdettie-darling-host-") as work:
        env = os.environ.copy()
        env["TMPDIR"] = work
        for name, implementation in TESTS.items():
            output = str(Path(work) / name)
            command = [
                compiler, "-std=gnu11", "-Wall", "-Wextra",
                *([] if name in {"prefix-state", "profile-prefix"} else ["-D_GNU_SOURCE"]),
                "-Isrc/startup", f"tests/launcher/{name}.c",
                *[f"src/startup/{p}" for p in implementation], "-o", output,
            ]
            build = run(command, str(source), env)
            execution = None
            status = "build_failed"
            if build["exit_code"] == 0 and not build["timed_out"]:
                execution = run([output], work, env)
                text = execution["stdout"] + execution["stderr"]
                if execution["timed_out"]:
                    status = "timed_out"
                elif execution["exit_code"] != 0:
                    status = "failed"
                elif "skip" in text.lower():
                    status = "passed_with_skips"
                else:
                    status = "passed"
            report["tests"].append({
                "name": name, "status": status, "build": build, "run": execution,
            })
        # Temporary names add no reproducible evidence and should not enter docs.
        rendered = json.dumps(report, indent=2).replace(work, "<temporary-work-directory>") + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")
    return 0 if all(t["status"] == "passed" for t in report["tests"]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
