---
name: mac-oracle
description: 'Use the networked Mac as an SSH-backed validation oracle for macOS behavior, Xcode CLI checks, fixture rebuilds, artifact extraction, and behavior comparison. Use when work needs real macOS execution, Xcode Command Line Tools, Apple tool output, or Mac-side provenance.'
argument-hint: 'What should be run or collected on the Mac?'
---

# Mac Oracle

Use this skill when a task genuinely needs a real Mac host, not when the work can stay inside the normal Windows or Linux flow.

This repo already treats the Mac as an extraction and validation oracle rather than a permanent runtime dependency. Keep that boundary intact.

## Use This For

- Running a bounded command on the networked Mac to observe real macOS behavior
- Capturing Xcode or Command Line Tools output for comparison against burdettie
- Rebuilding fixtures or extracting Apple tool artifacts on the Mac
- Fetching logs, plists, app metadata, or hashes from Mac-produced artifacts
- Verifying macOS version, Xcode version, SDK path, or developer directory state

## Do Not Use This For

- Ordinary repo work that already runs on Windows or Linux
- Long-lived interactive pair-programming sessions
- Editing arbitrary files on the Mac by hand
- Making the repo depend on a live Mac host for normal build or test flows
- Downloading broad external dependencies unless the user explicitly asked for it

## Required MCP Shape

This skill assumes an MCP server named `mac-oracle` exists or will exist. The expected tool surface is documented in [the MCP surface reference](./references/mcp-surface.md).

Minimum expected tools:

- `describe_host`
- `sync_to_mac`
- `run_on_mac`
- `fetch_from_mac`

If those tools are unavailable, stop and tell the user the skill is present but the `mac-oracle` MCP server still needs to be configured.

## Workflow

1. Decide whether the task truly requires a Mac.
2. Call `describe_host` first to capture host provenance.
3. Sync only the minimum files needed for the task.
4. Run a bounded Mac command through `run_on_mac`.
5. Fetch only the artifacts or logs that matter.
6. Report both the Mac-side result and why the work could not stay local.

## Repo-Specific Bias

- Prefer repo scripts over ad hoc shell where possible.
- Prefer artifact extraction and behavioral comparison over keeping a remote shell open.
- Keep imported Apple artifacts as validation inputs, not core runtime dependencies.
- Preserve provenance for every fetched artifact: host, macOS version, Xcode version if relevant, source path, and hash.

## Common Tasks

### Extract Xcode tools

Use the Mac checkout to run [scripts/extract_xcode_tools.sh](../../../scripts/extract_xcode_tools.sh), then fetch the produced tool bundle and record provenance.

### Rebuild fixtures

Sync only the changed fixture sources or scripts, run [scripts/build_fixtures.sh](../../../scripts/build_fixtures.sh), then fetch the rebuilt artifacts plus hashes.

### Capture ground truth behavior

Run targeted commands such as `xcrun --show-sdk-path`, `xcodebuild -list`, `plutil -p`, or `otool -L` and return the exact stdout or stderr needed for comparison.

## Safety Rules

- Do not assume unrestricted shell access.
- Do not use `sudo`, GUI automation, or interactive prompts.
- Do not modify files outside the remote repo mirror or scratch directory.
- Do not fetch entire trees when a few files or logs are enough.
- Do not leave the user with a hidden Mac dependency; explain what was Mac-only.

## Output Expectations

When you use the Mac oracle, report:

- why the Mac host was needed
- what command or script ran
- what artifacts or logs were fetched
- the key host provenance returned by `describe_host`
- whether the result changes repo code, docs, or only validation evidence