/* Source-owned qualification probe; never a sandbox backend or policy adapter.
 * Invoke with a NEW absolute fixture directory and a unique ASCII nonce.
 * All fixture data is retained. No Apple sandbox_check query is used as proof.
 */
#define _POSIX_C_SOURCE 200809L
#define _XOPEN_SOURCE 700
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <sandbox.h>
#endif

#define CASES 8
#define ROLES 2
#define PATH_CAP 4096
#define IPC_MAGIC UINT32_C(0x42535031)
#define NOT_ATTEMPTED (-2)
#define TIMEOUT_SECONDS 10

static const char profile[] =
    "(version 1) (allow default) "
    "(deny file-read-data file-write-data file-write-create "
    "(subpath (param \"DENY_ROOT\")))";
static const char *const case_names[CASES] = {
    "allow_read", "allow_write", "allow_create", "deny_read_open",
    "deny_write_open", "deny_create_open", "preopened_read", "preopened_write"
};
static const char *const operations[CASES] = {
    "read", "write", "write", "read", "write", "write", "pread", "pwrite"
};
static const char *const role_names[ROLES] = { "worker", "descendant" };
static const char *const file_names[CASES] = {
    "read", "write", "create", "read", "write", "create", "pre-read", "pre-write"
};
static volatile sig_atomic_t stopped;
static volatile sig_atomic_t own_group;
static char seed[128];
static size_t seed_length;

struct observation {
    int open_rc, open_errno, io_rc, io_errno, content_ok, sync_errno, close_errno;
};
struct role_result { struct observation cases[CASES]; };
struct worker_result {
    uint32_t magic;
    int native_api, profile_rc, profile_errno, setup_errno, completed_roles;
    int worker_pid, descendant_pid, descendant_exit;
    struct role_result roles[ROLES];
};
struct phase_result {
    int received, wait_exit, wait_signal;
    struct worker_result result;
};

static void stop_group(int signal_number) {
    if (stopped && signal_number == SIGALRM) {
        if (own_group > 0) (void)kill(-(pid_t)own_group, SIGKILL);
        _exit(124);
    }
    if (!stopped) {
        stopped = signal_number;
        /* Parent and its only forked workers share this private group. Workers
         * restore SIGTERM's default action; the parent can retain a result.
         * The second alarm also bounds a worker that cannot finish promptly. */
        (void)alarm(2);
        if (own_group > 0) (void)kill(-(pid_t)own_group, SIGTERM);
    }
}

static int private_group(void) {
    struct sigaction action;
    pid_t self = getpid();
    if (getpgrp() != self && setpgid(0, 0) != 0) return -1;
    if (getpgrp() != self) { errno = EPERM; return -1; }
    own_group = (sig_atomic_t)self;
    memset(&action, 0, sizeof(action));
    action.sa_handler = stop_group;
    sigemptyset(&action.sa_mask);
    sigaddset(&action.sa_mask, SIGTERM);
    sigaddset(&action.sa_mask, SIGINT);
    sigaddset(&action.sa_mask, SIGALRM);
    if (sigaction(SIGTERM, &action, NULL) || sigaction(SIGINT, &action, NULL)
        || sigaction(SIGALRM, &action, NULL)) return -1;
    return 0;
}

static int valid_nonce(const char *value) {
    size_t n = strlen(value), i;
    if (!n || n > 64) return 0;
    for (i = 0; i < n; ++i) {
        unsigned char c = (unsigned char)value[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
              (c >= '0' && c <= '9') || c == '-' || c == '_')) return 0;
    }
    return 1;
}

static int path_for(char output[PATH_CAP], const char *root, int role, int which) {
    int n = snprintf(output, PATH_CAP, "%s/%s/%s-%s", root,
                     which < 3 ? "allow" : "deny", role_names[role], file_names[which]);
    if (n < 0 || n >= PATH_CAP) { errno = ENAMETOOLONG; return -1; }
    return 0;
}

static int transfer(int fd, void *buffer, size_t bytes, int writing) {
    unsigned char *cursor = buffer;
    size_t done = 0;
    while (done < bytes) {
        ssize_t n = writing ? write(fd, cursor + done, bytes - done)
                            : read(fd, cursor + done, bytes - done);
        if (n < 0 && errno == EINTR && !stopped) continue;
        if (n <= 0) return -1;
        done += (size_t)n;
    }
    return 0;
}

static int create_phase(const char *root) {
    char path[PATH_CAP];
    int role, which, fd, saved;
    if (mkdir(root, 0700) != 0) return -1;
    for (which = 0; which < 2; ++which) {
        int n = snprintf(path, sizeof(path), "%s/%s", root, which ? "deny" : "allow");
        if (n < 0 || (size_t)n >= sizeof(path)) { errno = ENAMETOOLONG; return -1; }
        if (mkdir(path, 0700) != 0) return -1;
    }
    for (role = 0; role < ROLES; ++role) {
        for (which = 0; which < CASES; ++which) {
            if (which == 2 || which == 5) continue; /* Create cases stay absent. */
            if (path_for(path, root, role, which)) return -1;
            fd = open(path, O_WRONLY | O_CREAT | O_EXCL, 0600);
            if (fd < 0) return -1;
            if (transfer(fd, seed, seed_length, 1) || fsync(fd)) {
                saved = errno; (void)close(fd); errno = saved; return -1;
            }
            if (close(fd)) return -1;
        }
    }
    return 0;
}

static void observe(struct observation *value, const char *path, int which, int retained_fd) {
    char buffer[sizeof(seed)];
    const char marker = 'M';
    int fd = retained_fd, writing = which == 1 || which == 2 || which == 4 || which == 5 || which == 7;
    ssize_t count;
    memset(value, 0, sizeof(*value));
    value->open_rc = value->io_rc = NOT_ATTEMPTED;
    if (which < 6) {
        int flags = writing ? O_WRONLY : O_RDONLY;
        if (which == 2 || which == 5) flags |= O_CREAT | O_EXCL;
        errno = 0;
        fd = open(path, flags, 0600);
        value->open_errno = fd < 0 ? errno : 0;
    }
    value->open_rc = fd < 0 ? -1 : 0;
    if (fd < 0) return;
    errno = 0;
    if (writing) count = which == 7 ? pwrite(fd, &marker, 1, 0) : write(fd, &marker, 1);
    else count = which == 6 ? pread(fd, buffer, sizeof(buffer), 0) : read(fd, buffer, sizeof(buffer));
    value->io_rc = (int)count;
    value->io_errno = count < 0 ? errno : 0;
    value->content_ok = writing ? count == 1 :
        count == (ssize_t)seed_length && memcmp(buffer, seed, seed_length) == 0;
    if (writing && count >= 0 && fsync(fd) != 0) value->sync_errno = errno;
    if (close(fd) != 0) value->close_errno = errno;
}

static int run_role(struct role_result *result, const char *root, int role, int retained[ROLES][2]) {
    char path[PATH_CAP];
    int which;
    for (which = 0; which < CASES; ++which) {
        if (path_for(path, root, role, which)) return -1;
        observe(&result->cases[which], path, which, which >= 6 ? retained[role][which - 6] : -1);
        /* observe closes even a retained descriptor. Clear its slot before a
         * later pipe/open can reuse that number; cleanup must not close it twice. */
        if (which >= 6) retained[role][which - 6] = -1;
    }
    return 0;
}

static void worker(int report_fd, const char *root, int restricted) {
    struct worker_result result;
    int retained[ROLES][2] = { { -1, -1 }, { -1, -1 } };
    int role, index, channel[2], status = 0;
    pid_t child;
    char path[PATH_CAP], deny_root[PATH_CAP];
    memset(&result, 0, sizeof(result));
    result.magic = IPC_MAGIC;
    result.worker_pid = (int)getpid();
    result.profile_rc = NOT_ATTEMPTED;
    result.descendant_exit = NOT_ATTEMPTED;
    (void)signal(SIGTERM, SIG_DFL);
    (void)signal(SIGINT, SIG_DFL);
    (void)signal(SIGALRM, SIG_DFL);
    (void)alarm(0);
    for (role = 0; role < ROLES; ++role) {
        for (index = 0; index < 2; ++index) {
            if (path_for(path, root, role, index + 6)) goto setup_failed;
            retained[role][index] = open(path, index ? O_WRONLY : O_RDONLY);
            if (retained[role][index] < 0) goto setup_failed;
        }
    }
    if (restricted) {
        int n = snprintf(deny_root, sizeof(deny_root), "%s/deny", root);
        if (n < 0 || (size_t)n >= sizeof(deny_root)) { errno = ENAMETOOLONG; goto setup_failed; }
#if defined(__APPLE__)
        {
            char *error = NULL;
            /* Both parameter names and the canonical nonempty value are fixed
             * here. Missing parameters in the pinned compiler match nothing. */
            const char *parameters[] = { "DENY_ROOT", deny_root, NULL };
            result.native_api = 1;
            errno = 0;
            result.profile_rc = sandbox_init_with_parameters(profile, 0, parameters, &error);
            result.profile_errno = result.profile_rc ? errno : 0;
            sandbox_free_error(error);
        }
#else
        /* An unavailable API is an explicit nonzero blocked result, never a
         * simulated sandbox or a missing-fixture test skip. */
        (void)profile;
        result.profile_rc = -1;
        result.profile_errno = ENOSYS;
#endif
        if (result.profile_rc != 0) goto done;
    }
    if (run_role(&result.roles[0], root, 0, retained)) goto setup_failed;
    result.completed_roles = 1;
    if (pipe(channel)) goto setup_failed;
    child = fork();
    if (child < 0) { (void)close(channel[0]); (void)close(channel[1]); goto setup_failed; }
    if (child == 0) {
        struct role_result descendant;
        int code;
        (void)close(channel[0]);
        (void)close(report_fd);
        memset(&descendant, 0, sizeof(descendant));
        code = run_role(&descendant, root, 1, retained);
        if (!code) code = transfer(channel[1], &descendant, sizeof(descendant), 1);
        (void)close(channel[1]);
        _exit(code ? 3 : 0);
    }
    result.descendant_pid = (int)child;
    (void)close(channel[1]);
    index = transfer(channel[0], &result.roles[1], sizeof(result.roles[1]), 0);
    (void)close(channel[0]);
    while (waitpid(child, &status, 0) < 0) { if (errno != EINTR) goto setup_failed; }
    result.descendant_exit = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    if (index || result.descendant_exit != 0) { errno = EIO; goto setup_failed; }
    result.completed_roles = 2;
    goto done;
setup_failed:
    result.setup_errno = errno ? errno : EIO;
done:
    for (role = 0; role < ROLES; ++role)
        for (index = 0; index < 2; ++index)
            if (retained[role][index] >= 0) (void)close(retained[role][index]);
    index = result.setup_errno ? 3 : restricted && result.profile_rc != 0 ? 2 : 0;
    if (transfer(report_fd, &result, sizeof(result), 1)) index = 3;
    (void)close(report_fd);
    _exit(index);
}

static int run_phase(struct phase_result *phase, const char *root, int restricted) {
    int channel[2], status = 0, received;
    pid_t child, waited;
    memset(phase, 0, sizeof(*phase));
    phase->wait_exit = NOT_ATTEMPTED;
    phase->result.profile_rc = phase->result.descendant_exit = NOT_ATTEMPTED;
    if (pipe(channel)) return -1;
    child = fork();
    if (child < 0) { (void)close(channel[0]); (void)close(channel[1]); return -1; }
    if (child == 0) { (void)close(channel[0]); worker(channel[1], root, restricted); }
    (void)close(channel[1]);
    (void)alarm(TIMEOUT_SECONDS);
    received = transfer(channel[0], &phase->result, sizeof(phase->result), 0);
    (void)close(channel[0]);
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    if (!stopped) (void)alarm(0);
    phase->received = !received && phase->result.magic == IPC_MAGIC
                      && phase->result.worker_pid == (int)child;
    if (waited == child) {
        phase->wait_exit = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
        phase->wait_signal = WIFSIGNALED(status) ? WTERMSIG(status) : 0;
    }
    return waited == child && phase->received ? 0 : -1;
}

static int allowed(const struct observation *value, int which) {
    return value->open_rc == 0 && value->open_errno == 0 && value->io_errno == 0
        && value->io_rc == (which == 0 || which == 3 || which == 6 ? (int)seed_length : 1)
        && value->content_ok == 1 && value->sync_errno == 0 && value->close_errno == 0;
}

static int denied(const struct observation *value, int which) {
    if (which < 6) return value->open_rc == -1 && value->open_errno == EPERM
        && value->io_rc == NOT_ATTEMPTED && value->io_errno == 0 && value->content_ok == 0
        && value->sync_errno == 0 && value->close_errno == 0;
    return value->open_rc == 0 && value->open_errno == 0 && value->io_rc == -1
        && value->io_errno == EPERM && value->content_ok == 0
        && value->sync_errno == 0 && value->close_errno == 0;
}

static int complete(const struct phase_result *phase) {
    return phase->received && phase->wait_exit == 0 && phase->wait_signal == 0
        && phase->result.setup_errno == 0 && phase->result.completed_roles == ROLES
        && phase->result.descendant_pid > 0 && phase->result.descendant_exit == 0;
}

static int cases_pass(const struct phase_result *phase, int restricted) {
    int role, which;
    if (!complete(phase)) return 0;
    for (role = 0; role < ROLES; ++role)
        for (which = 0; which < CASES; ++which)
            if (!(restricted && which >= 3 ? denied(&phase->result.roles[role].cases[which], which)
                                           : allowed(&phase->result.roles[role].cases[which], which))) return 0;
    return 1;
}

static int unchanged(const char *root) {
    char path[PATH_CAP], data[sizeof(seed)];
    int role, which, fd;
    struct stat info;
    for (role = 0; role < ROLES; ++role) {
        for (which = 4; which <= 7; ++which) {
            if (which == 6) continue;
            if (path_for(path, root, role, which)) return 0;
            if (which == 5) {
                if (lstat(path, &info) == 0 || errno != ENOENT) return 0;
            } else {
                ssize_t count;
                if (lstat(path, &info) || !S_ISREG(info.st_mode)) return 0;
                fd = open(path, O_RDONLY | O_NOFOLLOW);
                if (fd < 0) return 0;
                count = read(fd, data, sizeof(data));
                if (close(fd) || count != (ssize_t)seed_length || memcmp(data, seed, seed_length)) return 0;
            }
        }
    }
    return 1;
}

static void print_phase(const char *name, const struct phase_result *phase) {
    int role, which;
    const struct worker_result *r = &phase->result;
    printf("\"%s\":{\"received\":%d,\"worker_pid\":%d,\"worker_exit\":%d,\"worker_signal\":%d,"
           "\"native_api\":%d,\"profile_rc\":%d,\"profile_errno\":%d,\"setup_errno\":%d,"
           "\"descendant_pid\":%d,\"descendant_exit\":%d,\"roles\":[", name, phase->received,
           r->worker_pid, phase->wait_exit, phase->wait_signal, r->native_api, r->profile_rc,
           r->profile_errno, r->setup_errno, r->descendant_pid, r->descendant_exit);
    for (role = 0; role < r->completed_roles && role < ROLES; ++role) {
        if (role) putchar(',');
        printf("{\"role\":\"%s\",\"cases\":[", role_names[role]);
        for (which = 0; which < CASES; ++which) {
            const struct observation *o = &r->roles[role].cases[which];
            if (which) putchar(',');
            printf("{\"case\":\"%s\",\"operation\":\"%s\",\"open_rc\":%d,\"open_errno\":%d,"
                   "\"io_rc\":%d,\"io_errno\":%d,\"content_ok\":%d,\"sync_errno\":%d,\"close_errno\":%d}",
                   case_names[which], operations[which], o->open_rc, o->open_errno,
                   o->io_rc, o->io_errno, o->content_ok, o->sync_errno, o->close_errno);
        }
        printf("]}");
    }
    printf("]}");
}

int main(int argc, char **argv) {
    char root[PATH_CAP], baseline_path[PATH_CAP], restricted_path[PATH_CAP];
    struct phase_result baseline, restricted;
    const char *status = "blocked_setup";
    int code = 2, baseline_ok = 0, unchanged_ok = 0, saved = 0, n;
    memset(&baseline, 0, sizeof(baseline));
    memset(&restricted, 0, sizeof(restricted));
    baseline.wait_exit = restricted.wait_exit = NOT_ATTEMPTED;
    baseline.result.profile_rc = baseline.result.descendant_exit = NOT_ATTEMPTED;
    restricted.result.profile_rc = restricted.result.descendant_exit = NOT_ATTEMPTED;
    if (argc != 3 || argv[1][0] != '/' || !valid_nonce(argv[2]) || strlen(argv[1]) > PATH_CAP - 256) {
        fprintf(stderr, "usage: m1_sandbox_probe NEW_ABSOLUTE_DIRECTORY ASCII_NONCE\n");
        return 2;
    }
    if (private_group()) { perror("private process group"); return 2; }
    (void)umask(0077);
    n = snprintf(seed, sizeof(seed), "Burdettie sandbox probe nonce=%s\n", argv[2]);
    if (n < 0 || (size_t)n >= sizeof(seed)) return 2;
    seed_length = (size_t)n;
    if (mkdir(argv[1], 0700) || realpath(argv[1], root) == NULL) goto finish;
    n = snprintf(baseline_path, sizeof(baseline_path), "%s/baseline", root);
    if (n < 0 || (size_t)n >= sizeof(baseline_path)) { errno = ENAMETOOLONG; goto finish; }
    n = snprintf(restricted_path, sizeof(restricted_path), "%s/restricted", root);
    if (n < 0 || (size_t)n >= sizeof(restricted_path)) { errno = ENAMETOOLONG; goto finish; }
    if (create_phase(baseline_path) || create_phase(restricted_path)) goto finish;
    if (run_phase(&baseline, baseline_path, 0)) { status = "baseline_incomplete"; goto finish; }
    if (!complete(&baseline)) { status = "baseline_incomplete"; goto finish; }
    baseline_ok = cases_pass(&baseline, 0);
    if (!baseline_ok) { status = "baseline_failed"; goto finish; }
    if (run_phase(&restricted, restricted_path, 1)) { status = "probe_incomplete"; goto finish; }
    unchanged_ok = unchanged(restricted_path);
    if (restricted.result.setup_errno) { status = "probe_incomplete"; goto finish; }
    if (restricted.result.profile_rc != 0) { status = "blocked_policy"; goto finish; }
    if (!complete(&restricted)) { status = "probe_incomplete"; goto finish; }
    code = cases_pass(&restricted, 1) && unchanged_ok ? 0 : 1;
    status = code == 0 ? "passed_limited_probe" : "coverage_gap";
finish:
    saved = strcmp(status, "blocked_setup") == 0 ? errno : 0;
    if (stopped) { status = "interrupted"; code = 2; }
    (void)alarm(0);
    printf("{\"schema_version\":1,\"scope\":\"native_puredarwin_file_probe\","
           "\"nonce\":\"%s\",\"status\":\"%s\",\"exit_code\":%d,\"parent_pid\":%d,\"process_group\":%d,"
           "\"seed_bytes\":%lu,\"denial_errno\":%d,\"baseline_pass\":%d,\"forbidden_files_unchanged\":%d,"
           "\"setup_errno\":%d,\"interrupted_signal\":%d,\"qualified_capabilities\":[],",
           argv[2], status, code, (int)getpid(), (int)getpgrp(), (unsigned long)seed_length,
           EPERM, baseline_ok, unchanged_ok, saved, (int)stopped);
    print_phase("baseline", &baseline);
    putchar(',');
    print_phase("restricted", &restricted);
    puts("}");
    return code;
}
