# Optional local tool inputs

This directory holds optional Mach-O compatibility inputs extracted from a
macOS host. The repository keeps this README; extracted tools are excluded
from Git and from the default Docker build context.

Build the runtime image from source without staging imported tools:

```sh
docker build -f docker/Dockerfile.burdettie -t burdettie-os .
```

For a local compatibility run, extract tools on a macOS host and mount the
result into the container read-only:

```sh
./scripts/extract_xcode_tools.sh
docker run --rm \
  --mount type=bind,src="$PWD/docker/xcode-tools",dst=/opt/xcode-tools,readonly \
  burdettie-os
```

These local inputs are independent of the runtime's source license. Keep
Apple SDKs, application bundles, frameworks and extracted tools out of
public repositories and published images unless their applicable terms
permit that distribution. The runtime can also consume a separate imported
artifact overlay; see [the development guide](../../docs/DOCKER_FIRST_DEVELOPMENT.md).
