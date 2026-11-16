#!/usr/bin/env python3
"""Build an owned Darwin arithmetic object; optionally execute a Linux host check.

Uses only installed, trusted tools and a caller-hashed host builtins archive.
Never executes the Darwin output, downloads tools, or runs a selected Nix build.
Host checks are preparatory evidence, not selected PureDarwin ABI qualification.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import platform
import re
import signal
import struct
import subprocess

import capture_m1_link as execution
import inspect_puredarwin_compiler_rt as macho
import m1_compiler_rt_vectors as vectors
from m1_evidence import (EvidenceError, digest, encoded, exact_keys, hash_value,
                         load_json, publish, read_regular, require)
from probe_m1_local_toolchain import tool_identity

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'fixtures' / 'm1_compiler_rt_probe'
SOURCE_NAMES = ('probe.h', 'probe.c', 'main.c')
SYMBOLS = ('__divti3', '__modti3', '__udivti3', '__umodti3', '__fixdfti', '__floattidf')
MAX_SOURCE = 2 * 1024 * 1024
MAX_ARCHIVE = 64 * 1024 * 1024
COMMON_FLAGS = ['-std=c11', '-O2', '-Wall', '-Wextra', '-Werror', '-fno-builtin',
                '-fno-stack-protector', '-fno-lto']
LIMITATIONS = {
    'selected_source_build': 'not_run', 'selected_compiler_rt_qualified': False,
    'selected_libsystem_qualified': False, 'selected_runtime_abi_qualified': False,
    'darwin_execution': 'not_run', 'guest_boot': 'not_run',
    'security_enforcement': 'not_run', 'sdk_provenance_verified': False,
    'host_builtins_source_provenance_verified': False,
    'host_transitive_inputs_verified': False, 'shared_host_service_qualified': False,
}


def check_target_object(raw: bytes) -> dict:
    result = macho.inspect_member('probe.o', raw, '100644')
    require(result['cpu_subtype'] == 3 and result['symbol_table_present'],
            'expected baseline x86_64 object with symbol table')
    require(result['defined_symbols'] == ['_m1_compiler_rt_probe']
            and result['undefined_symbols'] == sorted('_' + s for s in SYMBOLS)
            and not result['common_symbols'] and not result['indirect_symbols']
            and not result['private_external_symbols'],
            'Darwin probe must import exactly the six arithmetic helpers')
    require(result['platform_versions'] == [{'platform_id': 1, 'minimum_os': '26.5.0',
                                             'sdk': '0.0.0'}],
            'freestanding Darwin object must retain macOS 26.5 / no-SDK metadata')
    return result


def validate_execution(raw: bytes, exit_code: int, expected_counts: dict,
                       expected_platform: str = 'linux') -> dict:
    """Bind every category count and observed exit; never infer a pass from JSON alone."""
    require(type(exit_code) is int and exit_code == 0, 'arithmetic executable did not exit zero')
    report = load_json(raw, 8192)
    exact_keys(report, {'schema_version', 'scope', 'platform', 'architecture', 'rounding', 'cases'},
               'arithmetic report')
    require(type(report['schema_version']) is int and report['schema_version'] == 1
            and report['scope'] == 'm1_compiler_rt_arithmetic_execution'
            and report['platform'] == expected_platform and report['architecture'] == 'x86_64'
            and report['rounding'] == 'nearest_even', 'execution platform or schema mismatch')
    exact_keys(expected_counts, set(vectors.CATEGORIES), 'expected case counts')
    exact_keys(report['cases'], set(vectors.CATEGORIES), 'case categories')
    for name, expected in expected_counts.items():
        require(type(expected) is int and expected > 0, 'invalid expected case count')
        item = report['cases'][name]
        exact_keys(item, {'checked', 'failed', 'first_failure'}, 'category result')
        require(all(type(item[k]) is int for k in item), 'case counts must be integers, not booleans')
        require(item['checked'] == expected, 'executed case count differs: ' + name)
        require(item['failed'] == 0 and item['first_failure'] == 0,
                'arithmetic mismatch: ' + name)
    return report


def check_host_elf(raw: bytes) -> None:
    require(len(raw) >= 64 and raw[:7] == b'\x7fELF\x02\x01\x01',
            'refuse to execute non-ELF64 little-endian host output')
    kind, cpu = struct.unpack_from('<HH', raw, 16)
    require(kind in (2, 3) and cpu == 62, 'refuse to execute non-x86_64 host output')


def check_symbol_origins(raw: bytes, archive_path: Path) -> dict:
    """GNU ld trace supplies exact defining archive spellings; it is not authentication."""
    text = raw.decode('utf-8', 'strict')
    selected = {}
    for symbol in SYMBOLS:
        suffix = ': definition of ' + symbol
        definitions = [line[:-len(suffix)] for line in text.splitlines() if line.endswith(suffix)]
        require(len(definitions) == 1, 'missing or duplicate defining trace: ' + symbol)
        match = re.fullmatch(r'[^\n]+: ' + re.escape(str(archive_path))
                             + r'\(([A-Za-z0-9_.+-]{1,255})\)', definitions[0])
        require(match is not None and match.group(1) not in ('.', '..'),
                'arithmetic helper defined by an unexpected input: ' + symbol)
        selected[symbol] = match.group(1)
    return selected


def run(output: Path, clang: str, *, host_builtins: Path | None = None,
        host_builtins_sha256: str | None = None, host_linker: str = 'ld',
        run_host: bool = False) -> dict:
    report = {'schema_version': 1, 'scope': 'owned_m1_compiler_rt_abi_probe', 'status': 'fail',
              'input_origin': 'owned_fixture', 'host_execution': 'not_run', 'phases': [], **LIMITATIONS}
    directory = None
    try:
        require(type(run_host) is bool, 'run_host must be a boolean')
        require(run_host == (host_builtins is not None and host_builtins_sha256 is not None)
                and (run_host or (host_builtins is None and host_builtins_sha256 is None)),
                'host execution requires both explicit builtins path and digest')
        if run_host:
            require(platform.system() == 'Linux' and platform.machine() == 'x86_64',
                    'host execution lane requires Linux x86_64')
        output = output.absolute()
        require(not os.path.lexists(output) and output.parent.is_dir()
                and output.parent.resolve(strict=True) == output.parent
                and not output.is_relative_to('/nix/store'), 'output must be a new permitted directory')
        source_bytes = {n: read_regular(FIXTURE / n, MAX_SOURCE) for n in SOURCE_NAMES}
        archive_raw = None
        if run_host:
            require(host_builtins.resolve(strict=True) == host_builtins.absolute(),
                    'host builtins path must not contain symlinks')
            archive_raw = read_regular(host_builtins, MAX_ARCHIVE)
            require(digest(archive_raw) == hash_value(host_builtins_sha256)
                    and archive_raw.startswith(b'!<arch>\n'), 'host builtins archive digest/format mismatch')
        tools = {'clang': tool_identity(clang)}
        if run_host:
            tools['host_linker'] = tool_identity(host_linker)
        output.mkdir(mode=0o700); directory = output
        sources = output / 'sources'; sources.mkdir(mode=0o700)
        phases = output / 'phases'; phases.mkdir(mode=0o700)
        data = vectors.corpus(); expected_counts = vectors.counts(data)
        generated = vectors.header(data)
        for name, raw in source_bytes.items():
            publish(sources / name, raw)
        publish(sources / 'vectors.h', generated)
        publish(output / 'vectors.json', encoded(data))
        report.update(tools=tools, expected_counts=expected_counts,
            expected_total=sum(expected_counts.values()), vectors_sha256=digest(generated),
            source_sha256={n: digest(b) for n, b in source_bytes.items()})
        helpers = {n: Path(__file__).with_name(n) for n in ('probe_m1_compiler_rt_abi.py',
            'm1_compiler_rt_vectors.py', 'm1_evidence.py', 'capture_m1_link.py',
            'probe_m1_local_toolchain.py', 'inspect_puredarwin_compiler_rt.py', 'verify_compiler_rt_link.py')}
        report['helper_sha256'] = {n: digest(read_regular(p, MAX_SOURCE)) for n, p in helpers.items()}
        host_archive = output / 'host-builtins.a'
        if archive_raw is not None:
            publish(host_archive, archive_raw)
            report['host_archive'] = {'input_path': str(host_builtins.absolute()),
                'sha256': digest(archive_raw), 'bytes': len(archive_raw), 'snapshot': host_archive.name}

        def recheck() -> None:
            require(all(read_regular(FIXTURE / n, MAX_SOURCE) == b
                        and read_regular(sources / n, MAX_SOURCE) == b for n, b in source_bytes.items()),
                    'fixture source or snapshot changed')
            require(read_regular(sources / 'vectors.h', MAX_SOURCE) == generated, 'reference vectors changed')
            require(read_regular(output / 'vectors.json', MAX_SOURCE) == encoded(data),
                    'reference corpus snapshot changed')
            require({n: digest(read_regular(p, MAX_SOURCE)) for n, p in helpers.items()}
                    == report['helper_sha256'], 'probe helper changed')
            require(all(tool_identity(i['invoked_path']) == i for i in tools.values()), 'probe tool changed')
            if archive_raw is not None:
                require(read_regular(host_builtins, MAX_ARCHIVE) == archive_raw
                        and read_regular(host_archive, MAX_ARCHIVE) == archive_raw,
                        'host builtins or snapshot changed')

        def phase(name: str, argv: list[str]) -> dict:
            recheck()
            target = phases / name; target.mkdir(mode=0o700)
            value = {'name': name, 'evidence_directory': str(target.relative_to(output))}
            report['phases'].append(value)
            execution.observe(argv, output, target, 30, value)
            recheck()
            return value

        compiler = tools['clang']['invoked_path']
        phase('clang-version', [compiler, '--version'])
        target = output / 'darwin-probe.o'
        phase('darwin-compile', [compiler, '--target=x86_64-apple-macosx26.5', *COMMON_FLAGS,
            '-ffreestanding', '-nostdinc', '-c', str(sources / 'probe.c'), '-o', str(target)])
        target_raw = read_regular(target, MAX_SOURCE)
        checked = check_target_object(target_raw)
        publish(output / 'darwin-object-inspection.json', encoded(checked))
        report['darwin_object'] = {'sha256': digest(target_raw), 'bytes': len(target_raw),
                                  'inspection': 'darwin-object-inspection.json', 'sdk_field': '0.0.0'}
        report['status'] = 'darwin_object_checked_host_not_run'
        if run_host:
            phase('host-linker-version', [tools['host_linker']['invoked_path'], '--version'])
            version = (phases / 'host-linker-version' / 'link.stdout.log').read_text()
            require(version.startswith('GNU ld '), 'native trace qualification requires GNU ld')
            core, main, executable = output / 'host-core.o', output / 'host-main.o', output / 'host-probe'
            phase('host-core-compile', [compiler, *COMMON_FLAGS, '-ffreestanding', '-nostdinc',
                '-c', str(sources / 'probe.c'), '-o', str(core)])
            phase('host-main-compile', [compiler, *COMMON_FLAGS, '-c', str(sources / 'main.c'), '-o', str(main)])
            core_raw, main_raw = read_regular(core, MAX_SOURCE), read_regular(main, MAX_SOURCE)
            report['host_objects'] = {p.name: {'sha256': digest(b), 'bytes': len(b)}
                                      for p, b in ((core, core_raw), (main, main_raw))}
            phase('host-link', [compiler, '-fuse-ld=' + tools['host_linker']['invoked_path'],
                '-nodefaultlibs', str(core), str(main), str(host_archive), '-lc', '-lm',
                '-Wl,-Map,' + str(output / 'host-link.map'),
                '-Wl,' + ','.join('--trace-symbol=' + s for s in SYMBOLS), '-o', str(executable)])
            origin = check_symbol_origins((phases / 'host-link' / 'link.stderr.log').read_bytes(), host_archive)
            map_raw = read_regular(output / 'host-link.map', MAX_SOURCE)
            report['host_link_map_sha256'] = digest(map_raw)
            elf_raw = read_regular(executable, MAX_SOURCE)
            check_host_elf(elf_raw)
            report['host_binary_sha256'] = digest(elf_raw)
            report['host_helper_definitions'] = origin
            report['host_execution'] = 'attempted'
            ran = phase('host-execute', [str(executable)])
            stdout = (phases / 'host-execute' / 'link.stdout.log').read_bytes()
            require((phases / 'host-execute' / 'link.stderr.log').stat().st_size == 0,
                    'host arithmetic run emitted unexpected stderr')
            result = validate_execution(stdout, ran['exit_code'], expected_counts)
            require(read_regular(core, MAX_SOURCE) == core_raw and read_regular(main, MAX_SOURCE) == main_raw
                    and read_regular(executable, MAX_SOURCE) == elf_raw
                    and read_regular(output / 'host-link.map', MAX_SOURCE) == map_raw,
                    'host execution inputs or map changed')
            publish(output / 'host-arithmetic-result.json', encoded(result))
            report.update(host_execution='linux_x86_64_arithmetic_pass',
                          status='darwin_object_and_linux_arithmetic_checked')
        require(read_regular(target, MAX_SOURCE) == target_raw, 'Darwin object changed')
        recheck()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        report.update(status='fail', error=str(error))
    if directory is not None:
        try:
            publish(directory / 'result.json', encoded(report))
        except (OSError, ValueError) as error:
            report.update(status='fail', error='cannot publish terminal evidence: ' + str(error))
    return report


def main(argv=None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output-dir', type=Path, required=True)
    p.add_argument('--clang', default='clang')
    p.add_argument('--run-host', action='store_true', help='execute the newly built Linux x86_64 fixture')
    p.add_argument('--host-builtins', type=Path)
    p.add_argument('--host-builtins-sha256')
    p.add_argument('--host-linker', default='ld')
    a = p.parse_args(argv)
    old = signal.getsignal(signal.SIGTERM)
    def interrupted(_signum, _frame):
        raise EvidenceError('probe interrupted by SIGTERM')
    signal.signal(signal.SIGTERM, interrupted)
    try:
        result = run(a.output_dir, a.clang, run_host=a.run_host, host_builtins=a.host_builtins,
                     host_builtins_sha256=a.host_builtins_sha256, host_linker=a.host_linker)
    finally:
        signal.signal(signal.SIGTERM, old)
    print(encoded(result).decode(), end='')
    return 1 if result['status'] == 'fail' else 0


if __name__ == '__main__':
    raise SystemExit(main())
