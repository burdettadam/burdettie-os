#!/usr/bin/env python3
"""Bounded native C build/run/edit/diagnostic probe; no IDE qualification claim."""

import argparse
import hashlib
import json
import platform
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path


def run(argv, cwd):
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, timeout=30)
    return {"argv": argv, "returncode": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cc", default=shutil.which("cc"))
    args = parser.parse_args()
    tools = ["cargo", "rustc", "clang", "clang-19", "clangd", "clangd-19",
             "lldb", "lldb-19", "lldb-dap", "lldb-dap-19", "gdb", "swift",
             "sourcekit-lsp", "nix", "mkosi", "labwc", "qemu-system-x86_64"]
    report = {"scope": "native C only; no LSP, DAP, IDE or GUI qualification",
              "host": platform.platform(), "tools": {t: shutil.which(t) for t in tools}}
    if not args.cc:
        report.update(status="blocked", reason="No C compiler available")
        print(json.dumps(report, indent=2))
        return 2
    with tempfile.TemporaryDirectory(prefix="burdettie-devtools-probe-") as temp:
        directory = Path(temp)
        report["compiler"] = run([args.cc, "--version"], directory)
        builds = []
        for value in (41, 42):
            (directory / "main.c").write_text(
                '#include <stdio.h>\nint main(void) { printf("native value=%d\\n", '
                + str(value) + '); return 0; }\n')
            build = run([args.cc, "-std=c11", "-Wall", "-Wextra", "-Werror", "-g",
                         "-O0", "main.c", "-o", "native-probe"], directory)
            item = {"source_value": value, "build": build}
            if build["returncode"] == 0:
                data = (directory / "native-probe").read_bytes()
                item["sha256"] = hashlib.sha256(data).hexdigest()
                item["elf_magic"] = data[:4].hex()
                item["elf_class"] = data[4]
                item["elf_machine"] = struct.unpack(
                    "<H" if data[5] == 1 else ">H", data[18:20])[0]
                item["run"] = run(["./native-probe"], directory)
            builds.append(item)
        report["builds"] = builds
        (directory / "invalid.c").write_text("int main(void) { return missing_symbol; }\n")
        report["invalid_source"] = run(
            [args.cc, "-std=c11", "-fdiagnostics-format=json", "-c", "invalid.c"], directory)
        try:
            report["structured_diagnostics"] = json.loads(report["invalid_source"]["stderr"])
        except json.JSONDecodeError:
            report["structured_diagnostics"] = None
        report["checks"] = {
            "both_builds_succeeded": all(b["build"]["returncode"] == 0 for b in builds),
            "native_runs_match_edit": all(
                b.get("run", {}).get("returncode") == 0 and
                b["run"]["stdout"] == f"native value={b['source_value']}\n" for b in builds),
            "elf64_x86_64": all(b.get("elf_magic") == "7f454c46" and
                                b.get("elf_class") == 2 and b.get("elf_machine") == 62
                                for b in builds),
            "artifact_changed_after_edit": builds[0].get("sha256") is not None and
                                           builds[0].get("sha256") != builds[1].get("sha256"),
            "invalid_source_rejected": report["invalid_source"]["returncode"] != 0,
            "structured_error_has_source_location": any(
                d.get("kind") == "error" and any(
                    loc.get("caret", {}).get("file") == "invalid.c" and
                    loc.get("caret", {}).get("line") == 1 for loc in d.get("locations", []))
                for d in (report["structured_diagnostics"] or [])),
        }
    report["status"] = "passed" if all(report["checks"].values()) else "failed"
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
