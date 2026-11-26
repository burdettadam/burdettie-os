#!/usr/bin/env python3
"""Exercise the pinned xcrun wrapper's discovery branches without Nix or an SDK.

This renders only the shell body and two literal Nix parameters. It does not
evaluate a Nix derivation, invoke a compiler, or qualify an SDK/package build.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap


REVISION = "0c1990c803c13b68cad4df3f6dc34ba4306fc789"
SHA256 = "59c146da8798d5bc5a9aab2c2288fc045e209147076803a60488bba0a5a9e972"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--toolchain-nix", required=True, type=Path)
    args = parser.parse_args()
    data = args.toolchain_nix.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA256:
        parser.error("source differs from the pinned upstream toolchain.nix")
    source = data.decode()
    start = source.index('  xcrunShim = writeShellScriptBin "xcrun" \'\'\n')
    start = source.index("\n", start) + 1
    end = source.index("\n  '';", start)
    body = textwrap.dedent(source[start:end])
    values = {}
    for name in ("defaultSdkRoot", "target"):
        match = re.search(r", " + name + r' \? "([^"\n]+)"', source)
        if match is None:
            parser.error(f"cannot find literal parameter {name}")
        values[name] = match.group(1)
        body = body.replace("${" + name + "}", values[name])
    body = body.replace("''${", "${")
    report = {"upstream_revision": REVISION, "source_sha256": SHA256,
              "scope": "rendered discovery branches only; no Nix evaluation or SDK build", "cases": []}
    with tempfile.TemporaryDirectory(prefix="burdettie-sdk-probe-") as work:
        wrapper = Path(work) / "xcrun"
        wrapper.write_text("#!/bin/bash\n" + body + "\n")
        wrapper.chmod(0o700)
        for override in (None, "/fixture/MacOSX26.5.sdk"):
            env = os.environ.copy()
            env.pop("DARWIN_SDK_ROOT", None)
            if override is not None:
                env["DARWIN_SDK_ROOT"] = override
            observed = {}
            for option in ("--show-sdk-path", "--show-sdk-version"):
                result = subprocess.run([str(wrapper), option], env=env,
                                        capture_output=True, text=True, timeout=5)
                observed[option] = {"exit_code": result.returncode,
                                    "stdout": result.stdout.strip(), "stderr": result.stderr}
            report["cases"].append({"sdk_root_override": override, "observed": observed})
    print(json.dumps(report, indent=2))
    # This is a reproducer: command success is not SDK consistency success.
    return 0 if all(v["exit_code"] == 0 for c in report["cases"] for v in c["observed"].values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
