// The agent's questions, one at a time.
//
// WHAT THIS REPLACED, AND WHY. The first version was a form of empty fields: the
// agent named what it needed, the user typed it in. Wrong shape. The agent has just
// read the files and knows what the plausible answers are; a blank box hands that
// thinking back to the person who asked it to do the work. So every question arrives
// with the answers the agent believes are likely, each with a line saying what
// choosing it would mean — answerable in one click, with an "Other" box underneath
// for when the list is wrong.
//
// Writing those options is the part that does the real work. `core::ask` refuses a
// question that has none, because a model that cannot name two candidate answers has
// not understood the question well enough to be asking it.
//
// Number keys select, Enter advances, Skip leaves one unanswered — and the model is
// told it was skipped, so it does not quietly fill the gap in.

export interface AskChoice {
  label: string;
  description?: string;
}
export interface AskQuestion {
  question: string;
  options: AskChoice[];
  multi?: boolean;
}

/** Sentinel for the free-text row. Leading space, so it can never equal a real label. */
export const OTHER = " other";
