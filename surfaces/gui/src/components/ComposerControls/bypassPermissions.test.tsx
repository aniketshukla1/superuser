import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ContextRing, EffortChip, ModeMenu, ModelMenu, type ModeOption } from "../ComposerControls";

afterEach(cleanup);
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
describe("bypass permissions", () => {
  const OPTIONS: ModeOption[] = [
    { value: "ask", label: "Ask me", description: "Stop before anything that writes" },
  ];

  it("asks before switching the guard off, and leaves the mode alone until you say yes", () => {
    const onModeChange = vi.fn();
    render(<ModeMenu mode="ask" options={OPTIONS} bypass={BYPASS} onModeChange={onModeChange} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    fireEvent.click(screen.getByTestId("mode-auto"));
    expect(onModeChange).not.toHaveBeenCalled();
    expect(screen.getByTestId("mode-bypass-confirm")).toBeTruthy();

    fireEvent.click(screen.getByTestId("mode-bypass-yes"));
    expect(onModeChange).toHaveBeenCalledWith("auto");
  });

  it("does not ask again when it is already the mode you are in", () => {
    const onModeChange = vi.fn();
    render(<ModeMenu mode="auto" options={OPTIONS} bypass={BYPASS} onModeChange={onModeChange} />);
    fireEvent.click(screen.getByTestId("mode-chip"));
    fireEvent.click(screen.getByTestId("mode-ask"));
    expect(onModeChange).toHaveBeenCalledWith("ask");
    expect(screen.queryByTestId("mode-bypass-confirm")).toBeNull();
  });
});

describe("the model menu", () => {
  const MODELS = ["a", "b", "c", "d", "e", "f"];

  it("shows four and puts the rest behind More models", () => {
    render(<ModelMenu model="a" models={MODELS} onChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("model-chip"));
    expect(screen.getByTestId("model-a")).toBeTruthy();
    expect(screen.getByTestId("model-d")).toBeTruthy();
    expect(screen.queryByTestId("model-e")).toBeNull();
    fireEvent.click(screen.getByTestId("model-more"));
    expect(screen.getByTestId("model-e")).toBeTruthy();
  });

  it("keeps the model in use listed even when the server has stopped offering it", () => {
    render(<ModelMenu model="retired" models={MODELS} onChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("model-chip"));
    expect(screen.getByTestId("model-retired").getAttribute("aria-checked")).toBe("true");
  });

  it("marks the default without pretending it is the current one", () => {
    render(<ModelMenu model="b" models={MODELS} defaultModel="a" onChange={vi.fn()} />);
    fireEvent.click(screen.getByTestId("model-chip"));
    expect(screen.getByTestId("model-a").textContent).toContain("Default");
    expect(screen.getByTestId("model-a").getAttribute("aria-checked")).toBe("false");
    expect(screen.getByTestId("model-b").getAttribute("aria-checked")).toBe("true");
  });
});

describe("effort", () => {
  const LEVELS = ["low", "medium", "high", "xhigh", "max"];

  it("puts the handle where the current level is", () => {
    render(<EffortChip effort="high" levels={LEVELS} onChange={vi.fn()} />);
    expect(screen.getByTestId("effort-chip").textContent).toBe("High");
    fireEvent.click(screen.getByTestId("effort-chip"));
    const range = screen.getByTestId("effort-range") as HTMLInputElement;
    expect(range.value).toBe("2");
    expect(range.max).toBe("4");
  });

  it("moving the slider reports the level, not the index", () => {
    const onChange = vi.fn();
    render(<EffortChip effort="low" levels={LEVELS} onChange={onChange} />);
    fireEvent.click(screen.getByTestId("effort-chip"));
    fireEvent.change(screen.getByTestId("effort-range"), { target: { value: "4" } });
    expect(onChange).toHaveBeenCalledWith("max");
  });
});

describe("the context ring", () => {
  it("says the window is unknown rather than drawing an empty gauge", () => {
    render(<ContextRing contextUsed={1234} />);
    fireEvent.click(screen.getByTestId("context-ring"));
    expect(screen.getByTestId("context-menu").textContent).toContain("isn’t known");
  });

  // The whole reason this section is "tokens through this app" and not "plan usage".
  it("says when a window's total is only a floor", async () => {
    render(<ContextRing contextUsed={100} contextWindow={1000} />);
    fireEvent.click(screen.getByTestId("context-ring"));
    const week = await screen.findByTestId("usage-this week");
    expect(week.textContent).toContain("didn’t report a cost");
    const today = screen.getByTestId("usage-today");
    expect(today.textContent).not.toContain("didn’t report a cost");
  });

  // There is no quota here, so nothing may be drawn as a fraction of one: the fuller
  // window filled its bar completely and counted down to a "reset" that means nothing.
  it("reports usage as counts, with no quota bar and no countdown", async () => {
    render(<ContextRing contextUsed={100} contextWindow={1000} />);
    fireEvent.click(screen.getByTestId("context-ring"));
    const week = await screen.findByTestId("usage-this week");
    expect(week.textContent).not.toMatch(/Resets in/);
    expect(week.querySelector(".composer-usage-track")).toBeNull();
    expect(week.textContent).toContain("5k");
  });

  it("keeps the per-model table, one disclosure down", async () => {
    render(<ContextRing contextUsed={100} contextWindow={1000} breakdown={<div>per-model rows</div>} />);
    fireEvent.click(screen.getByTestId("context-ring"));
    expect(screen.queryByText("per-model rows")).toBeNull();
    fireEvent.click(await screen.findByTestId("usage-breakdown"));
    expect(screen.getByText("per-model rows")).toBeTruthy();
  });
});
