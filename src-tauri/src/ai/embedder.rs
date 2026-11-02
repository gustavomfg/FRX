//! Similaridade entre sintoma e candidato. `MiniLmEmbedder` é só o wrapper:
//! nesta etapa não carrega o modelo e devolve erro explícito.
//!
//! Só o MiniLM público, sem rótulos do FRX, é aceito (`runtime-frozen-control`).
//! O recuperador R02 da pesquisa foi treinado com rótulos do Codex e continua
//! fora do aplicativo por decisão do responsável; o construtor recusa o hash dele.

use std::path::{Path, PathBuf};

/// SHA-256 de `frozen-int8.onnx`: all-MiniLM-L6-v2 público, quantizado.
pub const PUBLIC_MINILM_SHA256: &str =
    "61b894f53611e70fcb02b261b37b80c807fe0b65665d93ee4e7944038fc3c4bc";
/// SHA-256 de `retriever-int8.onnx` (R02): recusado no aplicativo.
pub const RESEARCH_ONLY_R02_SHA256: &str =
    "67578e150a480885e935fffc8c95849fb5d8f6688b48a596837a76984f536bd8";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedError {
    /// O modelo ainda não é carregado nesta etapa.
    NotLoaded,
    /// Pesos de pesquisa ou de origem desconhecida.
    ModelNotAllowed(String),
    Invalid(String),
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLoaded => write!(f, "O modelo de similaridade ainda não foi carregado."),
            Self::ModelNotAllowed(m) => write!(f, "Modelo não permitido: {m}"),
            Self::Invalid(m) => write!(f, "Resposta inválida do modelo: {m}"),
        }
    }
}

pub trait Embedder: Send + Sync {
    /// Identifica o método em cada resultado.
    fn id(&self) -> &str;
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// Cosseno. Vetores vazios, nulos ou de tamanhos diferentes dão zero.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let (na, nb): (f32, f32) = (
        a.iter().map(|x| x * x).sum::<f32>().sqrt(),
        b.iter().map(|x| x * x).sum::<f32>().sqrt(),
    );
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

#[derive(Debug, Clone)]
pub struct MiniLmEmbedder {
    model: PathBuf,
    tokenizer: PathBuf,
}

impl MiniLmEmbedder {
    /// Só registra os caminhos. O hash esperado precisa ser o do modelo público.
    pub fn new(model: &Path, tokenizer: &Path, expected_sha256: &str) -> Result<Self, EmbedError> {
        if expected_sha256 != PUBLIC_MINILM_SHA256 {
            return Err(EmbedError::ModelNotAllowed(
                "só o MiniLM público é aceito no aplicativo".into(),
            ));
        }
        Ok(Self {
            model: model.to_owned(),
            tokenizer: tokenizer.to_owned(),
        })
    }

    pub fn paths(&self) -> (&Path, &Path) {
        (&self.model, &self.tokenizer)
    }
}

impl Embedder for MiniLmEmbedder {
    fn id(&self) -> &str {
        "minilm-public"
    }

    fn embed(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Err(EmbedError::NotLoaded)
    }
}

const LEXICAL_DIMENSIONS: usize = 512;
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "is", "it", "my", "i", "with", "for",
    "this", "that", "when", "at", "o", "e", "de", "do", "da", "em", "no", "na", "com", "um", "uma",
];

/// Alternativa léxica até o modelo existir: palavras em comum, sem semântica.
/// "crash" não encontra "travou". Cada resultado declara este método.
#[derive(Debug, Clone, Copy, Default)]
pub struct LexicalEmbedder;

impl Embedder for LexicalEmbedder {
    fn id(&self) -> &str {
        "lexical-fallback"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0.0_f32; LEXICAL_DIMENSIONS];
                let low = t.to_lowercase();
                for w in low
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty() && !STOPWORDS.contains(w))
                {
                    // FNV-1a: estável entre execuções e plataformas.
                    let h = w.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
                        (h ^ b as u64).wrapping_mul(0x100000001b3)
                    });
                    v[(h % LEXICAL_DIMENSIONS as u64) as usize] = 1.0;
                }
                v
            })
            .collect())
    }
}

#[cfg(test)]
mod ai_ranker_embedder {
    use super::*;

    #[test]
    fn the_wrapper_only_accepts_the_public_model_and_does_not_load_anything() {
        let (m, t) = (Path::new("/nao/existe.onnx"), Path::new("/nao/existe.json"));
        assert!(matches!(
            MiniLmEmbedder::new(m, t, RESEARCH_ONLY_R02_SHA256),
            Err(EmbedError::ModelNotAllowed(_))
        ));
        assert!(MiniLmEmbedder::new(m, t, "desconhecido").is_err());
        let e = MiniLmEmbedder::new(m, t, PUBLIC_MINILM_SHA256).unwrap();
        assert_eq!(e.embed(&["x"]), Err(EmbedError::NotLoaded));
    }

    #[test]
    fn cosine_handles_edge_cases() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(cosine(&[], &[]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine(&[0.0], &[0.0]), 0.0);
    }

    #[test]
    fn lexical_is_deterministic_and_finds_shared_words_only() {
        let e = LexicalEmbedder;
        let v = e
            .embed(&["game crash on start", "crash at start", "stuttering audio"])
            .unwrap();
        assert_eq!(
            v,
            e.embed(&["game crash on start", "crash at start", "stuttering audio"])
                .unwrap()
        );
        assert!(cosine(&v[0], &v[1]) > 0.5);
        assert_eq!(cosine(&v[0], &v[2]), 0.0);
    }
}
