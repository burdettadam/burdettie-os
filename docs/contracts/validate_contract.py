#!/usr/bin/env python3
"""Validate M1 wire examples and unsafe claim invariants, never OS enforcement.

Requires jsonschema >= 4.18, < 5. Run with --self-test or JSON paths.
This is a development document validator, not a runtime authorization engine.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
from pathlib import PurePosixPath
import re
import sys
import unittest

from jsonschema import Draft202012Validator, FormatChecker, ValidationError

ROOT = Path(__file__).resolve().parent
SCHEMA = json.loads((ROOT / "launch-v1.schema.json").read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATOR = Draft202012Validator(SCHEMA, format_checker=FormatChecker())

# Required engineering allocation coverage, not a policy-to-capability compiler.
# The trusted runtime must still derive a complete required list per launch.
M1_ALLOCATION_CONTROLS = {
    "launch.supervision": ("supervision", {"application", "descendants"}),
    "resource.limits": ("resource_enforcement", {"application", "descendants"}),
    "filesystem.grants": ("resource_enforcement", {"application", "descendants"}),
    "network.policy": ("resource_enforcement", {"application", "descendants"}),
    "process.descendants": ("resource_enforcement", {"application", "descendants"}),
    "process.inspection": ("resource_enforcement", {"application", "descendants"}),
    "memory.executable": ("resource_enforcement", {"application", "descendants"}),
    "ipc.services": ("resource_enforcement", {"application", "descendants"}),
    "devices.grants": ("resource_enforcement", {"application", "descendants"}),
    "code.dependencies": ("admission", {"application", "dependencies", "inserted_libraries", "dynamic_loads", "providers"}),
    "identity.immutable_inputs": ("admission", {"application", "dependencies"}),
    "identity.signature": ("admission", {"application", "dependencies", "bundle_resources"}),
    "identity.entitlements": ("admission", {"application", "dependencies"}),
    "launch.environment": ("admission", {"application", "descendants"}),
    "policy.storage": ("supervision", {"supervisor"}),
    "broker.authorization": ("supervision", {"supervisor", "application"}),
    "audit.events": ("observation", {"application", "descendants", "supervisor"}),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def check_backend(backend: dict) -> None:
    context = backend["context"]
    if context["execution_route"] == "native":
        require(context["host_arch"] == context["guest_arch"], "native route requires matching host and guest architecture in v1")
    capabilities = backend["capabilities"]
    ids = [item["id"] for item in capabilities]
    require(len(ids) == len(set(ids)), "duplicate capability ID")
    for item in capabilities:
        if item["id"] == "launch.supervision":
            require(item["kind"] == "supervision", "launch.supervision has a fixed capability kind")
        if item["implementation"] == "unsupported":
            require(item["availability"] != "available", "unsupported control cannot be available")
        if item["implementation"] == "verified":
            require(item["evidence"] is not None, "verified control needs qualification evidence")
        if item["evidence"] is not None:
            evidence = item["evidence"]
            require(evidence["capability_id"] == item["id"], "qualification evidence belongs to another capability")
            require(evidence["kind"] == item["kind"], "qualification evidence covers another capability kind")
            require(set(item["scopes"]).issubset(evidence["scopes"]), "capability advertises scopes absent from its qualification evidence")
            require(
                evidence["context"] == backend["context"],
                "capability evidence does not qualify this backend/route/platform/architecture",
            )


def validate_record(record: dict) -> None:
    VALIDATOR.validate(record)
    kind = record["message_type"]
    if kind == "backend_capabilities":
        check_backend(record)
    elif kind == "policy_snapshot":
        trust = record["code_trust"]
        if trust["requirement"] != "none":
            require(not trust["allow_unsigned"], "required signed integrity cannot admit unsigned code")
        if trust["requirement"] == "trusted_signer":
            require(not trust["allow_ad_hoc"], "ad-hoc signatures do not establish a trusted signer")
            require(bool(trust["trust_anchor_sha256"]), "trusted signer policy needs trust anchors")
            require(trust["time_validation"] != "not_required", "trusted signer policy needs explicit time validation")
        else:
            require(not trust["trust_anchor_sha256"] and not trust["allowed_signers"], "unused signer and anchor lists must be empty")
            require(trust["time_validation"] == "not_required", "certificate time policy requires trusted_signer")
            require(trust["revocation"] == "not_required", "certificate revocation policy requires trusted_signer")
        resources = record["resources"]
        if resources["filesystem"]["mode"] == "unrestricted":
            require(not resources["filesystem"]["grants"] and not resources["filesystem"]["denied_object_refs"], "unrestricted filesystem cannot also list restrictions")
        for family, list_key in (("ipc", "service_refs"), ("subprocess", "reviewed_app_sha256"), ("devices", "grant_refs")):
            if resources[family]["mode"] in ("deny", "unrestricted"):
                require(not resources[family][list_key], f"{family} mode has contradictory grant list")
    elif kind == "launch_report":
        backend = record["backend"]
        check_backend(backend)
        require(record["identity"]["selected_slice"] == backend["context"]["guest_arch"], "selected slice and backend guest architecture differ")
        establishment = record["establishment"]
        ids = [item["capability_id"] for item in establishment]
        require(len(ids) == len(set(ids)), "duplicate establishment ID")
        required = record["required_capabilities"]
        req_ids = [item["id"] for item in required]
        require(len(req_ids) == len(set(req_ids)), "duplicate required capability ID")
        cap_by_id = {item["id"]: item for item in backend["capabilities"]}
        for item in establishment:
            if item["state"] == "established":
                require(item["capability_id"] in cap_by_id, "established capability absent from manifest")
                cap = cap_by_id[item["capability_id"]]
                require(cap["implementation"] != "unsupported", "unsupported capability cannot be established")
                require(cap["availability"] == "available", "unavailable capability cannot be established")
        for needed in required:
            if needed["id"] in cap_by_id:
                require(cap_by_id[needed["id"]]["kind"] == needed["kind"], "required capability kind does not match the supplied implementation")
        for item in record["findings"]:
            if item["finding"] in ("not_evaluated", "unknown", "unsupported"):
                require(item["evidence_state"] != "verified", "missing evaluation cannot claim verified evidence")
            if item["finding"] == "not_evaluated":
                require(item["evidence_state"] == "not_evaluated", "skipped finding must identify skipped evidence")
            if item["finding"] == "allow":
                require(item["action"] != "denied", "allowed finding cannot claim a denied action")
        if record["disposition"] == "allow":
            require(record["effective_mode"] == record["requested_mode"], "requested mode cannot silently downgrade")
            require(record["effective_profile"] == record["requested_profile"], "effective profile must match requested revision")
            require(record["failure_category"] == "none", "allowed launch cannot carry a failure category")
            require(all(item["action"] != "denied" for item in record["findings"]), "allowed launch cannot contain an actual denial")
        else:
            require(record["effective_mode"] is None and record["effective_profile"] is None, "refused launch cannot claim an established effective mode/profile")
            require(record["failure_category"] != "none", "refusal needs a reason category")
        if record["requested_mode"] in ("audit", "permissive"):
            require(record["failure_category"] != "policy", "optional policy cannot cause audit/permissive refusal")
            require(all(item["action"] in ("none", "observed_only") for item in record["findings"]), "optional policy must not deny, strip, limit or establish restrictions in audit/permissive")
            require(all(item["category"] != "policy" for item in record["environment_transformations"]), "audit/permissive must not apply optional policy environment changes")
            for item in establishment:
                if item["state"] == "established":
                    require(item["capability_id"] in cap_by_id, "established capability absent from manifest")
                    require(cap_by_id[item["capability_id"]]["kind"] != "resource_enforcement", "optional resource policy cannot be established in audit/permissive")
        if record["requested_mode"] == "audit":
            require(record["audit_coverage"] != "not_requested", "audit request requires declared observation coverage")
            if any(item["finding"] in ("unknown", "unsupported", "not_evaluated") for item in record["findings"]):
                require(record["audit_coverage"] == "incomplete", "missing audit observation cannot be complete")
        if record["requested_mode"] == "enforced" and record["disposition"] == "allow":
            require(backend["context"]["worker_model"] == "managed_process", "legacy in-process route cannot claim managed enforced launch")
            require(bool(record["findings"]), "allowed enforced report needs finalized findings")
            supervision = next((item for item in required if item["id"] == "launch.supervision"), None)
            require(supervision is not None, "enforced launch requires explicit launch.supervision capability")
            require(supervision["kind"] == "supervision", "launch.supervision must require supervision kind")
            require({"application", "descendants"}.issubset(supervision["scopes"]), "supervision must cover the application and descendants")
            for item in record["findings"]:
                require(item["finding"] != "deny", "denial cannot coexist with allowed enforced launch")
                if item["required"]:
                    require(item["finding"] == "allow", "required unknown, skipped or unsupported control must refuse enforced launch")
            established = {item["capability_id"]: item["state"] for item in establishment}
            for needed in required:
                cap = cap_by_id.get(needed["id"])
                require(cap is not None, "required capability absent")
                require(cap["kind"] == needed["kind"], "required capability kind does not match")
                require(cap["implementation"] == "verified", "required control is not qualified")
                require(cap["availability"] == "available", "required control unavailable on this launch")
                require(set(needed["scopes"]).issubset(cap["scopes"]), "required scope not covered")
                require(established.get(needed["id"]) == "established", "required control not established")


def example(name: str) -> dict:
    return json.loads((ROOT / "examples" / name).read_text())


def validate_allocation(record: dict) -> None:
    """Check engineering coverage; this never authorizes a guest launch."""
    def fields(value, names, label):
        require(isinstance(value, dict) and set(value) == set(names), f"{label} has missing or unknown fields")

    def nonempty(value):
        return isinstance(value, str) and bool(value.strip())

    def strings(value, label):
        require(isinstance(value, list) and bool(value) and all(nonempty(x) for x in value), f"{label} needs nonempty strings")

    def revision(value):
        return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{40}", value) is not None

    fields(record, ("allocation_version", "purpose", "boundary", "platform_source", "backend", "allocations", "related_work"), "allocation")
    require(type(record["allocation_version"]) is int and record["allocation_version"] == 1, "unknown allocation version")
    require(record["purpose"] == "engineering_allocation_not_authorization", "allocation must not be an authorization record")
    require(record["boundary"] == "guest_application", "host VM isolation cannot qualify a guest application backend")
    source = record["platform_source"]
    fields(source, ("repository", "revision"), "platform source")
    require(source["repository"] == "PureDarwin/PureDarwin" and revision(source["revision"]), "allocation requires a pinned PureDarwin source")
    backend = record["backend"]
    validate_record(backend)
    require(backend["message_type"] == "backend_capabilities", "allocation needs a backend capability record")
    context = backend["context"]
    require(context["host_os"] == "Darwin" and context["host_arch"] == context["guest_arch"] == "x86_64", "allocation target is the x86_64 Darwin guest")
    require(context["execution_route"] == "native" and context["worker_model"] == "managed_process", "allocation target is a native managed worker")
    require(revision(context["runtime_revision"]), "allocation requires the inspected runtime revision")
    caps = {item["id"]: item for item in backend["capabilities"]}
    require(set(caps) == set(M1_ALLOCATION_CONTROLS), "allocation is missing a required control or adds an unknown control")
    allocations = record["allocations"]
    require(isinstance(allocations, list), "allocations must be a list")
    allocated = set()
    for item in allocations:
        fields(item, ("capability_id", "owner", "backend_mechanism", "candidate_mechanism", "source_refs", "gaps", "qualification_tests"), "control allocation")
        identity = item["capability_id"]
        require(isinstance(identity, str) and identity in caps and identity not in allocated, "unknown or duplicate allocation ID")
        allocated.add(identity)
        require(nonempty(item["owner"]), "each control needs an engineering owner")
        require(nonempty(item["candidate_mechanism"]), "each control needs a concrete next integration decision")
        cap = caps[identity]
        expected_kind, expected_scopes = M1_ALLOCATION_CONTROLS[identity]
        require(cap["kind"] == expected_kind, "allocation capability kind changed")
        require(expected_scopes.issubset(cap["scopes"]), "allocation omits required scope")
        if cap["implementation"] == "unsupported":
            require(item["backend_mechanism"] is None, "unsupported control cannot claim an integrated backend")
        else:
            require(nonempty(item["backend_mechanism"]), "implemented control needs its actual backend mechanism")
        if cap["implementation"] == "verified":
            require(re.fullmatch(r"sha256:[0-9a-f]{64}", context["image_revision"]) is not None, "guest qualification requires the built image digest")
            require(context["kernel_build"] != "not-observed", "guest qualification requires an observed kernel identity")
        require(isinstance(item["gaps"], list) and all(nonempty(gap) for gap in item["gaps"]), "gaps must be a list of descriptions")
        if cap["implementation"] != "verified":
            require(bool(item["gaps"]), "unqualified control needs explicit gaps")
        strings(item["qualification_tests"], "qualification tests")
        refs = item["source_refs"]
        require(isinstance(refs, list) and bool(refs), "allocation needs inspected source references")
        for ref in refs:
            fields(ref, ("repository", "revision", "path", "observation"), "source reference")
            require(ref["repository"] in ("burdettadam/burdettie-os", "PureDarwin/PureDarwin") and revision(ref["revision"]), "source reference must name a reviewed repository revision")
            require(nonempty(ref["path"]), "source reference requires a relative path")
            path = PurePosixPath(ref["path"])
            require(not path.is_absolute() and ".." not in path.parts and ":" not in ref["path"], "source reference must not contain a private absolute path or URL")
            require(nonempty(ref["observation"]), "source reference needs its observed limitation")
    require(allocated == set(caps), "every control needs exactly one allocation")
    require(isinstance(record["related_work"], list), "related work must be a list")
    for work in record["related_work"]:
        fields(work, ("url", "revision", "status", "scope"), "related work")
        require(isinstance(work["url"], str) and re.fullmatch(r"https://github\.com/burdettadam/burdettie-os/pull/[1-9][0-9]*", work["url"]), "related work must identify a public source PR")
        require(revision(work["revision"]) and work["status"] in ("published_unmerged", "merged", "closed_unmerged") and nonempty(work["scope"]), "related work needs exact revision, state and scope")


class ContractTests(unittest.TestCase):
    def test_published_examples(self):
        for path in sorted((ROOT / "examples").glob("*.json")):
            with self.subTest(path=path.name):
                validate_record(json.loads(path.read_text()))

    def invalid(self, record):
        with self.assertRaises((ValueError, ValidationError)):
            # jsonschema.ValidationError and semantic ValueError are both expected.
            validate_record(record)

    def synthetic_allowed_report(self):
        """Fabricated validator input, never stored as qualification evidence."""
        r = example("report-enforced-refused.json")
        r.update(disposition="allow", effective_mode="enforced", effective_profile=r["requested_profile"], failure_category="none")
        for cap in r["backend"]["capabilities"]:
            cap.update(implementation="verified", availability="available", evidence={
                "test_ref": "synthetic-test-only",
                "context": copy.deepcopy(r["backend"]["context"]),
                "capability_id": cap["id"], "kind": cap["kind"],
                "scopes": copy.deepcopy(cap["scopes"]),
            })
        for established in r["establishment"]:
            established["state"] = "established"
        r["findings"][0].update(finding="allow", action="established", evidence_state="verified", reason_code="synthetic-only")
        return r

    def test_unsigned_and_enforced_are_independent(self):
        policy = example("policy-local-notes.json")
        self.assertTrue(policy["code_trust"]["allow_unsigned"])
        self.assertEqual(example("request-enforced-unsigned.json")["security"]["mode"], "enforced")
        validate_record(policy)

    def test_unknown_mode_or_missing_mode(self):
        r = example("request-enforced-unsigned.json")
        r["security"]["mode"] = "secure"
        self.invalid(r)
        del r["security"]["mode"]
        self.invalid(r)

    def test_unknown_policy_field_is_rejected(self):
        p = example("policy-local-notes.json")
        p["resources"]["network"] = {"allow_hosts": ["example.test"]}
        self.invalid(p)

    def test_report_cannot_contain_environment_values(self):
        r = example("report-enforced-refused.json")
        r["environment"] = {"TOKEN": "must-not-be-serialized"}
        self.invalid(r)

    def test_unsupported_enforced_refuses(self):
        r = example("report-enforced-refused.json")
        r.update(disposition="allow", effective_mode="enforced", effective_profile=r["requested_profile"], failure_category="none")
        self.invalid(r)

    def test_synthetic_allowed_shape(self):
        validate_record(self.synthetic_allowed_report())

    def test_verification_evidence_cannot_cross_architecture(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][0]["evidence"]["context"]["guest_arch"] = "arm64"
        self.invalid(r)

    def test_qualification_evidence_cannot_be_reused_for_another_capability(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][1]["evidence"] = copy.deepcopy(r["backend"]["capabilities"][0]["evidence"])
        self.invalid(r)

    def test_qualification_evidence_must_cover_kind_and_scopes(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][1]["evidence"]["kind"] = "admission"
        self.invalid(r)
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][1]["evidence"]["scopes"] = ["application"]
        self.invalid(r)

    def test_required_resource_capability_cannot_have_empty_scope(self):
        r = self.synthetic_allowed_report()
        r["required_capabilities"][1]["scopes"] = []
        r["backend"]["capabilities"][1]["scopes"] = []
        self.invalid(r)

    def test_cross_architecture_is_not_native(self):
        r = example("backend-unsupported.json")
        r["context"]["guest_arch"] = "arm64"
        self.invalid(r)

    def test_silent_mode_downgrade(self):
        r = self.synthetic_allowed_report()
        r["effective_mode"] = "permissive"
        self.invalid(r)

    def test_policy_revision_substitution(self):
        r = self.synthetic_allowed_report()
        r["effective_profile"] = {"id": "local-notes", "revision": "2"}
        self.invalid(r)

    def test_missing_descendant_scope(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][0]["scopes"].remove("descendants")
        self.invalid(r)

    def test_observer_is_not_enforcer(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][0]["kind"] = "observation"
        self.invalid(r)

    def test_runtime_establishment_required(self):
        r = self.synthetic_allowed_report()
        r["establishment"][0]["state"] = "failed"
        self.invalid(r)

    def test_final_entitlement_denial_changes_aggregate(self):
        r = self.synthetic_allowed_report()
        r["findings"].append({"control": "entitlements", "required": True, "finding": "deny", "action": "denied", "evidence_state": "observed", "reason_code": "denied_key"})
        self.invalid(r)

    def test_audit_does_not_enforce_and_declares_missing_observation(self):
        r = example("report-enforced-refused.json")
        r.update(requested_mode="audit", effective_mode="audit", effective_profile=r["requested_profile"], disposition="allow", failure_category="none", audit_coverage="incomplete")
        r["findings"][0]["action"] = "observed_only"
        validate_record(r)
        r["environment_transformations"] = [{"name": "DYLD_INSERT_LIBRARIES", "action": "stripped", "category": "policy"}]
        self.invalid(r)
        r["environment_transformations"] = []
        r["audit_coverage"] = "complete"
        self.invalid(r)

    def test_shared_address_space_does_not_claim_isolation(self):
        r = self.synthetic_allowed_report()
        r["backend"]["context"]["worker_model"] = "legacy_in_process"
        for cap in r["backend"]["capabilities"]:
            cap["evidence"]["context"]["worker_model"] = "legacy_in_process"
        self.invalid(r)

    def test_admission_control_cannot_replace_resource_enforcement(self):
        r = self.synthetic_allowed_report()
        r["backend"]["capabilities"][1]["kind"] = "admission"
        self.invalid(r)

    def test_audit_cannot_secretly_establish_resource_controls(self):
        r = self.synthetic_allowed_report()
        r.update(requested_mode="audit", effective_mode="audit", audit_coverage="complete")
        r["findings"][0]["action"] = "observed_only"
        self.invalid(r)
        r["establishment"][1]["state"] = "not_established"
        validate_record(r)  # Non-policy supervision remains permitted.

    def test_allowed_launch_cannot_contain_denied_action(self):
        r = self.synthetic_allowed_report()
        r["findings"][0]["action"] = "denied"
        self.invalid(r)

    def test_missing_findings_cannot_claim_verified_evidence(self):
        for finding in ("not_evaluated", "unknown", "unsupported"):
            with self.subTest(finding=finding):
                r = example("report-enforced-refused.json")
                r["findings"][0].update(finding=finding, evidence_state="verified")
                self.invalid(r)

    def test_empty_report_cannot_claim_enforced_success(self):
        r = self.synthetic_allowed_report()
        r.update(findings=[], required_capabilities=[], establishment=[])
        self.invalid(r)

    def test_enforced_launch_requires_supervision_and_findings(self):
        r = self.synthetic_allowed_report()
        r["required_capabilities"] = r["required_capabilities"][1:]
        self.invalid(r)
        r = self.synthetic_allowed_report()
        r["findings"] = []
        self.invalid(r)

    def test_optional_policy_cannot_refuse_audit_or_permissive(self):
        for mode in ("audit", "permissive"):
            with self.subTest(mode=mode):
                r = example("report-enforced-refused.json")
                r.update(requested_mode=mode, audit_coverage="incomplete" if mode == "audit" else "not_requested")
                r["findings"][0]["action"] = "observed_only"
                self.invalid(r)

    def test_unqualified_process_inspection_and_executable_memory_are_rejected(self):
        p = example("policy-local-notes.json")
        p["resources"]["process_inspection"] = "allow_reviewed"
        self.invalid(p)
        p = example("policy-local-notes.json")
        p["code_loading"]["executable_memory"] = "allow_reviewed"
        self.invalid(p)

    def test_signer_identifiers_require_exact_leaf_certificate_digest(self):
        p = example("policy-local-notes.json")
        p["code_trust"].update(requirement="trusted_signer", allow_unsigned=False, allow_ad_hoc=False, trust_anchor_sha256=["1" * 64], time_validation="current_time", allowed_signers=["2" * 64])
        validate_record(p)
        p["code_trust"]["allowed_signers"] = ["Example Vendor TEAM123456"]
        self.invalid(p)
        p["code_trust"]["allowed_signers"] = []
        validate_record(p)  # No additional identity filter beyond valid chain to anchors.


class BackendAllocationTests(unittest.TestCase):
    def allocation(self):
        return json.loads((ROOT / "backends" / "puredarwin-x86_64.json").read_text())

    def invalid(self, record):
        with self.assertRaises((ValueError, ValidationError)):
            validate_allocation(record)

    def test_published_allocation_is_complete_but_unqualified(self):
        record = self.allocation()
        validate_allocation(record)
        self.assertTrue(all(cap["implementation"] == "unsupported" for cap in record["backend"]["capabilities"]))
        self.assertTrue(all(cap["evidence"] is None for cap in record["backend"]["capabilities"]))

    def test_omitting_a_control_cannot_complete_the_allocation(self):
        record = self.allocation()
        record["backend"]["capabilities"] = record["backend"]["capabilities"][1:]
        record["allocations"] = record["allocations"][1:]
        self.invalid(record)

    def test_duplicate_or_unknown_owner_records_are_rejected(self):
        record = self.allocation()
        record["allocations"].append(copy.deepcopy(record["allocations"][0]))
        self.invalid(record)
        record = self.allocation()
        record["allocations"][0]["owner"] = " "
        self.invalid(record)

    def test_unqualified_controls_need_gaps_and_qualification_tests(self):
        for key in ("gaps", "qualification_tests", "source_refs"):
            with self.subTest(key=key):
                record = self.allocation()
                record["allocations"][0][key] = []
                self.invalid(record)

    def test_host_isolation_is_not_a_guest_backend(self):
        record = self.allocation()
        record["boundary"] = "host_qemu_process"
        self.invalid(record)
        record = self.allocation()
        record["backend"]["context"]["host_os"] = "Linux"
        self.invalid(record)

    def test_allocated_kind_and_descendant_scope_cannot_be_weakened(self):
        record = self.allocation()
        record["backend"]["capabilities"][1]["kind"] = "observation"
        self.invalid(record)
        record = self.allocation()
        record["backend"]["capabilities"][1]["scopes"] = ["application"]
        self.invalid(record)

    def test_unobserved_image_cannot_gain_qualification(self):
        record = self.allocation()
        cap = record["backend"]["capabilities"][0]
        cap.update(implementation="verified", availability="available", evidence={
            "test_ref": "synthetic-test-only", "context": copy.deepcopy(record["backend"]["context"]),
            "capability_id": cap["id"], "kind": cap["kind"], "scopes": copy.deepcopy(cap["scopes"]),
        })
        record["allocations"][0]["backend_mechanism"] = "synthetic test input, not a real implementation"
        self.invalid(record)

    def test_private_paths_and_unpinned_source_refs_are_rejected(self):
        for key, value in (("path", "/private/host/run.log"), ("path", "../../run.log"), ("revision", "main")):
            with self.subTest(key=key, value=value):
                record = self.allocation()
                record["allocations"][0]["source_refs"][0][key] = value
                self.invalid(record)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--allocation", action="append", default=[], type=Path,
                        help="validate engineering backend allocation (does not authorize a launch)")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.TestSuite([
            unittest.defaultTestLoader.loadTestsFromTestCase(ContractTests),
            unittest.defaultTestLoader.loadTestsFromTestCase(BackendAllocationTests),
        ])
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    if not args.paths and not args.allocation:
        parser.error("pass --self-test, JSON record paths, or --allocation PATH")
    for path, validate in ([(p, validate_record) for p in args.paths]
                           + [(p, validate_allocation) for p in args.allocation]):
        try:
            validate(json.loads(path.read_text()))
        except (OSError, ValueError, ValidationError) as error:
            print(f"INVALID {path}: {error}", file=sys.stderr)
            return 1
        print(f"VALID {path}")
    print("Record validation only; no runtime, trust or confinement claim is established.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
