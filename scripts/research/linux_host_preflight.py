#!/usr/bin/env python3
"""Small compiler-free host probes; no runtime or confinement qualification.

Run as the intended unprivileged worker account. Uses disposable child
processes, private tmpfs sockets and its own memory. It never changes host
policy, mounts a filesystem, installs a package or contacts a network.
"""

import ctypes
import errno
import fcntl
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import socket
import subprocess
import tempfile


def child_result(operation):
    """Keep all process-local restriction changes in a disposable child."""
    read_fd, write_fd = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(read_fd)
        signal.alarm(10)
        try:
            value = operation()
        except Exception as error:
            value = {"error": str(error), "type": type(error).__name__}
        os.write(write_fd, json.dumps(value).encode())
        os.close(write_fd)
        os._exit(0)
    os.close(write_fd)
    data = bytearray()
    with os.fdopen(read_fd, "rb") as stream:
        data.extend(stream.read())
    _, status = os.waitpid(pid, 0)
    return {"wait_status": status, "result": json.loads(data) if data else None}


def memory_probe():
    libc = ctypes.CDLL(None, use_errno=True)

    class Iovec(ctypes.Structure):
        _fields_ = [("base", ctypes.c_void_p), ("length", ctypes.c_size_t)]

    source = ctypes.c_ubyte(42)
    destination = ctypes.c_ubyte(0)
    local = Iovec(ctypes.addressof(destination), 1)
    remote = Iovec(ctypes.addressof(source), 1)
    libc.process_vm_readv.restype = ctypes.c_ssize_t
    libc.process_vm_readv.argtypes = [
        ctypes.c_int, ctypes.POINTER(Iovec), ctypes.c_ulong,
        ctypes.POINTER(Iovec), ctypes.c_ulong, ctypes.c_ulong,
    ]
    ctypes.set_errno(0)
    copied = libc.process_vm_readv(os.getpid(), ctypes.byref(local), 1,
                                  ctypes.byref(remote), 1, 0)
    read_error = ctypes.get_errno() if copied < 0 else 0
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong,
                          ctypes.c_ulong, ctypes.c_ulong]
    ctypes.set_errno(0)
    ptracer = libc.prctl(0x59616D61, ctypes.c_ulong(-1).value, 0, 0, 0)
    return {"process_vm_readv_self": copied, "errno": read_error,
            "own_byte_matches": destination.value == 42,
            "pr_set_ptracer_any": ptracer,
            "ptracer_errno": ctypes.get_errno() if ptracer < 0 else 0}


def landlock_probe():
    # Linux x86_64 syscall numbers. Refuse other architectures explicitly.
    if platform.machine() != "x86_64":
        return {"status": "unsupported_probe_architecture"}
    libc = ctypes.CDLL(None, use_errno=True)
    libc.syscall.restype = ctypes.c_long
    libc.syscall.argtypes = [ctypes.c_long]  # Remaining arguments are variadic.
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong,
                          ctypes.c_ulong, ctypes.c_ulong]
    abi = libc.syscall(444, ctypes.c_void_p(), ctypes.c_size_t(0), ctypes.c_ulong(1))
    if abi < 0:
        return {"abi": abi, "errno": ctypes.get_errno()}

    class Ruleset(ctypes.Structure):
        _fields_ = [("handled_access_fs", ctypes.c_uint64)]

    class PathRule(ctypes.Structure):
        _pack_ = 1
        _fields_ = [("allowed_access", ctypes.c_uint64),
                    ("parent_fd", ctypes.c_int32)]

    rule_set = Ruleset((1 << 2) | (1 << 3))  # READ_FILE | READ_DIR
    fd = libc.syscall(444, ctypes.byref(rule_set), ctypes.c_size_t(ctypes.sizeof(rule_set)),
                      ctypes.c_ulong(0))
    if fd < 0:
        return {"abi": abi, "create": fd, "errno": ctypes.get_errno()}
    allowed = os.open("/etc/os-release", os.O_PATH | os.O_CLOEXEC)
    rule = PathRule(1 << 2, allowed)
    added = libc.syscall(445, ctypes.c_long(fd), ctypes.c_ulong(1), ctypes.byref(rule),
                         ctypes.c_ulong(0))
    add_errno = ctypes.get_errno() if added < 0 else 0
    os.close(allowed)
    if added < 0:
        os.close(fd)
        return {"abi": abi, "add": added, "errno": add_errno}
    no_new_privs = libc.prctl(38, 1, 0, 0, 0)
    restricted = libc.syscall(446, ctypes.c_long(fd), ctypes.c_ulong(0)) if no_new_privs == 0 else -1
    restrict_errno = ctypes.get_errno() if restricted < 0 else 0
    os.close(fd)
    results = {"abi": abi, "restrict": restricted,
               "restrict_errno": restrict_errno}
    if restricted == 0:
        for label, path in [("allowed", "/etc/os-release"),
                            ("denied", "/etc/hostname")]:
            try:
                opened = os.open(path, os.O_RDONLY | os.O_CLOEXEC)
                os.close(opened)
                results[label] = {"opened": True}
            except OSError as error:
                results[label] = {"opened": False, "errno": error.errno}
        results["selected_read_boundary_observed"] = (
            results["allowed"].get("opened") is True
            and results["denied"].get("errno") == errno.EACCES
        )
    return results


def sockets_probe():
    if not os.access("/dev/shm", os.W_OK):
        return {"status": "tmpfs_scratch_unavailable"}
    results = []
    with tempfile.TemporaryDirectory(prefix="bdt-probe-", dir="/dev/shm") as work:
        for name, kind in [("stream", socket.SOCK_STREAM),
                           ("datagram", socket.SOCK_DGRAM)]:
            stage = "socket"
            try:
                with socket.socket(socket.AF_UNIX, kind) as server:
                    server.settimeout(2)
                    stage = "bind"
                    server.bind(str(Path(work) / name))
                    if kind == socket.SOCK_STREAM:
                        server.listen(1)
                    with socket.socket(socket.AF_UNIX, kind) as client:
                        client.settimeout(2)
                        stage = "roundtrip"
                        client.connect(str(Path(work) / name))
                        client.send(b"p")
                        if kind == socket.SOCK_STREAM:
                            connection, _ = server.accept()
                            with connection:
                                received = connection.recv(1)
                        else:
                            received = server.recv(1)
                        results.append({"type": name, "passed": received == b"p"})
            except OSError as error:
                results.append({"type": name, "stage": stage,
                                "errno": error.errno})
    return results


def main():
    status = Path("/proc/self/status").read_text()
    report = {
        "scope": "host_prerequisites_not_runtime_or_confinement_qualification",
        "kernel": platform.release(), "architecture": platform.machine(),
        "uid": os.getuid(), "gid": os.getgid(),
        "process_status": {
            key: value.strip()
            for key, value in (line.split(":", 1) for line in status.splitlines())
            if key in {"NoNewPrivs", "Seccomp", "CapEff", "CapBnd"}
        },
    }
    report["host_controls"] = {}
    for path in ["/proc/sys/kernel/apparmor_restrict_unprivileged_userns",
                 "/proc/sys/kernel/unprivileged_userns_clone",
                 "/proc/sys/kernel/yama/ptrace_scope", "/proc/self/attr/current"]:
        try:
            report["host_controls"][path] = Path(path).read_text().strip()
        except OSError as error:
            report["host_controls"][path] = {"errno": error.errno}
    report["sockets"] = sockets_probe()
    report["memory"] = child_result(memory_probe)
    report["landlock"] = child_result(landlock_probe)
    report["namespaces"] = []
    if shutil.which("unshare"):
        for extra in [[], ["--mount", "--propagation", "unchanged"],
                      ["--uts"], ["--ipc"], ["--net"], ["--pid", "--fork"]]:
            command = ["unshare", "--user", "--map-root-user", *extra, "/bin/true"]
            result = subprocess.run(command, capture_output=True, text=True, timeout=10)
            report["namespaces"].append({"argv": command, "exit": result.returncode,
                                         "stdout": result.stdout, "stderr": result.stderr})
    mount_ns = os.open("/proc/self/ns/mnt", os.O_RDONLY | os.O_CLOEXEC)
    try:
        owner_ns = fcntl.ioctl(mount_ns, 0xB701)  # NS_GET_USERNS
        os.close(owner_ns)
        report["mount_namespace_owner_ioctl"] = {"passed": True}
    except OSError as error:
        report["mount_namespace_owner_ioctl"] = {"passed": False, "errno": error.errno}
    finally:
        os.close(mount_ns)
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
