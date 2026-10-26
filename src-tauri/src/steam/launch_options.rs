//! Opções de inicialização da Steam: leitura e edição **cirúrgica**.
//!
//! Compartilhado pelo Performance Lab (linha do MangoHud) e pelo Fix Engine
//! (variáveis e argumentos gerenciados). A linha é dividida em palavras com
//! posições na string original — aspas, escapes e espaços são preservados —
//! e nenhuma expansão de shell é interpretada ou executada. Edições trocam
//! apenas as fatias que pertencem ao FRX; o resto da linha fica intacto.

pub fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-+=:@".contains(c))
    {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellWord {
    pub start: usize,
    pub end: usize,
    pub value: String,
}

/// Apenas localiza palavras; nunca interpreta expansões nem executa um shell.
/// As fatias originais preservam aspas, escapes e espaços dentro dos argumentos.
pub fn shell_words(text: &str) -> Result<Vec<ShellWord>, String> {
    let mut words = Vec::new();
    let mut start = None;
    let mut value = String::new();
    let mut quote = None;
    let mut escaped = false;
    for (i, ch) in text.char_indices() {
        if ch.is_control() && ch != '\t' {
            return Err(
                "As opções de inicialização contêm quebras de linha ou caracteres de controle."
                    .into(),
            );
        }
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if quote != Some('\'')
            && (ch == '`' || (ch == '$' && text.as_bytes().get(i + 1) == Some(&b'(')))
        {
            return Err("As opções atuais usam substituição de comandos. Mantenha-as e importe o log manualmente; a mesclagem automática não é segura neste caso.".into());
        }
        if ch == '\\' && quote != Some('\'') {
            start.get_or_insert(i);
            escaped = true;
        } else if quote == Some(ch) {
            quote = None;
        } else if quote.is_none() && matches!(ch, '\'' | '"') {
            start.get_or_insert(i);
            quote = Some(ch);
        } else if quote.is_none() && ch.is_whitespace() {
            if let Some(a) = start.take() {
                words.push(ShellWord {
                    start: a,
                    end: i,
                    value: std::mem::take(&mut value),
                });
            }
        } else {
            if quote.is_none() && ";|&<>()".contains(ch) {
                return Err("As opções atuais usam operadores de shell. Preserve essa linha e importe o log manualmente; o FRX não a reescreve automaticamente.".into());
            }
            start.get_or_insert(i);
            value.push(ch);
        }
    }
    if quote.is_some() || escaped {
        return Err("As opções de inicialização têm aspas ou escapes sem fechamento.".into());
    }
    if let Some(a) = start {
        words.push(ShellWord {
            start: a,
            end: text.len(),
            value,
        });
    }
    Ok(words)
}

/// `NOME=valor` válido no início da linha (variável de ambiente).
pub fn assignment(word: &str) -> Option<(&str, &str)> {
    let (k, v) = word.split_once('=')?;
    let ok = !k.is_empty()
        && k.chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()));
    ok.then_some((k, v))
}

/// Estrutura de uma linha: variáveis iniciais, `%command%` e o restante.
#[derive(Debug, Clone)]
pub struct LaunchLine {
    pub text: String,
    pub words: Vec<ShellWord>,
    /// Índice da palavra `%command%`, se houver.
    pub command: Option<usize>,
}

impl LaunchLine {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim().to_owned();
        let words = shell_words(&text)?;
        let commands: Vec<usize> = words
            .iter()
            .enumerate()
            .filter(|(_, w)| w.value == "%command%")
            .map(|(i, _)| i)
            .collect();
        if commands.len() > 1
            || words
                .iter()
                .any(|w| w.value.contains("%command%") && text[w.start..w.end] != *"%command%")
        {
            return Err("A linha precisa ter um único %command%, fora de aspas.".into());
        }
        Ok(Self {
            command: commands.first().copied(),
            text,
            words,
        })
    }

    /// Variáveis de ambiente do começo da linha (antes de qualquer wrapper/comando).
    pub fn leading_env(&self) -> Vec<(usize, String, String)> {
        self.words
            .iter()
            .enumerate()
            .take_while(|(i, w)| Some(*i) != self.command && assignment(&w.value).is_some())
            .filter_map(|(i, w)| assignment(&w.value).map(|(k, v)| (i, k.to_owned(), v.to_owned())))
            .collect()
    }

    /// Só argumentos do jogo, sem `%command%` (a Steam os coloca depois do executável).
    pub fn args_only(&self) -> bool {
        self.command.is_none()
            && self
                .words
                .iter()
                .find(|w| assignment(&w.value).is_none())
                .is_none_or(|w| w.value.starts_with('-'))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvChange {
    pub line: String,
    /// Valor anterior da variável, se já existia (para restaurar no rollback).
    pub previous: Option<String>,
}

fn join(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Define `KEY=value` no começo da linha, substituindo só essa variável se já existir.
pub fn set_env(existing: Option<&str>, key: &str, value: &str) -> Result<EnvChange, String> {
    let token = format!("{key}={}", shell_quote(value));
    let ex = existing.map(str::trim).unwrap_or("");
    if ex.is_empty() {
        return Ok(EnvChange {
            line: format!("{token} %command%"),
            previous: None,
        });
    }
    let line = LaunchLine::parse(ex)?;
    if line.command.is_none() && !line.args_only() {
        return Err("A linha atual usa um wrapper sem %command%; o FRX não consegue editá-la com segurança.".into());
    }
    if let Some((i, _, prev)) = line.leading_env().into_iter().find(|(_, k, _)| k == key) {
        let w = &line.words[i];
        let mut out = line.text.clone();
        out.replace_range(w.start..w.end, &token);
        return Ok(EnvChange {
            line: out,
            previous: Some(prev),
        });
    }
    let rest = if line.command.is_none() {
        format!("%command% {}", line.text)
    } else {
        line.text.clone()
    };
    Ok(EnvChange {
        line: join(&[&token, &rest]),
        previous: None,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Revert {
    /// Linha nova, com a variável removida ou restaurada.
    Done(String),
    /// A variável não está mais na linha (outro programa ou o usuário removeu).
    Missing,
    /// O valor atual não é o que o FRX escreveu: preservamos e informamos.
    Changed(String),
}

/// Desfaz `set_env`: só mexe se o valor atual ainda for o que o FRX escreveu.
pub fn revert_env(
    existing: Option<&str>,
    key: &str,
    written: &str,
    previous: Option<&str>,
) -> Result<Revert, String> {
    let ex = existing.map(str::trim).unwrap_or("");
    if ex.is_empty() {
        return Ok(Revert::Missing);
    }
    let line = LaunchLine::parse(ex)?;
    let Some((i, _, cur)) = line.leading_env().into_iter().find(|(_, k, _)| k == key) else {
        return Ok(Revert::Missing);
    };
    if cur != written {
        return Ok(Revert::Changed(cur));
    }
    let w = &line.words[i];
    let mut out = line.text.clone();
    match previous {
        Some(p) => out.replace_range(w.start..w.end, &format!("{key}={}", shell_quote(p))),
        None => {
            // Remove a palavra e o espaço seguinte.
            let end = out[w.end..]
                .find(|c: char| !c.is_whitespace())
                .map_or(out.len(), |n| w.end + n);
            out.replace_range(w.start..end, "");
        }
    }
    let out = out.trim().to_owned();
    // "%command%" sozinho equivale a não ter opções.
    Ok(Revert::Done(if out == "%command%" {
        String::new()
    } else {
        out
    }))
}

/// Acrescenta argumentos do jogo ao fim da linha, sem duplicar os que já existem.
pub fn add_args(existing: Option<&str>, args: &[String]) -> Result<(String, Vec<String>), String> {
    let ex = existing.map(str::trim).unwrap_or("");
    let line = LaunchLine::parse(ex)?;
    if !ex.is_empty() && line.command.is_none() && !line.args_only() {
        return Err("A linha atual usa um wrapper sem %command%; o FRX não consegue editá-la com segurança.".into());
    }
    let present: Vec<&str> = line
        .words
        .iter()
        .skip(line.command.map_or(0, |c| c + 1))
        .map(|w| w.value.as_str())
        .collect();
    let added: Vec<String> = args
        .iter()
        .filter(|a| !present.contains(&a.as_str()))
        .cloned()
        .collect();
    let base = if ex.is_empty() {
        "%command%".to_owned()
    } else if line.command.is_none() {
        format!("%command% {ex}")
    } else {
        ex.to_owned()
    };
    let mut parts = vec![base];
    parts.extend(added.iter().cloned());
    Ok((
        join(&parts.iter().map(String::as_str).collect::<Vec<_>>()),
        added,
    ))
}

/// Remove exatamente os argumentos que o FRX acrescentou (depois de `%command%`).
pub fn remove_args(existing: Option<&str>, added: &[String]) -> Result<Revert, String> {
    let ex = existing.map(str::trim).unwrap_or("");
    if ex.is_empty() {
        return Ok(Revert::Missing);
    }
    let line = LaunchLine::parse(ex)?;
    let from = line.command.map_or(0, |c| c + 1);
    let mut targets: Vec<&ShellWord> = Vec::new();
    for a in added {
        match line
            .words
            .iter()
            .skip(from)
            .rev()
            .find(|w| &w.value == a && !targets.iter().any(|t| t.start == w.start))
        {
            Some(w) => targets.push(w),
            None => return Ok(Revert::Missing),
        }
    }
    targets.sort_by_key(|w| std::cmp::Reverse(w.start));
    let mut out = line.text.clone();
    for w in targets {
        let start = out[..w.start]
            .rfind(|c: char| !c.is_whitespace())
            .map_or(0, |n| n + 1);
        out.replace_range(start..w.end, "");
    }
    let out = out.trim().to_owned();
    Ok(Revert::Done(if out == "%command%" {
        String::new()
    } else {
        out
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_env_on_empty_and_existing_lines() {
        assert_eq!(
            set_env(None, "PROTON_LOG", "1").unwrap().line,
            "PROTON_LOG=1 %command%"
        );
        let c = set_env(
            Some("PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%"),
            "PROTON_LOG",
            "1",
        )
        .unwrap();
        assert_eq!(
            c.line,
            "PROTON_LOG=1 PROTON_NVIDIA_LIBS=1 PROTON_USE_WOW64=0 %command%"
        );
        assert_eq!(c.previous, None);
        let c = set_env(Some("PROTON_LOG=0 %command% -dx11"), "PROTON_LOG", "1").unwrap();
        assert_eq!(c.line, "PROTON_LOG=1 %command% -dx11");
        assert_eq!(c.previous.as_deref(), Some("0"));
        assert_eq!(
            set_env(Some("-dx11 -skipintro"), "A", "1").unwrap().line,
            "A=1 %command% -dx11 -skipintro"
        );
    }

    #[test]
    fn preserves_mangohud_gamescope_and_quotes() {
        let line = "MANGOHUD_CONFIGFILE='/home/u/a b/x.conf' gamescope -f -- mangohud %command% -arg \"com espaço\"";
        let c = set_env(Some(line), "PROTON_LOG", "1").unwrap();
        assert_eq!(c.line, format!("PROTON_LOG=1 {line}"));
        assert_eq!(
            revert_env(Some(&c.line), "PROTON_LOG", "1", None).unwrap(),
            Revert::Done(line.to_owned())
        );
    }

    #[test]
    fn revert_restores_previous_and_respects_third_party_changes() {
        assert_eq!(
            revert_env(
                Some("PROTON_LOG=1 %command% -dx11"),
                "PROTON_LOG",
                "1",
                Some("0")
            )
            .unwrap(),
            Revert::Done("PROTON_LOG=0 %command% -dx11".into())
        );
        assert_eq!(
            revert_env(Some("PROTON_LOG=1 %command%"), "PROTON_LOG", "1", None).unwrap(),
            Revert::Done(String::new())
        );
        assert_eq!(
            revert_env(Some("PROTON_LOG=2 %command%"), "PROTON_LOG", "1", None).unwrap(),
            Revert::Changed("2".into())
        );
        assert_eq!(
            revert_env(Some("%command%"), "PROTON_LOG", "1", None).unwrap(),
            Revert::Missing
        );
        assert_eq!(revert_env(None, "X", "1", None).unwrap(), Revert::Missing);
        // Variável depois de um wrapper não é variável de ambiente da linha.
        assert_eq!(
            revert_env(
                Some("gamemoderun PROTON_LOG=1 %command%"),
                "PROTON_LOG",
                "1",
                None
            )
            .unwrap(),
            Revert::Missing
        );
    }

    #[test]
    fn refuses_unsafe_or_ambiguous_lines() {
        assert!(
            set_env(Some("gamemoderun"), "A", "1").is_err(),
            "wrapper sem %command%"
        );
        assert!(set_env(Some("A=1 %command% && rm -rf ~"), "B", "1").is_err());
        assert!(set_env(Some("A=$(id) %command%"), "B", "1").is_err());
        assert!(set_env(Some("%command% %command%"), "B", "1").is_err());
        assert!(set_env(Some("\"%command%\""), "B", "1").is_err());
        assert!(set_env(Some("A='sem fim %command%"), "B", "1").is_err());
    }

    #[test]
    fn args_are_added_once_and_removed_exactly() {
        let (l, added) = add_args(None, &["-fullscreen".into(), "-vulkan".into()]).unwrap();
        assert_eq!(l, "%command% -fullscreen -vulkan");
        let (l2, added2) = add_args(
            Some("PROTON_LOG=1 %command% -vulkan"),
            &["-fullscreen".into(), "-vulkan".into()],
        )
        .unwrap();
        assert_eq!(l2, "PROTON_LOG=1 %command% -vulkan -fullscreen");
        assert_eq!(
            added2,
            vec!["-fullscreen"],
            "não duplica nem reivindica o que já existia"
        );
        assert_eq!(
            remove_args(Some(&l2), &added2).unwrap(),
            Revert::Done("PROTON_LOG=1 %command% -vulkan".into())
        );
        assert_eq!(
            remove_args(Some(&l), &added).unwrap(),
            Revert::Done(String::new())
        );
        assert_eq!(
            remove_args(Some("%command% -dx11"), &["-vulkan".into()]).unwrap(),
            Revert::Missing
        );
    }
}
