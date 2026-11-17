#!/usr/bin/env python3
"""Exercise installed tools with tiny source-owned Mach-O fixtures, not PureDarwin.

No download, installation, AWS access, selected build, guest execution, fallback
linker or metadata stripping is performed. A failed real link remains a failure.
Run only already trusted host tools. This is not the qualified shared-host build
service; deadlines target owned process groups, not detached untrusted children.
"""
from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import signal
import stat
import subprocess
import sys
import time

import inspect_puredarwin_compiler_rt as archive
import verify_compiler_rt_link as link
from m1_evidence import EvidenceError, digest, encoded, publish, read_regular, require

MAX_LOG = 2 * 1024 * 1024
SOURCES = {
    'entry.c': 'extern int owned_mul(int, int); int owned_entry(void) { return owned_mul(6, 7); }\n',
    'mul.c': 'int owned_mul(int a, int b) { return a * b; }\n',
    'os_version_check.c': 'int owned_unused_availability(void) { return 0; }\n',
}
LIMITATIONS = {
    'selected_source_build': 'not_run', 'selected_compiler_rt_qualified': False,
    'selected_libsystem_qualified': False, 'selected_dyld_qualified': False,
    'guest_execution': 'not_run', 'security_enforcement': 'not_run',
    'expectation_origin': 'owned_smoke_generated_not_independently_reviewed',
    'shared_host_service_qualified': False,
}


def tool_identity(spelling: str) -> dict:
    found = shutil.which(spelling)
    require(found, 'required installed tool is unavailable: ' + spelling)
    invoked = Path(found).absolute()
    target = invoked.resolve(strict=True)
    before = target.stat()
    require(stat.S_ISREG(before.st_mode) and 0 < before.st_size <= 512 * 1024 * 1024
            and os.access(target, os.X_OK), 'tool must be a bounded executable file')
    h = hashlib.sha256()
    with target.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    after = target.stat()
    fields = ('st_dev', 'st_ino', 'st_size', 'st_mtime_ns', 'st_ctime_ns')
    require(all(getattr(before, f) == getattr(after, f) for f in fields)
            and invoked.resolve(strict=True) == target, 'tool changed while fingerprinting')
    # Preserve invoked basename: ld64.lld is a multicall symlink to lld.
    return {'invoked_path': str(invoked), 'resolved_path': str(target),
            'sha256': h.hexdigest(), 'size_bytes': before.st_size}


def run_phase(argv: list[str], *, cwd: Path, directory: Path, name: str,
              report: dict, timeout: float = 15.0) -> dict:
    out, err = directory / (name + '.stdout.log'), directory / (name + '.stderr.log')
    record = {'name': name, 'argv': argv, 'exit_code': None, 'timed_out': False,
              'status': 'fail', 'stdout_file': out.name, 'stderr_file': err.name}
    report['phases'].append(record)
    started = time.monotonic()
    with out.open('xb') as stdout, err.open('xb') as stderr:
        env = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'HOME': str(cwd)}
        process = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                   stdout=stdout, stderr=stderr, start_new_session=True)
        failure = None
        try:
            while process.poll() is None:
                if time.monotonic() - started >= timeout:
                    record['timed_out'] = True; failure = 'phase deadline exceeded'; break
                if out.stat().st_size > MAX_LOG or err.stat().st_size > MAX_LOG:
                    failure = 'phase log bound exceeded'; break
                time.sleep(0.02)
        finally:
            if process.poll() is None:
                try: os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError: pass
            process.wait(timeout=5)
        record['exit_code'] = process.returncode
        record['elapsed_seconds'] = round(time.monotonic() - started, 6)
        stdout.flush(); stderr.flush()
    for label, path in [('stdout', out), ('stderr', err)]:
        size = path.stat().st_size
        # Logs are already retained even when a producer exceeded the bound.
        record[label + '_bytes'] = size
        if size <= MAX_LOG:
            record[label + '_sha256'] = digest(path.read_bytes())
        else:
            failure = 'phase log bound exceeded'
    require(not failure, name + ': ' + str(failure))
    require(process.returncode == 0, name + ' returned exit ' + str(process.returncode))
    record['status'] = 'pass'
    return record


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--clang', default='clang')
    parser.add_argument('--ar', default='llvm-ar')
    parser.add_argument('--linker', default='ld64.lld')
    args = parser.parse_args(argv)
    result = {'schema_version': 1, 'scope': 'owned_m1_toolchain_probe', 'status': 'fail',
              'input_origin': 'owned_fixture', 'phases': [], **LIMITATIONS}
    directory = None
    try:
        root = args.output_dir.absolute()
        require(root.parent.is_dir() and root.parent.resolve(strict=True) == root.parent,
                'new output directory requires an existing canonical parent')
        root.mkdir(mode=0o700, exist_ok=False); directory = root
        helper_paths = {name: Path(__file__).with_name(name) for name in
                        ('m1_evidence.py', 'inspect_puredarwin_compiler_rt.py',
                         'verify_compiler_rt_link.py', 'probe_m1_local_toolchain.py')}
        result['helper_sha256'] = {name: digest(read_regular(path, 2 * 1024 * 1024)) for name, path in helper_paths.items()}
        tools = {key: tool_identity(value) for key, value in [('clang', args.clang), ('ar', args.ar), ('linker', args.linker)]}
        result['tools'] = tools
        for name, identity in tools.items():
            run_phase([identity['invoked_path'], '--version'], cwd=root, directory=root,
                      name=name + '-version', report=result)
        for name, source in SOURCES.items():
            path = root / name; path.write_text(source)
            run_phase([tools['clang']['invoked_path'], '--target=x86_64-apple-macosx26.5',
                       '-ffreestanding', '-fno-stack-protector', '-c', str(path), '-o', str(path.with_suffix('.o'))],
                      cwd=root, directory=root, name='compile-' + path.stem, report=result)
        arpath, mappath, outpath = root / 'libcompiler_rt.a', root / 'link.map', root / 'libowned_fixture.dylib'
        run_phase([tools['ar']['invoked_path'], '--format=darwin', 'rcs', str(arpath),
                   str(root / 'mul.o'), str(root / 'os_version_check.o')],
                  cwd=root, directory=root, name='archive', report=result)
        raw = read_regular(arpath, archive.MAX_ARCHIVE)
        exp = encoded({'schema_version': 1, 'scope': 'm1_compiler_rt_expectations',
             'source_revision': archive.SOURCE_REVISION, 'target_triple': 'x86_64-apple-darwin20.4',
             'archive_sha256': digest(raw), 'member_names': ['mul.o', 'os_version_check.o']})
        publish(root / 'owned-expectations.json', exp)
        inspection = archive.inspect_archive(raw, exp, digest(exp))
        ir = encoded(inspection); publish(root / 'archive-inspection.json', ir)
        result['archive_inspection'] = {'status': 'pass', 'receipt_sha256': digest(ir),
                                        'archive_sha256': digest(raw), 'member_count': inspection['member_count']}
        command = [tools['linker']['invoked_path'], '-dylib', '-arch', 'x86_64', '-platform_version',
                   'macos', '26.5', '26.5', '-o', str(outpath), str(root / 'entry.o'), str(arpath), '-map', str(mappath)]
        phase = run_phase(command, cwd=root, directory=root, name='link', report=result)
        map_raw, output_raw = read_regular(mappath, link.MAX), read_regular(outpath, link.MAX_OUTPUT)
        cap = encoded({'schema_version': 2, 'scope': 'm1_compiler_rt_link_capture',
            'source_revision': link.SOURCE_REVISION, 'input_origin': 'owned_fixture', 'argv_kind': 'ld64_direct',
            'argv': command, 'cwd': str(root), 'exit_code': phase['exit_code'], 'timed_out': phase['timed_out'],
            'archive': {'path': str(arpath), 'sha256': digest(raw)},
            'link_map': {'path': str(mappath), 'sha256': digest(map_raw)},
            'output': {'path': str(outpath), 'sha256': digest(output_raw)}})
        publish(root / 'link-capture.json', cap)
        expected = {'capture': digest(cap), 'map': digest(map_raw), 'archive': digest(raw),
                    'inspection': digest(ir), 'output': digest(output_raw)}
        checked = link.verify(cap, map_raw, raw, ir, output_raw, expected)
        publish(root / 'link-verification.json', encoded(checked))
        for key, identity in tools.items():
            require(tool_identity(identity['invoked_path']) == identity, 'tool identity changed during probe: ' + key)
        require({name: digest(read_regular(path, 2 * 1024 * 1024)) for name, path in helper_paths.items()}
                == result['helper_sha256'], 'helper source changed during probe')
        result['status'] = 'owned_archive_and_link_checked'
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        result.update(status='fail', error=str(error))
    if directory is not None:
        try:
            publish(directory / 'result.json', encoded(result))
        except (OSError, ValueError) as error:
            result.update(status='fail', error='terminal receipt retention failed: ' + str(error))
    print(encoded(result).decode(), end='')
    return 0 if result['status'] == 'owned_archive_and_link_checked' else 1


if __name__ == '__main__':
    raise SystemExit(main())
