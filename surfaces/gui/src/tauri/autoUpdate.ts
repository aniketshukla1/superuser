import { invoke, invokeStrict } from "./DictationStatus";

// Updating the desktop app. In a browser the checks return null and the rest throw.

export type UpdateInfo = { version: string; notes: string };

/** What a check found. Only `available` should bring up a banner; `reason` explains an empty
 *  answer, because "you're up to date" and "this build can't check" used to be the same null. */
export type UpdateStatus = {
  available: boolean;
  version: string | null;
  notes: string | null;
  reason: string | null;
};

/** Ask the shell whether a newer release exists (from a verified manifest; see lib.rs). */
export const checkForUpdate = () => invoke<UpdateStatus | null>("check_for_update");

/** Fetch and verify the update in the background so installing is instant. The shell keeps it
 *  by version, so asking again for the same one does nothing. It fails when there is no update
 *  or the download fails, and callers then download at install time instead. */
export const downloadUpdate = () => invokeStrict<void>("download_update");

/** Drop the fetched update (on "Later"), so a release the user put off doesn't hold tens of MB
 *  for weeks of uptime. */
export const clearPendingUpdate = () => invokeStrict<void>("clear_pending_update");

/** Install the update, from the fetched copy if there is one or else downloading and verifying
 *  it now, then relaunch. It only resolves when something failed: on success macOS restarts the
 *  process and Windows hands over to the installer. */
export const installUpdate = () => invokeStrict<void>("install_update");
