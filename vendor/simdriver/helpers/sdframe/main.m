/**
 * sdframe — native iOS-simulator frame pump for simdriver (derived from
 * pip-mini's simframe).
 *
 * Chain (all inside one long-lived process):
 *
 *   SimDeviceSet -----> SimDevice.UDID match
 *   SimDevice.io -----> SimDeviceIOClient -ioPorts
 *   port.descriptor --> id<SimScreen> (new-style) else id<SimDisplayRenderable> (legacy)
 *   frameRendered/damage ------> IOSurface lock + packed (optionally downscaled) BGRA
 *   shared memory -------------> composed straight into ring slot n % 3 under a seqlock
 *   stdout --------------------> JSON lines only:
 *                                {"t":"shm","name":"/sdf-<pid>-<gen>","slots":3,"cap":N}
 *                                {"t":"frame","slot":S,"seq":Q}
 *
 * Segment layout (little-endian; contract with crates/simdriver/src/shm.rs):
 *   file header, 64 bytes: [magic u32 0x31464453][version u32 1][slots u32][pad u32][cap u64]
 *   slot i at 64 + i * (64 + cap):
 *     slot header, 64 bytes: [seq u64][width u32][height u32][source_width u32][source_height u32][bpp u32 = 4]
 *     pixels, cap bytes (cap = w*h*4 rounded up to 64)
 * Seqlock writer: seq = 2k+1 (relaxed), release fence, header + pixels, seq = 2k+2
 * (release); the frame line carries the even value. A new segment generation is
 * created whenever the output size changes; the previous one is unlinked 1 s later.
 *
 * srcW/srcH are the device framebuffer's own pixel dims, so the consumer can
 * map input to device points even when --scale shrinks the payload.
 *
 * Threading: every callback family AND the seed-watchdog poll run on ONE
 * serial queue (g_frameQueue). They share strong references (held surface,
 * current surface); touching those from two queues over-releases and
 * segfaults within a second at 60fps. stdout lines go out on a separate
 * writer queue so a slow reader never blocks the frame queue.
 *
 * Private framework protocols use Meta's vendored headers (PrivateHeaders/).
 * Not a TCC screen capture: the framebuffer is the device's own GPU surface.
 *
 * Usage: sdframe <udid> [--fps 60] [--scale 2] [--verbose]
 * Stops (unlinking its segments) on stdin EOF, SIGINT, SIGTERM or parent exit, and
 * when the device leaves Booted (its surfaces go dead silently across a reboot, so the
 * owner restarts sdframe once the device is back).
 * Exit codes: 0 ok/stop, 2 usage, 3 device not found/not booted, 4 no surface,
 * 5 the device left Booted while running.
 */

#import <Foundation/Foundation.h>
#import <Accelerate/Accelerate.h>
#import <CoreGraphics/CoreGraphics.h>
#import <IOSurface/IOSurface.h>
#import <IOSurface/IOSurfaceObjC.h> // the NSObject wrapper (toll-free with IOSurfaceRef)
#import <objc/message.h>
#import <sys/sysctl.h>
#import <errno.h>
#import <unistd.h>

// Meta's vendored private headers (compiled as plain includes; the dylibs are
// linked separately via -framework, so the real Apple frameworks are used).
#import "CoreSimulator/SimDeviceSet.h"
#import "CoreSimulator/SimDevice.h"
#import "CoreSimulator/SimDeviceIOClient.h"
#import "CoreSimDeviceIO/SimScreen-Protocol.h"
#import "CoreSimDeviceIO/SimScreenAdapter-Protocol.h"
#import "CoreSimDeviceIO/SimScreenProperties-Protocol.h"
#import "CoreSimDeviceIO/SimDisplayRenderable-Protocol.h"
#import "CoreSimDeviceIO/SimDisplayIOSurfaceRenderable-Protocol.h"
#import "CoreSimDeviceIO/SimDisplayDescriptorState-Protocol.h"
#import "CoreSimDeviceIO/SimDeviceIOPortInterface-Protocol.h"

#import <os/lock.h>
#import <stdatomic.h>
#import <sys/mman.h>
#import <fcntl.h>

static const uint32_t SHM_MAGIC = 0x31464453;   // bytes "S","D","F","1" read as LE u32
static const uint32_t SHM_VERSION = 1;
static const uint32_t SHM_SLOTS = 3;
static const size_t SHM_FILE_HEADER = 64;
static const size_t SHM_SLOT_HEADER = 64;

// The one serial queue all surface state lives on (see header comment).
static dispatch_queue_t g_frameQueue = nil;
static BOOL g_verbose = NO;     // per-frame notes on stderr (off: they cost a write per frame)
static NSUInteger g_scale = 1;  // integer downscale factor for the emitted payload
// --rotate-cw: frames turned a quarter clockwise (a foldable's panel mounted sideways), via a scratch copy.
static BOOL g_rotate = NO;
static uint8_t *g_rotateTemp = NULL;
static size_t g_rotateTempSize = 0;
static BOOL g_stopping = NO;    // set on the frame queue by requestStop

// Current shm segment (frame queue only).
static uint8_t *g_shmBase = NULL;
static size_t g_shmLen = 0;
static uint64_t g_shmCap = 0;
static NSUInteger g_shmWidth = 0;
static NSUInteger g_shmHeight = 0;
static unsigned g_shmGen = 0;
static NSString *g_shmName = nil;
static uint64_t g_framesWritten = 0; // across generations; frame n gets seq 2n

// Line mailbox for the stdout writer (see handleFrameRendered). Everything
// below is guarded by g_mailboxLock: the writer runs on its own queue.
static os_unfair_lock g_mailboxLock = OS_UNFAIR_LOCK_INIT;
static NSMutableArray<NSString *> *g_pendingShmLines = nil; // never dropped, written first
static BOOL g_framePending = NO;     // latest frame line wins
static uint32_t g_pendingSlot = 0;
static uint64_t g_pendingSeq = 0;
static BOOL g_writing = NO;          // writer loop in flight
static dispatch_queue_t g_writerQueue = nil;
// Segment names this process still has linked; requestStop unlinks them all.
static NSMutableArray<NSString *> *g_linkedShmNames = nil;
// Compose-time surface identity attached to the emit note.
static IOSurfaceID g_noteId = 0;
static NSUInteger g_noteSeed = 0;
static NSString *g_noteSource = @""; // which lane produced the frame
static NSString *g_currentEmitSource = @""; // lane currently triggering emission (frame queue only)
// vImage scratch for the downscale, sized once per geometry (frame queue only).
static void *g_scaleTemp = NULL;
static size_t g_scaleTempSize = 0;

#if defined(__ARM_NEON)
#import <arm_neon.h>
#endif

// Exact 2x reduction as a 2x2 box average — the right filter for an integer
// halving and ~10x cheaper than vImageScale's Lanczos, which capped --scale 2
// at ~44fps (measured; --scale 1's plain memcpy ran 56). Odd trailing
// row/column is dropped, matching outWidth = width / 2.
static void downscale2xBGRA(const uint8_t *src, size_t srcStride, uint8_t *dst,
                            size_t outWidth, size_t outHeight) {
  for (size_t y = 0; y < outHeight; ++y) {
    const uint8_t *r0 = src + (2 * y) * srcStride;
    const uint8_t *r1 = r0 + srcStride;
    uint8_t *out = dst + y * outWidth * 4;
    size_t x = 0;
#if defined(__ARM_NEON)
    // 8 source pixels per row → 4 output pixels per iteration.
    for (; x + 4 <= outWidth; x += 4) {
      uint32x4x2_t a = vld2q_u32((const uint32_t *)(r0 + x * 8));
      uint32x4x2_t b = vld2q_u32((const uint32_t *)(r1 + x * 8));
      uint8x16_t top = vrhaddq_u8(vreinterpretq_u8_u32(a.val[0]), vreinterpretq_u8_u32(a.val[1]));
      uint8x16_t bot = vrhaddq_u8(vreinterpretq_u8_u32(b.val[0]), vreinterpretq_u8_u32(b.val[1]));
      vst1q_u8(out + x * 4, vrhaddq_u8(top, bot));
    }
#endif
    for (; x < outWidth; ++x) {
      const uint8_t *p = r0 + x * 8, *q = r1 + x * 8;
      for (int c = 0; c < 4; ++c) {
        out[x * 4 + c] = (uint8_t)((p[c] + p[c + 4] + q[c] + q[c + 4] + 2) >> 2);
      }
    }
  }
}

static void emitNote(NSString *line) {
  fprintf(stderr, "[sdframe] %s\n", line.UTF8String);
  fflush(stderr);
}

static void requestStop(int signalNumber);

// ─── stdout line writer ─────────────────────────────────────────────────────
// Single-consumer mailbox. The frame queue may NEVER block on the consumer:
// the render server that delivers these callbacks blocks in turn, so write(2)
// lives on the writer queue. While a write is in flight, newer frame lines
// overwrite the pending one (latest wins; the slot it names would be lapped
// soon anyway). shm lines are never dropped and always go out before the
// pending frame line, which is cleared whenever a new segment is announced.

static BOOL writeAllStdout(const char *bytes, size_t left) {
  // Raw write(2), not stdio: a writer blocked in fwrite holds stdout's FILE
  // lock, and exit()'s flush then deadlocks on it.
  while (left > 0) {
    ssize_t n = write(STDOUT_FILENO, bytes, left);
    if (n < 0 && errno == EINTR) continue;
    if (n <= 0) return NO;
    bytes += n;
    left -= (size_t)n;
  }
  return YES;
}

static void writerLoop(void) {
  for (;;) {
    os_unfair_lock_lock(&g_mailboxLock);
    NSArray<NSString *> *shmLines = g_pendingShmLines.count > 0 ? [g_pendingShmLines copy] : nil;
    [g_pendingShmLines removeAllObjects];
    BOOL haveFrame = g_framePending;
    uint32_t slot = g_pendingSlot;
    uint64_t seq = g_pendingSeq;
    g_framePending = NO;
    if (shmLines == nil && !haveFrame) {
      g_writing = NO;
      os_unfair_lock_unlock(&g_mailboxLock);
      return;
    }
    IOSurfaceID noteId = g_noteId;
    NSUInteger noteSeed = g_noteSeed;
    NSString *noteSource = g_noteSource;
    os_unfair_lock_unlock(&g_mailboxLock);

    NSMutableString *out = [NSMutableString string];
    for (NSString *line in shmLines) [out appendString:line];
    if (haveFrame) {
      [out appendFormat:@"{\"t\":\"frame\",\"slot\":%u,\"seq\":%llu}\n", slot, (unsigned long long)seq];
    }
    const char *bytes = out.UTF8String;
    if (!writeAllStdout(bytes, strlen(bytes))) {
      requestStop(SIGPIPE); // reader gone (EPIPE): nobody to stream to
      return;
    }
    if (haveFrame && g_verbose) {
      emitNote([NSString stringWithFormat:@"emit seq=%llu slot=%u (%@ id=%u seed=%llu)",
                (unsigned long long)seq, slot, noteSource, noteId,
                (unsigned long long)noteSeed]);
    }
  }
}

static void kickWriterLocked(void) {
  if (g_writing) return;
  g_writing = YES;
  dispatch_async(g_writerQueue, ^{ writerLoop(); });
}

// ─── shared-memory segments ─────────────────────────────────────────────────

static void unlinkSegmentNamed(NSString *name) {
  os_unfair_lock_lock(&g_mailboxLock);
  BOOL linked = [g_linkedShmNames containsObject:name];
  [g_linkedShmNames removeObject:name];
  os_unfair_lock_unlock(&g_mailboxLock);
  if (linked) shm_unlink(name.UTF8String);
}

static void unlinkAllSegments(void) {
  os_unfair_lock_lock(&g_mailboxLock);
  NSArray<NSString *> *names = [g_linkedShmNames copy];
  [g_linkedShmNames removeAllObjects];
  os_unfair_lock_unlock(&g_mailboxLock);
  for (NSString *name in names) shm_unlink(name.UTF8String);
}

// Frame queue only. A failing shm call would otherwise note on every frame.
static void noteShmFailure(NSString *what) {
  static double lastAt = 0.0;
  double now = CFAbsoluteTimeGetCurrent();
  if (now - lastAt < 5.0) return;
  lastAt = now;
  emitNote(what);
}

// Frame queue only. Makes sure the current segment fits (outWidth, outHeight);
// on a size change creates generation n+1, announces it, and retires the old one.
static BOOL ensureSegment(NSUInteger outWidth, NSUInteger outHeight) {
  if (g_shmBase != NULL && outWidth == g_shmWidth && outHeight == g_shmHeight) return YES;
  // cap rounded up to 64 so every slot (and its seq word) stays 64-byte aligned.
  uint64_t cap = ((uint64_t)outWidth * outHeight * 4 + 63) & ~(uint64_t)63;
  size_t len = SHM_FILE_HEADER + SHM_SLOTS * (SHM_SLOT_HEADER + (size_t)cap);
  unsigned gen = ++g_shmGen;
  NSString *name = [NSString stringWithFormat:@"/sdf-%d-%u", getpid(), gen];
  int fd = shm_open(name.UTF8String, O_RDWR | O_CREAT | O_EXCL, 0600);
  if (fd < 0 && errno == EEXIST) {
    // Left behind by a SIGKILLed process that had our pid.
    shm_unlink(name.UTF8String);
    fd = shm_open(name.UTF8String, O_RDWR | O_CREAT | O_EXCL, 0600);
  }
  if (fd < 0) {
    noteShmFailure([NSString stringWithFormat:@"shm_open %@ failed: %s", name, strerror(errno)]);
    return NO;
  }
  os_unfair_lock_lock(&g_mailboxLock);
  [g_linkedShmNames addObject:name];
  os_unfair_lock_unlock(&g_mailboxLock);
  if (ftruncate(fd, (off_t)len) != 0) {
    noteShmFailure([NSString stringWithFormat:@"ftruncate %@ (%zu bytes) failed: %s", name, len, strerror(errno)]);
    close(fd);
    unlinkSegmentNamed(name);
    return NO;
  }
  void *p = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
  close(fd);
  if (p == MAP_FAILED) {
    noteShmFailure([NSString stringWithFormat:@"mmap %@ failed: %s", name, strerror(errno)]);
    unlinkSegmentNamed(name);
    return NO;
  }
  // ftruncate zero-fills: every slot starts at seq 0, which no frame line names.
  uint8_t *base = (uint8_t *)p;
  uint32_t fileHeader[4] = { SHM_MAGIC, SHM_VERSION, SHM_SLOTS, 0 };
  memcpy(base, fileHeader, sizeof(fileHeader));
  memcpy(base + 16, &cap, sizeof(cap));

  // Retire the previous generation: the writer never touches it again, but a
  // reader may still be opening it off an in-flight announcement — keep the
  // name linked for 1 s.
  if (g_shmBase != NULL) {
    munmap(g_shmBase, g_shmLen);
    NSString *old = g_shmName;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)NSEC_PER_SEC),
                   dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{ unlinkSegmentNamed(old); });
  }
  g_shmBase = base;
  g_shmLen = len;
  g_shmCap = cap;
  g_shmWidth = outWidth;
  g_shmHeight = outHeight;
  g_shmName = name;
  emitNote([NSString stringWithFormat:@"shm %@: %lux%lu, %u slots, cap %llu",
            name, (unsigned long)outWidth, (unsigned long)outHeight, SHM_SLOTS,
            (unsigned long long)cap]);

  os_unfair_lock_lock(&g_mailboxLock);
  [g_pendingShmLines addObject:[NSString stringWithFormat:@"{\"t\":\"shm\",\"name\":\"%@\",\"slots\":%u,\"cap\":%llu}\n",
                                name, SHM_SLOTS, (unsigned long long)cap]];
  g_framePending = NO; // a pending line names the old generation's slot
  kickWriterLocked();
  os_unfair_lock_unlock(&g_mailboxLock);
  return YES;
}

@interface FramePumper : NSObject

@property (nonatomic, strong) NSObject<SimDeviceIOPortInterface> *portInterface;
@property (nonatomic, strong) NSObject *descriptor;           // SimScreen + SimDisplayIOSurfaceRenderable conformance resolved at runtime (callback target)
@property (nonatomic, strong) NSObject *portDescriptor;       // IO-port DISPLAY descriptor (class 0) — seed-watchdog poll source
@property (nonatomic, strong) NSUUID *token;                   // registration token for clean unregister
@property (nonatomic, strong) id currentSurfaceId;             // IOSurface * as id, bridged to IOSurfaceRef per call
@property (nonatomic, assign) NSUInteger surfaceStride;        // surface bytes per row
@property (nonatomic, assign) double lastEmit;
@property (nonatomic, assign) double minInterval;              // seconds between frames at --fps
@property (nonatomic, assign) NSUInteger lastEmittedSeed;      // content identity of the last frame sent
@property (nonatomic, assign) IOSurfaceID lastEmittedId;
@property (atomic, assign) BOOL registrationFailed;

- (void)handleSurfacesChanged:(id)primary masked:(id)masked;
- (void)handleFrameRendered;

@end

@implementation FramePumper

- (instancetype)init {
  if ((self = [super init])) {
    self.token = [NSUUID UUID];
    self.minInterval = 1.0 / 30.0;
    self.lastEmit = 0.0;
  }
  return self;
}

- (void)handleSurfacesChanged:(id)primary masked:(id)masked {
  if (primary == nil) {
    emitNote(@"surfacesChanged delivered nil primary surface");
    return;
  }
  // Tolerance check the way Meta does it: the ROCK proxy is untyped, so confirm
  // the incoming object is really an IOSurface before adopting it.
  IOSurfaceRef candidate = (__bridge IOSurfaceRef)primary;
  if (candidate == NULL) {
    emitNote(@"surfacesChange value did not bridge to an IOSurfaceRef");
    return;
  }
  self.currentSurfaceId = primary;
}

- (void)handleFrameRendered {
  if (g_stopping) return;
  // Strong local for the whole copy: the bridged ref below borrows from it.
  id surfaceObject = self.currentSurfaceId;
  if (surfaceObject == nil) return;
  IOSurfaceRef surface = (__bridge IOSurfaceRef)surfaceObject;
  if (surface == NULL) return;

  // Every lane (callbacks + watchdog) funnels here, so skip content already
  // sent — a duplicate copy would also eat the rate-limit window of the next
  // real present.
  IOSurfaceID surfaceId = IOSurfaceGetID(surface);
  NSUInteger seed = IOSurfaceGetSeed(surface);
  if (seed == self.lastEmittedSeed && surfaceId == self.lastEmittedId) return;

  // Rate limit: emit at most --fps frames/second. We always work on the LATEST
  // surface state, and the watchdog retries an unsent seed next tick, so
  // back-pressure costs nothing durable. 0.8× tolerance: presents arrive with
  // jitter around the display period, so a strict compare at --fps 60 drops
  // every frame that lands 0.1ms early.
  double now = CFAbsoluteTimeGetCurrent();
  if (now - self.lastEmit < self.minInterval * 0.8) return;
  self.lastEmit = now;
  self.lastEmittedSeed = seed;
  self.lastEmittedId = surfaceId;

  NSUInteger width = IOSurfaceGetWidth(surface);
  NSUInteger height = IOSurfaceGetHeight(surface);
  NSUInteger stride = IOSurfaceGetBytesPerRow(surface);
  if (width == 0 || height == 0 || stride == 0) return;

  uint32_t pixfmt = (uint32_t)IOSurfaceGetPixelFormat(surface);
  if (pixfmt != 'BGRA' && pixfmt != 'ARGB') {
    static BOOL warnedPrefixMismatch = NO;
    if (!warnedPrefixMismatch) {
      warnedPrefixMismatch = YES;
      emitNote([NSString stringWithFormat:@"unexpected pixel format %.4s — colors may need a swizzle",
                (const char *)&pixfmt]);
    }
  }

  // Compose straight into the next ring slot (no intermediate scratch — one
  // pass per frame, zero allocations once the segment exists). With --scale N
  // the pass is a downscale instead of a memcpy: a 3x iPhone frame is
  // 12.6 MB, ~4x more than a 310pt pane can show.
  NSUInteger outWidth = g_scale > 1 ? width / g_scale : width;
  NSUInteger outHeight = g_scale > 1 ? height / g_scale : height;
  if (outWidth == 0 || outHeight == 0) return;
  NSUInteger packed = outWidth * 4 * outHeight;
  if (g_rotate && g_rotateTempSize < packed) {
    free(g_rotateTemp);
    g_rotateTemp = malloc(packed);
    g_rotateTempSize = g_rotateTemp ? packed : 0;
    if (g_rotateTemp == NULL) return;
  }
  if (!ensureSegment(g_rotate ? outHeight : outWidth, g_rotate ? outWidth : outHeight)) return;
  uint64_t frameIndex = g_framesWritten++;
  uint64_t seq = 2 * (frameIndex + 1); // even = complete; unique per frame
  uint32_t slot = (uint32_t)(frameIndex % SHM_SLOTS);
  uint8_t *slotBase = g_shmBase + SHM_FILE_HEADER + slot * (SHM_SLOT_HEADER + (size_t)g_shmCap);
  _Atomic uint64_t *seqWord = (_Atomic uint64_t *)slotBase;
  // Seqlock begin: odd seq, then a release fence so no header/pixel store
  // below can become visible before the slot reads as "being written".
  atomic_store_explicit(seqWord, seq - 1, memory_order_relaxed);
  atomic_thread_fence(memory_order_release);
  uint32_t slotHeader[5] = {
    (uint32_t)(g_rotate ? outHeight : outWidth), (uint32_t)(g_rotate ? outWidth : outHeight),
    (uint32_t)(g_rotate ? height : width), (uint32_t)(g_rotate ? width : height), 4,
  };
  memcpy(slotBase + 8, slotHeader, sizeof(slotHeader));

  IOSurfaceLock(surface, kIOSurfaceLockReadOnly, NULL);
  uint8_t *srcStart = (uint8_t *)IOSurfaceGetBaseAddress(surface);
  uint8_t *dstStart = g_rotate ? g_rotateTemp : slotBase + SHM_SLOT_HEADER;
  if (g_scale == 2) {
    downscale2xBGRA(srcStart, stride, dstStart, outWidth, outHeight);
  } else if (outWidth != width) {
    vImage_Buffer src = { srcStart, height, width, stride };
    vImage_Buffer dst = { dstStart, outHeight, outWidth, outWidth * 4 };
    vImage_Error need = vImageScale_ARGB8888(&src, &dst, NULL, kvImageGetTempBufferSize);
    if (need > 0 && (size_t)need > g_scaleTempSize) {
      free(g_scaleTemp);
      g_scaleTemp = malloc((size_t)need);
      g_scaleTempSize = g_scaleTemp ? (size_t)need : 0;
    }
    vImageScale_ARGB8888(&src, &dst, g_scaleTemp, kvImageNoFlags);
  } else if (stride == width * 4) {
    memcpy(dstStart, srcStart, packed);
  } else {
    uint8_t *src = srcStart;
    uint8_t *dst = dstStart;
    for (NSUInteger row = 0; row < height; ++row) {
      memcpy(dst, src, width * 4);
      src += stride;
      dst += width * 4;
    }
  }
  IOSurfaceUnlock(surface, kIOSurfaceLockReadOnly, NULL);
  if (g_rotate) {
    vImage_Buffer upright = { g_rotateTemp, outHeight, outWidth, outWidth * 4 };
    vImage_Buffer turned = { slotBase + SHM_SLOT_HEADER, outWidth, outHeight, outHeight * 4 };
    Pixel_8888 black = { 0, 0, 0, 255 };
    vImageRotate90_ARGB8888(&upright, &turned, kRotate90DegreesClockwise, black, kvImageNoFlags);
  }
  // Seqlock end: the even value publishes header + pixels.
  atomic_store_explicit(seqWord, seq, memory_order_release);

  // Hand the notification to the writer queue (see writerLoop), with the
  // compose-time surface identity for the verbose emit note.
  os_unfair_lock_lock(&g_mailboxLock);
  g_noteId = surfaceId;
  g_noteSeed = seed;
  g_noteSource = g_currentEmitSource ?: @"bind";
  g_pendingSlot = slot;
  g_pendingSeq = seq;
  g_framePending = YES;
  kickWriterLocked();
  os_unfair_lock_unlock(&g_mailboxLock);
}

- (void)shutdownUserWarning {
  // no-op: logging helper kept for symmetry
}

@end

typedef enum {
  SDFRAME_ALL,
  SDFRAME_NEW_ONLY,
  SDFRAME_LEGACY_ONLY,
} sdframeRegisterMode;

static sdframeRegisterMode g_registerMode = SDFRAME_ALL;
static NSUUID *g_legacyToken = nil; // set in registerCallbacks (legacy families' token)
static double g_rereregInterval = 0.0; // --rerereg seconds; 0 = register once
static double g_lastRegisterAt = 0.0;  // CFAbsoluteTime of last registration

static FramePumper *g_pumper = nil;

typedef id (*ctx_send_t)(Class, SEL, NSString *, id *);
typedef id (*set_send_t)(id, SEL, id *);

/// Resolve the active developer directory the same way FBSimulatorControl's
/// XcodeConfiguration does: DEVELOPER_DIR env first, then `xcode-select -p`.
static NSString *currentDeveloperDir(void) {
  NSString *fromEnv = [NSProcessInfo.processInfo.environment objectForKey:@"DEVELOPER_DIR"];
  if (fromEnv != nil && fromEnv.length > 0) return fromEnv;
  FILE *pipe = popen("xcode-select -p 2>/dev/null", "r");
  if (pipe == NULL) return @"";
  char line[PATH_MAX];
  NSString *result = @"";
  if (fgets(line, sizeof(line), pipe) != NULL) {
    result = [NSString stringWithUTF8String:line];
    while ([result hasSuffix:@"\n"] || [result hasSuffix:@"\r"] || [result hasSuffix:@" "]) {
      result = [result substringToIndex:result.length - 1];
    }
  }
  pclose(pipe);
  return result;
}

// Owned-surface adoption used by both the poll watchdog and the callback lane.
static void adoptHeldSurface(id object, NSString *tag);

// Run on g_frameQueue — directly when already there. The legacy callback
// families take no queue argument and ROCKit delivers them on its own thread,
// so without this hop they race the watchdog over the same surface.
static void *g_frameQueueKey = &g_frameQueueKey;
static void onFrameQueue(void (^work)(void)) {
  if (dispatch_get_specific(g_frameQueueKey) != NULL) work();
  else dispatch_async(g_frameQueue, work);
}

static void guardedProxyCall(NSString *what, void (^action)(void)) {  @try {
    action();
  } @catch (NSException *ex) {
    emitNote([NSString stringWithFormat:@"%@ raised: %@", what, ex.reason ?: ex.name]);
  }
}

static void registerCallbacks(id screen) {
  // Rotate tokens every cycle: registering the SAME UUID twice is a no-op on
  // this CoreSimulator build (delivery woke only under fresh tokens), and a
  // re-register under a stale token risks the last-write-wipes-earlier bug.
  g_pumper.token = [NSUUID UUID];
  g_legacyToken = [NSUUID UUID];
  g_lastRegisterAt = CFAbsoluteTimeGetCurrent();

  // Meta parity: a plain serial queue at userInteractive qos. All callbacks
  // deliver on it — and it is the same queue the seed-watchdog polls on.
  dispatch_queue_t queue = g_frameQueue;
  // New-style callbacks deliver ON `queue` (per SimScreen-Protocol.h), so
  // onFrameQueue runs them inline; it only hops when a build ignores `queue`.
  void (^frameTick)(void) = ^{
    onFrameQueue(^{
      g_currentEmitSource = @"cb";
      [g_pumper handleFrameRendered];
    });
  };
  void (^surfacesChanged)(id, id) = ^(id primary, id masked) {
    onFrameQueue(^{
      if (primary != nil && [primary isKindOfClass:[NSObject class]]) {
        adoptHeldSurface(primary, @"surfcb");
      }
      g_currentEmitSource = @"surfcb";
      [g_pumper handleSurfacesChanged:primary masked:masked];
      [g_pumper handleFrameRendered];
    });
  };

  __block NSString *identity = nil;
  guardedProxyCall(@"screenProperties", ^{
    id props = [screen screenProperties];
    id uid = props ? [props uniqueId] : nil;
    if ([uid isKindOfClass:[NSString class]]) identity = uid;
  });

  // Registering multiple callback families under ONE token has shown the last
  // one to wipe the earlier ones (frames stopped entirely). One family per
  // token: new-style first with its OWN token; legacy with a separate token.
  BOOL registeredAny = NO;
  g_legacyToken = [NSUUID UUID];

  if (g_registerMode != SDFRAME_LEGACY_ONLY && [screen conformsToProtocol:@protocol(SimScreen)]) {
    guardedProxyCall(@"SimScreen register", ^{
      [(NSObject<SimScreen> *)screen registerScreenCallbacksWithUUID:g_pumper.token
                                                      callbackQueue:queue
                                                      frameCallback:frameTick
                                          surfacesChangedCallback:surfacesChanged
                                        propertiesChangedCallback:^(id props){}];
    });
    registeredAny = YES;
  }

  // Legacy damageRectangles + surface-change. On this CoreSimulator the render
  // server delivers damage callbacks per presented frame (a whole-frame
  // compositor invokes it with an empty rect array — see Meta's comment).
  if (g_registerMode != SDFRAME_NEW_ONLY) {
    SEL damage = NSSelectorFromString(@"registerCallbackWithUUID:damageRectanglesCallback:");
    if ([screen respondsToSelector:damage]) {
      guardedProxyCall(@"damageRectangles register", ^{
        typedef void (*fb_send_t)(id, SEL, NSUUID *, void (^)(NSArray<NSValue *> *));
        ((fb_send_t)objc_msgSend)(screen, damage, g_legacyToken, ^(NSArray<NSValue *> *rects) {
          (void)rects;
          onFrameQueue(^{
            g_currentEmitSource = @"damage";
            [g_pumper handleFrameRendered];
          });
        });
      });
      registeredAny = YES;
    }
    SEL plural = NSSelectorFromString(@"registerCallbackWithUUID:ioSurfacesChangeCallback:");
    SEL singular = NSSelectorFromString(@"registerCallbackWithUUID:ioSurfaceChangeCallback:");
    void (^surfaceIncoming)(id) = ^(id surface) {
      onFrameQueue(^{
        adoptHeldSurface(surface, @"legacy");
        g_currentEmitSource = @"legacy";
        [g_pumper handleSurfacesChanged:surface masked:nil];
        [g_pumper handleFrameRendered];
      });
    };
    SEL which = [screen respondsToSelector:plural] ? plural
             : [screen respondsToSelector:singular] ? singular : nil;
    if (which != nil) {
      guardedProxyCall(@"ioSurfaceChange register", ^{
        typedef void (*fb_send_t)(id, SEL, NSUUID *, void (^)(id));
        ((fb_send_t)objc_msgSend)(screen, which, g_legacyToken, surfaceIncoming);
      });
      registeredAny = YES;
    }
  }

  if (!registeredAny) {
    emitNote(@"no framebuffer callback could be registered");
    exit(4);
  }
  emitNote([NSString stringWithFormat:@"callbacks registered (mode %s token-new %@ screen uid %@)",
            g_registerMode == SDFRAME_NEW_ONLY ? "new" : g_registerMode == SDFRAME_LEGACY_ONLY ? "legacy" : "all",
            g_pumper.token.UUIDString, identity ?: @"(none)"]);
}

static void unregisterCallbacks(void) {
  if (g_pumper == nil || g_pumper.descriptor == nil) return;
  id screen = g_pumper.descriptor;
  if (g_registerMode != SDFRAME_LEGACY_ONLY && [screen conformsToProtocol:@protocol(SimScreen)]) {
    guardedProxyCall(@"SimScreen unregister", ^{
      [(NSObject<SimScreen> *)screen unregisterScreenCallbacksWithUUID:g_pumper.token];
    });
  }
  if (g_registerMode == SDFRAME_NEW_ONLY) {
    return;
  }
  // Legacy families were registered under legacyToken — unregister the same.
  // The legacy unregister selectors take the token, not the family, so run
  // them all; an unknown family is a no-op raise guarded away.
  NSArray<NSString *> *unregisterNames = @[
    @"unregisterDamageRectanglesCallbackWithUUID:",
    @"unregisterIOSurfacesChangeCallbackWithUUID:",
    @"unregisterIOSurfaceChangeCallbackWithUUID:",
  ];
  for (NSString *name in unregisterNames) {
    SEL sel = NSSelectorFromString(name);
    if ([screen respondsToSelector:sel]) {
      guardedProxyCall([NSString stringWithFormat:@"unregister %@", name], ^{
        typedef void (*fb_send_t)(id, SEL, NSUUID *);
        ((fb_send_t)objc_msgSend)(screen, sel, g_legacyToken);
      });
    }
  }
}

// ─── seed-watchdog poll (hybrid mode) ───────────────────────────────────────
// Callback delivery proved unreliable across Xcode 27 CoreSimulator builds
// (one burst at registration, then silence). The renderable vends its CURRENT
// surface synchronously via -framebufferSurface, and IOSurfaceGetSeed changes
// on every content update, so polling closes the gap: the pump keeps working
// even when callbacks stop. Emission still honors the --fps rate limiter.

static dispatch_queue_t g_pollQueue = nil;
static dispatch_source_t g_pollSource = nil;
static dispatch_source_t g_deviceWatch = nil;
// Per-source last-seen (id, seed): "screen" = enumerated display object,
// "desc" = io-port display descriptor. Either source changing means presents
// happened; emit from the source that moved.
static NSUInteger g_lastSeedScreen = 0;
static NSUInteger g_lastSeedDesc = 0;
static IOSurfaceID g_lastIdScreen = 0;
static IOSurfaceID g_lastIdDesc = 0;
// The surface OBJECT we hold and poll seeds on. Fresh-attach surfaces live
// (seed advances per present); after a display page-flip or connection
// staleness the held object can go dead — the stale-rebind below re-probes.
static id g_heldSurfaceObject = nil;
static NSUInteger g_lastSeedHeld = 0;
static IOSurfaceID g_lastIdHeld = 0;
static double g_lastHeldChangeAt = 0.0; // CFAbsoluteTime of last held change
static double g_lastRebindProbeAt = 0.0;

static id probeFramebufferSurface(id target) {
  if (target == nil) return nil;
  __block id surfaceObject = nil;
  guardedProxyCall(@"poll framebufferSurface", ^{
    surfaceObject = [(id<SimDisplayIOSurfaceRenderable>)target framebufferSurface];
  });
  return surfaceObject;
}

// Take ownership of a surface for the held fast path (strong ref through the
// wrapper id; the ref itself is rebridged per read).
static void adoptHeldSurface(id object, NSString *tag) {
  if (object == nil) return;
  IOSurfaceRef newRef = (__bridge IOSurfaceRef)object;
  if (newRef == NULL) return;
  BOOL alreadyHeld =
      g_heldSurfaceObject != nil
      && (__bridge IOSurfaceRef)g_heldSurfaceObject == newRef;
  g_heldSurfaceObject = object;
  g_lastSeedHeld = IOSurfaceGetSeed(newRef);
  g_lastIdHeld = IOSurfaceGetID(newRef);
  g_lastHeldChangeAt = CFAbsoluteTimeGetCurrent();
  if (!alreadyHeld) {
    emitNote([NSString stringWithFormat:@"held adopted (%@): id=%u seed=%llu", tag,
              g_lastIdHeld, (unsigned long long)g_lastSeedHeld]);
  }
}

static void emitFromSource(id surfaceObject, NSString *tag) {
  g_currentEmitSource = tag;
  g_pumper.currentSurfaceId = surfaceObject;
  [g_pumper handleFrameRendered];
}

static void pollTick(void) {
  if (g_pumper == nil) return;

  // --rerereg N: every N seconds, drop all callback tokens and re-register
  // fresh. Diagnostic lever for "delivery follows registration" (the
  // parallel-with-mjpeg run lit up right after a second consumer registered,
  // while solo runs go silent after the first burst).
  if (g_rereregInterval > 0.0 && CFAbsoluteTimeGetCurrent() - g_lastRegisterAt >= g_rereregInterval) {
    emitNote(@"re-register cycle: start");
    unregisterCallbacks();
    emitNote(@"re-register cycle: unregistered");
    registerCallbacks(g_pumper.descriptor);
    emitNote(@"re-registered callbacks");
  }

  // A. Held-surface fast path: no proxy round trips, just cheap seed reads
  // on the IOSurface object we already own.
  double now = CFAbsoluteTimeGetCurrent();
  if (g_heldSurfaceObject != nil) {
    IOSurfaceRef surface = (__bridge IOSurfaceRef)g_heldSurfaceObject;
    if (surface != NULL) {
      NSUInteger seed = IOSurfaceGetSeed(surface);
      IOSurfaceID surfaceId = IOSurfaceGetID(surface);
      if (surfaceId == g_lastIdHeld) {
        if (seed != g_lastSeedHeld) {
          g_lastSeedHeld = seed;
          g_lastHeldChangeAt = now;
          if (g_verbose) {
            emitNote([NSString stringWithFormat:@"poll held: id=%u seed=%llu",
                      surfaceId, (unsigned long long)seed]);
          }
        }
        // Offer every tick: handleFrameRendered skips seeds already sent, and
        // one the rate limiter refused goes out on the next tick instead of
        // waiting for another present.
        emitFromSource(g_heldSurfaceObject, @"held");
        // Live surface: no proxy probes needed. Each probe vends a fresh
        // IOSurface object over XPC — at 120 ticks/s that churn was the
        // backdrop for the unlock crash.
        if (now - g_lastHeldChangeAt <= 0.55) return;
      }
      // Id moved or the surface went quiet: fall through to the rebind scan.
    }
  } else {
    // Acquire once: the descriptor's surface is the canonical display one.
    id candidate = probeFramebufferSurface(g_pumper.portDescriptor)
        ?: probeFramebufferSurface(g_pumper.descriptor);
    if (candidate != nil) adoptHeldSurface(candidate, @"attach");
  }

  // A2. Stale rebind: if the held surface has been silent a while, the
  // display may have page-flipped to a NEW IOSurface (app launch, rotation).
  // Re-probe the property sources and re-hold on identity change.
  if (g_heldSurfaceObject != nil
      && now - g_lastHeldChangeAt > 0.55
      && now - g_lastRebindProbeAt > 0.55) {
    g_lastRebindProbeAt = now;
    id fresh = probeFramebufferSurface(g_pumper.portDescriptor)
        ?: probeFramebufferSurface(g_pumper.descriptor);
    if (fresh != nil) {
      IOSurfaceRef freshRef = (__bridge IOSurfaceRef)fresh;
      IOSurfaceID freshId = freshRef != NULL ? IOSurfaceGetID(freshRef) : 0;
      IOSurfaceRef heldRef = (__bridge IOSurfaceRef)g_heldSurfaceObject;
      IOSurfaceID heldId = heldRef != NULL ? IOSurfaceGetID(heldRef) : 0;
      if (freshId != 0 && freshId != heldId) {
        adoptHeldSurface(fresh, @"rebind");
        emitFromSource(fresh, @"rebind");
        return;
      }
    }
  }

  // B. Property re-fetch on both sources — only while nothing is held; once a
  // surface is held, the throttled rebind above covers swaps/id changes.
  if (g_heldSurfaceObject != nil) return;
  struct Source {
    NSString *tag;
    id target;
    NSUInteger *lastSeed;
    IOSurfaceID *lastId;
  };
  struct Source sources[2] = {
    { @"screen", g_pumper.descriptor, &g_lastSeedScreen, &g_lastIdScreen },
    { @"desc", g_pumper.portDescriptor, &g_lastSeedDesc, &g_lastIdDesc },
  };

  for (NSUInteger i = 0; i < 2; ++i) {
    struct Source *source = &sources[i];
    id surfaceObject = probeFramebufferSurface(source->target);
    if (surfaceObject == nil) continue;
    IOSurfaceRef surface = (__bridge IOSurfaceRef)surfaceObject;
    if (surface == NULL) continue;
    NSUInteger seed = IOSurfaceGetSeed(surface);
    IOSurfaceID surfaceId = IOSurfaceGetID(surface);
    if (seed == *source->lastSeed && surfaceId == *source->lastId) continue;
    *source->lastSeed = seed;
    *source->lastId = surfaceId;
    if (g_verbose) {
      emitNote([NSString stringWithFormat:@"poll %@: id=%u seed=%llu", source->tag,
                surfaceId, (unsigned long long)seed]);
    }
    if ([source->tag isEqualToString:@"desc"]) adoptHeldSurface(surfaceObject, @"probedesc");
    emitFromSource(surfaceObject, source->tag);
    return; // one emit per tick
  }
}

static void stopWith(NSString *why, int code) {
  @autoreleasepool {
    // Hop onto the frame queue so teardown never races a poll tick or callback.
    dispatch_sync(g_frameQueue, ^{
      g_stopping = YES;
      if (g_pollSource != nil) {
        dispatch_source_cancel(g_pollSource);
        g_pollSource = nil;
      }
      unregisterCallbacks();
      unlinkAllSegments();
    });
    emitNote(why);
    // _exit: skip stdio teardown — the writer thread may still be parked in a
    // write to a pipe nobody drains, and nothing buffered is worth flushing.
    _exit(code);
  }
}

static void requestStop(int signalNumber) {
  stopWith(signalNumber == 0 ? @"stopping on stdin EOF"
                             : [NSString stringWithFormat:@"stopping on signal %d", signalNumber],
           0);
}

int main(int argc, char * argv[]) {
  @autoreleasepool {
    NSArray<NSString *> *args = [[NSProcessInfo processInfo] arguments];
    if (args.count < 2) {
      fprintf(stderr, "usage: sdframe <udid> [--fps N] [--scale N] [--verbose]\n");
      return 2;
    }
    NSString *udid = args[1];
    double maxFps = 30.0;
    // --screen-size WxH: stream the screen of that pixel size (either way round) instead of the largest: a foldable's
    // other panel (iPhone Duo's cover is 1398x2034 beside its 2007x2853 inner one)
    CGSize wantSize = CGSizeZero;
    for (NSUInteger i = 2; i < args.count; ++i) {
      NSString *flag = args[i];
      if ([flag isEqualToString:@"--verbose"]) {
        g_verbose = YES;
        continue;
      }
      if (i + 1 >= args.count) break;
      NSString *value = args[++i];
      if ([flag isEqualToString:@"--fps"]) {
        maxFps = value.doubleValue;
      } else if ([flag isEqualToString:@"--scale"]) {
        NSInteger s = value.integerValue;
        g_scale = s > 1 ? (NSUInteger)s : 1;
      } else if ([flag isEqualToString:@"--mode"]) {
        if ([value isEqualToString:@"new"]) g_registerMode = SDFRAME_NEW_ONLY;
        else if ([value isEqualToString:@"legacy"]) g_registerMode = SDFRAME_LEGACY_ONLY;
        else g_registerMode = SDFRAME_ALL;
      } else if ([flag isEqualToString:@"--rotate-cw"]) {
        g_rotate = [value isEqualToString:@"1"];
      } else if ([flag isEqualToString:@"--screen-size"]) {
        NSArray<NSString *> *wh = [value componentsSeparatedByString:@"x"];
        if (wh.count == 2) wantSize = CGSizeMake(wh[0].doubleValue, wh[1].doubleValue);
      } else if ([flag isEqualToString:@"--rerereg"]) {
        g_rereregInterval = value.doubleValue;
      }
    }
    if (maxFps <= 0) maxFps = 30.0;

    // CoreSimulator device-set entry (mirrors FBSimulatorControl's
    // SimulatorServiceContext): +sharedServiceContextForDeveloperDir:error:
    // then -defaultDeviceSetWithError:. There is no +defaultSet factory.
    Class contextClass = NSClassFromString(@"SimServiceContext");
    if (contextClass == nil) {
      emitNote(@"SimServiceContext class unavailable (CoreSimulator not loaded)");
      return 3;
    }
    NSString *developerDir = currentDeveloperDir();
    id contextError = nil;
    id context = ((ctx_send_t)objc_msgSend)(contextClass, @selector(sharedServiceContextForDeveloperDir:error:),
                                            developerDir, &contextError);
    if (context == nil) {
      emitNote([NSString stringWithFormat:@"SimServiceContext unavailable for %@: %@", developerDir, contextError]);
      return 3;
    }
    id setError = nil;
    SimDeviceSet *set = ((set_send_t)objc_msgSend)(context, @selector(defaultDeviceSetWithError:), &setError);
    if (set == nil) {
      emitNote([NSString stringWithFormat:@"default device set unavailable: %@", setError]);
      return 3;
    }

    SimDevice *device = nil;
    for (SimDevice *candidate in set.devices) {
      if ([candidate.UDID.UUIDString isEqualToString:udid]) {
        device = candidate;
        break;
      }
    }
    if (device == nil) {
      emitNote([NSString stringWithFormat:@"device %@ not found", udid]);
      return 3;
    }
    // Trust CoreSimulator's own state string (numeric enum differs across
    // Xcode versions: 1155 says 2, 1174 says 3, so compare text instead).
    NSString *bootState = [device.stateString copy];
    if ([bootState isEqualToString:@"Booted"] == NO) {
      emitNote([NSString stringWithFormat:@"device %@ is not booted (state %@)", udid, bootState]);
      return 3;
    }

    SimDeviceIOClient *io = (SimDeviceIOClient *)device.io;
    NSArray *ports = [io ioPorts];
    if (ports.count == 0) {
      emitNote(@"no IO ports for device");
      return 4;
    }

    g_pumper = [FramePumper new];
    g_pumper.minInterval = 1.0 / maxFps;

    g_frameQueue = dispatch_queue_create(
        "simdriver.sdframe.frames",
        dispatch_queue_attr_make_with_qos_class(DISPATCH_QUEUE_SERIAL, QOS_CLASS_USER_INTERACTIVE, 0));
    dispatch_queue_set_specific(g_frameQueue, g_frameQueueKey, g_frameQueueKey, NULL);
    g_writerQueue = dispatch_queue_create("simdriver.sdframe.writer", DISPATCH_QUEUE_SERIAL);
    g_pendingShmLines = [NSMutableArray array];
    g_linkedShmNames = [NSMutableArray array];
    dispatch_set_target_queue(g_writerQueue, dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0));

    // Meta's discovery flow (FramebufferSurfaceLocator.surface): the io port
    // descriptor doubles as a SimScreenAdapter; enumerate screens over every
    // port and pick the main display — on iOS sims that is the screen with a
    // (string) uniqueId. Registering callbacks on the descriptor itself only
    // delivered one burst, so the enumerated screen object is the correct
    // callback target.
    NSMutableArray *screens = [NSMutableArray array];
    NSObject *chosenPortDescriptor = nil;
    for (id<SimDeviceIOPortInterface> portInterface in ports) {
      id descriptor = [portInterface descriptor];

      // Probe state/displayClass on EVERY port, BEFORE the adapter filter —
      // the live display descriptor may live on a port without SimScreenAdapter
      // conformance. Meta's Framebuffer reads -displayClass off this state to
      // find the main display; resolve it defensively (plain proxy calls raise
      // unrecognized-selector, so respondsToSelector first).
      __block unsigned short displayClass = 0;
      __block BOOL stateResolved = NO;
      guardedProxyCall(@"descriptor state", ^{
        SEL stateSel = NSSelectorFromString(@"state");
        if ([descriptor respondsToSelector:stateSel]) {
          id stateValue = ((id (*)(id, SEL))objc_msgSend)(descriptor, stateSel);
          if (stateValue != nil && [stateValue conformsToProtocol:@protocol(SimDisplayDescriptorState)]) {
            displayClass = [(id<SimDisplayDescriptorState>)stateValue displayClass];
            stateResolved = YES;
          }
        }
      });
      if (chosenPortDescriptor == nil && stateResolved && displayClass == 0) {
        chosenPortDescriptor = (NSObject *)descriptor;
        emitNote([NSString stringWithFormat:@"poll source: port %@ displayClass 0", portInterface.uuid.UUIDString]);
      }

      if (![descriptor conformsToProtocol:@protocol(SimScreenAdapter)]) continue;

      dispatch_semaphore_t sema = dispatch_semaphore_create(0);
      __block NSArray *reported = nil;
      dispatch_queue_t enumQueue = dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0);
      guardedProxyCall(@"enumerateScreens", ^{
        [(id<SimScreenAdapter>)descriptor
            enumerateScreensWithCompletionQueue:enumQueue
                              completionHandler:^(NSArray *list) {
          reported = list;
          dispatch_semaphore_signal(sema);
        }];
      });
      if (dispatch_semaphore_wait(sema, dispatch_time(DISPATCH_TIME_NOW, 4LL * NSEC_PER_SEC)) != 0) {
        emitNote(@"screen enumeration timed out on one port");
        continue;
      }
      if (reported.count > 0) [screens addObjectsFromArray:reported];
    }

    // Dedupe ports that shadow each other, then take the screen with the
    // largest displaySize — every sampled report has shown the main display at
    // native resolution (1206×2622 for an iPhone 3x) while auxiliary screens
    // report zero. uniqueId ordering means nothing: the first aux screen won
    // and registered callbacks on a surface that never lit up.
    NSMutableArray *deduped = [NSMutableArray array];
    NSMutableSet *seenIds = [NSMutableSet set];
    for (id candidate in screens) {
      __block NSString *uid = nil;
      guardedProxyCall(@"probe uniqueId", ^{
        id props = [candidate screenProperties];
        id u = props ? [props uniqueId] : nil;
        if ([u isKindOfClass:[NSString class]]) uid = u;
      });
      if (uid != nil) {
        if ([seenIds containsObject:uid]) continue;
        [seenIds addObject:uid];
      }
      [deduped addObject:candidate];
    }
    emitNote([NSString stringWithFormat:@"enumerated %lu distinct screen(s)", (unsigned long)deduped.count]);

    NSObject *screen = nil;
    __block NSUInteger bestArea = 0;
    for (id candidate in deduped) {
      __block CGSize size = CGSizeZero;
      id<SimDisplayRenderable> probe = nil;
      if ([candidate conformsToProtocol:@protocol(SimDisplayRenderable)]) {
        probe = (id<SimDisplayRenderable>)candidate;
      }
      if (probe != nil) {
        guardedProxyCall(@"probe displaySize", ^{
          size = probe.displaySize;
        });
      }
      NSUInteger area = (NSUInteger)(size.width * size.height);
      BOOL wanted = wantSize.width > 0 &&
          ((size.width == wantSize.width && size.height == wantSize.height) ||
           (size.width == wantSize.height && size.height == wantSize.width));
      if (wanted) {
        bestArea = area;
        screen = (NSObject *)candidate;
        chosenPortDescriptor = nil; // the main display's port would watch the wrong panel
        break;
      }
      if (area > bestArea) {
        bestArea = area;
        screen = (NSObject *)candidate;
      }
    }
    if (screen == nil) {
      // Fallback for proxies that never surface a displaySize (tvOS TVOut):
      // any usable screen still beats failing outright.
      screen = (NSObject *)deduped.firstObject;
    }
    emitNote([NSString stringWithFormat:@"picked screen area=%lu", (unsigned long)bestArea]);
    if (screen == nil) {
      emitNote(@"no SimScreenAdapter screens found");
      return 4;
    }

    g_pumper.descriptor = screen;
    // Seed-watchdog source: the live DISPLAY descriptor, not the flagged screen.
    g_pumper.portDescriptor = chosenPortDescriptor ?: screen;
    emitNote([NSString stringWithFormat:@"poll source resolved (%@)",
              chosenPortDescriptor != nil ? @"io-port display descriptor" : @"enumerated screen fallback"]);
    dispatch_sync(g_frameQueue, ^{ registerCallbacks((id)screen); });

    // Seed-watchdog poll at 2× the fps cap: catches presents that callbacks
    // miss (and works standalone when callback delivery is silent). With fps
    // 60 that is 120 polls/s — one cheap seed compare each on the held path.
    // Leeway must stay well under a display period: the old 50ms leeway let
    // ticks coalesce and capped delivery far below the requested rate.
    g_pollQueue = g_frameQueue;
    g_pollSource = dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, DISPATCH_TIMER_STRICT, g_pollQueue);
    dispatch_source_set_timer(g_pollSource,
                              dispatch_time(DISPATCH_TIME_NOW, 0),
                              (uint64_t)(1e9 / (2.0 * maxFps)),
                              NSEC_PER_MSEC);
    dispatch_source_set_event_handler(g_pollSource, ^{ pollTick(); });
    dispatch_resume(g_pollSource);

    dispatch_source_t sigTerm = dispatch_source_create(DISPATCH_SOURCE_TYPE_SIGNAL, SIGTERM, 0, dispatch_get_main_queue());
    dispatch_source_set_event_handler(sigTerm, ^{ requestStop(SIGTERM); });
    dispatch_resume(sigTerm);
    signal(SIGTERM, SIG_IGN); // let the dispatch source handle it
    dispatch_source_t sigInt = dispatch_source_create(DISPATCH_SOURCE_TYPE_SIGNAL, SIGINT, 0, dispatch_get_main_queue());
    dispatch_source_set_event_handler(sigInt, ^{ requestStop(SIGINT); });
    dispatch_resume(sigInt);
    signal(SIGINT, SIG_IGN);
    signal(SIGPIPE, SIG_IGN); // a closed reader surfaces as EPIPE in the writer

    // stdin EOF = the owner closed our pipe (or died): stop. Input is ignored.
    dispatch_source_t stdinSource = dispatch_source_create(
        DISPATCH_SOURCE_TYPE_READ, STDIN_FILENO, 0, dispatch_get_main_queue());
    dispatch_source_set_event_handler(stdinSource, ^{
      char sink[256];
      ssize_t n = read(STDIN_FILENO, sink, sizeof(sink));
      if (n == 0 || (n < 0 && errno != EINTR && errno != EAGAIN)) requestStop(0);
    });
    dispatch_resume(stdinSource);

    // Parent watch: if the app dies without signalling us (crash, hot-reload
    // churn, SIGKILL), stop instead of pumping frames into nobody.
    dispatch_source_t parentExit = dispatch_source_create(
        DISPATCH_SOURCE_TYPE_PROC, (uintptr_t)getppid(), DISPATCH_PROC_EXIT, dispatch_get_main_queue());
    dispatch_source_set_event_handler(parentExit, ^{ requestStop(SIGHUP); });
    dispatch_resume(parentExit);
    if (getppid() == 1) requestStop(SIGHUP); // already orphaned before the watch armed

    // Device watch: after a shutdown the held surfaces never fire again, so leave.
    g_deviceWatch = dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0,
                                           dispatch_get_global_queue(QOS_CLASS_UTILITY, 0));
    dispatch_source_set_timer(g_deviceWatch, dispatch_time(DISPATCH_TIME_NOW, 0), 500 * NSEC_PER_MSEC,
                              50 * NSEC_PER_MSEC);
    dispatch_source_set_event_handler(g_deviceWatch, ^{
      NSString *state = device.stateString;
      if (![state isEqualToString:@"Booted"]) {
        stopWith([NSString stringWithFormat:@"device left Booted (state %@)", state], 5);
      }
    });
    dispatch_resume(g_deviceWatch);

    emitNote(@"running; Ctrl/C SIGTERM to stop");
    dispatch_main();
    return 0;
  }
}
