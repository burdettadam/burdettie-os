/* Source-owned, child-process-only Linux mechanism probe; not a sandbox launcher. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/landlock.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/uio.h>
#include <sys/wait.h>
#include <unistd.h>

static void result(const char *name, long value, int error) {
    printf("{\"probe\":\"%s\",\"result\":%ld,\"errno\":%d}\n", name, value, error);
    fflush(stdout);
}

static void open_probe(const char *name, const char *path, int raw) {
    errno = 0;
    int fd = raw ? syscall(SYS_openat, AT_FDCWD, path, O_RDONLY, 0)
                 : open(path, O_RDONLY);
    int error = errno;
    result(name, fd >= 0 ? 0 : -1, error);
    if (fd >= 0) close(fd);
}

static int wait_child(pid_t child) {
    int status;
    if (child < 0 || waitpid(child, &status, 0) < 0) return 1;
    return WIFEXITED(status) ? WEXITSTATUS(status) : 1;
}

static int landlock_probe(const char *allowed, const char *denied) {
    errno = 0;
    int abi = syscall(SYS_landlock_create_ruleset, NULL, 0,
                      LANDLOCK_CREATE_RULESET_VERSION);
    result("landlock_abi", abi, errno);
    if (abi < 1) return 77;
    open_probe("landlock_before_denied_file_open", denied, 0);
    int inherited = open(denied, O_RDONLY);
    int allowed_path = open(allowed, O_PATH | O_CLOEXEC);
    if (inherited < 0 || allowed_path < 0) return 1;
    struct landlock_ruleset_attr rules = {
        .handled_access_fs = LANDLOCK_ACCESS_FS_READ_FILE |
                             LANDLOCK_ACCESS_FS_WRITE_FILE,
    };
    int ruleset = syscall(SYS_landlock_create_ruleset, &rules, sizeof(rules), 0);
    if (ruleset < 0) return 1;
    struct landlock_path_beneath_attr path_rule = {
        .allowed_access = LANDLOCK_ACCESS_FS_READ_FILE,
        .parent_fd = allowed_path,
    };
    if (syscall(SYS_landlock_add_rule, ruleset, LANDLOCK_RULE_PATH_BENEATH,
                &path_rule, 0) < 0) return 1;
    pid_t child = fork();
    if (child == 0) {
        if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) _exit(1);
        errno = 0;
        int rc = syscall(SYS_landlock_restrict_self, ruleset, 0);
        result("landlock_restrict_child", rc, errno);
        close(ruleset);
        close(allowed_path);
        if (rc < 0) _exit(77);
        open_probe("landlock_allowed_file_open", allowed, 0);
        open_probe("landlock_denied_file_open", denied, 0);
        open_probe("landlock_denied_file_raw_openat", denied, 1);
        char byte;
        errno = 0;
        ssize_t n = read(inherited, &byte, 1);
        result("landlock_inherited_fd_read", n, errno);
        close(inherited);
        pid_t grandchild = fork();
        if (grandchild == 0) {
            open_probe("landlock_descendant_denied_file_open", denied, 1);
            _exit(0);
        }
        _exit(wait_child(grandchild));
    }
    close(inherited);
    close(allowed_path);
    close(ruleset);
    return wait_child(child);
}

static int seccomp_probe(void) {
#if defined(__x86_64__)
    const unsigned int arch = AUDIT_ARCH_X86_64;
#elif defined(__aarch64__)
    const unsigned int arch = AUDIT_ARCH_AARCH64;
#else
    result("seccomp_arch_unsupported", -1, ENOTSUP);
    return 77;
#endif
    errno = 0;
    long before = syscall(SYS_getppid);
    result("seccomp_before_getppid", before > 0 ? 0 : -1, errno);
    pid_t child = fork();
    if (child == 0) {
        struct sock_filter filter[] = {
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, arch, 1, 0),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_getppid, 0, 1),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        };
        struct sock_fprog program = { .len = sizeof(filter) / sizeof(filter[0]),
                                     .filter = filter };
        if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) _exit(1);
        errno = 0;
        int rc = prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program);
        result("seccomp_install_child", rc, errno);
        if (rc < 0) _exit(77);
        errno = 0;
        long v = syscall(SYS_getppid);
        result("seccomp_denied_getppid", v, errno);
        errno = 0;
        v = syscall(SYS_getpid);
        result("seccomp_allowed_getpid", v > 0 ? 0 : -1, errno);
        pid_t grandchild = fork();
        if (grandchild == 0) {
            errno = 0;
            v = syscall(SYS_getppid);
            result("seccomp_descendant_denied_getppid", v, errno);
            _exit(0);
        }
        _exit(wait_child(grandchild));
    }
    return wait_child(child);
}

static void memory_read_probe(const char *name, pid_t target, const char *address) {
    char copy = 0;
    struct iovec local = { .iov_base = &copy, .iov_len = 1 };
    struct iovec remote = { .iov_base = (void *)address, .iov_len = 1 };
    errno = 0;
    long n = process_vm_readv(target, &local, 1, &remote, 1, 0);
    result(name, n, errno);
}

static int landlock_memory_probe(void) {
    /* Only this fixture's own memory/process is exposed. No application data. */
    const char marker = 'p';
    pid_t parent = getpid();
    memory_read_probe("memory_self", parent, &marker);
    errno = 0;
    int rc = prctl(PR_SET_PTRACER, PR_SET_PTRACER_ANY, 0, 0, 0);
    result("fixture_allow_ptracer", rc, errno);
    pid_t child = fork();
    if (child == 0) {
        memory_read_probe("memory_before_landlock_parent", parent, &marker);
        struct landlock_ruleset_attr rules = {
            .handled_access_fs = LANDLOCK_ACCESS_FS_WRITE_FILE,
        };
        errno = 0;
        int ruleset = syscall(SYS_landlock_create_ruleset, &rules, sizeof(rules), 0);
        result("memory_landlock_create", ruleset >= 0 ? 0 : -1, errno);
        if (ruleset < 0 || prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) _exit(77);
        errno = 0;
        rc = syscall(SYS_landlock_restrict_self, ruleset, 0);
        result("memory_landlock_restrict_child", rc, errno);
        close(ruleset);
        if (rc < 0) _exit(77);
        memory_read_probe("memory_after_landlock_parent", parent, &marker);
        _exit(0);
    }
    rc = wait_child(child);
    prctl(PR_SET_PTRACER, 0, 0, 0, 0);
    return rc;
}

static int executable_mapping_probe(void) {
#if defined(__x86_64__)
    const unsigned int arch = AUDIT_ARCH_X86_64;
#elif defined(__aarch64__)
    const unsigned int arch = AUDIT_ARCH_AARCH64;
#else
    return 77;
#endif
    pid_t child = fork();
    if (child == 0) {
        /* Narrow demonstration of a W^X-style filter, NOT a launch policy. */
        struct sock_filter filter[] = {
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, arch)),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, arch, 1, 0),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS),
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_mprotect, 0, 3),
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2])),
            BPF_JUMP(BPF_JMP | BPF_JSET | BPF_K, PROT_EXEC, 0, 7),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, SYS_mmap, 0, 5),
            BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, args[2])),
            BPF_STMT(BPF_ALU | BPF_AND | BPF_K, PROT_WRITE | PROT_EXEC),
            BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, PROT_WRITE | PROT_EXEC, 0, 2),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ERRNO | EPERM),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
            BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),
        };
        struct sock_fprog program = { .len = sizeof(filter) / sizeof(filter[0]),
                                     .filter = filter };
        if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0) _exit(1);
        errno = 0;
        int rc = prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &program);
        result("wx_filter_install_child", rc, errno);
        if (rc < 0) _exit(77);
        errno = 0;
        void *rwx = mmap(NULL, 4096, PROT_READ | PROT_WRITE | PROT_EXEC,
                         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        result("wx_denied_rwx_mapping", rwx == MAP_FAILED ? -1 : 0, errno);
        if (rwx != MAP_FAILED) munmap(rwx, 4096);
        void *rw = mmap(NULL, 4096, PROT_READ | PROT_WRITE,
                       MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (rw == MAP_FAILED) _exit(1);
        errno = 0;
        rc = mprotect(rw, 4096, PROT_READ | PROT_EXEC);
        result("wx_denied_mprotect_rx", rc, errno);
        munmap(rw, 4096);
        errno = 0;
        int fd = syscall(SYS_memfd_create, "burdettie-source-owned-probe", 0);
        result("wx_memfd_create", fd >= 0 ? 0 : -1, errno);
        if (fd < 0) _exit(77);
        if (ftruncate(fd, 4096) != 0) _exit(1);
        errno = 0;
        void *write_alias = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        result("wx_memfd_write_alias", write_alias == MAP_FAILED ? -1 : 0, errno);
        errno = 0;
        void *exec_alias = mmap(NULL, 4096, PROT_READ | PROT_EXEC, MAP_SHARED, fd, 0);
        result("wx_memfd_exec_alias", exec_alias == MAP_FAILED ? -1 : 0, errno);
        /* No machine instructions are written or executed. */
        if (write_alias != MAP_FAILED) munmap(write_alias, 4096);
        if (exec_alias != MAP_FAILED) munmap(exec_alias, 4096);
        close(fd);
        _exit(0);
    }
    return wait_child(child);
}

int main(int argc, char **argv) {
    if (argc == 4 && !strcmp(argv[1], "landlock"))
        return landlock_probe(argv[2], argv[3]);
    if (argc == 2 && !strcmp(argv[1], "seccomp")) return seccomp_probe();
    if (argc == 2 && !strcmp(argv[1], "memory")) return landlock_memory_probe();
    if (argc == 2 && !strcmp(argv[1], "executable-memory")) return executable_mapping_probe();
    fprintf(stderr, "usage: %s landlock ALLOWED DENIED | seccomp | memory | executable-memory\n", argv[0]);
    return 2;
}
