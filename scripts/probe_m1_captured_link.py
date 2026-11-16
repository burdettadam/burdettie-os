#!/usr/bin/env python3
"""Exercise the observed-link producer with an owned positive and negative link.

Uses artifacts from probe_m1_local_toolchain.py, then makes another real link
and a deliberate undefined-symbol link. No selected library or guest is built.
The generated requests are test inputs, not independent review attestations.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import subprocess

import capture_m1_link as capture
import inspect_puredarwin_compiler_rt as archive
from m1_evidence import digest, encoded, publish, read_regular, require
from probe_m1_local_toolchain import run_phase, tool_identity

MISSING = '_owned_missing_for_negative_link'
SOURCE = ('extern int owned_missing_for_negative_link(void);\n'
          'int owned_negative_entry(void) { return owned_missing_for_negative_link(); }\n')


def expected_negative(result: dict, directory: Path) -> bool:
    phase = result.get('phase', {})
    require(result.get('status') == 'fail' and type(phase.get('exit_code')) is int
            and phase['exit_code'] > 0 and phase.get('timed_out') is False
            and phase.get('logs_truncated') is False,
            'negative link was not an observed nonzero linker rejection')
    stderr = read_regular(directory / 'link.stderr.log', capture.MAX_LOG)
    require(MISSING.encode() in stderr and b'undefined' in stderr.lower(),
            'negative link failed for another reason, not the missing owned symbol')
    require(digest(stderr) == phase['logs']['stderr']['sha256']
            and not (directory / 'link-capture.json').exists()
            and not (directory / 'link-verification.json').exists(),
            'negative link has inconsistent logs or a successful capture')
    require(read_regular(directory / 'result.json', capture.MAX_REQUEST) == encoded(result),
            'negative-link terminal failure was not retained exactly')
    return True


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input-dir', type=Path, required=True)
    parser.add_argument('--clang', required=True)
    parser.add_argument('--linker', required=True)
    args = parser.parse_args(argv)
    result = {'schema_version': 1, 'scope': 'owned_m1_captured_link_probe', 'status': 'fail',
              'phases': [], 'request_origin': 'generated_owned_fixture_not_independent_review',
              **capture.LIMITATIONS}
    root = None
    try:
        parent = args.input_dir.absolute()
        require(parent.is_dir() and parent.resolve(strict=True) == parent, 'input directory must be canonical')
        candidate = parent / 'captured-links'
        candidate.mkdir(mode=0o700, exist_ok=False); root = candidate
        tools = {key: tool_identity(value) for key, value in [('clang', args.clang), ('linker', args.linker)]}
        result['tools'] = tools
        arpath, object_path, inspection_path = (parent / name for name in
            ('libcompiler_rt.a', 'entry.o', 'archive-inspection.json'))
        inputs = {str(p): digest(read_regular(p, capture.MAX_INPUT)) for p in (arpath, object_path)}
        irhash = digest(read_regular(inspection_path, archive.MAX_RECEIPT))
        def request(output, extra=None):
            files = dict(inputs)
            direct = [str(object_path), str(arpath)]
            if extra is not None:
                direct.append(str(extra)); files[str(extra)] = digest(read_regular(extra, capture.MAX_INPUT))
            return {'schema_version': 1, 'scope': 'm1_direct_link_request',
                'source_revision': archive.SOURCE_REVISION, 'input_origin': 'owned_fixture',
                'argv': [tools['linker']['invoked_path'], '-dylib', '-arch', 'x86_64',
                         '-platform_version', 'macos', '26.5', '26.5', *direct,
                         '-o', str(output / 'linked.dylib'), '-map', str(output / 'link.map')],
                'cwd': str(root), 'linker_sha256': tools['linker']['sha256'], 'archive_path': str(arpath),
                'archive_inspection': {'path': str(inspection_path), 'sha256': irhash},
                'inputs': files, 'timeout_seconds': 15}
        def execute(name, extra=None):
            output = root / name
            raw = encoded(request(output, extra))
            path = root / (name + '-request.json'); publish(path, raw)
            return capture.run(path, digest(raw), output)
        positive = execute('positive')
        result['positive'] = positive
        require(positive['status'] == 'observed_link_and_selection_checked',
                'positive observed link failed: ' + positive.get('error', 'unknown'))
        require(positive['selected_members'] == ['mul.o'], 'positive link selected unexpected members')
        source, obj = root / 'negative.c', root / 'negative.o'
        publish(source, SOURCE.encode())
        run_phase([tools['clang']['invoked_path'], '--target=x86_64-apple-macosx26.5',
                   '-ffreestanding', '-fno-stack-protector', '-c', str(source), '-o', str(obj)],
                  cwd=root, directory=root, name='compile-negative', report=result)
        negative = execute('negative', obj)
        result['negative'] = negative
        expected_negative(negative, root / 'negative')
        for key, identity in tools.items():
            require(tool_identity(identity['invoked_path']) == identity, 'probe tool changed: ' + key)
        result.update(status='owned_positive_and_negative_links_checked',
                      negative_link_rejected=True, guest_execution='not_run')
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as exc:
        result.update(status='fail', error=str(exc))
    if root is not None:
        try:
            publish(root / 'result.json', encoded(result))
        except (OSError, ValueError) as exc:
            result.update(status='fail', error='terminal receipt retention failed: ' + str(exc))
    print(encoded(result).decode(), end='')
    return 0 if result['status'] == 'owned_positive_and_negative_links_checked' else 1


if __name__ == '__main__':
    raise SystemExit(main())
