#!/usr/bin/env python3
"""Realize one verified cross-toolchain inside an existing bounded Linux cgroup.

Revalidate the retained plan with the unchanged staged-libraries planner, then
optionally realize its exact output offline and inspect its installed wrappers.
This never configures a cgroup, cloud service, stop timer, or Nix installation.
Keep the output directory private: receipts include local paths and identities.
"""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import time

import inspect_puredarwin_cross_toolchain as cross
import plan_puredarwin_staged_libraries as planner


base, native, inspection = planner.base, planner.native, planner.inspection
SCRIPT = Path(__file__).resolve()
SCOPE = "staged_cross_toolchain_build"
SUCCESS = "cross_toolchain_built"
READY = "cross_toolchain_ready_not_built"
GIB = 1024 ** 3
DISK_RESERVE = 8 * GIB
MAX_GROWTH = 128 * 1024 ** 2
MAX_LOG = 20 * 1024 ** 2
TOTAL_SECONDS = 600
SERVICE_PROPERTIES = ("Id", "Type", "ActiveState", "SubState", "User", "MainPID", "ControlPID",
                      "ControlGroup", "KillMode", "ExitType", "RemainAfterExit", "SendSIGKILL",
                      "KillSignal", "FinalKillSignal", "Delegate", "RuntimeMaxUSec", "TimeoutStopUSec",
                      "PrivateNetwork", "NetworkNamespacePath", "JoinsNamespaceOf", "NoNewPrivileges", "Nice", "IOWeight")
PHASE_SECONDS = {"fresh-plan": 300, "cross-toolchain-build": 180, "cross-output-inspection": 180}
CODE = {path.name: path.resolve() for path in planner.FILES.values()}
CODE.update({path.name: path for path in (SCRIPT, Path(planner.__file__).resolve(),
             Path(cross.__file__).resolve(), SCRIPT.with_name("collect_m1_inventory.py"))})
SEMANTIC_FIELDS = ("source", "sdk", "expected_sdk", "upstream", "previous_bounded", "rebound_unbounded",
                   "candidate", "native_linker", "cross_toolchain", "compiler_rt", "baseline_checks",
                   "preserved", "successful_native_linker", "current_libtapi_inspection")


def now():
    return datetime.now(timezone.utc).isoformat()


class OperationTimeout(base.BuildError):
    pass


class OperationInterrupted(base.BuildError):
    pass


def interrupted(signum, _frame):
    raise OperationInterrupted("Operation interrupted by signal " + str(signum))


@contextmanager
def controlled_environment(value):
    previous = dict(os.environ)
    os.environ.clear()
    os.environ.update(value)
    try:
        yield
    finally:
        os.environ.clear()
        os.environ.update(previous)


def child_environment(executable, temporary):
    # Retain the owner's home identity, but never load their Git/Nix user config.
    value = {key: os.environ[key] for key in ("HOME", "USER", "LOGNAME") if key in os.environ}
    value.update(PATH=str(executable.parent) + ":/usr/bin:/bin", LC_ALL="C", NIX_REMOTE="local",
                 NIX_USER_CONF_FILES="/dev/null", PYTHONDONTWRITEBYTECODE="1", GIT_TERMINAL_PROMPT="0",
                 GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null", GIT_ALLOW_PROTOCOL="",
                 GIT_CONFIG_COUNT="2", GIT_CONFIG_KEY_0="core.fsmonitor", GIT_CONFIG_VALUE_0="false",
                 GIT_CONFIG_KEY_1="core.hooksPath", GIT_CONFIG_VALUE_1="/dev/null", TMPDIR=str(temporary))
    native.require(executable.name == "nix" and
                   Path(shutil.which("nix", path=value["PATH"]) or "/missing-nix").resolve() == executable,
                   "Child PATH must resolve nix to the exact reviewed executable")
    return value


def checksum(value):
    native.require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None,
                   "An explicit SHA256 checksum is required")
    return value


def read_record(path, expected):
    path = Path(path)
    native.require(stat.S_ISREG(path.lstat().st_mode), "Receipt must be a regular file")
    return native.read_json(path, checksum(expected))


def real_path(path):
    path = Path(path).absolute()
    native.require(".." not in path.parts and path.parent.resolve(strict=True) == path.parent,
                   "Path needs a canonical existing parent without symlinks")
    return path


def new_directory(path):
    path = real_path(path)
    native.require(not path.is_relative_to(cross.STORE), "Evidence cannot be written into the Nix store")
    path.mkdir(mode=0o700)  # Exclusive creation also rejects dangling symlinks.
    return path


def reviewed_code(path, expected):
    manifest, raw = read_record(path, expected)
    native.require(set(manifest) == {"schema_version", "scope", "sha256"}
                   and type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
                   and manifest["scope"] == "reviewed_cross_toolchain_source"
                   and isinstance(manifest["sha256"], dict) and set(manifest["sha256"]) == set(CODE),
                   "Reviewed source manifest must name the complete helper/recipe set")
    for name, source in CODE.items():
        native.require(stat.S_ISREG(source.lstat().st_mode) and not source.is_symlink()
                       and source.parent.resolve(strict=True) == source.parent,
                       "Reviewed source must be a regular file in its canonical directory")
        native.require(base.sha256(source) == checksum(manifest["sha256"][name]),
                       "Reviewed helper bytes changed: " + name)
    return manifest, raw


def planned_commands(candidate, output):
    commands = planner.linker.build_commands(candidate, output, False)
    result = {key: inspection.NIX + argv[len(planner.linker.NIX):] for key, argv in commands.items()}
    result["realize"][-1] = str(output / "result-cross-toolchain")
    return result


def validate_plan(value, code, driver_checksum, directory):
    native.require(type(value["schema_version"]) is int and value["schema_version"] == 1
                   and value["scope"] == "staged_native_linker_library_plan_only"
                   and value["status"] == "planned_not_built" and value["stage"] == "cross-toolchain"
                   and value["planning_mode"] == "offline_graph_only"
                   and value["substituter_queries_enabled"] is False,
                   "An exact successful offline cross-toolchain plan is required")
    native.require(value["helper_sha256"] == code[Path(planner.__file__).name], "Cross planner changed")
    for key, path in planner.FILES.items():
        native.require(value[key] == code[path.name], "Cross-plan helper or recipe changed: " + path.name)
    for field in ("dependency_realization", "dependency_downloads", "fixture_linking", "guest_execution",
                  "burdettie_runtime_execution", "security_enforcement"):
        native.require(value[field] == "not_run", "A metadata plan cannot claim " + field)
    native.require(value["peak_space_qualified"] is False
                   and value["source_inventory_matches_successful_build"] is True
                   and value["current_libtapi_inspection"]["matches_successful_receipt"] is True,
                   "Plan source/TAPI proof is missing or metadata was promoted to capacity evidence")
    native.require(re.fullmatch(r"[0-9a-f]{40}", value["burdettie_revision"]) is not None,
                   "Cross-plan source revision is missing")
    native.require(value["nix_version"] == "nix (Nix) 2.28.3", "The selected Nix 2.28.3 is required")
    for name in ("upstream", "previous_bounded", "rebound_unbounded", "candidate", "sdk"):
        native.require(set(value[name]) == {"drv", "out"}, "Unexpected candidate identity fields")
        cross.store_path(value[name]["drv"], derivation=True)
        cross.store_path(value[name]["out"])
    for key in ("drv", "out"):
        native.require(len({value[name][key] for name in
                       ("upstream", "previous_bounded", "rebound_unbounded", "candidate")}) == 4,
                       "Distinct upstream, rebound and bounded candidates are required")
    preserved = {name: None if name in ("baseline_compiler_rt", "baseline_libsystem") else True
                 for name in planner.PRESERVED}
    native.require(value["preserved"] == preserved and value["compiler_rt"] is None
                   and value["baseline_checks"]["compiler_rt"] is None
                   and value["baseline_checks"]["libsystem"] is None
                   and set(value["baseline_checks"]["cross_toolchain"]) ==
                   {"name", "build_command", "wrapper_names", "only_linker_wrappers_changed"}
                   and all(item is True for item in value["baseline_checks"]["cross_toolchain"].values())
                   and value["cross_toolchain"]["candidate"] == value["candidate"],
                   "Cross-only recipe preservation is incomplete")
    success = value["successful_native_linker"]
    for key in ("plan_sha256", "build_result_sha256", "driver_sha256"):
        checksum(success[key])
    native.require(success["driver_sha256"] == checksum(driver_checksum), "Reviewed native driver changed")
    current = value["current_native_inspection"]
    native.require(current["schema_version"] == 1 and current["scope"] == inspection.INSPECTION_SCOPE
                   and current["status"] == "pass"
                   and all(current["registered_output"][key] == success["registered_output"][key]
                           for key in cross.IDENTITY_KEYS), "Successful native identity changed")
    native.require(value["sdk"] == value["expected_sdk"]
                   and current["dependencies"]["sdk"]["path"] == value["sdk"]["out"]
                   and current["dependencies"]["sdk"]["deriver"] == value["sdk"]["drv"],
                   "Selected SDK identity changed")
    native.require(inspection.zero(value["dry_run"]["exit_code"])
                   and value["dry_run"]["timed_out"] is False
                   and base.sha256(directory / "dry-run.log") == checksum(value["dry_run"]["log_sha256"]),
                   "Retained dry-run failed, timed out, or changed")
    old = value["commands"]["realize"]
    native.require(isinstance(old, list) and old and all(isinstance(x, str) for x in old),
                   "Missing planned realization argv")
    out = Path(old[-1])
    native.require(out.is_absolute() and ".." not in out.parts and str(out) == old[-1]
                   and out.name == "result-cross-toolchain", "Invalid original output link")
    native.require(all(value["commands"][key] == argv for key, argv in
                       planned_commands(value["candidate"], out.parent).items()),
                   "Plan argv changed candidate, local-store selection, network or job bounds")


def compare_plans(retained, fresh):
    for key in (*SEMANTIC_FIELDS, "helper_sha256", *planner.FILES):
        native.require(retained[key] == fresh[key], "Fresh cross plan differs: " + key)
    native.require(retained["commands"]["evaluate"] == fresh["commands"]["evaluate"],
                   "Fresh offline evaluation differs from the retained source/expression")
    # Path-info serialization and loader ASLR addresses are not content identity.
    for key in ("files", "symlinks", "elf_files", "references", "dependencies", "tapi_dynamic_binding",
                "version_identity", "loader_registered_output", "loader_resolution", "smoke_test"):
        native.require(retained["current_native_inspection"][key] == fresh["current_native_inspection"][key],
                       "Fresh native inspection differs: " + key)


def cgroup_state():
    entries = Path("/proc/self/cgroup").read_text().splitlines()
    native.require(len(entries) == 1 and entries[0].startswith("0::/"), "A unified cgroup v2 is required")
    relative = Path(entries[0][3:])
    native.require(relative != Path("/") and ".." not in relative.parts, "A bounded child cgroup is required")
    root = Path("/sys/fs/cgroup") / relative.relative_to("/")
    native.require(root.resolve(strict=True) == root, "Unexpected cgroup path")
    return {"path": str(root), **{name: (root / name).read_text().strip() for name in
            ("cpu.max", "cpu.weight", "memory.high", "memory.max", "memory.swap.max", "pids.max", "memory.events")}}


def check_cgroup(value):
    cpu = value["cpu.max"].split()
    native.require(len(cpu) == 2 and all(part.isdecimal() for part in cpu), "A finite CPU quota is required")
    quota, period = map(int, cpu)
    native.require(0 < quota * 4 <= period and period > 0, "CPU quota exceeds 25 percent of one core")
    for key, maximum in (("cpu.weight", 10), ("memory.high", 1280 * 1024 ** 2),
                         ("memory.max", 1536 * 1024 ** 2), ("pids.max", 128)):
        native.require(value[key].isdecimal() and 0 < int(value[key]) <= maximum, "Unsafe cgroup bound: " + key)
    native.require(value["memory.swap.max"] == "0", "The operation cgroup must disable swap")
    events = dict(line.split() for line in value["memory.events"].splitlines())
    native.require(all(events.get(key, "").isdecimal() for key in ("oom", "oom_kill")), "Missing cgroup OOM counters")
    return {key: int(events[key]) for key in ("oom", "oom_kill")}


def duration_seconds(value):
    parts = value.split()
    units = {"us": 0.000001, "ms": 0.001, "s": 1, "min": 60, "h": 3600}
    native.require(0 < len(parts) <= 5, "Finite systemd duration required")
    total = 0
    for part in parts:
        match = re.fullmatch(r"([0-9]{1,8}(?:\.[0-9]{1,6})?)(us|ms|s|min|h)", part)
        native.require(match is not None, "Unsupported or unbounded systemd duration")
        total += float(match[1]) * units[match[2]]
    return total


def service_state(unit):
    native.require(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.@-]{0,180}\.service", unit) is not None,
                   "An explicit existing service name is required")
    argv = ["/usr/bin/systemctl", "show", "--no-pager", "--property=" + ",".join(SERVICE_PROPERTIES), unit]
    result = subprocess.run(argv, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=10,
                            env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"}, check=False)
    native.require(inspection.zero(result.returncode) and len(result.stdout) < 16384,
                   "Could not inspect the existing containing service")
    fields = [line.split("=", 1) for line in result.stdout.splitlines()]
    native.require(all(len(item) == 2 for item in fields), "Malformed containing-service observation")
    value = dict(fields)
    native.require(len(value) == len(fields) and set(value) == set(SERVICE_PROPERTIES),
                   "Incomplete or ambiguous containing-service observation")
    return value


def check_service(value, unit, cgroup):
    relative = "/" + str(Path(cgroup["path"]).relative_to("/sys/fs/cgroup"))
    native.require(value["Id"] == unit and value["Type"] in ("exec", "simple")
                   and value["ActiveState"] == "active" and value["SubState"] == "running"
                   and value["User"] in (str(os.geteuid()), pwd.getpwuid(os.geteuid()).pw_name)
                   and value["MainPID"] == str(os.getpid()) and value["ControlPID"] == "0"
                   and value["ControlGroup"] == relative, "Producer is not the sole main process of the selected service")
    native.require(value["KillMode"] == "control-group" and value["ExitType"] == "main"
                   and value["RemainAfterExit"] == "no" and value["SendSIGKILL"] == "yes"
                   and value["KillSignal"] == "15" and value["FinalKillSignal"] == "9"
                   and value["Delegate"] == "no", "Containing service does not guarantee descendant cleanup on main exit")
    native.require(value["PrivateNetwork"] == "yes" and value["NetworkNamespacePath"] == ""
                   and value["JoinsNamespaceOf"] == "" and value["NoNewPrivileges"] == "yes",
                   "The existing operation service must have its own private network and prevent privilege gains")
    native.require(value["Nice"].isdecimal() and 15 <= int(value["Nice"]) <= 19
                   and value["IOWeight"].isdecimal() and 1 <= int(value["IOWeight"]) <= 10,
                   "The operation must preserve low CPU scheduling and I/O priorities")
    native.require(0 < duration_seconds(value["RuntimeMaxUSec"]) <= 600
                   and 0 < duration_seconds(value["TimeoutStopUSec"]) <= 25,
                   "Containing service needs finite runtime and bounded forced cleanup")


def cgroup_processes(root):
    root = Path(root)
    entries = [root, *(path for path in root.rglob("*") if path.is_dir())]
    native.require(len(entries) <= 128, "Unexpected number of operation cgroups")
    processes = set()
    for entry in entries:
        native.require(not entry.is_symlink() and entry.resolve(strict=True).is_relative_to(root),
                       "Unexpected nested cgroup path")
        info = entry.stat()
        native.require(info.st_uid == 0 and not info.st_mode & 0o022 and not os.access(entry, os.W_OK),
                       "The operation owner must not be able to move tasks between cgroups")
        values = (entry / "cgroup.procs").read_text().split()
        native.require(len(values) <= 128 and all(value.isdecimal() and int(value) > 0 for value in values),
                       "Invalid or oversized operation process census")
        processes.update(int(value) for value in values)
    native.require(len(processes) <= 128, "Operation task census exceeds the containing limit")
    return processes


def resources(paths):
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    available = memory["MemAvailable"].split()
    native.require(len(available) == 2 and available[1] == "kB", "Missing host available-memory observation")
    result = {"available_memory_bytes": int(available[0]) * 1024, "filesystems": {}}
    for path in paths:
        info = os.statvfs(path)
        result["filesystems"][str(path)] = {"free_bytes": info.f_bavail * info.f_frsize, "free_inodes": info.f_favail}
    return result


def check_resources(value, initial=None):
    native.require(type(value["available_memory_bytes"]) is int
                   and value["available_memory_bytes"] >= (GIB if initial else 2 * GIB), "Host memory reserve reached")
    native.require(value["filesystems"] and (initial is None or
                   set(value["filesystems"]) == set(initial["filesystems"])), "Filesystem census changed")
    for path, item in value["filesystems"].items():
        native.require(all(type(item[key]) is int and item[key] >= 0 for key in ("free_bytes", "free_inodes")),
                       "Invalid filesystem observation")
        native.require(item["free_bytes"] >= DISK_RESERVE + (0 if initial else MAX_GROWTH)
                       and item["free_inodes"] >= (50000 if initial else 100000), "Disk or inode reserve reached")
        if initial is not None:
            native.require(initial["filesystems"][path]["free_bytes"] - item["free_bytes"] <= MAX_GROWTH,
                           "Whole-filesystem growth budget reached")


class Guard:
    def __init__(self, paths, unit):
        self.paths, self.started = paths, time.monotonic()
        self.cgroup = cgroup_state()
        self.events = check_cgroup(self.cgroup)
        self.initial = resources(paths)
        check_resources(self.initial)
        self.unit = unit
        self.service = service_state(unit)
        check_service(self.service, unit, self.cgroup)
        self.require_idle()

    def check(self):
        if time.monotonic() - self.started >= TOTAL_SECONDS:
            raise OperationTimeout("Overall operation timeout reached")
        current = cgroup_state()
        native.require(current["path"] == self.cgroup["path"] and check_cgroup(current) == self.events,
                       "Operation cgroup changed or reported OOM")
        value = resources(self.paths)
        check_resources(value, self.initial)
        return value

    def require_idle(self):
        state = service_state(self.unit)
        check_service(state, self.unit, self.cgroup)
        native.require(state == self.service, "Containing service configuration changed")
        native.require(cgroup_processes(self.cgroup["path"]) == {os.getpid()},
                       "The operation cgroup still contains another process; its service must clean up on exit")


def stop_group(process):
    """Only the group created for this child; never a UID or process-name kill."""
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
    if process.poll() is None:
        process.wait(timeout=2)


class Runner:
    def __init__(self, directory, guard, report, env):
        self.directory, self.guard, self.report, self.env = directory, guard, report, env

    def run(self, name, argv, timeout):
        item = {"name": name, "argv": argv, "status": "fail", "started_at": now(),
                "exit_code": None, "timed_out": False, "output_limit_reached": False,
                "timeout_seconds": timeout, "output_limit_bytes": MAX_LOG}
        self.report.setdefault("phases", []).append(item)
        process, failure, total = None, None, 0
        started = time.monotonic()
        paths = {key: self.directory / (name + "." + key) for key in ("stdout", "stderr")}
        try:
            self.guard.check()
            self.guard.require_idle()
            with paths["stdout"].open("xb") as out, paths["stderr"].open("xb") as err, selectors.DefaultSelector() as selector:
                process = subprocess.Popen(argv, env=self.env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                           stderr=subprocess.PIPE, start_new_session=True)
                for pipe, destination in ((process.stdout, out), (process.stderr, err)):
                    os.set_blocking(pipe.fileno(), False)
                    selector.register(pipe, selectors.EVENT_READ, destination)
                while selector.get_map() or process.poll() is None:
                    self.guard.check()
                    if time.monotonic() - started >= timeout:
                        item["timed_out"] = True
                        raise OperationTimeout(name + " timed out")
                    for selected, _ in selector.select(timeout=0.2):
                        data = os.read(selected.fileobj.fileno(), 65536)
                        if not data:
                            selector.unregister(selected.fileobj)
                            selected.fileobj.close()
                            continue
                        count = min(len(data), MAX_LOG - total)
                        selected.data.write(data[:count])
                        total += count
                        if count < len(data):
                            item["output_limit_reached"] = True
                            raise base.BuildError(name + " exceeded its output bound")
                native.require(inspection.zero(process.wait(timeout=1)), name + " failed")
                out.flush()
                err.flush()
                self.guard.check()
                self.guard.require_idle()
                item["status"] = "pass"
        except BaseException as error:
            failure = error
            item["timed_out"] = isinstance(error, (OperationTimeout, subprocess.TimeoutExpired))
            item["error"] = str(error) or type(error).__name__
        finally:
            if process is not None:
                try:
                    stop_group(process)
                except Exception as error:
                    failure = failure or error
                    item.update(status="fail", cleanup_error=str(error))
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()
                item["exit_code"] = process.returncode
            item["finished_at"] = now()
            for key, path in paths.items():
                if path.exists():
                    item[key + "_sha256"] = base.sha256(path)
        if failure is not None:
            raise failure
        return item


INSPECT_CODE = """
import json, pathlib, sys
sys.path.insert(0, sys.argv[1])
import inspect_puredarwin_cross_toolchain as cross
data, _ = cross.native.read_json(pathlib.Path(sys.argv[2]), sys.argv[3])
report = {}
try:
    cross.inspect_cross_toolchain_output(data['candidate'], data['dependencies'], pathlib.Path(sys.argv[4]), report)
finally:
    print(json.dumps(report))
"""

PLANNER_CODE = """
import runpy, sys
sys.path.insert(0, sys.argv.pop(1))
sys.argv = sys.argv[1:]
runpy.run_path(sys.argv[0], run_name='__main__')
"""


def fresh_plan_argv(args, plan, directory):
    prior = plan["successful_native_linker"]
    return [sys.executable, "-I", "-B", "-c", PLANNER_CODE, str(SCRIPT.parent),
            str(Path(planner.__file__).resolve()), "--stage", "cross-toolchain",
            "--puredarwin-source", str(args.puredarwin_source),
            "--native-linker-plan", str(args.native_linker_plan), "--native-linker-plan-sha256", prior["plan_sha256"],
            "--native-linker-build-result", str(args.native_linker_build_result),
            "--native-linker-build-result-sha256", prior["build_result_sha256"],
            "--native-linker-driver-sha256", args.native_linker_driver_sha256,
            "--expected-sdk-output", plan["sdk"]["out"], "--expected-sdk-derivation", plan["sdk"]["drv"],
            "--output-dir", str(directory)]


def realize_argv(candidate, executable, output):
    value = planned_commands(candidate, output)["realize"]
    value[0] = str(executable)
    # An offline failure is evidence that the proposed small stage is not ready.
    # Do not silently fetch a missing dependency or broaden the approved budget.
    return value[:-2] + ["--offline", "--option", "substitute", "false"] + value[-2:]


def execute(args, directory, report):
    native.require(base.platform.system() == "Linux" and base.platform.machine() == "x86_64",
                   "An x86_64 Linux build host is required")
    native.require(os.geteuid() != 0, "Use the existing unprivileged build owner")
    native.require(os.environ.get("NIX_REMOTE", "") in ("", "local"), "A direct local Nix store is required")
    guard = Guard([cross.STORE, directory], args.service_unit)
    report.update(initial_resources=guard.initial, cgroup_at_start=guard.cgroup,
                  containing_service_at_start=guard.service, reserve_bytes=DISK_RESERVE,
                  maximum_filesystem_growth_bytes=MAX_GROWTH,
                  growth_scope="net free-byte reduction on each observed filesystem",
                  automatic_stop_guard="not_checked_by_this_helper")
    manifest, raw = reviewed_code(args.reviewed_code, args.reviewed_code_sha256)
    (directory / "reviewed-code.json").write_bytes(raw)
    plan, raw = read_record(args.cross_plan, args.cross_plan_sha256)
    (directory / "cross-plan.json").write_bytes(raw)
    validate_plan(plan, manifest["sha256"], args.native_linker_driver_sha256, args.cross_plan.parent)
    args.puredarwin_source = args.puredarwin_source.resolve(strict=True)
    executable = args.nix_executable.resolve(strict=True)
    native.require(stat.S_ISREG(executable.lstat().st_mode) and os.access(executable, os.X_OK)
                   and base.sha256(executable) == checksum(args.nix_sha256), "Reviewed Nix executable changed")
    temporary = new_directory(directory / "tmp")
    env = child_environment(executable, temporary)
    with controlled_environment(env):
        execute_checked(args, directory, report, manifest, plan, executable, guard, env)


def execute_checked(args, directory, report, manifest, plan, executable, guard, env):
    guard.check()
    source = base.source_identity(args.puredarwin_source)
    native.require(source == plan["source"], "Selected source or immutable lock changed")
    revision = base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip()
    native.require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None and
                   base.command(["git", "-C", str(base.ROOT), "status", "--porcelain", "--untracked-files=no"]).strip() == "",
                   "Use a committed, clean helper checkout")
    report.update(plan_result_sha256=args.cross_plan_sha256, plan_burdettie_revision=plan["burdettie_revision"],
                  burdettie_revision=revision, reviewed_code_sha256=args.reviewed_code_sha256,
                  helper_sha256=manifest["sha256"][SCRIPT.name], source=source, candidate=plan["candidate"],
                  successful_native_linker=plan["successful_native_linker"], nix_executable=str(executable),
                  nix_sha256=args.nix_sha256)
    runner = Runner(directory, guard, report, env)

    def unchanged():
        reviewed_code(args.reviewed_code, args.reviewed_code_sha256)
        native.require(base.sha256(executable) == args.nix_sha256, "Nix executable changed during the operation")
        native.require(base.source_identity(args.puredarwin_source) == source, "Selected source changed during the operation")
        native.require(base.command(["git", "-C", str(base.ROOT), "rev-parse", "HEAD"]).strip() == revision
                       and base.command(["git", "-C", str(base.ROOT), "status", "--porcelain", "--untracked-files=no"]).strip() == "",
                       "Helper checkout changed during the operation")
        read_record(args.cross_plan, args.cross_plan_sha256)
        read_record(args.native_linker_plan, plan["successful_native_linker"]["plan_sha256"])
        read_record(args.native_linker_build_result, plan["successful_native_linker"]["build_result_sha256"])
        guard.check()
        guard.require_idle()

    report["status"] = "fail"
    try:
        report["nix_version"] = base.command([str(executable), "--version"], env=env).strip()
        native.require(report["nix_version"] == plan["nix_version"], "Actual Nix version differs from the frozen plan")
        fresh_dir = directory / "fresh-plan"
        runner.run("fresh-plan", fresh_plan_argv(args, plan, fresh_dir), PHASE_SECONDS["fresh-plan"])
        fresh, _ = read_record(fresh_dir / "result.json", base.sha256(fresh_dir / "result.json"))
        validate_plan(fresh, manifest["sha256"], args.native_linker_driver_sha256, fresh_dir)
        compare_plans(plan, fresh)
        report["fresh_plan_result_sha256"] = base.sha256(fresh_dir / "result.json")
        unchanged()
        if not args.realize:
            report["status"] = READY
            return
        link = directory / "result-cross-toolchain"
        native.require(not link.exists() and not link.is_symlink(), "Refusing an existing output link")
        argv = realize_argv(fresh["candidate"], executable, directory)
        report["realization"] = {"argv": argv, "out_link": str(link), "out_link_was_absent": True,
                                 "nix_executable": str(executable), "offline": True,
                                 "rebuild_requested": False,
                                 "candidate_preexisting": Path(fresh["candidate"]["out"]).exists()}
        report["cross_toolchain_realization"] = "attempted"
        runner.run("cross-toolchain-build", argv, PHASE_SECONDS["cross-toolchain-build"])
        built = json.loads((directory / "cross-toolchain-build.stdout").read_bytes())
        native.require(isinstance(built, list) and len(built) == 1 and
                       built[0]["drvPath"] == fresh["candidate"]["drv"] and
                       built[0]["outputs"] == {"out": fresh["candidate"]["out"]}, "Realization selected another output")
        native.require(link.is_symlink() and os.readlink(link) == fresh["candidate"]["out"],
                       "Realization did not create its exact planned output link")
        report["realization_command_succeeded"] = True
        report["cross_toolchain_realization"] = "succeeded"
        unchanged()
        selected = fresh["current_native_inspection"]
        inputs = {"candidate": fresh["candidate"], "dependencies": {
                  "native_linker": selected["registered_output"], "sdk": selected["dependencies"]["sdk"]}}
        input_path = directory / "inspection-inputs.json"
        input_path.write_text(json.dumps(inputs, sort_keys=True) + "\n")
        report["cross_toolchain_output_inspection_status"] = "fail"
        runner.run("cross-output-inspection", [sys.executable, "-I", "-B", "-c", INSPECT_CODE,
                   str(SCRIPT.parent), str(input_path), base.sha256(input_path), str(directory / "inspection")],
                   PHASE_SECONDS["cross-output-inspection"])
        inspected_path = directory / "inspection/inspection.json"
        inspected, _ = read_record(inspected_path, base.sha256(inspected_path))
        native.require(inspected["schema_version"] == 1 and inspected["scope"] == cross.INSPECTION_SCOPE
                       and inspected["status"] == "pass" and
                       inspected["registered_output"]["path"] == fresh["candidate"]["out"] and
                       inspected["registered_output"]["deriver"] == fresh["candidate"]["drv"],
                       "Cross output inspection is missing or selects another candidate")
        report.update(output_inspection=inspected, output_inspection_sha256=base.sha256(inspected_path),
                      realized_output=fresh["candidate"]["out"], cross_toolchain_output_inspection_status="pass")
        unchanged()
        report["status"] = SUCCESS
    finally:
        if isinstance(sys.exc_info()[1], (OperationInterrupted, KeyboardInterrupt)):
            # Return promptly so the verified service can kill detached children.
            report["final_validation"] = "interrupted_not_completed"
        else:
            report["final_resources"] = guard.check()
            report["cgroup_at_finish"] = cgroup_state()
            unchanged()


def publish_result(directory, report):
    # No terminal name exists until a complete file has flushed and closed.
    # Retain the staging name as evidence; link creation is atomic and exclusive.
    temporary = directory / "result.json.partial"
    with temporary.open("x") as stream:
        stream.write(json.dumps(report, indent=2) + "\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.link(temporary, directory / "result.json")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("cross-plan", "native-linker-plan", "native-linker-build-result", "puredarwin-source",
                 "nix-executable", "reviewed-code", "output-dir"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("cross-plan-sha256", "native-linker-driver-sha256", "nix-sha256", "reviewed-code-sha256"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--service-unit", required=True, help="Existing dedicated systemd service whose main process is this producer")
    parser.add_argument("--realize", action="store_true", help="Build only the verified candidate, offline and within existing bounds")
    args = parser.parse_args(argv)
    report = {"schema_version": 1, "scope": SCOPE, "status": "blocked", "started_at": now(), "phases": [],
              "cross_toolchain_realization": "not_run", "realization_command_succeeded": False,
              "cross_toolchain_output_inspection_status": "not_run", "dependency_downloads": "not_run",
              "wrapper_execution": "not_run", "compiler_target_qualification": "not_run",
              "compiler_rt_qualification": "not_run", "libsystem_qualification": "not_run",
              "rebuild_execution": "not_established",
              "fixture_linking": "not_run", "guest_execution": "not_run", "security_enforcement": "not_run"}
    directory = None
    previous_signal = signal.signal(signal.SIGTERM, interrupted)
    try:
        directory = new_directory(args.output_dir)
        execute(args, directory, report)
    except (Exception, KeyboardInterrupt) as error:
        report["status"] = "fail" if report["phases"] else "blocked"
        report["error"] = str(error) or type(error).__name__
    report["finished_at"] = now()
    try:
        if directory is not None:
            publish_result(directory, report)
    except (OSError, OperationInterrupted, KeyboardInterrupt) as error:
        report["status"] = "fail"
        report["error"] = "Could not retain terminal receipt: " + str(error)
    finally:
        signal.signal(signal.SIGTERM, previous_signal)
    print(json.dumps(report, indent=2))
    return 0 if report["status"] in (SUCCESS, READY) else (1 if report["status"] == "fail" else 2)


if __name__ == "__main__":
    raise SystemExit(main())
