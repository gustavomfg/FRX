//! Extração de ações a partir de uma descoberta normalizada.
//!
//! Interpretadores são plugáveis (`Interpreter`): hoje só existe o
//! determinístico, que lê as ações já estruturadas pela pesquisa (análise
//! estática do UMU-Protonfixes, extração de comentários) e reinterpreta cada
//! uma com parsers estritos. Um interpretador futuro (ex.: modelo de IA) teria
//! de devolver o mesmo `Extracted` — e passaria pelas mesmas validações.
//! Nenhum texto vira comando: o resultado é sempre um par `NOME=valor` ou uma
//! ação marcada como não suportada.

use super::DynClass;
use crate::fixes::models::{ActionKind, FixFinding};
use crate::steam::launch_options::{assignment, LaunchLine};

pub const MAX_ACTIONS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum Extracted {
    /// Variável de ambiente nas opções de inicialização (ainda não validada).
    Env {
        kind: ActionKind,
        key: String,
        value: String,
        raw: String,
        conditional: bool,
    },
    /// Algo que o Fix Engine não gera sozinho, com a categoria e o motivo.
    Unsupported {
        kind: ActionKind,
        raw: String,
        class: DynClass,
        reason: String,
    },
}

pub trait Interpreter {
    fn name(&self) -> &'static str;
    /// `Err` = conteúdo malformado (nada é gerado).
    fn interpret(&self, f: &FixFinding) -> Result<Vec<Extracted>, String>;
}

pub struct Deterministic;

fn short(s: &str) -> String {
    let t: String = s.chars().filter(|c| !c.is_control()).take(160).collect();
    t.trim().to_owned()
}

fn unsupported(kind: ActionKind, raw: &str, class: DynClass, reason: String) -> Extracted {
    Extracted::Unsupported {
        kind,
        raw: short(raw),
        class,
        reason,
    }
}

/// `NOME=valor` estrito: nome de variável válido, valor sem espaços nem aspas.
fn env_pair(text: &str) -> Option<(String, String)> {
    let (k, v) = assignment(text.trim())?;
    let ok = k.len() <= 64
        && !v.is_empty()
        && v.len() <= 120
        && v.bytes()
            .all(|b| b.is_ascii_graphic() && !b"\"'`$\\;|&<>()".contains(&b));
    ok.then(|| (k.to_owned(), v.to_owned()))
}

fn launch_line(raw: &str, conditional: bool, out: &mut Vec<Extracted>) -> Result<(), String> {
    let line =
        LaunchLine::parse(raw).map_err(|e| format!("Opção de inicialização malformada: {e}"))?;
    let Some(cmd) = line.command else {
        out.push(unsupported(
            ActionKind::LaunchOption,
            raw,
            DynClass::NeedsValidation,
            "A linha não tem %command%; não dá para saber onde ela entra.".into(),
        ));
        return Ok(());
    };
    for (i, w) in line.words.iter().enumerate() {
        if i == cmd {
            continue;
        }
        let quoted = line.text[w.start..w.end] != w.value;
        if i < cmd {
            match env_pair(&w.value).filter(|_| !quoted) {
                Some((key, value)) => out.push(Extracted::Env {
                    kind: ActionKind::LaunchOption,
                    key,
                    value,
                    raw: w.value.clone(),
                    conditional,
                }),
                None if assignment(&w.value).is_some() => {
                    return Err(format!("Valor malformado em {}.", short(&w.value)))
                }
                None => out.push(unsupported(
                    ActionKind::LaunchOption,
                    &w.value,
                    DynClass::NeedsValidation,
                    format!(
                        "A linha executa “{}” antes do jogo. O FRX não adiciona programas às opções de inicialização sozinho.",
                        short(&w.value)
                    ),
                )),
            }
        } else {
            out.push(unsupported(
                ActionKind::LaunchOption,
                &w.value,
                DynClass::NeedsValidation,
                format!(
                    "O argumento “{}” vai para o executável do jogo e tem efeito diferente em cada jogo; o FRX não o gera automaticamente.",
                    short(&w.value)
                ),
            ));
        }
    }
    Ok(())
}

impl Interpreter for Deterministic {
    fn name(&self) -> &'static str {
        "deterministic"
    }

    fn interpret(&self, f: &FixFinding) -> Result<Vec<Extracted>, String> {
        if f.actions.len() > MAX_ACTIONS {
            return Err("A descoberta lista ações demais para uma correção única.".into());
        }
        let mut out = Vec::new();
        for a in &f.actions {
            let v = a.value.as_str();
            if v.len() > 400 || v.chars().any(|c| c.is_control()) {
                return Err("Conteúdo da ação malformado.".into());
            }
            match a.kind {
                ActionKind::Env => match env_pair(v) {
                    Some((key, value)) => out.push(Extracted::Env {
                        kind: a.kind,
                        key,
                        value,
                        raw: v.to_owned(),
                        conditional: a.conditional,
                    }),
                    None if !v.contains('=') && assignment(&format!("{v}=x")).is_some() => {
                        out.push(unsupported(
                            a.kind,
                            v,
                            DynClass::NeedsValidation,
                            format!("Remove a variável {}; o FRX só sabe adicionar variáveis.", short(v)),
                        ))
                    }
                    None => return Err(format!("Variável malformada: {}", short(v))),
                },
                ActionKind::LaunchOption => launch_line(v, a.conditional, &mut out)?,
                ActionKind::Winetricks => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::NeedsValidation,
                    format!("Instala “{}” no prefixo do Proton com winetricks: baixa arquivos e executa instaladores, o que o FRX não faz automaticamente.", short(v)),
                )),
                ActionKind::DllOverride => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::NeedsValidation,
                    "Substitui DLLs no Wine. Troca de bibliotecas exige validação e nunca é gerada automaticamente.".into(),
                )),
                ActionKind::Registry => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::NeedsValidation,
                    "Altera o registro do Wine no prefixo do jogo; o Fix Engine ainda não tem ação reversível para isso.".into(),
                )),
                ActionKind::ConfigFile => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::NeedsValidation,
                    "Modifica arquivos do jogo. Só uma receita revisada para este jogo pode fazer isso.".into(),
                )),
                ActionKind::ProtonVersion => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::Informational,
                    format!("Indica a versão “{}” da ferramenta. A troca do Proton é feita nas propriedades do jogo na Steam; o FRX não altera essa configuração.", short(v)),
                )),
                ActionKind::Other => out.push(unsupported(
                    a.kind,
                    v,
                    DynClass::NeedsValidation,
                    "Ação não reconhecida pela análise estática.".into(),
                )),
            }
        }
        Ok(out)
    }
}
