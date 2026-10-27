//! Modelos do Performance Lab (espelhados em `src/types/performance.ts`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Versão do formato salvo em disco. Aumente e trate em `persistence::migrate`.
pub const SCHEMA_VERSION: u32 = 1;
/// Versão da metodologia de cálculo. Sessões antigas são reprocessadas a partir do log bruto.
pub const ANALYSIS_VERSION: u32 = 4;

/// Métricas reconhecidas nos logs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Metric {
    Fps,
    Frametime,
    CpuLoad,
    GpuLoad,
    CpuTemp,
    GpuTemp,
    CpuPower,
    GpuPower,
    GpuCoreClock,
    GpuMemClock,
    CpuMhz,
    Vram,
    Ram,
    Swap,
    ProcessRss,
}

impl Metric {
    pub fn unit(self) -> &'static str {
        match self {
            Metric::Fps => "FPS",
            Metric::Frametime => "ms",
            Metric::CpuLoad | Metric::GpuLoad => "%",
            Metric::CpuTemp | Metric::GpuTemp => "°C",
            Metric::CpuPower | Metric::GpuPower => "W",
            Metric::GpuCoreClock | Metric::GpuMemClock | Metric::CpuMhz => "MHz",
            Metric::Vram | Metric::Ram | Metric::Swap | Metric::ProcessRss => "GiB",
        }
    }

    /// Faixa física plausível — fora dela o valor é descartado e registrado.
    pub fn plausible(self, v: f64) -> bool {
        if !v.is_finite() || v < 0.0 {
            return false;
        }
        match self {
            Metric::Fps => v <= 10_000.0,
            Metric::Frametime => v > 0.0 && v <= 10_000.0,
            Metric::CpuLoad | Metric::GpuLoad => v <= 100.5,
            Metric::CpuTemp | Metric::GpuTemp => v <= 150.0,
            Metric::CpuPower | Metric::GpuPower => v <= 2_000.0,
            Metric::GpuCoreClock | Metric::GpuMemClock | Metric::CpuMhz => v <= 20_000.0,
            Metric::Vram | Metric::Ram | Metric::Swap | Metric::ProcessRss => v <= 4_096.0,
        }
    }

    /// Sensores (não incluem FPS/frametime, que vêm de cada quadro).
    pub fn is_sensor(self) -> bool {
        !matches!(self, Metric::Fps | Metric::Frametime)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LogFormat {
    /// MangoHud com `log_versioning` (cabeçalho v1 + versão).
    MangohudV1,
    /// MangoHud sem versionamento (cabeçalho de sistema + colunas).
    Mangohud,
    /// CSV com coluna de frametime (ex.: PresentMon `MsBetweenPresents`).
    GenericFrametime,
}

/// Cada linha é um quadro (log_interval=0) ou uma amostra periódica.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureMode {
    PerFrame,
    Periodic,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogSystemInfo {
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub ram_kib: Option<u64>,
    pub kernel: Option<String>,
    pub driver: Option<String>,
    pub cpu_scheduler: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

impl Issue {
    pub fn new(severity: Severity, code: &str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QualityLevel {
    /// Boa para comparação.
    Good,
    /// Utilizável, com ressalvas.
    Usable,
    /// Não deve ser usada como base de comparação.
    Unsuitable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Quality {
    pub level: QualityLevel,
    pub issues: Vec<Issue>,
    pub total_rows: usize,
    pub excluded_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FpsStats {
    /// Média ponderada pelo tempo: quadros ÷ tempo total.
    pub avg: Option<f64>,
    pub median: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// 1% low = 1000 ÷ média dos 1% piores frametimes (precisa de ≥ 100 quadros).
    pub low1: Option<f64>,
    /// 0.1% low = 1000 ÷ média dos 0,1% piores frametimes (precisa de ≥ 1000 quadros).
    pub low01: Option<f64>,
    /// `true` quando os lows vêm de amostras periódicas (estimativa grosseira).
    pub lows_estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistogramBucket {
    pub label: String,
    pub from_ms: f64,
    pub to_ms: Option<f64>,
    /// Fração do tempo da sessão gasta em quadros desta faixa (0–1).
    pub time_share: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrametimeStats {
    pub avg: Option<f64>,
    pub median: Option<f64>,
    pub p90: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
    pub p999: Option<f64>,
    pub max: Option<f64>,
    pub stdev: Option<f64>,
    pub histogram: Vec<HistogramBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Interval {
    pub start_s: f64,
    pub end_s: f64,
    pub spikes: u32,
    pub worst_ms: f64,
    /// Coeficiente de variação do frametime no intervalo.
    pub cv: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stability {
    /// Quadros com frametime ≥ 2× a mediana local e ≥ 8 ms acima dela.
    pub spikes: u32,
    pub spikes_per_min: Option<f64>,
    pub unstable_intervals: Vec<Interval>,
    /// FPS médio no primeiro e no último terço da sessão.
    pub first_third_fps: Option<f64>,
    pub last_third_fps: Option<f64>,
    pub trend_pct: Option<f64>,
    /// Coeficiente de variação do frametime na sessão inteira.
    pub cv: Option<f64>,
    /// Desvio padrão do FPS médio por segundo (base da incerteza na comparação).
    pub per_second_fps_stdev: Option<f64>,
    pub seconds: usize,
    /// Explica limitações (ex.: amostras periódicas não detectam picos isolados).
    pub resolution_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStat {
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,
    pub unit: String,
    pub samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub mode: CaptureMode,
    pub duration_s: f64,
    pub rows: usize,
    pub valid_rows: usize,
    pub frames: Option<u64>,
    pub fps: FpsStats,
    pub frametime: FrametimeStats,
    pub stability: Stability,
    pub resources: BTreeMap<Metric, ResourceStat>,
    /// Métricas presentes no log (com pelo menos um valor real).
    pub available: Vec<Metric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub key: String,
    pub metric: Metric,
    pub unit: String,
    /// Pontos `[segundos, valor]`, reduzidos para o gráfico (estatísticas usam os dados originais).
    pub points: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Conditions {
    /// Build Steam observado na captura, quando disponível. Legados ficam
    /// desconhecidos; o jogo aberto nunca preenche uma sessão antiga.
    #[serde(default)]
    pub game_build_id: Option<String>,
    pub proton: Option<String>,
    pub gpu: Option<String>,
    pub driver: Option<String>,
    pub kernel: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub cpu_scheduler: Option<String>,
    pub resolution: Option<String>,
    pub preset: Option<String>,
    pub fps_limit: Option<String>,
    pub vsync: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionSource {
    Import,
    Assisted,
    /// Dados de teste identificados; nunca entram no ranking real.
    Synthetic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    pub schema_version: u32,
    pub analysis_version: u32,
    pub id: String,
    pub app_id: Option<String>,
    pub game_name: Option<String>,
    pub label: String,
    pub notes: String,
    pub created_at: String,
    pub captured_at: Option<String>,
    pub source: SessionSource,
    pub original_name: Option<String>,
    pub original_path: Option<String>,
    pub raw_hash: String,
    pub raw_bytes: u64,
    pub format: LogFormat,
    pub mangohud_version: Option<String>,
    pub conditions: Conditions,
    pub stats: SessionStats,
    pub quality: Quality,
    #[serde(default)]
    pub fix_ids: Vec<String>,
    /// Correções do Fix Engine ativas durante a captura assistida. `None` =
    /// desconhecido (importações e sessões anteriores à v0.4).
    #[serde(default)]
    pub active_fixes: Option<Vec<crate::fix_engine::models::ActiveFixRef>>,
    /// Configuração do FRX na captura, como Smart Pack (desde os Smart Packs).
    /// Ausente em sessões antigas; pode ser declarada pelo usuário.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smart_pack: Option<crate::smart_packs::models::PackSnapshot>,
}

/// Sessão completa (metadados + séries do gráfico).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub meta: SessionMeta,
    pub series: Vec<Series>,
}

/// Dados informados pelo usuário ao importar/preparar.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInput {
    pub app_id: Option<String>,
    pub game_name: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub conditions: Conditions,
}
