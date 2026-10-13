import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { AttachMenu, ContextRing, MicControl, ModeMenu, type ModeOption } from "../ComposerControls";

afterEach(cleanup);

const MODES: ModeOption[] = [
  { value: "discuss", label: "Discuss", description: "Chat and explore" },
  { value: "interactive", label: "Ask for approval", description: "Ask before edits" },
];
const BYPASS: ModeOption = { value: "auto", label: "Bypass permissions", description: "Run everything" };

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string) => ({
      ok: true,
      json: async () => {
        if (String(url).includes("/v1/voice/devices")) {
          return {
            devices: [
              { name: "MacBook Pro Microphone (Built-in)", is_default: true, usable: true },
              { name: "Microsoft Teams Audio Device (Virtual)", is_default: false, usable: false },
            ],
            selected: "",
            selected_present: true,
            hold_to_record: false,
          };
        }
        if (String(url).includes("/v1/usage/rollup")) {
          return {
            now: 1_800_000_000,
            today: { input: 900, output: 100, tokens: 1000, counted: 4, uncounted: 0, resets_at: 1_800_003_600 },
            week: { input: 4000, output: 1000, tokens: 5000, counted: 9, uncounted: 3, resets_at: 1_800_200_000 },
          };
        }
        return {};
      },
    })) as never,
  );
});
describe("the mode menu", () => {
  it("keeps the guard-off mode out of the plain list of ways to work", () => {
    render(<ModeMenu mode="interactive" options={MODES} bypass={BYPASS} onModeChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    const menu = screen.getByTestId("mode-menu");
    // Below a divider, with a verb of its own rather than a third slot in the list.
    expect(within(menu).getByTestId("mode-auto").textContent).toContain("Enable");
    expect(within(menu).getByTestId("mode-interactive").textContent).not.toContain("Enable");
  });

  it("marks the current mode and names the chip after it", () => {
    render(<ModeMenu mode="discuss" options={MODES} bypass={BYPASS} onModeChange={vi.fn()} />);
    expect(screen.getByTestId("mode-chip").textContent).toBe("Discuss");
    fireEvent.click(screen.getByTestId("mode-chip"));
    expect(screen.getByTestId("mode-discuss").getAttribute("aria-checked")).toBe("true");
    expect(screen.getByTestId("mode-interactive").getAttribute("aria-checked")).toBe("false");
  });

  // The badges are a promise. An unbound number key makes every other badge a lie.
  it("the number on a row actually picks that row", () => {
    const onModeChange = vi.fn();
    render(<ModeMenu mode="discuss" options={MODES} bypass={BYPASS} onModeChange={onModeChange} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    fireEvent.keyDown(document, { key: "2" });
    expect(onModeChange).toHaveBeenCalledWith("interactive");
  });

  it("leaves the number keys to the textarea while the menu is shut", () => {
    const onModeChange = vi.fn();
    render(<ModeMenu mode="discuss" options={MODES} bypass={BYPASS} onModeChange={onModeChange} />);
    fireEvent.keyDown(document, { key: "2" });
    expect(onModeChange).not.toHaveBeenCalled();
  });

  // Plan was hidden for a release and unhidden on the owner's ask. The description has
  // to keep stating the gap — there is no approve-this-plan button — so this asserts the
  // caveat as well as the row.
  it("offers Plan, and says what it does not do", () => {
    const withPlan = [
      MODES[0],
      { value: "plan", label: "Plan", description: "Outline the approach first; nothing runs until you switch mode" },
      MODES[1],
    ];
    render(<ModeMenu mode="discuss" options={withPlan} bypass={BYPASS} onModeChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    const plan = screen.getByTestId("mode-plan");
    expect(plan.textContent).toContain("nothing runs until you switch mode");
    // And it stays above the divider — it is a way of working, not a guard being lifted.
    expect(plan.textContent).not.toContain("Enable");
  });

  it("closes on Escape, so a keyboard user is not trapped in the panel", () => {
    render(<ModeMenu mode="discuss" options={MODES} bypass={BYPASS} onModeChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    expect(screen.getByTestId("mode-menu")).toBeTruthy();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("mode-menu")).toBeNull();
  });
});

describe("the context ring's gauge", () => {
  const arcs = () => document.querySelectorAll(".composer-ring circle").length;

  // A round line cap paints a dot even on a zero-length arc, so an empty gauge showed a
  // mark that reads as "a little bit used" — the wrong thing to say when the window is
  // unknown or the session has not started.
  it("draws no arc at all when there is nothing to show", () => {
    render(<ContextRing contextUsed={0} contextWindow={100000} />);
    expect(arcs()).toBe(1); // the track, and only the track
  });

  it("draws the arc once there is something to draw", () => {
    render(<ContextRing contextUsed={50000} contextWindow={100000} />);
    expect(arcs()).toBe(2);
  });

  it("draws nothing when the window is unknown, rather than a full or empty guess", () => {
    render(<ContextRing contextUsed={50000} />);
    expect(arcs()).toBe(1);
  });
});

describe("the + menu", () => {
  it("offers one files row rather than asking you to classify the file first", () => {
    const onFiles = vi.fn();
    render(<AttachMenu onFiles={onFiles} onSlashCommands={vi.fn()} />);
    fireEvent.click(screen.getByTestId("attach-chip"));
    fireEvent.click(screen.getByTestId("attach-files"));
    expect(onFiles).toHaveBeenCalled();
  });

  // A menu is a list of things you can do. "Add plugins…" and "Enable fast mode" were
  // neither — disabled rows for features that do not exist — and took a row each from the
  // two menus a person opens most.
  it("lists nothing it cannot actually do", () => {
    render(<AttachMenu onFiles={vi.fn()} onSlashCommands={vi.fn()} />);
    fireEvent.click(screen.getByTestId("attach-chip"));
    expect(screen.queryByTestId("attach-plugins")).toBeNull();
    expect(screen.getByTestId("attach-menu").textContent).not.toContain("Coming soon");
  });

  it("calls the command list what Settings calls it", () => {
    render(<AttachMenu onFiles={vi.fn()} onSlashCommands={vi.fn()} />);
    fireEvent.click(screen.getByTestId("attach-chip"));
    expect(screen.getByTestId("attach-slash").textContent).toContain("Skills");
  });

  it("omits rows it has nowhere to send you", () => {
    render(<AttachMenu onFiles={vi.fn()} onSlashCommands={vi.fn()} />);
    fireEvent.click(screen.getByTestId("attach-chip"));
    expect(screen.queryByTestId("attach-connectors")).toBeNull();
    expect(screen.queryByTestId("attach-folder")).toBeNull();
  });
});

describe("the microphone", () => {
  it("reads the device list when opened, not once at startup", async () => {
    render(<MicControl recording={false} ready onToggle={vi.fn()} />);
    expect(globalThis.fetch).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId("mic-chevron"));
    await waitFor(() => expect(screen.getByText("MacBook Pro Microphone (Built-in)")).toBeTruthy());
  });

  // A device that enumerates but will not open is worse than one that is missing: it
  // looks choosable and fails at record time.
  it("disables a device that will not open", async () => {
    render(<MicControl recording={false} ready onToggle={vi.fn()} />);
    fireEvent.click(screen.getByTestId("mic-chevron"));
    const row = await screen.findByTestId("mic-device-Microsoft Teams Audio Device (Virtual)");
    expect(row.hasAttribute("disabled")).toBe(true);
    expect(row.textContent).toContain("unavailable");
  });

  it("a muted mic still takes the click and still says why", () => {
    render(<MicControl recording={false} ready={false} onToggle={vi.fn()} />);
    const mic = screen.getByLabelText("Configure Voice Input in Settings");
    expect(mic.getAttribute("aria-disabled")).toBe("true");
    expect(mic.hasAttribute("disabled")).toBe(false);
  });
});
