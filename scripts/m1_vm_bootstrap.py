#!/usr/bin/env python3
"""Bounded, offline PureDarwin 20.5 VM experiment; not M1 candidate qualification.

Supply the official image and a matching OVMF CODE/VARS pair already on disk.
This script never downloads, installs, modifies its inputs or opens a listener.
It keeps a writable image copy and raw evidence in a new private output directory.
Use an enclosing cgroup (for example systemd MemoryMax/CPUQuota) on shared hosts.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import secrets
import selectors
import shutil
import signal
import stat
import subprocess
import time


IMAGE_URL = "https://github.com/PureDarwin/PureDarwin/releases/download/20.5/puredarwin-minimal.img"
IMAGE_SHA256 = "ef65a816395680ef9210f401cebe3783c131badca1a942cb80e56bec97e717ac"
IMAGE_BYTES = 274726912
RELEASE_REVISION = "1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290"
M1_CANDIDATE = "0163b2db225b85e3bf96fc25646ea290105ac298"
MIB = 1024 * 1024
MAX_LOG_BYTES = 16 * MIB  # Combined serial and QEMU stderr, per boot.
MAX_FIRMWARE_BYTES = 16 * MIB
ANSI_CSI = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
PROMPT = re.compile(rb"(?:^|[\r\n])# $")


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(MIB), b""):
            digest.update(chunk)
    return digest.hexdigest()


def memory_available():
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise OSError("MemAvailable not available")


def process_rss(pid):
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except FileNotFoundError:
        pass
    return 0


def complete_line(data, expected):
    """Require a complete response line, not a prompt, substring or partial line."""
    return re.search(rb"(?:^|\n)" + re.escape(expected) + rb"\r?\n", data) is not None


def shell_challenge(nonce):
    first, second = nonce[:len(nonce) // 2], nonce[len(nonce) // 2:]
    command = ("printf '\\nBDT_SHELL_%s%s_%s_%s\\n' "
               f"'{first}' '{second}' \"$(uname -s)\" \"$(uname -m)\"")
    expected = f"BDT_SHELL_{nonce}_Darwin_x86_64".encode()
    assert expected not in command.encode()
    return command, expected


def data_challenge(operation, data_nonce, response_nonce):
    """The success response is assembled in the guest, absent from echoed input."""
    path = f"/var/tmp/burdettie-m1-{data_nonce}.txt"
    data_a, data_b = data_nonce[:16], data_nonce[16:]
    response_a, response_b = response_nonce[:16], response_nonce[16:]
    if operation == "write":
        action = f"printf '%s%s\\n' '{data_a}' '{data_b}' > '{path}' && sync"
        marker = "BDT_WRITE_"
    else:
        expected_path = f"/var/tmp/burdettie-m1-expected-{response_nonce}.txt"
        action = (f"printf '%s%s\\n' '{data_a}' '{data_b}' > '{expected_path}'"
                  f" && cmp -s '{path}' '{expected_path}'")
        marker = "BDT_PERSIST_"
    command = action + f" && printf '\\n{marker}%s%s\\n' '{response_a}' '{response_b}'"
    expected = (marker + response_nonce).encode()
    assert expected not in command.encode()
    return command, expected


def qemu_command(qemu, directory, memory_mib, vcpus):
    # Derived from the release runner, not the newer selected M1 image recipe.
    # TCG single-thread plus child CPU affinity bounds host CPU consumption.
    return [qemu, "-M", "q35", "-accel", "tcg,thread=single",
            "-cpu", "IvyBridge,vendor=GenuineIntel", "-m", str(memory_mib),
            "-smp", str(vcpus), "-vga", "std", "-display", "none",
            "-fw_cfg", "name=opt/ovmf/X-PciMmio64Mb,string=2048",
            "-drive", f"if=pflash,format=raw,unit=0,readonly=on,file={directory / 'firmware-code.fd'}",
            "-drive", f"if=pflash,format=raw,unit=1,file={directory / 'firmware-vars.fd'}",
            "-drive", f"id=root,format=raw,file={directory / 'guest.raw'}",
            "-device", "qemu-xhci,id=xhci", "-device", "usb-kbd,bus=xhci.0",
            "-device", "usb-mouse,bus=xhci.0", "-device", "intel-hda,id=hda",
            "-device", "hda-duplex,audiodev=snd0", "-audiodev", "none,id=snd0",
            "-nic", "none", "-monitor", "none", "-serial", "stdio",
            "-no-reboot", "-no-shutdown"]


def stop_process(process):
    """End this QEMU process group and wait; a second boot cannot overlap it."""
    method = "already_exited"
    if process.poll() is None:
        method = "sigterm_after_guest_sync_or_failed_probe"
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            method = "sigkill_after_termination_timeout"
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)
    return {"method": method, "exit_code": process.returncode,
            "process_exited": process.poll() is not None}


def run_phase(command, directory, phase, data_nonce, timeout, memory_mib, cpu_ids):
    nonce = secrets.token_hex(16)
    shell_command, shell_expected = shell_challenge(nonce)
    data_command, data_expected = data_challenge(phase, data_nonce, nonce)
    record = {"phase": phase, "status": "fail", "started_at": utc_now(),
              "kernel_banner_observed": False, "shell_verified": False,
              "data_verified": False, "peak_rss_bytes": 0,
              "serial_log": f"{phase}-serial.log", "qemu_log": f"{phase}-qemu.log",
              "shell_expected": shell_expected.decode(), "data_expected": data_expected.decode()}
    state, buffer, output_bytes = "prompt", b"", 0

    def limits():
        os.nice(10)
        os.sched_setaffinity(0, cpu_ids)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_CPU, (timeout + 10, timeout + 15))
        resource.setrlimit(resource.RLIMIT_FSIZE, (IMAGE_BYTES, IMAGE_BYTES))

    started = time.monotonic()
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, bufsize=0, start_new_session=True,
                               preexec_fn=limits, cwd=directory,
                               env={"PATH": os.environ.get("PATH", os.defpath),
                                    "LC_ALL": "C", "HOME": str(directory)})
    try:
        with selectors.DefaultSelector() as selector, \
                (directory / record["serial_log"]).open("wb") as serial_log, \
                (directory / record["qemu_log"]).open("wb") as qemu_log:
            selector.register(process.stdout, selectors.EVENT_READ, (serial_log, True))
            selector.register(process.stderr, selectors.EVENT_READ, (qemu_log, False))
            while time.monotonic() - started < timeout:
                rss = process_rss(process.pid)
                record["peak_rss_bytes"] = max(record["peak_rss_bytes"], rss)
                if rss > (memory_mib + 1024) * MIB:
                    record["reason"] = "qemu_rss_limit_exceeded"
                    break
                if process.poll() is not None and not selector.get_map():
                    record["reason"] = f"qemu_exited_waiting_for_{state}"
                    break
                for key, _ in selector.select(timeout=0.25):
                    data = os.read(key.fileobj.fileno(), 65536)
                    if not data:
                        selector.unregister(key.fileobj)
                        continue
                    if output_bytes + len(data) > MAX_LOG_BYTES:
                        record["reason"] = "log_limit_exceeded"
                        break
                    output_bytes += len(data)
                    log, is_serial = key.data
                    log.write(data)
                    log.flush()
                    if not is_serial:
                        continue
                    buffer = (buffer + data)[-MIB:]
                    record["kernel_banner_observed"] |= b"Darwin Kernel Version" in buffer
                    if state == "prompt" and PROMPT.search(ANSI_CSI.sub(b"", buffer)):
                        process.stdin.write(("\n" + shell_command + "\n").encode())
                        state, buffer = "shell_response", b""
                    elif state == "shell_response" and complete_line(buffer, shell_expected):
                        record["shell_verified"] = True
                        process.stdin.write((data_command + "\n").encode())
                        state, buffer = "data_response", b""
                    elif state == "data_response" and complete_line(buffer, data_expected):
                        record.update(status="pass", data_verified=True)
                        break
                if record["status"] == "pass" or "reason" in record:
                    break
            else:
                record["reason"] = f"timeout_waiting_for_{state}"
    except (OSError, BrokenPipeError) as exc:
        record["reason"] = f"serial_io_error:{type(exc).__name__}"
    finally:
        record["termination"] = stop_process(process)
        for stream in (process.stdin, process.stdout, process.stderr):
            stream.close()
        record["duration_seconds"] = round(time.monotonic() - started, 3)
        record["finished_at"] = utc_now()
    return record


def write_report(directory, report):
    temporary = directory / "result.json.tmp"
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(directory / "result.json")


def run(args):
    directory = args.output_dir.resolve()
    report = {"schema_version": 1, "status": "blocked", "scope": "puredarwin_20.5_bootstrap_only",
              "release_revision": RELEASE_REVISION, "image_url": IMAGE_URL,
              "expected_image_sha256": IMAGE_SHA256,
              "host": {"os": platform.system(), "architecture": platform.machine(),
                       "kernel": platform.release()},
              "harness_sha256": sha256(Path(__file__)),
              "selected_m1_candidate_revision": M1_CANDIDATE,
              "selected_m1_candidate_qualified": False, "burdettie_runtime_executed": False,
              "apple_application_qualified": False, "security_enforcement_qualified": False,
              "started_at": utc_now(), "blockers": [], "phases": [],
              "notes": ["This older release and resource profile do not qualify the selected M1 candidate.",
                        "A pass means nonce-verified shell execution and exact-byte persistence across two QEMU processes.",
                        "The first process is stopped by the host after guest sync; graceful guest shutdown is not tested.",
                        "Use an enclosing cgroup memory/CPU limit on shared hosts; the script also bounds affinity, RSS, logs and wall time."]}
    if any(c in str(directory) for c in (",", "\n", "\r")):
        report["blockers"].append("Output path contains a QEMU option separator or newline.")
        return report
    try:
        directory.mkdir(mode=0o700, parents=False, exist_ok=False)
    except OSError as exc:
        report["blockers"].append(f"A new output directory is required: {type(exc).__name__}.")
        return report
    try:
        if platform.system() != "Linux" or platform.machine() != "x86_64":
            report["blockers"].append("This bootstrap profile requires x86_64 Linux.")
        qemu = shutil.which("qemu-system-x86_64")
        if not qemu:
            report["blockers"].append("qemu-system-x86_64 is unavailable.")
        for label, path, maximum in (("image", args.image, IMAGE_BYTES),
                                     ("firmware_code", args.firmware_code, MAX_FIRMWARE_BYTES),
                                     ("firmware_vars", args.firmware_vars, MAX_FIRMWARE_BYTES)):
            try:
                info = path.stat()
                if not stat.S_ISREG(info.st_mode) or info.st_size <= 0 or info.st_size > maximum:
                    raise ValueError("file type or size")
                if label == "image" and info.st_size != IMAGE_BYTES:
                    raise ValueError("image size")
                report[label] = {"bytes": info.st_size, "sha256": sha256(path)}
            except (OSError, ValueError) as exc:
                report["blockers"].append(f"Invalid {label} input: {type(exc).__name__}.")
        if report.get("image", {}).get("sha256") != IMAGE_SHA256:
            report["blockers"].append("Image digest does not match the pinned official release asset.")
        free_disk = shutil.disk_usage(directory).free
        available_memory = memory_available()
        report["host_capacity"] = {"free_disk_bytes": free_disk, "available_memory_bytes": available_memory}
        if free_disk < 1024 * MIB:
            report["blockers"].append("At least 1 GiB free disk is required for this bounded experiment.")
        if available_memory < (args.memory_mib + 512) * MIB:
            report["blockers"].append("Insufficient available memory for the guest plus 512 MiB headroom.")
        if report["blockers"]:
            return report
        version = subprocess.run([qemu, "--version"], capture_output=True, timeout=10, check=True)
        report["qemu_version"] = version.stdout.decode(errors="replace").splitlines()[0][:512]
        report["qemu_sha256"] = sha256(Path(qemu))
        for source, name, label in ((args.image, "guest.raw", "image"),
                                   (args.firmware_code, "firmware-code.fd", "firmware_code"),
                                   (args.firmware_vars, "firmware-vars.fd", "firmware_vars")):
            destination = directory / name
            shutil.copyfile(source, destination)
            destination.chmod(0o600)
            if sha256(destination) != report[label]["sha256"]:
                raise ValueError(f"Copied {label} digest mismatch")
        cpu_ids = [max(os.sched_getaffinity(0))]
        command = qemu_command(qemu, directory, args.memory_mib, args.vcpus)
        report["vm"] = {"memory_mib": args.memory_mib, "vcpus": args.vcpus,
                        "host_cpu_affinity": cpu_ids, "accelerator": "tcg,thread=single",
                        "guest_network": "none", "public_listeners": False,
                        "per_boot_timeout_seconds": args.timeout, "argv": command}
        report["status"] = "running"
        write_report(directory, report)
        data_nonce = secrets.token_hex(16)
        report["guest_data_file"] = f"/var/tmp/burdettie-m1-{data_nonce}.txt"
        report["expected_data_sha256"] = hashlib.sha256((data_nonce + "\n").encode()).hexdigest()
        for phase in ("write", "read"):
            result = run_phase(command, directory, phase, data_nonce, args.timeout,
                               args.memory_mib, cpu_ids)
            report["phases"].append(result)
            write_report(directory, report)
            if result["status"] != "pass" or not result["termination"]["process_exited"]:
                report["status"] = "fail"
                break
        else:
            report["status"] = "pass"
        for path in sorted(directory.glob("*.log")):
            report.setdefault("evidence", {})[path.name] = {"bytes": path.stat().st_size, "sha256": sha256(path)}
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        report["status"] = "fail" if report["status"] in ("running", "pass", "fail") else "blocked"
        report["error"] = f"{type(exc).__name__}: {exc}"
    except KeyboardInterrupt:
        report["status"] = "fail"
        report["error"] = "interrupted"
    finally:
        report["finished_at"] = utc_now()
        write_report(directory, report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--firmware-code", required=True, type=Path)
    parser.add_argument("--firmware-vars", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path, help="New directory; parent must exist")
    parser.add_argument("--memory-mib", type=int, default=2048, choices=range(1024, 4097), metavar="1024..4096")
    parser.add_argument("--vcpus", type=int, default=2, choices=range(1, 5))
    parser.add_argument("--timeout", type=int, default=600, choices=range(30, 1801), metavar="30..1800",
                        help="Wall-time cap per boot in seconds (default 600)")
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, signal.default_int_handler)
    report = run(args)
    print(json.dumps(report, indent=2))
    return {"pass": 0, "fail": 1, "blocked": 2}.get(report["status"], 1)


if __name__ == "__main__":
    raise SystemExit(main())
