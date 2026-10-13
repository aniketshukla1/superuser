import { invoke, invokeStrict } from "./DictationStatus";

// --- Auto-update (desktop only; browser builds see null / throw) -----------------

export type UpdateInfo = { version: string; notes: string };

/** What a check found. `available` is the only thing that should raise a banner;
 *  `reason` says why there is nothing when there is nothing — "you are up to date"
 *  and "this build cannot check" used to be the same null. */
export type UpdateStatus = {
  available: boolean;
  version: string | null;
  notes: string | null;
  reason: string | null;
};

/** Ask the shell whether a newer release exists (verified manifest; see lib.rs). */
export const checkForUpdate = () => invoke<UpdateStatus | null>("check_for_update");

/** Pre-fetch + verify the update bytes in the background so the install is instant.
 * The shell caches them keyed by version; calling again for the same version is a no-op.
 * Rejects when there is no update or the download fails — callers fall back to the
 * download-on-install path. */
export const downloadUpdate = () => invokeStrict<void>("download_update");

/** Drop the pre-fetched update bundle (freed on "Later" so a dismissed release
 * doesn't pin tens of MB for a weeks-long app run). */
export const clearPendingUpdate = () => invokeStrict<void>("clear_pending_update");

/** Install the update (pre-fetched bytes when available, else download + verify now),
 * then relaunch. Resolves only on failure paths (success restarts the process on macOS;
 * Windows hands off to the installer). */
export const installUpdate = () => invokeStrict<void>("install_update");
