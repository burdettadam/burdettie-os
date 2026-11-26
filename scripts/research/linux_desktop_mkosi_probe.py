#!/usr/bin/env python3
"""Parse a candidate Linux image with upstream mkosi; never build or boot it."""

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


CONFIG = """[Distribution]
Distribution=debian
Release=trixie
Architecture=x86-64

[Output]
Format=disk
Output=burdettie-evaluation
ImageId=burdettie-evaluation

[Content]
Bootable=yes
Bootloader=systemd-boot
Packages=systemd,systemd-sysv,systemd-boot-efi,linux-image-amd64,labwc
"""


def run(argv, cwd):
    result = subprocess.run(argv, cwd=cwd, text=True, capture_output=True, timeout=30)
    return {"argv": argv, "returncode": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mkosi-source", type=Path, required=True)
    args = parser.parse_args()
    source = args.mkosi_source.resolve()
    report = {"scope": "configuration parsing only; package, image and boot unqualified",
              "configuration": CONFIG,
              "upstream_revision": run(["git", "rev-parse", "HEAD"], source),
              "version": run([sys.executable, "-m", "mkosi", "--version"], source),
              "dependencies": run([sys.executable, "-m", "mkosi", "dependencies"], source)}
    with tempfile.TemporaryDirectory(prefix="burdettie-mkosi-probe-") as temp:
        (Path(temp) / "mkosi.conf").write_text(CONFIG)
        result = run([sys.executable, "-m", "mkosi", "--directory", temp,
                      "--json", "summary"], source)
        report["summary_result"] = {k: v for k, v in result.items() if k != "stdout"}
        if result["returncode"] == 0:
            # Keep only image-planning fields, not inherited host/proxy details.
            fields = ["Image", "Distribution", "Release", "Architecture", "Format",
                      "Bootable", "Bootloader", "Output", "OutputExtension", "Packages",
                      "Snapshot", "ToolsTree", "SecureBoot", "RepositoryKeyCheck"]
            report["parsed_images"] = [
                {key: image.get(key) for key in fields}
                for image in json.loads(result["stdout"])["Images"]]
    report["host_tools"] = {t: shutil.which(t) for t in
                            ["qemu-system-x86_64", "systemd-repart", "systemd-ukify", "nix", "labwc"]}
    report["status"] = "configuration_parsed" if result["returncode"] == 0 else "blocked"
    print(json.dumps(report, indent=2))
    return 0 if result["returncode"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
