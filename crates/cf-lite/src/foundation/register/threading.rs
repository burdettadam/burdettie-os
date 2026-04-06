use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nslock, nsobj, nsoperation, nsthread, nstimer};

/// Register `NSTimer` with timer lifecycle methods.
pub fn register_nstimer_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstimer_class_addr();
    let metaclass_addr = nsobj::nstimer_metaclass_addr();

    let info = ClassInfo {
        name: "NSTimer".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("fire", nstimer::shim_nstimer_fire as *const () as u64, "v@:"),
            me("invalidate", nstimer::shim_nstimer_invalidate as *const () as u64, "v@:"),
            me("isValid", nstimer::shim_nstimer_is_valid as *const () as u64, "B@:"),
            me(
                "timeInterval",
                nstimer::shim_nstimer_time_interval as *const () as u64,
                "d@:",
            ),
            me(
                "description",
                nstimer::shim_nstimer_description as *const () as u64,
                "@@:",
            ),
            me("fireDate", nstimer::shim_nstimer_fire_date as *const () as u64, "@@:"),
            me("setFireDate:", nstimer::shim_nstimer_set_fire_date as *const () as u64, "v@:@"),
            me("tolerance", nstimer::shim_nstimer_tolerance as *const () as u64, "d@:"),
            me("setTolerance:", nstimer::shim_nstimer_set_tolerance as *const () as u64, "v@:d"),
        ],
        class_methods: vec![
            me(
                "scheduledTimerWithTimeInterval:repeats:block:",
                nstimer::shim_nstimer_scheduled_with_block as *const () as u64,
                "@@:dB@?",
            ),
            me(
                "timerWithTimeInterval:repeats:block:",
                nstimer::shim_nstimer_timer_with_block as *const () as u64,
                "@@:dB@?",
            ),
            me(
                "scheduledTimerWithTimeInterval:target:selector:userInfo:repeats:",
                nstimer::shim_nstimer_scheduled_with_target as *const () as u64,
                "@@:d@:@B",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTimer");
}

/// Register `NSThread` with thread management methods.
pub fn register_nsthread_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsthread_class_addr();
    let metaclass_addr = nsobj::nsthread_metaclass_addr();

    let info = ClassInfo {
        name: "NSThread".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "isMainThread",
                nsthread::shim_nsthread_is_main_thread as *const () as u64,
                "B@:",
            ),
            me("name", nsthread::shim_nsthread_name as *const () as u64, "@@:"),
            me("setName:", nsthread::shim_nsthread_set_name as *const () as u64, "v@:@"),
            me(
                "description",
                nsthread::shim_nsthread_description as *const () as u64,
                "@@:",
            ),
            me(
                "initWithTarget:selector:object:",
                nsthread::shim_nsthread_init_with_target as *const () as u64,
                "@@:@:@",
            ),
            me("start", nsthread::shim_nsthread_start as *const () as u64, "v@:"),
            me(
                "isExecuting",
                nsthread::shim_nsthread_is_executing as *const () as u64,
                "B@:",
            ),
            me(
                "isFinished",
                nsthread::shim_nsthread_is_finished as *const () as u64,
                "B@:",
            ),
            me(
                "threadDictionary",
                nsthread::shim_nsthread_thread_dictionary as *const () as u64,
                "@@:",
            ),
            me(
                "threadPriority",
                nsthread::shim_nsthread_thread_priority as *const () as u64,
                "d@:",
            ),
            me(
                "setThreadPriority:",
                nsthread::shim_nsthread_set_thread_priority as *const () as u64,
                "v@:d",
            ),
            me(
                "qualityOfService",
                nsthread::shim_nsthread_quality_of_service as *const () as u64,
                "q@:",
            ),
            me(
                "setQualityOfService:",
                nsthread::shim_nsthread_set_quality_of_service as *const () as u64,
                "v@:q",
            ),
            me(
                "stackSize",
                nsthread::shim_nsthread_stack_size as *const () as u64,
                "Q@:",
            ),
            me(
                "setStackSize:",
                nsthread::shim_nsthread_set_stack_size as *const () as u64,
                "v@:Q",
            ),
        ],
        class_methods: vec![
            me(
                "currentThread",
                nsthread::shim_nsthread_current_thread as *const () as u64,
                "@@:",
            ),
            me(
                "mainThread",
                nsthread::shim_nsthread_main_thread as *const () as u64,
                "@@:",
            ),
            me(
                "isMainThread",
                nsthread::shim_nsthread_is_main_thread_class as *const () as u64,
                "B@:",
            ),
            me(
                "isMultiThreaded",
                nsthread::shim_nsthread_is_multi_threaded as *const () as u64,
                "B@:",
            ),
            me(
                "sleepForTimeInterval:",
                nsthread::shim_nsthread_sleep_for_time_interval as *const () as u64,
                "v@:d",
            ),
            me(
                "detachNewThreadSelector:toTarget:withObject:",
                nsthread::shim_nsthread_detach as *const () as u64,
                "v@::@@",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 48,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSThread");
}

/// Register `NSLock` with locking methods.
pub fn register_nslock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslock_class_addr();
    let metaclass_addr = nsobj::nslock_metaclass_addr();

    let info = ClassInfo {
        name: "NSLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nslock_init as *const () as u64, "@@:"),
            me("lock", nslock::shim_nslock_lock as *const () as u64, "v@:"),
            me("unlock", nslock::shim_nslock_unlock as *const () as u64, "v@:"),
            me("tryLock", nslock::shim_nslock_try_lock as *const () as u64, "B@:"),
            me("name", nslock::shim_nslock_name as *const () as u64, "@@:"),
            me("setName:", nslock::shim_nslock_set_name as *const () as u64, "v@:@"),
            me("dealloc", nslock::shim_nslock_dealloc as *const () as u64, "v@:"),
            me("description", nslock::shim_nslock_description as *const () as u64, "@@:"),
        ],
        class_methods: vec![me("alloc", nslock::shim_nslock_alloc as *const () as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLock");
}

/// Register `NSRecursiveLock`.
pub fn register_nsrecursivelock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsrecursivelock_class_addr();
    let metaclass_addr = nsobj::nsrecursivelock_metaclass_addr();

    let info = ClassInfo {
        name: "NSRecursiveLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nsrecursivelock_init as *const () as u64, "@@:"),
            me("lock", nslock::shim_nsrecursivelock_lock as *const () as u64, "v@:"),
            me("unlock", nslock::shim_nsrecursivelock_unlock as *const () as u64, "v@:"),
            me(
                "tryLock",
                nslock::shim_nsrecursivelock_try_lock as *const () as u64,
                "B@:",
            ),
            me(
                "dealloc",
                nslock::shim_nsrecursivelock_dealloc as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nslock::shim_nsrecursivelock_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 80,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSRecursiveLock");
}

/// Register `NSCondition`.
pub fn register_nscondition_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscondition_class_addr();
    let metaclass_addr = nsobj::nscondition_metaclass_addr();

    let info = ClassInfo {
        name: "NSCondition".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nscondition_init as *const () as u64, "@@:"),
            me("lock", nslock::shim_nscondition_lock as *const () as u64, "v@:"),
            me("unlock", nslock::shim_nscondition_unlock as *const () as u64, "v@:"),
            me("wait", nslock::shim_nscondition_wait as *const () as u64, "v@:"),
            me("signal", nslock::shim_nscondition_signal as *const () as u64, "v@:"),
            me(
                "broadcast",
                nslock::shim_nscondition_broadcast as *const () as u64,
                "v@:",
            ),
            me(
                "waitUntilDate:",
                nslock::shim_nscondition_wait_until_date as *const () as u64,
                "c@:@",
            ),
            me("name", nslock::shim_nscondition_name as *const () as u64, "@@:"),
            me("setName:", nslock::shim_nscondition_set_name as *const () as u64, "v@:@"),
            me("dealloc", nslock::shim_nscondition_dealloc as *const () as u64, "v@:"),
        ],
        class_methods: vec![me("alloc", nslock::shim_nscondition_alloc as *const () as u64, "@@:")],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 128,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCondition");
}

/// Register `NSConditionLock`.
pub fn register_nsconditionlock_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsconditionlock_class_addr();
    let metaclass_addr = nsobj::nsconditionlock_metaclass_addr();

    let info = ClassInfo {
        name: "NSConditionLock".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nslock::shim_nsconditionlock_init as *const () as u64, "@@:"),
            me(
                "initWithCondition:",
                nslock::shim_nsconditionlock_init_with_condition as *const () as u64,
                "@@:q",
            ),
            me(
                "condition",
                nslock::shim_nsconditionlock_condition as *const () as u64,
                "q@:",
            ),
            me("lock", nslock::shim_nsconditionlock_lock as *const () as u64, "v@:"),
            me("unlock", nslock::shim_nsconditionlock_unlock as *const () as u64, "v@:"),
            me(
                "lockWhenCondition:",
                nslock::shim_nsconditionlock_lock_when_condition as *const () as u64,
                "v@:q",
            ),
            me(
                "unlockWithCondition:",
                nslock::shim_nsconditionlock_unlock_with_condition as *const () as u64,
                "v@:q",
            ),
            me(
                "tryLock",
                nslock::shim_nsconditionlock_try_lock as *const () as u64,
                "B@:",
            ),
            me("name", nslock::shim_nsconditionlock_name as *const () as u64, "@@:"),
            me(
                "setName:",
                nslock::shim_nsconditionlock_set_name as *const () as u64,
                "v@:@",
            ),
            me(
                "dealloc",
                nslock::shim_nsconditionlock_dealloc as *const () as u64,
                "v@:",
            ),
        ],
        class_methods: vec![me(
            "alloc",
            nslock::shim_nsconditionlock_alloc as *const () as u64,
            "@@:",
        )],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 136,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSConditionLock");
}

/// Register `NSOperation`.
pub fn register_nsoperation_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsoperation_class_addr();
    let metaclass_addr = nsobj::nsoperation_metaclass_addr();

    let info = ClassInfo {
        name: "NSOperation".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsoperation::shim_nsoperation_init as *const () as u64, "@@:"),
            me("start", nsoperation::shim_nsoperation_start as *const () as u64, "v@:"),
            me("main", nsoperation::shim_nsoperation_main as *const () as u64, "v@:"),
            me("cancel", nsoperation::shim_nsoperation_cancel as *const () as u64, "v@:"),
            me("isCancelled", nsoperation::shim_nsoperation_is_cancelled as *const () as u64, "B@:"),
            me("isFinished", nsoperation::shim_nsoperation_is_finished as *const () as u64, "B@:"),
        ],
        class_methods: vec![
            me("alloc", nsoperation::shim_nsoperation_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsoperation::OP_INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOperation");
}

/// Register `NSBlockOperation`.
pub fn register_nsblockoperation_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsblockoperation_class_addr();
    let metaclass_addr = nsobj::nsblockoperation_metaclass_addr();

    let info = ClassInfo {
        name: "NSBlockOperation".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("start", nsoperation::shim_block_op_start as *const () as u64, "v@:"),
            me("cancel", nsoperation::shim_nsoperation_cancel as *const () as u64, "v@:"),
            me("isCancelled", nsoperation::shim_nsoperation_is_cancelled as *const () as u64, "B@:"),
            me("isFinished", nsoperation::shim_nsoperation_is_finished as *const () as u64, "B@:"),
        ],
        class_methods: vec![
            me("blockOperationWithBlock:", nsoperation::shim_block_op_with_block as *const () as u64, "@@:@?"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsoperation::BLOCK_OP_INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsoperation_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSBlockOperation");
}

/// Register `NSOperationQueue`.
pub fn register_nsoperationqueue_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsoperationqueue_class_addr();
    let metaclass_addr = nsobj::nsoperationqueue_metaclass_addr();

    let info = ClassInfo {
        name: "NSOperationQueue".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsoperation::shim_queue_init as *const () as u64, "@@:"),
            me("addOperation:", nsoperation::shim_queue_add_operation as *const () as u64, "v@:@"),
            me("addOperationWithBlock:", nsoperation::shim_queue_add_block as *const () as u64, "v@:@?"),
            me("setSuspended:", nsoperation::shim_queue_set_suspended as *const () as u64, "v@:B"),
            me("isSuspended", nsoperation::shim_queue_is_suspended as *const () as u64, "B@:"),
            me("setName:", nsoperation::shim_queue_set_name as *const () as u64, "v@:@"),
            me("name", nsoperation::shim_queue_name as *const () as u64, "@@:"),
            me("setMaxConcurrentOperationCount:", nsoperation::shim_queue_set_max_concurrent as *const () as u64, "v@:q"),
            me("maxConcurrentOperationCount", nsoperation::shim_queue_max_concurrent as *const () as u64, "q@:"),
            me("description", nsoperation::shim_queue_description as *const () as u64, "@@:"),
            me("qualityOfService", nsoperation::shim_queue_quality_of_service as *const () as u64, "q@:"),
            me("setQualityOfService:", nsoperation::shim_queue_set_quality_of_service as *const () as u64, "v@:q"),
        ],
        class_methods: vec![
            me("alloc", nsoperation::shim_queue_alloc as *const () as u64, "@@:"),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: nsoperation::QUEUE_INSTANCE_SIZE,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSOperationQueue");
}
