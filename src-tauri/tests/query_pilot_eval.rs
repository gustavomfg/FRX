//! Experimental: o piloto foi encerrado sem execução (ver `docs/IA-SEARCH.md`).
//!
//! Piloto de reescrita de consulta. Mede só linhas com `labeledBy: "human"`:
//! exemplos e TODOs nunca entram nas taxas nem na decisão.
//!
//! `#[ignore]` é fixo em tempo de compilação, então a execução manual é sempre
//! explícita e o CI nunca a roda. Abaixo de `MIN_FOR_RATES` entradas, o relatório
//! só mostra a estrutura.

mod ai_query_pilot_eval {
    use std::collections::BTreeMap;
    use std::path::Path;

    use frx_lib::ai::models::*;
    use frx_lib::ai::query::*;
    use serde::Deserialize;

    const PILOT_SIZE: usize = 50;
    const MIN_FOR_RATES: usize = 30;
    /// O piloto foi encerrado com 47 entradas. As 3 que faltam não serão preenchidas.
    const FROZEN_ENTRIES: usize = 47;
    const PILOT: &str = include_str!("fixtures/query_pilot.jsonl");

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PilotSystem {
        gpu_model: Option<String>,
        driver: Option<String>,
        proton: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PilotEntry {
        id: String,
        category: String,
        #[serde(default)]
        todo: bool,
        labeled_by: Option<String>,
        symptom: Option<String>,
        appid: Option<u32>,
        game_name: Option<String>,
        system: Option<PilotSystem>,
        #[serde(default)]
        expected: BTreeMap<String, Vec<String>>,
    }

    impl PilotEntry {
        fn is_human(&self) -> bool {
            !self.todo && self.labeled_by.as_deref() == Some("human")
        }

        fn is_example(&self) -> bool {
            !self.todo && self.labeled_by.as_deref() == Some("example")
        }

        fn input(&self) -> QueryInput {
            let system = self.system.as_ref();
            let name = system.and_then(|s| s.gpu_model.clone());
            let driver = system.and_then(|s| s.driver.clone());
            let mesa = driver
                .as_deref()
                .and_then(|d| d.strip_prefix("Mesa "))
                .map(str::to_owned);
            let low = name.as_deref().unwrap_or("").to_lowercase();
            let (vendor, module) = if low.contains("rtx") || low.contains("gtx") {
                ("nvidia", Some("nvidia"))
            } else if low.contains("radeon") || low.contains("rx ") {
                ("amd", Some("amdgpu"))
            } else if low.contains("arc") || low.contains("uhd") || low.contains("iris") {
                ("intel", Some("i915"))
            } else {
                ("other", None)
            };
            let gpu = name.as_ref().map(|n| {
                serde_json::from_value::<GpuInfo>(serde_json::json!({
                    "vendor": vendor, "vendorId": "0", "deviceId": "0", "name": n,
                    "driver": module, "driverVersion": driver, "bootVga": true
                }))
                .unwrap()
            });
            QueryInput {
                symptom: self.symptom.clone().unwrap_or_default(),
                appid: self.appid.unwrap_or(0),
                game: GameInfo {
                    name: self.game_name.clone().unwrap_or_default(),
                    proton_tool: system.and_then(|s| s.proton.clone()),
                },
                system: SystemSnapshot {
                    gpus: vec![],
                    primary_gpu: gpu,
                    vulkan_drivers: vec![],
                    mesa_version: mesa,
                },
            }
        }
    }

    fn entries() -> Vec<PilotEntry> {
        PILOT
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l}: {e}")))
            .collect()
    }

    /// Token inteiro, sem sinônimo: "crash" não casa com "crashes", e "RTX 4060"
    /// exige as duas palavras em sequência.
    fn contains_token(query: &str, token: &str) -> bool {
        let query = tokens(query);
        let token = tokens(token);
        !token.is_empty() && query.windows(token.len()).any(|w| w == token)
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    struct Tally {
        any: usize,
        all: usize,
        total: usize,
    }

    impl Tally {
        fn add(&mut self, query: Option<&str>, expected: &[String]) {
            let hits = expected
                .iter()
                .filter(|t| query.is_some_and(|q| contains_token(q, t)))
                .count();
            self.total += 1;
            self.any += usize::from(hits >= 1);
            self.all += usize::from(!expected.is_empty() && hits == expected.len());
        }

        fn row(&self, label: &str) -> String {
            let rate = |n: usize| n as f64 / self.total.max(1) as f64;
            format!(
                "| {label} | {}/{} ({:.2}) | {}/{} ({:.2}) |\n",
                self.any,
                self.total,
                rate(self.any),
                self.all,
                self.total,
                rate(self.all)
            )
        }
    }

    #[derive(Debug, Default)]
    struct Scores {
        overall: Tally,
        by_provider: BTreeMap<String, Tally>,
        by_category: BTreeMap<String, Tally>,
    }

    fn score_entry(scores: &mut Scores, entry: &PilotEntry, queries: &[RewrittenQuery]) {
        for provider in ProviderKind::ALL {
            let Some(expected) = entry.expected.get(provider.slug()) else {
                continue;
            };
            let query = queries
                .iter()
                .find(|q| q.provider == provider)
                .map(|q| q.query.as_str());
            scores.overall.add(query, expected);
            scores
                .by_provider
                .entry(provider.slug().to_owned())
                .or_default()
                .add(query, expected);
            scores
                .by_category
                .entry(entry.category.clone())
                .or_default()
                .add(query, expected);
        }
    }

    fn score(rewriter: &dyn QueryRewriter, entries: &[&PilotEntry]) -> Scores {
        let mut scores = Scores::default();
        for entry in entries {
            let queries = rewriter.rewrite(&entry.input()).unwrap_or_default();
            score_entry(&mut scores, entry, &queries);
        }
        scores
    }

    #[derive(Debug, PartialEq)]
    enum Decision {
        Deterministic,
        Qwen,
        AwaitingQwen,
        Undecided(&'static str),
    }

    /// As regras da Semana 2, em contagens para evitar erro de arredondamento na
    /// fronteira. A regra da Qwen vem primeiro: a da determinística pressupõe que
    /// a Qwen "não superou". Casos sem regra não são decididos.
    fn decide(deterministic: usize, qwen: Option<usize>, total: usize) -> Decision {
        let reaches = |n: usize| n * 5 >= total * 4;
        let Some(qwen) = qwen else {
            return if reaches(deterministic) {
                Decision::AwaitingQwen
            } else {
                Decision::Undecided("a determinística não chegou a 0.80 e a Qwen não foi medida")
            };
        };
        if reaches(qwen) && qwen > deterministic && (qwen - deterministic) * 10 >= total {
            return Decision::Qwen;
        }
        if reaches(deterministic) {
            return Decision::Deterministic;
        }
        if !reaches(qwen) {
            return Decision::Undecided("nenhuma das duas chegou a 0.80");
        }
        Decision::Undecided("a Qwen chegou a 0.80 sem 10 pontos de vantagem; a regra não cobre")
    }

    fn section(title: &str, scores: &Scores) -> String {
        let head = "| | ≥1 token (decisão) | todos os tokens (informativo) |\n|---|---|---|\n";
        let mut out = format!("## {title}\n\n{head}");
        out += &scores.overall.row("total");
        out += "\n### Por provedor\n\n";
        out += head;
        for (name, tally) in &scores.by_provider {
            out += &tally.row(name);
        }
        out += "\n### Por categoria\n\n";
        out += head;
        for (name, tally) in &scores.by_category {
            out += &tally.row(name);
        }
        out + "\n"
    }

    fn example_section(examples: &[&PilotEntry]) -> String {
        let mut out = String::from(
            "## Exemplos do formato (fora da métrica)\n\nConfere o caminho do harness. Não é resultado.\n\n",
        );
        for entry in examples {
            out += &format!("- `{}`\n", entry.id);
            for q in Deterministic.rewrite(&entry.input()).unwrap_or_default() {
                out += &format!("  - {}: `{}`\n", q.provider.slug(), q.query);
            }
        }
        out + "\n"
    }

    /// Mede a Qwen quando o build a inclui. Devolve também quantas entradas caíram
    /// no fallback: nelas, o número não é da Qwen.
    #[cfg(feature = "qwen-query")]
    fn measure_qwen(entries: &[&PilotEntry]) -> Result<(Scores, usize), String> {
        use frx_lib::ai::qwen::{QwenLocal, RewriteOutcome};
        let qwen = QwenLocal::load().map_err(|e| e.to_string())?;
        let mut scores = Scores::default();
        let mut fallbacks = 0;
        for entry in entries {
            let (queries, outcome) = qwen.rewrite_traced(&entry.input()).unwrap_or((
                vec![],
                RewriteOutcome::Fallback {
                    reason: "erro".into(),
                },
            ));
            fallbacks += usize::from(matches!(outcome, RewriteOutcome::Fallback { .. }));
            score_entry(&mut scores, entry, &queries);
        }
        Ok((scores, fallbacks))
    }

    #[cfg(not(feature = "qwen-query"))]
    fn measure_qwen(_: &[&PilotEntry]) -> Result<(Scores, usize), String> {
        Err("build sem a feature qwen-query".into())
    }

    fn report(entries: &[PilotEntry]) -> String {
        let human: Vec<&PilotEntry> = entries.iter().filter(|e| e.is_human()).collect();
        let examples: Vec<&PilotEntry> = entries.iter().filter(|e| e.is_example()).collect();
        let mut out = format!(
            "# Piloto de reescrita de consulta\n\npiloto: {}/{PILOT_SIZE} preenchidos\n\n",
            human.len()
        );
        if human.len() < MIN_FOR_RATES {
            out += &format!(
                "Sem taxas: são necessárias ao menos {MIN_FOR_RATES} entradas com \
`labeledBy: \"human\"`. Sem número real, não há decisão.\n\n"
            );
            return out + &example_section(&examples);
        }
        let deterministic = score(&Deterministic, &human);
        out += &section("deterministic", &deterministic);
        let qwen = measure_qwen(&human);
        match &qwen {
            Ok((scores, fallbacks)) => {
                out += &format!(
                    "Qwen: {fallbacks}/{} entradas caíram no fallback determinístico.\n\n",
                    human.len()
                );
                out += &section("qwen-local", scores);
            }
            Err(why) => out += &format!("Qwen não medida: {why}\n\n"),
        }
        if human.len() < PILOT_SIZE {
            out += "## Decisão\n\nPiloto incompleto: sem decisão até as 50 entradas.\n";
            return out;
        }
        let qwen_any = qwen.as_ref().ok().map(|(s, _)| s.overall.any);
        let decision = decide(
            deterministic.overall.any,
            qwen_any,
            deterministic.overall.total,
        );
        out + &format!(
            "## Decisão\n\n{decision:?}\n\nA licença qwen-research precisa de avaliação \
própria antes de a Qwen virar default num produto.\n"
        )
    }

    #[test]
    fn fixture_is_frozen_at_47_entries_with_unique_ids() {
        let all = entries();
        assert_eq!(all.len(), FROZEN_ENTRIES);
        let mut per_category: BTreeMap<&str, usize> = BTreeMap::new();
        let mut ids: Vec<&str> = all.iter().map(|e| e.id.as_str()).collect();
        for entry in &all {
            *per_category.entry(&entry.category).or_default() += 1;
        }
        assert_eq!(per_category.len(), 10);
        assert!(per_category.values().all(|n| *n <= 5), "{per_category:?}");
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), FROZEN_ENTRIES);
    }

    #[test]
    fn filled_entries_are_complete() {
        for entry in entries().iter().filter(|e| !e.todo) {
            assert!(
                matches!(entry.labeled_by.as_deref(), Some("human" | "example")),
                "{}",
                entry.id
            );
            let input = entry.input();
            assert!(
                !input.symptom.is_empty() && input.appid != 0,
                "{}",
                entry.id
            );
            assert!(!input.game.name.is_empty(), "{}", entry.id);
            for provider in ProviderKind::ALL {
                let expected = entry.expected.get(provider.slug());
                assert!(
                    expected.is_some_and(|e| !e.is_empty()),
                    "{} {provider:?}",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn tokens_match_whole_words_without_synonyms() {
        assert!(contains_token("GE-Proton9 crash", "GE-Proton9"));
        assert!(contains_token("1091500 crash RTX 4060", "rtx 4060"));
        assert!(!contains_token("crashes often", "crash"));
        assert!(!contains_token("15800 fps", "580"));
        assert!(!contains_token("RTX 4070", "RTX 4060"));
        assert!(!contains_token("anything", ""));
    }

    #[test]
    fn tally_separates_any_token_from_all_tokens() {
        let expected = ["a".to_owned(), "b".to_owned()];
        let mut tally = Tally::default();
        tally.add(Some("a x"), &expected);
        tally.add(Some("a b"), &expected);
        tally.add(Some("x"), &expected);
        tally.add(None, &expected);
        assert_eq!(
            tally,
            Tally {
                any: 2,
                all: 1,
                total: 4
            }
        );
    }

    #[test]
    fn decision_rules_at_their_boundaries() {
        use Decision::*;
        assert_eq!(decide(40, Some(40), 50), Deterministic);
        assert_eq!(decide(45, Some(30), 50), Deterministic);
        assert_eq!(
            decide(40, Some(44), 50),
            Deterministic,
            "menos de 10 pontos"
        );
        assert_eq!(decide(40, Some(45), 50), Qwen, "exatamente 10 pontos");
        assert_eq!(decide(30, Some(40), 50), Qwen);
        assert_eq!(
            decide(39, None, 50),
            Undecided("a determinística não chegou a 0.80 e a Qwen não foi medida")
        );
        assert_eq!(decide(40, None, 50), AwaitingQwen);
        assert_eq!(
            decide(39, Some(39), 50),
            Undecided("nenhuma das duas chegou a 0.80")
        );
        assert_eq!(
            decide(39, Some(20), 50),
            Undecided("nenhuma das duas chegou a 0.80")
        );
        assert!(matches!(decide(39, Some(41), 50), Undecided(why) if why.contains("não cobre")));
    }

    #[test]
    fn report_shows_no_rate_or_decision_before_human_labels() {
        let all = entries();
        let human = all.iter().filter(|e| e.is_human()).count();
        if human >= MIN_FOR_RATES {
            return;
        }
        let text = report(&all);
        assert!(text.contains(&format!("piloto: {human}/{PILOT_SIZE} preenchidos")));
        assert!(text.contains("Sem taxas"));
        assert!(!text.contains("≥1 token"));
        assert!(!text.contains("Decisão"));
    }

    #[test]
    #[ignore = "execução manual: preencha o piloto antes; mede o pipeline, não a qualidade"]
    fn run_pilot_and_write_report() {
        let all = entries();
        let text = report(&all);
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/query-pilot-report.md");
        std::fs::write(&path, &text).unwrap();
        assert!(path.is_file());
        println!("{text}");
    }
}
