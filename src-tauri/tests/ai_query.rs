//! Reescrita de consulta ligada aos provedores mockados e ao ranker, sem rede.

mod ai_query {
    use frx_lib::ai::models::*;
    use frx_lib::ai::query::*;
    use frx_lib::ai::{research_mocked, research_with};

    const NOW: u64 = 1_793_491_200;

    fn request(symptom: &str) -> ResearchQuery {
        let gpu: GpuInfo = serde_json::from_value(serde_json::json!({
            "vendor": "nvidia", "vendorId": "10de", "deviceId": "0",
            "name": "NVIDIA GeForce RTX 4060", "driver": "nvidia",
            "driverVersion": "580.65.06", "bootVga": true
        }))
        .unwrap();
        ResearchQuery {
            symptom: symptom.into(),
            appid: 1091500,
            system: SystemSnapshot {
                gpus: vec![gpu.clone()],
                primary_gpu: Some(gpu),
                vulkan_drivers: vec![],
                mesa_version: None,
            },
            game: GameInfo {
                name: "Cyberpunk 2077".into(),
                proton_tool: Some("GE-Proton9-20".into()),
            },
        }
    }

    #[test]
    fn research_still_returns_ranked_mock_candidates() {
        let out = research_mocked(&request("crash on launch"), NOW).unwrap();
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|c| c.candidate.title.starts_with("[MOCK]")));
        assert!(out.windows(2).all(|w| w[0].relevance >= w[1].relevance));
    }

    #[test]
    fn query_input_uses_the_cleaned_symptom() {
        let input = QueryInput::from(&request("crash\u{0}\nno menu"));
        assert!(!input.symptom.chars().any(char::is_control));
        assert_eq!(input.appid, 1091500);
    }

    #[test]
    fn rewritten_query_serializes_with_kebab_case_provider() {
        let out = Deterministic
            .rewrite(&QueryInput::from(&request("crash")))
            .unwrap();
        let json = serde_json::to_value(&out[2]).unwrap();
        assert_eq!(json["provider"], "release-notes");
        assert_eq!(json["query"], "GE-Proton9 crash");
        assert!(json["intent"]["goal"].is_string());
        assert!(json["intent"]["keywords"].is_array());
    }

    /// Um reescritor falho interrompe a pesquisa com o erro dele.
    struct Failing;

    impl QueryRewriter for Failing {
        fn rewrite(&self, _: &QueryInput) -> Result<Vec<RewrittenQuery>, QueryError> {
            Err(QueryError::Model("modelo ausente".into()))
        }

        fn id(&self) -> &'static str {
            "failing"
        }
    }

    #[test]
    fn research_reports_the_rewriter_error() {
        let e = research_with(&request("crash"), &Failing, NOW).unwrap_err();
        assert_eq!(e.to_string(), "modelo ausente");
    }

    /// Só o provedor consultado devolve candidatos.
    struct OnlyReleaseNotes;

    impl QueryRewriter for OnlyReleaseNotes {
        fn rewrite(&self, _: &QueryInput) -> Result<Vec<RewrittenQuery>, QueryError> {
            Ok(vec![RewrittenQuery {
                provider: ProviderKind::ReleaseNotes,
                query: "GE-Proton9 crash".into(),
                intent: QueryIntent {
                    goal: "teste".into(),
                    keywords: vec![],
                },
            }])
        }

        fn id(&self) -> &'static str {
            "only-release-notes"
        }
    }

    #[test]
    fn candidates_come_only_from_providers_that_got_a_query() {
        let out = research_with(&request("crash"), &OnlyReleaseNotes, NOW).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].candidate.source, SourceKind::ReleaseNotes);
    }
}
