/**
 * sdax — accessibility snapshots of a simulator's frontmost app, for simdriver.
 *
 * Reads the iOS accessibility tree from the host side, the way Meta's idb does
 * (FBSimulatorControl, FBSimulatorAccessibilityCommands; MIT): no XCTest bundle,
 * nothing installed in the simulator.
 *
 * Chain:
 *   AXPTranslator (AccessibilityPlatformTranslation.framework, private)
 *     -frontmostApplicationWithDisplayId:bridgeDelegateToken: → translation object
 *     -macPlatformElementFromTranslation: → NSAccessibilityElement tree
 *   every attribute read the translator makes comes back to our bridge delegate,
 *   which forwards it to the guest with -[SimDevice sendAccessibilityRequestAsync:…]
 *
 * Frames are iOS screen points, top-left origin: the same space as sdhid's touches.
 * The first snapshot after launch is slow (~1–2 s: the guest side warms up); later
 * ones take tens of ms, one guest round trip per attribute read.
 *
 * stdin, one JSON object per line:
 *   {"t":"snapshot","id":n}
 * stdout, one JSON object per line:
 *   {"t":"ready"}
 *   {"t":"ax","id":n,"ms":..,"requests":..,"root":node}
 *   {"t":"ax","id":n,"error":".."}      the snapshot failed; sdax keeps running
 *   node = {"role":"AXButton","label"?:..,"value"?:..,"id"?:..,"enabled":bool,
 *           "frame":[x,y,w,h],"children"?:[node…]}
 *
 * Exit codes: 0 stdin EOF / parent gone · 2 usage · 3 CoreSimulator / device not
 * found · 4 this Xcode/macOS lacks the accessibility API sdax needs (the detail names
 * the missing piece).
 *
 * Usage: sdax <udid>
 */
#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#import <objc/message.h>
#import <objc/runtime.h>
#import <dlfcn.h>
#import <unistd.h>

#import "CoreSimulator/SimDeviceSet.h"
#import "CoreSimulator/SimDevice.h"
#import "CoreSimulator/SimDeviceType.h"

static NSString *const kAXFramework =
    @"/System/Library/PrivateFrameworks/AccessibilityPlatformTranslation.framework/AccessibilityPlatformTranslation";
// One guest round trip; past this the read answers empty rather than hang the snapshot.
static const int64_t kRequestTimeoutMs = 3000;
// Bounds on a pathological tree (UIKit only exposes what is on screen, so real trees
// are far smaller).
static const int kMaxDepth = 64;
static const int kMaxNodes = 4000;

// Selectors of the private API, called through these declarations. Each is checked
// with respondsToSelector before use, so an Xcode that renames one exits 4, not crashes.
@protocol SDAXTranslator
+ (instancetype)sharedInstance;
- (void)setBridgeTokenDelegate:(id)delegate;
- (void)setSupportsDelegateTokens:(BOOL)supports;
- (id)frontmostApplicationWithDisplayId:(unsigned)displayId bridgeDelegateToken:(NSString *)token;
- (id)macPlatformElementFromTranslation:(id)translation;
- (id)objectAtPoint:(CGPoint)point displayId:(unsigned)displayId bridgeDelegateToken:(NSString *)token;
@end

@protocol SDAXTranslation
- (void)setBridgeDelegateToken:(NSString *)token;
@end

@protocol SDAXPlatformElement
- (id)translation;
@end

static SimDevice *g_device = nil;
static dispatch_queue_t g_replyQueue = nil;
static int g_requests = 0;
static int g_nodes = 0;
static NSString *g_token = nil;

// ─── stdout protocol ─────────────────────────────────────────────────────────

static void emit(NSDictionary *obj) {
  NSData *json = [NSJSONSerialization dataWithJSONObject:obj options:0 error:nil];
  if (json == nil) return;
  @synchronized ([NSFileHandle class]) {
    fwrite(json.bytes, 1, json.length, stdout);
    fputc('\n', stdout);
    fflush(stdout);
  }
}

static void failWith(int code, NSString *detail) {
  emit(@{ @"t" : @"error", @"detail" : detail });
  _exit(code);
}

// ─── device ──────────────────────────────────────────────────────────────────

static NSString *currentDeveloperDir(void) {
  NSString *fromEnv = NSProcessInfo.processInfo.environment[@"DEVELOPER_DIR"];
  if (fromEnv.length > 0) return fromEnv;
  FILE *pipe = popen("xcode-select -p 2>/dev/null", "r");
  if (pipe == NULL) return @"";
  char line[PATH_MAX];
  NSString *result = @"";
  if (fgets(line, sizeof(line), pipe) != NULL) {
    result = [[NSString stringWithUTF8String:line]
        stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
  }
  pclose(pipe);
  return result;
}

typedef id (*ctx_send_t)(Class, SEL, NSString *, id *);
typedef id (*set_send_t)(id, SEL, id *);

static SimDevice *findDevice(NSString *udid) {
  Class contextClass = NSClassFromString(@"SimServiceContext");
  if (contextClass == nil) failWith(3, @"SimServiceContext unavailable (CoreSimulator not loaded)");
  id error = nil;
  id context = ((ctx_send_t)objc_msgSend)(contextClass, @selector(sharedServiceContextForDeveloperDir:error:),
                                          currentDeveloperDir(), &error);
  if (context == nil) failWith(3, [NSString stringWithFormat:@"SimServiceContext: %@", error]);
  SimDeviceSet *set = ((set_send_t)objc_msgSend)(context, @selector(defaultDeviceSetWithError:), &error);
  if (set == nil) failWith(3, [NSString stringWithFormat:@"device set: %@", error]);
  for (SimDevice *candidate in set.devices) {
    if ([candidate.UDID.UUIDString isEqualToString:udid]) return candidate;
  }
  failWith(3, [NSString stringWithFormat:@"device %@ not found", udid]);
  return nil;
}

// ─── bridge delegate: the translator's attribute reads go to the guest ───────

@interface SDAXBridge : NSObject
@end

@implementation SDAXBridge
- (CGRect)accessibilityTranslationConvertPlatformFrameToSystem:(CGRect)rect withToken:(NSString *)token {
  return rect; // keep guest points
}

- (id)accessibilityTranslationRootParentWithToken:(NSString *)token {
  return nil;
}

- (id)accessibilityTranslationDelegateBridgeCallbackWithToken:(NSString *)token {
  return ^id(id request) {
    g_requests++;
    __block id response = nil;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    [g_device sendAccessibilityRequestAsync:request
                            completionQueue:g_replyQueue
                          completionHandler:^(id r) {
                            response = r;
                            dispatch_semaphore_signal(done);
                          }];
    if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, kRequestTimeoutMs * NSEC_PER_MSEC)) != 0) {
      return ((id(*)(Class, SEL))objc_msgSend)(NSClassFromString(@"AXPTranslatorResponse"),
                                               @selector(emptyResponse));
    }
    return response;
  };
}
@end

// ─── snapshot ────────────────────────────────────────────────────────────────

static id<SDAXTranslator> g_translator = nil;

static void requireSelector(id target, SEL sel, NSString *what) {
  if (![target respondsToSelector:sel]) {
    failWith(4, [NSString stringWithFormat:@"%@ lacks %@", what, NSStringFromSelector(sel)]);
  }
}

/// Load the framework and wire the bridge; exits 4 if any piece is missing.
static void setUpTranslator(void) {
  if (dlopen(kAXFramework.fileSystemRepresentation, RTLD_NOW) == NULL) {
    failWith(4, @"AccessibilityPlatformTranslation.framework not loadable");
  }
  Class translatorClass = NSClassFromString(@"AXPTranslator");
  if (translatorClass == nil) failWith(4, @"AXPTranslator class missing");
  if (NSClassFromString(@"AXPTranslatorResponse") == nil) failWith(4, @"AXPTranslatorResponse class missing");
  requireSelector(translatorClass, @selector(sharedInstance), @"AXPTranslator");
  Protocol *helper = objc_getProtocol("AXPTranslationTokenDelegateHelper");
  if (helper == nil) failWith(4, @"AXPTranslationTokenDelegateHelper protocol missing");
  if (![g_device respondsToSelector:@selector(sendAccessibilityRequestAsync:completionQueue:completionHandler:)]) {
    failWith(4, @"SimDevice lacks sendAccessibilityRequestAsync:completionQueue:completionHandler:");
  }
  g_translator = [translatorClass sharedInstance];
  requireSelector(g_translator, @selector(setBridgeTokenDelegate:), @"AXPTranslator");
  requireSelector(g_translator, @selector(setSupportsDelegateTokens:), @"AXPTranslator");
  requireSelector(g_translator, @selector(frontmostApplicationWithDisplayId:bridgeDelegateToken:), @"AXPTranslator");
  requireSelector(g_translator, @selector(macPlatformElementFromTranslation:), @"AXPTranslator");
  class_addProtocol([SDAXBridge class], helper);
  static SDAXBridge *bridge; // kept alive here: the translator may hold it weakly
  bridge = [SDAXBridge new];
  [g_translator setSupportsDelegateTokens:YES];
  [g_translator setBridgeTokenDelegate:bridge];
}

static NSDictionary *node(id element, int depth) {
  g_nodes++;
  if ([element respondsToSelector:@selector(translation)]) {
    id translation = [(id<SDAXPlatformElement>)element translation];
    if ([translation respondsToSelector:@selector(setBridgeDelegateToken:)]) {
      [(id<SDAXTranslation>)translation setBridgeDelegateToken:g_token];
    }
  }
  NSAccessibilityElement *el = element;
  NSMutableDictionary *out = [NSMutableDictionary new];
  out[@"role"] = el.accessibilityRole ?: @"";
  NSString *label = el.accessibilityLabel;
  if (label.length > 0) out[@"label"] = label;
  id value = el.accessibilityValue;
  if (value != nil && ![value isKindOfClass:NSNull.class]) {
    NSString *text = [value isKindOfClass:NSString.class] ? value : [value description];
    if (text.length > 0) out[@"value"] = text;
  }
  NSString *placeholder = el.accessibilityPlaceholderValue;
  if (placeholder.length > 0) out[@"placeholder"] = placeholder;
  NSString *identifier = el.accessibilityIdentifier;
  if (identifier.length > 0) out[@"id"] = identifier;
  out[@"enabled"] = @(el.isAccessibilityEnabled);
  NSRect f = el.accessibilityFrame;
  out[@"frame"] = @[ @(f.origin.x), @(f.origin.y), @(f.size.width), @(f.size.height) ];
  if (depth < kMaxDepth) {
    NSMutableArray *kids = [NSMutableArray new];
    for (id child in el.accessibilityChildren) {
      if (g_nodes >= kMaxNodes) break;
      [kids addObject:node(child, depth + 1)];
    }
    if (kids.count > 0) out[@"children"] = kids;
  }
  return out;
}

// ─── hit-test fallback ───────────────────────────────────────────────────────
//
// A system prompt raised over an app with its keyboard up (Safari asking for location)
// leaves the frontmost-app tree as SpringBoard's bare status bar: the prompt is on
// screen but not in the tree. Hit-testing still reaches it, as idb's describe-point
// does, so when the tree has nothing to act on we probe a coarse grid of points, climb
// each hit to its outermost container short of full screen, and dump those.

static const double kProbeStep = 40;          // pt between probes; buttons are ≥ 44 pt
static const CFTimeInterval kProbeBudget = 2; // s, so a pathological screen stays bounded

static BOOL actionable(NSDictionary *n) {
  NSString *role = n[@"role"];
  if (!([role isEqual:@"AXApplication"] || [role isEqual:@"AXStaticText"] ||
        [role isEqual:@"AXGenericElement"] || [role isEqual:@"AXGroup"] || [role isEqual:@"AXImage"])) {
    return YES;
  }
  for (NSDictionary *child in n[@"children"]) {
    if (actionable(child)) return YES;
  }
  return NO;
}

static void adoptToken(id element) {
  if (![element respondsToSelector:@selector(translation)]) return;
  id translation = [(id<SDAXPlatformElement>)element translation];
  if ([translation respondsToSelector:@selector(setBridgeDelegateToken:)]) {
    [(id<SDAXTranslation>)translation setBridgeDelegateToken:g_token];
  }
}

static BOOL coversScreen(NSRect f, CGSize screen) {
  return f.size.width * f.size.height >= 0.9 * screen.width * screen.height;
}

/// Containers found by hit-testing, as nodes; empty when nothing answers.
static NSArray *probe(void) {
  if (![(id)g_translator respondsToSelector:@selector(objectAtPoint:displayId:bridgeDelegateToken:)]) return @[];
  CGSize px = g_device.deviceType.mainScreenSize;
  double scale = g_device.deviceType.mainScreenScale ?: 1;
  CGSize screen = CGSizeMake(px.width / scale, px.height / scale);
  NSMutableArray *found = [NSMutableArray new];
  NSMutableArray<NSValue *> *covered = [NSMutableArray new];
  CFAbsoluteTime deadline = CFAbsoluteTimeGetCurrent() + kProbeBudget;
  for (double y = kProbeStep; y < screen.height - kProbeStep / 2; y += kProbeStep) {
    for (double x = kProbeStep / 2; x < screen.width; x += kProbeStep) {
      if (CFAbsoluteTimeGetCurrent() > deadline) return found;
      NSPoint p = NSMakePoint(x, y);
      BOOL seen = NO;
      for (NSValue *r in covered) {
        if (NSPointInRect(p, r.rectValue)) { seen = YES; break; }
      }
      if (seen) continue;
      id hit = [g_translator objectAtPoint:p displayId:0 bridgeDelegateToken:g_token];
      if (hit == nil) continue;
      [(id<SDAXTranslation>)hit setBridgeDelegateToken:g_token];
      NSAccessibilityElement *el = [g_translator macPlatformElementFromTranslation:hit];
      if (el == nil) continue;
      for (int i = 0; i < kMaxDepth; i++) {
        NSAccessibilityElement *parent = el.accessibilityParent;
        if (parent == nil) break;
        adoptToken(parent);
        if ([parent.accessibilityRole isEqual:@"AXApplication"] || coversScreen(parent.accessibilityFrame, screen)) break;
        el = parent;
      }
      NSRect f = el.accessibilityFrame;
      if (coversScreen(f, screen) || f.size.width <= 0 || f.size.height <= 0) continue;
      [covered addObject:[NSValue valueWithRect:f]];
      [found addObject:node(el, 1)];
    }
  }
  return found;
}

static void snapshot(id requestId) {
  @autoreleasepool {
    g_requests = 0;
    g_nodes = 0;
    g_token = NSUUID.UUID.UUIDString; // a fresh token: nothing cached from the last read
    CFAbsoluteTime t0 = CFAbsoluteTimeGetCurrent();
    id translation = [g_translator frontmostApplicationWithDisplayId:0 bridgeDelegateToken:g_token];
    if (translation == nil) {
      emit(@{ @"t" : @"ax", @"id" : requestId, @"error" : @"no frontmost application" });
      return;
    }
    [(id<SDAXTranslation>)translation setBridgeDelegateToken:g_token];
    id root = [g_translator macPlatformElementFromTranslation:translation];
    if (root == nil) {
      emit(@{ @"t" : @"ax", @"id" : requestId, @"error" : @"no element for the frontmost application" });
      return;
    }
    NSDictionary *tree = node(root, 0);
    if (!actionable(tree)) {
      NSArray *extra = probe();
      if (extra.count > 0) {
        NSMutableDictionary *merged = [tree mutableCopy];
        merged[@"children"] = [(tree[@"children"] ?: @[]) arrayByAddingObjectsFromArray:extra];
        tree = merged;
      }
    }
    double ms = (CFAbsoluteTimeGetCurrent() - t0) * 1000.0;
    emit(@{ @"t" : @"ax", @"id" : requestId, @"ms" : @(round(ms * 10) / 10), @"requests" : @(g_requests),
            @"root" : tree });
  }
}

int main(int argc, char *argv[]) {
  @autoreleasepool {
    if (argc < 2) {
      fprintf(stderr, "usage: sdax <udid>\n");
      return 2;
    }
    g_device = findDevice([NSString stringWithUTF8String:argv[1]]);
    g_replyQueue = dispatch_queue_create("sdax.reply", DISPATCH_QUEUE_SERIAL);
    setUpTranslator();

    // Parent watch: stop if the app dies without closing our stdin.
    dispatch_source_t parentExit = dispatch_source_create(
        DISPATCH_SOURCE_TYPE_PROC, (uintptr_t)getppid(), DISPATCH_PROC_EXIT,
        dispatch_get_global_queue(QOS_CLASS_UTILITY, 0));
    dispatch_source_set_event_handler(parentExit, ^{ _exit(0); });
    dispatch_resume(parentExit);

    emit(@{ @"t" : @"ready" });

    // Requests are served one at a time, on this thread; replies land on g_replyQueue.
    char *line = NULL;
    size_t cap = 0;
    while (getline(&line, &cap, stdin) > 0) {
      NSData *data = [NSData dataWithBytes:line length:strlen(line)];
      NSDictionary *msg = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
      if (![msg isKindOfClass:NSDictionary.class]) continue;
      if ([msg[@"t"] isEqual:@"snapshot"]) snapshot(msg[@"id"] ?: @0);
    }
    free(line);
    return 0;
  }
}
