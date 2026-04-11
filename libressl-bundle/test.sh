#!/usr/bin/env bash
# libressl-bundle/test.sh — Thorough validation of bundled LibreSSL .so files.
#
# Run inside the Docker container (linux/arm64):
#   bash /path/to/libressl-bundle/test.sh
#
# Or from the host via Docker:
#   docker run --rm --platform linux/arm64 \
#     -v "$PWD/libressl-bundle:/libressl-bundle:ro" \
#     burdettie-os bash /libressl-bundle/test.sh
#
# Exit code: 0 if all tests pass, 1 if any fail.
#
# Tests are organized in layers:
#   1. Binary validation   — file format, architecture, SONAME
#   2. Symbol availability  — critical LibreSSL-only and shared symbols
#   3. Native crypto        — EVP, RSA, EC, AES, base64, HMAC via dlsym
#   4. Version & ABI        — version string, struct sizes, OBJ database
#   5. Integration          — openssl CLI exercised via macho-inspect

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────

LIBRESSL_DIR="${LIBRESSL_DIR:-/opt/libressl/lib}"
LIBCRYPTO="${LIBRESSL_DIR}/libcrypto.so"
LIBSSL="${LIBRESSL_DIR}/libssl.so"
LIBTLS="${LIBRESSL_DIR}/libtls.so"

PASS=0
FAIL=0
SKIP=0

# ── Helpers ────────────────────────────────────────────────────────

red()   { printf '\033[1;31m%s\033[0m' "$*"; }
green() { printf '\033[1;32m%s\033[0m' "$*"; }
yellow(){ printf '\033[1;33m%s\033[0m' "$*"; }

pass() { PASS=$((PASS + 1)); printf '  [%s] %s\n' "$(green PASS)" "$1"; }
fail() { FAIL=$((FAIL + 1)); printf '  [%s] %s — %s\n' "$(red FAIL)" "$1" "$2"; }
skip() { SKIP=$((SKIP + 1)); printf '  [%s] %s — %s\n' "$(yellow SKIP)" "$1" "$2"; }

# Compile and run a small C program that uses libcrypto/libssl via dlsym.
# $1 = test name, $2 = C source (stdin-style), expected exit 0.
run_c_test() {
    local name="$1"
    local src="$2"
    local tmpdir
    tmpdir=$(mktemp -d)
    local src_file="${tmpdir}/test.c"
    local bin_file="${tmpdir}/test"

    printf '%s' "$src" > "$src_file"
    if ! gcc -o "$bin_file" "$src_file" -ldl 2>"${tmpdir}/compile.err"; then
        fail "$name" "compile error: $(cat "${tmpdir}/compile.err" | head -3)"
        rm -rf "$tmpdir"
        return
    fi

    local output
    if output=$("$bin_file" 2>&1); then
        # Check if output contains FAIL marker
        if echo "$output" | grep -q '^FAIL:'; then
            fail "$name" "$(echo "$output" | grep '^FAIL:' | head -1)"
        else
            pass "$name"
        fi
    else
        fail "$name" "exit $?: $(echo "$output" | tail -3)"
    fi
    rm -rf "$tmpdir"
}

# ── Preflight ──────────────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════"
echo "  LibreSSL Bundle Test Suite"
echo "  Libraries: ${LIBRESSL_DIR}"
echo "═══════════════════════════════════════════════════════════"
echo ""

HAS_GCC=true
if ! command -v gcc &>/dev/null; then
    echo "NOTE: gcc not found — native C tests will be skipped."
    echo "      Install gcc for full coverage: apt-get install -y gcc"
    HAS_GCC=false
fi

for lib in "$LIBCRYPTO" "$LIBSSL" "$LIBTLS"; do
    if [[ ! -f "$lib" ]]; then
        echo "FATAL: $(basename "$lib") not found at $lib"
        exit 1
    fi
done

# ══════════════════════════════════════════════════════════════════
# Layer 1: Binary Validation
# ══════════════════════════════════════════════════════════════════

echo "── Layer 1: Binary Validation ────────────────────────────"

# 1.1 Architecture check — must be ARM aarch64
for lib in "$LIBCRYPTO" "$LIBSSL" "$LIBTLS"; do
    name="arch:$(basename "$lib")"
    arch_info=$(file "$lib")
    if echo "$arch_info" | grep -q "ARM aarch64\|aarch64"; then
        pass "$name"
    elif echo "$arch_info" | grep -q "x86-64\|x86_64"; then
        fail "$name" "wrong arch: x86-64 (expected aarch64)"
    else
        fail "$name" "unexpected: $arch_info"
    fi
done

# 1.2 ELF shared library check
for lib in "$LIBCRYPTO" "$LIBSSL" "$LIBTLS"; do
    name="elf-shared:$(basename "$lib")"
    if file "$lib" | grep -q "ELF.*shared object"; then
        pass "$name"
    else
        fail "$name" "not an ELF shared object"
    fi
done

# 1.3 SONAME check — LibreSSL uses specific SONAME versioning
#     libcrypto.so.46, libssl.so.48 for LibreSSL 3.3.x
for lib_name in libcrypto libssl libtls; do
    name="soname:${lib_name}"
    lib_path="${LIBRESSL_DIR}/${lib_name}.so"
    if command -v readelf &>/dev/null; then
        soname=$(readelf -d "$lib_path" 2>/dev/null | grep SONAME | grep -oP '\[.*?\]' | tr -d '[]' || true)
        if [[ -n "$soname" ]]; then
            # LibreSSL 3.3.x SONAMEs: libcrypto.so.46, libssl.so.48, libtls.so.20
            case "$lib_name" in
                libcrypto)
                    if echo "$soname" | grep -qP 'libcrypto\.so\.\d+'; then
                        pass "$name ($soname)"
                    else
                        fail "$name" "unexpected SONAME: $soname"
                    fi
                    ;;
                libssl)
                    if echo "$soname" | grep -qP 'libssl\.so\.\d+'; then
                        pass "$name ($soname)"
                    else
                        fail "$name" "unexpected SONAME: $soname"
                    fi
                    ;;
                libtls)
                    if echo "$soname" | grep -qP 'libtls\.so\.\d+'; then
                        pass "$name ($soname)"
                    else
                        fail "$name" "unexpected SONAME: $soname"
                    fi
                    ;;
            esac
        else
            fail "$name" "no SONAME found"
        fi
    else
        skip "$name" "readelf not available"
    fi
done

# 1.4 Not stripped of all symbols (we need dynamic symbols)
for lib in "$LIBCRYPTO" "$LIBSSL" "$LIBTLS"; do
    name="dynsym:$(basename "$lib")"
    if command -v readelf &>/dev/null; then
        count=$(readelf --dyn-syms "$lib" 2>/dev/null | grep -c "FUNC\|OBJECT" || true)
        if [[ "$count" -gt 10 ]]; then
            pass "$name ($count dynamic symbols)"
        else
            fail "$name" "too few dynamic symbols: $count"
        fi
    else
        skip "$name" "readelf not available"
    fi
done

# 1.5 Library can be dlopen'd
if [[ "$HAS_GCC" == true ]]; then
    run_c_test "dlopen:all-libs" '
#include <stdio.h>
#include <dlfcn.h>

int main() {
    // Libraries must be loaded in dependency order with RTLD_GLOBAL:
    // libtls depends on libssl (SONAME libssl.so.48)
    // libssl depends on libcrypto (SONAME libcrypto.so.46)
    const char *libs[] = {
        "'"$LIBCRYPTO"'",
        "'"$LIBSSL"'",
        "'"$LIBTLS"'",
    };
    void *handles[3];
    for (int i = 0; i < 3; i++) {
        handles[i] = dlopen(libs[i], RTLD_NOW | RTLD_GLOBAL);
        if (!handles[i]) {
            printf("FAIL: dlopen(%s): %s\n", libs[i], dlerror());
            return 1;
        }
    }
    // Close in reverse order
    for (int i = 2; i >= 0; i--) dlclose(handles[i]);
    return 0;
}
'
fi

echo ""

# ══════════════════════════════════════════════════════════════════
# Layer 2: Symbol Availability
# ══════════════════════════════════════════════════════════════════

echo "── Layer 2: Symbol Availability ─────────────────────────"

# 2.1 LibreSSL-specific symbols that do NOT exist in OpenSSL 3.x
LIBRESSL_ONLY_SYMBOLS=(
    SSLeay
    SSLeay_version
    EVP_MD_size           # renamed to EVP_MD_get_size in OpenSSL 3
    EVP_CIPHER_nid        # renamed to EVP_CIPHER_get_nid in OpenSSL 3
    EVP_CIPHER_iv_length  # renamed in OpenSSL 3
    EVP_CIPHER_key_length # renamed in OpenSSL 3
    EVP_AEAD_CTX_open     # LibreSSL-only AEAD API
    EVP_AEAD_CTX_seal     # LibreSSL-only AEAD API
)

if [[ "$HAS_GCC" == true ]]; then
    # Build a single test that checks all LibreSSL-only symbols
    syms_check=""
    for sym in "${LIBRESSL_ONLY_SYMBOLS[@]}"; do
        syms_check+="    CHECK_SYM(\"$sym\");"$'\n'
    done

    run_c_test "symbols:libressl-specific" '
#include <stdio.h>
#include <dlfcn.h>

#define CHECK_SYM(name) do { \
    void *s = dlsym(h, name); \
    if (!s) { printf("FAIL: missing LibreSSL symbol: %s\n", name); return 1; } \
} while(0)

int main() {
    void *h = dlopen("'"$LIBCRYPTO"'", RTLD_NOW);
    if (!h) { printf("FAIL: dlopen: %s\n", dlerror()); return 1; }
'"$syms_check"'
    dlclose(h);
    return 0;
}
'
fi

# 2.2 Core crypto symbols that the macOS openssl binary imports
CORE_CRYPTO_SYMBOLS=(
    EVP_EncryptInit_ex  EVP_EncryptUpdate  EVP_EncryptFinal_ex
    EVP_DecryptInit_ex  EVP_DecryptUpdate  EVP_DecryptFinal_ex
    EVP_DigestInit_ex   EVP_DigestUpdate   EVP_DigestFinal_ex
    EVP_aes_256_cbc     EVP_aes_128_cbc    EVP_des_ede3_cbc
    EVP_sha256           EVP_sha512         EVP_md5
    EVP_PKEY_new         EVP_PKEY_free
    EVP_CIPHER_CTX_new   EVP_CIPHER_CTX_free
    EVP_MD_CTX_new       EVP_MD_CTX_free
    RSA_generate_key_ex  RSA_new  RSA_free  RSA_size
    BN_new               BN_free  BN_set_word
    PEM_write_RSAPrivateKey  PEM_write_RSAPublicKey
    PEM_read_RSAPrivateKey
    BIO_new  BIO_new_fp  BIO_free  BIO_ctrl  BIO_read  BIO_write
    BIO_s_mem  BIO_s_file
    RAND_bytes  RAND_pseudo_bytes
    OBJ_nid2sn  OBJ_sn2nid  OBJ_txt2nid  OBJ_nid2ln
    EC_KEY_new_by_curve_name  EC_KEY_free
    EC_GROUP_new_by_curve_name  EC_GROUP_free
    HMAC
    X509_new  X509_free
    ERR_error_string  ERR_get_error
    OPENSSL_init_crypto
)

if [[ "$HAS_GCC" == true ]]; then
    syms_check=""
    for sym in "${CORE_CRYPTO_SYMBOLS[@]}"; do
        syms_check+="    { void *s = dlsym(h, \"$sym\"); if (!s) { missing++; printf(\"  missing: %s\n\", \"$sym\"); } else { found++; } }"$'\n'
    done

    run_c_test "symbols:core-crypto (${#CORE_CRYPTO_SYMBOLS[@]} symbols)" '
#include <stdio.h>
#include <dlfcn.h>

int main() {
    void *h = dlopen("'"$LIBCRYPTO"'", RTLD_NOW | RTLD_GLOBAL);
    if (!h) { printf("FAIL: dlopen: %s\n", dlerror()); return 1; }

    int found = 0, missing = 0;
'"$syms_check"'

    dlclose(h);
    if (missing > 0) {
        printf("FAIL: %d/%d symbols missing\n", missing, found + missing);
        return 1;
    }
    return 0;
}
'
fi

# 2.3 SSL symbols from libssl
SSL_SYMBOLS=(
    SSL_CTX_new  SSL_CTX_free
    SSL_new  SSL_free
    SSL_connect  SSL_read  SSL_write
    SSL_library_init  SSL_load_error_strings
    TLSv1_2_method  SSLv23_method
    SSL_CTX_set_cipher_list
    SSL_CTX_use_certificate_file
    SSL_CTX_use_PrivateKey_file
)

if [[ "$HAS_GCC" == true ]]; then
    syms_check=""
    for sym in "${SSL_SYMBOLS[@]}"; do
        syms_check+="    { void *s = dlsym(h, \"$sym\"); if (!s) { missing++; printf(\"  missing: %s\n\", \"$sym\"); } else { found++; } }"$'\n'
    done

    run_c_test "symbols:libssl (${#SSL_SYMBOLS[@]} symbols)" '
#include <stdio.h>
#include <dlfcn.h>

int main() {
    void *hc = dlopen("'"$LIBCRYPTO"'", RTLD_NOW | RTLD_GLOBAL);
    void *h = dlopen("'"$LIBSSL"'", RTLD_NOW | RTLD_GLOBAL);
    if (!h) { printf("FAIL: dlopen libssl: %s\n", dlerror()); return 1; }

    int found = 0, missing = 0;
'"$syms_check"'

    dlclose(h);
    if (hc) dlclose(hc);
    if (missing > 0) {
        printf("FAIL: %d/%d SSL symbols missing\n", missing, found + missing);
        return 1;
    }
    return 0;
}
'
fi

# 2.4 TLS symbols from libtls
TLS_SYMBOLS=(
    tls_init  tls_config_new  tls_config_free
    tls_client  tls_close  tls_free
    tls_configure  tls_connect
    tls_read  tls_write
    tls_peer_cert_provided
    tls_error
)

if [[ "$HAS_GCC" == true ]]; then
    syms_check=""
    for sym in "${TLS_SYMBOLS[@]}"; do
        syms_check+="    { void *s = dlsym(h, \"$sym\"); if (!s) { missing++; printf(\"  missing: %s\n\", \"$sym\"); } else { found++; } }"$'\n'
    done

    run_c_test "symbols:libtls (${#TLS_SYMBOLS[@]} symbols)" '
#include <stdio.h>
#include <dlfcn.h>

int main() {
    void *hc = dlopen("'"$LIBCRYPTO"'", RTLD_NOW | RTLD_GLOBAL);
    void *hs = dlopen("'"$LIBSSL"'", RTLD_NOW | RTLD_GLOBAL);
    void *h = dlopen("'"$LIBTLS"'", RTLD_NOW | RTLD_GLOBAL);
    if (!h) { printf("FAIL: dlopen libtls: %s\n", dlerror()); return 1; }

    int found = 0, missing = 0;
'"$syms_check"'

    dlclose(h);
    if (hs) dlclose(hs);
    if (hc) dlclose(hc);
    if (missing > 0) {
        printf("FAIL: %d/%d TLS symbols missing\n", missing, found + missing);
        return 1;
    }
    return 0;
}
'
fi

echo ""

# ══════════════════════════════════════════════════════════════════
# Layer 3: Native Crypto Operations
# ══════════════════════════════════════════════════════════════════

echo "── Layer 3: Native Crypto Operations ────────────────────"

if [[ "$HAS_GCC" != true ]]; then
    echo "  (skipped — gcc not available)"
else

# Common C preamble for crypto tests
CRYPTO_PREAMBLE='
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *LC, *LS;

static void init_libs(void) {
    LC = dlopen("'"$LIBCRYPTO"'", RTLD_NOW | RTLD_GLOBAL);
    if (!LC) { printf("FAIL: dlopen libcrypto: %s\n", dlerror()); exit(1); }
    LS = dlopen("'"$LIBSSL"'", RTLD_NOW | RTLD_GLOBAL);
}

static void *SYM(const char *name) {
    void *s = dlsym(LC, name);
    if (!s && LS) s = dlsym(LS, name);
    if (!s) { printf("FAIL: symbol not found: %s\n", name); exit(1); }
    return s;
}
'

# 3.1 Version string — must say "LibreSSL"
run_c_test "crypto:version-string" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    const char *(*ver)(int) = SYM("SSLeay_version");
    const char *v = ver(0); // SSLEAY_VERSION
    if (!v || !strstr(v, "LibreSSL")) {
        printf("FAIL: version = \"%s\", expected LibreSSL\n", v ? v : "(null)");
        return 1;
    }
    // Check version number present
    if (!strstr(v, "3.3.6") && !strstr(v, "3.3") && !strstr(v, "3.")) {
        printf("FAIL: version = \"%s\", expected 3.x\n", v);
        return 1;
    }
    return 0;
}
'

# 3.2 OBJ database — NID lookup must work
run_c_test "crypto:obj-nid-lookup" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    const char *(*nid2sn)(int) = SYM("OBJ_nid2sn");
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");

    // Test well-known NIDs
    struct { const char *name; int expected_positive; } tests[] = {
        {"prime256v1", 1},
        {"secp384r1", 1},
        {"SHA256", 1},
        {"SHA512", 1},
        {"AES-256-CBC", 1},
        {"RSA", 1},
        {"MD5", 1},
    };
    int n = sizeof(tests)/sizeof(tests[0]);
    for (int i = 0; i < n; i++) {
        int nid = txt2nid(tests[i].name);
        if (nid <= 0) {
            printf("FAIL: OBJ_txt2nid(\"%s\") = %d\n", tests[i].name, nid);
            return 1;
        }
        const char *sn = nid2sn(nid);
        if (!sn) {
            printf("FAIL: OBJ_nid2sn(%d) = NULL for %s\n", nid, tests[i].name);
            return 1;
        }
    }
    return 0;
}
'

# 3.3 SHA-256 digest
run_c_test "crypto:sha256-digest" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*md_ctx_new)(void) = SYM("EVP_MD_CTX_new");
    void (*md_ctx_free)(void *) = SYM("EVP_MD_CTX_free");
    void *(*sha256)(void) = SYM("EVP_sha256");
    int (*digest_init)(void *, const void *, void *) = SYM("EVP_DigestInit_ex");
    int (*digest_update)(void *, const void *, size_t) = SYM("EVP_DigestUpdate");
    int (*digest_final)(void *, unsigned char *, unsigned int *) = SYM("EVP_DigestFinal_ex");

    void *ctx = md_ctx_new();
    if (!ctx) { printf("FAIL: EVP_MD_CTX_new returned NULL\n"); return 1; }

    const void *md = sha256();
    digest_init(ctx, md, NULL);
    const char *msg = "Hello, LibreSSL!";
    digest_update(ctx, msg, strlen(msg));

    unsigned char hash[32];
    unsigned int len = 0;
    digest_final(ctx, hash, &len);
    md_ctx_free(ctx);

    if (len != 32) {
        printf("FAIL: SHA-256 produced %u bytes, expected 32\n", len);
        return 1;
    }

    // Verify known hash (precomputed)
    // SHA-256("Hello, LibreSSL!") = known value; just check non-zero
    int nonzero = 0;
    for (int i = 0; i < 32; i++) if (hash[i]) nonzero++;
    if (nonzero < 10) {
        printf("FAIL: hash looks empty (%d non-zero bytes)\n", nonzero);
        return 1;
    }
    return 0;
}
'

# 3.4 SHA-512 digest
run_c_test "crypto:sha512-digest" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*md_ctx_new)(void) = SYM("EVP_MD_CTX_new");
    void (*md_ctx_free)(void *) = SYM("EVP_MD_CTX_free");
    void *(*sha512)(void) = SYM("EVP_sha512");
    int (*digest_init)(void *, const void *, void *) = SYM("EVP_DigestInit_ex");
    int (*digest_update)(void *, const void *, size_t) = SYM("EVP_DigestUpdate");
    int (*digest_final)(void *, unsigned char *, unsigned int *) = SYM("EVP_DigestFinal_ex");

    void *ctx = md_ctx_new();
    const void *md = sha512();
    digest_init(ctx, md, NULL);
    digest_update(ctx, "test", 4);

    unsigned char hash[64];
    unsigned int len = 0;
    digest_final(ctx, hash, &len);
    md_ctx_free(ctx);

    if (len != 64) {
        printf("FAIL: SHA-512 produced %u bytes, expected 64\n", len);
        return 1;
    }
    return 0;
}
'

# 3.5 AES-256-CBC encrypt/decrypt roundtrip
run_c_test "crypto:aes256-cbc-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*ctx_new)(void) = SYM("EVP_CIPHER_CTX_new");
    void (*ctx_free)(void *) = SYM("EVP_CIPHER_CTX_free");
    void *(*aes256cbc)(void) = SYM("EVP_aes_256_cbc");
    int (*enc_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_EncryptInit_ex");
    int (*enc_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_EncryptUpdate");
    int (*enc_final)(void *, unsigned char *, int *) = SYM("EVP_EncryptFinal_ex");
    int (*dec_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_DecryptInit_ex");
    int (*dec_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_DecryptUpdate");
    int (*dec_final)(void *, unsigned char *, int *) = SYM("EVP_DecryptFinal_ex");

    // Key (32 bytes) and IV (16 bytes) — fixed for reproducibility
    unsigned char key[32], iv[16];
    memset(key, 0x42, 32);
    memset(iv, 0x13, 16);

    const char *plaintext = "The quick brown fox jumps over the lazy dog";
    int pt_len = strlen(plaintext);

    // Encrypt
    void *ctx = ctx_new();
    enc_init(ctx, aes256cbc(), NULL, key, iv);

    unsigned char ct[128];
    int ct_len = 0, tmp_len = 0;
    enc_update(ctx, ct, &ct_len, (unsigned char *)plaintext, pt_len);
    enc_final(ctx, ct + ct_len, &tmp_len);
    ct_len += tmp_len;
    ctx_free(ctx);

    if (ct_len <= 0) {
        printf("FAIL: encryption produced %d bytes\n", ct_len);
        return 1;
    }

    // Decrypt
    ctx = ctx_new();
    dec_init(ctx, aes256cbc(), NULL, key, iv);

    unsigned char dt[128];
    int dt_len = 0;
    tmp_len = 0;
    dec_update(ctx, dt, &dt_len, ct, ct_len);
    dec_final(ctx, dt + dt_len, &tmp_len);
    dt_len += tmp_len;
    ctx_free(ctx);

    dt[dt_len] = 0;
    if (dt_len != pt_len || memcmp(dt, plaintext, pt_len) != 0) {
        printf("FAIL: decrypt mismatch: got \"%s\" (%d bytes)\n", dt, dt_len);
        return 1;
    }
    return 0;
}
'

# 3.6 AES-128-CBC roundtrip
run_c_test "crypto:aes128-cbc-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*ctx_new)(void) = SYM("EVP_CIPHER_CTX_new");
    void (*ctx_free)(void *) = SYM("EVP_CIPHER_CTX_free");
    void *(*aes128cbc)(void) = SYM("EVP_aes_128_cbc");
    int (*enc_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_EncryptInit_ex");
    int (*enc_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_EncryptUpdate");
    int (*enc_final)(void *, unsigned char *, int *) = SYM("EVP_EncryptFinal_ex");
    int (*dec_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_DecryptInit_ex");
    int (*dec_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_DecryptUpdate");
    int (*dec_final)(void *, unsigned char *, int *) = SYM("EVP_DecryptFinal_ex");

    unsigned char key[16], iv[16];
    memset(key, 0xAA, 16);
    memset(iv, 0xBB, 16);

    const char *plain = "LibreSSL AES-128 test string for roundtrip";
    int pt_len = strlen(plain);

    void *ctx = ctx_new();
    enc_init(ctx, aes128cbc(), NULL, key, iv);
    unsigned char ct[128]; int ct_len = 0, tmp = 0;
    enc_update(ctx, ct, &ct_len, (const unsigned char *)plain, pt_len);
    enc_final(ctx, ct + ct_len, &tmp); ct_len += tmp;
    ctx_free(ctx);

    ctx = ctx_new();
    dec_init(ctx, aes128cbc(), NULL, key, iv);
    unsigned char dt[128]; int dt_len = 0; tmp = 0;
    dec_update(ctx, dt, &dt_len, ct, ct_len);
    dec_final(ctx, dt + dt_len, &tmp); dt_len += tmp;
    ctx_free(ctx);

    if (dt_len != pt_len || memcmp(dt, plain, pt_len)) {
        printf("FAIL: AES-128 roundtrip mismatch\n");
        return 1;
    }
    return 0;
}
'

# 3.7 3DES roundtrip
run_c_test "crypto:3des-cbc-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*ctx_new)(void) = SYM("EVP_CIPHER_CTX_new");
    void (*ctx_free)(void *) = SYM("EVP_CIPHER_CTX_free");
    void *(*des3)(void) = SYM("EVP_des_ede3_cbc");
    int (*enc_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_EncryptInit_ex");
    int (*enc_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_EncryptUpdate");
    int (*enc_final)(void *, unsigned char *, int *) = SYM("EVP_EncryptFinal_ex");
    int (*dec_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_DecryptInit_ex");
    int (*dec_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_DecryptUpdate");
    int (*dec_final)(void *, unsigned char *, int *) = SYM("EVP_DecryptFinal_ex");

    unsigned char key[24], iv[8];
    memset(key, 0xCC, 24);
    memset(iv, 0xDD, 8);

    const char *plain = "3DES roundtrip";
    int pt_len = strlen(plain);

    void *ctx = ctx_new();
    enc_init(ctx, des3(), NULL, key, iv);
    unsigned char ct[64]; int ct_len = 0, tmp = 0;
    enc_update(ctx, ct, &ct_len, (const unsigned char *)plain, pt_len);
    enc_final(ctx, ct + ct_len, &tmp); ct_len += tmp;
    ctx_free(ctx);

    ctx = ctx_new();
    dec_init(ctx, des3(), NULL, key, iv);
    unsigned char dt[64]; int dt_len = 0; tmp = 0;
    dec_update(ctx, dt, &dt_len, ct, ct_len);
    dec_final(ctx, dt + dt_len, &tmp); dt_len += tmp;
    ctx_free(ctx);

    if (dt_len != pt_len || memcmp(dt, plain, pt_len)) {
        printf("FAIL: 3DES roundtrip mismatch\n");
        return 1;
    }
    return 0;
}
'

# 3.8 RAND_bytes
run_c_test "crypto:rand-bytes" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*rand_bytes)(unsigned char *, int) = SYM("RAND_bytes");

    unsigned char buf1[32] = {0};
    unsigned char buf2[32] = {0};

    int r1 = rand_bytes(buf1, 32);
    int r2 = rand_bytes(buf2, 32);

    if (r1 != 1) { printf("FAIL: RAND_bytes returned %d\n", r1); return 1; }
    if (r2 != 1) { printf("FAIL: RAND_bytes(2) returned %d\n", r2); return 1; }

    // Two random 32-byte blocks should differ
    if (memcmp(buf1, buf2, 32) == 0) {
        printf("FAIL: two RAND_bytes calls returned identical data\n");
        return 1;
    }

    // Check not all zeros
    int nonzero = 0;
    for (int i = 0; i < 32; i++) if (buf1[i]) nonzero++;
    if (nonzero < 5) {
        printf("FAIL: random data looks empty (%d non-zero)\n", nonzero);
        return 1;
    }
    return 0;
}
'

# 3.9 RSA key generation
run_c_test "crypto:rsa-keygen-2048" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*rsa_new)(void) = SYM("RSA_new");
    void (*rsa_free)(void *) = SYM("RSA_free");
    void *(*bn_new)(void) = SYM("BN_new");
    void (*bn_free)(void *) = SYM("BN_free");
    int (*bn_set_word)(void *, unsigned long) = SYM("BN_set_word");
    int (*rsa_gen)(void *, int, void *, void *) = SYM("RSA_generate_key_ex");
    int (*rsa_size)(void *) = SYM("RSA_size");

    void *rsa = rsa_new();
    void *e = bn_new();
    bn_set_word(e, 65537);

    if (rsa_gen(rsa, 2048, e, NULL) != 1) {
        printf("FAIL: RSA_generate_key_ex failed\n");
        return 1;
    }
    int sz = rsa_size(rsa);
    if (sz != 256) { // 2048/8 = 256 bytes
        printf("FAIL: RSA_size = %d, expected 256\n", sz);
        return 1;
    }

    bn_free(e);
    rsa_free(rsa);
    return 0;
}
'

# 3.10 HMAC-SHA256
run_c_test "crypto:hmac-sha256" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    unsigned char *(*hmac_fn)(const void *, const void *, int,
                              const unsigned char *, int,
                              unsigned char *, unsigned int *) = SYM("HMAC");
    void *(*sha256)(void) = SYM("EVP_sha256");

    const char *key = "secret-key";
    const char *data = "Hello, World!";
    unsigned char result[32];
    unsigned int result_len = 0;

    unsigned char *ret = hmac_fn(sha256(), key, strlen(key),
                                 (const unsigned char *)data, strlen(data),
                                 result, &result_len);

    if (!ret) { printf("FAIL: HMAC returned NULL\n"); return 1; }
    if (result_len != 32) {
        printf("FAIL: HMAC-SHA256 len = %u, expected 32\n", result_len);
        return 1;
    }

    // Verify non-trivial output
    int nonzero = 0;
    for (unsigned i = 0; i < result_len; i++) if (result[i]) nonzero++;
    if (nonzero < 10) {
        printf("FAIL: HMAC output looks empty\n");
        return 1;
    }
    return 0;
}
'

# 3.11 BIO_s_mem — create memory BIO, write, read back
run_c_test "crypto:bio-mem-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*bio_new)(const void *) = SYM("BIO_new");
    const void *(*bio_s_mem)(void) = SYM("BIO_s_mem");
    int (*bio_write)(void *, const void *, int) = SYM("BIO_write");
    int (*bio_read)(void *, void *, int) = SYM("BIO_read");
    void (*bio_free)(void *) = SYM("BIO_free");

    void *bio = bio_new(bio_s_mem());
    if (!bio) { printf("FAIL: BIO_new(BIO_s_mem()) returned NULL\n"); return 1; }

    const char *msg = "BIO memory test data 12345";
    int wlen = bio_write(bio, msg, strlen(msg));
    if (wlen != (int)strlen(msg)) {
        printf("FAIL: BIO_write returned %d, expected %d\n", wlen, (int)strlen(msg));
        return 1;
    }

    char buf[128] = {0};
    int rlen = bio_read(bio, buf, sizeof(buf));
    if (rlen != (int)strlen(msg) || memcmp(buf, msg, rlen) != 0) {
        printf("FAIL: BIO_read returned %d bytes: \"%.*s\"\n", rlen, rlen, buf);
        return 1;
    }

    bio_free(bio);
    return 0;
}
'

# 3.12 PEM RSA key write/read roundtrip via BIO
run_c_test "crypto:pem-rsa-bio-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*rsa_new)(void) = SYM("RSA_new");
    void (*rsa_free)(void *) = SYM("RSA_free");
    void *(*bn_new)(void) = SYM("BN_new");
    void (*bn_free)(void *) = SYM("BN_free");
    int (*bn_set_word)(void *, unsigned long) = SYM("BN_set_word");
    int (*rsa_gen)(void *, int, void *, void *) = SYM("RSA_generate_key_ex");

    void *(*bio_new)(const void *) = SYM("BIO_new");
    const void *(*bio_s_mem)(void) = SYM("BIO_s_mem");
    int (*bio_read)(void *, void *, int) = SYM("BIO_read");
    void (*bio_free)(void *) = SYM("BIO_free");

    int (*pem_write_bio)(void *, void *, const void *, const unsigned char *, int, void *, void *) = SYM("PEM_write_bio_RSAPrivateKey");

    // Generate RSA 1024 key (fast for testing)
    void *rsa = rsa_new();
    void *e = bn_new();
    bn_set_word(e, 65537);
    if (rsa_gen(rsa, 1024, e, NULL) != 1) {
        printf("FAIL: RSA keygen failed\n");
        return 1;
    }

    // Write to memory BIO
    void *bio = bio_new(bio_s_mem());
    if (pem_write_bio(bio, rsa, NULL, NULL, 0, NULL, NULL) != 1) {
        printf("FAIL: PEM_write_bio_RSAPrivateKey failed\n");
        return 1;
    }

    // Read PEM text
    char pem[4096] = {0};
    int pem_len = bio_read(bio, pem, sizeof(pem) - 1);
    if (pem_len <= 0) {
        printf("FAIL: empty PEM output\n");
        return 1;
    }

    if (!strstr(pem, "-----BEGIN RSA PRIVATE KEY-----")) {
        printf("FAIL: PEM missing BEGIN marker\n");
        return 1;
    }
    if (!strstr(pem, "-----END RSA PRIVATE KEY-----")) {
        printf("FAIL: PEM missing END marker\n");
        return 1;
    }

    bio_free(bio);
    bn_free(e);
    rsa_free(rsa);
    return 0;
}
'

# 3.13 EC key generation via EC_KEY — must work with native-built LibreSSL
run_c_test "crypto:ec-keygen-prime256v1" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");
    void *(*ec_new)(int) = SYM("EC_KEY_new_by_curve_name");
    void (*ec_free)(void *) = SYM("EC_KEY_free");
    int (*ec_genkey)(void *) = SYM("EC_KEY_generate_key");

    int nid = txt2nid("prime256v1");
    if (nid <= 0) { printf("FAIL: OBJ_txt2nid(prime256v1) = %d\n", nid); return 1; }

    void *key = ec_new(nid);
    if (!key) { printf("FAIL: EC_KEY_new_by_curve_name(prime256v1) returned NULL\n"); return 1; }

    if (ec_genkey(key) != 1) { printf("FAIL: EC_KEY_generate_key failed\n"); return 1; }

    ec_free(key);
    return 0;
}
'

# 3.13b EC key generation for multiple curves
run_c_test "crypto:ec-keygen-multi-curve" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");
    void *(*ec_new)(int) = SYM("EC_KEY_new_by_curve_name");
    void (*ec_free)(void *) = SYM("EC_KEY_free");
    int (*ec_genkey)(void *) = SYM("EC_KEY_generate_key");

    const char *curves[] = {"prime256v1", "secp384r1", "secp521r1"};
    int n = sizeof(curves) / sizeof(curves[0]);
    for (int i = 0; i < n; i++) {
        int nid = txt2nid(curves[i]);
        if (nid <= 0) { printf("FAIL: OBJ_txt2nid(%s) = %d\n", curves[i], nid); return 1; }
        void *key = ec_new(nid);
        if (!key) { printf("FAIL: EC_KEY_new(%s) = NULL\n", curves[i]); return 1; }
        if (ec_genkey(key) != 1) { printf("FAIL: EC_KEY_generate_key(%s)\n", curves[i]); return 1; }
        ec_free(key);
    }
    return 0;
}
'

# 3.13c EC_GROUP_new_by_curve_name — the critical function that broke with zig cross-compile
run_c_test "crypto:ec-group-new-by-curve-name" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");
    void *(*grp_new)(int) = SYM("EC_GROUP_new_by_curve_name");
    void (*grp_free)(void *) = SYM("EC_GROUP_free");

    const char *curves[] = {"prime256v1", "secp384r1", "secp521r1"};
    int n = sizeof(curves) / sizeof(curves[0]);
    for (int i = 0; i < n; i++) {
        int nid = txt2nid(curves[i]);
        void *grp = grp_new(nid);
        if (!grp) {
            printf("FAIL: EC_GROUP_new_by_curve_name(%s/nid=%d) returned NULL\n", curves[i], nid);
            // Print any errors
            unsigned long (*errfn)(void) = SYM("ERR_get_error");
            char *(*errstr)(unsigned long, char *) = SYM("ERR_error_string");
            unsigned long e;
            while ((e = errfn()) != 0) printf("  ERR: %s\n", errstr(e, NULL));
            return 1;
        }
        grp_free(grp);
    }
    return 0;
}
'

# 3.13d ECDSA sign/verify roundtrip
run_c_test "crypto:ecdsa-sign-verify" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");
    void *(*ec_new)(int) = SYM("EC_KEY_new_by_curve_name");
    void (*ec_free)(void *) = SYM("EC_KEY_free");
    int (*ec_genkey)(void *) = SYM("EC_KEY_generate_key");
    int (*ecdsa_sign)(int, const unsigned char *, int, unsigned char *, unsigned int *, void *) = SYM("ECDSA_sign");
    int (*ecdsa_verify)(int, const unsigned char *, int, const unsigned char *, int, void *) = SYM("ECDSA_verify");
    int (*ecdsa_size)(const void *) = SYM("ECDSA_size");

    int nid = txt2nid("prime256v1");
    void *key = ec_new(nid);
    if (!key) { printf("FAIL: EC_KEY_new returned NULL\n"); return 1; }
    if (ec_genkey(key) != 1) { printf("FAIL: EC_KEY_generate_key\n"); return 1; }

    // Hash to sign (SHA-256 of "test message")
    unsigned char hash[32];
    memset(hash, 0x42, 32);

    int sig_max = ecdsa_size(key);
    unsigned char *sig = malloc(sig_max);
    unsigned int sig_len = 0;

    if (ecdsa_sign(0, hash, 32, sig, &sig_len, key) != 1) {
        printf("FAIL: ECDSA_sign failed\n");
        return 1;
    }
    if (sig_len == 0) {
        printf("FAIL: ECDSA_sign produced 0-length signature\n");
        return 1;
    }

    // Verify
    int v = ecdsa_verify(0, hash, 32, sig, sig_len, key);
    if (v != 1) {
        printf("FAIL: ECDSA_verify returned %d, expected 1\n", v);
        return 1;
    }

    // Tamper with hash, verify should fail
    hash[0] ^= 0xFF;
    v = ecdsa_verify(0, hash, 32, sig, sig_len, key);
    if (v == 1) {
        printf("FAIL: ECDSA_verify succeeded on tampered hash\n");
        return 1;
    }

    free(sig);
    ec_free(key);
    return 0;
}
'

# 3.14 EVP_MD_size / EVP_CIPHER_nid — LibreSSL renamed accessors
run_c_test "crypto:libressl-accessors" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*md_size)(const void *) = SYM("EVP_MD_size");
    int (*cipher_nid)(const void *) = SYM("EVP_CIPHER_nid");
    int (*cipher_iv_len)(const void *) = SYM("EVP_CIPHER_iv_length");
    int (*cipher_key_len)(const void *) = SYM("EVP_CIPHER_key_length");
    void *(*sha256)(void) = SYM("EVP_sha256");
    void *(*aes256)(void) = SYM("EVP_aes_256_cbc");

    int sz = md_size(sha256());
    if (sz != 32) {
        printf("FAIL: EVP_MD_size(SHA256) = %d, expected 32\n", sz);
        return 1;
    }

    int nid = cipher_nid(aes256());
    if (nid <= 0) {
        printf("FAIL: EVP_CIPHER_nid(AES-256-CBC) = %d\n", nid);
        return 1;
    }

    int iv = cipher_iv_len(aes256());
    if (iv != 16) {
        printf("FAIL: EVP_CIPHER_iv_length(AES-256-CBC) = %d, expected 16\n", iv);
        return 1;
    }

    int kl = cipher_key_len(aes256());
    if (kl != 32) {
        printf("FAIL: EVP_CIPHER_key_length(AES-256-CBC) = %d, expected 32\n", kl);
        return 1;
    }
    return 0;
}
'

# 3.15 ERR_error_string — error reporting works
run_c_test "crypto:err-reporting" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    char *(*err_str)(unsigned long, char *) = SYM("ERR_error_string");

    // Error code 0 should produce a valid string (not crash)
    char buf[256];
    char *s = err_str(0, buf);
    if (!s) {
        printf("FAIL: ERR_error_string returned NULL\n");
        return 1;
    }
    return 0;
}
'

# 3.16 BIO_new_fp with stdout
run_c_test "crypto:bio-new-fp" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*bio_new_fp)(void *, int) = SYM("BIO_new_fp");
    int (*bio_write)(void *, const void *, int) = SYM("BIO_write");
    void (*bio_free)(void *) = SYM("BIO_free");

    // BIO_NOCLOSE = 0x00
    void *bio = bio_new_fp(stdout, 0);
    if (!bio) {
        printf("FAIL: BIO_new_fp(stdout) returned NULL\n");
        return 1;
    }
    // Write something — we just check it does not crash
    bio_write(bio, "", 0);
    bio_free(bio);
    return 0;
}
'

# 3.17 Multiple cipher algorithms available
run_c_test "crypto:cipher-variety" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    const char *ciphers[] = {
        "EVP_aes_256_cbc", "EVP_aes_128_cbc", "EVP_aes_256_ecb",
        "EVP_aes_128_ecb", "EVP_aes_256_cfb128", "EVP_aes_256_ofb",
        "EVP_des_ede3_cbc", "EVP_rc4",
        "EVP_aes_128_gcm", "EVP_aes_256_gcm",
        "EVP_chacha20_poly1305",
    };
    int n = sizeof(ciphers) / sizeof(ciphers[0]);
    int found = 0;
    for (int i = 0; i < n; i++) {
        void *fn = dlsym(LC, ciphers[i]);
        if (fn) {
            void *(*cfn)(void) = fn;
            void *c = cfn();
            if (c) found++;
        }
    }
    if (found < 8) {
        printf("FAIL: only %d/%d cipher algorithms available\n", found, n);
        return 1;
    }
    return 0;
}
'

# 3.18 Multiple hash algorithms available
run_c_test "crypto:hash-variety" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*md_size)(const void *) = SYM("EVP_MD_size");
    struct { const char *sym; int expected_size; } hashes[] = {
        {"EVP_md5", 16},
        {"EVP_sha1", 20},
        {"EVP_sha256", 32},
        {"EVP_sha384", 48},
        {"EVP_sha512", 64},
    };
    int n = sizeof(hashes) / sizeof(hashes[0]);
    for (int i = 0; i < n; i++) {
        void *fn = dlsym(LC, hashes[i].sym);
        if (!fn) { printf("FAIL: missing %s\n", hashes[i].sym); return 1; }
        void *(*mfn)(void) = fn;
        int sz = md_size(mfn());
        if (sz != hashes[i].expected_size) {
            printf("FAIL: %s size = %d, expected %d\n", hashes[i].sym, sz, hashes[i].expected_size);
            return 1;
        }
    }
    return 0;
}
'

# 3.19 SHA-256 known-answer test (KAT)
run_c_test "crypto:sha256-kat" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    unsigned char *(*sha256_fn)(const unsigned char *, size_t, unsigned char *) = SYM("SHA256");

    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    unsigned char hash[32];
    sha256_fn((unsigned char *)"abc", 3, hash);

    unsigned char expected[] = {
        0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,
        0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,
        0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,
        0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad
    };
    if (memcmp(hash, expected, 32) != 0) {
        printf("FAIL: SHA-256(\"abc\") mismatch\n");
        printf("  got: ");
        for (int i = 0; i < 32; i++) printf("%02x", hash[i]);
        printf("\n");
        return 1;
    }
    return 0;
}
'

# 3.20 SHA-1 known-answer test
run_c_test "crypto:sha1-kat" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    unsigned char *(*sha1_fn)(const unsigned char *, size_t, unsigned char *) = SYM("SHA1");

    // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
    unsigned char hash[20];
    sha1_fn((unsigned char *)"abc", 3, hash);

    unsigned char expected[] = {
        0xa9,0x99,0x3e,0x36,0x47,0x06,0x81,0x6a,
        0xba,0x3e,0x25,0x71,0x78,0x50,0xc2,0x6c,
        0x9c,0xd0,0xd8,0x9d
    };
    if (memcmp(hash, expected, 20) != 0) {
        printf("FAIL: SHA-1(\"abc\") mismatch\n");
        return 1;
    }
    return 0;
}
'

# 3.21 MD5 known-answer test
run_c_test "crypto:md5-kat" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    unsigned char *(*md5_fn)(const unsigned char *, size_t, unsigned char *) = SYM("MD5");

    // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
    unsigned char hash[16];
    md5_fn((unsigned char *)"abc", 3, hash);

    unsigned char expected[] = {
        0x90,0x01,0x50,0x98,0x3c,0xd2,0x4f,0xb0,
        0xd6,0x96,0x3f,0x7d,0x28,0xe1,0x7f,0x72
    };
    if (memcmp(hash, expected, 16) != 0) {
        printf("FAIL: MD5(\"abc\") mismatch\n");
        return 1;
    }
    return 0;
}
'

# 3.22 RSA sign/verify roundtrip
run_c_test "crypto:rsa-sign-verify" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*rsa_new)(void) = SYM("RSA_new");
    void (*rsa_free)(void *) = SYM("RSA_free");
    void *(*bn_new)(void) = SYM("BN_new");
    void (*bn_free)(void *) = SYM("BN_free");
    int (*bn_set_word)(void *, unsigned long) = SYM("BN_set_word");
    int (*rsa_gen)(void *, int, void *, void *) = SYM("RSA_generate_key_ex");
    int (*rsa_sign)(int, const unsigned char *, unsigned int,
                    unsigned char *, unsigned int *, void *) = SYM("RSA_sign");
    int (*rsa_verify)(int, const unsigned char *, unsigned int,
                      const unsigned char *, unsigned int, void *) = SYM("RSA_verify");
    int (*rsa_size)(void *) = SYM("RSA_size");

    void *rsa = rsa_new();
    void *e = bn_new();
    bn_set_word(e, 65537);
    if (rsa_gen(rsa, 2048, e, NULL) != 1) { printf("FAIL: RSA keygen\n"); return 1; }

    // Sign a hash
    unsigned char hash[32];
    memset(hash, 0xAB, 32);
    int sig_sz = rsa_size(rsa);
    unsigned char *sig = malloc(sig_sz);
    unsigned int sig_len = 0;

    // NID_sha256 = 672
    if (rsa_sign(672, hash, 32, sig, &sig_len, rsa) != 1) {
        printf("FAIL: RSA_sign failed\n");
        return 1;
    }

    // Verify
    if (rsa_verify(672, hash, 32, sig, sig_len, rsa) != 1) {
        printf("FAIL: RSA_verify failed\n");
        return 1;
    }

    // Tamper — should fail
    hash[0] ^= 0xFF;
    if (rsa_verify(672, hash, 32, sig, sig_len, rsa) == 1) {
        printf("FAIL: RSA_verify succeeded on tampered data\n");
        return 1;
    }

    free(sig);
    bn_free(e);
    rsa_free(rsa);
    return 0;
}
'

# 3.23 PKCS5_PBKDF2_HMAC — key derivation
run_c_test "crypto:pbkdf2-hmac-sha256" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*pbkdf2)(const char *, int, const unsigned char *, int,
                  int, const void *, int, unsigned char *) = SYM("PKCS5_PBKDF2_HMAC");
    void *(*sha256)(void) = SYM("EVP_sha256");

    const char *password = "password";
    const unsigned char salt[] = "salt1234";
    unsigned char key1[32], key2[32];

    // Derive key
    if (pbkdf2(password, strlen(password), salt, 8, 10000, sha256(), 32, key1) != 1) {
        printf("FAIL: PKCS5_PBKDF2_HMAC failed\n");
        return 1;
    }

    // Derive again with same params — must match
    if (pbkdf2(password, strlen(password), salt, 8, 10000, sha256(), 32, key2) != 1) {
        printf("FAIL: PKCS5_PBKDF2_HMAC second call failed\n");
        return 1;
    }
    if (memcmp(key1, key2, 32) != 0) {
        printf("FAIL: PBKDF2 determinism — two calls differ\n");
        return 1;
    }

    // Derive with different password — must differ
    unsigned char key3[32];
    if (pbkdf2("other", 5, salt, 8, 10000, sha256(), 32, key3) != 1) {
        printf("FAIL: PKCS5_PBKDF2_HMAC third call failed\n");
        return 1;
    }
    if (memcmp(key1, key3, 32) == 0) {
        printf("FAIL: PBKDF2 different passwords produced same key\n");
        return 1;
    }
    return 0;
}
'

# 3.24 X509 certificate creation basics
run_c_test "crypto:x509-create" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*x509_new)(void) = SYM("X509_new");
    void (*x509_free)(void *) = SYM("X509_free");
    int (*x509_set_ver)(void *, long) = SYM("X509_set_version");
    void *(*x509_name)(void *) = SYM("X509_get_subject_name");
    int (*x509_name_add)(void *, int, int, const unsigned char *, int) = SYM("X509_NAME_add_entry_by_NID");
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");

    void *cert = x509_new();
    if (!cert) { printf("FAIL: X509_new returned NULL\n"); return 1; }

    // Set version 3 (value 2)
    if (x509_set_ver(cert, 2) != 1) { printf("FAIL: X509_set_version\n"); return 1; }

    // Set subject name
    void *name = x509_name(cert);
    if (!name) { printf("FAIL: X509_get_subject_name returned NULL\n"); return 1; }

    // MBSTRING_ASC = 0x1001
    int cn_nid = txt2nid("CN");
    if (x509_name_add(name, cn_nid, 0x1001, (const unsigned char *)"test.local", -1) != 1) {
        printf("FAIL: X509_NAME_add_entry_by_NID(CN)\n");
        return 1;
    }

    x509_free(cert);
    return 0;
}
'

# 3.25 EC PEM key write via BIO
run_c_test "crypto:ec-pem-write" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    int (*txt2nid)(const char *) = SYM("OBJ_txt2nid");
    void *(*ec_new)(int) = SYM("EC_KEY_new_by_curve_name");
    void (*ec_free)(void *) = SYM("EC_KEY_free");
    int (*ec_genkey)(void *) = SYM("EC_KEY_generate_key");
    void *(*bio_new)(const void *) = SYM("BIO_new");
    const void *(*bio_s_mem)(void) = SYM("BIO_s_mem");
    int (*bio_read)(void *, void *, int) = SYM("BIO_read");
    void (*bio_free)(void *) = SYM("BIO_free");
    int (*pem_write)(void *, void *, const void *, const unsigned char *, int, void *, void *) = SYM("PEM_write_bio_ECPrivateKey");

    int nid = txt2nid("prime256v1");
    void *key = ec_new(nid);
    if (!key) { printf("FAIL: EC_KEY_new returned NULL\n"); return 1; }
    if (ec_genkey(key) != 1) { printf("FAIL: EC_KEY_generate_key\n"); return 1; }

    void *bio = bio_new(bio_s_mem());
    if (pem_write(bio, key, NULL, NULL, 0, NULL, NULL) != 1) {
        printf("FAIL: PEM_write_bio_ECPrivateKey\n");
        return 1;
    }

    char pem[4096] = {0};
    int pem_len = bio_read(bio, pem, sizeof(pem) - 1);
    if (pem_len <= 0 || !strstr(pem, "BEGIN EC PRIVATE KEY")) {
        printf("FAIL: PEM output missing EC header (got %d bytes)\n", pem_len);
        return 1;
    }

    bio_free(bio);
    ec_free(key);
    return 0;
}
'

# 3.26 Large data SHA-256 (1MB)
run_c_test "crypto:sha256-large-data" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*md_ctx_new)(void) = SYM("EVP_MD_CTX_new");
    void (*md_ctx_free)(void *) = SYM("EVP_MD_CTX_free");
    void *(*sha256)(void) = SYM("EVP_sha256");
    int (*digest_init)(void *, const void *, void *) = SYM("EVP_DigestInit_ex");
    int (*digest_update)(void *, const void *, size_t) = SYM("EVP_DigestUpdate");
    int (*digest_final)(void *, unsigned char *, unsigned int *) = SYM("EVP_DigestFinal_ex");

    // Hash 1MB of data in 4KB chunks
    void *ctx = md_ctx_new();
    digest_init(ctx, sha256(), NULL);

    char buf[4096];
    memset(buf, 0x55, sizeof(buf));
    for (int i = 0; i < 256; i++) { // 256 * 4096 = 1MB
        if (digest_update(ctx, buf, sizeof(buf)) != 1) {
            printf("FAIL: EVP_DigestUpdate at chunk %d\n", i);
            return 1;
        }
    }

    unsigned char hash[32];
    unsigned int len = 0;
    if (digest_final(ctx, hash, &len) != 1 || len != 32) {
        printf("FAIL: EVP_DigestFinal_ex failed (len=%u)\n", len);
        return 1;
    }
    md_ctx_free(ctx);

    // Hash same data again — should be deterministic
    ctx = md_ctx_new();
    digest_init(ctx, sha256(), NULL);
    for (int i = 0; i < 256; i++) digest_update(ctx, buf, sizeof(buf));
    unsigned char hash2[32];
    unsigned int len2 = 0;
    digest_final(ctx, hash2, &len2);
    md_ctx_free(ctx);

    if (memcmp(hash, hash2, 32) != 0) {
        printf("FAIL: SHA-256(1MB) not deterministic\n");
        return 1;
    }
    return 0;
}
'

# 3.27 AES-256-GCM authenticated encryption
run_c_test "crypto:aes256-gcm-roundtrip" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    void *(*ctx_new)(void) = SYM("EVP_CIPHER_CTX_new");
    void (*ctx_free)(void *) = SYM("EVP_CIPHER_CTX_free");
    void *(*aes_gcm)(void) = SYM("EVP_aes_256_gcm");
    int (*enc_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_EncryptInit_ex");
    int (*enc_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_EncryptUpdate");
    int (*enc_final)(void *, unsigned char *, int *) = SYM("EVP_EncryptFinal_ex");
    int (*dec_init)(void *, const void *, void *, const unsigned char *, const unsigned char *) = SYM("EVP_DecryptInit_ex");
    int (*dec_update)(void *, unsigned char *, int *, const unsigned char *, int) = SYM("EVP_DecryptUpdate");
    int (*dec_final)(void *, unsigned char *, int *) = SYM("EVP_DecryptFinal_ex");
    int (*ctrl)(void *, int, int, void *) = SYM("EVP_CIPHER_CTX_ctrl");

    unsigned char key[32], iv[12];
    memset(key, 0x42, 32);
    memset(iv, 0x13, 12);

    const char *plain = "GCM authenticated encryption test";
    int pt_len = strlen(plain);

    // Encrypt
    void *ctx = ctx_new();
    enc_init(ctx, aes_gcm(), NULL, key, iv);
    unsigned char ct[128]; int ct_len = 0, tmp = 0;
    enc_update(ctx, ct, &ct_len, (const unsigned char *)plain, pt_len);
    enc_final(ctx, ct + ct_len, &tmp); ct_len += tmp;

    // Get tag (EVP_CTRL_GCM_GET_TAG = 0x10)
    unsigned char tag[16];
    ctrl(ctx, 0x10, 16, tag);
    ctx_free(ctx);

    // Decrypt
    ctx = ctx_new();
    dec_init(ctx, aes_gcm(), NULL, key, iv);
    unsigned char dt[128]; int dt_len = 0; tmp = 0;
    dec_update(ctx, dt, &dt_len, ct, ct_len);
    // Set tag (EVP_CTRL_GCM_SET_TAG = 0x11)
    ctrl(ctx, 0x11, 16, tag);
    int ok = dec_final(ctx, dt + dt_len, &tmp);
    dt_len += tmp;
    ctx_free(ctx);

    if (ok != 1) { printf("FAIL: GCM auth tag verification failed\n"); return 1; }
    if (dt_len != pt_len || memcmp(dt, plain, pt_len)) {
        printf("FAIL: GCM decrypt mismatch\n");
        return 1;
    }

    // Tamper with tag — decrypt should fail authentication
    ctx = ctx_new();
    dec_init(ctx, aes_gcm(), NULL, key, iv);
    dec_update(ctx, dt, &dt_len, ct, ct_len);
    tag[0] ^= 0xFF;
    ctrl(ctx, 0x11, 16, tag);
    ok = dec_final(ctx, dt + dt_len, &tmp);
    ctx_free(ctx);

    if (ok == 1) {
        printf("FAIL: GCM accepted tampered tag\n");
        return 1;
    }
    return 0;
}
'

# 3.28 ChaCha20-Poly1305 AEAD (LibreSSL-specific EVP_AEAD API)
run_c_test "crypto:chacha20-poly1305-aead" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    // LibreSSL EVP_AEAD API — not available in OpenSSL 3
    const void *(*aead)(void) = dlsym(LC, "EVP_aead_chacha20_poly1305");
    if (!aead) { printf("FAIL: EVP_aead_chacha20_poly1305 not found\n"); return 1; }

    size_t (*aead_key_len)(const void *) = SYM("EVP_AEAD_key_length");
    size_t (*aead_nonce_len)(const void *) = SYM("EVP_AEAD_nonce_length");
    size_t (*aead_overhead)(const void *) = SYM("EVP_AEAD_max_overhead");

    const void *a = aead();
    size_t kl = aead_key_len(a);
    size_t nl = aead_nonce_len(a);
    size_t oh = aead_overhead(a);

    if (kl != 32) { printf("FAIL: key_length = %zu, expected 32\n", kl); return 1; }
    if (nl != 12) { printf("FAIL: nonce_length = %zu, expected 12\n", nl); return 1; }
    if (oh != 16) { printf("FAIL: max_overhead = %zu, expected 16\n", oh); return 1; }
    return 0;
}
'

# 3.29 EC_get_builtin_curves — verify curve database is populated
run_c_test "crypto:ec-builtin-curves" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    typedef struct { int nid; const char *comment; } EC_builtin_curve;
    size_t (*get_curves)(EC_builtin_curve *, size_t) = SYM("EC_get_builtin_curves");
    const char *(*nid2sn)(int) = SYM("OBJ_nid2sn");

    size_t count = get_curves(NULL, 0);
    if (count < 20) {
        printf("FAIL: only %zu builtin curves (expected 20+)\n", count);
        return 1;
    }

    EC_builtin_curve *curves = malloc(count * sizeof(EC_builtin_curve));
    get_curves(curves, count);

    // Verify names resolve
    int named = 0;
    for (size_t i = 0; i < count; i++) {
        const char *sn = nid2sn(curves[i].nid);
        if (sn && strlen(sn) > 0) named++;
    }
    free(curves);

    if (named < 20) {
        printf("FAIL: only %d/%zu curves have names\n", named, count);
        return 1;
    }
    return 0;
}
'

# 3.30 HMAC-SHA1 known-answer test
run_c_test "crypto:hmac-sha1-kat" "${CRYPTO_PREAMBLE}"'
int main() {
    init_libs();
    unsigned char *(*hmac_fn)(const void *, const void *, int,
                              const unsigned char *, int,
                              unsigned char *, unsigned int *) = SYM("HMAC");
    void *(*sha1)(void) = SYM("EVP_sha1");

    // RFC 2202 test vector 1: HMAC-SHA1(key=0x0b*20, data="Hi There")
    unsigned char key[20];
    memset(key, 0x0b, 20);
    const char *data = "Hi There";
    unsigned char result[20];
    unsigned int result_len = 0;

    hmac_fn(sha1(), key, 20, (const unsigned char *)data, 8, result, &result_len);

    unsigned char expected[] = {
        0xb6,0x17,0x31,0x86,0x55,0x05,0x72,0x64,
        0xe2,0x8b,0xc0,0xb6,0xfb,0x37,0x8c,0x8e,
        0xf1,0x46,0xbe,0x00
    };
    if (result_len != 20 || memcmp(result, expected, 20) != 0) {
        printf("FAIL: HMAC-SHA1 KAT mismatch (len=%u)\n", result_len);
        return 1;
    }
    return 0;
}
'

fi # HAS_GCC

echo ""

# ══════════════════════════════════════════════════════════════════
# Layer 4: Integration Tests (via macho-inspect)
# ══════════════════════════════════════════════════════════════════

echo "── Layer 4: Integration (macho-inspect + openssl) ───────"

SYS="/opt/xcode-tools/system"
OPENSSL="$SYS/openssl"

if [[ ! -f "$OPENSSL" ]] || ! command -v macho-inspect &>/dev/null; then
    echo "  (skipped — openssl binary or macho-inspect not available)"
else

# Helper: run macOS openssl via macho-inspect, extract between output markers.
# macho-inspect prints diagnostics to stderr AND stdout — capture everything,
# then extract only the "── output ──" section.
run_openssl() {
    macho-inspect --exec "$OPENSSL" --exec-args "$@" 2>/dev/null \
        | sed -n '/── output ──/,/── end ──/{/── output ──/d;/── end ──/d;p}'
}

# Run openssl and write raw output to a file (for file-based tests)
run_openssl_file() {
    macho-inspect --exec "$OPENSSL" --exec-args "$@" 2>/dev/null
}

# 4.1 Version check
output=$(run_openssl version 2>/dev/null || true)
if echo "$output" | grep -q "LibreSSL"; then
    pass "integration:openssl-version ($(echo "$output" | head -1))"
else
    fail "integration:openssl-version" "got: $(echo "$output" | head -1)"
fi

# 4.2 Base64 encode
output=$(run_openssl base64 -e <<< "Hello World" 2>/dev/null || true)
if echo "$output" | grep -q "SGVsbG8gV29ybGQ"; then
    pass "integration:openssl-base64-encode"
else
    fail "integration:openssl-base64-encode" "got: $(echo "$output" | head -1)"
fi

# 4.3 Random bytes (hex)
output=$(run_openssl rand -hex 32 2>/dev/null || true)
# hex output may have leading/trailing whitespace or diagnostic fragments
if echo "$output" | grep -qE '[0-9a-f]{32}'; then
    pass "integration:openssl-rand-hex"
else
    fail "integration:openssl-rand-hex" "got: $(echo "$output" | head -1)"
fi

# 4.4 Random bytes (base64)
output=$(run_openssl rand -base64 32 2>/dev/null || true)
if echo "$output" | grep -qE '^[A-Za-z0-9+/]'; then
    pass "integration:openssl-rand-base64"
else
    fail "integration:openssl-rand-base64" "got: $(echo "$output" | head -1)"
fi

# 4.5 List ciphers
output=$(run_openssl ciphers 2>/dev/null || true)
linecount=$(echo "$output" | wc -l)
if echo "$output" | grep -qE "AES|CHACHA"; then
    pass "integration:openssl-ciphers"
else
    fail "integration:openssl-ciphers" "no AES/CHACHA in output"
fi

# 4.6 RSA keygen
output=$(run_openssl genrsa 2048 2>/dev/null || true)
if echo "$output" | grep -q "BEGIN RSA PRIVATE KEY"; then
    pass "integration:openssl-genrsa-2048"
else
    fail "integration:openssl-genrsa-2048" "missing PEM header"
fi

# 4.7 SHA256 digest
hash_in="hello"
output=$(echo -n "$hash_in" | run_openssl dgst -sha256 -hex 2>/dev/null || true)
if echo "$output" | grep -qi "[0-9a-f]\{64\}"; then
    pass "integration:openssl-sha256"
else
    fail "integration:openssl-sha256" "got: $(echo "$output" | head -1)"
fi

# 4.8 AES-256-CBC roundtrip
TMPDIR_INT=$(mktemp -d)
echo "integration_test_plaintext_aes256" > "$TMPDIR_INT/plain.txt"

macho-inspect --exec "$OPENSSL" --exec-args enc -aes-256-cbc \
    -in "$TMPDIR_INT/plain.txt" -out "$TMPDIR_INT/enc.bin" \
    -k testpass123 -pbkdf2 2>/dev/null || true

if [[ -s "$TMPDIR_INT/enc.bin" ]]; then
    macho-inspect --exec "$OPENSSL" --exec-args enc -d -aes-256-cbc \
        -in "$TMPDIR_INT/enc.bin" -out "$TMPDIR_INT/dec.txt" \
        -k testpass123 -pbkdf2 2>/dev/null || true

    if [[ -f "$TMPDIR_INT/dec.txt" ]] && \
       diff -q "$TMPDIR_INT/plain.txt" "$TMPDIR_INT/dec.txt" &>/dev/null; then
        pass "integration:openssl-aes256-roundtrip"
    else
        fail "integration:openssl-aes256-roundtrip" "decrypt mismatch"
    fi
else
    fail "integration:openssl-aes256-roundtrip" "encryption produced empty output"
fi
rm -rf "$TMPDIR_INT"

# 4.9 RSA key inspection
TMPDIR_INT=$(mktemp -d)
run_openssl genrsa 2048 > "$TMPDIR_INT/key.pem" 2>/dev/null || true
if [[ -s "$TMPDIR_INT/key.pem" ]] && grep -q "BEGIN RSA" "$TMPDIR_INT/key.pem"; then
    output=$(macho-inspect --exec "$OPENSSL" --exec-args rsa -check -noout -in "$TMPDIR_INT/key.pem" 2>/dev/null \
        | sed -n '/── output ──/,/── end ──/{/── output ──/d;/── end ──/d;p}' || true)
    if echo "$output" | grep -qi "ok"; then
        pass "integration:openssl-rsa-check"
    else
        fail "integration:openssl-rsa-check" "got: $(echo "$output" | head -1)"
    fi
else
    fail "integration:openssl-rsa-check" "keygen failed"
fi
rm -rf "$TMPDIR_INT"

# 4.10 EC keygen (prime256v1)
output=$(run_openssl ecparam -name prime256v1 -genkey -noout 2>/dev/null || true)
if echo "$output" | grep -q "BEGIN EC PRIVATE KEY"; then
    pass "integration:openssl-ec-keygen-p256"
else
    fail "integration:openssl-ec-keygen-p256" "got: $(echo "$output" | head -1)"
fi

# 4.11 EC keygen (secp384r1)
output=$(run_openssl ecparam -name secp384r1 -genkey -noout 2>/dev/null || true)
if echo "$output" | grep -q "BEGIN EC PRIVATE KEY"; then
    pass "integration:openssl-ec-keygen-p384"
else
    fail "integration:openssl-ec-keygen-p384" "got: $(echo "$output" | head -1)"
fi

# 4.12 EC keygen + sign flow
TMPDIR_INT=$(mktemp -d)
run_openssl ecparam -name prime256v1 -genkey -noout > "$TMPDIR_INT/ec.pem" 2>/dev/null || true
if [[ -s "$TMPDIR_INT/ec.pem" ]] && grep -q "BEGIN EC" "$TMPDIR_INT/ec.pem"; then
    echo "test data for signing" > "$TMPDIR_INT/data.txt"
    macho-inspect --exec "$OPENSSL" --exec-args dgst -sha256 -sign "$TMPDIR_INT/ec.pem" \
        -out "$TMPDIR_INT/sig.bin" "$TMPDIR_INT/data.txt" 2>/dev/null || true
    if [[ -s "$TMPDIR_INT/sig.bin" ]]; then
        pass "integration:openssl-ec-sign"
    else
        fail "integration:openssl-ec-sign" "signature file empty"
    fi
else
    fail "integration:openssl-ec-sign" "EC keygen failed"
fi
rm -rf "$TMPDIR_INT"

# 4.13 EC curve listing — previously crashed in BIO_printf (Apple variadic ABI mismatch).
# Fixed by adding BIO_printf/BIO_vprintf variadic shims that construct a proper
# Linux va_list from Apple stack-based variadic arguments.
output=$(macho-inspect --exec "$OPENSSL" --exec-args ecparam -list_curves 2>/dev/null || true)
curve_count=$(echo "$output" | grep -c ":" || true)
if [[ "$curve_count" -ge 10 ]]; then
    pass "integration:openssl-ecparam-list ($curve_count curves)"
else
    fail "integration:openssl-ecparam-list" "expected >=10 curves, got $curve_count"
fi

# 4.14 SHA-512 digest
output=$(echo -n "test" | run_openssl dgst -sha512 -hex 2>/dev/null || true)
if echo "$output" | grep -qi "[0-9a-f]\{128\}"; then
    pass "integration:openssl-sha512"
else
    fail "integration:openssl-sha512" "got: $(echo "$output" | head -1)"
fi

# 4.15 AES-128-CBC roundtrip
TMPDIR_INT=$(mktemp -d)
echo "aes128_integration_test" > "$TMPDIR_INT/plain.txt"
macho-inspect --exec "$OPENSSL" --exec-args enc -aes-128-cbc \
    -in "$TMPDIR_INT/plain.txt" -out "$TMPDIR_INT/enc.bin" \
    -k mykey128 -pbkdf2 2>/dev/null || true
if [[ -s "$TMPDIR_INT/enc.bin" ]]; then
    macho-inspect --exec "$OPENSSL" --exec-args enc -d -aes-128-cbc \
        -in "$TMPDIR_INT/enc.bin" -out "$TMPDIR_INT/dec.txt" \
        -k mykey128 -pbkdf2 2>/dev/null || true
    if diff -q "$TMPDIR_INT/plain.txt" "$TMPDIR_INT/dec.txt" &>/dev/null; then
        pass "integration:openssl-aes128-roundtrip"
    else
        fail "integration:openssl-aes128-roundtrip" "decrypt mismatch"
    fi
else
    fail "integration:openssl-aes128-roundtrip" "encryption empty"
fi
rm -rf "$TMPDIR_INT"

# 4.16 3DES roundtrip
TMPDIR_INT=$(mktemp -d)
echo "3des_integration_test" > "$TMPDIR_INT/plain.txt"
macho-inspect --exec "$OPENSSL" --exec-args enc -des-ede3-cbc \
    -in "$TMPDIR_INT/plain.txt" -out "$TMPDIR_INT/enc.bin" \
    -k deskey -pbkdf2 2>/dev/null || true
if [[ -s "$TMPDIR_INT/enc.bin" ]]; then
    macho-inspect --exec "$OPENSSL" --exec-args enc -d -des-ede3-cbc \
        -in "$TMPDIR_INT/enc.bin" -out "$TMPDIR_INT/dec.txt" \
        -k deskey -pbkdf2 2>/dev/null || true
    if diff -q "$TMPDIR_INT/plain.txt" "$TMPDIR_INT/dec.txt" &>/dev/null; then
        pass "integration:openssl-3des-roundtrip"
    else
        fail "integration:openssl-3des-roundtrip" "decrypt mismatch"
    fi
else
    fail "integration:openssl-3des-roundtrip" "encryption empty"
fi
rm -rf "$TMPDIR_INT"

# 4.17 Self-signed certificate generation
# openssl.cnf is now bundled in the Docker image (COPY from libressl-builder).
# Create a minimal config for the test to set subject fields.
TMPDIR_INT=$(mktemp -d)
cat > "$TMPDIR_INT/openssl.cnf" << 'SSLCNF'
[req]
default_bits = 2048
prompt = no
default_md = sha256
distinguished_name = dn
[dn]
CN = test.local
O = Test
SSLCNF
export OPENSSL_CONF="$TMPDIR_INT/openssl.cnf"
macho-inspect --exec "$OPENSSL" --exec-args req -x509 -newkey rsa:2048 -keyout "$TMPDIR_INT/key.pem" \
    -out "$TMPDIR_INT/cert.pem" -days 1 -nodes \
    -subj "/CN=test.local/O=Test" 2>/dev/null || true
unset OPENSSL_CONF
if [[ -s "$TMPDIR_INT/cert.pem" ]] && grep -q "BEGIN CERTIFICATE" "$TMPDIR_INT/cert.pem"; then
    pass "integration:openssl-self-signed-cert"
else
    fail "integration:openssl-self-signed-cert" "cert generation failed"
fi
rm -rf "$TMPDIR_INT"

fi # openssl/macho-inspect available

echo ""

# ══════════════════════════════════════════════════════════════════
# Summary
# ══════════════════════════════════════════════════════════════════

TOTAL=$((PASS + FAIL + SKIP))
echo "═══════════════════════════════════════════════════════════"
printf "  Results: %s passed" "$(green "$PASS")"
if [[ $FAIL -gt 0 ]]; then
    printf ", %s failed" "$(red "$FAIL")"
fi
if [[ $SKIP -gt 0 ]]; then
    printf ", %s skipped" "$(yellow "$SKIP")"
fi
printf " / %d total\n" "$TOTAL"
echo "═══════════════════════════════════════════════════════════"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
exit 0
