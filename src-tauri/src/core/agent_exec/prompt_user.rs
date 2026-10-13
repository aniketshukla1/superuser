use super::*;

/// How long a prompt waits before the run gives up on being answered.
///
/// Half an hour, because the thing being waited on is a person — someone who
/// stepped away from a running turn should be able to come back to it, and a
/// two-minute timeout would make asking useless in exactly the case where asking
/// matters most. It is bounded at all so a forgotten window cannot pin a turn open
/// forever; Stop always ends the wait sooner.
pub(super) const ASK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Put something to the user and wait for the reply.
///
/// Split out from `ask_user` so that everything a blocking prompt has to get right
/// — parking, the unattended path, honouring Stop, the timeout, and refusing to wait
/// when nobody is attached — sits in one place rather than being re-derived by the
/// next tool that needs to stop and ask. `kind` is the event the GUI dispatches on.
///
/// The return value IS the answer, which is what makes these tools different from
/// every other one here: they have to reach a human and come back before the model
/// takes its next step. See [`crate::core::ask`] for the rendezvous, and for why
/// this was worth building — the version that didn't wait taught assistants that
/// checking first was a waste of a turn.
pub(super) async fn prompt_user(
    ctx: &ToolCtx<'_>,
    kind: &str,
    summary: &str,
    payload: Value,
    raw_args: &Value,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;

    // Nobody is watching a scheduled run, so there is no answer coming. Park it
    // where the user will find it and tell the model to stop rather than assume —
    // an unattended run that guesses is the one case where a wrong guess acts
    // unsupervised.
    if ctx.unattended {
        let ask = crate::core::inbox::Inbox::load(&ctx.data_dir).park(crate::core::inbox::Ask::new(
            &ctx.session_id,
            ctx.task_id.clone(),
            "ask_user",
            "user",
            summary,
            raw_args.clone(),
        ));
        return Err(format!(
            "No one is attending this run, so what you asked is waiting in the user's Inbox (id {}). \
             You will not get an answer during this run. Do not guess it and do not carry on as if you \
             had one — finish by saying what you need and what you would do with each answer.",
            ask.id
        ));
    }

    // What to say when there is nobody to put this in front of — and it has to say
    // *what* could not be put in front of them.
    //
    // This was one sentence for every kind, about questions, and it misled. Read off
    // the write-path traces: `run_shell("sed -i …")` needs approval, the eval run has no
    // socket, and the model was told "a question cannot reach anyone" about a call
    // that asked no question. It then reasoned about a question it had not asked. The
    // refusal was right; the words described a different event.
    let unattached = || -> String {
        match kind {
            "permission" => format!(
                "error: this needs the user's approval ({summary}) and nothing is attached to this run to \
                 ask them, so it did not run and nothing changed. Say in your answer what you wanted to do \
                 and why; do not describe it as done."
            ),
            _ => "error: nothing is attached to this run, so a question cannot reach anyone. \
                  Say what you need in your answer instead of guessing at it."
                .to_string(),
        }
    };

    let Some(events) = &ctx.events else { return Err(unattached()) };

    let rx = crate::core::ask::park(&ctx.session_id);
    let mut frame = payload;
    frame["type"] = serde_json::json!(kind);
    if events.send(frame).is_err() {
        crate::core::ask::abandon(&ctx.session_id);
        return Err(unattached());
    }

    // Wait — but never unconditionally. Stop has to end the wait, and a window left
    // open overnight has to end it too.
    let cancel = crate::commands::session_cancel_flag(&ctx.session_id);
    let deadline = std::time::Instant::now() + ASK_TIMEOUT;
    tokio::pin!(rx);
    loop {
        tokio::select! {
            answered = &mut rx => {
                return match answered {
                    Ok(a) if !a.trim().is_empty() => Ok(a.trim().to_string()),
                    // The sender was dropped: either this run asked something else
                    // (which replaced this) or the turn is being torn down. Either way
                    // there is no answer, and reporting one would be a fabrication.
                    _ => Err("No answer came back — the question was replaced or the turn ended. Do not assume one.".into()),
                };
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                if cancel.load(Ordering::SeqCst) {
                    crate::core::ask::abandon(&ctx.session_id);
                    return Err("The user stopped this turn before answering. Stop here.".into());
                }
                if std::time::Instant::now() >= deadline {
                    crate::core::ask::abandon(&ctx.session_id);
                    return Err(
                        "No answer after 30 minutes. Do not guess and do not proceed as if you had one — \
                         end your turn by saying what you asked and what you need it for."
                            .into(),
                    );
                }
            }
        }
    }
}

/// Put the agent's questions to the user and wait for the answers.
pub(super) async fn ask_user(ctx: &ToolCtx<'_>, args: &Value) -> String {
    let ask = match crate::core::ask::Ask::parse(args) {
        Ok(a) => a,
        Err(e) => return e,
    };
    match prompt_user(ctx, "question", &ask.summary(), ask.payload(), args).await {
        Ok(reply) => ask.read_answer(&reply),
        Err(e) => e,
    }
}

/// Put a short form to the user and wait for the values.
///
/// Rides the same rendezvous as `ask_user` — the run parks on one receiver either
/// way, and the tool that parked it knows how to read what comes back. The card
/// answers with the same `question_response` frame, so this adds no socket surface.
pub(super) async fn ask_form(ctx: &ToolCtx<'_>, args: &Value) -> String {
    let form = match crate::core::ask::Form::parse(args) {
        Ok(f) => f,
        Err(e) => return e,
    };
    match prompt_user(ctx, "form", &form.summary(), form.payload(), args).await {
        Ok(reply) => form.read_answer(&reply),
        Err(e) => e,
    }
}

/// What the auto-approve reviewer decided about one proposed action.
pub(super) enum Reviewed {
    /// Skip the card and run it.
    Allow,
    /// Put it to the human, carrying this note onto the card.
    Human(Option<String>),
}
