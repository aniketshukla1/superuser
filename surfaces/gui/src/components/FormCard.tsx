/** A short form the agent put to you: values only you hold.
 *
 * The sibling of AskCard, and deliberately narrower. AskCard is for a DECISION and
 * requires options, because writing the options is the agent's work; `core::ask`
 * records that a form of empty fields was tried there and rejected for exactly that
 * reason. This is the other case — a target margin, a start date, an account id —
 * where there is nothing to enumerate and offering "likely answers" would mean
 * inventing them.
 *
 * Everything can be skipped. A required field is marked, and leaving it blank is
 * still allowed: the answer reports it as blank in as many words, which is a truer
 * result than a value the user typed to get past a validator.
 */
import { useMemo, useRef, useState } from "react";

export type FormField = {
  name: string;
  label: string;
  kind?: "text" | "number" | "date";
  placeholder?: string;
  help?: string;
  required?: boolean;
};

export type FormRequest = { title?: string; intro?: string; fields: FormField[] };

export function FormCard({
  form,
  onDone,
  compact,
}: {
  form: FormRequest;
  /** Keyed by field name. Blank entries are sent as "" — the agent is told which
   *  were left blank, rather than being handed a shorter object to read a guess into. */
  onDone: (answers: Record<string, string>) => void;
  compact?: boolean;
}) {
  const fields = useMemo(() => form.fields.filter((f) => f?.name && f?.label), [form.fields]);
  const [values, setValues] = useState<Record<string, string>>({});
  const [sent, setSent] = useState(false);
  const firstRef = useRef<HTMLInputElement>(null);

  if (fields.length === 0) return null;

  const submit = () => {
    if (sent) return;
    setSent(true);
    onDone(Object.fromEntries(fields.map((f) => [f.name, (values[f.name] ?? "").trim()])));
  };

  const missing = fields.filter((f) => f.required !== false && !(values[f.name] ?? "").trim());

  return (
    <form
      className={"formcard" + (compact ? " is-compact" : "")}
      data-testid="formcard"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      {form.title && <div className="formcard-title">{form.title}</div>}
      {form.intro && <p className="formcard-intro">{form.intro}</p>}
      <div className="formcard-fields">
        {fields.map((f, i) => (
          <label className="formcard-field" key={f.name}>
            <span className="formcard-label">
              {f.label}
              {f.required === false && <span className="formcard-optional"> optional</span>}
            </span>
            <input
              ref={i === 0 ? firstRef : undefined}
              className="formcard-input"
              type={f.kind === "number" ? "number" : f.kind === "date" ? "date" : "text"}
              placeholder={f.placeholder || undefined}
              value={values[f.name] ?? ""}
              disabled={sent}
              data-testid={`formcard-input-${f.name}`}
              onChange={(e) => setValues((v) => ({ ...v, [f.name]: e.target.value }))}
            />
            {f.help && <span className="formcard-help">{f.help}</span>}
          </label>
        ))}
      </div>
      <div className="formcard-actions">
        {/* Says what is still empty rather than blocking on it. Blank is a real
            answer — it comes back as "left blank", which beats a number typed to
            get past a validator. */}
        <span className="formcard-note">
          {missing.length > 0 ? `${missing.length} still empty` : ""}
        </span>
        <button className="formcard-send" type="submit" disabled={sent} data-testid="formcard-send">
          {sent ? "Sent" : "Send"}
        </button>
      </div>
    </form>
  );
}
