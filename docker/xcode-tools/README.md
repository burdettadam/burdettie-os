# Optional Xcode Tool Staging

This directory is intentionally tracked so `docker/Dockerfile.burdettie` can
build even when no imported Apple tools have been staged yet.

Current in-repo workflow:

- leave this directory empty when developing the Linux runtime without imported
  Apple tools
- populate it with `scripts/extract_xcode_tools.sh` when running imported Xcode
  tool compatibility checks

The longer-term external Compose lane may mount a read-only imported-artifact
overlay instead of baking tool binaries into the image.