# Darwin Loader Lab — Compatibility & Coverage Metrics

> All counts are as of 2026-04-03. See [FEATURES.md](FEATURES.md) for the full
> feature index and [roadmap.md](roadmap.md) for what comes next.

---

## Summary

| Metric | Value | Notes |
|---|---|---|
| Workspace members | 44 | 41 crates + 3 GUI apps |
| Fixture binaries | 85 directories | ARM64 Mach-O binaries compiled from source |
| Compat-scored fixtures | 63/63 passing | 0 failing |
| Corpus CLI tools | 39 dirs, 30/30 passing | 30 compat-scored CLI patterns; 100% pass rate |
| Total tests | 5,599 passing | 0 failing, 11 ignored (arm64 arch-specific) |
| Registered ObjC classes | 193 | 148 NS* Foundation + 40 AppKit + misc framework stubs |
| Registered selectors | 1,380 | 867 Foundation + 513 AppKit |
| cf-lite shim functions | 2,001 | Across 124 source modules, 2,538 tests |
| ObjC runtime shims | 131 | msgSend family + runtime introspection — 208 tests |
| libSystem shim functions | 443 | 37 darwin_stubs modules — 309 tests |
| Rust LOC | ~186,000 + 620 asm | 620 lines of hand-written ARM64 + x86_64 assembly |
| Interpreted fixtures passing | 22/22 | On macOS ARM64 host; E4a+E4b+E4c complete |
| PAL trait implementations | 2 complete | macOS (`DarwinHostPlatform`), Linux (`LinuxHostPlatform`) |
| GUI controls | 13 | AppKit controls with full property read/write — 244 tests |
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
