import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { AskCard } from "./AskCard";

afterEach(cleanup);

const Q1 = {
  question: "Where is this initiative right now?",
  options: [
    { label: "Still pre-presentation", description: "The deck hasn't been shown to stakeholders yet." },
    { label: "Presented, now piloting", description: "Leadership has seen it; phase 1 is executing." },
  ],
};
const Q2 = {
  question: "Which teams first?",
  options: [{ label: "Platform" }, { label: "Apps" }],
};

const one = (onDone = vi.fn()) => {
  render(<AskCard questions={[Q1]} onDone={onDone} />);
  return onDone;
};

describe("AskCard", () => {
  it("shows the answers the agent thinks are likely, with what each one means", () => {
    // The whole point of the rebuild: not a blank box, but the agent's own candidates.
    one();
    expect(screen.getByText("Still pre-presentation")).toBeTruthy();
    expect(screen.getByText("Leadership has seen it; phase 1 is executing.")).toBeTruthy();
  });

  it("always offers a way out of a wrong list", () => {
    one();
    expect(screen.getByTestId("ask-other")).toBeTruthy();
    expect(screen.getByPlaceholderText("Type your own answer here")).toBeTruthy();
  });

  it("sends the chosen answer", () => {
    const onDone = one();
    fireEvent.click(screen.getByText("Presented, now piloting"));
    fireEvent.click(screen.getByTestId("ask-next"));
    expect(onDone).toHaveBeenCalledWith(["Presented, now piloting"]);
  });

  it("sends what the user typed when none of the options fit", () => {
    const onDone = one();
    fireEvent.change(screen.getByPlaceholderText("Type your own answer here"), {
      target: { value: "we're doing both at once" },
    });
    fireEvent.click(screen.getByTestId("ask-next"));
    expect(onDone).toHaveBeenCalledWith(["we're doing both at once"]);
  });

  it("will not advance on nothing — Skip is the way past", () => {
    const onDone = one();
    expect((screen.getByTestId("ask-next") as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByTestId("ask-skip"));
    // null, not "" — the model is told it was skipped rather than answered blankly.
    expect(onDone).toHaveBeenCalledWith([null]);
  });

  it("steps through several questions with the same card", () => {
    const onDone = vi.fn();
    render(<AskCard questions={[Q1, Q2]} onDone={onDone} />);
    expect(screen.getByTestId("ask-step").textContent).toBe("1/2");
    expect(screen.getByTestId("ask-next").textContent).toBe("Next");

    fireEvent.click(screen.getByText("Still pre-presentation"));
    fireEvent.click(screen.getByTestId("ask-next"));
    expect(onDone).not.toHaveBeenCalled();

    expect(screen.getByTestId("ask-step").textContent).toBe("2/2");
    expect(screen.getByText("Which teams first?")).toBeTruthy();
    // Last question submits rather than pretending there is another.
    expect(screen.getByTestId("ask-next").textContent).toBe("Done");

    fireEvent.click(screen.getByText("Apps"));
    fireEvent.click(screen.getByTestId("ask-next"));
    expect(onDone).toHaveBeenCalledWith(["Still pre-presentation", "Apps"]);
  });

  it("does not show a counter when there is only one question", () => {
    one();
    expect(screen.queryByTestId("ask-step")).toBeNull();
  });

  it("carries a selection forward without leaking it into the next question", () => {
    const onDone = vi.fn();
    render(<AskCard questions={[Q1, Q2]} onDone={onDone} />);
    fireEvent.click(screen.getByText("Still pre-presentation"));
    fireEvent.click(screen.getByTestId("ask-next"));
    // Nothing pre-selected on arrival, so Next is inert until this question is answered.
    expect((screen.getByTestId("ask-next") as HTMLButtonElement).disabled).toBe(true);
  });

  it("dismissing declines the whole ask, not just this question", () => {
    const onDone = vi.fn();
    render(<AskCard questions={[Q1, Q2]} onDone={onDone} />);
    fireEvent.click(screen.getByTestId("ask-dismiss"));
    expect(onDone).toHaveBeenCalledWith([null, null]);
  });

  it("selects by number key", () => {
    const onDone = one();
    fireEvent.keyDown(screen.getByTestId("ask-card"), { key: "2" });
    expect(screen.getByText("Presented, now piloting").closest("button")?.getAttribute("aria-pressed")).toBe("true");
    fireEvent.keyDown(screen.getByTestId("ask-card"), { key: "Enter" });
    expect(onDone).toHaveBeenCalledWith(["Presented, now piloting"]);
  });

  it("leaves digits alone once you are typing your own answer", () => {
    // Otherwise "Q4" picks option 4 and swallows the answer.
    one();
    const input = screen.getByPlaceholderText("Type your own answer here");
    fireEvent.change(input, { target: { value: "Q4" } });
    fireEvent.keyDown(input, { key: "4" });
    expect(screen.getByText("Still pre-presentation").closest("button")?.getAttribute("aria-pressed")).toBe("false");
    expect((input as HTMLInputElement).value).toBe("Q4");
  });

  it("lets several be chosen when the question allows it", () => {
    const onDone = vi.fn();
    render(<AskCard questions={[{ ...Q2, multi: true }]} onDone={onDone} />);
    fireEvent.click(screen.getByText("Platform"));
    fireEvent.click(screen.getByText("Apps"));
    fireEvent.click(screen.getByTestId("ask-next"));
    expect(onDone).toHaveBeenCalledWith(["Platform, Apps"]);
  });
});
