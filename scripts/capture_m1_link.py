#!/usr/bin/env python3
"""Observe one reviewed direct ld64 invocation and retain verifiable evidence.

No shell, compiler driver, network setup, package install, or automatic retry.
This is for trusted tools in a disposable/local test environment or an already
qualified operation service. It does not establish shared-host containment,
producer authenticity, source/SDK/Nix provenance, or selected-library readiness.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import selectors
import resource
import signal
import subprocess
import time

import inspect_puredarwin_compiler_rt as archive
import verify_compiler_rt_link as verifier
from m1_evidence import (EvidenceError, digest, encoded, exact_keys, hash_value,
                         load_json, publish, read_regular, require)

MAX_REQUEST = 1024 * 1024
MAX_INPUT = 64 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024
MAX_LOG = 2 * 1024 * 1024
MAX_LINKER = 512 * 1024 * 1024
FILE_OPTIONS = {'-force_load', '-reexport_library', '-weak_library',
                '-needed_library', '-load_hidden', '-order_file',
                '-exported_symbols_list', '-unexported_symbols_list'}
SEARCH_OPTIONS = {'-L', '-F', '-syslibroot', '-rpath'}
LIMITATIONS = {
    'capture_execution_authenticated': False, 'build_provenance_verified': False,
    'transitive_inputs_verified': False, 'selected_compiler_rt_qualified': False,
    'selected_libsystem_qualified': False, 'shared_host_service_qualified': False,
    'guest_execution': 'not_run', 'security_enforcement': 'not_run',
}


def regular_path(value: str) -> Path:
    path = Path(verifier.canonical_path(value))
    require(path.resolve(strict=True) == path, 'input path contains a symlink')
    return path


def explicit_inputs(argv: list[str], cwd: str) -> set[str]:
    """Only call after scan_argv; reject hidden library searches, never guess."""
    paths = set()
    i = 1
    while i < len(argv):
        arg = argv[i]
        require(arg not in SEARCH_OPTIONS and not arg.startswith(('-l', '-L', '-F')),
                'implicit library/search inputs require a separately reviewed expanded command')
        if arg in verifier.ARITY:
            values = argv[i + 1:i + 1 + verifier.ARITY[arg]]
            if arg in FILE_OPTIONS:
                paths.add(verifier.argument_path(values[0], cwd))
            elif arg == '-dylib_file':
                parts = values[0].split(':')
                require(len(parts) == 2 and parts[0].startswith('/') and parts[1],
                        'invalid explicit dylib-file mapping')
                paths.add(verifier.argument_path(parts[1], cwd))
            i += 1 + verifier.ARITY[arg]
        elif arg in verifier.FLAGS:
            i += 1
        else:
            paths.add(verifier.argument_path(arg, cwd))
            i += 1
    return paths


def validate_request(raw: bytes, expected_hash: str, root: Path) -> dict:
    require(digest(raw) == hash_value(expected_hash), 'reviewed request digest mismatch')
    req = load_json(raw, MAX_REQUEST)
    exact_keys(req, {'schema_version', 'scope', 'source_revision', 'input_origin', 'argv',
                     'cwd', 'linker_sha256', 'archive_path', 'archive_inspection',
                     'inputs', 'timeout_seconds'}, 'link request')
    require(type(req['schema_version']) is int and req['schema_version'] == 1
            and req['scope'] == 'm1_direct_link_request'
            and req['source_revision'] == archive.SOURCE_REVISION
            and req['input_origin'] in ('owned_fixture', 'selected_source_build'),
            'invalid request schema, source, or origin')
    require(type(req['timeout_seconds']) is int and 1 <= req['timeout_seconds'] <= 30,
            'link timeout must be an integer from 1 through 30 seconds')
    cwd = regular_path(req['cwd'])
    require(cwd.is_dir(), 'link cwd must already be a canonical directory')
    arpath = str(regular_path(req['archive_path']))
    verifier.scan_argv(req['argv'], str(cwd), arpath, str(root / 'link.map'), str(root / 'linked.dylib'))
    hash_value(req['linker_sha256'])
    require(isinstance(req['inputs'], dict) and 0 < len(req['inputs']) <= 1024,
            'explicit input manifest must be a bounded nonempty object')
    require(set(req['inputs']) == explicit_inputs(req['argv'], str(cwd)),
            'input manifest must match every explicit linker input exactly')
    for name, sha in req['inputs'].items():
        regular_path(name); hash_value(sha)
        require(not Path(name).is_relative_to(root), 'input must not be inside the new output directory')
    exact_keys(req['archive_inspection'], {'path', 'sha256'}, 'archive inspection reference')
    regular_path(req['archive_inspection']['path']); hash_value(req['archive_inspection']['sha256'])
    return req


def inspect_archive_receipt(raw: bytes, arraw: bytes) -> None:
    receipt = load_json(raw, archive.MAX_RECEIPT)
    require(type(receipt.get('schema_version')) is int and receipt['schema_version'] == 2
            and receipt.get('scope') == 'm1_compiler_rt_archive_static_inspection'
            and receipt.get('status') == 'pass' and receipt.get('archive_sha256') == digest(arraw)
            and receipt.get('source_revision') == archive.SOURCE_REVISION
            and receipt.get('target_triple') == 'x86_64-apple-darwin20.4'
            and receipt.get('member_architecture') == 'x86_64', 'archive receipt identity mismatch')
    members = archive.parse_ar(arraw)
    require(type(receipt.get('archive_size')) is int and receipt['archive_size'] == len(arraw)
            and type(receipt.get('member_count')) is int and receipt['member_count'] == len(members)
            and encoded({'members': receipt.get('members')}) == encoded({'members': members}),
            'archive receipt inventory mismatch')
    for key, value in archive.LIMITATIONS.items():
        require(encoded({'v': receipt.get(key)}) == encoded({'v': value}),
                'archive receipt qualification claim mismatch: ' + key)


def tool_identity(spelling: str, expected_hash: str) -> dict:
    invoked = Path(verifier.canonical_path(spelling))
    target = invoked.resolve(strict=True)
    require(os.access(target, os.X_OK), 'selected linker is not executable')
    sha = digest(read_regular(target, MAX_LINKER))
    require(sha == hash_value(expected_hash) and invoked.resolve(strict=True) == target,
            'selected linker digest or symlink target changed')
    # Preserve argv[0]: ld64.lld is commonly a multicall symlink named for its mode.
    return {'invoked_path': str(invoked), 'resolved_path': str(target), 'sha256': sha}


def child_limits() -> None:
    """Constrain only this child, never the caller or a shared service."""
    for kind, ceiling in ((resource.RLIMIT_CORE, 0),
                          (resource.RLIMIT_FSIZE, verifier.MAX_OUTPUT),
                          (resource.RLIMIT_CPU, 30)):
        soft, hard = resource.getrlimit(kind)
        effective = ceiling if hard == resource.RLIM_INFINITY else min(ceiling, hard)
        if soft != resource.RLIM_INFINITY:
            effective = min(effective, soft)
        resource.setrlimit(kind, (effective, effective))


def observe(argv: list[str], cwd: Path, root: Path, timeout: int, phase: dict) -> None:
    """Bound retained pipes; a timeout/signal/overflow is never a zero-exit pass.

    Cleanup targets only the process group created here. Detached descendants
    require the caller's containing service; no hostile-child isolation claim.
    """
    started = time.monotonic()
    phase.update(argv=argv, exit_code=None, timed_out=False, status='fail', logs={})
    streams, buffers = {}, {'stdout': bytearray(), 'stderr': bytearray()}
    failure = None
    process = None
    try:
        with selectors.DefaultSelector() as selector:
            process = subprocess.Popen(argv, cwd=cwd, stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
                preexec_fn=child_limits,
                env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'HOME': str(root)})
            for label in buffers:
                stream = getattr(process, label)
                streams[label] = stream
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ, label)
            while selector.get_map() or process.poll() is None:
                if time.monotonic() - started >= timeout:
                    phase['timed_out'] = True; failure = 'link deadline exceeded'; break
                for key, _ in selector.select(0.02):
                    block = os.read(key.fileobj.fileno(), 65536)
                    if not block:
                        selector.unregister(key.fileobj)
                        continue
                    dest = buffers[key.data]
                    if len(dest) + len(block) > MAX_LOG:
                        dest.extend(block[:MAX_LOG - len(dest)])
                        failure = 'link log bound exceeded'; break
                    dest.extend(block)
                if failure:
                    break
    finally:
        if process is not None:
            # Kill the operation-owned group even after its leader exits so an
            # ordinary descendant cannot keep running beyond this operation.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            phase['exit_code'] = process.wait(timeout=5)
        for stream in streams.values():
            stream.close()
        phase['elapsed_seconds'] = round(time.monotonic() - started, 6)
        for label, content in buffers.items():
            filename = 'link.' + label + '.log'
            publish(root / filename, bytes(content))
            phase['logs'][label] = {'file': filename, 'sha256': digest(content), 'bytes': len(content)}
        phase['logs_truncated'] = failure == 'link log bound exceeded'
    require(not failure, str(failure))
    require(type(phase['exit_code']) is int and phase['exit_code'] == 0,
            'link returned exit ' + str(phase['exit_code']))
    phase['status'] = 'pass'


def run(request_path: Path, request_hash: str, root: Path) -> dict:
    """Create a new evidence directory; always retain a terminal failure when possible."""
    result = {'schema_version': 1, 'scope': 'm1_observed_link_operation',
              'status': 'fail', 'phase': {}, **LIMITATIONS}
    directory = None
    try:
        root = root.absolute()
        require(root.parent.is_dir() and root.parent.resolve(strict=True) == root.parent
                and not root.is_relative_to('/nix/store'), 'invalid new evidence directory parent')
        require(not os.path.lexists(root), 'output directory must be new')
        raw = read_regular(request_path, MAX_REQUEST)
        req = validate_request(raw, request_hash, root)
        root.mkdir(mode=0o700); directory = root
        publish(root / 'request.json', raw)
        result.update(request_sha256=digest(raw), input_origin=req['input_origin'])
        helpers = {name: Path(__file__).with_name(name) for name in
                   ('capture_m1_link.py', 'm1_evidence.py', 'inspect_puredarwin_compiler_rt.py',
                    'verify_compiler_rt_link.py')}
        result['helper_sha256'] = {n: digest(read_regular(p, MAX_REQUEST)) for n, p in helpers.items()}
        identity = tool_identity(req['argv'][0], req['linker_sha256'])
        result['linker'] = identity
        snapshots = {}; total = 0
        snapshot_dir = root / 'inputs'; snapshot_dir.mkdir(mode=0o700)
        for index, (name, expected) in enumerate(sorted(req['inputs'].items())):
            data = read_regular(regular_path(name), MAX_INPUT)
            require(digest(data) == expected, 'explicit linker input digest mismatch: ' + name)
            total += len(data); require(total <= MAX_TOTAL, 'total explicit input size exceeds bound')
            filename = 'inputs/' + str(index) + '.input'
            publish(root / filename, data)
            snapshots[name] = {'sha256': expected, 'snapshot': filename, 'bytes': len(data)}
        result['inputs'] = snapshots
        arraw = read_regular(regular_path(req['archive_path']), MAX_INPUT)
        inspection_ref = req['archive_inspection']
        iraw = read_regular(regular_path(inspection_ref['path']), archive.MAX_RECEIPT)
        require(digest(iraw) == inspection_ref['sha256'], 'archive inspection digest mismatch')
        require(digest(arraw) == req['inputs'][req['archive_path']], 'archive changed after snapshot')
        inspect_archive_receipt(iraw, arraw)
        publish(root / 'archive-inspection.json', iraw)
        # Drift checks detect mistakes/concurrent changes, not same-UID adversaries.
        def recheck():
            require(digest(read_regular(request_path, MAX_REQUEST)) == request_hash, 'request changed')
            require(tool_identity(req['argv'][0], req['linker_sha256']) == identity, 'linker identity changed')
            for name, expected in req['inputs'].items():
                require(digest(read_regular(regular_path(name), MAX_INPUT)) == expected,
                        'explicit linker input changed: ' + name)
            require(read_regular(regular_path(inspection_ref['path']), archive.MAX_RECEIPT) == iraw,
                    'archive inspection changed')
            require({n: digest(read_regular(p, MAX_REQUEST)) for n, p in helpers.items()}
                    == result['helper_sha256'], 'capture helper changed')
        recheck()
        observe(req['argv'], Path(req['cwd']), root, req['timeout_seconds'], result['phase'])
        recheck()
        map_raw = read_regular(root / 'link.map', verifier.MAX)
        output_raw = read_regular(root / 'linked.dylib', verifier.MAX_OUTPUT)
        cap = {'schema_version': 2, 'scope': 'm1_compiler_rt_link_capture',
               'source_revision': archive.SOURCE_REVISION, 'input_origin': req['input_origin'],
               'argv_kind': 'ld64_direct', 'argv': req['argv'], 'cwd': req['cwd'],
               'exit_code': result['phase']['exit_code'], 'timed_out': result['phase']['timed_out'],
               'archive': {'path': req['archive_path'], 'sha256': digest(arraw)},
               'link_map': {'path': str(root / 'link.map'), 'sha256': digest(map_raw)},
               'output': {'path': str(root / 'linked.dylib'), 'sha256': digest(output_raw)}}
        craw = encoded(cap)
        hashes = {'capture': digest(craw), 'archive': digest(arraw), 'inspection': digest(iraw),
                  'map': digest(map_raw), 'output': digest(output_raw)}
        publish(root / 'link-capture.json', craw)
        checked = verifier.verify(craw, map_raw, arraw, iraw, output_raw, hashes)
        recheck()
        require(read_regular(root / 'link.map', verifier.MAX) == map_raw
                and read_regular(root / 'linked.dylib', verifier.MAX_OUTPUT) == output_raw,
                'link output/map changed during inspection')
        publish(root / 'link-verification.json', encoded(checked))
        result.update(status='observed_link_and_selection_checked', evidence_sha256=hashes,
                      selected_members=checked['selected_members'])
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        result.update(status='fail', error=str(exc))
    if directory is not None:
        try:
            publish(directory / 'result.json', encoded(result))
        except (OSError, ValueError) as exc:
            result.update(status='fail', error='terminal evidence retention failed: ' + str(exc))
    return result


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--request', required=True, type=Path)
    parser.add_argument('--request-sha256', required=True)
    parser.add_argument('--output-dir', required=True, type=Path)
    args = parser.parse_args(argv)
    def terminate(signum, frame):
        raise EvidenceError('capture interrupted by signal ' + str(signum))
    prior = signal.signal(signal.SIGTERM, terminate)
    try:
        result = run(args.request, args.request_sha256, args.output_dir)
    finally:
        signal.signal(signal.SIGTERM, prior)
    print(encoded(result).decode(), end='')
    return 0 if result['status'] == 'observed_link_and_selection_checked' else 1


if __name__ == '__main__':
    raise SystemExit(main())
