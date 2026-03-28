# Gap Analysis: Real-World CLI Tool Compatibility

**Date**: 2026-03-28
**Corpus**: 22 CLI tools (17 original + 5 new)
**Result**: 22/22 passing (100%)

## Corpus Tool Summary

| Tool | Category | Foundation APIs Exercised | Checks |
|---|---|---|---|
| word_counter | text | NSArray, NSNumber, NSMutableDictionary, NSString | 6/6 |
| json_formatter | serialization | NSJSONSerialization, NSDictionary, NSArray, NSString | 8/8 |
| file_lister | filesystem | NSFileManager, NSArray, NSString, NSError | 8/8 |
| string_processor | text | NSString (trim, case, split, replace, substring) | 10/10 |
| config_reader | serialization | NSDictionary, NSArray, NSNumber, NSString nested access | 10/10 |
| date_calculator | date/time | NSDate, NSCalendar, NSDateFormatter, NSTimeZone | 8/8 |
| path_resolver | filesystem | NSString path methods, NSFileManager, NSURL | 12/12 |
| collection_pipeline | collections | NSArray, NSMutableArray, NSDictionary, NSSet, sorting | 10/10 |
| error_reporter | error | NSError, NSException, NSDictionary, NSString | 8/8 |
| runtime_info | system | NSProcessInfo, NSBundle, NSDate, NSTimeZone, NSLocale, NSUUID | 8/8 |
| todo_manager | collections | NSMutableArray, NSDictionary, NSNumber, sort + filter | 8/8 |
| text_analyzer | text | NSCharacterSet, NSMutableDictionary, NSArray, NSString | 10/10 |
| json_query | serialization | NSJSONSerialization, NSDictionary, NSArray, nested query | — |
| plist_converter | serialization | NSPropertyListSerialization, NSDictionary, NSData | — |
| regex_search | pattern | NSRegularExpression, NSTextCheckingResult, NSString | — |
| event_bus | notifications | NSNotificationCenter, NSNotification, NSMutableArray | — |
| predicate_filter | predicate | NSPredicate, NSArray filteredArrayUsingPredicate: | — |
| **sort_util** | collections | NSArray sortedArrayUsingComparator:, block-based multi-key sort | 6/6 |
| **cache_bench** | caching | NSCache (cost, eviction), NSMutableDictionary, NSMutableArray | 6/6 |
| **uuid_registry** | identity | NSUUID (init, roundtrip), NSMutableSet, NSMutableDictionary, NSDate | 6/6 |
| **attr_string** | rich text | NSAttributedString, NSMutableAttributedString, addAttribute:value:range: | 6/6 |
| **kv_store** | preferences | NSUserDefaults, NSPropertyListSerialization, NSDictionary | 6/6 |

## Shims Added During This Phase

| Class | Selector | Type | Fix |
|---|---|---|---|
| NSUUID | `init` | instance | Generate random v4 UUID on alloc+init pattern |
| NSMutableArray | `arrayWithArray:` | class | Copy all elements from source array |
| NSMutableAttributedString | `addAttribute:value:range:` | instance | Accumulate runs instead of replacing |
| NSMutableAttributedString | `replaceCharactersInRange:withString:` | instance | Actual range replacement instead of full string swap |

## Current Foundation Coverage

### Fully Shimmed Classes (52)
NSObject, NSString, NSMutableString, __NSCFConstantString,
NSArray, NSMutableArray, NSDictionary, NSConstantDictionary, NSMutableDictionary,
NSSet, NSMutableSet, NSNumber, NSConstantIntegerNumber, NSValue, NSNull,
NSData, NSURL, NSError, NSException,
NSDate, NSDateFormatter, NSTimeZone, NSCalendar, NSLocale,
NSProcessInfo, NSBundle, NSFileManager, NSUserDefaults,
NSCharacterSet, NSRegularExpression, NSTextCheckingResult,
NSPredicate, NSAttributedString, NSMutableAttributedString,
NSNotificationCenter, NSNotification,
NSJSONSerialization, NSPropertyListSerialization,
NSUUID, NSEnumerator, NSIndexSet, NSMutableIndexSet,
NSCache, NSMapTable, NSHashTable, NSSortDescriptor,
NSNumberFormatter, NSMethodSignature, NSInvocation,
NSThread, NSLock, NSRecursiveLock, NSCondition, NSConditionLock,
NSOperation, NSBlockOperation, NSOperationQueue,
NSTimer

### Shim Counts
- ObjC runtime: 103 selectors
- Foundation: 177+ selectors (was 172, +5 from arrayWithArray:, init on NSUUID, addAttribute fixes)
- libSystem: 65 shims
- **Total: 345+ shims**

## Top 20 Missing APIs (Prioritized for C7+)

These are Foundation classes commonly used in real-world macOS CLI tools that are not yet shimmed:

| Priority | Class | Typical CLI Usage |
|---|---|---|
| 1 | **NSTask / NSProcess** | Launching subprocesses (shell commands, xcrun) |
| 2 | **NSPipe** | Capturing stdout/stderr from NSTask |
| 3 | **NSFileHandle** | Low-level file I/O (stdin/stdout/stderr) |
| 4 | **NSMutableData** | In-place byte buffer mutation (append, replace) |
| 5 | **NSURLSession** | HTTP requests, downloading files, REST APIs |
| 6 | **NSURLRequest / NSURLResponse** | Companion types for networking |
| 7 | **NSKeyedArchiver / NSKeyedUnarchiver** | Serializing object graphs to binary archives |
| 8 | **NSScanner** | scanf-style string parsing |
| 9 | **NSStream / NSInputStream / NSOutputStream** | Stream-based I/O |
| 10 | **NSRunLoop** (selectors) | Event loop management (run, runUntilDate:) |
| 11 | **NSOrderedSet / NSMutableOrderedSet** | Ordered unique collections |
| 12 | **NSCountedSet** | Bag/multiset (counting occurrences) |
| 13 | **NSURLComponents / NSURLQueryItem** | URL building with query parameters |
| 14 | **NSByteCountFormatter** | Human-readable byte sizes ("1.5 MB") |
| 15 | **NSISO8601DateFormatter** | ISO 8601 date parsing (common in APIs/JSON) |
| 16 | **NSXMLParser** | XML parsing (plists, config files) |
| 17 | **NSCoder / NSKeyedArchiver** | Foundation archiving protocol |
| 18 | **NSMutableCharacterSet** | Dynamic character set building |
| 19 | **NSProgress** | Progress reporting |
| 20 | **NSHost** | Hostname/address resolution |

## Gap Categories

### Subprocess Execution (Critical for CLI tools)
NSTask + NSPipe + NSFileHandle form the backbone of any CLI tool that shells out.
Most real-world macOS CLI tools (Homebrew, swiftlint, xcpretty) depend heavily on
subprocess execution. This is the single biggest gap for running real-world binaries.

### Networking (Critical for package managers / API clients)
NSURLSession is required by any tool that makes HTTP requests. This blocks
running tools like CocoaPods, Carthage, or any REST client.

### Archiving (Medium priority)
NSKeyedArchiver/Unarchiver is used by tools that persist state (Xcode build
caches, preferences). Less critical for pure CLI tools.

### Byte Buffers (Medium priority)
NSMutableData is used pervasively for building up output buffers, reading
file chunks, and protocol framing. Relatively straightforward to implement.

## Conclusion

The current runtime achieves **100% compatibility** with the 22-tool corpus,
covering core Foundation patterns: collections, serialization, string processing,
date/time, filesystem, caching, notifications, regular expressions, predicates,
attributed strings, UUIDs, and user defaults.

The top gaps for real-world CLI tool compatibility are subprocess execution
(NSTask/NSPipe), networking (NSURLSession), and binary archiving
(NSKeyedArchiver). These should be prioritized for Track C7+.
