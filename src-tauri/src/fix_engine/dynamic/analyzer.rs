//! Analisa cada descoberta e decide a categoria (A, B ou C) com o motivo
//! específico. Cada descoberta é analisada sozinha: ações de comentários
//! diferentes nunca são somadas numa correção só. Descobertas diferentes que
//! levam exatamente às mesmas ações viram **uma** receita, com todas as
//! origens preservadas.

use super::extraction::{Deterministic, Extracted, Interpreter};
use super::generator::{self, Input};
use super::validation::{self, EnvVerdict};
use super::{provenance, ActionView, Assessment, DynStatus, EvidenceLevel};
use crate::fix_engine::catalog::Catalog;
use crate::fix_engine::models::{Recipe, RecipeAction, Trust};
use crate::fix_engine::security::validate_generated;
use crate::fixes::compat::vendor_name;
use crate::fixes::models::{AnalysisState, Compatibility, FindingKind, FixFinding, GpuReq};

/// O que a análise precisa saber do sistema (sem varrer a Steam).
pub struct Ctx<'a> {
    pub app_id: &'a str,
    pub gpu: Option<GpuReq>,
    pub tool_name: Option<&'a str>,
    pub catalog: &'a Catalog,
}

fn finish(mut a: Assessment, status: DynStatus, reasons: Vec<String>) -> Assessment {
    a.status = status;
    a.class = status.class();
    a.reasons.extend(reasons);
    a
}

/// Ações do catálogo como conjunto `NOME=valor` (só receitas apenas de variáveis).
fn env_set(r: &Recipe) -> Option<Vec<(String, String)>> {
    let mut v = Vec::new();
    for a in &r.actions {
        match a {
            RecipeAction::SetLaunchEnv { key, value } => v.push((key.clone(), value.clone())),
            _ => return None,
        }
    }
    v.sort();
    Some(v)
}

pub fn assess(f: &FixFinding, ctx: &Ctx<'_>) -> Assessment {
    assess_with(f, ctx, &Deterministic)
}

pub fn assess_with(f: &FixFinding, ctx: &Ctx<'_>, interp: &dyn Interpreter) -> Assessment {
    let (evidence, evidence_note) = validation::evidence(f);
    let a = Assessment {
        performance_warnings: vec![],
        finding_id: f.id.clone(),
        app_id: ctx.app_id.to_owned(),
        class: super::DynClass::Informational,
        status: DynStatus::NoActions,
        reasons: vec![],
        evidence,
        evidence_note,
        experimental: false,
        actions: vec![],
        recipe_id: None,
        recipe: None,
        provenance: provenance::collect(f),
        interpreter: interp.name().into(),
    };

    if f.game_app_id != ctx.app_id || !crate::art::valid_app_id(&f.game_app_id) {
        return finish(
            a,
            DynStatus::Invalid,
            vec![format!(
                "A descoberta pertence a outro jogo (AppID {}); nada é gerado para este.",
                f.game_app_id.chars().take(12).collect::<String>()
            )],
        );
    }
    if a.provenance.is_empty() {
        return finish(
            a,
            DynStatus::Invalid,
            vec!["Sem fonte rastreável (HTTPS no GitHub); nada é gerado.".into()],
        );
    }
    if f.compatibility == Compatibility::AlreadyIncluded {
        return finish(
            a,
            DynStatus::AlreadyIncluded,
            vec![f.compatibility_note.clone()],
        );
    }

    let items = match interp.interpret(f) {
        Ok(i) => i,
        Err(e) => return finish(a, DynStatus::Invalid, vec![e]),
    };
    let mut a = a;
    a.actions = items
        .iter()
        .map(|x| match x {
            Extracted::Env { kind, raw, .. } => ActionView {
                kind: *kind,
                raw: raw.clone(),
                supported: false,
                note: String::new(),
            },
            Extracted::Unsupported {
                kind, raw, reason, ..
            } => ActionView {
                kind: *kind,
                raw: raw.clone(),
                supported: false,
                note: reason.clone(),
            },
        })
        .collect();
    if items.is_empty() {
        let why = match f.kind {
            FindingKind::Report => "Relato geral sobre o jogo, sem ação técnica estruturada. Consulte a fonte original.",
            FindingKind::ReleaseNote => "Indica que o problema foi corrigido numa versão da ferramenta; não há ação para aplicar.",
            _ => "Nenhuma ação técnica reconhecida nesta descoberta.",
        };
        return finish(a, DynStatus::NoActions, vec![why.into()]);
    }

    let unsupported: Vec<(super::DynClass, String)> = items
        .iter()
        .filter_map(|x| match x {
            Extracted::Unsupported { class, reason, .. } => Some((*class, reason.clone())),
            _ => None,
        })
        .collect();
    let mut envs: Vec<(String, String)> = Vec::new();
    let mut conditional = false;
    for x in &items {
        if let Extracted::Env {
            key,
            value,
            conditional: c,
            ..
        } = x
        {
            conditional |= *c;
            match envs.iter().find(|(k, _)| k == key) {
                Some((_, v)) if v != value => {
                    return finish(a, DynStatus::Invalid, vec![format!("A descoberta define {key} com valores diferentes; não dá para saber qual vale.")])
                }
                Some(_) => {}
                None => envs.push((key.clone(), value.clone())),
            }
        }
    }

    a.performance_warnings = super::compatibility::performance_warnings(
        envs.iter().map(|(k, v)| (k.as_str(), v.as_str())),
    );
    for view in a.actions.iter_mut().filter(|x| x.note.is_empty()) {
        view.note = "Variável reconhecida; avaliada junto com o restante da correção.".into();
    }
    if !unsupported.is_empty() {
        let needs = unsupported
            .iter()
            .any(|(c, _)| *c == super::DynClass::NeedsValidation);
        let mut reasons: Vec<String> = unsupported.into_iter().map(|(_, r)| r).collect();
        if !envs.is_empty() {
            reasons.push("A correção tem outras partes além das variáveis; aplicar só uma parte mudaria o procedimento, então nada é gerado.".into());
        }
        return finish(
            a,
            if needs {
                DynStatus::NeedsValidation
            } else {
                DynStatus::NoActions
            },
            reasons,
        );
    }
    if f.analysis != AnalysisState::Parsed {
        return finish(a, DynStatus::NeedsValidation, vec!["Parte do arquivo original não foi interpretada com segurança; a correção completa pode ter passos que o FRX não vê.".into()]);
    }
    if conditional {
        return finish(a, DynStatus::NeedsValidation, vec!["No script original a variável só é definida sob certas condições; aplicá-la sempre mudaria o comportamento.".into()]);
    }

    // Segurança da operação, variável por variável.
    let mut known = Vec::new();
    let mut blocked = Vec::new();
    let mut unknown = Vec::new();
    let mut notes: Vec<(String, bool, String)> = Vec::new();
    for (k, v) in &envs {
        match validation::env(k, v) {
            EnvVerdict::Ok(kn) => {
                notes.push((k.clone(), true, kn.effect.into()));
                known.push((k.clone(), v.clone(), kn));
            }
            EnvVerdict::Blocked(r) => {
                notes.push((k.clone(), false, r.clone()));
                blocked.push(r);
            }
            EnvVerdict::Unknown(r) => {
                notes.push((k.clone(), false, r.clone()));
                unknown.push(r);
            }
        }
    }
    for (view, item) in a.actions.iter_mut().zip(&items) {
        if let Extracted::Env { key, .. } = item {
            if let Some((_, ok, n)) = notes.iter().find(|(k, ..)| k == key) {
                view.supported = *ok;
                view.note = n.clone();
            }
        }
    }
    if !blocked.is_empty() {
        return finish(a, DynStatus::Blocked, blocked);
    }
    if !unknown.is_empty() {
        return finish(a, DynStatus::NeedsValidation, unknown);
    }

    // Uma receita do catálogo faz exatamente isto?
    let mut set = envs.clone();
    set.sort();
    if let Some(r) = ctx
        .catalog
        .recipes
        .iter()
        .find(|r| env_set(r).as_ref() == Some(&set))
    {
        match r.trust {
            Trust::Reviewed => {
                let mut a = finish(
                    a,
                    DynStatus::Reviewed,
                    vec!["Uma receita revisada do FRX faz exatamente este ajuste.".into()],
                );
                a.recipe_id = Some(r.id.clone());
                if evidence == EvidenceLevel::Contested {
                    a.reasons.push(a.evidence_note.clone());
                }
                return a;
            }
            Trust::Revoked => {
                return finish(
                    a,
                    DynStatus::Blocked,
                    vec![
                        "O FRX revogou a receita para este ajuste por um problema conhecido."
                            .into(),
                    ],
                )
            }
            _ => {}
        }
    }

    // Compatibilidade com o sistema.
    if f.compatibility == Compatibility::Incompatible {
        return finish(
            a,
            DynStatus::Incompatible,
            vec![f.compatibility_note.clone()],
        );
    }
    if let Some(req) = known.iter().find_map(|(_, _, k)| k.gpu) {
        if ctx.gpu != Some(req) {
            return finish(
                a,
                DynStatus::Incompatible,
                vec![match ctx.gpu {
                    Some(g) => format!(
                        "Só faz sentido em GPUs {}; seu sistema usa {}.",
                        vendor_name(req),
                        vendor_name(g)
                    ),
                    None => format!(
                        "Só faz sentido em GPUs {}, e não foi possível identificar a sua.",
                        vendor_name(req)
                    ),
                }],
            );
        }
    }

    // Confiabilidade.
    if evidence == EvidenceLevel::Contested {
        let note = a.evidence_note.clone();
        return finish(
            a,
            DynStatus::Contested,
            vec![format!(
                "{note} O FRX não oferece aplicar uma correção que os relatos contestam."
            )],
        );
    }

    let recipe = generator::build(Input {
        finding: f,
        app_id: ctx.app_id,
        actions: known,
        evidence,
        evidence_note: a.evidence_note.clone(),
        provenance: a.provenance.clone(),
        interpreter: interp.name(),
    });
    if let Err(e) = validate_generated(&recipe) {
        return finish(
            a,
            DynStatus::NeedsValidation,
            vec![format!(
                "A receita gerada não passou na validação do núcleo: {e}"
            )],
        );
    }
    let mut reasons = vec!["Receita gerada automaticamente a partir da descoberta. Não foi revisada por pessoas: confira a prévia antes de aplicar.".to_owned()];
    let experimental = !matches!(
        evidence,
        EvidenceLevel::Documented | EvidenceLevel::Corroborated
    );
    if experimental {
        reasons.push(format!("Experimental: {}", a.evidence_note));
    }
    if !f.requirements.proton_versions.is_empty() {
        reasons.push(format!(
            "Relatada com {}{}.",
            f.requirements.proton_versions.join(", "),
            ctx.tool_name
                .map(|t| format!("; o jogo usa {t}"))
                .unwrap_or_default()
        ));
    }
    let mut a = finish(a, DynStatus::Generated, reasons);
    a.experimental = experimental;
    a.recipe_id = Some(recipe.id.clone());
    a.recipe = Some(recipe);
    a
}

/// Mesmas ações em descobertas diferentes → uma receita só, com todas as
/// origens e a melhor evidência entre elas.
pub fn merge_duplicates(list: &mut [Assessment]) {
    for i in 0..list.len() {
        let Some(id) = list[i].recipe.as_ref().map(|r| r.id.clone()) else {
            continue;
        };
        for j in 0..i {
            if list[j].recipe.as_ref().is_some_and(|r| r.id == id) {
                let (head, tail) = list.split_at_mut(i);
                let (first, dup) = (&mut head[j], &mut tail[0]);
                let (Some(r1), Some(r2)) = (first.recipe.as_mut(), dup.recipe.as_ref()) else {
                    break;
                };
                let (Some(g1), Some(g2)) = (r1.generated.as_mut(), r2.generated.as_ref()) else {
                    break;
                };
                for fid in &g2.finding_ids {
                    if !g1.finding_ids.contains(fid) {
                        g1.finding_ids.push(fid.clone());
                    }
                }
                for p in &g2.provenance {
                    if g1.provenance.len() < 8 && !g1.provenance.iter().any(|x| x.url == p.url) {
                        g1.provenance.push(p.clone());
                    }
                }
                for s in &r2.sources {
                    if r1.sources.len() < 8 && !r1.sources.iter().any(|x| x.url == s.url) {
                        r1.sources.insert(r1.sources.len() - 1, s.clone());
                    }
                }
                g1.discovered_at = g1.discovered_at.clone().max(g2.discovered_at.clone());
                if g2.evidence < g1.evidence {
                    g1.evidence = g2.evidence;
                    g1.evidence_note = g2.evidence_note.clone();
                }
                dup.recipe = Some(r1.clone());
                break;
            }
        }
    }
    // A cópia mais completa vale para todas as descobertas da mesma receita.
    for i in 0..list.len() {
        if let Some(id) = list[i].recipe.as_ref().map(|r| r.id.clone()) {
            if let Some(first) = list
                .iter()
                .position(|x| x.recipe.as_ref().is_some_and(|r| r.id == id))
            {
                if first != i {
                    list[i].recipe = list[first].recipe.clone();
                }
            }
        }
    }
}

pub fn assess_all(findings: &[FixFinding], ctx: &Ctx<'_>) -> Vec<Assessment> {
    let mut v: Vec<Assessment> = findings.iter().map(|f| assess(f, ctx)).collect();
    merge_duplicates(&mut v);
    v
}

/// Regenera a receita a partir das descobertas atuais e a revalida. Usado
/// antes de cada prévia e aplicação: o registro em disco nunca é executado.
pub fn resolve(findings: &[FixFinding], ctx: &Ctx<'_>, recipe_id: &str) -> Result<Recipe, String> {
    let r = assess_all(findings, ctx)
        .into_iter()
        .find_map(|a| a.recipe.filter(|r| r.id == recipe_id))
        .ok_or("A receita gerada não corresponde mais às descobertas atuais (a pesquisa, o sistema ou as regras mudaram). Analise a correção de novo.")?;
    validate_generated(&r).map_err(|e| format!("Receita gerada recusada: {e}"))?;
    Ok(r)
}
