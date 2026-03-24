# Fixture Baseline — v0.1.0

> The v1 compatibility baseline for darwin-loader-lab.
> Every fixture listed here must pass all pipeline stages and exit
> with the correct code at all times.  The `compat_score_regression_gate`
> test enforces this automatically.

---

## Totals

| Metric | Value |
|--------|-------|
| Total fixture directories | 64 |
| Excluded (intentionally malformed) | 1 (`malformed`) |
| Testable fixtures | 63 |
| Required pass rate | 63/63 (100%) |

---

## Fixture Inventory

### Core Loader (7 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 1 | `return42` | `app` | 42 | Entry point returns non-zero |
| 2 | `simple_cli` | `app` | 0 | Minimal puts/printf |
| 3 | `real_cli` | `app` | 0 | argv/argc processing |
| 4 | `constructor_test` | `app` | 0 | C constructors (__mod_init_func) |
| 5 | `dylib_chain` | `app` | 0 | Multi-level dylib dependency chain |
| 6 | `fat_binary` | `app` | 0 | Universal (fat) binary with arm64 slice |
| 7 | `weak_dylib` | `app` | 0 | Weak-linked missing dylib |

### Framework & Dynamic Loading (2 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 8 | `framework_style` | `app` | 0 | .framework bundle loading |
| 9 | `dispatch_test` | `dispatch_test` | 0 | GCD dispatch_once |

### Thread-Local Variables (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 10 | `tlv_basic` | `app` | 0 | __thread / _Thread_local storage |

### Process Info (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 11 | `process_info` | `app` | 0 | getprogname, _NSGetExecutablePath |

### ObjC Runtime (7 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 12 | `objc_minimal` | `app` | 0 | Minimal ObjC alloc+init+release |
| 13 | `objc_classes` | `app` | 0 | Class registration and lookup |
| 14 | `objc_inherit` | `app` | 0 | Class inheritance chains |
| 15 | `objc_super` | `app` | 0 | objc_msgSendSuper |
| 16 | `objc_category` | `app` | 0 | Category method merging |
| 17 | `objc_foundation` | `app` | 0 | ObjC + Foundation combined |
| 18 | `objc_dylib_chain` | `app` | 0 | ObjC classes across dylib boundaries |

### ObjC Introspection (2 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 19 | `runtime_introspection` | `app` | 0 | class_copyMethodList, method_getName, etc. |
| 20 | `property_introspection` | `app` | 0 | class_copyPropertyList, property_getAttributes |

### Blocks (2 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 21 | `block_basics` | `app` | 0 | Global and stack blocks |
| 22 | `blocks_foundation` | `app` | 0 | Blocks with Foundation (enumerateObjectsUsingBlock:) |

### Autorelease (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 23 | `autorelease` | `app` | 0 | @autoreleasepool, objc_autoreleasePoolPush/Pop |

### Exception Handling (3 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 24 | `exception_basics` | `app` | 0 | @try/@catch/@finally |
| 25 | `try_catch_test` | `app` | 0 | Multiple exception scenarios |
| 26 | `minimal_throw` | `app` | 0 | NSException throw + catch |

### NSString (6 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 27 | `nsstring_ops` | `app` | 0 | Basic NSString operations |
| 28 | `nsstring_manip` | `app` | 0 | String manipulation (substring, replace, etc.) |
| 29 | `nsstring_format` | `app` | 0 | +[NSString stringWithFormat:] |
| 30 | `nsstring_dynamic` | `app` | 0 | Dynamic string allocation |
| 31 | `nsstring_mutable` | `app` | 0 | NSMutableString operations |
| 32 | `nsstring_path` | `app` | 0 | Path manipulation (lastPathComponent, etc.) |

### NSArray & Collections (4 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 33 | `nsarray_basics` | `app` | 0 | NSArray creation and access |
| 34 | `nsmutablearray_basics` | `app` | 0 | NSMutableArray add/remove/sort |
| 35 | `collection_enum` | `app` | 0 | NSFastEnumeration (for-in loops) |
| 36 | `nsdictionary_basics` | `app` | 0 | NSDictionary creation and access |

### NSDictionary & Sets (3 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 37 | `nsmutabledictionary_basics` | `app` | 0 | NSMutableDictionary mutations |
| 38 | `nsset_basics` | `app` | 0 | NSSet / NSMutableSet operations |
| 39 | `nsindexset_basics` | `app` | 0 | NSIndexSet range operations |

### NSNumber & Values (4 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 40 | `nsnumber_basics` | `app` | 0 | NSNumber boxing/unboxing |
| 41 | `nsnumberformatter_basics` | `app` | 0 | NSNumberFormatter locale formatting |
| 42 | `nsvalue_basics` | `app` | 0 | NSValue wrapping (NSRange, pointers) |
| 43 | `nsnull_basics` | `app` | 0 | NSNull singleton |

### NSData (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 44 | `nsdata_basics` | `app` | 0 | NSData creation, length, bytes |

### NSLog (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 45 | `nslog_format_specifiers` | `app` | 0 | NSLog with %@, %d, %f, etc. |

### NSObject (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 46 | `nsobject_methods` | `app` | 0 | respondsToSelector:, isKindOfClass:, etc. |

### NSError (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 47 | `nserror_basics` | `app` | 0 | NSError creation and access |

### NSDate & Calendar (4 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 48 | `nsdate_basics` | `app` | 0 | NSDate creation, timeIntervalSince1970 |
| 49 | `nsdateformatter_basics` | `app` | 0 | NSDateFormatter string→date |
| 50 | `nstimezone_basics` | `app` | 0 | NSTimeZone name, abbreviation, offset |
| 51 | `nscalendar_basics` | `app` | 0 | NSCalendar components from date |

### Foundation Services (8 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 52 | `nsbundle_basics` | `app` | 0 | NSBundle mainBundle |
| 53 | `nsfilemanager_basics` | `app` | 0 | NSFileManager file operations |
| 54 | `nsprocessinfo_basics` | `app` | 0 | NSProcessInfo processName, arguments |
| 55 | `nsuserdefaults_basics` | `app` | 0 | NSUserDefaults standardUserDefaults |
| 56 | `nsurl_basics` | `app` | 0 | NSURL creation and components |
| 57 | `nsuuid_basics` | `app` | 0 | NSUUID generation and UUIDString |
| 58 | `nslocale_basics` | `app` | 0 | NSLocale currentLocale, identifiers |
| 59 | `nscharacterset_basics` | `app` | 0 | NSCharacterSet predefined sets |

### Notification & Sorting (3 fixtures)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 60 | `nsnotification_basics` | `app` | 0 | NSNotificationCenter post/observe |
| 61 | `nssortdescriptor_basics` | `app` | 0 | NSSortDescriptor compare: |
| 62 | `nstimer_basics` | `app` | 0 | NSTimer (immediate-fire mode) |

### Threading (1 fixture)

| # | Fixture | Binary | Expected exit | Category |
|---|---------|--------|--------------|----------|
| 63 | `pthread_basic` | `app` | 0 | pthread_create/join, mutex, condvar |

---

## Excluded Fixtures

| Fixture | Reason |
|---------|--------|
| `malformed` | Contains intentionally invalid Mach-O files (bad magic, truncated headers) for negative testing — not a runtime fixture |

---

## Scoring Dimensions

Each fixture is evaluated across 7 dimensions:

1. **Parse** — Mach-O binary successfully parsed (header, load commands, segments)
2. **Load** — All images mapped into memory, dependencies resolved
3. **Fixups** — Rebases + symbol binds applied correctly
4. **Enter** — Entry point (LC_MAIN) called successfully
5. **Complete** — Binary ran to completion (no crash/abort)
6. **Exit correct** — Exit code matches expected value
7. **Expected exit** — The reference exit code (0 for all except `return42` = 42)

## Regression Policy

- The `compat_score_regression_gate` test in `crates/macho-runtime/tests/compat_score.rs` enforces 100% pass rate
- New fixtures may be added (increasing the baseline) but existing fixtures must never regress
- The gate asserts `fixtures.len() >= 63` to prevent fixture removal
