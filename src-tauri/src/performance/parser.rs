//! Leitor de logs do MangoHud (e CSVs simples de frametime).
//!
//! Formato do MangoHud 0.6–0.8 (ver `src/logging.cpp` do projeto):
//!
//! ```text
//! v1                                  ← só com `log_versioning`
//! 0.8.4-1+                            ← versão (idem)
//! ---------------------SYSTEM INFO---------------------
//! os,cpu,gpu,ram,kernel,driver,cpuscheduler
//! CachyOS,AMD Ryzen 5 4600G,...       ← ram em KiB
//! --------------------FRAME METRICS--------------------
//! fps,frametime,cpu_load,...,elapsed  ← colunas variam entre versões
//! 177.8,5.62,6.4,...,16277187         ← elapsed em nanossegundos
//! ```
//!
//! O arquivo `*_summary.csv` é só um resumo, sem série temporal — é
//! recusado com uma mensagem que aponta para o log principal.

use std::collections::BTreeMap;

use super::models::{Issue, LogFormat, LogSystemInfo, Metric, Severity};

pub const MAX_BYTES: usize = 256 * 1024 * 1024;
const MAX_LINE: usize = 8 * 1024;
const MAX_COLUMNS: usize = 128;
const MAX_ROWS: usize = 5_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Empty,
    TooLarge,
    Binary,
    Summary,
    NoHeader,
    NoFrameData,
    NoRows,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ParseError::Empty => "O arquivo está vazio.",
            ParseError::TooLarge => "O arquivo passa do limite de 256 MB.",
            ParseError::Binary => "O arquivo não é um CSV de texto.",
            ParseError::Summary => "Este é o resumo (_summary.csv) do MangoHud. Importe o log principal, com o mesmo nome sem “_summary”.",
            ParseError::NoHeader => "Não encontramos o cabeçalho de colunas do MangoHud (fps, frametime, …).",
            ParseError::NoFrameData => "O log não tem coluna de frametime nem de FPS.",
            ParseError::NoRows => "O log tem cabeçalho, mas nenhuma linha de dados — a captura pode ter sido interrompida.",
        };
        f.write_str(msg)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLog {
    pub format: LogFormat,
    pub mangohud_version: Option<String>,
    pub system: LogSystemInfo,
    pub columns: BTreeMap<Metric, Vec<Option<f64>>>,
    /// Segundos desde o início do log, quando houver coluna de tempo.
    pub elapsed_s: Option<Vec<Option<f64>>>,
    pub rows: usize,
    /// Linhas descartadas por estrutura inválida (colunas a mais/menos, texto).
    pub malformed_rows: usize,
    pub issues: Vec<Issue>,
}

fn metric_for(name: &str) -> Option<Metric> {
    Some(match name {
        "fps" => Metric::Fps,
        "frametime" | "frametime_ms" | "frame_time" | "msbetweenpresents" => Metric::Frametime,
        "cpu_load" => Metric::CpuLoad,
        "gpu_load" => Metric::GpuLoad,
        "cpu_temp" => Metric::CpuTemp,
        "gpu_temp" => Metric::GpuTemp,
        "cpu_power" => Metric::CpuPower,
        "gpu_power" => Metric::GpuPower,
        "gpu_core_clock" => Metric::GpuCoreClock,
        "gpu_mem_clock" => Metric::GpuMemClock,
        "cpu_mhz" => Metric::CpuMhz,
        "gpu_vram_used" | "vram_used" | "vram" => Metric::Vram,
        "ram_used" | "ram" => Metric::Ram,
        "swap_used" => Metric::Swap,
        "process_rss" => Metric::ProcessRss,
        _ => return None,
    })
}

enum TimeCol {
    Nanos,
    Seconds,
}

fn clean(s: &str) -> Option<String> {
    let t = s.trim().trim_matches('"').trim();
    (!t.is_empty()).then(|| t.chars().filter(|c| !c.is_control()).take(200).collect())
}

fn num(s: &str) -> Option<f64> {
    let v: f64 = s.trim().trim_matches('"').parse().ok()?;
    v.is_finite().then_some(v)
}

pub fn parse(bytes: &[u8]) -> Result<ParsedLog, ParseError> {
    if bytes.is_empty() {
        return Err(ParseError::Empty);
    }
    if bytes.len() > MAX_BYTES {
        return Err(ParseError::TooLarge);
    }
    if bytes[..bytes.len().min(8192)].contains(&0) {
        return Err(ParseError::Binary);
    }
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text
        .lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| !l.trim().is_empty())
        .peekable();

    let mut issues = Vec::new();
    let mut version = None;
    let mut system = LogSystemInfo::default();
    let mut versioned = false;
    let mut header: Option<Vec<String>> = None;

    // Cabeçalhos até achar a linha de colunas de métricas.
    while let Some(line) = lines.next() {
        if line.len() > MAX_LINE {
            continue;
        }
        let lower = line.trim().to_ascii_lowercase();
        if lower.contains("0.1% min fps") || lower.contains("average fps") {
            return Err(ParseError::Summary);
        }
        if lower == "v1" {
            versioned = true;
            if let Some(v) = lines.peek() {
                if v.chars().next().is_some_and(|c| c.is_ascii_digit()) && v.len() < 40 {
                    version = clean(v);
                    lines.next();
                }
            }
            continue;
        }
        if lower.starts_with("---") {
            continue;
        }
        if lower.starts_with("os,") && lower.contains("cpu") {
            let keys: Vec<String> = lower.split(',').map(|s| s.trim().to_owned()).collect();
            if let Some(vals) = lines.next() {
                let vals: Vec<&str> = vals.split(',').collect();
                // Nomes de CPU às vezes têm vírgulas: o excesso vai para o campo "cpu".
                let extra = vals.len().saturating_sub(keys.len());
                let mut fields: Vec<String> = Vec::new();
                let mut i = 0;
                for k in &keys {
                    let take = if k == "cpu" { 1 + extra } else { 1 };
                    fields.push(
                        vals.get(i..i + take)
                            .map(|s| s.join(","))
                            .unwrap_or_default(),
                    );
                    i += take;
                }
                for (k, v) in keys.iter().zip(fields) {
                    let v = clean(&v);
                    match k.as_str() {
                        "os" => system.os = v,
                        "cpu" => system.cpu = v,
                        "gpu" => system.gpu = v,
                        "ram" => system.ram_kib = v.and_then(|s| s.parse().ok()),
                        "kernel" => system.kernel = v,
                        "driver" => system.driver = v,
                        "cpuscheduler" => system.cpu_scheduler = v,
                        _ => {}
                    }
                }
            }
            continue;
        }
        let cols: Vec<String> = lower
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_owned())
            .collect();
        if cols.len() <= MAX_COLUMNS
            && cols
                .iter()
                .any(|c| metric_for(c) == Some(Metric::Frametime) || c == "fps")
        {
            header = Some(cols);
            break;
        }
    }
    let header = header.ok_or(ParseError::NoHeader)?;

    let mut index: Vec<(usize, Metric)> = Vec::new();
    let mut time_col: Option<(usize, TimeCol)> = None;
    for (i, c) in header.iter().enumerate() {
        if let Some(m) = metric_for(c) {
            if !index.iter().any(|(_, x)| *x == m) {
                index.push((i, m));
            }
        } else if c == "elapsed" {
            time_col = Some((i, TimeCol::Nanos));
        } else if c == "timeinseconds" || c == "time_s" {
            time_col = Some((i, TimeCol::Seconds));
        }
    }
    if !index
        .iter()
        .any(|(_, m)| matches!(m, Metric::Frametime | Metric::Fps))
    {
        return Err(ParseError::NoFrameData);
    }
    let format =
        if header.iter().any(|c| c == "msbetweenpresents") || !header.iter().any(|c| c == "fps") {
            LogFormat::GenericFrametime
        } else if versioned {
            LogFormat::MangohudV1
        } else {
            LogFormat::Mangohud
        };

    let mut columns: BTreeMap<Metric, Vec<Option<f64>>> =
        index.iter().map(|(_, m)| (*m, Vec::new())).collect();
    let mut elapsed_raw: Vec<Option<f64>> = Vec::new();
    let mut rows = 0usize;
    let mut malformed = 0usize;
    for line in lines {
        if rows >= MAX_ROWS {
            issues.push(Issue::new(
                Severity::Warning,
                "truncated",
                format!("O log passa de {MAX_ROWS} linhas; o restante foi ignorado."),
            ));
            break;
        }
        if line.len() > MAX_LINE || line.trim_start().starts_with("---") {
            malformed += 1;
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != header.len() {
            malformed += 1;
            continue;
        }
        for (i, m) in &index {
            columns
                .get_mut(m)
                .expect("coluna registrada")
                .push(num(fields[*i]));
        }
        if let Some((i, _)) = &time_col {
            elapsed_raw.push(num(fields[*i]));
        }
        rows += 1;
    }
    if rows == 0 {
        return Err(ParseError::NoRows);
    }
    if malformed > 0 {
        issues.push(Issue::new(
            Severity::Warning,
            "malformed-rows",
            format!("{malformed} linha(s) com estrutura inválida foram ignoradas."),
        ));
    }

    let elapsed_s = time_col.map(|(_, unit)| {
        let factor = match unit {
            TimeCol::Seconds => 1.0,
            TimeCol::Nanos => {
                infer_time_factor(&elapsed_raw, columns.get(&Metric::Frametime), &mut issues)
            }
        };
        elapsed_raw.iter().map(|v| v.map(|x| x * factor)).collect()
    });

    Ok(ParsedLog {
        format,
        mangohud_version: version,
        system,
        columns,
        elapsed_s,
        rows,
        malformed_rows: malformed,
        issues,
    })
}

/// O MangoHud grava `elapsed` em nanossegundos. Validamos isso contra a soma dos
/// frametimes; se outra unidade explicar muito melhor os dados, ela é usada e o
/// fato fica registrado.
fn infer_time_factor(
    elapsed: &[Option<f64>],
    frametime: Option<&Vec<Option<f64>>>,
    issues: &mut Vec<Issue>,
) -> f64 {
    let vals: Vec<f64> = elapsed.iter().flatten().copied().collect();
    let (Some(first), Some(last)) = (vals.first(), vals.last()) else {
        return 1e-9;
    };
    let span = last - first;
    if span <= 0.0 {
        return 1e-9;
    }
    let Some(ft) = frametime else { return 1e-9 };
    let sum_s: f64 = ft
        .iter()
        .flatten()
        .filter(|v| **v > 0.0 && **v <= 10_000.0)
        .sum::<f64>()
        / 1000.0;
    if sum_s <= 0.0 {
        return 1e-9;
    }
    let score = |f: f64| ((span * f) / sum_s).ln().abs();
    let ns = score(1e-9);
    let best = [1e-9, 1e-6, 1e-3, 1.0]
        .into_iter()
        .min_by(|a, b| score(*a).total_cmp(&score(*b)))
        .unwrap_or(1e-9);
    // Só troca de unidade se nanossegundos for implausível (>×50 de diferença) e a outra encaixar.
    if best != 1e-9 && ns > 50f64.ln() && score(best) < 2f64.ln() {
        let name = if best == 1e-6 {
            "microssegundos"
        } else if best == 1e-3 {
            "milissegundos"
        } else {
            "segundos"
        };
        issues.push(Issue::new(Severity::Warning, "time-unit", format!("A coluna elapsed parece estar em {name}, não em nanossegundos; a unidade foi ajustada.")));
        return best;
    }
    1e-9
}

#[cfg(test)]
pub mod fixtures {
    //! Logs sintéticos com resultados conhecidos.

    /// Cabeçalho idêntico ao do MangoHud 0.8.4 com `log_versioning`.
    pub const V1_HEADER: &str = "v1\n0.8.4-1+\n---------------------SYSTEM INFO---------------------\nos,cpu,gpu,ram,kernel,driver,cpuscheduler\nCachyOS,AMD Ryzen 5 4600G with Radeon Graphics,NVIDIA GeForce RTX 4060,15717688,7.3.0-4-cachyos,,schedutil\n--------------------FRAME METRICS--------------------\nfps,frametime,cpu_load,cpu_power,gpu_load,cpu_temp,gpu_temp,gpu_core_clock,gpu_mem_clock,gpu_vram_used,gpu_power,ram_used,swap_used,process_rss,cpu_mhz,elapsed\n";

    /// Log por quadro: `frametimes` em ms, sensores constantes, elapsed em ns.
    pub fn per_frame(frametimes: &[f64], gpu_load: f64) -> String {
        let mut s = String::from(V1_HEADER);
        let mut t = 0.0f64;
        for ft in frametimes {
            t += ft;
            s.push_str(&format!(
                "{:.3},{ft},25,0,{gpu_load},55,62,1800,8000,4.5,95,8.2,0,0,4100,{}\n",
                1000.0 / ft,
                (t * 1e6) as u64
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn parses_mangohud_v1_with_system_info() {
        let log = per_frame(&[10.0, 20.0, 10.0], 90.0);
        let p = parse(log.as_bytes()).unwrap();
        assert_eq!(p.format, LogFormat::MangohudV1);
        assert_eq!(p.mangohud_version.as_deref(), Some("0.8.4-1+"));
        assert_eq!(p.system.gpu.as_deref(), Some("NVIDIA GeForce RTX 4060"));
        assert_eq!(p.system.ram_kib, Some(15_717_688));
        assert_eq!(
            p.system.driver, None,
            "campo vazio é nulo, não string vazia"
        );
        assert_eq!(p.rows, 3);
        assert_eq!(
            p.columns[&Metric::Frametime],
            vec![Some(10.0), Some(20.0), Some(10.0)]
        );
        let e = p.elapsed_s.unwrap();
        assert!((e[2].unwrap() - 0.040).abs() < 1e-9, "ns → s");
    }

    #[test]
    fn parses_legacy_header_without_versioning_and_fewer_columns() {
        let log = "os,cpu,gpu,ram,kernel,driver\nArch,Ryzen, 7 5800X,RX 6800,32000000,6.1,Mesa 23\nfps,frametime,cpu_load,gpu_load,cpu_temp,gpu_temp,gpu_core_clock,gpu_mem_clock,gpu_vram_used,gpu_power,ram_used,elapsed\n60,16.6,30,99,60,70,2000,1000,6,200,9,16600000\n";
        let p = parse(log.as_bytes()).unwrap();
        assert_eq!(p.format, LogFormat::Mangohud);
        assert_eq!(
            p.system.cpu.as_deref(),
            Some("Ryzen, 7 5800X"),
            "vírgula no nome da CPU"
        );
        assert!(p.columns.contains_key(&Metric::GpuLoad));
        assert!(
            !p.columns.contains_key(&Metric::CpuPower),
            "coluna ausente não vira zero"
        );
    }

    #[test]
    fn accepts_presentmon_style_frametime_csv() {
        let log = "Application,ProcessID,TimeInSeconds,MsBetweenPresents\ngame.exe,1,0.0,16.6\ngame.exe,1,0.0166,16.8\n";
        let p = parse(log.as_bytes()).unwrap();
        assert_eq!(p.format, LogFormat::GenericFrametime);
        assert_eq!(p.columns[&Metric::Frametime], vec![Some(16.6), Some(16.8)]);
        assert_eq!(p.elapsed_s.unwrap()[1], Some(0.0166));
    }

    #[test]
    fn rejects_bad_inputs_with_clear_errors() {
        assert_eq!(parse(b""), Err(ParseError::Empty));
        assert_eq!(parse(b"\x00\x01binary"), Err(ParseError::Binary));
        assert_eq!(
            parse(b"0.1% Min FPS,1% Min FPS,Average FPS\n1,2,3\n"),
            Err(ParseError::Summary)
        );
        assert_eq!(parse(b"a,b,c\n1,2,3\n"), Err(ParseError::NoHeader));
        assert_eq!(parse(V1_HEADER.as_bytes()), Err(ParseError::NoRows));
    }

    #[test]
    fn malformed_rows_and_garbage_values_are_counted_not_zeroed() {
        let mut log = per_frame(&[10.0, 10.0], 50.0);
        log.push_str("1,2,3\n");
        log.push_str("abc,NaN,x,0,0,0,0,0,0,0,0,0,0,0,0,999\n");
        let p = parse(log.as_bytes()).unwrap();
        assert_eq!(p.rows, 3);
        assert_eq!(p.malformed_rows, 1);
        assert_eq!(p.columns[&Metric::Fps][2], None);
        assert_eq!(p.columns[&Metric::Frametime][2], None);
    }

    #[test]
    fn infers_time_unit_only_when_nanoseconds_are_implausible() {
        // elapsed escrito em milissegundos por outra ferramenta
        let mut log = String::from("fps,frametime,elapsed\n");
        let mut t = 0.0;
        for _ in 0..200 {
            t += 10.0;
            log.push_str(&format!("100,10,{t}\n"));
        }
        let p = parse(log.as_bytes()).unwrap();
        assert!(p.issues.iter().any(|i| i.code == "time-unit"));
        assert!((p.elapsed_s.unwrap()[199].unwrap() - 2.0).abs() < 1e-9);
    }
}
