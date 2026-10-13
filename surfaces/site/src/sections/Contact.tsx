import { motion, useReducedMotion } from "framer-motion";
import { useRef, useState, type FormEvent } from "react";
import { CONTACT_ENDPOINT } from "../config";
import { onScroll, rise, still } from "../motion";

// Business enquiries. The free licence covers personal use, so this is where a company
// asks for anything else. The form posts to Formspree (see config.ts), which forwards to
// the maintainer: their address is in that account, never in this page or the repository.

const TOPICS = ["A business licence", "Partnership or integration", "Press", "Something else"];

type Status =
  | { kind: "idle" }
  | { kind: "sending" }
  | { kind: "sent"; email: string }
  | { kind: "failed"; message: string };

type Field = "name" | "email" | "message";

function check(data: Record<string, string>): Partial<Record<Field, string>> {
  const errors: Partial<Record<Field, string>> = {};
  if (!data.name.trim()) errors.name = "Add your name, so the reply can use it.";
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(data.email.trim())) errors.email = "Add an email address the reply can reach.";
  if (data.message.trim().length < 20) errors.message = "Add a sentence or two about what you need.";
  return errors;
}

export function Contact() {
  const reduced = useReducedMotion() ?? false;
  const v = reduced ? still : rise;
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [errors, setErrors] = useState<Partial<Record<Field, string>>>({});
  const done = useRef<HTMLDivElement>(null);

  const submit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = e.currentTarget;
    const raw = new FormData(form);
    const data = {
      name: String(raw.get("name") ?? ""),
      email: String(raw.get("email") ?? ""),
      company: String(raw.get("company") ?? ""),
      topic: String(raw.get("topic") ?? TOPICS[0]),
      message: String(raw.get("message") ?? ""),
      trap: String(raw.get("_gotcha") ?? ""),
    };

    const found = check(data);
    setErrors(found);
    const first = (Object.keys(found) as Field[])[0];
    if (first) {
      form.querySelector<HTMLElement>(`[name="${first}"]`)?.focus();
      return;
    }
    // Only a bot fills the hidden field. It is told it worked and nothing is sent.
    if (data.trap) {
      setStatus({ kind: "sent", email: data.email.trim() });
      return;
    }
    if (!CONTACT_ENDPOINT) {
      setStatus({ kind: "failed", message: "This form isn't connected yet. Please try again soon." });
      return;
    }

    setStatus({ kind: "sending" });
    try {
      const res = await fetch(CONTACT_ENDPOINT, {
        method: "POST",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify({
          name: data.name.trim(),
          email: data.email.trim(),
          company: data.company.trim(),
          topic: data.topic,
          message: data.message.trim(),
          _subject: `Superuser: ${data.topic.toLowerCase()} (${data.name.trim()})`,
        }),
      });
      if (res.ok) {
        setStatus({ kind: "sent", email: data.email.trim() });
        form.reset();
        requestAnimationFrame(() => done.current?.focus());
        return;
      }
      const body = await res.json().catch(() => null);
      const why: string | undefined = body?.errors?.[0]?.message;
      setStatus({
        kind: "failed",
        message: why ? `That didn't send: ${why.replace(/\.$/, "")}.` : "That didn't send. Please try again in a minute.",
      });
    } catch {
      setStatus({ kind: "failed", message: "Couldn't reach the form service. Check your connection and try again." });
    }
  };

  const fieldProps = (name: Field) => ({
    name,
    id: `contact-${name}`,
    "aria-invalid": errors[name] ? true : undefined,
    "aria-describedby": errors[name] ? `contact-${name}-error` : undefined,
    onInput: () => errors[name] && setErrors((e) => ({ ...e, [name]: undefined })),
  });

  return (
    <section id="contact" className="band">
      <div className="wrap contact">
        <motion.div {...onScroll} variants={v} className="section-head">
          <p className="eyebrow">Business use</p>
          <h2>Using Superuser at work?</h2>
          <p className="lead">
            The free licence covers personal use. For a business licence, a partnership, press or
            anything else, write here. It goes straight to the maintainer, and only to them.
          </p>
          <ul className="contact-points">
            <li>Say what your team would use it for, and roughly how many people.</li>
            <li>Mention any constraint that matters: on-premises, a model you must use, an audit you have to pass.</li>
          </ul>
        </motion.div>

        <motion.div {...onScroll} variants={v} className="form-card">
          {status.kind === "sent" ? (
            <div className="form-done" ref={done} tabIndex={-1} role="status">
              <h3>Thanks. It&rsquo;s on its way.</h3>
              <p>
                The reply will come to <strong>{status.email}</strong>.
              </p>
              <button type="button" className="btn btn-ghost" onClick={() => setStatus({ kind: "idle" })}>
                Send another message
              </button>
            </div>
          ) : (
            <form onSubmit={submit} noValidate>
              <div className="field-row">
                <div className="field">
                  <label htmlFor="contact-name">Name</label>
                  <input type="text" autoComplete="name" {...fieldProps("name")} />
                  {errors.name && (
                    <p className="field-error" id="contact-name-error">
                      {errors.name}
                    </p>
                  )}
                </div>
                <div className="field">
                  <label htmlFor="contact-email">Work email</label>
                  <input type="email" autoComplete="email" inputMode="email" {...fieldProps("email")} />
                  {errors.email && (
                    <p className="field-error" id="contact-email-error">
                      {errors.email}
                    </p>
                  )}
                </div>
              </div>

              <div className="field-row">
                <div className="field">
                  <label htmlFor="contact-company">
                    Company <span className="optional">optional</span>
                  </label>
                  <input type="text" name="company" id="contact-company" autoComplete="organization" />
                </div>
                <div className="field">
                  <label htmlFor="contact-topic">What it&rsquo;s about</label>
                  <select name="topic" id="contact-topic" defaultValue={TOPICS[0]}>
                    {TOPICS.map((t) => (
                      <option key={t}>{t}</option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="field">
                <label htmlFor="contact-message">Message</label>
                <textarea rows={5} {...fieldProps("message")} />
                {errors.message && (
                  <p className="field-error" id="contact-message-error">
                    {errors.message}
                  </p>
                )}
              </div>

              {/* For bots only: hidden from people and from assistive technology. */}
              <div className="trap" aria-hidden="true">
                <label htmlFor="contact-trap">Leave this empty</label>
                <input type="text" name="_gotcha" id="contact-trap" tabIndex={-1} autoComplete="off" />
              </div>

              <div className="form-foot">
                <button type="submit" className="btn btn-primary" disabled={status.kind === "sending"}>
                  {status.kind === "sending" ? "Sending…" : "Send message"}
                </button>
                <p className="form-note">Sent through Formspree to the maintainer, and used only to reply to you.</p>
              </div>
              <p className="form-failed" role="alert">
                {status.kind === "failed" ? status.message : ""}
              </p>
            </form>
          )}
        </motion.div>
      </div>
    </section>
  );
}
