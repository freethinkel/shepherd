/**
 * sdhid — native touch/keyboard/button injector for simdriver (derived from
 * pip-mini's simhid).
 *
 * Talks to the simulator's own `dtuhidd` digitizer service directly, the way
 * Meta's FBSimulatorControl DTUHID transport does (MIT; HID/DTUHIDModels.swift,
 * HID/SimulatorDTUHIDConnection.swift, XPC/SimulatorXPCConnection.swift), so it
 * can send what idb's gRPC surface cannot:
 *
 *   - an EDGE FLAG on every contact sample — iOS reads its system gestures
 *     (home indicator, app switcher, Notification/Control Centre, back) from
 *     that flag, not from where the contact is;
 *   - a SECOND CONTACT, streamed live (pinch / rotate / two-finger pan).
 *
 * Chain:
 *   SimDeviceSet → SimDevice(udid) -lookup:"…hid.digitizer" → mach port
 *   xpc_endpoint_create_mach_port_4sim → xpc_connection_create_from_endpoint
 *   xpc_connection_enable_sim2host_4sim (without it the service sees the peer but no payload)
 *   barrier round-trip = liveness (dtuhidd may be throttled-but-vended)
 *
 * Wire (plain XPC dictionaries, as dtuhidd's Codable decode expects):
 *   { messageType, isBarrier, featureIdentifier, payload }
 *   IndigoDigitizerEvent      { pointOne{x,y}, pointTwo?{x,y}, eventType u64 (0 start,1 position,2 end),
 *                               edge u64 (0 none,1 top,2 left,3 bottom,4 right), target u64 }
 *   IndigoButtonEvent         { usagePage u64, usageCode u64, state u64 (1 down, 2 up) }
 *   IndigoKeyboardButtonEvent { usageCode u64, state u64 }
 * Points are normalized to 0…1 of the portrait screen.
 *
 * dtuhidd serves one client per service: a second connection invalidates the
 * first (the "dtuhidd connection … invalidated" error). sdhid therefore owns
 * ALL HID — touches, buttons, keys, text — but does not fight for the service:
 * when displaced it prints {"t":"displaced"} once, drops every command except a
 * gesture start (touch/touch2 down, button, key, text), and on the next gesture
 * start reconnects, prints {"t":"reclaimed"} and sends that command. A reclaim gets
 * a short liveness budget (3 × 1 s, 250 ms apart); if it fails, sdhid prints
 * {"t":"error"} and stays displaced, so the next gesture start tries again.
 *
 * When the device leaves Booted (shutdown, reboot) sdhid exits 5: its
 * connection would otherwise be dead for good (dtuhidd only reports it
 * "interrupted"), and the owner restarts it once the device is back.
 *
 * stdin, one JSON object per line (device points):
 *   {"t":"touch","p":"down|move|up","x":..,"y":..,"e":0-4}
 *   {"t":"touch2","p":"down|move|up","x1":..,"y1":..,"x2":..,"y2":..}
 *   {"t":"button","b":"HOME|LOCK|SIDE_BUTTON|SIRI|VOLUME_UP|VOLUME_DOWN","n":presses?,"d":holdSeconds?}
 *   {"t":"key","code":usage,"mods":[usage,...]?} press + release with modifiers held
 *   {"t":"text","s":"..."}             US layout, ASCII
 *   {"t":"_invalidate"}                TEST ONLY: cancel the connection and take the same
 *                                      path as XPC_ERROR_CONNECTION_INVALID (displaced)
 * stdout, one JSON object per line:
 *   {"t":"ready","w":pt,"h":pt} · {"t":"displaced"} · {"t":"reclaimed"} · {"t":"warn","detail":..}
 *   {"t":"error","detail":..}   then exit (see below), except after a failed reclaim
 *
 * Exit codes: 0 stdin EOF / parent gone · 1 dtuhidd never came up · 2 usage ·
 * 3 CoreSimulator / device not found / not booted · 4 no main screen size ·
 * 5 the device left Booted while running.
 *
 * Usage: sdhid <udid>
 */

#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <objc/message.h>
#import <xpc/xpc.h>
#import <dlfcn.h>
#import <signal.h>
#import <stdatomic.h>
#import <unistd.h>

#import "CoreSimulator/SimDeviceSet.h"
#import "CoreSimulator/SimDevice.h"
#import "CoreSimulator/SimDeviceType.h"

static NSString *const kDigitizerService = @"com.apple.coredevice.feature.remote.hid.digitizer";

// Liveness, from Meta's DTUHIDTiming: the first send demand-launches dtuhidd,
// and a daemon launchd has throttled still vends a port that swallows sends.
static const int64_t kLivenessTimeoutMs = 4000;
static const int kLivenessAttempts = 5;
static const useconds_t kLivenessBackoffUs = 4 * 1000 * 1000;
// Reclaiming after displacement: the user is waiting on a touch, and the daemon is
// already up, so retry fast and give up soon (the next gesture start tries again).
static const int64_t kReclaimTimeoutMs = 1000;
static const int kReclaimAttempts = 3;
static const useconds_t kReclaimBackoffUs = 250 * 1000;
static const int64_t kDeviceWatchMs = 500;
static const useconds_t kReplyTailUs = 200 * 1000;
static const useconds_t kButtonHoldUs = 60 * 1000;
static const useconds_t kButtonGapUs = 120 * 1000; // inside iOS's double-click window
static const useconds_t kKeyGapUs = 4 * 1000;

typedef xpc_object_t (*endpoint_from_port_t)(mach_port_t, uint64_t, uint64_t);
typedef xpc_connection_t (*connection_from_endpoint_t)(xpc_object_t);
typedef void (*enable_sim2host_t)(xpc_connection_t);
typedef id (*ctx_send_t)(Class, SEL, NSString *, id *);
typedef id (*set_send_t)(id, SEL, id *);

static SimDevice *g_device = nil;
static double g_widthPt = 0, g_heightPt = 0;
static xpc_connection_t g_conn = NULL;
static volatile BOOL g_invalid = YES;
// Set from the connection event handler (xpc queue) or the test-only `_invalidate`
// (input queue) when another client took the service over; cleared on the input queue
// when a gesture start reclaims it. Atomic so one invalidation reports exactly once.
static _Atomic bool g_displaced = false;
// Every send happens on this serial queue, so contact phases stay ordered and
// button holds / key gaps can simply sleep it.
static dispatch_queue_t g_inputQueue = nil;
static dispatch_queue_t g_xpcQueue = nil;
static dispatch_source_t g_deviceWatch = nil;
static BOOL g_touchActive = NO;
/// Which screen touches land on (--target): 0 is the main one; a foldable's panels have their own ids.
static uint64_t g_target = 0;
static BOOL g_touch2Active = NO;

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

static void warn(NSString *detail) { emit(@{ @"t" : @"warn", @"detail" : detail }); }

static void failWith(int code, NSString *detail) {
  emit(@{ @"t" : @"error", @"detail" : detail });
  _exit(code);
}

static void fail(NSString *detail) { failWith(3, detail); }

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

static SimDevice *findDevice(NSString *udid) {
  Class contextClass = NSClassFromString(@"SimServiceContext");
  if (contextClass == nil) fail(@"SimServiceContext unavailable (CoreSimulator not loaded)");
  id error = nil;
  id context = ((ctx_send_t)objc_msgSend)(contextClass, @selector(sharedServiceContextForDeveloperDir:error:),
                                          currentDeveloperDir(), &error);
  if (context == nil) fail([NSString stringWithFormat:@"SimServiceContext: %@", error]);
  SimDeviceSet *set = ((set_send_t)objc_msgSend)(context, @selector(defaultDeviceSetWithError:), &error);
  if (set == nil) fail([NSString stringWithFormat:@"device set: %@", error]);
  for (SimDevice *candidate in set.devices) {
    if ([candidate.UDID.UUIDString isEqualToString:udid]) return candidate;
  }
  fail([NSString stringWithFormat:@"device %@ not found", udid]);
  return nil;
}

/// Wait for CoreSimulator's bootstatus ("fully booted") before the first connect.
/// dtuhidd answers its liveness barrier while the guest is still coming up — right after
/// a shutdown+boot — so a connect then yields a Ready helper whose digitizer events iOS
/// silently drops (observed in the reboot IT). On a device that is already fully up this
/// returns immediately. Best effort: `xcrun` unavailable counts as done. No `-b`: that
/// would boot a device that shut down after our Booted check, and booting is the host's call.
/// No shell (argv only), and bounded: a bootstatus still waiting after
/// kBootstatusTimeoutSecs is killed and we connect anyway rather than hang `start`.
static const int64_t kBootstatusTimeoutSecs = 30;

static BOOL waitBootstatus(NSString *udid) {
  NSString *dev = currentDeveloperDir();
  if (dev.length == 0) return YES;
  NSTask *task = [NSTask new];
  task.executableURL = [NSURL fileURLWithPath:[dev stringByAppendingPathComponent:@"usr/bin/simctl"]];
  task.arguments = @[ @"bootstatus", udid ];
  task.standardInput = [NSFileHandle fileHandleWithNullDevice];
  task.standardOutput = [NSFileHandle fileHandleWithNullDevice];
  task.standardError = [NSFileHandle fileHandleWithNullDevice];
  dispatch_semaphore_t done = dispatch_semaphore_create(0);
  task.terminationHandler = ^(NSTask *t) { dispatch_semaphore_signal(done); };
  if (![task launchAndReturnError:nil]) return YES;
  if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, kBootstatusTimeoutSecs * NSEC_PER_SEC)) != 0) {
    [task terminate];
    return NO;
  }
  return task.terminationStatus == 0;
}

// ─── XPC connection to dtuhidd ───────────────────────────────────────────────

static xpc_object_t message(const char *type, xpc_object_t payload, bool barrier) {
  xpc_object_t msg = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_string(msg, "messageType", type);
  xpc_dictionary_set_bool(msg, "isBarrier", barrier);
  xpc_dictionary_set_string(msg, "featureIdentifier", kDigitizerService.UTF8String);
  xpc_dictionary_set_value(msg, "payload", payload);
  return msg;
}

static xpc_object_t keyboardPayload(uint64_t usage, uint64_t state) {
  xpc_object_t p = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_uint64(p, "usageCode", usage);
  xpc_dictionary_set_uint64(p, "state", state);
  return p;
}

/// Another client took the service over: step aside and report it once. Shared by the
/// XPC_ERROR_CONNECTION_INVALID handler and the test-only `_invalidate` command.
static void markDisplaced(void) {
  g_invalid = YES;
  if (!atomic_exchange(&g_displaced, true)) emit(@{ @"t" : @"displaced" });
}

static xpc_connection_t openConnection(NSString **why) {
  void *handle = dlopen(NULL, RTLD_NOW);
  endpoint_from_port_t endpointFromPort = (endpoint_from_port_t)dlsym(handle, "xpc_endpoint_create_mach_port_4sim");
  connection_from_endpoint_t connectionFromEndpoint =
      (connection_from_endpoint_t)dlsym(handle, "xpc_connection_create_from_endpoint");
  enable_sim2host_t enableSim2Host = (enable_sim2host_t)dlsym(handle, "xpc_connection_enable_sim2host_4sim");
  if (!endpointFromPort || !connectionFromEndpoint || !enableSim2Host) {
    *why = @"private _4sim XPC symbols unavailable";
    return NULL;
  }
  NSError *error = nil;
  mach_port_t port = [g_device lookup:kDigitizerService error:&error];
  if (port == MACH_PORT_NULL) {
    *why = [NSString stringWithFormat:@"lookup %@ failed: %@", kDigitizerService, error];
    return NULL;
  }
  xpc_object_t endpoint = endpointFromPort(port, 0, 0); // consumes the send right
  xpc_connection_t conn = endpoint ? connectionFromEndpoint(endpoint) : NULL;
  if (conn == NULL) {
    *why = @"could not create a connection from the digitizer endpoint";
    return NULL;
  }
  enableSim2Host(conn);
  xpc_connection_set_target_queue(conn, g_xpcQueue);
  xpc_connection_set_event_handler(conn, ^(xpc_object_t event) {
    if (event == XPC_ERROR_CONNECTION_INVALID) {
      // Another client took the service over: step aside, report it once.
      if (conn == g_conn) markDisplaced();
    } else if (event == XPC_ERROR_CONNECTION_INTERRUPTED) {
      warn(@"dtuhidd connection interrupted (XPC reconnects on the next send)");
    }
  });
  xpc_connection_resume(conn);
  return conn;
}

/// A keyboard usage 0 barrier — "no event" — answered only by a live daemon.
static BOOL confirmLiveness(xpc_connection_t conn, int64_t timeoutMs) {
  dispatch_semaphore_t done = dispatch_semaphore_create(0);
  __block BOOL ok = NO;
  xpc_connection_send_message_with_reply(conn, message("IndigoKeyboardButtonEvent", keyboardPayload(0, 2), true),
                                         g_xpcQueue, ^(xpc_object_t reply) {
    ok = xpc_get_type(reply) != XPC_TYPE_ERROR;
    dispatch_semaphore_signal(done);
  });
  if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, timeoutMs * NSEC_PER_MSEC)) != 0) {
    return NO;
  }
  if (ok) usleep(kReplyTailUs); // device-open tail, paid once so later sends are warm
  return ok;
}

/// (Re)connect with liveness retries. Runs on the input queue; blocks it. On failure
/// `*why` says what the last attempt hit.
static BOOL connectWith(int64_t timeoutMs, int attempts, useconds_t backoffUs, NSString **why) {
  *why = @"no attempt";
  for (int attempt = 1; attempt <= attempts; attempt++) {
    xpc_connection_t conn = openConnection(why);
    if (conn != NULL) {
      if (confirmLiveness(conn, timeoutMs)) {
        xpc_connection_t old = g_conn;
        g_conn = conn; // swap first so the old connection's INVALID is not mistaken for displacement
        if (old) xpc_connection_cancel(old);
        g_invalid = NO;
        g_touchActive = g_touch2Active = NO; // contacts did not survive the old connection
        return YES;
      }
      xpc_connection_cancel(conn);
      *why = @"dtuhidd did not answer the liveness probe";
    }
    if (attempt < attempts) usleep(backoffUs);
  }
  return NO;
}

/// Connect with the startup budget (dtuhidd may need demand-launching).
static BOOL connectDigitizer(void) {
  NSString *why = nil;
  if (connectWith(kLivenessTimeoutMs, kLivenessAttempts, kLivenessBackoffUs, &why)) return YES;
  warn([NSString stringWithFormat:@"digitizer unavailable after %d attempts: %@", kLivenessAttempts, why]);
  return NO;
}

static void send(xpc_object_t msg) {
  if (atomic_load(&g_displaced)) return; // displaced mid-command: step aside until the next gesture start
  if (g_invalid) {
    warn(@"dtuhidd connection was taken over or lost — reconnecting");
    if (!connectDigitizer()) return;
  }
  xpc_connection_send_message(g_conn, msg);
}

// ─── events ──────────────────────────────────────────────────────────────────

static xpc_object_t point(double xPt, double yPt) {
  xpc_object_t p = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_double(p, "x", fmin(1.0, fmax(0.0, xPt / g_widthPt)));
  xpc_dictionary_set_double(p, "y", fmin(1.0, fmax(0.0, yPt / g_heightPt)));
  return p;
}

/// down → start, move → position (start if no contact is down), up → end.
static uint64_t phase(NSString *p, BOOL *active) {
  if ([p isEqualToString:@"up"]) {
    *active = NO;
    return 2;
  }
  if ([p isEqualToString:@"down"] || !*active) {
    *active = YES;
    return 0;
  }
  return 1;
}

static void sendTouch(NSString *p, double x, double y, uint64_t edge) {
  if ([p isEqualToString:@"up"] && !g_touchActive) return; // lost with a reconnect
  xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_value(payload, "pointOne", point(x, y));
  xpc_dictionary_set_uint64(payload, "eventType", phase(p, &g_touchActive));
  xpc_dictionary_set_uint64(payload, "edge", edge);
  xpc_dictionary_set_uint64(payload, "target", g_target);
  send(message("IndigoDigitizerEvent", payload, false));
}

static void sendTouch2(NSString *p, double x1, double y1, double x2, double y2) {
  if ([p isEqualToString:@"up"] && !g_touch2Active) return;
  xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_value(payload, "pointOne", point(x1, y1));
  xpc_dictionary_set_value(payload, "pointTwo", point(x2, y2));
  xpc_dictionary_set_uint64(payload, "eventType", phase(p, &g_touch2Active));
  xpc_dictionary_set_uint64(payload, "edge", 0);
  xpc_dictionary_set_uint64(payload, "target", g_target);
  send(message("IndigoDigitizerEvent", payload, false));
}

static void sendButtonState(uint64_t page, uint64_t code, uint64_t state) {
  xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_uint64(payload, "usagePage", page);
  xpc_dictionary_set_uint64(payload, "usageCode", code);
  xpc_dictionary_set_uint64(payload, "state", state);
  send(message("IndigoButtonEvent", payload, false));
}

/// Consumer-page usages (FBSimulatorControl SimulatorHIDButtonIdentity).
static BOOL buttonUsage(NSString *name, uint64_t *code) {
  NSDictionary<NSString *, NSNumber *> *map = @{
    @"HOME" : @0x40, @"LOCK" : @0x30, @"SIDE_BUTTON" : @0x30, @"SIRI" : @0xCF,
    @"PLAY_PAUSE" : @0xCD, @"VOLUME_UP" : @0xE9, @"VOLUME_DOWN" : @0xEA,
  };
  NSNumber *n = map[name];
  if (n == nil) return NO;
  *code = n.unsignedLongLongValue;
  return YES;
}

static void pressButton(NSString *name, NSInteger presses, double holdSec) {
  uint64_t code = 0;
  if (!buttonUsage(name, &code)) {
    warn([NSString stringWithFormat:@"unknown button %@", name]);
    return;
  }
  useconds_t hold = holdSec > 0 ? (useconds_t)(holdSec * 1e6) : kButtonHoldUs;
  for (NSInteger i = 0; i < MAX(1, presses); i++) {
    if (i > 0) usleep(kButtonGapUs);
    sendButtonState(0x0C, code, 1);
    usleep(hold);
    sendButtonState(0x0C, code, 2);
  }
}

static void pressKey(uint64_t usage, BOOL shift) {
  if (shift) send(message("IndigoKeyboardButtonEvent", keyboardPayload(225, 1), false));
  send(message("IndigoKeyboardButtonEvent", keyboardPayload(usage, 1), false));
  usleep(kKeyGapUs);
  send(message("IndigoKeyboardButtonEvent", keyboardPayload(usage, 2), false));
  if (shift) send(message("IndigoKeyboardButtonEvent", keyboardPayload(225, 2), false));
  usleep(kKeyGapUs);
}

static void pressKeyWithModifiers(uint64_t usage, NSArray *modifiers) {
  for (NSNumber *modifier in modifiers) {
    send(message("IndigoKeyboardButtonEvent", keyboardPayload(modifier.unsignedLongLongValue, 1), false));
    usleep(kKeyGapUs);
  }
  send(message("IndigoKeyboardButtonEvent", keyboardPayload(usage, 1), false));
  usleep(kKeyGapUs);
  send(message("IndigoKeyboardButtonEvent", keyboardPayload(usage, 2), false));
  usleep(kKeyGapUs);
  for (NSNumber *modifier in modifiers.reverseObjectEnumerator) {
    send(message("IndigoKeyboardButtonEvent", keyboardPayload(modifier.unsignedLongLongValue, 2), false));
    usleep(kKeyGapUs);
  }
}

/// US-layout HID keyboard usage for an ASCII character (USB HID usage table 0x07).
static BOOL keyFor(unichar c, uint64_t *usage, BOOL *shift) {
  *shift = NO;
  if (c >= 'a' && c <= 'z') { *usage = 4 + (c - 'a'); return YES; }
  if (c >= 'A' && c <= 'Z') { *usage = 4 + (c - 'A'); *shift = YES; return YES; }
  if (c >= '1' && c <= '9') { *usage = 30 + (c - '1'); return YES; }
  if (c == '0') { *usage = 39; return YES; }
  static const char *plain = "\n\t -=[]\\;'`,./";
  static const uint64_t plainUsage[] = {40, 43, 44, 45, 46, 47, 48, 49, 51, 52, 53, 54, 55, 56};
  static const char *shifted = "!@#$%^&*()_+{}|:\"~<>?";
  static const uint64_t shiftedUsage[] = {30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 45, 46, 47, 48, 49, 51, 52, 53, 54, 55, 56};
  for (int i = 0; plain[i]; i++) if (c == (unichar)plain[i]) { *usage = plainUsage[i]; return YES; }
  for (int i = 0; shifted[i]; i++) if (c == (unichar)shifted[i]) { *usage = shiftedUsage[i]; *shift = YES; return YES; }
  return NO;
}

static void typeString(NSString *text) {
  for (NSUInteger i = 0; i < text.length; i++) {
    uint64_t usage = 0;
    BOOL shift = NO;
    if (keyFor([text characterAtIndex:i], &usage, &shift)) pressKey(usage, shift);
    else warn([NSString stringWithFormat:@"no US key for U+%04X", [text characterAtIndex:i]]);
  }
}

// ─── stdin ───────────────────────────────────────────────────────────────────

static double num(NSDictionary *cmd, NSString *key) {
  id v = cmd[key];
  return [v isKindOfClass:NSNumber.class] ? [v doubleValue] : 0;
}

static BOOL startsGesture(NSDictionary *cmd) {
  NSString *t = cmd[@"t"];
  if ([t isEqualToString:@"touch"] || [t isEqualToString:@"touch2"]) return [cmd[@"p"] isEqualToString:@"down"];
  return [t isEqualToString:@"button"] || [t isEqualToString:@"key"] || [t isEqualToString:@"text"];
}

static void handle(NSDictionary *cmd) {
  NSString *t = cmd[@"t"];
  if ([t isEqualToString:@"_invalidate"]) { // test-only: simulate being displaced
    if (g_conn) xpc_connection_cancel(g_conn);
    markDisplaced();
    return;
  }
  if (atomic_load(&g_displaced)) {
    if (!startsGesture(cmd)) return;
    NSString *why = nil;
    if (!connectWith(kReclaimTimeoutMs, kReclaimAttempts, kReclaimBackoffUs, &why)) {
      // Still displaced: the next gesture start tries again.
      emit(@{ @"t" : @"error",
              @"detail" : [NSString stringWithFormat:@"reclaim failed after %d attempts: %@", kReclaimAttempts, why] });
      return;
    }
    atomic_store(&g_displaced, false);
    emit(@{ @"t" : @"reclaimed" });
  }
  if ([t isEqualToString:@"touch"]) {
    sendTouch(cmd[@"p"], num(cmd, @"x"), num(cmd, @"y"), (uint64_t)num(cmd, @"e"));
  } else if ([t isEqualToString:@"touch2"]) {
    sendTouch2(cmd[@"p"], num(cmd, @"x1"), num(cmd, @"y1"), num(cmd, @"x2"), num(cmd, @"y2"));
  } else if ([t isEqualToString:@"button"]) {
    pressButton(cmd[@"b"], (NSInteger)num(cmd, @"n"), num(cmd, @"d"));
  } else if ([t isEqualToString:@"key"]) {
    NSArray *mods = [cmd[@"mods"] isKindOfClass:[NSArray class]] ? cmd[@"mods"] : @[];
    if (mods.count) pressKeyWithModifiers((uint64_t)num(cmd, @"code"), mods);
    else pressKey((uint64_t)num(cmd, @"code"), NO);
  } else if ([t isEqualToString:@"text"]) {
    typeString(cmd[@"s"] ?: @"");
  } else {
    warn([NSString stringWithFormat:@"unknown command %@", t]);
  }
}

static void readStdin(void) {
  char *line = NULL;
  size_t cap = 0;
  ssize_t n;
  while ((n = getline(&line, &cap, stdin)) > 0) {
    NSData *data = [NSData dataWithBytes:line length:(NSUInteger)n];
    NSDictionary *cmd = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if (![cmd isKindOfClass:NSDictionary.class]) {
      warn(@"bad command line");
      continue;
    }
    dispatch_async(g_inputQueue, ^{ @autoreleasepool { handle(cmd); } });
  }
  // Parent closed stdin: let queued sends go out, then leave.
  dispatch_sync(g_inputQueue, ^{
    if (g_touchActive) sendTouch(@"up", 0, 0, 0); // never leave a finger down
  });
  usleep(100 * 1000);
  _exit(0);
}

int main(int argc, char *argv[]) {
  @autoreleasepool {
    if (argc < 2) {
      fprintf(stderr, "usage: sdhid <udid>\n");
      return 2;
    }
    signal(SIGPIPE, SIG_IGN);
    for (int i = 2; i + 1 < argc; i += 2) {
      if (strcmp(argv[i], "--target") == 0) g_target = strtoull(argv[i + 1], NULL, 0);
    }
    g_xpcQueue = dispatch_queue_create("simdriver.sdhid.xpc", DISPATCH_QUEUE_SERIAL);
    g_inputQueue = dispatch_queue_create(
        "simdriver.sdhid.input",
        dispatch_queue_attr_make_with_qos_class(DISPATCH_QUEUE_SERIAL, QOS_CLASS_USER_INTERACTIVE, 0));

    g_device = findDevice([NSString stringWithUTF8String:argv[1]]);
    if (![g_device.stateString isEqualToString:@"Booted"]) {
      fail([NSString stringWithFormat:@"device is not booted (%@)", g_device.stateString]);
    }
    CGSize px = g_device.deviceType.mainScreenSize;
    double scale = g_device.deviceType.mainScreenScale ?: 1;
    g_widthPt = px.width / scale;
    g_heightPt = px.height / scale;
    if (g_widthPt <= 0 || g_heightPt <= 0) failWith(4, @"device reports no main screen size");

    // A shut-down device leaves our connection dead without a word (dtuhidd only reports
    // "interrupted"): leave, and let the owner restart us once it is back.
    g_deviceWatch = dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0,
                                           dispatch_get_global_queue(QOS_CLASS_UTILITY, 0));
    dispatch_source_set_timer(g_deviceWatch, dispatch_time(DISPATCH_TIME_NOW, 0), kDeviceWatchMs * NSEC_PER_MSEC,
                              50 * NSEC_PER_MSEC);
    dispatch_source_set_event_handler(g_deviceWatch, ^{
      NSString *state = g_device.stateString;
      if (![state isEqualToString:@"Booted"]) {
        failWith(5, [NSString stringWithFormat:@"device left Booted (state %@)", state]);
      }
    });
    dispatch_resume(g_deviceWatch);

    __block BOOL up = NO;
    if (!waitBootstatus([NSString stringWithUTF8String:argv[1]])) {
      warn(@"bootstatus did not finish cleanly; connecting anyway");
    }
    dispatch_sync(g_inputQueue, ^{ up = connectDigitizer(); });
    if (!up) failWith(1, @"dtuhidd digitizer service did not come up");
    emit(@{ @"t" : @"ready", @"w" : @(g_widthPt), @"h" : @(g_heightPt) });

    // Parent watch: stop if the app dies without closing our stdin.
    dispatch_source_t parentExit = dispatch_source_create(
        DISPATCH_SOURCE_TYPE_PROC, (uintptr_t)getppid(), DISPATCH_PROC_EXIT, dispatch_get_main_queue());
    dispatch_source_set_event_handler(parentExit, ^{ _exit(0); });
    dispatch_resume(parentExit);
    if (getppid() == 1) _exit(0);

    [NSThread detachNewThreadWithBlock:^{ readStdin(); }];
    dispatch_main();
  }
}
