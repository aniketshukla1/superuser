// Markdown, read aloud as plain text — for the places that show a glimpse of a result
// rather than the result: a run's two-line preview, a list row.
//
// Those glimpses printed the markdown source: "**Completing financial budget task**" with the
// asterisks, "### Project Overview" with the hashes, a list that began "*   **Status:**". The
// full result renders through <Markdown>; a preview has no room for formatting, so it gets
// the words and none of the syntax.

export function plainText(md: string): string {
  return (
    md
      // Fenced code: keep the code, drop the fence and its language tag.
      .replace(/```[\w-]*\n?([\s\S]*?)```/g, "$1")
      // Images before links, or the link rule eats the `!`.
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
      // Headings, block quotes and list markers at the start of a line.
      .replace(/^\s{0,3}#{1,6}\s+/gm, "")
      .replace(/^\s{0,3}>\s?/gm, "")
      .replace(/^\s*(?:[-*+]|\d+[.)])\s+/gm, "")
      // Emphasis and inline code. Bold before italic, so `**x**` is not read as `*` + `*x*` + `*`.
      .replace(/(\*\*|__)(.+?)\1/g, "$2")
      .replace(/(\*|_)(\S(?:.*?\S)?)\1/g, "$2")
      .replace(/`([^`]+)`/g, "$1")
      // Horizontal rules and table pipes carry no words.
      .replace(/^\s*(?:-{3,}|\*{3,}|_{3,})\s*$/gm, "")
      .replace(/^\s*\|?(?:\s*:?-+:?\s*\|)+\s*$/gm, "")
      .replace(/\s*\|\s*/g, " · ")
      // Collapse the blank lines the removals leave behind.
      .replace(/\n{3,}/g, "\n\n")
      .trim()
  );
}
