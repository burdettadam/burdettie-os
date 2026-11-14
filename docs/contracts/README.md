# Launch contract examples and validation

[launch-policy-v1.md](launch-policy-v1.md) fixes the M1 behavior contract. `launch-v1.schema.json` fixes version 1 wire shapes for a launch request, owner policy snapshot, backend capability manifest, pre-initialization launch report, and subsequent lifecycle events. M2 implements these interfaces in the actual runtime and launch adapters.

The four JSON examples are illustrative. All zero digests and `example` or `fixture` identifiers are placeholders, not application identities, credentials, platform support, or test evidence. The enforced example intentionally accepts unsigned code in its policy and refuses launch because the required resource backend is unsupported. It does not present a mock backend as a working security layer.

## Run

The development validator needs Python 3.10 or later and the pinned `jsonschema==4.26.0`:

```sh
python3 -m pip install -r requirements.txt
python3 validate_contract.py --self-test
python3 validate_contract.py examples/*.json
python3 validate_contract.py --allocation backends/puredarwin-x86_64.json
```

Run these commands from this directory. Installation is unnecessary when the required package is already present. The validator is development tooling; it is not linked into the OS or used as an authorization service.

The self-tests cover unknown or missing modes, unsupported enforced controls, silent downgrade, substituted policy revisions, incomplete audit, hidden policy effects or resource establishment in audit, policy-caused refusal in audit/permissive, architecture-specific evidence, native cross-architecture claims, missing descendant coverage, admission or observation confused with enforcement, failed establishment, omitted supervision/findings, contradictory actions or verification claims, final entitlement aggregation, and unsafe in-process confinement claims. A synthetic successful report exists only inside the test, labelled as fabricated input. It is not published as support evidence.

## Authority and semantic boundaries

The schema cannot prove that a record came from the trusted supervisor, a digest identifies real immutable bytes, a trust claim is cryptographically correct, a grant is owner-authorized, or a backend actually confines direct syscalls. `required_capabilities` is derived by the trusted supervisor from the complete policy; the validator checks consistency of this list but does not derive or authenticate it. Lifecycle sequencing and report authenticity also require trusted implementation. These checks belong to M2 through M6.

Owner authority references, document grants, and profile revisions must be resolved and authorized by the supervisor. Reports contain environment key names and transformations, never environment values or reusable grant tokens. An application-provided JSON document cannot grant privileges or declare its own effective policy.

Network rules in v1 are `deny` or `allow_all`. Unsupported destination rules are rejected instead of accepted with partial enforcement. Limits explicitly use milliseconds, bytes per process tree, process count, and separate stdout/stderr bytes; `null` means no optional ceiling. Operational ceilings and host permission failures are separately reported.

Backend evidence binds the backend/runtime/image version, OS/kernel, host architecture, guest architecture, execution route, and worker strategy. Reusing ARM64 evidence to qualify x86_64 is invalid. Each required capability must cover the needed scopes, be verified and available, and be established before an enforced launch is allowed. A valid record still needs real test evidence before it can represent supported behavior.

Evidence also names its exact capability ID, kind and covered scopes; a supervision test cannot qualify filesystem confinement. Required capabilities cannot have empty scopes. The [M1 backend allocation](BACKEND_ALLOCATION.md) maps all 17 control families to owners, source findings, candidate mechanisms and qualification tests. Its validator checks engineering coverage separately from launch records. Every control is still unsupported on the selected Burdettie guest; no allocation document authorizes launch or proves enforcement.

Every allowed enforced report must include a verified, available and established `launch.supervision` capability of kind `supervision` with application and descendant scope, as well as finalized findings. Each required capability names its expected kind; the provided control must match it exactly. This structural baseline does not derive the full selected policy. No document validator can establish complete policy enforcement by validating self-reported records.

Version 1 restricts process inspection and runtime-generated executable memory to `deny`. Reviewed libraries match the dependency digest list; reviewed subprocesses match the application digest list; device grants are explicit supervisor-resolved references. Empty resource and reviewed-code lists grant nothing. For trusted-signature profiles only, an empty signer identity list means no additional signer filter beyond the nonempty trust-anchor set and complete verification. A nonempty signer list uses lowercase SHA-256 digests of the exact DER leaf signing certificate that verified the CMS signature, never display names or Team IDs.

Entitlement rules in v1 operate on key presence: deny keys take precedence regardless of value; request-only keys grant nothing regardless of value; unknown keys follow the explicit deny/record choice. There is no value-predicate language in this schema.
