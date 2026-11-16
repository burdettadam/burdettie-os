/* M1 POSIX thread-specific state and lifecycle probe. No VM/security claim. */
#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#define WORKERS 3
#define ROUNDS 32
#define BATCHES 2

#if defined(__APPLE__)
#define PLATFORM "darwin"
#elif defined(__linux__)
#define PLATFORM "linux"
#else
#define PLATFORM "other"
#endif
#if defined(__aarch64__) || defined(__arm64__)
#define ARCHITECTURE "arm64"
#elif defined(__x86_64__)
#define ARCHITECTURE "x86_64"
#else
#define ARCHITECTURE "other"
#endif

struct worker {
    int id, failures, tls_checks, errno_checks, rounds, destructors, pthread_errors;
    int *errno_slot;
};
struct totals {
    int started, joined, failures, tls_checks, errno_checks, rounds;
    int destructors, join_checks, pthread_errors, errno_address_checks;
};
static pthread_once_t once = PTHREAD_ONCE_INIT;
static int once_calls;
static pthread_key_t key;
static pthread_mutex_t mutex;
static pthread_cond_t condition;
/* All accesses to the following three fields hold mutex. */
static int arrived, epoch, stopped;
static struct timespec deadline;

static void must_succeed(int code) {
    /* Returning while a worker uses this image/state would be unsafe. */
    if (code != 0) {
        fputs("M1_THREAD_STATE_FATAL\n", stderr);
        abort();
    }
}
static void initialize_once(void) { ++once_calls; }
static void destroy_value(void *value) {
    struct worker *w = value;
    ++w->destructors;
    ++w->tls_checks;
    /* POSIX clears the key before invoking its destructor. */
    if (pthread_getspecific(key) != NULL) ++w->failures;
}
static void *work(void *value) {
    struct worker *w = value;
    ++w->tls_checks;
    if (pthread_getspecific(key) != NULL) ++w->failures;
    if (pthread_once(&once, initialize_once) != 0) {
        ++w->failures;
        ++w->pthread_errors;
    }
    if (pthread_setspecific(key, w) != 0) {
        ++w->failures;
        ++w->pthread_errors;
    }
    ++w->tls_checks;
    if (pthread_getspecific(key) != w) ++w->failures;
    w->errno_slot = &errno;
    must_succeed(pthread_mutex_lock(&mutex));
    for (int round = 0; round < ROUNDS && !stopped; ++round) {
        ++arrived;
        must_succeed(pthread_cond_broadcast(&condition));
        while (epoch == round && !stopped) {
            int rc = pthread_cond_timedwait(&condition, &mutex, &deadline);
            if (rc != 0) {
                ++w->failures;
                ++w->pthread_errors;
                stopped = 1;
                must_succeed(pthread_cond_broadcast(&condition));
            }
        }
        if (stopped) break;
        ++w->tls_checks;
        if (pthread_getspecific(key) != w) ++w->failures;
        /* No library call separates this write/read. POSIX does not promise
         * that arbitrary successful calls preserve errno. */
        volatile int *slot = &errno;
        *slot = 1100 + w->id;
        ++w->errno_checks;
        if (*slot != 1100 + w->id || slot != w->errno_slot) ++w->failures;
        ++w->rounds;
    }
    must_succeed(pthread_mutex_unlock(&mutex));
    return w;
}

static void batch(struct totals *t, int number) {
    pthread_t threads[WORKERS];
    struct worker workers[WORKERS] = {{0}};
    struct worker parent = {0};
    int created = 0;
    arrived = epoch = stopped = 0;
    parent.errno_slot = &errno;
    /* CLOCK_REALTIME matches a condition variable with default attributes. */
    if (clock_gettime(CLOCK_REALTIME, &deadline) != 0) {
        ++t->failures;
        return;
    }
    deadline.tv_sec += 2;
    must_succeed(pthread_mutex_init(&mutex, NULL));
    must_succeed(pthread_cond_init(&condition, NULL));
    if (pthread_key_create(&key, destroy_value) != 0) {
        ++t->pthread_errors;
        ++t->failures;
        must_succeed(pthread_cond_destroy(&condition));
        must_succeed(pthread_mutex_destroy(&mutex));
        return;
    }
    ++t->tls_checks;
    if (pthread_getspecific(key) != NULL) ++t->failures;
    if (pthread_setspecific(key, &parent) != 0) {
        ++t->failures;
        ++t->pthread_errors;
    }
    ++t->tls_checks;
    if (pthread_getspecific(key) != &parent) ++t->failures;
    for (int i = 0; i < WORKERS; ++i) {
        workers[i].id = number * WORKERS + i;
        if (pthread_create(&threads[i], NULL, work, &workers[i]) != 0) {
            ++t->pthread_errors;
            ++t->failures;
            break;
        }
        ++created;
        ++t->started;
    }
    must_succeed(pthread_mutex_lock(&mutex));
    if (created != WORKERS) stopped = 1;
    for (int round = 0; round < ROUNDS && !stopped; ++round) {
        while (arrived < (round + 1) * WORKERS && !stopped) {
            int rc = pthread_cond_timedwait(&condition, &mutex, &deadline);
            if (rc != 0) {
                ++t->pthread_errors;
                ++t->failures;
                stopped = 1;
            }
        }
        if (stopped) break;
        ++t->tls_checks;
        if (pthread_getspecific(key) != &parent) ++t->failures;
        volatile int *slot = &errno;
        *slot = 2200 + number;
        ++t->errno_checks;
        if (*slot != 2200 + number || slot != parent.errno_slot) ++t->failures;
        /* All workers are alive, and published these addresses before their
         * arrivals under mutex. Compare addresses; never read foreign errno. */
        for (int i = 0; i < WORKERS; ++i) {
            ++t->errno_address_checks;
            if (workers[i].errno_slot == NULL ||
                workers[i].errno_slot == parent.errno_slot) ++t->failures;
            for (int j = 0; j < i; ++j) {
                ++t->errno_address_checks;
                if (workers[i].errno_slot == workers[j].errno_slot) ++t->failures;
            }
        }
        ++epoch;
        must_succeed(pthread_cond_broadcast(&condition));
    }
    must_succeed(pthread_cond_broadcast(&condition));
    must_succeed(pthread_mutex_unlock(&mutex));
    for (int i = 0; i < created; ++i) {
        void *returned = NULL;
        /* A failed join does not establish termination or permit state reuse. */
        must_succeed(pthread_join(threads[i], &returned));
        ++t->joined;
        ++t->join_checks;
        if (returned != &workers[i]) ++t->failures;
        if (workers[i].destructors != 1) ++t->failures;
        t->failures += workers[i].failures;
        t->pthread_errors += workers[i].pthread_errors;
        t->tls_checks += workers[i].tls_checks;
        t->errno_checks += workers[i].errno_checks;
        t->rounds += workers[i].rounds;
        t->destructors += workers[i].destructors;
    }
    if (parent.destructors != 0) ++t->failures;
    if (pthread_setspecific(key, NULL) != 0) {
        ++t->failures;
        ++t->pthread_errors;
    }
    ++t->tls_checks;
    if (pthread_getspecific(key) != NULL) ++t->failures;
    must_succeed(pthread_key_delete(key));
    must_succeed(pthread_cond_destroy(&condition));
    must_succeed(pthread_mutex_destroy(&mutex));
}

int main(void) {
    struct totals t = {0};
    for (int n = 0; n < BATCHES; ++n) batch(&t, n);
    if (once_calls != 1 || t.started != BATCHES * WORKERS ||
        t.joined != BATCHES * WORKERS || t.rounds != BATCHES * WORKERS * ROUNDS ||
        t.destructors != BATCHES * WORKERS || t.join_checks != BATCHES * WORKERS ||
        t.tls_checks != BATCHES * (WORKERS + 1) * (ROUNDS + 3) ||
        t.errno_checks != BATCHES * (WORKERS + 1) * ROUNDS ||
        t.errno_address_checks != BATCHES * ROUNDS * WORKERS * (WORKERS + 1) / 2) ++t.failures;
    int rc = printf("{\"schema_version\":1,\"scope\":\"m1_pthread_state_execution\","
        "\"platform\":\"%s\",\"architecture\":\"%s\",\"workers_requested\":%d,"
        "\"workers_started\":%d,\"workers_joined\":%d,\"rounds\":%d,"
        "\"tls_checks\":%d,\"errno_checks\":%d,\"errno_address_checks\":%d,"
        "\"once_calls\":%d,"
        "\"destructors\":%d,\"join_checks\":%d,\"pthread_errors\":%d,"
        "\"failures\":%d,\"status\":\"%s\"}\n",
        PLATFORM, ARCHITECTURE, BATCHES * WORKERS, t.started, t.joined, t.rounds,
        t.tls_checks, t.errno_checks, t.errno_address_checks, once_calls, t.destructors, t.join_checks,
        t.pthread_errors, t.failures, t.failures ? "fail" : "pass");
    if (rc < 0 || fflush(stdout) != 0) return 2;
    return t.failures ? 1 : 0;
}
