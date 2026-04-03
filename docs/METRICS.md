# Darwin Loader Lab — Compatibility & Coverage Metrics

> All counts are as of 2026-03-28. See [FEATURES.md](FEATURES.md) for the full
> feature index and [ROADMAP.md](ROADMAP.md) for what comes next.

---

## Summary

| Metric | Value | Notes |
|---|---|---|
| Fixture binaries | 85+ directories | ARM64 Mach-O binaries compiled from source |
| Compat-scored fixtures | 63/63 passing | 0 failing |
| Corpus CLI tools | 30/30 passing | 30 real-world CLI patterns; 100% pass rate |
| Total tests | 1,090+ passing | 0 failing, 0 regressions since Phase 30 freeze |
| Foundation classes — substantive | 52 | Full logic + selectors; NSObject through NSTask |
| Foundation classes — total shimmed | 71 | Includes AppKit stubs and constant/synthetic variants |
| Foundation selectors (Selector Count) | 523+ | All `objc_msgSend`-dispatchable Foundation methods |
| Shim registrations (Registration Count) | 340+ | Unique `ShimTable::register()` calls at runtime start |
| ObjC runtime shims | 103 | msgSend family + runtime introspection functions |
| libSystem shims | 65 | pthread, dispatch, mach, process, I/O, allocation |
| Rust LOC | ~70,000 + 620 asm | 620 lines of hand-written ARM64 + x86_64 assembly |
| Interpreted fixtures passing | 22/22 | On macOS ARM64 host; E4a+E4b+E4c complete |
| PAL trait implementations | 2 complete | macOS (`DarwinHostPlatform`), Linux (`LinuxHostPlatform`) |
| GUI controls | 7 | Label, Button, TextField, ScrollView, StackView, TableView, SplitView |
| GUI apps | 3 | hello-darwin, control-gallery, foundation-explorer |

---

## Corpus Tool Breakdown

All 30 corpus tools pass at 100%:

| Tool | Domain | APIs Exercised |
|---|---|---|
| word_counter | text | NSArray, NSNumber, NSMutableDictionary, NSString |
| json_formatter | serialization | NSJSONSerialization, NSDictionary, NSArray, NSString |
| file_lister | filesystem | NSFileManager, NSArray, NSString, NSError |
| string_processor | text | NSString (trim, case, split, replace, substring) |
| config_reader | serialization | NSDictionary, NSArray, NSNumber, nested access |
| date_calculator | date/time | NSDate, NSCalendar, NSDateFormatter, NSTimeZone |
| path_resolver | filesystem | NSString path methods, NSFileManager, NSURL |
| collection_pipeline | collections | NSArray, NSMutableArray, NSDictionary, NSSet, sorting |
| error_reporter | error | NSError, NSException, NSDictionary, NSString |
| runtime_info | system | NSProcessInfo, NSBundle, NSDate, NSTimeZone, NSLocale, NSUUID |
| todo_manager | collections | NSMutableArray, NSDictionary, NSNumber, sort + filter |
| text_analyzer | text | NSCharacterSet, NSMutableDictionary, NSArray, NSString |
| json_query | serialization | NSJSONSerialization, NSDictionary, NSArray, nested query |
| plist_converter | serialization | NSPropertyListSerialization, NSDictionary, NSData |
| regex_search | pattern | NSRegularExpression, NSTextCheckingResult, NSString |
| event_bus | notifications | NSNotificationCenter, NSNotification, NSMutableArray |
| predicate_filter | predicate | NSPredicate, NSArray filteredArrayUsingPredicate: |
| sort_util | collections | NSArray sortedArrayUsingComparator:, multi-key sort |
| cache_bench | caching | NSCache (cost, eviction), NSMutableDictionary |
| uuid_registry | identity | NSUUID, NSMutableSet, NSMutableDictionary, NSDate |
| attr_string | rich text | NSAttributedString, NSMutableAttributedString |
| kv_store | preferences | NSUserDefaults, NSPropertyListSerialization |
| data_buffer | byte buffers | NSMutableData append/replace/setLength |
| scanner_parse | parsing | NSScanner scanInt:/scanDouble:/scanString: |
| ordered_collections | collections | NSOrderedSet, NSMutableOrderedSet, NSCountedSet |
| url_builder | networking | NSURLComponents, NSURLQueryItem |
| byte_format | formatting | NSByteCountFormatter decimal/binary |
| progress_track | progress | NSProgress totalUnitCount, fractionCompleted, cancel |
| pipe_io | IPC | NSPipe, NSFileHandle, NSURLRequest |
| host_info | networking | NSHost, NSISO8601DateFormatter, NSHTTPURLResponse |
