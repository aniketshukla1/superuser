import { describe, expect, it } from "vitest";
import { clock, dayAndTime, exactTime, shortDate, timeAgo, toMillis } from "./when";

// A fixed "now" at local noon, so day boundaries do not depend on when the suite runs.
const NOW = new Date(2026, 8, 14, 12, 0, 0).getTime(); // 14 Sep 2026, 12:00 local
const secs = (ms: number) => Math.floor(ms / 1000);
const minus = (ms: number) => secs(NOW - ms);

describe("toMillis", () => {
  it("reads epoch seconds, ISO strings, numeric strings and Dates", () => {
    expect(toMillis(1_700_000_000)).toBe(1_700_000_000_000);
    expect(toMillis("1700000000")).toBe(1_700_000_000_000);
    expect(toMillis("2026-09-14T00:00:00Z")).toBe(Date.parse("2026-09-14T00:00:00Z"));
    expect(toMillis(new Date(5000))).toBe(5000);
  });

  it("reads SQLite's space-separated stamps the same way in every engine", () => {
    expect(toMillis("2026-07-01 08:00:00")).toBe(Date.parse("2026-07-01T08:00:00"));
  });

  it("returns null for nothing and for nonsense", () => {
    expect(toMillis(null)).toBeNull();
    expect(toMillis("")).toBeNull();
    expect(toMillis("not a date")).toBeNull();
    expect(toMillis(0)).toBeNull();
  });
});

describe("timeAgo", () => {
  it("says an age for recent moments", () => {
    expect(timeAgo(minus(10_000), NOW)).toBe("just now");
    expect(timeAgo(minus(60_000), NOW)).toBe("1 minute ago");
    expect(timeAgo(minus(5 * 60_000), NOW)).toBe("5 minutes ago");
    expect(timeAgo(minus(3 * 3_600_000), NOW)).toBe("3 hours ago");
  });

  it("says yesterday rather than a count of hours across midnight", () => {
    const lastNight = new Date(2026, 8, 13, 23, 0).getTime();
    expect(timeAgo(secs(lastNight), NOW)).toBe("yesterday");
  });

  it("counts days within the week, then switches to a date", () => {
    expect(timeAgo(secs(new Date(2026, 8, 10, 9, 0).getTime()), NOW)).toBe("4 days ago");
    const old = new Date(2026, 7, 5, 9, 0).getTime();
    expect(timeAgo(secs(old), NOW)).toBe(shortDate(secs(old), NOW));
  });

  it("treats a stamp slightly in the future as now", () => {
    expect(timeAgo(secs(NOW + 30_000), NOW)).toBe("just now");
  });
});

describe("shortDate", () => {
  it("leaves the year off this year's dates and puts it on older ones", () => {
    const thisYear = shortDate(secs(new Date(2026, 7, 5).getTime()), NOW);
    const lastYear = shortDate(secs(new Date(2025, 7, 5).getTime()), NOW);
    expect(thisYear).not.toContain("2026");
    expect(lastYear).toContain("2025");
  });
});

describe("dayAndTime", () => {
  it("names today, yesterday and tomorrow, and dates the rest", () => {
    const at = (d: number, h: number) => secs(new Date(2026, 8, d, h, 0).getTime());
    expect(dayAndTime(at(14, 7), NOW)).toBe(`today at ${clock(at(14, 7))}`);
    expect(dayAndTime(at(13, 7), NOW)).toBe(`yesterday at ${clock(at(13, 7))}`);
    expect(dayAndTime(at(15, 7), NOW)).toBe(`tomorrow at ${clock(at(15, 7))}`);
    expect(dayAndTime(at(10, 7), NOW)).toBe(`${shortDate(at(10, 7), NOW)} at ${clock(at(10, 7))}`);
  });
});

describe("exactTime", () => {
  it("is empty for an unreadable stamp and full otherwise", () => {
    expect(exactTime(null)).toBe("");
    expect(exactTime(secs(NOW)).length).toBeGreaterThan(8);
  });
});
