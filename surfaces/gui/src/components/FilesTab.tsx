// The Files tab — the session's folder as a tree, with a preview beside it.
//
// The old version rendered a flat list: every artifact at the same level, each row
// opening a new document tab. That loses the shape of the work. A run that builds a
// spreadsheet also writes the script that built it, and the screenshots this is
// modelled on show `build_subscription_tracker.py` sitting next to the `.xlsx` it
// produced — the script is part of the deliverable, not scaffolding to hide.
//
// So: directories are directories, one click previews in place, and opening a full
// tab is still there for when you want to keep a file around while you work.

export { FilesTab } from "./FilesTab/FilesTab";
