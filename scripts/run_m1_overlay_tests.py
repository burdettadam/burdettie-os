#!/usr/bin/env python3
"""Run the handoff's tests; a skipped or undiscovered group is a failure."""
from pathlib import Path
import sys
import unittest

TEST_FILES = (
    'test_inspect_puredarwin_dyld.py',
    'test_dyld_layout_regressions.py',
    'test_inspect_puredarwin_compiler_rt.py',
    'test_verify_compiler_rt_link.py',
    'test_compiler_rt_regressions.py',
    'test_m1_evidence.py',
    'test_probe_m1_local_toolchain.py',
    'test_capture_m1_link.py',
    'test_probe_m1_captured_link.py',
    'test_m1_compiler_rt_abi.py',
    'test_build_native_fixtures.py',
    'test_m1_thread_state_probe.py',
)


def main() -> int:
    directory = Path(__file__).resolve().parent / 'tests'
    suite = unittest.TestSuite()
    for pattern in TEST_FILES:
        found = unittest.TestLoader().discover(str(directory), pattern=pattern)
        if found.countTestCases() == 0:
            print('Required test group was not discovered: ' + pattern, file=sys.stderr)
            return 1
        suite.addTests(found)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.skipped:
        print('Skipped tests do not qualify this handoff. Install/identify the required trusted tools first.', file=sys.stderr)
        return 1
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
