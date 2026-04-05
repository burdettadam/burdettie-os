/*
 * libressl_compat.c — LibreSSL-to-OpenSSL 3 compatibility shim.
 *
 * macOS ships LibreSSL whose API predates OpenSSL 3's renames/removals.
 * This .so provides the deprecated LibreSSL symbol names by resolving
 * OpenSSL 3 functions at runtime via dlsym, avoiding header conflicts.
 *
 * Build (inside Dockerfile, Linux ARM64):
 *   gcc -shared -fPIC -O2 -o /usr/local/lib/libressl_compat.so \
 *       docker/libressl_compat.c -ldl -Wl,--no-as-needed
 */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <time.h>

/* Opaque types — avoids OpenSSL headers entirely */
typedef struct ssl_st SSL;
typedef struct ssl_ctx_st SSL_CTX;
typedef struct ssl_cipher_st SSL_CIPHER;
typedef struct evp_md_st EVP_MD;
typedef struct evp_md_ctx_st EVP_MD_CTX;
typedef struct evp_cipher_st EVP_CIPHER;
typedef struct evp_cipher_ctx_st EVP_CIPHER_CTX;
typedef struct evp_pkey_st EVP_PKEY;
typedef struct hmac_ctx_st HMAC_CTX;
typedef struct x509_st X509;
typedef struct X509_name_st X509_NAME;
typedef struct X509_crl_st X509_CRL;
typedef struct x509_store_ctx_st X509_STORE_CTX;
typedef struct x509_object_st X509_OBJECT;
typedef struct asn1_string_st ASN1_TIME;
typedef struct bio_st BIO;
typedef struct rsa_meth_st RSA_METHOD;
typedef struct stack_st OPENSSL_STACK;
typedef struct lhash_st OPENSSL_LHASH;
typedef struct engine_st ENGINE;

static void *sym(const char *name) { return dlsym(RTLD_DEFAULT, name); }

/* ── Renamed / removed ─────────────────────────────────────────── */

int SSL_library_init(void) {
    int (*fn)(uint64_t, void*) = sym("OPENSSL_init_ssl");
    return fn ? fn(0, NULL) : 0;
}

void SSL_load_error_strings(void) {
    int (*fn)(uint64_t, void*) = sym("OPENSSL_init_ssl");
    if (fn) fn(0x00200000ULL | 0x02000000ULL, NULL);
}

void EVP_cleanup(void) {}
void CRYPTO_cleanup_all_ex_data(void) {}
void ERR_free_strings(void) {}
void OBJ_cleanup(void) {}

void OPENSSL_add_all_algorithms_noconf(void) {
    int (*fn)(uint64_t, void*) = sym("OPENSSL_init_crypto");
    if (fn) fn(0x04 | 0x08, NULL);
}

long SSLeay(void) {
    long (*fn)(void) = sym("OpenSSL_version_num");
    return fn ? fn() : 0;
}

const char *SSLeay_version(int t) {
    const char *(*fn)(int) = sym("OpenSSL_version");
    return fn ? fn(t) : "unknown";
}

X509 *SSL_get_peer_certificate(const SSL *s) {
    X509 *(*fn)(const SSL*) = sym("SSL_get1_peer_certificate");
    return fn ? fn(s) : NULL;
}

const RSA_METHOD *RSA_PKCS1_SSLeay(void) {
    const RSA_METHOD *(*fn)(void) = sym("RSA_PKCS1_OpenSSL");
    return fn ? fn() : NULL;
}

/* ── Accessor renames ──────────────────────────────────────────── */

int EVP_MD_size(const EVP_MD *md) { int (*f)(const EVP_MD*)=sym("EVP_MD_get_size"); return f?f(md):0; }
int EVP_MD_type(const EVP_MD *md) { int (*f)(const EVP_MD*)=sym("EVP_MD_get_type"); return f?f(md):0; }
unsigned long EVP_MD_flags(const EVP_MD *md) { unsigned long (*f)(const EVP_MD*)=sym("EVP_MD_get_flags"); return f?f(md):0; }
int EVP_CIPHER_nid(const EVP_CIPHER *c) { int (*f)(const EVP_CIPHER*)=sym("EVP_CIPHER_get_nid"); return f?f(c):0; }
int EVP_CIPHER_key_length(const EVP_CIPHER *c) { int (*f)(const EVP_CIPHER*)=sym("EVP_CIPHER_get_key_length"); return f?f(c):0; }
int EVP_CIPHER_iv_length(const EVP_CIPHER *c) { int (*f)(const EVP_CIPHER*)=sym("EVP_CIPHER_get_iv_length"); return f?f(c):0; }
int EVP_PKEY_id(const EVP_PKEY *p) { int (*f)(const EVP_PKEY*)=sym("EVP_PKEY_get_id"); return f?f(p):0; }
int EVP_PKEY_base_id(const EVP_PKEY *p) { int (*f)(const EVP_PKEY*)=sym("EVP_PKEY_get_base_id"); return f?f(p):0; }
int EVP_PKEY_bits(const EVP_PKEY *p) { int (*f)(const EVP_PKEY*)=sym("EVP_PKEY_get_bits"); return f?f(p):0; }
int EVP_PKEY_size(const EVP_PKEY *p) { int (*f)(const EVP_PKEY*)=sym("EVP_PKEY_get_size"); return f?f(p):0; }
uint16_t SSL_CIPHER_get_value(const SSL_CIPHER *c) { uint16_t (*f)(const SSL_CIPHER*)=sym("SSL_CIPHER_get_protocol_id"); return f?f(c):0; }
int SSL_state(const SSL *s) { int (*f)(const SSL*)=sym("SSL_get_state"); return f?f(s):0; }
int SSL_cache_hit(SSL *s) { int (*f)(SSL*)=sym("SSL_session_reused"); return f?f(s):0; }

/* ── CTX init/cleanup renames ──────────────────────────────────── */

void EVP_MD_CTX_init(EVP_MD_CTX *c) { int (*f)(EVP_MD_CTX*)=sym("EVP_MD_CTX_reset"); if(f) f(c); }
int EVP_MD_CTX_cleanup(EVP_MD_CTX *c) { int (*f)(EVP_MD_CTX*)=sym("EVP_MD_CTX_reset"); return f?f(c):0; }
void EVP_CIPHER_CTX_init(EVP_CIPHER_CTX *c) { int (*f)(EVP_CIPHER_CTX*)=sym("EVP_CIPHER_CTX_reset"); if(f) f(c); }
int EVP_CIPHER_CTX_cleanup(EVP_CIPHER_CTX *c) { int (*f)(EVP_CIPHER_CTX*)=sym("EVP_CIPHER_CTX_reset"); return f?f(c):0; }
void HMAC_CTX_init(HMAC_CTX *c) { int (*f)(HMAC_CTX*)=sym("HMAC_CTX_reset"); if(f) f(c); }
void HMAC_CTX_cleanup(HMAC_CTX *c) { int (*f)(HMAC_CTX*)=sym("HMAC_CTX_reset"); if(f) f(c); }

/* ── X509 renames ──────────────────────────────────────────────── */

int X509_CRL_set_lastUpdate(X509_CRL *x, const ASN1_TIME *t) {
    int (*f)(X509_CRL*,const ASN1_TIME*)=sym("X509_CRL_set1_lastUpdate"); return f?f(x,t):0;
}
int X509_CRL_set_nextUpdate(X509_CRL *x, const ASN1_TIME *t) {
    int (*f)(X509_CRL*,const ASN1_TIME*)=sym("X509_CRL_set1_nextUpdate"); return f?f(x,t):0;
}
unsigned long X509_NAME_hash(X509_NAME *x) {
    unsigned long (*f)(X509_NAME*,void*,void*,void*)=sym("X509_NAME_hash_ex"); return f?f(x,NULL,NULL,NULL):0;
}

/* ── SSL protocol version ──────────────────────────────────────── */

long SSL_CTX_set_min_proto_version(SSL_CTX *ctx, int v) {
    long (*f)(SSL_CTX*,int,long,void*)=sym("SSL_CTX_ctrl"); return f?f(ctx,123,v,NULL):0;
}
long SSL_CTX_set_max_proto_version(SSL_CTX *ctx, int v) {
    long (*f)(SSL_CTX*,int,long,void*)=sym("SSL_CTX_ctrl"); return f?f(ctx,124,v,NULL):0;
}
long SSL_CTX_set1_groups_list(SSL_CTX *ctx, const char *l) {
    long (*f)(SSL_CTX*,int,long,void*)=sym("SSL_CTX_ctrl"); return f?f(ctx,92,0,(void*)l):0;
}

/* ── EVP_AEAD stubs (LibreSSL-only) ────────────────────────────── */

typedef struct { int d; } EVP_AEAD;
typedef struct { int d; } EVP_AEAD_CTX;

static const EVP_AEAD stub_aead_128 = {0}, stub_aead_256 = {0}, stub_aead_chacha = {0};

const EVP_AEAD *EVP_aead_aes_128_gcm(void) { return &stub_aead_128; }
const EVP_AEAD *EVP_aead_aes_256_gcm(void) { return &stub_aead_256; }
const EVP_AEAD *EVP_aead_chacha20_poly1305(void) { return &stub_aead_chacha; }
size_t EVP_AEAD_key_length(const EVP_AEAD *a) { (void)a; return 0; }
size_t EVP_AEAD_nonce_length(const EVP_AEAD *a) { (void)a; return 0; }

int EVP_AEAD_CTX_init(EVP_AEAD_CTX *c, const EVP_AEAD *a,
    const unsigned char *k, size_t kl, size_t tl, ENGINE *e) {
    (void)c;(void)a;(void)k;(void)kl;(void)tl;(void)e; return 0;
}
void EVP_AEAD_CTX_cleanup(EVP_AEAD_CTX *c) { (void)c; }
int EVP_AEAD_CTX_seal(const EVP_AEAD_CTX *c,
    unsigned char *o, size_t *ol, size_t mo,
    const unsigned char *n, size_t nl,
    const unsigned char *i, size_t il,
    const unsigned char *a, size_t al) {
    (void)c;(void)o;(void)ol;(void)mo;(void)n;(void)nl;(void)i;(void)il;(void)a;(void)al;
    return 0;
}

/* ── NETSCAPE/RSA_NET stubs ───────────────────────────────────── */

void NETSCAPE_X509_free(void *a) { (void)a; }
const void *NETSCAPE_X509_it = NULL;
void *d2i_RSA_NET(void **a,const unsigned char **p,long l,void *cb,int s) { (void)a;(void)p;(void)l;(void)cb;(void)s; return NULL; }
int i2d_RSA_NET(const void *a,unsigned char **p,void *cb,int s) { (void)a;(void)p;(void)cb;(void)s; return -1; }

/* ── STACK wrappers ────────────────────────────────────────────── */

OPENSSL_STACK *sk_new_null(void) { OPENSSL_STACK*(*f)(void)=sym("OPENSSL_sk_new_null"); return f?f():NULL; }
int sk_push(OPENSSL_STACK *s,const void *d) { int(*f)(OPENSSL_STACK*,const void*)=sym("OPENSSL_sk_push"); return f?f(s,d):0; }
void sk_pop_free(OPENSSL_STACK *s,void(*fn)(void*)) { void(*f)(OPENSSL_STACK*,void(*)(void*))=sym("OPENSSL_sk_pop_free"); if(f)f(s,fn); }
void sk_free(OPENSSL_STACK *s) { void(*f)(OPENSSL_STACK*)=sym("OPENSSL_sk_free"); if(f)f(s); }
int sk_num(const OPENSSL_STACK *s) { int(*f)(const OPENSSL_STACK*)=sym("OPENSSL_sk_num"); return f?f(s):-1; }
void *sk_value(const OPENSSL_STACK *s,int i) { void*(*f)(const OPENSSL_STACK*,int)=sym("OPENSSL_sk_value"); return f?f(s,i):NULL; }
void *sk_delete(OPENSSL_STACK *s,int i) { void*(*f)(OPENSSL_STACK*,int)=sym("OPENSSL_sk_delete"); return f?f(s,i):NULL; }
void *sk_shift(OPENSSL_STACK *s) { void*(*f)(OPENSSL_STACK*)=sym("OPENSSL_sk_shift"); return f?f(s):NULL; }

/* ── LHASH wrappers ───────────────────────────────────────────── */

OPENSSL_LHASH *lh_new(void *h,void *c) { OPENSSL_LHASH*(*f)(void*,void*)=sym("OPENSSL_LH_new"); return f?f(h,c):NULL; }
void lh_free(OPENSSL_LHASH *l) { void(*f)(OPENSSL_LHASH*)=sym("OPENSSL_LH_free"); if(f)f(l); }
void *lh_insert(OPENSSL_LHASH *l,void *d) { void*(*f)(OPENSSL_LHASH*,void*)=sym("OPENSSL_LH_insert"); return f?f(l,d):NULL; }
void *lh_retrieve(OPENSSL_LHASH *l,const void *d) { void*(*f)(OPENSSL_LHASH*,const void*)=sym("OPENSSL_LH_retrieve"); return f?f(l,d):NULL; }
void lh_doall(OPENSSL_LHASH *l,void(*fn)(void*)) { void(*f)(OPENSSL_LHASH*,void(*)(void*))=sym("OPENSSL_LH_doall"); if(f)f(l,fn); }
unsigned long lh_strhash(const char *c) { unsigned long(*f)(const char*)=sym("OPENSSL_LH_strhash"); return f?f(c):0; }
void lh_node_stats_bio(const OPENSSL_LHASH *l,BIO *b) { void(*f)(const OPENSSL_LHASH*,BIO*)=sym("OPENSSL_LH_node_stats_bio"); if(f)f(l,b); }
void lh_node_usage_stats_bio(const OPENSSL_LHASH *l,BIO *b) { void(*f)(const OPENSSL_LHASH*,BIO*)=sym("OPENSSL_LH_node_usage_stats_bio"); if(f)f(l,b); }
void lh_stats_bio(const OPENSSL_LHASH *l,BIO *b) { void(*f)(const OPENSSL_LHASH*,BIO*)=sym("OPENSSL_LH_stats_bio"); if(f)f(l,b); }

/* ── BSD/misc ──────────────────────────────────────────────────── */

void explicit_bzero(void *buf, size_t len) {
    void (*f)(void*,size_t)=sym("OPENSSL_cleanse"); if(f) f(buf,len); else memset(buf,0,len);
}

void freezero(void *ptr, size_t size) { if(ptr){memset(ptr,0,size);free(ptr);} }

void *reallocarray(void *ptr, size_t n, size_t s) {
    if(n!=0 && s>(size_t)-1/n) return NULL; return realloc(ptr,n*s);
}

long long strtonum(const char *np, long long mn, long long mx, const char **ep) {
    char *e; long long v;
    if(ep) *ep=NULL;
    if(mn>mx){if(ep)*ep="invalid";return 0;}
    v=strtoll(np,&e,10);
    if(np==e||*e!='\0'){if(ep)*ep="invalid";return 0;}
    if(v<mn){if(ep)*ep="too small";return 0;}
    if(v>mx){if(ep)*ep="too large";return 0;}
    return v;
}

char *string_to_hex(const char *str, long *len) {
    if(!str) return NULL;
    size_t sl=strlen(str); if(sl%2) return NULL;
    size_t bl=sl/2;
    unsigned char *b=malloc(bl); if(!b) return NULL;
    for(size_t i=0;i<bl;i++){ unsigned v; if(sscanf(str+i*2,"%2x",&v)!=1){free(b);return NULL;} b[i]=(unsigned char)v; }
    if(len) *len=(long)bl;
    return (char*)b;
}

/* ── ASN1 time ─────────────────────────────────────────────────── */

int ASN1_TIME_set_tm(void *s, struct tm *tm) {
    void*(*f)(void*,time_t)=sym("ASN1_TIME_set"); time_t t=mktime(tm); return f?(f(s,t)?1:0):0;
}
int ASN1_time_parse(const char *b,size_t l,struct tm *t,int m) { (void)b;(void)l;(void)t;(void)m; return -1; }

/* ── OCSP ──────────────────────────────────────────────────────── */

int OCSP_parse_url(const char *u,char **h,char **po,char **pa,int *ssl) { (void)u;(void)h;(void)po;(void)pa;(void)ssl; return 0; }
void OCSP_REQ_CTX_free(void *r) { (void)r; }
int OCSP_REQ_CTX_set1_req(void *r,void *q) { (void)r;(void)q; return 0; }
int OCSP_REQ_CTX_add1_header(void *r,const char *n,const char *v) { (void)r;(void)n;(void)v; return 0; }
int OCSP_sendreq_nbio(void **p,void *r) { (void)p;(void)r; return -1; }

/* ── X509 store ────────────────────────────────────────────────── */

void X509_OBJECT_free_contents(void *a) { (void)a; }
int X509_STORE_get_by_subject(X509_STORE_CTX *v,int t,X509_NAME *n,X509_OBJECT *r) {
    int(*f)(X509_STORE_CTX*,int,X509_NAME*,X509_OBJECT*)=sym("X509_STORE_CTX_get_by_subject"); return f?f(v,t,n,r):0;
}
void X509_STORE_CTX_trusted_stack(X509_STORE_CTX *c,void *s) {
    void(*f)(X509_STORE_CTX*,void*)=sym("X509_STORE_CTX_set0_trusted_stack"); if(f)f(c,s);
}

/* ── BIO DER helpers ───────────────────────────────────────────── */

void *d2i_DHparams_bio(BIO *bp,void **x) {
    void*(*f)(void*(*)(void),void*(*)(void**,const unsigned char**,long),BIO*,void**)=sym("ASN1_d2i_bio");
    void*(*n)(void)=sym("DH_new"); void*(*d)(void**,const unsigned char**,long)=sym("d2i_DHparams");
    return (f&&n&&d)?f(n,(void*)d,bp,x):NULL;
}
int i2d_DHparams_bio(BIO *bp,const void *x) {
    int(*f)(int(*)(const void*,unsigned char**),BIO*,const void*)=sym("ASN1_i2d_bio");
    int(*i)(const void*,unsigned char**)=sym("i2d_DHparams"); return (f&&i)?f(i,bp,x):-1;
}
void *d2i_DSAparams_bio(BIO *bp,void **x) {
    void*(*f)(void*(*)(void),void*(*)(void**,const unsigned char**,long),BIO*,void**)=sym("ASN1_d2i_bio");
    void*(*n)(void)=sym("DSA_new"); void*(*d)(void**,const unsigned char**,long)=sym("d2i_DSAparams");
    return (f&&n&&d)?f(n,(void*)d,bp,x):NULL;
}
int i2d_DSAparams_bio(BIO *bp,const void *x) {
    int(*f)(int(*)(const void*,unsigned char**),BIO*,const void*)=sym("ASN1_i2d_bio");
    int(*i)(const void*,unsigned char**)=sym("i2d_DSAparams"); return (f&&i)?f(i,bp,x):-1;
}
void *d2i_OCSP_REQUEST_bio(BIO *bp,void **a) {
    void*(*f)(void*(*)(void),void*(*)(void**,const unsigned char**,long),BIO*,void**)=sym("ASN1_d2i_bio");
    void*(*n)(void)=sym("OCSP_REQUEST_new"); void*(*d)(void**,const unsigned char**,long)=sym("d2i_OCSP_REQUEST");
    return (f&&n&&d)?f(n,(void*)d,bp,a):NULL;
}
int i2d_OCSP_REQUEST_bio(BIO *bp,const void *a) {
    int(*f)(int(*)(const void*,unsigned char**),BIO*,const void*)=sym("ASN1_i2d_bio");
    int(*i)(const void*,unsigned char**)=sym("i2d_OCSP_REQUEST"); return (f&&i)?f(i,bp,a):-1;
}
void *d2i_OCSP_RESPONSE_bio(BIO *bp,void **a) {
    void*(*f)(void*(*)(void),void*(*)(void**,const unsigned char**,long),BIO*,void**)=sym("ASN1_d2i_bio");
    void*(*n)(void)=sym("OCSP_RESPONSE_new"); void*(*d)(void**,const unsigned char**,long)=sym("d2i_OCSP_RESPONSE");
    return (f&&n&&d)?f(n,(void*)d,bp,a):NULL;
}
int i2d_OCSP_RESPONSE_bio(BIO *bp,const void *a) {
    int(*f)(int(*)(const void*,unsigned char**),BIO*,const void*)=sym("ASN1_i2d_bio");
    int(*i)(const void*,unsigned char**)=sym("i2d_OCSP_RESPONSE"); return (f&&i)?f(i,bp,a):-1;
}

/* ── PKCS12 ─────────────────────────────────────────────────────── */

X509 *PKCS12_certbag2x509(void *bag) { (void)bag; return NULL; }

/* ── Misc ───────────────────────────────────────────────────────── */

void OpenSSLDie(const char *f, int l, const char *a) {
    fprintf(stderr, "OpenSSL internal error: %s:%d: %s\n", f, l, a);
    abort();
}

void *ERR_get_string_table(void) { return NULL; }
