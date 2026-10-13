// A real spreadsheet editor, rather than one we maintain.
//
// The hand-built grid this replaces had to own selection, keyboard handling, the
// edit lifecycle and — the part that never ends — a formula engine. Univer's
// open-source core (Apache 2.0) has all of it already.
//
// What we deliberately do NOT use is Univer's import/export, which is a Pro feature
// requiring their server (docs: "The import and export functionality requires
// support from the Univer server"). Sending the user's documents to a server is the
// opposite of what this app is. We don't need it: SheetJS already parses .xlsx in
// the browser, and `sheets::write_cells` already writes it in Rust. So the workbook
// is converted here, in memory, and never leaves the machine.
//
// Univer is large, so it is imported lazily — the module only loads when someone
// actually opens a spreadsheet.

export { columnLabel, externalChangeAction, toCsv } from "./UniverSheet/externalChangeAction";
export { UniverSheet } from "./UniverSheet/UniverSheet";
