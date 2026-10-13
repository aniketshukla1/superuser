// The Read/Write control on one source.
//
// Tested in isolation rather than through the whole Access panel: the panel fetches
// connections, the connector index, subscriptions and recent channels on mount, and
// none of that is what this control is. Rendering it directly is what lets the gate
// be verified with no connector connected and no credentials anywhere near a test.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ScopePicker } from "./AccessSection";

afterEach(cleanup);

describe("choosing how far a session may go through a source", () => {
  it("shows both ends of the choice", () => {
    render(<ScopePicker scope="write" onPick={() => {}} />);
    expect(screen.getByTestId("scope-read")).toBeTruthy();
    expect(screen.getByTestId("scope-write")).toBeTruthy();
  });

  it("marks the scope currently in force, so the row states it rather than implying it", () => {
    render(<ScopePicker scope="read" onPick={() => {}} />);
    expect(screen.getByTestId("scope-read").getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByTestId("scope-write").getAttribute("aria-pressed")).toBe("false");
  });

  it("narrows a source to read when the reader asks for it", () => {
    const onPick = vi.fn();
    render(<ScopePicker scope="write" onPick={onPick} />);
    fireEvent.click(screen.getByTestId("scope-read"));
    expect(onPick).toHaveBeenCalledWith("read");
  });

  it("widens back to write", () => {
    const onPick = vi.fn();
    render(<ScopePicker scope="read" onPick={onPick} />);
    fireEvent.click(screen.getByTestId("scope-write"));
    expect(onPick).toHaveBeenCalledWith("write");
  });

  it("does not re-send the scope already in force", () => {
    // Every pick is a round trip to the server; clicking the option you are already
    // on should cost nothing.
    const onPick = vi.fn();
    render(<ScopePicker scope="read" onPick={onPick} />);
    fireEvent.click(screen.getByTestId("scope-read"));
    expect(onPick).not.toHaveBeenCalled();
  });

  it("goes inert when the source is muted, without hiding what the setting is", () => {
    // A muted source has no reach to narrow, but the chosen scope stays legible so
    // un-muting it holds no surprise.
    const onPick = vi.fn();
    render(<ScopePicker scope="read" disabled onPick={onPick} />);
    const read = screen.getByTestId("scope-read") as HTMLButtonElement;
    const write = screen.getByTestId("scope-write") as HTMLButtonElement;
    expect(write.disabled).toBe(true);
    expect(read.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(write);
    expect(onPick).not.toHaveBeenCalled();
  });

  it("says what each end means, for anyone who has not read the docs", () => {
    render(<ScopePicker scope="write" onPick={() => {}} />);
    expect(screen.getByTestId("scope-read").getAttribute("title")).toMatch(/never send/i);
    expect(screen.getByTestId("scope-write").getAttribute("title")).toMatch(/send/i);
  });
});
