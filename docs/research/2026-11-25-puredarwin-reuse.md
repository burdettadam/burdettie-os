# PureDarwin reuse: executable probes and contribution priorities

Recorded 2026-11-25 (America/Denver). Advances RQ-02, RQ-07, RQ-08 and RQ-09
under [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and
[ADR 0004](../adr/0004-linux-integration-boundaries.md).

Inspected PureDarwin revision:
`0c1990c803c13b68cad4df3f6dc34ba4306fc789`.
This record distinguishes executed Linux probes from source inspection. It
does not qualify a PureDarwin SDK build, Darwin framework execution, certificate
verifier or trusted Burdettie dependency.

## Findings that change the next work

1. **SDK discovery is the first small contribution candidate.** An executable
   wrapper probe reproduces inconsistent path/version reporting. Some major
   build paths supply an explicit SDK, so this is not evidence that every
   PureDarwin build is broken.
2. **libDER is portable enough to test natively, but not ready for trust
   adoption on this evidence.** The five unchanged C implementation files build
   on Linux. Both ordinary and ASan/UBSan runs pass 28 of 30 selected cases,
   with the same two semantic failures. Preserve and discuss those regressions
   upstream before using the affected behavior in admission.
3. **Framework reuse needs a coherent guest runtime closure.** PureDarwin's
   library recipes are not independent native Linux replacements for arbitrary
   parts of our runtime. Evaluate frameworks inside a matched compatibility
   backend and share behavior fixtures across projects.
4. **Rust should express the boundary.** Borrowed buffer lifetimes, C ABI types,
   complete-input validation and explicit errors are useful wrapper work.
   Another DER parser or parallel Objective-C runtime is not the next task.

## 1. Native libDER experiment

### Sources and provenance

The upstream [CMake definition](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/CMakeLists.txt)
lists five C files: `DER_Decode.c`, `DER_Encode.c`, `DER_CertCrl.c`,
`DER_Keys.c` and `oids.c`. The experiment compiled those files unchanged with
their upstream headers and Burdettie's
[source-owned probe](../../scripts/research/puredarwin_libder_probe.c).
The input receipt records SHA-256 hashes of the inspected files.

The five implementation inputs in that receipt are:

| Pinned source | SHA-256 |
| --- | --- |
| [DER_Decode.c](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Decode.c) | `2d61438bd515252b054a9f4ef9750aae3edf4f87b814ae5f18ba51845fe2b014` |
| [DER_Encode.c](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Encode.c) | `5b98116102bb5f37a1829cb4b42e17a1e0adbd1b73ced5a281c4cc095050dcd6` |
| [DER_CertCrl.c](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_CertCrl.c) | `4ac367198165df068d8bfbe5318bbcb516114acc1e51f5e495141d9185dd898f` |
| [DER_Keys.c](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Keys.c) | `f5b5d4864f8cfdace1950741c3272c7a78db80c636ffe5ea20add955bab9b6c4` |
| [oids.c](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/oids.c) | `1782f10ed36119870384e970ca6c70127ce0fcafabc5e3cecfa7754780d610c5` |

Use the matching pinned headers, including
[DERItem.h](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/DERItem.h),
[DER_Decode.h](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/DER_Decode.h)
and [DER_Encode.h](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/DER_Encode.h).

This particular implementation identifies itself as **PureDarwin's independent
2026 implementation**, based on the public interface and X.690. It must not be
described as adoption of Apple's libDER implementation. The inspected source
headers carry `MPL-2.0 OR BSD-2-Clause`; the repository's component/file notices
remain authoritative. [Configuration/provenance header](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/libDER_config.h),
[NOTICE](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/NOTICE).

### Executed results

The Linux x86_64 experiment used GCC 14. The plain build and the build with
AddressSanitizer/UndefinedBehaviorSanitizer both compiled with warnings treated
as errors. An independent repeat of the builds and runs produced identical
case results.

| Case group | Cases | Passed | What was exercised |
| --- | ---: | ---: | --- |
| TLV/tag/length handling | 14 | 14 | Selected valid encodings, truncations, nonminimal lengths/tags and overflow inputs |
| Unsigned integer helper | 6 | 6 | Zero, sign padding, negative input, nonminimal padding and 32-bit range |
| BIT STRING helper | 4 | 3 | Unused-bit count, empty content and nonzero unused final bits |
| Boolean helper | 2 | 2 | Canonical and noncanonical true encoding |
| Encoding | 2 | 1 | Integer and constructed SEQUENCE |
| Borrowed view / prefix decoding | 2 | 2 | Returned content aliases the input; a following TLV is permitted by the prefix decoder |
| **Total per run** | **30** | **28** | **Two semantic failures** |

Both test executables intentionally exited **1** because of those failures;
the compilation commands exited 0. Sanitizer runs had no diagnostics for this
fixed corpus. LeakSanitizer was disabled (`detect_leaks=0`), so no leak-check
result is claimed. No fuzzing, concurrency, alternate ABI, certificate-chain
or complete application qualification was performed. Passing the other cases
is not a claim that the parser implements every DER rule.

### Failure A: BIT STRING canonicality at the helper boundary

`DERParseBitString` received content bytes `{0x03, 0xa7}`: three unused final
bits, whose stored values are nonzero. The strict-DER test expected
`DR_DecodeError` (3); the helper returned `DR_Success` (0).

X.690 clause 11.2.1 requires unused final bits to be zero for DER. However, the
public helper returns the unused-bit count to its caller. Its intended API
contract may leave canonicality checks to that caller. This result therefore
requires a contract discussion: either enforce strictness in the helper or
make the required caller validation explicit and test its actual consumers.
It is not evidence of an exploit, an out-of-bounds access or a successful
signature bypass.

Sources: [helper implementation](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Decode.c),
[public decode interface](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/DER_Decode.h),
[ITU-T X.690 (02/2021)](https://www.itu.int/rec/T-REC-X.690-202102-I/en).

### Failure B: public constructed-tag representation

The probe calls `DEREncodeItem` with `ASN1_CONSTR_SEQUENCE` and content
`02 01 01`. It returns success but produces:

| Encoding | Bytes |
| --- | --- |
| Expected constructed SEQUENCE | `30 03 02 01 01` |
| Observed output | `1f 30 03 02 01 01` |

The default configuration enables multibyte tags and a 64-bit `DERTag`.
The encoder extracts class/constructed bits from the tag's top byte, while
the public `ASN1_CONSTR_SEQUENCE` macro is the low-byte value `0x30`. The
decoder's low-tag path likewise returns the full identifier byte in the low
part of the integer. The source therefore exposes an inconsistent tag
representation across public constants and encode/decode paths.

The upstream discussion should establish the intended ABI/API representation
and add constructed/classed tag and decode-to-encode regressions before choosing
a fix. Do not silently hide the inconsistency behind a Burdettie-only Rust tag
conversion while other users retain a different contract.

Sources: [tag constants](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/include/libDER/asn1Types.h),
[encoder](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Encode.c),
[decoder](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/lib/DER_Decode.c).

### What a Rust wrapper should guarantee

The probe confirms that decoded content is a borrowed view into the original
input. A safe wrapper must tie that view to the input lifetime, validate range
conversion, and prevent mutation/aliasing assumptions unsupported by the C API.
The C layout and build configuration matter: `DERSize` is `size_t`, while
`DERTag` depends on the configured width and multibyte-tag option. Use matching
native C ABI declarations rather than treating this in-memory representation
as a portable wire format.

`DERDecodeItem` intentionally accepts a following TLV: it decodes a prefix.
That behavior is not counted as a failure. A caller requiring exactly one
complete object must separately enforce full consumption and schema rules.
A successful parse does not establish CMS verification, a valid certificate
chain, signed content integrity or an owner trust decision.

### Reproducing the corpus

Use an unmodified checkout at the pinned PureDarwin revision. The following is
an equivalent reproduction recipe from the Burdettie repository root; it does
not build the full PureDarwin system or modify the upstream sources:

```sh
pd_source=/path/to/pinned/PureDarwin
python3 - "$pd_source" <<'PY'
from pathlib import Path
import os
import subprocess
import sys
import tempfile

der = Path(sys.argv[1]) / "src/Libraries/libDER"
probe = Path("scripts/research/puredarwin_libder_probe.c").resolve()
sources = [der / "lib" / name for name in (
    "DER_Decode.c", "DER_Encode.c", "DER_CertCrl.c", "DER_Keys.c", "oids.c")]
with tempfile.TemporaryDirectory(prefix="burdettie-libder-") as scratch:
    for sanitized in (False, True):
        binary = Path(scratch) / ("sanitized" if sanitized else "plain")
        command = ["cc", "-std=c11", "-O1", "-g", "-Wall", "-Wextra", "-Werror",
                   "-I", str(der / "include")]
        if sanitized:
            command += ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
        command += [str(probe), *map(str, sources), "-o", str(binary)]
        subprocess.run(command, check=True)
        environment = os.environ.copy()
        if sanitized:
            environment["ASAN_OPTIONS"] = "detect_leaks=0"
        result = subprocess.run([str(binary)], env=environment, check=False)
        print("sanitized=", sanitized, "exit=", result.returncode)
PY
```

The expected result for this pin is 30 cases with two failures, not a green
qualification result. Retain failed outputs when discussing a correction.

## 2. SDK discovery inconsistency reproduced

The pinned [SDK recipe](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/apple-sdk-pinned.nix)
declares version **26.5**. The
[toolchain](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/toolchain.nix)
defaults Clang's target to `x86_64-apple-macosx26.5`, but its fallback SDK path
names **11.3** and `xcrun --show-sdk-version` prints **11.3** independently of
the selected path.

The [SDK probe](../../scripts/research/puredarwin_sdk_probe.py) checks the exact
input hash, extracts the unchanged shell wrapper body and renders two literal
Nix parameters (`defaultSdkRoot` and `target`). It then executes only the path
and version discovery branches. It does **not** evaluate Nix, build a derivation,
inspect a real SDK or invoke a compiler. The override path below is a fixture.

```sh
python3 scripts/research/puredarwin_sdk_probe.py \
  --toolchain-nix /path/to/pinned/PureDarwin/nix/pkgs/toolchain/toolchain.nix
```

Required `toolchain.nix` SHA-256:
`59c146da8798d5bc5a9aab2c2288fc045e209147076803a60488bba0a5a9e972`.

| Environment | `--show-sdk-path` | `--show-sdk-version` |
| --- | --- | --- |
| No `DARWIN_SDK_ROOT` override | `/usr/local/osxcross/SDK/MacOSX11.3.sdk` | `11.3` |
| `DARWIN_SDK_ROOT=/fixture/MacOSX26.5.sdk` | `/fixture/MacOSX26.5.sdk` | `11.3` |

All four commands exited 0. That proves successful execution of inconsistent
discovery behavior, not consistency or successful compilation.

### Caller impact and bootstrap constraints

The inspected [main build](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/build.nix),
[native linker build](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/native-ld.nix)
and [compiler-rt build](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/compiler-rt.nix)
set explicit SDK paths. The fallback mismatch does not establish that these
builds use the wrong headers. Version discovery remains inconsistent under an
override, and external users of the exposed cross-toolchain need a coherent
path/version contract.

The [flake](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/flake.nix)
also has staged SDK/MIG generation and a bootstrap toolchain used to build the
native linker. Do not solve discovery by blindly injecting the final SDK into
every stage-zero input and creating a dependency cycle. A focused correction
can pass consistent discovery metadata or expose a host SDK/toolchain package,
with actual caller and dependency-graph tests. Simply replacing the hardcoded
version with another hardcoded value would leave overridden SDKs ambiguous.

## 3. Framework adoption remains a whole-runtime decision

The inspected recipes show explicit Darwin linking and object-model coupling:

| Component | Relevant dependency boundary |
| --- | --- |
| Objective-C runtime | Darwin libSystem, dyld integration and matching runtime headers |
| CoreFoundation | libSystem/libobjc, Foundation class/re-export relationships and Darwin linker metadata |
| Foundation | Objective-C/CoreFoundation ownership, Darwin libraries and dylib identity |
| CoreGraphics | Foundation/CoreFoundation/libobjc and its rendering dependencies |

Sources: [libobjc](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/libobjc.nix),
[CoreFoundation](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/corefoundation.nix),
[Foundation](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/foundation.nix),
[CoreGraphics](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/coregraphics.nix).

**Architecture inference:** establish the coherent Darling guest closure first,
then test any proposed PureDarwin framework provider within that declared ABI.
Do not replace individual libraries based only on exported names or compile
success. Keep objects, allocators, exceptions and callbacks within the owning
runtime, and bridge product services through narrow interfaces. Prefer existing
Linux ports for native Linux APIs. Shared semantic fixtures can benefit both
projects even when their execution backends differ.

## 4. Contribution order and acceptance

1. **SDK discovery test and focused fix.** Reproduce the affected supported
   caller path, align metadata with inputs, and preserve bootstrap stages.
2. **libDER regressions and contract correction.** Present the two focused
   cases, agree on canonicality/tag semantics, then correct and qualify the
   shared implementation. Native Linux packaging is useful alongside this
   work; it is not a substitute for semantic qualification.
3. **One shared framework workflow fixture.** Use precise output, error and
   lifetime expectations on both the chosen Linux route and PureDarwin when
   that upstream target is affected.

PureDarwin's inspected [contribution policy](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/CONTRIBUTING.md)
directs newcomers toward Nix packages and asks for coordination before changes
to existing kernel/system-library behavior. Follow the relevant target tests
and provenance requirements. AI-assisted implementation requires contributor
review/responsibility; ordinary issues, PR descriptions and review replies
must be human-authored under the stated policy, with its separate disclosed
automation exception. Maintainers decide acceptance.

No upstream message or submission was sent. This report and its reproducers
prepare evidence for contribution; they do not imply endorsement, acceptance
or adoption of the library into Burdettie's trust boundary.
