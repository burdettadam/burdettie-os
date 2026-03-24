// real_cli — A realistic Foundation CLI tool exercising many classes together.
//
// This program acts as a "directory reporter": it inspects process info,
// scans a directory, builds a sorted file list, formats a summary report,
// handles error cases, and produces structured output. It exercises:
//   NSString (path utils, format, compare, mutation)
//   NSMutableString (report building)
//   NSArray (creation, enumeration, sorting with blocks)
//   NSMutableArray (building file lists)
//   NSDictionary (config/metadata)
//   NSMutableDictionary (building reports)
//   NSNumber (counts, statistics)
//   NSData (raw byte handling)
//   NSProcessInfo (process identity)
//   NSFileManager (directory scanning, file existence)
//   NSBundle (bundle paths)
//   NSError (error propagation)
//   NSException (error objects)
//   NSLog (formatted output)
//   Blocks (enumerateObjectsUsingBlock:, sortedArrayUsingComparator:)
//   @autoreleasepool
//
// Compiled with:
//   xcrun clang -fno-objc-arc -framework Foundation -arch arm64 -o app main.m

#import <Foundation/Foundation.h>
#include <objc/runtime.h>
#include <objc/message.h>

// ---- typed objc_msgSend casts ----
typedef id (*send0_t)(id, SEL);
typedef id (*send1_t)(id, SEL, id);
typedef id (*send2_t)(id, SEL, id, id);
typedef id (*send3_t)(id, SEL, id, id, id);
typedef id (*sendI_t)(id, SEL, NSInteger);
typedef id (*sendU64_t)(id, SEL, NSUInteger);
typedef id (*sendPL_t)(id, SEL, const void *, NSUInteger);
typedef id (*sendS_t)(id, SEL, const char *);
typedef id (*sendB_t)(id, SEL, BOOL);
typedef BOOL (*bsend0_t)(id, SEL);
typedef BOOL (*bsend1_t)(id, SEL, id);
typedef NSInteger (*isend0_t)(id, SEL);
typedef NSUInteger (*usend0_t)(id, SEL);
typedef double (*dsend0_t)(id, SEL);
typedef const char *(*csend0_t)(id, SEL);
typedef id (*sendOC_t)(id, SEL, id, NSUInteger);
typedef id (*sendIC_t)(id, SEL, int);
typedef id (*sendDC_t)(id, SEL, double);

int main() {
    @autoreleasepool {
        send0_t send0 = (send0_t)objc_msgSend;
        send1_t send1 = (send1_t)objc_msgSend;
        send2_t send2 = (send2_t)objc_msgSend;
        send3_t send3 = (send3_t)objc_msgSend;
        sendI_t sendI = (sendI_t)objc_msgSend;
        sendU64_t sendU64 = (sendU64_t)objc_msgSend;
        sendPL_t sendPL = (sendPL_t)objc_msgSend;
        sendS_t sendS = (sendS_t)objc_msgSend;
        sendB_t sendB = (sendB_t)objc_msgSend;
        bsend1_t bsend1 = (bsend1_t)objc_msgSend;
        isend0_t isend0 = (isend0_t)objc_msgSend;
        usend0_t usend0 = (usend0_t)objc_msgSend;
        csend0_t csend0 = (csend0_t)objc_msgSend;
        sendIC_t sendIC = (sendIC_t)objc_msgSend;
        sendDC_t sendDC = (sendDC_t)objc_msgSend;

        Class strCls = objc_getClass("NSString");
        Class mutStrCls = objc_getClass("NSMutableString");
        Class arrCls = objc_getClass("NSArray");
        Class mutArrCls = objc_getClass("NSMutableArray");
        Class dictCls = objc_getClass("NSDictionary");
        Class mutDictCls = objc_getClass("NSMutableDictionary");
        Class numCls = objc_getClass("NSNumber");
        Class dataCls = objc_getClass("NSData");
        Class procInfoCls = objc_getClass("NSProcessInfo");
        Class fmCls = objc_getClass("NSFileManager");
        Class bundleCls = objc_getClass("NSBundle");
        Class errorCls = objc_getClass("NSError");
        Class excCls = objc_getClass("NSException");

        SEL swu = sel_registerName("stringWithUTF8String:");
        SEL utf8 = sel_registerName("UTF8String");
        SEL desc = sel_registerName("description");
        SEL count = sel_registerName("count");

        // ========== SECTION 1: Process Identity ==========
        // Get process info
        id procInfo = send0((id)procInfoCls, sel_registerName("processInfo"));
        if (!procInfo) { printf("FAIL: 1 - processInfo nil\n"); return 1; }

        id procName = send0(procInfo, sel_registerName("processName"));
        const char *pn = csend0(procName, utf8);
        if (!pn || strlen(pn) == 0) { printf("FAIL: 1 - processName empty\n"); return 1; }
        printf("CHECK 1: processName = %s\n", pn);

        NSInteger pid = isend0(procInfo, sel_registerName("processIdentifier"));
        if (pid <= 0) { printf("FAIL: 2 - pid <= 0\n"); return 2; }
        printf("CHECK 2: pid = %ld\n", (long)pid);

        // ========== SECTION 2: Bundle & Path Info ==========
        id bundle = send0((id)bundleCls, sel_registerName("mainBundle"));
        id bundlePath = send0(bundle, sel_registerName("bundlePath"));
        const char *bp = csend0(bundlePath, utf8);
        if (!bp || strlen(bp) == 0) { printf("FAIL: 3 - bundlePath empty\n"); return 3; }
        printf("CHECK 3: bundlePath = %s\n", bp);

        // Use path utilities on the bundle path
        id pathExt = send0(bundlePath, sel_registerName("pathExtension"));
        id lastComp = send0(bundlePath, sel_registerName("lastPathComponent"));
        const char *lc = csend0(lastComp, utf8);
        printf("CHECK 4: lastPathComponent = %s\n", lc);

        BOOL isAbs = ((bsend0_t)objc_msgSend)(bundlePath, sel_registerName("isAbsolutePath"));
        if (!isAbs) { printf("FAIL: 5 - bundlePath not absolute\n"); return 5; }
        printf("CHECK 5: bundlePath isAbsolutePath = YES\n");

        // ========== SECTION 3: File System Operations ==========
        id fm = send0((id)fmCls, sel_registerName("defaultManager"));

        // Get current directory
        id cwd = send0(fm, sel_registerName("currentDirectoryPath"));
        const char *cwdStr = csend0(cwd, utf8);
        printf("CHECK 6: cwd = %s\n", cwdStr);

        // Check that cwd exists
        BOOL cwdExists = bsend1(fm, sel_registerName("fileExistsAtPath:"), cwd);
        if (!cwdExists) { printf("FAIL: 7 - cwd does not exist\n"); return 7; }
        printf("CHECK 7: cwd exists = YES\n");

        // List files in /tmp (a directory that always exists)
        id tmpPath = sendS((id)strCls, swu, "/tmp");
        NSError *dirErr = nil;
        id files = ((id(*)(id,SEL,id,NSError**))objc_msgSend)(
            fm, sel_registerName("contentsOfDirectoryAtPath:error:"),
            tmpPath, &dirErr);
        if (!files) { printf("FAIL: 8 - contentsOfDirectoryAtPath nil\n"); return 8; }
        NSUInteger fileCount = usend0(files, count);
        printf("CHECK 8: /tmp has %lu entries\n", (unsigned long)fileCount);

        // Try a non-existent path — should get an error
        id badPath = sendS((id)strCls, swu, "/nonexistent_path_xyz");
        NSError *badErr = nil;
        id badFiles = ((id(*)(id,SEL,id,NSError**))objc_msgSend)(
            fm, sel_registerName("contentsOfDirectoryAtPath:error:"),
            badPath, &badErr);
        if (badFiles) { printf("FAIL: 9 - nonexistent path returned files\n"); return 9; }
        if (!badErr) { printf("FAIL: 9 - no error for nonexistent path\n"); return 9; }
        NSInteger errCode = isend0((id)badErr, sel_registerName("code"));
        printf("CHECK 9: error code = %ld (NSFileReadNoSuchFileError)\n", (long)errCode);

        // ========== SECTION 4: Building a Config Dictionary ==========
        id keyApp = sendS((id)strCls, swu, "appName");
        id keyVer = sendS((id)strCls, swu, "version");
        id keyDebug = sendS((id)strCls, swu, "debug");
        id valApp = sendS((id)strCls, swu, "DirReporter");
        id valVer = sendS((id)strCls, swu, "1.0");
        id valDebug = sendB((id)numCls, sel_registerName("numberWithBool:"), YES);

        id keys[3] = { keyApp, keyVer, keyDebug };
        id vals[3] = { valApp, valVer, valDebug };
        id config = ((id(*)(id,SEL,id*,id*,NSUInteger))objc_msgSend)(
            (id)dictCls, sel_registerName("dictionaryWithObjects:forKeys:count:"),
            vals, keys, 3);
        if (!config) { printf("FAIL: 10 - config dict nil\n"); return 10; }
        NSUInteger configCount = usend0(config, count);
        if (configCount != 3) { printf("FAIL: 10 - config count %lu\n", (unsigned long)configCount); return 10; }

        // Retrieve a value
        id appNameVal = send1(config, sel_registerName("objectForKey:"), keyApp);
        const char *an = csend0(appNameVal, utf8);
        if (strcmp(an, "DirReporter") != 0) { printf("FAIL: 10 - appName mismatch\n"); return 10; }
        printf("CHECK 10: config[appName] = %s, count = %lu\n", an, (unsigned long)configCount);

        // ========== SECTION 5: Building a File List with NSMutableArray ==========
        id fileList = send0((id)mutArrCls, sel_registerName("array"));

        // Add some simulated filenames
        const char *names[] = {"README.md", "Cargo.toml", "main.rs",
                               "lib.rs", "build.sh", "test.py"};
        for (int i = 0; i < 6; i++) {
            id fname = sendS((id)strCls, swu, names[i]);
            send1(fileList, sel_registerName("addObject:"), fname);
        }
        NSUInteger listCount = usend0(fileList, count);
        if (listCount != 6) { printf("FAIL: 11 - fileList count %lu\n", (unsigned long)listCount); return 11; }
        printf("CHECK 11: fileList count = %lu\n", (unsigned long)listCount);

        // Sort alphabetically using a comparator block
        typedef NSComparisonResult (*cmp_t)(id, SEL, id);
        cmp_t cmpSend = (cmp_t)objc_msgSend;
        id sorted = send1(fileList, sel_registerName("sortedArrayUsingComparator:"),
            (id)^NSComparisonResult(id a, id b) {
                return cmpSend(a, sel_registerName("compare:"), b);
            });
        // Verify sort order: build.sh < Cargo.toml (case-sensitive) actually...
        // On case-sensitive comparison: uppercase < lowercase in ASCII
        id first = sendU64(sorted, sel_registerName("objectAtIndex:"), 0);
        const char *firstName = csend0(first, utf8);
        printf("CHECK 12: sorted first = %s\n", firstName);

        // ========== SECTION 6: String Manipulation ==========
        // Build a report header with NSMutableString
        id report = send0((id)mutStrCls, sel_registerName("string"));
        send1(report, sel_registerName("appendString:"),
              sendS((id)strCls, swu, "=== Directory Report ===\n"));
        send1(report, sel_registerName("appendString:"),
              sendS((id)strCls, swu, "App: DirReporter v1.0\n"));
        send1(report, sel_registerName("appendString:"),
              sendS((id)strCls, swu, "========================\n"));

        NSUInteger reportLen = usend0(report, sel_registerName("length"));
        if (reportLen == 0) { printf("FAIL: 13 - report empty\n"); return 13; }
        printf("CHECK 13: report length = %lu\n", (unsigned long)reportLen);

        // String operations
        id prefix = sendS((id)strCls, swu, "===");
        BOOL hasPfx = bsend1(report, sel_registerName("hasPrefix:"), prefix);
        if (!hasPfx) { printf("FAIL: 14 - report doesn't start with ===\n"); return 14; }
        printf("CHECK 14: report hasPrefix '===' = YES\n");

        // ========== SECTION 7: Enumerate Files with Block ==========
        // Build a summary using enumerateObjectsUsingBlock:
        __block int dotCount = 0;
        ((void(*)(id,SEL,id))objc_msgSend)(sorted,
            sel_registerName("enumerateObjectsUsingBlock:"),
            (id)^(id obj, NSUInteger idx, BOOL *stop) {
                const char *s = csend0(obj, utf8);
                // Count files with dots
                if (strchr(s, '.') != NULL) {
                    dotCount++;
                }
            });
        // All 6 files have extensions
        if (dotCount != 6) { printf("FAIL: 15 - dotCount %d\n", dotCount); return 15; }
        printf("CHECK 15: files with extensions = %d\n", dotCount);

        // ========== SECTION 8: NSNumber + Statistics ==========
        id countNum = sendIC((id)numCls, sel_registerName("numberWithInt:"), (int)listCount);
        int countBack = (int)isend0(countNum, sel_registerName("intValue"));
        if (countBack != 6) { printf("FAIL: 16 - countBack %d\n", countBack); return 16; }

        id pi = sendDC((id)numCls, sel_registerName("numberWithDouble:"), 3.14159);
        double piBack = ((dsend0_t)objc_msgSend)(pi, sel_registerName("doubleValue"));
        if (piBack < 3.14 || piBack > 3.15) { printf("FAIL: 16 - pi %f\n", piBack); return 16; }
        printf("CHECK 16: NSNumber roundtrip int=%d, double=%.2f\n", countBack, piBack);

        // ========== SECTION 9: NSData ==========
        const char *rawBytes = "Hello, Foundation!";
        id data = sendPL((id)dataCls, sel_registerName("dataWithBytes:length:"),
                         rawBytes, strlen(rawBytes));
        if (!data) { printf("FAIL: 17 - NSData nil\n"); return 17; }
        NSUInteger dataLen = usend0(data, sel_registerName("length"));
        if (dataLen != strlen(rawBytes)) {
            printf("FAIL: 17 - length %lu vs %lu\n", (unsigned long)dataLen,
                   (unsigned long)strlen(rawBytes));
            return 17;
        }
        printf("CHECK 17: NSData length = %lu\n", (unsigned long)dataLen);

        // ========== SECTION 10: NSException for error objects ==========
        id excName = sendS((id)strCls, swu, "InvalidArgument");
        id excReason = sendS((id)strCls, swu, "Path cannot be empty");
        id exc = send3((id)excCls, sel_registerName("exceptionWithName:reason:userInfo:"),
                       excName, excReason, nil);
        if (!exc) { printf("FAIL: 18 - exception nil\n"); return 18; }
        id excNameBack = send0(exc, sel_registerName("name"));
        const char *en = csend0(excNameBack, utf8);
        if (strcmp(en, "InvalidArgument") != 0) {
            printf("FAIL: 18 - exc name %s\n", en); return 18;
        }
        printf("CHECK 18: NSException name = %s\n", en);

        // ========== SECTION 11: NSMutableDictionary — build results ==========
        id results = send0((id)mutDictCls, sel_registerName("dictionary"));
        send2(results, sel_registerName("setObject:forKey:"),
              countNum, sendS((id)strCls, swu, "totalFiles"));
        send2(results, sel_registerName("setObject:forKey:"),
              sendIC((id)numCls, sel_registerName("numberWithInt:"), dotCount),
              sendS((id)strCls, swu, "filesWithExt"));
        send2(results, sel_registerName("setObject:forKey:"),
              procName, sendS((id)strCls, swu, "processName"));

        NSUInteger resCount = usend0(results, count);
        if (resCount != 3) { printf("FAIL: 19 - results count %lu\n", (unsigned long)resCount); return 19; }
        printf("CHECK 19: results dict count = %lu\n", (unsigned long)resCount);

        // Look up a value from mutable dict
        id totalKey = sendS((id)strCls, swu, "totalFiles");
        id totalVal = send1(results, sel_registerName("objectForKey:"), totalKey);
        int total = (int)isend0(totalVal, sel_registerName("intValue"));
        if (total != 6) { printf("FAIL: 19 - total %d\n", total); return 19; }

        // ========== SECTION 12: Dictionary Enumeration with Block ==========
        __block int kvCount = 0;
        ((void(*)(id,SEL,id))objc_msgSend)(results,
            sel_registerName("enumerateKeysAndObjectsUsingBlock:"),
            (id)^(id key, id obj, BOOL *stop) {
                kvCount++;
            });
        if (kvCount != 3) { printf("FAIL: 20 - kvCount %d\n", kvCount); return 20; }
        printf("CHECK 20: enumerated %d key-value pairs\n", kvCount);

        // ========== SECTION 13: NSString Path Utilities ==========
        id fullPath = sendS((id)strCls, swu, "/Users/test/Documents/report.txt");
        id pathLast = send0(fullPath, sel_registerName("lastPathComponent"));
        id pathDir = send0(fullPath, sel_registerName("stringByDeletingLastPathComponent"));
        id pathNoExt = send0(fullPath, sel_registerName("stringByDeletingPathExtension"));
        id pathExt2 = send0(fullPath, sel_registerName("pathExtension"));

        const char *plStr = csend0(pathLast, utf8);
        const char *pdStr = csend0(pathDir, utf8);
        const char *pnStr = csend0(pathNoExt, utf8);
        const char *peStr = csend0(pathExt2, utf8);

        if (strcmp(plStr, "report.txt") != 0) { printf("FAIL: 21 - lastPathComponent %s\n", plStr); return 21; }
        if (strcmp(pdStr, "/Users/test/Documents") != 0) { printf("FAIL: 21 - dir %s\n", pdStr); return 21; }
        if (strcmp(peStr, "txt") != 0) { printf("FAIL: 21 - ext %s\n", peStr); return 21; }
        printf("CHECK 21: path = %s | dir = %s | ext = %s\n", plStr, pdStr, peStr);

        // Build a new path with stringByAppendingPathComponent:
        id basePath = sendS((id)strCls, swu, "/Users/test");
        id newPath = send1(basePath, sel_registerName("stringByAppendingPathComponent:"),
                          sendS((id)strCls, swu, "output.log"));
        const char *np = csend0(newPath, utf8);
        if (strcmp(np, "/Users/test/output.log") != 0) {
            printf("FAIL: 22 - appendPathComponent %s\n", np); return 22;
        }
        printf("CHECK 22: appendPathComponent = %s\n", np);

        // ========== SECTION 14: String Search & Split ==========
        id csv = sendS((id)strCls, swu, "apple,banana,cherry,date");
        id parts = send1(csv, sel_registerName("componentsSeparatedByString:"),
                        sendS((id)strCls, swu, ","));
        NSUInteger partCount = usend0(parts, count);
        if (partCount != 4) { printf("FAIL: 23 - partCount %lu\n", (unsigned long)partCount); return 23; }
        id secondPart = sendU64(parts, sel_registerName("objectAtIndex:"), 1);
        const char *sp = csend0(secondPart, utf8);
        if (strcmp(sp, "banana") != 0) { printf("FAIL: 23 - part[1] = %s\n", sp); return 23; }
        printf("CHECK 23: split result[1] = %s, count = %lu\n", sp, (unsigned long)partCount);

        // containsObject: on the parts array
        id cherry = sendS((id)strCls, swu, "cherry");
        BOOL foundCherry = bsend1(parts, sel_registerName("containsObject:"), cherry);
        if (!foundCherry) { printf("FAIL: 24 - cherry not found\n"); return 24; }
        id grape = sendS((id)strCls, swu, "grape");
        BOOL foundGrape = bsend1(parts, sel_registerName("containsObject:"), grape);
        if (foundGrape) { printf("FAIL: 24 - grape found\n"); return 24; }
        printf("CHECK 24: containsObject cherry=YES, grape=NO\n");

        // ========== SECTION 15: Case Transform & NSError Description ==========
        id mixed = sendS((id)strCls, swu, "Hello World");
        id lower = send0(mixed, sel_registerName("lowercaseString"));
        id upper = send0(mixed, sel_registerName("uppercaseString"));
        const char *lo = csend0(lower, utf8);
        const char *up = csend0(upper, utf8);
        if (strcmp(lo, "hello world") != 0) { printf("FAIL: 25 - lower = %s\n", lo); return 25; }
        if (strcmp(up, "HELLO WORLD") != 0) { printf("FAIL: 25 - upper = %s\n", up); return 25; }
        printf("CHECK 25: lower = %s, upper = %s\n", lo, up);

        // NSError description
        id errDomain = sendS((id)strCls, swu, "com.test.error");
        id err = send3((id)errorCls, sel_registerName("errorWithDomain:code:userInfo:"),
                       errDomain, (id)42, nil);
        id errDesc = send0(err, sel_registerName("localizedDescription"));
        const char *ed = csend0(errDesc, utf8);
        if (!ed || strlen(ed) == 0) { printf("FAIL: 26 - error desc empty\n"); return 26; }
        printf("CHECK 26: NSError desc = %s\n", ed);

        printf("OK: real_cli — all 26 checks passed\n");
        return 0;
    }
}
