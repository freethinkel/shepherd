// iPhone Duo's hinge and how it's held, for Shepherd's Device tab: a small program run inside the simulator
// (`simctl spawn`), which registers the virtual sensors the Duo reads them from and reports to them. One command per
// line on stdin, `OK` or `ERROR` back per line, until EOF:
//   angle <0…180>            the hinge, in degrees (0 closed, 180 flat)
//   sweep <from> <to> <ms>   the hinge moved from one angle to another over a while, easing out, as a hand does:
//                            SpringBoard's fold transition follows the hinge, and a jump can leave it stuck
//   orientation <name>       how it's held: portrait, pud, landscape-left, landscape-right, faceup, facedown
// Built on first use with the simulator SDK (src/device/duo.rs). From serve-sim's SimDuoHID
// (https://github.com/EvanBacon/serve-sim, Sources/SimDuoHID/main.m at 0.1.49), Apache License 2.0, Copyright 2026
// Evan Bacon; changed: orientations by name (adding faceup and facedown), no buttons.
#import <Foundation/Foundation.h>
#import <dlfcn.h>
#import <mach/mach_time.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <math.h>
#import <unistd.h>

@interface DuoServiceDelegate : NSObject
@property NSDictionary *properties;
@end
@implementation DuoServiceDelegate
- (id)propertyForKey:(NSString *)key forService:(id)service { return self.properties[key]; }
- (BOOL)setProperty:(id)value forKey:(NSString *)key forService:(id)service { return YES; }
- (id)copyEventMatching:(id)matching forService:(id)service { return nil; }
- (BOOL)setOutputEvent:(id)event forService:(id)service { return YES; }
- (void)notification:(uint32_t)type withProperty:(id)property forService:(id)service {}
@end

static void header(NSMutableData *data, uint32_t count, uint8_t type) {
    uint8_t bytes[] = {count & 255, (count >> 8) & 255, (count >> 16) & 255, type};
    [data appendBytes:bytes length:4];
}
static void string(NSMutableData *data, const char *text, BOOL key, BOOL last) {
    size_t length = strlen(text) + (key ? 1 : 0);
    header(data, (uint32_t)length, (key ? 8 : 9) | (last ? 0x80 : 0));
    [data appendBytes:text length:length];
    while (data.length % 4) { uint8_t zero = 0; [data appendBytes:&zero length:1]; }
}
// The vendor event a Mac's Device Hub sends the Duo: one of its controls and the value it's set to.
static NSMutableData *control(const char *source, const char *type) {
    NSMutableData *data = [NSMutableData data];
    header(data, 0xd3, 0);
    header(data, 4, 0x81);
    string(data, "provider", YES, NO); string(data, "com.apple.Virtualization.VirtualMachines", NO, NO);
    string(data, "source", YES, NO); string(data, source, NO, NO);
    string(data, "type", YES, NO); string(data, type, NO, NO);
    string(data, "value", YES, NO);
    return data;
}
static NSData *hingePayload(double degrees) {
    NSMutableData *data = control("hinge-slider-control", "range");
    header(data, 0x3f, 0x84);
    [data appendBytes:&degrees length:sizeof(degrees)];
    return data;
}
static NSData *orientationPayload(const char *name) {
    static const char *names[] = {"portrait", "pud", "landscape-left", "landscape-right", "faceup", "facedown"};
    for (size_t i = 0; i < sizeof(names) / sizeof(*names); i++) {
        if (strcmp(name, names[i]) != 0) continue;
        NSMutableData *data = control("orientation-picker-control", "enum");
        string(data, names[i], NO, YES);
        return data;
    }
    return nil;
}
static id service(uint32_t page, uint32_t usage, BOOL builtIn) {
    Class cls = NSClassFromString(@"HIDVirtualEventService");
    if (!cls) return nil;
    id instance = [[cls alloc] init];
    DuoServiceDelegate *delegate = [DuoServiceDelegate new];
    delegate.properties = @{
        @"PrimaryUsagePage": @(page), @"PrimaryUsage": @(usage),
        @"DeviceUsagePairs": @[@{@"DeviceUsagePage": @(page), @"DeviceUsage": @(usage)}],
        @"Transport": @"CoreDevice", @"Product": @"Shepherd Duo HID",
        @"Built-In": @(builtIn), @"VendorID": @0, @"ProductID": @0,
        @"VersionNumber": @0, @"ReportInterval": @8000,
    };
    objc_setAssociatedObject(instance, @selector(properties), delegate, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    ((void (*)(id, SEL, id))objc_msgSend)(instance, sel_registerName("setDelegate:"), delegate);
    dispatch_queue_t queue = dispatch_queue_create("shepherd.duo", DISPATCH_QUEUE_SERIAL);
    ((void (*)(id, SEL, id))objc_msgSend)(instance, sel_registerName("setDispatchQueue:"), queue);
    ((void (*)(id, SEL))objc_msgSend)(instance, sel_registerName("activate"));
    return ((uint64_t (*)(id, SEL))objc_msgSend)(instance, sel_registerName("serviceID")) ? instance : nil;
}
static BOOL sendEvent(id service, CFTypeRef event) {
    if (!event) return NO;
    BOOL ok = ((BOOL (*)(id, SEL, id))objc_msgSend)(service, sel_registerName("dispatchEvent:"), (__bridge id)event);
    CFRelease(event);
    return ok;
}
int main(void) {
    @autoreleasepool {
        dlopen("/System/Library/PrivateFrameworks/HID.framework/HID", RTLD_NOW);
        void *io = dlopen("/System/Library/Frameworks/IOKit.framework/IOKit", RTLD_NOW);
        CFTypeRef (*vendor)(CFAllocatorRef, uint64_t, uint32_t, uint32_t, uint32_t, const void *, CFIndex, uint32_t) = dlsym(io, "IOHIDEventCreateVendorDefinedEvent");
        id hinge = service(0xff61, 0x5b, NO);
        if (!vendor || !hinge) { fprintf(stderr, "Duo HID service unavailable\n"); return 1; }
        usleep(300000); // backboardd enumerates the new services
        char *line = NULL; size_t capacity = 0;
        while (getline(&line, &capacity, stdin) > 0) {
            @autoreleasepool {
                double angle, from, ms; char name[32], extra;
                NSData *payload = nil;
                BOOL ok = NO;
                if (sscanf(line, "sweep %lf %lf %lf %c", &from, &angle, &ms, &extra) == 3 && isfinite(from) && from >= 0 &&
                    from <= 180 && isfinite(angle) && angle >= 0 && angle <= 180 && isfinite(ms) && ms >= 0 && ms <= 1500) {
                    // a step a display frame, easing out
                    int steps = MAX(1, (int)ceil(ms / 16.667));
                    ok = YES;
                    for (int i = 1; i <= steps && ok; i++) {
                        double eased = 1 - pow(1 - (double)i / steps, 3);
                        NSData *step = hingePayload(from + (angle - from) * eased);
                        ok = sendEvent(hinge, vendor(NULL, mach_absolute_time(), 0xff61, 0x5b, 0, step.bytes, step.length, 0));
                        if (i < steps) usleep((useconds_t)(ms * 1000 / steps));
                    }
                } else {
                    if (sscanf(line, "angle %lf %c", &angle, &extra) == 1 && isfinite(angle) && angle >= 0 && angle <= 180) {
                        payload = hingePayload(angle);
                    } else if (sscanf(line, "orientation %31s %c", name, &extra) == 1) {
                        payload = orientationPayload(name);
                    }
                    ok = payload && sendEvent(hinge, vendor(NULL, mach_absolute_time(), 0xff61, 0x5b, 0, payload.bytes, payload.length, 0));
                }
                puts(ok ? "OK" : "ERROR"); fflush(stdout);
            }
        }
        free(line);
        ((void (*)(id, SEL))objc_msgSend)(hinge, sel_registerName("cancel"));
    }
    return 0;
}
