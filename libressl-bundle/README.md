# libressl-bundle

LibreSSL shared libraries for Linux ARM64, built natively inside Docker.

## Why

macOS ships LibreSSL as `/usr/bin/openssl` and `libcrypto`/`libssl`. When running
these macOS binaries on Linux under darwin-loader, the OpenSSL symbols resolve to
the host system's OpenSSL 3.x. LibreSSL (forked from OpenSSL 1.0.1) has deep ABI
incompatibilities with OpenSSL 3.x:

- Opaque structs (`EVP_MD_CTX`, `EVP_CIPHER_CTX`, etc.) whose fields LibreSSL accesses directly
- Removed functions (`SSL_library_init`, `EVP_cleanup`, `SSLeay`, etc.)
- Renamed accessors (`EVP_MD_size` → `EVP_MD_get_size`, etc.)
- LibreSSL-only APIs (`EVP_AEAD_*`, `tls_*`)

The fix: bundle real LibreSSL `.so` files for Linux ARM64. The macOS
binary's LibreSSL symbols resolve to an ABI-identical library.

## Version

LibreSSL **3.3.6** — matches the version shipped in macOS 15.x (Sequoia).

Check the host version: `/usr/bin/openssl version`

When Apple updates the LibreSSL version in a new macOS release, bump
`LIBRESSL_VERSION` in `Dockerfile.burdettie` and rebuild.

## Building

LibreSSL is built natively inside the Docker multi-stage build
(see the `libressl-builder` stage in `docker/Dockerfile.burdettie`).

> **Note:** Cross-compiling with zig introduces a codegen bug in the EC
> big-number arithmetic ("point is not on curve" error), so native building
> inside the Docker container is required.

The `build.sh` script in this directory is retained for local testing/debugging
of the cross-compile toolchain but is **not** used in the Docker build.

## Testing

Run the test suite inside Docker (requires `gcc` and `binutils` in the container):

```sh
# Quick: mount the test script and run
docker run --rm --platform linux/arm64 \
  -v "$PWD/libressl-bundle:/libressl-bundle:ro" \
  burdettie-os bash -c '
    apt-get update -qq && apt-get install -y -qq gcc binutils > /dev/null 2>&1
    bash /libressl-bundle/test.sh
  '
```

The test suite validates 4 layers (44 tests):

1. **Binary validation** — ELF format, aarch64 architecture, SONAME, dynamic symbols
2. **Symbol availability** — LibreSSL-specific symbols, core crypto (55), SSL (14), TLS (12)
3. **Native crypto** — SHA-256/512, AES-128/256, 3DES, RAND, RSA keygen, HMAC, BIO, PEM, EC NIDs
4. **Integration** — macOS `openssl` CLI via macho-inspect (version, base64, rand, ciphers, AES roundtrip, RSA)

## Integration

The Docker image copies these into `/opt/libressl/lib/` and the loader's
`StubResolver` preloads them instead of the system OpenSSL 3.x.

## Maintenance

1. When macOS ships a new LibreSSL version:
   - Run `/usr/bin/openssl version` on the new macOS
   - Update `LIBRESSL_VERSION` ARG in `docker/Dockerfile.burdettie`
   - Rebuild the Docker image

2. LibreSSL release cadence: ~2 releases/year (tracks OpenBSD releases)

3. No code changes needed in the loader — the ABI is stable within LibreSSL's
   major version series.
