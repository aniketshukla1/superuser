import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ProfileSection } from "./ProfileSection";

// The profile is the one memory store that acts on its own — it is folded into
// every system prompt without anyone asking for it. So the thing worth testing is
// not that it renders, but that it is *honest*: what it acts on and what it is
// merely weighing must be told apart on screen, and the evidence must be there.

const FACETS = [
  {
    class: "style",
    key: "reply-length",
    value: "Wants short answers",
    state: "active",
    score: 2,
    cue: "explicit",
    observations: 1,
    last_seen: Date.now() / 1000,
    evidence: "stop explaining things I already know",
    pinned: false,
    from_form: false,
  },
  {
    class: "tooling",
    key: "editor",
    value: "Uses Zed",
    state: "provisional",
    score: 0.6,
    cue: "behavioral",
    observations: 1,
    last_seen: Date.now() / 1000,
    evidence: "opened it in zed",
    pinned: false,
    from_form: false,
  },
];

const PROFILE = {
  form: { name: "Jordan", call_me: "", role: "", about: "", tone: "", interests: "", likes: "", dislikes: "", goals: "" },
  facets: FACETS,
  forgotten: [],
  learning_enabled: true,
  path: "/vault/profile/PROFILE.md",
};

function stubFetch(profile: any = PROFILE) {
  const calls: { url: string; body: any }[] = [];
  const fn = vi.fn(async (url: string, init?: RequestInit) => {
    calls.push({ url: String(url), body: init?.body ? JSON.parse(String(init.body)) : null });
    if (String(url).includes("/v1/profile/")) {
      return { ok: true, json: async () => ({ ok: true, facets: profile.facets, forgotten: profile.forgotten }) } as Response;
    }
    return { ok: true, json: async () => profile } as Response;
  });
  vi.stubGlobal("fetch", fn);
  return calls;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("ProfileSection", () => {
  it("shows what it acts on, and keeps what it is only weighing out of that list", async () => {
    stubFetch();
    render(<ProfileSection />);

    expect(await screen.findByText("Wants short answers")).toBeTruthy();
    // A provisional facet does not reach a conversation, so it must not sit in the
    // list of things that do.
    expect(screen.queryByText("Uses Zed")).toBeNull();

    fireEvent.click(screen.getByTestId("profile-weighing"));
    expect(await screen.findByText("Uses Zed")).toBeTruthy();
  });

  it("shows the words behind a belief", async () => {
    stubFetch();
    render(<ProfileSection />);

    // Quoted: it is what the person said, and it reads as a quote.
    expect(await screen.findByText(/stop explaining things I already know/)).toBeTruthy();
  });

  it("addresses pin and forget by the whole facet, not just its key", async () => {
    // Two beliefs can share a key and compete for it. Sending only the key would
    // pin or forget whichever one happened to be winning.
    const calls = stubFetch();
    render(<ProfileSection />);

    fireEvent.click(await screen.findByText("Pin"));
    await waitFor(() => expect(calls.some((c) => c.url.includes("/v1/profile/pin"))).toBe(true));
    const pin = calls.find((c) => c.url.includes("/v1/profile/pin"))!;
    expect(pin.body).toMatchObject({ class: "style", key: "reply-length", value: "Wants short answers", pinned: true });
  });

  it("saves the form the user typed", async () => {
    const calls = stubFetch();
    render(<ProfileSection />);

    const field = await screen.findByTestId("profile-field-call_me");
    fireEvent.change(field, { target: { value: "Nik" } });
    fireEvent.click(screen.getByTestId("profile-save"));

    await waitFor(() => expect(calls.some((c) => c.url.includes("/v1/profile/form"))).toBe(true));
    expect(calls.find((c) => c.url.includes("/v1/profile/form"))!.body.call_me).toBe("Nik");
  });

  it("does not offer to forget something the form is still generating", async () => {
    // Forgetting a form field would leave it visible in the form below and no
    // longer applying — two parts of one page disagreeing about what is true.
    stubFetch({
      ...PROFILE,
      facets: [{ ...FACETS[0], from_form: true, evidence: "Your profile ▸ Preferred tone" }],
    });
    render(<ProfileSection />);

    // The field it came from rides the cue line, once, instead of a second line repeating it.
    expect(await screen.findByText(/from your profile · Preferred tone/)).toBeTruthy();
    expect(screen.queryByText("Forget")).toBeNull();
    expect(screen.queryByText("Pin")).toBeNull();
  });

  it("hides a storage record posing as evidence", async () => {
    // A fact saved by a tool call carried its own record as evidence, which printed the value
    // again inside the machine's punctuation.
    stubFetch({
      ...PROFILE,
      facets: [{ ...FACETS[0], value: "Do not ask any questions", evidence: "style | questions | Do not ask any questions | explicit" }],
    });
    render(<ProfileSection />);
    expect(await screen.findByText("Do not ask any questions")).toBeTruthy();
    expect(screen.queryByText(/style \| questions/)).toBeNull();
  });

  it("says what fills an empty profile rather than showing an empty frame", async () => {
    stubFetch({ ...PROFILE, facets: [] });
    render(<ProfileSection />);

    expect(await screen.findByText(/anything you say about how you want to be worked with/i)).toBeTruthy();
  });
});
