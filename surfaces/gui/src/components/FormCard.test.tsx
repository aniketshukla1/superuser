import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { FormCard, type FormRequest } from "./FormCard";

afterEach(cleanup);

const FORM: FormRequest = {
  title: "Model assumptions",
  intro: "Two numbers I cannot look up.",
  fields: [
    { name: "margin", label: "Target margin", kind: "number", help: "Gross, not net." },
    { name: "start", label: "Start date", kind: "date", required: false },
  ],
};

describe("a form the agent put to you", () => {
  it("renders a field per entry, typed, with the optional one marked", () => {
    render(<FormCard form={FORM} onDone={vi.fn()} />);
    expect(screen.getByText("Model assumptions")).toBeTruthy();
    expect(screen.getByTestId("formcard-input-margin").getAttribute("type")).toBe("number");
    expect(screen.getByTestId("formcard-input-start").getAttribute("type")).toBe("date");
    expect(screen.getByText("Gross, not net.")).toBeTruthy();
    // Required is the default, so only the exception is labelled.
    expect(screen.getByText(/optional/)).toBeTruthy();
  });

  it("answers keyed by field name, including the ones left blank", () => {
    // Blank must be SENT, not omitted. `Form::read_answer` reports it as "left blank";
    // a missing key would read as a field that was never asked, and that gap is where
    // a model puts its own guess.
    const onDone = vi.fn();
    render(<FormCard form={FORM} onDone={onDone} />);
    fireEvent.change(screen.getByTestId("formcard-input-margin"), { target: { value: "32" } });
    fireEvent.click(screen.getByTestId("formcard-send"));
    expect(onDone).toHaveBeenCalledWith({ margin: "32", start: "" });
  });

  it("trims what was typed", () => {
    // On a TEXT field, where it matters — a `number` input already refuses a value
    // with spaces in it, so trimming there would never be reached.
    const onDone = vi.fn();
    render(
      <FormCard
        form={{ fields: [{ name: "host", label: "Staging host" }] }}
        onDone={onDone}
      />,
    );
    fireEvent.change(screen.getByTestId("formcard-input-host"), { target: { value: "  box.internal  " } });
    fireEvent.click(screen.getByTestId("formcard-send"));
    expect(onDone).toHaveBeenCalledWith({ host: "box.internal" });
  });

  it("lets you send with a required field empty, and says what is still empty", () => {
    // Blank is a real answer. A validator here would buy a number typed to get past
    // it, which is worse than knowing the field was skipped.
    const onDone = vi.fn();
    render(<FormCard form={FORM} onDone={onDone} />);
    expect(screen.getByText("1 still empty")).toBeTruthy();
    fireEvent.click(screen.getByTestId("formcard-send"));
    expect(onDone).toHaveBeenCalledWith({ margin: "", start: "" });
  });

  it("cannot be sent twice", () => {
    // The run is parked on one receiver. A second answer has nothing to resolve, and
    // a double-click is the ordinary way to produce one.
    const onDone = vi.fn();
    render(<FormCard form={FORM} onDone={onDone} />);
    const send = screen.getByTestId("formcard-send");
    fireEvent.click(send);
    fireEvent.click(send);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect((send as HTMLButtonElement).disabled).toBe(true);
  });

  it("renders nothing rather than an empty frame when there are no usable fields", () => {
    const { container } = render(
      <FormCard form={{ fields: [{ name: "", label: "" }] }} onDone={vi.fn()} />,
    );
    expect(container.querySelector("[data-testid=formcard]")).toBeNull();
  });
});
