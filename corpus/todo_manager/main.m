// todo_manager — in-memory task manager
// Exercises: NSMutableArray, NSMutableDictionary, NSNumber, NSDate, NSSortDescriptor, blocks, NSString
#import <Foundation/Foundation.h>

static NSMutableDictionary *createTask(NSString *title, int priority, BOOL done) {
    NSMutableDictionary *task = [NSMutableDictionary dictionary];
    [task setObject:title forKey:@"title"];
    [task setObject:[NSNumber numberWithInt:priority] forKey:@"priority"];
    [task setObject:[NSNumber numberWithBool:done] forKey:@"done"];
    [task setObject:[NSDate date] forKey:@"created"];
    return task;
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        int checks = 0;

        // Create task list
        NSMutableArray *tasks = [NSMutableArray array];
        [tasks addObject:createTask(@"Buy groceries", 2, NO)];
        [tasks addObject:createTask(@"Write report", 1, NO)];
        [tasks addObject:createTask(@"Fix bug #123", 1, YES)];
        [tasks addObject:createTask(@"Review PR", 3, NO)];
        [tasks addObject:createTask(@"Deploy release", 1, NO)];
        [tasks addObject:createTask(@"Update docs", 3, YES)];

        if ([tasks count] == 6) { printf("CHECK 1 PASS: 6 tasks created\n"); checks++; }

        // Filter: incomplete tasks
        NSMutableArray *incomplete = [NSMutableArray array];
        [tasks enumerateObjectsUsingBlock:^(NSDictionary *task, NSUInteger idx, BOOL *stop) {
            if (![[task objectForKey:@"done"] boolValue]) {
                [incomplete addObject:task];
            }
        }];

        if ([incomplete count] == 4) { printf("CHECK 2 PASS: 4 incomplete tasks\n"); checks++; }

        // Sort by priority (ascending = highest priority first, since 1 = highest)
        NSArray *sorted = [incomplete sortedArrayUsingComparator:^NSComparisonResult(NSDictionary *a, NSDictionary *b) {
            return [[a objectForKey:@"priority"] compare:[b objectForKey:@"priority"]];
        }];

        NSString *firstTitle = [[sorted firstObject] objectForKey:@"title"];
        int firstPriority = [[[sorted firstObject] objectForKey:@"priority"] intValue];
        if (firstPriority == 1) { printf("CHECK 3 PASS: highest priority first (%s)\n", [firstTitle UTF8String]); checks++; }

        // Count by priority
        NSMutableDictionary *countByPriority = [NSMutableDictionary dictionary];
        [tasks enumerateObjectsUsingBlock:^(NSDictionary *task, NSUInteger idx, BOOL *stop) {
            NSNumber *pri = [task objectForKey:@"priority"];
            NSString *key = [NSString stringWithFormat:@"p%d", [pri intValue]];
            NSNumber *existing = [countByPriority objectForKey:key];
            int count = existing ? [existing intValue] + 1 : 1;
            [countByPriority setObject:[NSNumber numberWithInt:count] forKey:key];
        }];

        if ([[countByPriority objectForKey:@"p1"] intValue] == 3) { printf("CHECK 4 PASS: 3 priority-1 tasks\n"); checks++; }

        // Complete a task
        NSMutableDictionary *first = [tasks objectAtIndex:0];
        [first setObject:[NSNumber numberWithBool:YES] forKey:@"done"];
        if ([[first objectForKey:@"done"] boolValue]) { printf("CHECK 5 PASS: task marked done\n"); checks++; }

        // Remove completed tasks
        NSMutableArray *active = [NSMutableArray array];
        for (NSUInteger i = 0; i < [tasks count]; i++) {
            NSDictionary *task = [tasks objectAtIndex:i];
            if (![[task objectForKey:@"done"] boolValue]) {
                [active addObject:task];
            }
        }

        if ([active count] == 3) { printf("CHECK 6 PASS: 3 active tasks remain\n"); checks++; }

        // Verify task titles contain expected strings
        __block BOOL foundDeploy = NO;
        [active enumerateObjectsUsingBlock:^(NSDictionary *task, NSUInteger idx, BOOL *stop) {
            NSString *title = [task objectForKey:@"title"];
            if ([title isEqualToString:@"Deploy release"]) {
                foundDeploy = YES;
            }
        }];
        if (foundDeploy) { printf("CHECK 7 PASS: Deploy release in active list\n"); checks++; }

        // Summary
        int done = 0;
        for (NSUInteger i = 0; i < [tasks count]; i++) {
            if ([[[tasks objectAtIndex:i] objectForKey:@"done"] boolValue]) done++;
        }
        if (done == 3) { printf("CHECK 8 PASS: 3/6 tasks completed\n"); checks++; }

        printf("todo_manager: %d/8 checks passed\n", checks);
        return (checks == 8) ? 0 : 1;
    }
}
