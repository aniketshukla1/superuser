import { describe, expect, it } from "vitest";
import { plural, RUN_STALE_AFTER_S, runStatus, scheduleLabel, scheduleShort } from "./scheduleText";
import { clock } from "./when";

const at = (h: number, m: number) => {
  const d = new Date();
  d.setHours(h, m, 0, 0);
  return clock(d);
};

describe("scheduleLabel", () => {
  it("turns the tilde into a word and the time into the system clock", () => {
    expect(scheduleLabel("Every day at ~7:00 AM")).toBe(`Every day at about ${at(7, 0)}`);
    expect(scheduleLabel("Every weekday at ~7:10 PM")).toBe(`Every weekday at about ${at(19, 10)}`);
  });

  it("handles noon and midnight", () => {
    expect(scheduleLabel("Every day at ~12:00 PM")).toBe(`Every day at about ${at(12, 0)}`);
    expect(scheduleLabel("Every day at ~12:30 AM")).toBe(`Every day at about ${at(0, 30)}`);
  });

  it("leaves a raw cron it could not humanise alone", () => {
    expect(scheduleLabel("*/15 9-17 * * 1-5")).toBe("*/15 9-17 * * 1-5");
  });
});

describe("scheduleShort", () => {
  const at = (h: number, m: number) => {
    const d = new Date();
    d.setHours(h, m, 0, 0);
    return clock(d);
  };
  it("keeps only the clock for a daily schedule", () => {
    expect(scheduleShort("Every day at ~7:00 AM")).toBe(at(7, 0));
  });
  it("adds the day, short, when the schedule names one", () => {
    expect(scheduleShort("Every Monday at ~9:30 AM")).toBe(`Mon ${at(9, 30)}`);
    expect(scheduleShort("Fridays at 5:00 PM")).toBe(`Fri ${at(17, 0)}`);
    expect(scheduleShort("Weekdays at about 8:00 AM")).toBe(`Mon–Fri ${at(8, 0)}`);
  });
  it("leaves a schedule without a clock as the full label", () => {
    expect(scheduleShort("Every hour")).toBe("Every hour");
    expect(scheduleShort(null)).toBe("");
  });
});

describe("runStatus", () => {
  const now = Date.parse("2026-09-14T12:00:00Z");
  it("stops believing a run that has said running for hours", () => {
    const started = now / 1000 - RUN_STALE_AFTER_S - 60;
    expect(runStatus("running", started, now)).toEqual({ label: "didn't finish", tone: "stale" });
    expect(runStatus("running", now / 1000 - 60, now).tone).toBe("running");
  });

  it("keeps ok and failures as they are", () => {
    expect(runStatus("ok", 0, now).tone).toBe("ok");
    expect(runStatus("error", 0, now)).toEqual({ label: "error", tone: "error" });
  });
});

describe("plural", () => {
  it("says 1 file and 2 files, not 1 file(s)", () => {
    expect(plural(1, "file")).toBe("1 file");
    expect(plural(2, "file")).toBe("2 files");
    expect(plural(0, "run")).toBe("0 runs");
  });
});
