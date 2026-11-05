#ifndef SIMDRIVER_H
#define SIMDRIVER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define Grid_DEFAULT_COLS 6

/**
 * The opaque handle: a started Driver plus the C callbacks it delivers to.
 */
typedef struct sd_driver sd_driver;

/**
 * One delivered frame. `data` points at `len` tightly packed BGRA bytes and is valid
 * only inside the frame callback.
 */
typedef struct sd_frame {
  uint32_t width;
  uint32_t height;
  uint32_t source_width;
  uint32_t source_height;
  uint64_t seq;
  const uint8_t *data;
  size_t len;
} sd_frame;

typedef void (*sd_frame_cb)(const struct sd_frame *frame, void *user);

typedef void (*sd_state_cb)(int32_t state, void *user);

struct sd_driver *sd_start(const char *udid,
                           uint32_t fps,
                           uint32_t scale,
                           const char *helper_dir,
                           sd_frame_cb frame_cb,
                           sd_state_cb state_cb,
                           void *user);

void sd_pointer(struct sd_driver *d, int32_t phase_code, double x, double y, uint32_t mask);

void sd_scroll(struct sd_driver *d, int32_t phase_code, double x, double y, double dx, double dy);

/**
 * Two fingers at (x1, y1) and (x2, y2), each 0..1 in the box; `phase_code` as for
 * `sd_pointer`. For rotation and off-centre pinches.
 */
void sd_touch2(struct sd_driver *d, int32_t phase_code, double x1, double y1, double x2, double y2);

/**
 * A tap at (x, y), each 0..1 in the box: a pointer down and up.
 */
void sd_tap(struct sd_driver *d, double x, double y);

/**
 * Tap twice at (x, y), each 0..1 in the box, with a short pause for iOS double-tap recognition.
 */
void sd_double_tap(struct sd_driver *d, double x, double y);

/**
 * Press at (x, y), hold for `hold_secs`, then lift — iOS's long press (context menus).
 */
void sd_long_press(struct sd_driver *d, double x, double y, double hold_secs);

/**
 * Drag from (x1, y1) to (x2, y2), each 0..1 in the box: down, hold `hold_secs`, move
 * over `move_secs`, lift.
 */
void sd_drag(struct sd_driver *d,
             double x1,
             double y1,
             double x2,
             double y2,
             double hold_secs,
             double move_secs);

/**
 * The frontmost app's accessibility tree as JSON (one node: `role`, `label`?, `value`?,
 * `id`?, `enabled`, `frame` [x,y,w,h] in screen points, `children`?). Blocks; the first
 * call takes 1–2 s. Returns NULL on failure (see `sd_last_error`); free the result
 * with `sd_string_free`.
 */
char *sd_accessibility_json(struct sd_driver *d);

/**
 * Frees a string returned by simdriver. NULL-safe.
 */
void sd_string_free(char *s);

/**
 * Rows of the grid with `cols` columns over a `screen_w`×`screen_h` point screen.
 */
uint32_t sd_grid_rows(double screen_w, double screen_h, uint32_t cols);

/**
 * Centre of grid cell `name` (e.g. "C4": row C, column 4; rows skip E) in 0..1 of the screen,
 * written to `*x` and `*y`. False (nothing written) for a name outside the grid or a
 * NULL argument.
 */
bool sd_grid_cell(double screen_w,
                  double screen_h,
                  uint32_t cols,
                  const char *name,
                  double *x,
                  double *y);

/**
 * The device screen in points, written to `*w` and `*h`. Returns false (and writes
 * nothing) until input is connected, or if `d`, `w` or `h` is NULL.
 */
bool sd_screen_points(struct sd_driver *d, double *w, double *h);

bool sd_key(struct sd_driver *d, const char *name);

/**
 * Press a key combination such as `cmd+a`. Modifiers are cmd, shift, alt, or ctrl.
 */
bool sd_key_combo(struct sd_driver *d, const char *combo);

bool sd_open_url(struct sd_driver *d, const char *url);

bool sd_launch(struct sd_driver *d, const char *bundle_id);

bool sd_terminate(struct sd_driver *d, const char *bundle_id);

bool sd_set_location(struct sd_driver *d, double lat, double lon);

/**
 * Appearance code: 0 = light, 1 = dark.
 */
bool sd_set_appearance(struct sd_driver *d, int32_t appearance);

bool sd_grant_permission(struct sd_driver *d, const char *service, const char *bundle_id);

bool sd_clipboard_write(struct sd_driver *d, const char *text);

char *sd_clipboard_read(struct sd_driver *d);

bool sd_revoke_permission(struct sd_driver *d, const char *service, const char *bundle_id);

bool sd_reset_permission(struct sd_driver *d, const char *service, const char *bundle_id);

bool sd_set_content_size(struct sd_driver *d, const char *size);

bool sd_status_bar_override(struct sd_driver *d);

bool sd_status_bar_clear(struct sd_driver *d);

bool sd_push_notification(struct sd_driver *d, const char *bundle_id, const char *payload_path);

bool sd_reset_keychain(struct sd_driver *d);

/**
 * Clear an app's saved data container. This explicitly deletes its data and terminates it.
 */
bool sd_clear_app_data(struct sd_driver *d, const char *bundle_id);

/**
 * Return installed simulator apps as JSON; free the result with `sd_string_free`.
 */
char *sd_apps(struct sd_driver *d);

void sd_text(struct sd_driver *d, const char *s);

void sd_button(struct sd_driver *d, int32_t button_code, uint32_t presses, float hold_secs);

/**
 * Stops the driver and frees the handle. NULL-safe; the handle must not be used after.
 */
void sd_stop(struct sd_driver *d);

/**
 * The last error on this thread: valid until the next `sd_` call on this thread.
 * Empty when the last call succeeded. Never NULL.
 */
const char *sd_last_error(void);

#endif  /* SIMDRIVER_H */
