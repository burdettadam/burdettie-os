# SSH-Backed Mac Oracle MCP Surface

This document defines the narrow MCP surface that agents should use when they need evidence from a real Mac host while the main workspace remains on Windows.

## Goal

Give agents safe, auditable access to a networked Mac for validation, extraction, and comparison tasks.

## Non-Goals

- Full remote desktop access
- Human pair-programming sessions
- Arbitrary shell access with no guardrails
- Turning the Mac into a hidden always-on dependency for normal repo work

## Server Identity

- MCP server name: `mac-oracle`
- Transport: SSH with key-based authentication
- Remote account: dedicated non-admin user
- Remote access model: pinned repo mirror plus pinned scratch directory

Suggested remote layout:

- repo mirror: `~/burdettie-oracle/repos/burdettie-os`
- scratch root: `~/burdettie-oracle/runs`
- artifact export root: `~/burdettie-oracle/exports`

## Required Tools

### `describe_host`

Return stable provenance before any other Mac-side operation.

Suggested input:

```json
{}
```

Suggested response:

```json
{
  "hostAlias": "burdettie-mac",
  "hostname": "mac-studio.local",
  "architecture": "arm64",
  "macosVersion": "15.4",
  "macosBuild": "24E214",
  "xcodeVersion": "16.3",
  "xcodeBuild": "16E140",
  "developerDir": "/Applications/Xcode.app/Contents/Developer",
  "sdkPath": "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk",
  "timestampUtc": "2026-04-10T18:32:11Z"
}
```

### `sync_to_mac`

Copy only the minimum local files needed for a task into the remote repo mirror or scratch directory.

Suggested input:

```json
{
  "sources": [
    "scripts/build_fixtures.sh",
    "fixtures/simple_cli/main.c"
  ],
  "destination": "repo",
  "deleteExtraneous": false
}
```

Required behavior:

- Reject paths outside the workspace
- Preserve relative layout under the remote repo mirror
- Prefer hash-aware sync when available
- Return copied files with size and hash metadata

### `run_on_mac`

Execute a bounded command on the Mac inside either the remote repo mirror or a scratch run directory.

Suggested input:

```json
{
  "cwd": "repo",
  "argv": [
    "./scripts/extract_xcode_tools.sh",
    "docker/xcode-tools"
  ],
  "timeoutSeconds": 1800,
  "env": {
    "DEVELOPER_DIR": "/Applications/Xcode.app/Contents/Developer"
  },
  "purpose": "Extract Xcode CLI tools for compatibility validation"
}
```

Required behavior:

- No interactive TTY
- No `sudo`
- No shell metacharacter expansion through a raw string command
- Prefer argv execution over `bash -lc`
- Enforce a command allowlist or path allowlist
- Return `stdout`, `stderr`, `exitCode`, `durationMs`, and produced file paths

Suggested allowlist categories:

- repo scripts under the remote mirror
- Xcode and CLT utilities such as `xcrun`, `xcodebuild`, `plutil`, `otool`, `codesign`, `file`, `defaults`, `sw_vers`, `uname`, `shasum`
- safe inspection tools under `/usr/bin`

Suggested denylist examples:

- `sudo`
- `osascript`
- `open`
- `rm -rf` outside scratch
- package installers
- browser automation

### `fetch_from_mac`

Copy selected logs or artifacts from the remote repo mirror, scratch, or export root back into the local workspace.

Suggested input:

```json
{
  "paths": [
    "docker/xcode-tools/xcode-dev/xcodebuild",
    "docker/xcode-tools/xcode-dev/xcrun"
  ],
  "sourceRoot": "repo",
  "destination": "docker/xcode-tools"
}
```

Required behavior:

- Return local output paths
- Return size and SHA-256 for each fetched file
- Preserve provenance metadata for each artifact set

## Safety Model

### Account and transport

- Use a dedicated SSH key for the MCP server
- Use a dedicated non-admin user on the Mac
- Pin the host key
- Disable password auth for this lane

### Filesystem constraints

- Remote writes only under the repo mirror and scratch root
- Read-only access to system tool locations and Xcode contents
- No blanket home-directory browsing

### Network constraints

- Default to no extra outbound network activity
- Require explicit user intent for downloads, notarization, or remote package fetches

### Auditability

Every tool response should include enough data to reconstruct what happened:

- host provenance
- exact argv
- working directory
- duration
- exit code
- hashes for fetched artifacts

## Preferred Workflows For This Repo

### 1. Extract Apple tools

1. `describe_host`
2. `run_on_mac` with `./scripts/extract_xcode_tools.sh`
3. `fetch_from_mac` for the produced tool bundle
4. Record hashes and Xcode provenance

### 2. Rebuild fixtures on the Mac

1. `describe_host`
2. `sync_to_mac` for changed fixture sources or scripts
3. `run_on_mac` with `./scripts/build_fixtures.sh`
4. `fetch_from_mac` for rebuilt fixtures and logs

### 3. Capture Mac ground truth for comparison

1. `describe_host`
2. `run_on_mac` with a narrow inspection command such as `xcrun --show-sdk-path`
3. Return the exact output and note what local implementation is being compared

## Implementation Notes

The transport can stay simple.

- `describe_host` can be implemented with `ssh` plus a small remote script that returns JSON.
- `sync_to_mac` and `fetch_from_mac` can use `scp`, `sftp`, or `rsync` depending on what is available on the Windows host.
- `run_on_mac` should use structured argv execution in the wrapper process rather than interpolated shell strings.

If you later build a full MCP server, keep the server thin. The server should expose only these bounded operations and leave workflow decisions to the agent layer.