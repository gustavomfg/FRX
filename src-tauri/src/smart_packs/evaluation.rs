//! Avaliação automática de um teste de pacote.
//!
//! Depois de cada captura, a sessão é comparada com o baseline usando a mesma
//! metodologia do ranking (`evidence`), e o resultado vira uma conclusão
//! legível:
//!
//! - com ≥ 2 sessões comparáveis de cada lado: **melhoria/regressão medida**
//!   ou **sem diferença significativa** (e o ranking passa a considerar);
//! - com 1 sessão em algum lado: o mesmo cálculo, marcado como
//!   **preliminar** ("indício de regressão — repita o teste"). Uma captura só
//!   nunca comprova uma regressão;
//! - condições diferentes da referência: **não foi possível determinar**.
//!
//! O FRX não sabe se o trecho do jogo foi o mesmo: diferença de cena também
//! muda o FPS, e isso aparece nas limitações de toda conclusão.

use std::collections::HashMap;

use super::evidence::{compare, Grouped};
use super::models::*;
use crate::performance::models::SessionMeta;

const SAME_SCENE: &str = "A comparação assume o mesmo trecho do jogo, as mesmas configurações gráficas e duração parecida. Cenas diferentes também mudam o FPS. Duas sessões por configuração são um ponto inicial: a diferença observada não prova causalidade.";

fn label(v: EvalVerdict, preliminary: bool) -> &'static str {
    match (v, preliminary) {
        (EvalVerdict::Regression, true) => "Indício de regressão — recomendado repetir o teste",
        (EvalVerdict::Regression, false) => "Regressão de desempenho medida",
        (EvalVerdict::Improvement, true) => "Indício de melhoria — recomendado repetir o teste",
        (EvalVerdict::Improvement, false) => "Melhoria de desempenho medida",
        (EvalVerdict::NoDifference, true) => "Sem diferença relevante detectada (preliminar)",
        (EvalVerdict::NoDifference, false) => "Sem diferença relevante detectada",
        (EvalVerdict::Inconclusive, _) => "Resultado inconclusivo",
        (EvalVerdict::NotComparable, _) => "Não foi possível determinar a diferença",
        (EvalVerdict::NoBaseline, _) => "Sem referência para comparar",
        (EvalVerdict::Reference, _) => "Referência (baseline) registrada",
    }
}

fn fmt(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

/// Verdito inclui quedas e estabilidade; uma melhora não esconde piora em outra métrica.
pub fn verdict(vs: &VsBaseline) -> EvalVerdict {
    let get = |k: &str| vs.get(k).map(|d| d.outcome);
    let outs: Vec<Outcome> = ["fpsAvg", "fpsLow1", "fpsLow01", "ftP99", "spikesPerMin"]
        .into_iter()
        .filter_map(get)
        .collect();
    if outs.is_empty() {
        return EvalVerdict::Inconclusive;
    }
    let reg = outs.contains(&Outcome::Regression);
    let imp = outs.contains(&Outcome::Improvement);
    match (reg, imp) {
        (true, true) => EvalVerdict::Inconclusive,
        (true, false) => EvalVerdict::Regression,
        (false, true) => EvalVerdict::Improvement,
        _ => EvalVerdict::NoDifference,
    }
}

pub struct Ctx<'a> {
    pub app_id: &'a str,
    pub baseline_id: &'a str,
    pub grouped: &'a Grouped,
    pub metas: &'a HashMap<&'a str, &'a SessionMeta>,
    pub name_of: &'a dyn Fn(&str) -> Option<String>,
}

pub fn evaluate(
    c: &Ctx<'_>,
    session: &SessionMeta,
    config: Option<&PackSnapshot>,
) -> PackEvaluation {
    let mut e = PackEvaluation {
        conditions: session.conditions.clone(),
        app_id: c.app_id.to_owned(),
        pack_id: config.map(|s| s.pack_id.clone()).unwrap_or_default(),
        pack_name: String::new(),
        session_id: session.id.clone(),
        session_label: session.label.clone(),
        captured_at: session
            .captured_at
            .clone()
            .or_else(|| Some(session.created_at.clone())),
        verdict: EvalVerdict::NotComparable,
        preliminary: true,
        headline: String::new(),
        summary: String::new(),
        deltas: vec![],
        baseline_sessions: c
            .grouped
            .measurements
            .get(c.baseline_id)
            .map(|m| m.used())
            .unwrap_or(0),
        pack_sessions: 0,
        limitations: vec![],
        next_step: None,
        ranking_effect: "Não muda o ranking.".into(),
        baseline_id: c.baseline_id.to_owned(),
        current_pack_id: None,
    };
    let done = |mut e: PackEvaluation, v: EvalVerdict, summary: String| {
        e.verdict = v;
        e.headline = label(v, e.preliminary).into();
        e.summary = summary;
        e
    };
    let Some(cfg) = config else {
        return done(e, EvalVerdict::NotComparable, "O FRX não sabe quais correções estavam ativas nesta sessão (importada ou anterior à v0.4). Declare a configuração em Smart Packs para usá-la.".into());
    };
    if session.app_id.as_deref() != Some(c.app_id)
        || !super::benchmark::snapshot_matches_app(c.app_id, cfg)
    {
        return done(e, EvalVerdict::NotComparable, "A sessão e o pacote não pertencem ao mesmo AppID; nenhum resultado é transportado entre jogos.".into());
    }
    e.pack_name = (c.name_of)(&cfg.pack_id).unwrap_or_else(|| {
        if cfg.env.is_empty() {
            "Baseline — sem correções do FRX".into()
        } else {
            cfg.env
                .iter()
                .map(|p| format!("{}={}", p.key, p.value))
                .collect::<Vec<_>>()
                .join(" ")
        }
    });
    let Some(group) = c.grouped.measurements.get(&cfg.pack_id) else {
        return done(
            e,
            EvalVerdict::NotComparable,
            "Sessão de outro jogo ou sem dados de FPS.".into(),
        );
    };
    e.pack_sessions = group.used();
    let me = group.sessions.iter().find(|s| s.id == session.id);
    if me.is_none() {
        return done(
            e,
            EvalVerdict::NotComparable,
            "A sessão não integra as evidências deste jogo.".into(),
        );
    }
    if let Some(r) = me {
        e.limitations.extend(r.notes.clone());
        if let Some(why) = &r.excluded {
            return done(
                e,
                EvalVerdict::NotComparable,
                format!("{why} Esta sessão não entra na comparação."),
            );
        }
    }
    if cfg.source == SnapshotSource::Declared {
        e.limitations
            .push("Configuração declarada por você, não conferida pelo FRX.".into());
    } else if !cfg.verified {
        e.limitations
            .push("Alguma correção não estava confirmada no disco durante a captura.".into());
    }
    if session.stats.duration_s < 30.0 {
        e.limitations.push(format!(
            "Captura curta ({} s): picos e quedas raras podem não aparecer.",
            fmt(session.stats.duration_s, 0)
        ));
    }
    e.limitations.push(SAME_SCENE.into());

    if cfg.pack_id == c.baseline_id {
        e.preliminary = e.pack_sessions < 2;
        e.next_step = (e.pack_sessions < 2).then(|| "Capture mais uma sessão de referência no mesmo trecho: repetições ajudam a estimar a variação entre partidas.".into());
        e.ranking_effect = "Referência para todos os pacotes deste jogo.".into();
        let n = e.pack_sessions;
        return done(
            e,
            EvalVerdict::Reference,
            format!("{n} sessão(ões) de referência comparável(is) registrada(s)."),
        );
    }
    let base = c
        .grouped
        .measurements
        .get(c.baseline_id)
        .filter(|b| b.used() > 0);
    let Some(base) = base else {
        e.next_step = Some("Restaure a configuração anterior e capture o baseline no mesmo trecho; o FRX compara automaticamente.".into());
        e.ranking_effect = "Fica entre os candidatos até existir baseline.".into();
        return done(
            e,
            EvalVerdict::NoBaseline,
            "Ainda não há uma sessão sem correções do FRX para comparar.".into(),
        );
    };
    let vs = compare(base, group, c.metas);
    e.preliminary = vs.limited;
    e.deltas = vs.deltas.clone();
    e.limitations.extend(vs.notes.clone());
    let v = verdict(&vs);
    let avg = vs.get("fpsAvg");
    let low = vs.get("fpsLow1");
    let line = |d: Option<&MetricDelta>| {
        d.map(|d| {
            format!(
                "{} {} → {} {} ({}{}%, limiar ±{}%)",
                d.label,
                fmt(d.before, 1),
                fmt(d.after, 1),
                d.unit,
                if d.delta_pct >= 0.0 { "+" } else { "−" },
                fmt(d.delta_pct.abs(), 1),
                fmt(d.threshold_pct, 1)
            )
        })
    };
    let summary = [line(avg), line(low)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("; ");
    let (bn, pn) = (base.used(), group.used());
    if vs.limited {
        e.next_step = Some(format!(
            "Repita o teste no mesmo trecho: faltam {} sessão(ões) do baseline e {} do pacote para uma conclusão.",
            2usize.saturating_sub(bn),
            2usize.saturating_sub(pn)
        ));
        e.ranking_effect = match v {
            _ if !crate::fix_engine::dynamic::compatibility::performance_warnings(cfg.env.iter().map(|p| (p.key.as_str(),p.value.as_str()))).is_empty() => "Preliminar: a alternativa permanece separada em risco de perda de desempenho; não é sugerida como otimização nem exige novos testes para reconhecer esse risco.".into(),
            EvalVerdict::Regression => "Preliminar: o pacote fica no fim dos candidatos, marcado com indício de regressão, e não entra em combinações. Ainda não vai para “não recomendados”.".into(),
            _ => "Preliminar: ainda não entra no ranking medido (exige 2 sessões comparáveis de cada lado).".into(),
        };
    } else {
        e.ranking_effect = match v {
            EvalVerdict::Inconclusive => "Sinais mistos: a classificação depende do perfil; uma troca entre FPS e estabilidade não garante vencedor.".into(),
            EvalVerdict::Regression => "Observado em sessões repetidas: o pacote vai para “não recomendados” nos perfis em que a métrica piorou.".into(),
            EvalVerdict::Improvement => "Entra no ranking medido; pode ser o vencedor se não dividir a posição.".into(),
            _ => "Entra no ranking medido, sem diferença em relação ao baseline.".into(),
        };
        if v == EvalVerdict::Inconclusive {
            e.next_step = Some("Os sinais são mistos (uma métrica melhorou e outra piorou). Escolha o perfil conforme o que importa mais para você.".into());
        }
    }
    done(e, v, summary)
}
