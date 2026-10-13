use super::*;

    fn usage(input: u64, output: u64, read: u64, write: u64) -> TokenUsage {
        TokenUsage { input, output, cache_read: read, cache_write: write, model: None }
    }

    #[test]
    fn a_local_model_is_free_however_its_id_is_spelled() {
        // `gemma4:12b` is the one that caught this: an Ollama tag has a colon in it
        // and looks like `provider:model`, so a prefix list says "unknown" and the
        // dashboard shows a blank cost for a model that demonstrably cost nothing.
        // Observed live on a real turn before this was routed through `providers`.
        for id in ["gemma4:12b", "qwen2.5:7b-instruct", "llama3.2:latest", "ollama:llama3"] {
            assert_eq!(cost_of(id, &usage(7501, 40, 0, 0)), Some(0.0), "{id}");
        }
    }

    #[test]
    fn a_cloud_model_with_no_rate_is_unknown_rather_than_free() {
        // The distinction the module exists to keep: both would render "$0.00" if
        // they shared a representation, and for this one that is a fabrication.
        assert_eq!(cost_of("anthropic:claude-something-unreleased", &usage(1000, 1000, 0, 0)), None);
        assert_eq!(cost_of("openai:gpt-5.5", &usage(1000, 1000, 0, 0)), None);
        assert_eq!(cost_of("", &usage(1000, 1000, 0, 0)), None);
    }

    #[test]
    fn cache_reads_are_a_tenth_and_writes_are_a_quarter_more() {
        // The rates Phase 3 will be judged against. If these drift, every later
        // "we cut cost by N%" claim drifts with them and nothing complains.
        let r = rates_for("claude-opus-5").unwrap();
        assert_eq!(r.input, 5.0);
        assert_eq!(r.cache_read, 0.5);
        assert_eq!(r.cache_write, 6.25);
        // One million of each, so the arithmetic is readable.
        assert_eq!(r.cost(&usage(1_000_000, 0, 0, 0)), 5.0);
        assert_eq!(r.cost(&usage(0, 0, 1_000_000, 0)), 0.5);
        assert_eq!(r.cost(&usage(0, 0, 0, 1_000_000)), 6.25);
    }

    #[test]
    fn the_headline_saving_in_the_plan_is_what_this_table_produces() {
        // HARNESS-PLAN §2.7 claims a 15k-token prefix over 6 steps goes from $0.45
        // to $0.13. That number is the reason Phase 3 is scheduled where it is, so
        // it should be reproducible from the rates rather than trusted.
        let r = rates_for("claude-opus-5").unwrap();
        let uncached = r.cost(&usage(6 * 15_000, 0, 0, 0));
        let cached = r.cost(&usage(0, 0, 5 * 15_000, 15_000));
        assert!((uncached - 0.45).abs() < 0.005, "uncached prefix: {uncached}");
        assert!((cached - 0.13).abs() < 0.005, "cached prefix: {cached}");
        assert!(cached / uncached < 0.30, "the plan claims ~71% off the prefix");
    }

    #[test]
    fn a_dated_id_prices_the_same_as_its_undated_family() {
        // The registry ships both. A table keyed on exact ids would price one and
        // shrug at the other, and the shrug renders as "unknown cost" in the UI.
        assert_eq!(rates_for("claude-haiku-4-5-20251001"), rates_for("claude-haiku-4-5"));
        assert!(rates_for("claude-haiku-4-5-20251001").is_some());
    }

    #[test]
    fn a_provider_prefix_does_not_hide_the_model() {
        // The app's convention is `provider:model`, and `providers::model_name`
        // strips it only when the prefix is a real provider — which is exactly why
        // this can be delegated instead of re-implemented.
        assert_eq!(rates_for("anthropic:claude-opus-5"), rates_for("claude-opus-5"));
        assert_eq!(rates_for("claude-opus-5").unwrap().input, 5.0);
    }

    #[test]
    fn opencodes_free_tier_is_free_and_its_paid_models_are_not_guessed_at() {
        // The suffix is that provider's own convention and the registry curates the
        // ids that carry it. Its flagships are a different question — we have no
        // rates for those, and inventing one would be worse than saying so.
        assert_eq!(cost_of("opencode:nemotron-3.5-lightning-free", &usage(478_827, 34_137, 0, 0)), Some(0.0));
        assert_eq!(cost_of("opencode:some-flagship", &usage(1000, 1000, 0, 0)), None);
    }

    #[test]
    fn an_id_no_provider_claims_is_ollamas_and_therefore_free() {
        // `providers::resolve` returns None for anything it will not route to a
        // cloud provider, and the run path sends those to Ollama. So "unresolved"
        // here means "ran on this machine", not "we have no idea" — which is why
        // the unknown case above is spelled with a real provider prefix on it.
        assert_eq!(rates_for("some-model-nobody-registered"), Some(FREE));
        assert_eq!(rates_for("gemma4:12b"), Some(FREE));
    }

    #[test]
    fn the_longest_matching_prefix_wins() {
        // `claude-opus-5` and `claude-opus-4-7` are priced differently; a shorter
        // entry must never shadow a longer one whatever order the table is in.
        assert_eq!(rates_for("claude-opus-5").unwrap().input, 5.0);
        assert_eq!(rates_for("claude-opus-4-7").unwrap().input, 15.0);
    }
