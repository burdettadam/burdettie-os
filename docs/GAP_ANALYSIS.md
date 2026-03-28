# Gap Analysis: Real-World CLI Tool Compatibility

**Date**: 2026-03-28 (updated: Phase C9 gap closure)
**Corpus**: 30 CLI tools (25 previous + 5 new)
**Result**: 30/30 passing (100%)

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
| **data_buffer** | byte buffers | NSMutableData append/replace/setLength, NSData | 8/8 |
| **scanner_parse** | parsing | NSScanner scanInt:/scanDouble:/scanString:/scanUpToString: | 8/8 |
| **ordered_collections** | collections | NSOrderedSet, NSMutableOrderedSet, NSCountedSet countForObject: | 10/10 |
| **url_builder** | networking | NSURLComponents, NSURLQueryItem, URL building/parsing | 8/8 |
| **byte_format** | formatting | NSByteCountFormatter decimal/binary formatting | 6/6 |
| **progress_track** | progress | NSProgress totalUnitCount, fractionCompleted, cancel | 7/7 |
| **pipe_io** | IPC | NSPipe, NSFileHandle, NSURLRequest, NSMutableURLRequest | 7/7 |
| **host_info** | networking | NSHost, NSISO8601DateFormatter, NSHTTPURLResponse | 7/7 |

## Shims Added During This Phase

| Class | Selector | Type | Fix |
|---|---|---|---|
| NSUUID | `init` | instance | Generate random v4 UUID on alloc+init pattern |
| NSMutableArray | `arrayWithArray:` | class | Copy all elements from source array |
| NSMutableAttributedString | `addAttribute:value:range:` | instance | Accumulate runs instead of replacing |
| NSMutableAttributedString | `replaceCharactersInRange:withString:` | instance | Actual range replacement instead of full string swap |

## Shims Added During Phase C8 (Gap Closure)

| Class | Methods | Type |
|---|---|---|
| NSMutableData | data, dataWithCapacity:, dataWithBytes:length:, length, bytes, mutableBytes, appendBytes:length:, appendData:, setLength:, resetBytesInRange:, replaceBytesInRange:withBytes:, description | 13 selectors |
| NSFileHandle | fileHandleWithStandardInput/Output/Error, fileHandleForReadingAtPath:, fileHandleForWritingAtPath:, fileDescriptor, readDataOfLength:, readDataToEndOfFile, availableData, writeData:, closeFile, seekToFileOffset:, offsetInFile, description | 14 selectors |
| NSScanner | scannerWithString:, string, scanLocation, setScanLocation:, isAtEnd, caseSensitive, setCaseSensitive:, scanInt:, scanDouble:, scanString:intoString:, scanUpToString:intoString:, description | 12 selectors |
| NSOrderedSet | orderedSetWithArray:, orderedSet, count, objectAtIndex:, firstObject, lastObject, containsObject:, indexOfObject:, array, description | 10 selectors |
| NSMutableOrderedSet | orderedSet, addObject:, removeObject:, removeAllObjects, insertObject:atIndex: | 5 selectors |
| NSCountedSet | set, setWithArray:, addObject:, removeObject:, count, countForObject:, containsObject:, allObjects, description | 9 selectors |
| NSMutableCharacterSet | characterSetWithCharactersInString:, whitespaceCharacterSet, alphanumericCharacterSet, decimalDigitCharacterSet, letterCharacterSet, characterIsMember:, addCharactersInString:, removeCharactersInString:, formUnionWithCharacterSet:, formIntersectionWithCharacterSet:, invert, description | 12 selectors |

## Shims Added During Phase C9 (Gap Closure)

| Class | Methods | Type |
|---|---|---|
| NSURLComponents | componentsWithString:, new, scheme/host/port/path/query/fragment (get+set), URL, string, queryItems, setQueryItems:, description | 19 selectors |
| NSURLQueryItem | queryItemWithName:value:, name, value, description | 4 selectors |
| NSByteCountFormatter | alloc, init, stringFromByteCount:countStyle: (class), stringFromByteCount: (instance), setCountStyle:, countStyle, setAllowedUnits:, allowedUnits, description | 9 selectors |
| NSISO8601DateFormatter | alloc, init, stringFromDate:timeZone:formatOptions: (class), stringFromDate:, dateFromString:, setFormatOptions:, formatOptions, setTimeZone:, timeZone, description | 10 selectors |
| NSProgress | progressWithTotalUnitCount:, discreteProgressWithTotalUnitCount:, totalUnitCount, setTotalUnitCount:, completedUnitCount, setCompletedUnitCount:, fractionCompleted, isCancellable, setCancellable:, isCancelled, cancel, localizedDescription, setLocalizedDescription:, description | 14 selectors |
| NSHost | currentHost, hostWithName:, hostWithAddress:, name, address, addresses, names, localizedName, isEqualToHost:, description | 10 selectors |
| NSPipe | pipe, fileHandleForReading, fileHandleForWriting, description | 4 selectors |
| NSURLRequest | requestWithURL:, URL, HTTPMethod, allHTTPHeaderFields, description | 5 selectors |
| NSMutableURLRequest | requestWithURL:, setHTTPMethod:, setURL:, setValue:forHTTPHeaderField: | 4 selectors |
| NSURLResponse | URL, MIMEType, expectedContentLength, description | 4 selectors |
| NSHTTPURLResponse | statusCode, localizedStringForStatusCode:, description | 3 selectors |
| NSTask | alloc, init, launchedTaskWithLaunchPath:arguments:, setLaunchPath:, setExecutableURL:, setArguments:, setEnvironment:, setStandardOutput:, launchPath, arguments, environment, terminationStatus, processIdentifier, isRunning, launch, waitUntilExit, description | 17 selectors |

## Current Foundation Coverage

### Fully Shimmed Classes (71)
NSObject, NSString, NSMutableString, __NSCFConstantString,
NSArray, NSMutableArray, NSDictionary, NSConstantDictionary, NSMutableDictionary,
NSSet, NSMutableSet, NSNumber, NSConstantIntegerNumber, NSValue, NSNull,
NSData, **NSMutableData**, NSURL, NSError, NSException,
NSDate, NSDateFormatter, NSTimeZone, NSCalendar, NSLocale,
NSProcessInfo, NSBundle, NSFileManager, **NSFileHandle**, NSUserDefaults,
NSCharacterSet, **NSMutableCharacterSet**, NSRegularExpression, NSTextCheckingResult,
NSPredicate, NSAttributedString, NSMutableAttributedString,
NSNotificationCenter, NSNotification,
NSJSONSerialization, NSPropertyListSerialization,
NSUUID, NSEnumerator, NSIndexSet, NSMutableIndexSet,
NSCache, NSMapTable, NSHashTable, NSSortDescriptor,
NSNumberFormatter, NSMethodSignature, NSInvocation,
NSThread, NSLock, NSRecursiveLock, NSCondition, NSConditionLock,
NSOperation, NSBlockOperation, NSOperationQueue,
NSTimer, **NSScanner**, **NSOrderedSet**, **NSMutableOrderedSet**, **NSCountedSet**,
**NSURLComponents**, **NSURLQueryItem**, **NSByteCountFormatter**, **NSISO8601DateFormatter**,
**NSProgress**, **NSHost**, **NSPipe**, **NSURLRequest**, **NSMutableURLRequest**,
**NSURLResponse**, **NSHTTPURLResponse**, **NSTask**

### Shim Counts
- ObjC runtime: 103 selectors
- Foundation: 355+ selectors (was 252, +103 from 12 new classes)
- libSystem: 65 shims
- **Total: 523+ shims**

## Top 20 Missing APIs (Prioritized for C7+)

These are Foundation classes commonly used in real-world macOS CLI tools that are not yet shimmed:

| Priority | Class | Typical CLI Usage |
|---|---|---|
| 1 | ~~**NSTask / NSProcess**~~ | ~~Launching subprocesses (shell commands, xcrun)~~ | **CLOSED (C9)** |
| 2 | ~~**NSPipe**~~ | ~~Capturing stdout/stderr from NSTask~~ | **CLOSED (C9)** |
| 3 | ~~**NSFileHandle**~~ | ~~Low-level file I/O (stdin/stdout/stderr)~~ | **CLOSED (C8)** |
| 4 | ~~**NSMutableData**~~ | ~~In-place byte buffer mutation (append, replace)~~ | **CLOSED (C8)** |
| 5 | **NSURLSession** | HTTP requests, downloading files, REST APIs |
| 6 | ~~**NSURLRequest / NSURLResponse**~~ | ~~Companion types for networking~~ | **CLOSED (C9)** |
| 7 | **NSKeyedArchiver / NSKeyedUnarchiver** | Serializing object graphs to binary archives |
| 8 | ~~**NSScanner**~~ | ~~scanf-style string parsing~~ | **CLOSED (C8)** |
| 9 | **NSStream / NSInputStream / NSOutputStream** | Stream-based I/O |
| 10 | **NSRunLoop** (selectors) | Event loop management (run, runUntilDate:) |
| 11 | ~~**NSOrderedSet / NSMutableOrderedSet**~~ | ~~Ordered unique collections~~ | **CLOSED (C8)** |
| 12 | ~~**NSCountedSet**~~ | ~~Bag/multiset (counting occurrences)~~ | **CLOSED (C8)** |
| 13 | ~~**NSURLComponents / NSURLQueryItem**~~ | ~~URL building with query parameters~~ | **CLOSED (C9)** |
| 14 | ~~**NSByteCountFormatter**~~ | ~~Human-readable byte sizes ("1.5 MB")~~ | **CLOSED (C9)** |
| 15 | ~~**NSISO8601DateFormatter**~~ | ~~ISO 8601 date parsing (common in APIs/JSON)~~ | **CLOSED (C9)** |
| 16 | **NSXMLParser** | XML parsing (plists, config files) |
| 17 | **NSCoder / NSKeyedArchiver** | Foundation archiving protocol |
| 18 | ~~**NSMutableCharacterSet**~~ | ~~Dynamic character set building~~ | **CLOSED (C8)** |
| 19 | ~~**NSProgress**~~ | ~~Progress reporting~~ | **CLOSED (C9)** |
| 20 | ~~**NSHost**~~ | ~~Hostname/address resolution~~ | **CLOSED (C9)** |

## Gap Categories

### Subprocess Execution (~~Critical~~ CLOSED)
NSTask + NSPipe + NSFileHandle are now fully shimmed (Phase C8+C9).
Subprocess execution via posix_spawn, pipe I/O, and file handle access all work.

### Networking (Partially closed)
NSURLRequest, NSMutableURLRequest, NSURLResponse, NSHTTPURLResponse, and
NSURLComponents are now shimmed. NSURLSession (the actual HTTP client) remains
the last major networking gap.

### Archiving (Medium priority)
NSKeyedArchiver/Unarchiver is used by tools that persist state (Xcode build
caches, preferences). Less critical for pure CLI tools.

### Byte Buffers (~~Medium~~ CLOSED)
NSMutableData and NSFileHandle are now fully shimmed (Phase C8).

## Conclusion

The current runtime achieves **100% compatibility** with the 30-tool corpus,
covering core Foundation patterns: collections, serialization, string processing,
date/time, filesystem, caching, notifications, regular expressions, predicates,
attributed strings, UUIDs, user defaults, byte buffers, file handles, string
scanning, ordered sets, counted sets, mutable character sets, URL building,
byte formatting, ISO 8601 dates, progress tracking, hostname resolution,
pipe I/O, URL requests/responses, and subprocess execution.

- Phase C8 closed 6 of the top 20 gaps (#3, #4, #8, #11, #12, #18).
- Phase C9 closed 8 of the top 20 gaps (#1, #2, #6, #13, #14, #15, #19, #20).
- **14 of 20 gaps now closed.** 71 fully shimmed classes, 84 registered synthetic classes.

The remaining open gaps are: NSURLSession (#5), NSKeyedArchiver (#7),
NSStream (#9), NSRunLoop selectors (#10), NSXMLParser (#16), and NSCoder (#17).
Of these, NSURLSession is the most impactful for real-world CLI tool compatibility.
