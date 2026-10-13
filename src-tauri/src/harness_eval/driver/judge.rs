use super::*;

    pub(super) async fn judge(model: &str, local_only: bool, prompt: &str, rec: &TurnRecord) -> Result<String, String> {
        judge_allowed(model, local_only)?;
        use crate::core::model_router::{self, ChatMessage};
        const SYSTEM: &str = "You check whether an assistant's answer is supported by what its tools returned. \
            Judge only support, not style or completeness. Reply with exactly one line: \
            SUPPORTED, or UNSUPPORTED: followed by the first claim in the answer that the tool results do not support.";
        let evidence = if rec.calls.is_empty() {
            "(no tools were called)".to_string()
        } else {
            rec.calls
                .iter()
                .map(|c| format!("{}({}):\n{}", c.name, c.args.chars().take(200).collect::<String>(), c.result.chars().take(1500).collect::<String>()))
                .collect::<Vec<_>>()
                .join("\n---\n")
        };
        let mut s = crate::core::settings::Settings::default();
        s.local_only = local_only;
        let msgs = vec![
            ChatMessage { role: "system".into(), content: SYSTEM.into(), images: vec![], docs: vec![] },
            ChatMessage {
                role: "user".into(),
                content: format!("QUESTION:\n{prompt}\n\nTOOL RESULTS:\n{evidence}\n\nANSWER:\n{}", rec.answer),
                images: vec![],
                docs: vec![],
            },
        ];
        let (text, _) = model_router::complete_on(&s, &msgs, model).await.map_err(|e| format!("unreachable: {e}"))?;
        text.lines().map(str::trim).find(|l| !l.is_empty()).map(str::to_string).ok_or_else(|| "an empty reply".to_string())
    }

    #[test]
    pub(super) fn the_judge_obeys_the_local_only_switch() {
        // Off: any model may judge.
        assert!(judge_allowed("gemma4:12b", false).is_ok());
        // On: an Ollama tag is local and allowed.
        assert!(judge_allowed("gemma4:12b", true).is_ok());
        // On: a model a provider claims is cloud and refused, before any network.
        // Checked against whichever of these ids the registry actually claims, so the
        // test does not depend on the provider list's spelling of the day.
        let cloud = ["claude-sonnet-5", "anthropic:claude-sonnet-5", "openai:gpt-5.6", "gpt-5.6"]
            .into_iter()
            .find(|id| crate::core::providers::resolve(id).is_some_and(|p| !matches!(p.kind, crate::core::providers::ProviderKind::Ollama)));
        if let Some(id) = cloud {
            let err = judge_allowed(id, true).unwrap_err();
            assert!(err.contains("local-only"), "{err}");
        }
    }
