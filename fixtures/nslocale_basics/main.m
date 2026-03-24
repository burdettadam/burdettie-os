// nslocale_basics — Phase 11 fixture: NSLocale information
#import <Foundation/Foundation.h>
#include <stdio.h>

int main() {
    @autoreleasepool {
        // 1. currentLocale singleton
        NSLocale *locale = [NSLocale currentLocale];
        if (!locale) { printf("FAIL: currentLocale nil\n"); return 1; }

        // 2. localeIdentifier
        NSString *ident = [locale localeIdentifier];
        if (!ident) { printf("FAIL: localeIdentifier nil\n"); return 2; }
        printf("identifier=%s\n", [ident UTF8String]);

        // 3. languageCode
        NSString *lang = [locale languageCode];
        if (!lang) { printf("FAIL: languageCode nil\n"); return 3; }
        printf("language=%s\n", [lang UTF8String]);

        // 4. countryCode (may be nil for language-only locales)
        NSString *country = [locale countryCode];
        if (country) {
            printf("country=%s\n", [country UTF8String]);
        } else {
            printf("country=(nil)\n");
        }

        // 5. Singleton consistency
        NSLocale *locale2 = [NSLocale currentLocale];
        if (locale != locale2) { printf("FAIL: not singleton\n"); return 4; }

        printf("OK\n");
    }
    return 0;
}
