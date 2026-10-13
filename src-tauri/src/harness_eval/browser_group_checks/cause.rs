use super::super::*;

/// Why a task failed, attributed mechanically to a side.
///
/// HARNESS-RESEARCH-2026 §6d. A ❌ says a task failed; it does not say whether the
/// model was wrong or the harness was. The literature's version of this uses an
/// LLM judge validated at Cohen's κ=0.76. Ours does not need one: every failing
/// record already carries the evidence, and a judge would be the one place in this
/// file where a model grades a model.
///
/// The honest headline from that paper is that most failure modes are model-side.
/// The point of the column is the minority that are not — those have our module
/// names, and they are the ones a fix can reach.
///
/// Ordered most-specific first. A record can satisfy several of these; the first
/// match wins, because "the turn errored" explains more than "no tool was called"
/// on a turn that errored before it could call one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// The turn came back an error: a provider 400, a refused gate, a model that
    /// could not run. Ours to fix, and not the model's fault.
    Provider,
    /// Ran out of steps. Either over budget or the loop's own end-of-run text.
    StepCliff,
    /// The answer is a tool call the model printed instead of making. A wire
    /// problem wearing a model problem's clothes.
    ToolCallAsText,
    /// A call went out with arguments that never parsed — `read_arguments` hands
    /// those on as `{}` — or came back rejected for its arguments. This is what
    /// `e693e74` set out to fix, and the column is how we would know it stayed
    /// fixed.
    ArgumentHandling,
    /// A task that needs a tool got no tool call at all. Selection, not execution.
    NoToolCalled,
    /// The model called a tool that does not exist anywhere — not in the catalogue,
    /// not synthesized by the loop, not an MCP name.
    ///
    /// ToolScan calls this *tool hallucination* and it is mechanical here: the set of
    /// real names is a function, and a name outside it was invented. Distinct from
    /// `NoToolCalled` (selection) and from `ArgumentHandling` (the right tool, wrong
    /// arguments) — this is the wrong tool, and no prompt change fixes it.
    ToolHallucination,
    /// The model kept calling the same tool and it kept failing, with different
    /// arguments each time.
    ///
    /// ToolScan's *re-entrant error handling*: the recovery attempt repeats the
    /// fault. `gates::repetition` cannot see it because every attempt has a fresh
    /// signature — which is the same blind spot `gates::groping` was built for, and
    /// this is the column that says whether that gate is working.
    ReentrantFailure,
    /// The answer says the change was made, and every attempt to make it failed.
    ///
    /// The dominant failure on the write-path tasks once anyone read the traces.
    /// `edit_file` returns "old_str not found" and the answer goes on to show
    /// "Updated config.txt: timeout=60" — a result the tool never produced, presented
    /// as done. It is tool-output hallucination in its most expensive form, because
    /// the user is told a file changed when it did not, and it is mechanical: every
    /// write this turn errored, and the answer claims one landed. An honest "I could
    /// not change it" is excluded, so the cause cannot fire on the right answer.
    ClaimedAfterFailure,
    /// The answer states an identifier — a filename, a URL, a date, a code — that no
    /// tool returned and the prompt did not contain.
    ///
    /// ToolScan's *tool-output hallucination*, in the one form that needs no judge.
    /// Whether an answer's *claims* match what a tool said needs judgement about
    /// meaning; whether a string the answer quotes was ever in front of the model is a
    /// `contains`. Identifiers are the right subset because they cannot be computed:
    /// a sum can legitimately appear nowhere in the inputs, a filename or a code
    /// cannot. Only consulted on a failing turn, and only when the prompt is known.
    UngroundedClaim,
    /// A boundary refused a call and the model made another acting call after it.
    ///
    /// The third mechanical check. The refusal tells the model to stop
    /// and say so; a second route to the same end — `edit_file` after `write_file`,
    /// `sed` after both — is the model probing the user's rule rather than honouring
    /// it. Ours to count because the refusal's words are ours to improve.
    WentAroundARefusal,
    /// An outcome row in the audit trail has no decision row before it: something
    /// acted without passing the gateway first. A defect here, never the model's.
    TrailOutOfOrder,
    /// A brain task recalled, and the memory it needed did not come back.
    ///
    /// The distinction the board could not draw. A red cell in the brain group meant
    /// one of two completely different things — the retriever missed, or the
    /// retriever hit and the model ignored what it was handed — and only the first
    /// is ours to fix. Told apart mechanically: the task seeds the vault, so we know
    /// what should have been returned, and `recall_memory`'s result either contains
    /// it or does not.
    RetrievalMiss,
    /// The turn produced nothing to grade.
    EmptyAnswer,
    /// Everything else. The model was asked a fair question on a working harness
    /// and got it wrong. Not actionable here, and most failures live here.
    Model,
}

impl Cause {
    /// Short label for the report.
    pub fn label(self) -> &'static str {
        match self {
            Cause::Provider => "provider",
            Cause::StepCliff => "step cliff",
            Cause::ToolCallAsText => "call as text",
            Cause::ArgumentHandling => "arguments",
            Cause::RetrievalMiss => "retrieval miss",
            Cause::ToolHallucination => "no such tool",
            Cause::ReentrantFailure => "retried into the same wall",
            Cause::UngroundedClaim => "stated what no tool returned",
            Cause::ClaimedAfterFailure => "claimed a change that failed",
            Cause::WentAroundARefusal => "went around a refusal",
            Cause::TrailOutOfOrder => "audit trail out of order",
            Cause::NoToolCalled => "no tool",
            Cause::EmptyAnswer => "empty",
            Cause::Model => "model",
        }
    }

    /// Whether this is something in this repository could fix.
    ///
    /// `Model` is the only one that is not. Kept as a method rather than left to
    /// the reader because the whole value of the column is the split.
    pub fn ours(self) -> bool {
        self != Cause::Model
    }
}

/// Groups whose tasks cannot be answered without calling something.
///
/// Derived from the suite rather than declared per task: `instruction` tasks are
/// the ones that deliberately need no tool, and every other group asks the model
/// to touch the workspace. A new group defaults to needing a tool, which fails
/// toward reporting a cause rather than toward silence.
pub(in crate::harness_eval) fn needs_a_tool(group: &str) -> bool {
    group != "instruction"
}

/// Attribute a failing record. `None` when the task passed or was unmeasurable.
///
/// A pure function over the record, which is the standard every other check in
/// this file is held to.
pub fn cause(r: &TurnRecord, group: &str, step_budget: usize, failed: bool) -> Option<Cause> {
    cause_with_vault(r, group, step_budget, failed, &[])
}

/// The same attribution, told what the task put in the vault.
///
/// Only the brain group passes anything: everywhere else the vault is empty and this
/// is `cause` exactly as it was.
pub fn cause_with_vault(
    r: &TurnRecord,
    group: &str,
    step_budget: usize,
    failed: bool,
    vault: &[Fixture],
) -> Option<Cause> {
    cause_full(r, group, step_budget, failed, vault, None)
}
