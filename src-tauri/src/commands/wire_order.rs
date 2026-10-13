use crate::core::agent::ToolSpec;

    fn spec(name: &str) -> ToolSpec {
        ToolSpec {
            name: name.to_string(),
            description: format!("does {name}"),
            schema: serde_json::json!({ "type": "object", "properties": {} }),
            capability: "files".into(),
            risk: crate::core::agent::RiskClass::Read,
        }
    }

    /// The Anthropic path has asserted this for a while; the local path is where a
    /// miss actually hurts, and it was the one going out unsorted.
    #[test]
    fn the_ollama_tool_array_does_not_depend_on_capability_expansion_order() {
        let forward = [spec("write_file"), spec("ask_user"), spec("read_file")];
        let backward = [spec("read_file"), spec("write_file"), spec("ask_user")];
        let names = |v: &[serde_json::Value]| -> Vec<String> {
            v.iter().map(|t| t["function"]["name"].as_str().unwrap_or("").to_string()).collect()
        };
        assert_eq!(
            names(&super::tools_ollama(&forward)),
            names(&super::tools_ollama(&backward)),
            "two orderings of the same tools must produce the same wire array — \
             otherwise the prompt cache misses and a local turn pays a full prefill"
        );
        assert_eq!(names(&super::tools_ollama(&forward)), ["ask_user", "read_file", "write_file"]);
    }
