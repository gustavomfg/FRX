//! Parser do formato KeyValues (VDF/ACF) usado pela Steam.
//!
//! Implementa um tokenizador de verdade — strings entre aspas com escapes,
//! tokens sem aspas, comentários `//`, blocos `{}` e condicionais `[$X]` —
//! em vez de expressões regulares. Chaves repetidas são preservadas e a
//! busca por chave ignora maiúsculas/minúsculas, como a própria Steam.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VdfError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for VdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VDF inválido na linha {}: {}", self.line, self.message)
    }
}

impl std::error::Error for VdfError {}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

/// Limite defensivo de aninhamento: arquivos da Steam raramente passam de 10.
const MAX_DEPTH: usize = 64;

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        // Ignora BOM UTF-8, presente em alguns arquivos gerados no Windows.
        let src = src.strip_prefix('\u{feff}').unwrap_or(src);
        Self {
            chars: src.chars().peekable(),
            line: 1,
        }
    }

    fn err(&self, message: impl Into<String>) -> VdfError {
        VdfError {
            line: self.line,
            message: message.into(),
        }
    }

    fn next_token(&mut self) -> Result<Option<(Token, usize)>, VdfError> {
        loop {
            let Some(&c) = self.chars.peek() else {
                return Ok(None);
            };
            match c {
                '\n' => {
                    self.line += 1;
                    self.chars.next();
                }
                c if c.is_whitespace() => {
                    self.chars.next();
                }
                '/' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'/') {
                        for c in self.chars.by_ref() {
                            if c == '\n' {
                                self.line += 1;
                                break;
                            }
                        }
                    } else {
                        // Uma barra solta faz parte de um token sem aspas.
                        let mut s = String::from("/");
                        self.read_bare(&mut s);
                        return Ok(Some((Token::Str(s), self.line)));
                    }
                }
                '{' => {
                    self.chars.next();
                    return Ok(Some((Token::Open, self.line)));
                }
                '}' => {
                    self.chars.next();
                    return Ok(Some((Token::Close, self.line)));
                }
                '"' => {
                    self.chars.next();
                    let start = self.line;
                    let mut s = String::new();
                    loop {
                        match self.chars.next() {
                            None => {
                                return Err(VdfError {
                                    line: start,
                                    message: "string sem aspas de fechamento".into(),
                                })
                            }
                            Some('"') => break,
                            Some('\\') => match self.chars.next() {
                                Some('n') => s.push('\n'),
                                Some('t') => s.push('\t'),
                                Some('\\') => s.push('\\'),
                                Some('"') => s.push('"'),
                                // Caminhos do Windows usam "\\"; uma barra isolada é mantida.
                                Some(other) => {
                                    s.push('\\');
                                    s.push(other);
                                }
                                None => return Err(self.err("escape incompleto no fim do arquivo")),
                            },
                            Some('\n') => {
                                self.line += 1;
                                s.push('\n');
                            }
                            Some(c) => s.push(c),
                        }
                    }
                    return Ok(Some((Token::Str(s), start)));
                }
                '[' => {
                    // Condicional de plataforma ([$WIN32], [!$X360]) — descartado.
                    for c in self.chars.by_ref() {
                        if c == ']' {
                            break;
                        }
                        if c == '\n' {
                            self.line += 1;
                            break;
                        }
                    }
                }
                _ => {
                    let mut s = String::new();
                    self.read_bare(&mut s);
                    return Ok(Some((Token::Str(s), self.line)));
                }
            }
        }
    }

    fn read_bare(&mut self, s: &mut String) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                break;
            }
            s.push(c);
            self.chars.next();
        }
    }
}

/// Lê um documento VDF inteiro. O resultado é sempre um objeto com os pares de nível superior.
pub fn parse(src: &str) -> Result<Vdf, VdfError> {
    let mut lx = Lexer::new(src);
    let pairs = parse_pairs(&mut lx, 0, false)?;
    Ok(Vdf::Obj(pairs))
}

fn parse_pairs(lx: &mut Lexer, depth: usize, nested: bool) -> Result<Vec<(String, Vdf)>, VdfError> {
    if depth > MAX_DEPTH {
        return Err(lx.err("aninhamento excessivo"));
    }
    let mut out = Vec::new();
    loop {
        let key = match lx.next_token()? {
            None if nested => return Err(lx.err("bloco sem '}' de fechamento")),
            None => return Ok(out),
            Some((Token::Close, _)) if nested => return Ok(out),
            Some((Token::Close, line)) => {
                return Err(VdfError {
                    line,
                    message: "'}' inesperado".into(),
                })
            }
            Some((Token::Open, line)) => {
                return Err(VdfError {
                    line,
                    message: "'{' sem chave".into(),
                })
            }
            Some((Token::Str(k), _)) => k,
        };
        match lx.next_token()? {
            Some((Token::Str(v), _)) => out.push((key, Vdf::Str(v))),
            Some((Token::Open, _)) => out.push((key, Vdf::Obj(parse_pairs(lx, depth + 1, true)?))),
            Some((Token::Close, line)) => {
                return Err(VdfError {
                    line,
                    message: format!("chave '{key}' sem valor"),
                })
            }
            None => return Err(lx.err(format!("chave '{key}' sem valor no fim do arquivo"))),
        }
    }
}

impl Vdf {
    /// Primeiro valor com a chave informada (sem diferenciar maiúsculas).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(pairs) => pairs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    /// Caminho de chaves aninhadas, ex.: `["InstallConfigStore", "Software", "Valve"]`.
    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |node, k| node.get(k))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn str_at(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Vdf::as_str)
    }

    pub fn u64_at(&self, key: &str) -> Option<u64> {
        self.str_at(key).and_then(|s| s.trim().parse().ok())
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(pairs) => pairs,
            Vdf::Str(_) => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_objects_and_escapes() {
        let src = r#"
            // comentário
            "AppState"
            {
                "appid"     "200260"
                "name"      "Batman: \"Arkham\" City"
                "installdir"    "Batman Arkham City GOTY"
                "InstalledDepots" { "200261" { "size" "123" } }
            }
        "#;
        let doc = parse(src).unwrap();
        let app = doc.get("appstate").unwrap();
        assert_eq!(app.str_at("APPID"), Some("200260"));
        assert_eq!(app.str_at("name"), Some("Batman: \"Arkham\" City"));
        assert_eq!(
            app.path(&["InstalledDepots", "200261"])
                .and_then(|d| d.u64_at("size")),
            Some(123)
        );
    }

    #[test]
    fn accepts_unquoted_tokens_and_conditionals() {
        let doc = parse("root { key value [$WIN32] other \"x\" }").unwrap();
        let root = doc.get("root").unwrap();
        assert_eq!(root.str_at("key"), Some("value"));
        assert_eq!(root.str_at("other"), Some("x"));
    }

    #[test]
    fn keeps_windows_paths() {
        let doc = parse(r#""a" { "path" "D:\\Games\\Steam" }"#).unwrap();
        assert_eq!(
            doc.path(&["a", "path"]).and_then(Vdf::as_str),
            Some("D:\\Games\\Steam")
        );
    }

    #[test]
    fn rejects_unterminated_blocks_and_strings() {
        assert!(parse(r#""a" { "b" "c""#).is_err());
        assert!(parse(r#""a" "b"#).is_err());
        assert!(parse(r#"}"#).is_err());
        assert!(parse(r#""a""#).is_err());
    }

    #[test]
    fn rejects_absurd_nesting() {
        let src = "a {".repeat(100);
        assert!(parse(&src).is_err());
    }

    #[test]
    fn empty_document_is_empty_object() {
        assert_eq!(parse("  \n// nada\n").unwrap(), Vdf::Obj(vec![]));
    }
}
