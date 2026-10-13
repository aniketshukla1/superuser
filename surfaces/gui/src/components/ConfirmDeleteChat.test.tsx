import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ConfirmDeleteChat } from "./ConfirmDeleteChat";

const artifacts = vi.hoisted(() => vi.fn());
vi.mock("../api", () => ({ getArtifacts: artifacts }));

const props = {
  title: "Weekly plan",
  sessionId: "s-1",
  messages: 12,
  onConfirm: vi.fn(),
  onCancel: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
  artifacts.mockResolvedValue([]);
});

// The dialog portals to <body>, so it outlives the container RTL tears down on its
// own — without this each test finds every earlier test's dialog as well.
afterEach(cleanup);

describe("the delete-chat dialog", () => {
  /// Deleting is `remove_dir_all` on the session folder, so the files the agent
  /// produced go with the transcript. A dialog that only said "are you sure?"
  /// would be hiding the half of the loss the user actually cares about.
  it("names the files that will be destroyed, not just the messages", async () => {
    artifacts.mockResolvedValue([{ name: "brief.md" }, { name: "budget.xlsx" }]);
    render(<ConfirmDeleteChat {...props} />);

    await waitFor(() => expect(screen.getByText("brief.md")).toBeTruthy());
    expect(screen.getByText("budget.xlsx")).toBeTruthy();

    const dialog = screen.getByTestId("confirm-delete-chat");
    expect(dialog.textContent).toContain("12 messages");
    expect(dialog.textContent).toContain("2 files");
    expect(dialog.textContent).toContain("This cannot be undone");
  });

  /// A chat with nothing but conversation shouldn't be described as losing files,
  /// and shouldn't render an empty box where the list would be.
  it("says only what is true when the chat produced no files", async () => {
    render(<ConfirmDeleteChat {...props} messages={1} />);
    const dialog = screen.getByTestId("confirm-delete-chat");
    await waitFor(() => expect(artifacts).toHaveBeenCalledWith("s-1"));
    expect(dialog.textContent).toContain("1 message");
    expect(dialog.textContent).not.toContain("file");
  });

  /// A listing that fails must not become "no files" — claiming nothing will be
  /// lost is the one wrong answer worse than saying less.
  it("stays quiet about files it could not count rather than claiming none", async () => {
    artifacts.mockRejectedValue(new Error("offline"));
    render(<ConfirmDeleteChat {...props} />);
    await waitFor(() => expect(artifacts).toHaveBeenCalled());
    const dialog = screen.getByTestId("confirm-delete-chat");
    expect(dialog.textContent).toContain("12 messages");
    expect(dialog.textContent).toContain("This cannot be undone");
    expect(dialog.textContent).not.toContain("0 files");
  });

  /// The dialog is here because the action is irreversible, so the button that
  /// fires on a stray Enter has to be the one that changes nothing.
  it("puts focus on Cancel, not on Delete", async () => {
    render(<ConfirmDeleteChat {...props} />);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByTestId("confirm-delete-cancel")));
  });

  it("confirms and cancels through their own buttons", async () => {
    render(<ConfirmDeleteChat {...props} />);
    fireEvent.click(screen.getByTestId("confirm-delete-cancel"));
    expect(props.onCancel).toHaveBeenCalledTimes(1);
    expect(props.onConfirm).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("confirm-delete-confirm"));
    expect(props.onConfirm).toHaveBeenCalledTimes(1);
  });

  it("Escape cancels", async () => {
    render(<ConfirmDeleteChat {...props} />);
    fireEvent.keyDown(screen.getByTestId("confirm-delete-chat"), { key: "Escape" });
    expect(props.onCancel).toHaveBeenCalledTimes(1);
  });
});
