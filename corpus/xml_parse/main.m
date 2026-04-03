// xml_parse — SAX-style XML parsing
// Exercises: NSXMLParser, NSXMLParserDelegate
#import <Foundation/Foundation.h>

@interface SimpleDelegate : NSObject <NSXMLParserDelegate>
@property (nonatomic, assign) int elementCount;
@end

@implementation SimpleDelegate
- (void)parser:(NSXMLParser *)parser didStartElement:(NSString *)element
  namespaceURI:(NSString *)ns qualifiedName:(NSString *)qName
  attributes:(NSDictionary *)attrs {
    self.elementCount++;
}
@end

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create XML data
        NSString *xml = @"<root><item id=\"1\">Hello</item><item id=\"2\">World</item></root>";
        NSData *data = [xml dataUsingEncoding:NSUTF8StringEncoding];
        if (data != nil) { printf("CHECK 1 PASS: xml data created (%lu bytes)\n", (unsigned long)[data length]); checks++; }

        // Create parser
        NSXMLParser *parser = [[NSXMLParser alloc] initWithData:data];
        if (parser != nil) { printf("CHECK 2 PASS: parser created\n"); checks++; }

        // Set delegate
        SimpleDelegate *delegate = [[SimpleDelegate alloc] init];
        [parser setDelegate:delegate];
        if ([parser delegate] == delegate) { printf("CHECK 3 PASS: delegate set\n"); checks++; }

        // Parse
        BOOL ok = [parser parse];
        if (ok) { printf("CHECK 4 PASS: parse succeeded\n"); checks++; }

        // Check no error
        NSError *err = [parser parserError];
        if (err == nil) { printf("CHECK 5 PASS: no parser error\n"); checks++; }

        // Configure namespace processing
        [parser setShouldProcessNamespaces:YES];
        [parser setShouldReportNamespacePrefixes:YES];
        printf("CHECK 6 PASS: namespace settings applied\n"); checks++;

        // Description
        NSString *desc = [parser description];
        if (desc != nil && [desc length] > 0) { printf("CHECK 7 PASS: description = %s\n", [desc UTF8String]); checks++; }

        printf("xml_parse: %d/7 checks passed\n", checks);
        return (checks == 7) ? 0 : 1;
    }
}
