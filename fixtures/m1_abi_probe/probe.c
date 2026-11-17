/* Source-owned M1 probe. Build products and execution records stay local. */
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

static char exit_nonce[65];

static void record_exit(void) {
    printf("BURDETTIE_ABI_V1 nonce=%s exit_cleanup=pass\n", exit_nonce);
}

struct thread_input {
    const unsigned char *bytes;
    size_t size;
    unsigned long checksum;
};

static void *checksum_thread(void *opaque) {
    struct thread_input *input = opaque;
    unsigned long checksum = 0;
    size_t i;
    for (i = 0; i < input->size; ++i) {
        checksum += input->bytes[i];
    }
    input->checksum = checksum;
    return input;
}

static int check_nonce(const char *nonce) {
    size_t i, length = strlen(nonce);
    if (length == 0 || length > 64) return 0;
    for (i = 0; i < length; ++i) {
        unsigned char c = (unsigned char)nonce[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
              (c >= '0' && c <= '9') || c == '-' || c == '_')) return 0;
    }
    return 1;
}

static int verify_file(const char *path, const char *expected, size_t length) {
    char buffer[160];
    size_t used = 0;
    ssize_t count;
    int fd = open(path, O_RDONLY);
    if (fd < 0) return -1;
    for (;;) {
        count = read(fd, buffer + used, sizeof(buffer) - used);
        if (count < 0 && errno == EINTR) continue;
        if (count < 0) { close(fd); return -1; }
        if (count == 0) break;
        used += (size_t)count;
        /* A too-long file must fail, even when its prefix matches. */
        if (used > length) { close(fd); errno = EIO; return -1; }
    }
    if (close(fd) != 0) return -1;
    if (used != length || memcmp(buffer, expected, length) != 0) {
        errno = EIO;
        return -1;
    }
    return 0;
}

static int write_file(const char *path, const char *bytes, size_t length) {
    size_t done = 0;
    int fd = open(path, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd < 0) return -1; /* Never replace an earlier run's evidence. */
    while (done < length) {
        ssize_t count = write(fd, bytes + done, length - done);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) { close(fd); errno = EIO; return -1; }
        done += (size_t)count;
    }
    if (fsync(fd) != 0) { close(fd); return -1; }
    return close(fd);
}

int main(int argc, char **argv) {
    char expected[160];
    unsigned char *allocation, *grown;
    size_t i, length;
    unsigned long expected_checksum = 0;
    int count, error;
    pthread_t thread;
    void *returned = NULL;
    struct thread_input input;

    if (argc != 4 || (strcmp(argv[1], "write") != 0 && strcmp(argv[1], "read") != 0)
        || !check_nonce(argv[3])) {
        fprintf(stderr, "usage: m1_abi_probe {write|read} NEW_OR_RETAINED_FILE NONCE\n");
        return 2;
    }
    count = snprintf(expected, sizeof(expected),
                     "Burdettie M1 ABI v1\nnonce=%s\nunicode=caf\xc3\xa9 \xe9\x9b\xaa\n", argv[3]);
    if (count < 0 || (size_t)count >= sizeof(expected)) return 3;
    length = (size_t)count;
    allocation = malloc(length);
    if (allocation == NULL) return 4;
    memcpy(allocation, expected, length);
    grown = realloc(allocation, length + 1);
    if (grown == NULL) { free(allocation); return 4; }
    allocation = grown;
    allocation[length] = 0;
    if (memcmp(allocation, expected, length) != 0) { free(allocation); return 4; }

    input.bytes = allocation;
    input.size = length;
    input.checksum = 0;
    for (i = 0; i < length; ++i) expected_checksum += allocation[i];
    error = pthread_create(&thread, NULL, checksum_thread, &input);
    if (error != 0) {
        fprintf(stderr, "pthread_create: %d\n", error);
        free(allocation);
        return 5;
    }
    error = pthread_join(thread, &returned);
    if (error != 0) {
        /* The thread may still use its input; process exit owns cleanup. */
        fprintf(stderr, "pthread_join: %d\n", error);
        return 5;
    }
    if (returned != &input || input.checksum != expected_checksum) {
        free(allocation);
        return 5;
    }
    if (strcmp(argv[1], "write") == 0 && write_file(argv[2], expected, length) != 0) {
        perror("write/fsync/close");
        free(allocation);
        return 6;
    }
    if (verify_file(argv[2], expected, length) != 0) {
        perror("read/compare/close");
        free(allocation);
        return 7;
    }
    free(allocation);
    memcpy(exit_nonce, argv[3], strlen(argv[3]) + 1);
    if (atexit(record_exit) != 0) return 8;
    printf("BURDETTIE_ABI_V1 nonce=%s mode=%s allocation=pass file=pass thread=pass\n",
           argv[3], argv[1]);
    /* The caller must separately observe exit status 0 and the atexit marker. */
    return 0;
}
