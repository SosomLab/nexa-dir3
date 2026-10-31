/* ndir_xfer.h — C ABI of the Nexa Dir transfer engine (libndir_xfer.so / libndir_xfer.dylib / ndir_xfer.dll).
 *
 * Every string that crosses this boundary is UTF-8 JSON:
 *   - job      : JobSpec JSON   (docs/26 §4 — sources, dest, op, conflict, filter, verify, mirror, dry_run, journal, resume, ...)
 *                "journal": a file that gets one Report-item JSON line per finished top-level item; "resume": a previous
 *                Report JSON or journal whose "done" (source, dest) pairs are skipped (same ABI — keys only).
 *   - progress : one Progress JSON line per callback (docs/26 §5 — {"phase":..,"done":..,"total":..,"item":..,"items":..,"current":..})
 *   - result   : Report JSON    (docs/26 §6 — {"ok":bool,"canceled":bool,"dry_run":bool,"items":[..],"errors":[..],...})
 * Errors never come back as NULL: a failed call still returns a Report with "ok":false and "errors":[...].
 * Strings returned by ndir_xfer_run / ndir_xfer_validate must be released with ndir_xfer_free.
 * Release builds are compiled with panic=abort: an internal engine panic terminates the calling process instead of
 * returning a Report (input errors never panic — they come back as Report errors).
 * Relative paths in the job resolve against the process working directory; prefer absolute paths.
 */
#ifndef NDIR_XFER_H
#define NDIR_XFER_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version of this header; compare with ndir_xfer_abi_version() before use. */
#define NDIR_XFER_ABI_VERSION 1u

/* Progress callback. `line` is a NUL-terminated Progress JSON line valid only for the duration of the call
 * (copy it if you keep it). `user` is passed through unchanged. May be invoked from the calling thread or an
 * engine worker thread while ndir_xfer_run is in progress — keep it short and do your own synchronization. */
typedef void (*ndir_xfer_progress_cb)(const char *line, void *user);

/* ABI version implemented by the library (NDIR_XFER_ABI_VERSION of the build it was made from). */
uint32_t ndir_xfer_abi_version(void);

/* Engine version string, e.g. "0.28.0". Static — do not free. */
const char *ndir_xfer_version(void);

/* Run a job. `cb` and `cancel` may be NULL. `cancel` points to a bool the caller may set to true from any
 * thread to request cooperative cancellation; it must stay valid until the call returns.
 * Returns Report JSON (release with ndir_xfer_free). Blocks until the job finishes or is canceled. */
char *ndir_xfer_run(const char *job, ndir_xfer_progress_cb cb, void *user, const bool *cancel);

/* Parse and pre-validate a job (JSON shape + filesystem existence) without running it.
 * Returns Report JSON: "ok":true when the job would be accepted, otherwise "errors" explains why. */
char *ndir_xfer_validate(const char *job);

/* Release a string returned by ndir_xfer_run or ndir_xfer_validate. NULL is accepted. */
void ndir_xfer_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* NDIR_XFER_H */
