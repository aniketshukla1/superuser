import { describe, expect, it, vi, afterEach } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { CodeBlock, highlight, normaliseLang } from "./CodeBlock";
import { Markdown } from "./Markdown";

afterEach(cleanup);

describe("fence languages", () => {
  it("treats the names people actually write as one language", () => {
    expect(normaliseLang("js")).toBe("javascript");
    expect(normaliseLang("JSX")).toBe("javascript");
    expect(normaliseLang("py")).toBe("python");
    expect(normaliseLang("rs")).toBe("rust");
    expect(normaliseLang("bash")).toBe("shell");
    expect(normaliseLang(" TS ")).toBe("typescript");
    // Unknown stays itself — it still labels the block, it just will not highlight.
    expect(normaliseLang("brainfuck")).toBe("brainfuck");
    expect(normaliseLang(undefined)).toBe("");
  });
});

describe("highlighting", () => {
  it("loses no characters", async () => {
    // The property that matters. Highlighting rebuilds the string out of spans, and
    // a gap between two token ranges — or a tail after the last one — silently eats
    // code. Someone reading a snippet has no way to know a character went missing.
    const parser = (await import("@codemirror/lang-python")).python().language.parser;
    const code = 'def hello(name):\n    # greet\n    return f"hi {name}"\n\n\nhello("world")\n';
    const pieces = await highlight(code, parser as never);
    expect(pieces.map(([t]) => t).join("")).toBe(code);
    // And it actually did something, or the assertion above is vacuous.
    expect(pieces.some(([, cls]) => cls.includes("tok-keyword"))).toBe(true);
    expect(pieces.some(([, cls]) => cls.includes("tok-comment"))).toBe(true);
  });

  it("shows the code before any grammar has loaded, and for languages we ship none for", async () => {
    // A block must never be blank. Highlighting is an improvement on plain text, not
    // a precondition for showing it.
    render(<CodeBlock lang="brainfuck" code={"++++[>++++<-]>."} />);
    expect(screen.getByTestId("codeblock").textContent).toContain("++++[>++++<-]>.");
  });
});

describe("a code block in the conversation", () => {
  it("names its language and offers a copy", async () => {
    render(<CodeBlock lang="py" code={"def f(x):\n    return x + 1"} />);
    expect(screen.getByText("Python")).toBeTruthy();
    expect(screen.getByTestId("codeblock-copy")).toBeTruthy();
    // Asserted on `tok-keyword` specifically, not "any span": every grammar emits
    // punctuation, so "something got a class" would pass on a parser that understood
    // nothing. `def` being a keyword means it really parsed Python.
    await waitFor(() =>
      expect(screen.getByTestId("codeblock").querySelector(".tok-keyword")).toBeTruthy(),
    );
  });

  it("copies the code, not the markup", async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    const code = "const a = 1;\nconsole.log(a);";
    render(<CodeBlock lang="js" code={code} />);
    screen.getByTestId("codeblock-copy").click();
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(code));
    vi.unstubAllGlobals();
  });
});

describe("markdown routing", () => {
  it("sends a fence to the code block and leaves inline code alone", async () => {
    render(<Markdown text={"Use `npm test` first.\n\n```python\nprint(1)\n```\n"} />);
    // The fence became a block...
    await waitFor(() => expect(screen.getByTestId("codeblock")).toBeTruthy());
    expect(screen.getByText("Python")).toBeTruthy();
    // ...and the inline span did not: one block on the page, not two.
    expect(screen.getAllByTestId("codeblock")).toHaveLength(1);
    expect(screen.getByText("npm test").tagName).toBe("CODE");
  });

  it("treats an unlabelled multi-line fence as code, not prose", async () => {
    // No language is common in real output. It is still a block: a monospace,
    // scrollable, copyable one, just without highlighting.
    render(<Markdown text={"```\nline one\nline two\n```\n"} />);
    await waitFor(() => expect(screen.getByTestId("codeblock")).toBeTruthy());
    expect(screen.getByTestId("codeblock").textContent).toContain("line one");
  });

  it("still renders GFM tables", async () => {
    // Named because "tables in chat" was asked for as if it were missing. It is not;
    // remark-gfm has always been on. This is here so it cannot quietly stop being true.
    render(<Markdown text={"| Region | Revenue |\n| --- | --- |\n| North | 120 |\n"} />);
    expect(screen.getByRole("table")).toBeTruthy();
    expect(screen.getByText("Region")).toBeTruthy();
    expect(screen.getByText("120")).toBeTruthy();
  });
});
