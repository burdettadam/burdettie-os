// pthread_basic: Test basic pthread functionality
// Tests thread creation, joining, mutexes, and condition variables.
#include <stdio.h>
#include <pthread.h>
#include <stdlib.h>

// Shared state protected by mutex
static pthread_mutex_t counter_mutex = PTHREAD_MUTEX_INITIALIZER;
static int shared_counter = 0;

// Condition variable test state
static pthread_mutex_t cond_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cond_var = PTHREAD_COND_INITIALIZER;
static int ready_flag = 0;

void *increment_thread(void *arg) {
    int count = *(int *)arg;
    for (int i = 0; i < count; i++) {
        pthread_mutex_lock(&counter_mutex);
        shared_counter++;
        pthread_mutex_unlock(&counter_mutex);
    }
    return (void *)(long)count;
}

void *waiter_thread(void *arg) {
    (void)arg;
    pthread_mutex_lock(&cond_mutex);
    while (!ready_flag) {
        pthread_cond_wait(&cond_var, &cond_mutex);
    }
    pthread_mutex_unlock(&cond_mutex);
    return (void *)42L;
}

int main(void) {
    // Test 1: Create threads and join
    printf("Test 1: pthread_create and pthread_join\n");
    int counts[4] = {100, 100, 100, 100};
    pthread_t threads[4];
    for (int i = 0; i < 4; i++) {
        if (pthread_create(&threads[i], NULL, increment_thread, &counts[i]) != 0) {
            printf("FAIL: pthread_create\n");
            return 1;
        }
    }
    for (int i = 0; i < 4; i++) {
        void *retval;
        pthread_join(threads[i], &retval);
        if ((long)retval != 100) {
            printf("FAIL: thread return value = %ld, expected 100\n", (long)retval);
            return 1;
        }
    }
    if (shared_counter != 400) {
        printf("FAIL: shared_counter = %d, expected 400\n", shared_counter);
        return 1;
    }
    printf("PASS: 4 threads incremented counter to %d\n", shared_counter);

    // Test 2: Mutex trylock
    printf("Test 2: pthread_mutex_trylock\n");
    pthread_mutex_t trylock_mutex = PTHREAD_MUTEX_INITIALIZER;
    if (pthread_mutex_trylock(&trylock_mutex) != 0) {
        printf("FAIL: trylock on unlocked mutex\n");
        return 1;
    }
    pthread_mutex_unlock(&trylock_mutex);
    printf("PASS: trylock succeeded on unlocked mutex\n");

    // Test 3: Condition variable
    printf("Test 3: pthread_cond_wait and pthread_cond_signal\n");
    pthread_t cond_thread;
    pthread_create(&cond_thread, NULL, waiter_thread, NULL);

    // Signal the waiter
    pthread_mutex_lock(&cond_mutex);
    ready_flag = 1;
    pthread_cond_signal(&cond_var);
    pthread_mutex_unlock(&cond_mutex);

    void *cond_retval;
    pthread_join(cond_thread, &cond_retval);
    if ((long)cond_retval != 42) {
        printf("FAIL: cond thread return = %ld, expected 42\n", (long)cond_retval);
        return 1;
    }
    printf("PASS: condition variable signaling works\n");

    // Test 4: pthread_self
    printf("Test 4: pthread_self\n");
    pthread_t self_id = pthread_self();
    if (self_id == 0) {
        printf("FAIL: pthread_self returned 0\n");
        return 1;
    }
    printf("PASS: pthread_self returned non-zero\n");

    printf("All pthread tests passed!\n");
    return 0;
}
