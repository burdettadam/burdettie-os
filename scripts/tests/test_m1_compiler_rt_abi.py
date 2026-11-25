"""Reference-oracle, ABI fixture, strict-result and no-execution regression tests."""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import m1_compiler_rt_vectors as vectors
import probe_m1_compiler_rt_abi as probe
from m1_evidence import EvidenceError, encoded

CORPUS = vectors.corpus()
COUNTS = vectors.counts(CORPUS)
HEADER = vectors.header(CORPUS)


def success():
    return {'schema_version': 1, 'scope': 'm1_compiler_rt_arithmetic_execution',
        'platform': 'linux', 'architecture': 'x86_64', 'rounding': 'nearest_even',
        'cases': {n: {'checked': count, 'failed': 0, 'first_failure': 0} for n, count in COUNTS.items()}}


class VectorTests(unittest.TestCase):
    def test_corpus_is_deterministic_with_fixed_counts(self):
        self.assertEqual(HEADER, vectors.header(vectors.corpus()))
        self.assertEqual(COUNTS, dict(zip(vectors.CATEGORIES, (2197, 2197, 1058, 1058, 204, 294))))
        self.assertEqual(sum(COUNTS.values()), 7008)

    def test_signed_remainder_truncates_toward_zero(self):
        for a, b, q, r in ((-7, 3, -2, -1), (7, -3, -2, 1), (-7, -3, 2, -1)):
            self.assertEqual(vectors.trunc_divmod(a, b), (q, r))
        self.assertEqual(vectors.trunc_divmod(vectors.MIN_I128, 1), (vectors.MIN_I128, 0))

    def test_undefined_division_and_out_of_range_are_excluded(self):
        for a, b in ((1, 0), (vectors.MIN_I128, -1), (1 << 127, 1), (0, 1 << 127)):
            with self.subTest(a=a, b=b), self.assertRaises(ValueError):
                vectors.trunc_divmod(a, b)

    def test_all_division_vectors_satisfy_exact_integer_invariants(self):
        for a, b, q, r in CORPUS['signed']:
            self.assertEqual(a, b * q + r)
            self.assertLess(abs(r), abs(b))
            self.assertTrue(r == 0 or (r < 0) == (a < 0))
            self.assertTrue(vectors.MIN_I128 <= q <= vectors.MAX_I128)
        for a, b, q, r in CORPUS['unsigned']:
            self.assertTrue(0 <= a <= vectors.MASK and 0 < b <= vectors.MASK)
            self.assertEqual(a, b * q + r)
            self.assertTrue(0 <= r < b)

    def test_float_conversion_corpus_is_finite_in_range_and_contains_signed_zero(self):
        seen = set()
        for bits, integer in CORPUS['double_to_signed']:
            number = struct.unpack('>d', struct.pack('>Q', bits))[0]
            self.assertTrue(math.isfinite(number))
            self.assertTrue(vectors.MIN_I128 <= number < 1 << 127)
            self.assertEqual(integer, math.trunc(number))
            seen.add(bits)
        self.assertTrue({0, 1 << 63, 1, (1 << 63) | 1} <= seen)

    def test_binary64_ties_round_to_even_and_high_bit_values_present(self):
        by_integer = dict(CORPUS['signed_to_double'])
        self.assertEqual(by_integer[(1 << 53) + 1], vectors.binary64_bits(float(1 << 53)))
        self.assertEqual(by_integer[(1 << 53) + 3], vectors.binary64_bits(float((1 << 53) + 4)))
        self.assertIn(vectors.MIN_I128, by_integer)
        self.assertIn(vectors.MAX_I128, by_integer)

    def test_words_use_exact_twos_complement_without_large_signed_literals(self):
        self.assertEqual(vectors.pair(-1), '{0xffffffffffffffffULL, 0xffffffffffffffffULL}')
        self.assertEqual(vectors.signed(vectors.MASK), -1)
        self.assertEqual(vectors.signed(1 << 127), vectors.MIN_I128)
        self.assertTrue(HEADER.endswith(b'#endif\n'))


class ResultTests(unittest.TestCase):
    def test_valid_host_result_does_not_claim_darwin(self):
        r = probe.validate_execution(encoded(success()), 0, COUNTS)
        self.assertEqual(r['platform'], 'linux')
        self.assertFalse(probe.LIMITATIONS['selected_runtime_abi_qualified'])
        self.assertEqual(probe.LIMITATIONS['darwin_execution'], 'not_run')

    def test_actual_nonzero_signal_or_boolean_exit_cannot_reuse_success_body(self):
        for code in (False, True, 1, -9, None, 0.0):
            with self.subTest(code=code), self.assertRaises(EvidenceError):
                probe.validate_execution(encoded(success()), code, COUNTS)

    def test_boolean_counters_and_schema_are_rejected(self):
        for key in ('checked', 'failed', 'first_failure'):
            r = success(); r['cases']['signed_div'][key] = False
            with self.subTest(key=key), self.assertRaises(EvidenceError):
                probe.validate_execution(encoded(r), 0, COUNTS)
        r = success(); r['schema_version'] = True
        with self.assertRaises(EvidenceError): probe.validate_execution(encoded(r), 0, COUNTS)

    def test_incomplete_counts_or_categories_rejected(self):
        r = success(); r['cases']['signed_div']['checked'] -= 1
        with self.assertRaisesRegex(EvidenceError, 'case count'): probe.validate_execution(encoded(r), 0, COUNTS)
        r = success(); del r['cases']['unsigned_mod']
        with self.assertRaises(EvidenceError): probe.validate_execution(encoded(r), 0, COUNTS)

    def test_arithmetic_failure_with_zero_exit_still_fails(self):
        r = success(); r['cases']['signed_div'].update(failed=1, first_failure=3)
        with self.assertRaisesRegex(EvidenceError, 'arithmetic mismatch'):
            probe.validate_execution(encoded(r), 0, COUNTS)

    def test_nonzero_first_failure_and_invalid_expected_count_rejected(self):
        r = success(); r['cases']['signed_div']['first_failure'] = 1
        with self.assertRaises(EvidenceError): probe.validate_execution(encoded(r), 0, COUNTS)
        expected = dict(COUNTS); expected['signed_div'] = True
        with self.assertRaises(EvidenceError): probe.validate_execution(encoded(success()), 0, expected)

    def test_foreign_platform_architecture_or_rounding_rejected(self):
        for key, value in (('platform', 'darwin'), ('architecture', 'arm64'), ('rounding', 'toward_zero')):
            r = success(); r[key] = value
            with self.subTest(key=key), self.assertRaises(EvidenceError):
                probe.validate_execution(encoded(r), 0, COUNTS)

    def test_duplicate_keys_extra_claims_and_trailing_data_rejected(self):
        r = success(); r['guest_boot'] = 'pass'
        for raw in (encoded(r), encoded(success()) + b'{}', b'{"cases":{},"cases":{}}', b'[]'):
            with self.subTest(raw=raw[:60]), self.assertRaises(ValueError):
                probe.validate_execution(raw, 0, COUNTS)

    def test_native_elf_architecture_guard(self):
        raw = bytearray(64); raw[:7] = b'\x7fELF\x02\x01\x01'; struct.pack_into('<HH', raw, 16, 3, 62)
        probe.check_host_elf(bytes(raw))
        for bad in (b'\xcf\xfa\xed\xfe', bytes(64), bytes(raw[:63])):
            with self.assertRaises(EvidenceError): probe.check_host_elf(bad)
        struct.pack_into('<H', raw, 18, 183)
        with self.assertRaises(EvidenceError): probe.check_host_elf(bytes(raw))

    def test_native_definition_trace_requires_exact_archive_and_unique_definitions(self):
        archive = Path('/owned/host-builtins.a')
        raw = ''.join(f'/usr/bin/ld: {archive}({s[2:]}.c.o): definition of {s}\n' for s in probe.SYMBOLS).encode()
        self.assertEqual(len(probe.check_symbol_origins(raw, archive)), 6)
        for bad in (raw.replace(b'/owned/', b'/other/'), raw + raw.splitlines()[0] + b'\n',
                    b'\n'.join(raw.splitlines()[:-1]), raw.replace(b'divti3.c.o', b'../bad.o')):
            with self.subTest(bad=bad[:80]), self.assertRaises(EvidenceError):
                probe.check_symbol_origins(bad, archive)


class ProducerTests(unittest.TestCase):
    def test_existing_output_preserved_before_any_tool(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / 'sentinel').write_bytes(b'keep')
            with mock.patch.object(probe, 'tool_identity') as tool:
                result = probe.run(root, 'clang')
            self.assertEqual(result['status'], 'fail'); tool.assert_not_called()
            self.assertEqual((root / 'sentinel').read_bytes(), b'keep')

    def test_host_lane_requires_explicit_archive_and_digest(self):
        for kwargs in ({'run_host': True}, {'host_builtins': Path('/missing')},
                       {'host_builtins_sha256': '0' * 64}, {'run_host': 1}):
            with self.subTest(kwargs=kwargs), mock.patch.object(probe, 'tool_identity') as tool:
                result = probe.run(Path('/not-created-by-test'), 'clang', **kwargs)
                self.assertEqual(result['status'], 'fail'); tool.assert_not_called()

    def test_archive_digest_or_symlink_failure_precedes_tool_execution(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / 'builtins.a'; archive.write_bytes(b'!<arch>\n')
            link = root / 'link.a'; link.symlink_to(archive)
            for path, sha in ((archive, '0' * 64), (link, hashlib.sha256(archive.read_bytes()).hexdigest())):
                with self.subTest(path=path), mock.patch.object(probe, 'tool_identity') as tool:
                    result = probe.run(root / 'out', 'clang', run_host=True,
                                       host_builtins=path, host_builtins_sha256=sha)
                    self.assertEqual(result['status'], 'fail'); tool.assert_not_called()
                    self.assertFalse((root / 'out').exists())

    def test_non_linux_host_refused_before_any_tool(self):
        with mock.patch.object(probe.platform, 'system', return_value='Darwin'), \
             mock.patch.object(probe, 'tool_identity') as tool:
            result = probe.run(Path('/not-created-by-test'), 'clang', run_host=True,
                               host_builtins=Path('/none'), host_builtins_sha256='0' * 64)
        self.assertEqual(result['status'], 'fail'); tool.assert_not_called()

    def test_target_symbol_platform_and_definition_drift_fail(self):
        record = {'cpu_subtype': 3, 'symbol_table_present': True,
            'defined_symbols': ['_m1_compiler_rt_probe'], 'undefined_symbols': sorted('_' + s for s in probe.SYMBOLS),
            'common_symbols': [], 'indirect_symbols': [], 'private_external_symbols': [],
            'platform_versions': [{'platform_id': 1, 'minimum_os': '26.5.0', 'sdk': '0.0.0'}]}
        with mock.patch.object(probe.macho, 'inspect_member', return_value=record):
            self.assertEqual(probe.check_target_object(b''), record)
        changes = [('undefined_symbols', ['_memset']), ('defined_symbols', []), ('cpu_subtype', 8),
                   ('common_symbols', ['_wrong']), ('platform_versions', []),
                   ('symbol_table_present', False)]
        for key, value in changes:
            bad = deepcopy(record); bad[key] = value
            with self.subTest(key=key), mock.patch.object(probe.macho, 'inspect_member', return_value=bad), \
                 self.assertRaises(EvidenceError):
                probe.check_target_object(b'')

    def test_missing_tool_or_missing_output_are_failures_not_success(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertEqual(probe.run(root / 'out', '/does-not-exist')['status'], 'fail')
            self.assertFalse((root / 'out').exists())


class RealFixtureTests(unittest.TestCase):
    """Uses installed compilers; no fetched target program is executed."""
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.temp.name)
        cls.sources = cls.root / 'sources'; cls.sources.mkdir()
        for name in probe.SOURCE_NAMES:
            (cls.sources / name).write_bytes((probe.FIXTURE / name).read_bytes())
        (cls.sources / 'vectors.h').write_bytes(HEADER)
        cls.cc = shutil.which('cc')
        cls.clang = shutil.which('clang')
        if not cls.cc or not cls.clang:
            raise AssertionError('ABI fixture tests require existing cc and clang; no silent skip')

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def compile_host(self, output: Path, extra=()):
        result = subprocess.run([self.cc, *probe.COMMON_FLAGS, str(self.sources / 'probe.c'),
            str(self.sources / 'main.c'), *map(str, extra), '-lm', '-o', str(output)],
            capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr.decode())

    def test_real_host_fixture_runs_all_7008_cases(self):
        output = self.root / 'correct'; self.compile_host(output)
        process = subprocess.run([str(output)], capture_output=True, timeout=5)
        self.assertEqual(process.returncode, 0, process.stderr.decode())
        self.assertEqual(process.stderr, b'')
        result = probe.validate_execution(process.stdout, process.returncode, COUNTS)
        self.assertEqual(sum(v['checked'] for v in result['cases'].values()), 7008)

    def test_corrupted_real_helper_produces_nonzero_and_arithmetic_failures(self):
        broken = self.root / 'broken.c'
        broken.write_text('__int128 __divti3(__int128 a, __int128 b) { (void)a; (void)b; return 0; }\n')
        output = self.root / 'broken'; self.compile_host(output, (broken,))
        process = subprocess.run([str(output)], capture_output=True, timeout=5)
        self.assertEqual(process.returncode, 1, process.stderr.decode())
        result = json.loads(process.stdout)
        self.assertGreater(result['cases']['signed_div']['failed'], 0)
        self.assertEqual(result['cases']['signed_div']['checked'], COUNTS['signed_div'])
        with self.assertRaises(EvidenceError): probe.validate_execution(process.stdout, process.returncode, COUNTS)
        with self.assertRaises(EvidenceError): probe.validate_execution(process.stdout, 0, COUNTS)

    def test_actual_darwin_core_retains_six_imports_without_execution(self):
        output = self.root / 'probe.o'
        result = subprocess.run([self.clang, '--target=x86_64-apple-macosx26.5', *probe.COMMON_FLAGS,
            '-ffreestanding', '-nostdinc', '-c', str(self.sources / 'probe.c'), '-o', str(output)],
            capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        record = probe.check_target_object(output.read_bytes())
        self.assertEqual(record['architecture'], 'x86_64')
        self.assertEqual(len(record['undefined_symbols']), 6)

    def test_actual_object_only_producer_has_no_runtime_claim(self):
        result = probe.run(self.root / 'object-only', self.clang)
        self.assertEqual(result['status'], 'darwin_object_checked_host_not_run', result.get('error'))
        self.assertEqual(result['host_execution'], 'not_run')
        self.assertEqual(result['darwin_execution'], 'not_run')
        self.assertFalse(result['selected_compiler_rt_qualified'])
        self.assertTrue((self.root / 'object-only/result.json').is_file())


    def test_failed_terminal_publication_does_not_report_success(self):
        output = self.root / 'publication-failure'
        original = probe.publish
        def fail_terminal(path, payload):
            if path.name == 'result.json':
                raise OSError('injected terminal write failure')
            original(path, payload)
        with mock.patch.object(probe, 'publish', side_effect=fail_terminal):
            result = probe.run(output, self.clang)
        self.assertEqual(result['status'], 'fail')
        self.assertIn('terminal evidence', result['error'])
        self.assertFalse((output / 'result.json').exists())

    def test_vector_mutation_after_real_compilation_fails(self):
        output = self.root / 'vector-mutation'
        original = probe.execution.observe
        def mutate(argv, cwd, directory, timeout, phase):
            original(argv, cwd, directory, timeout, phase)
            if phase['name'] == 'darwin-compile':
                path = output / 'sources/vectors.h'
                path.write_bytes(path.read_bytes() + b'/* drift */')
        with mock.patch.object(probe.execution, 'observe', side_effect=mutate):
            result = probe.run(output, self.clang)
        self.assertEqual(result['status'], 'fail')
        self.assertIn('reference vectors changed', result['error'])
        self.assertEqual(json.loads((output / 'result.json').read_text())['status'], 'fail')

    def test_non_nearest_rounding_mode_cannot_claim_arithmetic_execution(self):
        wrong = self.root / 'rounding.c'
        wrong.write_text('#include <fenv.h>\nint fegetround(void) { return FE_DOWNWARD; }\n')
        output = self.root / 'wrong-rounding'; self.compile_host(output, (wrong,))
        process = subprocess.run([str(output)], capture_output=True, timeout=5)
        self.assertEqual(process.returncode, 2)
        self.assertEqual(process.stdout, b'')
        self.assertIn(b'unsupported floating-point rounding mode', process.stderr)


if __name__ == '__main__': unittest.main()
