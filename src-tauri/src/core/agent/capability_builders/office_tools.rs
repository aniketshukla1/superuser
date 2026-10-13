use super::super::*;

/// Word documents and slide decks. Part of `files` for the same reason the
/// spreadsheet tools are: the permission is writing inside the workspace, and a
/// assistant created tomorrow should not silently be unable to write a brief.
pub(in crate::core::agent) fn office_tools() -> Vec<ToolSpec> {
    vec![
        tool(
            "doc_write",
            "Write a Word document (.docx) from a title and paragraphs. Replaces the file if it exists.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "title": {"type": "string"},
                    "paragraphs": {"type": "array", "items": {"type": "string"}}
                }),
                &["path", "paragraphs"],
            ),
        ),
        // Reading and editing an existing deck, as distinct from replacing it.
        // `slides_write` builds one from titles and bullets; used on a real
        // presentation that would throw away the master, theme, images and layout.
        // These two let a deck be changed without being rebuilt.
        // Reading and editing an existing document, as distinct from replacing it.
        // `doc_write` builds a .docx from a title and paragraphs; used on a real Word
        // file that would flatten its headings, tables, images and styles. These two
        // let a document be changed without being rebuilt — the same split as
        // `slides_read`/`slides_edit` for decks, and for the same reason.
        tool(
            "doc_read",
            "Read a .docx: its paragraphs in order, numbered from 1 as `doc_edit` expects. Read before editing — the numbers are how you say which paragraph to change.",
            "files",
            RiskClass::Read,
            params(json!({ "path": {"type": "string"} }), &["path"]),
        ),
        tool(
            "doc_edit",
            "Change the words of paragraphs in an existing .docx, leaving its styles, tables, images and every other paragraph exactly as they are. `edits` is a list of {paragraph, text}, numbered from 1 exactly as `doc_read` lists them. Use this to EDIT a document; `doc_write` REPLACES one.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "paragraph": {"type": "integer"},
                                "text": {"type": "string"}
                            },
                            "required": ["paragraph", "text"]
                        }
                    }
                }),
                &["path", "edits"],
            ),
        ),
        tool(
            "slides_read",
            "Read a .pptx: each slide's shapes and the text in each, numbered from 1 as `slides_edit` expects. Read before editing — the numbers are how you say which text to change.",
            "files",
            RiskClass::Read,
            params(json!({ "path": {"type": "string"} }), &["path"]),
        ),
        tool(
            "slides_edit",
            "Change the text of shapes in an existing .pptx, leaving its design, images and every other shape exactly as they are. `edits` is a list of {slide, shape, text}; slide and shape are numbered from 1, exactly as `slides_read` lists them. Use this to EDIT a deck; `slides_write` REPLACES one.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "edits": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "slide": {"type": "integer"},
                                "shape": {"type": "integer"},
                                "text": {"type": "string"}
                            },
                            "required": ["slide", "shape", "text"]
                        }
                    }
                }),
                &["path", "edits"],
            ),
        ),
        tool(
            "slides_write",
            "Write a slide deck (.pptx). Each slide is a title and its bullets.",
            "files",
            RiskClass::WriteLocal,
            params(
                json!({
                    "path": {"type": "string"},
                    "slides": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"},
                                "bullets": {"type": "array", "items": {"type": "string"}}
                            },
                            "required": ["title"]
                        }
                    }
                }),
                &["path", "slides"],
            ),
        ),
    ]
}

pub(in crate::core::agent) fn search_tools() -> Vec<ToolSpec> {
    vec![tool("grep", "Search files for a regex pattern (ripgrep, .gitignore-aware).", "search", RiskClass::Read,
        params(json!({
            "pattern": {"type": "string", "description": "A regular expression, not a literal phrase. Escape regex characters to search for them literally."},
            "path": {"type": "string", "description": "Directory or file to search under. Defaults to the whole working directory."},
            "glob": {"type": "string", "description": "Filter by filename, e.g. \"*.rs\". Filters which files are searched, never what matches inside them."}
        }), &["pattern"]))]
}

pub(in crate::core::agent) fn shell_tools() -> Vec<ToolSpec> {
    vec![
        tool("process_start",
            "Start a command that keeps running — a dev server, a REPL, a file watcher — and get a handle back. \
             Use run_shell for anything that finishes on its own; use this for anything that doesn't. \
             It returns immediately; read its output with process_read.",
            "shell", RiskClass::Exec,
            params(json!({
                "command": {"type": "string", "description": "The command, e.g. \"npm run dev\" or \"python3 -i\"."},
                "cwd": {"type": "string", "description": "Folder to run it in, relative to the working directory. Defaults to the working directory."}
            }), &["command"])),
        tool("process_read",
            "Read what a running process has printed since you last read it, and whether it is still running. \
             Returns only new output, so reading again is how you wait for a server to come up.",
            "shell", RiskClass::Read,
            params(json!({
                "handle": {"type": "string", "description": "The handle process_start returned."},
                "wait_seconds": {"type": "integer", "description": "Wait up to this long (max 10) for new output before returning. Use it instead of reading in a tight loop."}
            }), &["handle"])),
        tool("process_send",
            "Type a line into a running process — an answer to a prompt, a line for a REPL.",
            "shell", RiskClass::Exec,
            params(json!({
                "handle": {"type": "string", "description": "The handle process_start returned."},
                "input": {"type": "string", "description": "What to type. A newline is added unless newline is false."},
                "newline": {"type": "boolean", "description": "Whether to press Enter after it. Defaults to true."}
            }), &["handle", "input"])),
        tool("process_stop",
            "Stop a process you started and get the last of its output. Stop servers when you are done with them — they hold ports.",
            "shell", RiskClass::Exec,
            params(json!({"handle": {"type": "string", "description": "The handle process_start returned."}}), &["handle"])),
        tool("run_shell", "Run a shell command in the session's persistent working directory and return its output.", "shell", RiskClass::Exec,
        params(json!({
            "command": {"type": "string", "description": "One shell command, which must terminate on its own — this waits for it to exit. To start something long-running like a dev server, redirect its output to a file and poll that file instead."},
            "cwd": {"type": "string", "description": "Run it somewhere else, relative to the working directory. The session keeps its own directory between calls, so this is rarely needed."}
        }), &["command"])),
    ]
}
