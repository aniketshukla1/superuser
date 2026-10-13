// A failure the server wrote as prose, turned into something a person can act on.
//
// Two kinds reach the transcript. A turn that dies is persisted as an `error` notice and
// rendered as "Error: <reason>". A memory question (@brain) that cannot reach a model does
// not die — it ANSWERS, with "I couldn't search my memory just now — <reason>" — so it
// arrived as an ordinary reply: the same type, the same bubble, the same thumbs as a real
// answer, with a provider's raw response pasted into the middle of it:
//
//   I couldn't search my memory just now — OpenCode Zen error 401 Unauthorized:
//   {"type":"error","error":{"type":"CreditsError","message":"No payment method…
//
// Both carry a reason in the same few shapes, because both come out of the same model
// router. This names what happened in a sentence, says where the fix is when there is
// one, and keeps the unedited reason for anyone who wants it.

export interface Failure {
  /** What failed, as a heading. */
  title: string;
  /** Why, in one sentence, with no provider JSON in it. */
  summary: string;
  /** The reason exactly as the server wrote it, for the Details disclosure. */
  detail: string;
  /** Where the fix lives, when it lives somewhere in the app. */
  fix?: "models";
}

/** The exact lead-in `api_server.rs` writes when a memory question cannot reach a model. */
export const BRAIN_FAILURE_PREFIX = "I couldn't search my memory just now — ";

const sentence = (s: string) => {
  const t = s.trim();
  if (!t) return t;
  const cased = t.charAt(0).toUpperCase() + t.slice(1);
  return /[.!?…]$/.test(cased) ? cased : `${cased}.`;
};

/** The first human-readable part of a reason: everything before a JSON body, cut at a
 *  sentence boundary and a sane length. */
function plainPart(reason: string): string {
  const beforeJson = reason.split(/[{[]/)[0] ?? "";
  const trimmed = beforeJson.replace(/[\s:–—-]+$/, "").trim();
  const firstSentence = trimmed.match(/^.*?[.!?](?=\s|$)/)?.[0] ?? trimmed;
  return firstSentence.length > 160 ? `${firstSentence.slice(0, 159)}…` : firstSentence;
}

/** Explain a model-router reason in one sentence. Exported for the tests. */
export function explainReason(reason: string): Pick<Failure, "summary" | "fix"> {
  const r = reason.trim();

  // "Add an API key for OpenAI in Settings → AI Models." The tab has been called Models
  // for a while; the sentence still names the old place, so it is rewritten rather than
  // repeated.
  const noKey = r.match(/^Add an API key for (.+?) in Settings/i);
  if (noKey) return { summary: `There's no API key for ${noKey[1]} yet.`, fix: "models" };

  // "Ollama (gemma4:12b) returned an empty response."
  const empty = r.match(/^(.+?) \(([^)]+)\) returned an empty response/i);
  if (empty) return { summary: `${empty[1]} sent back an empty reply from ${empty[2]}.`, fix: "models" };

  // "Codex: Codex failed without saying why."
  if (/failed without saying why/i.test(r)) {
    const who = r.match(/^([A-Z][\w .]*?)(?::|\s+failed)/)?.[1] ?? "The model";
    return { summary: `${who} stopped without saying why.`, fix: "models" };
  }

  // "<Provider> error <code> <status text>: <body>"
  const http = r.match(/^(.+?) error (\d{3})\b/);
  if (http) {
    const [, provider, codeText] = http;
    const code = Number(codeText);
    if (/credit|payment|billing|quota|insufficient/i.test(r))
      return { summary: `${provider} refused the request: the account is out of credit.`, fix: "models" };
    if (/timed out waiting for llama-server/i.test(r))
      return { summary: `The local model didn't start in time. Try again once it has loaded.` };
    if (code === 401 || code === 403) return { summary: `${provider} rejected the API key.`, fix: "models" };
    if (code === 404) return { summary: `${provider} doesn't recognise that model.`, fix: "models" };
    if (code === 429) return { summary: `${provider} is limiting requests right now. Try again in a moment.` };
    if (code >= 500) return { summary: `${provider} had a problem on its side (${code}). Try again in a moment.` };
    return { summary: `${provider} returned an error (${code}).` };
  }

  const plain = plainPart(r);
  return { summary: plain ? sentence(plain) : "Something went wrong, and no reason was given." };
}

/** A memory answer that is really a failure, or null for a real answer. */
export function brainFailure(text: string): Failure | null {
  if (!text.startsWith(BRAIN_FAILURE_PREFIX)) return null;
  const detail = text.slice(BRAIN_FAILURE_PREFIX.length).trim();
  return { title: "Couldn't search your memory", detail, ...explainReason(detail) };
}

/** A persisted or live error notice ("Error: <reason>"). */
export function noticeFailure(text: string): Failure {
  const detail = text.replace(/^Error:\s*/, "").trim();
  return { title: "This reply didn't finish", detail, ...explainReason(detail) };
}
