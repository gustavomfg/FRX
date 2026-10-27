//! Performance Lab: logs reais do MangoHud, estatísticas, histórico e comparação.
//!
//! Somente leitura em relação ao sistema: nada aqui altera jogos, prefixos,
//! opções de inicialização da Steam ou a configuração global do MangoHud.

pub mod capture;
pub mod comparison;
pub mod detection;
pub mod metrics;
pub mod models;
pub mod parser;
pub mod persistence;

use std::path::Path;

use crate::time::now_iso;
use models::*;

#[derive(Debug, Clone, Default)]
pub struct ProcessArgs {
    pub input: SessionInput,
    pub source: Option<SessionSource>,
    pub original_path: Option<String>,
    /// Condições conhecidas pelo FRX (Proton atual, perfil do sistema).
    pub known: Conditions,
}

/// Extrai `2026-10-24_18-01-29` do nome padrão do MangoHud (`exe_AAAA-MM-DD_HH-MM-SS.csv`).
pub fn captured_at_from_name(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".csv").unwrap_or(name);
    let tail = stem.get(stem.len().checked_sub(19)?..)?;
    let b = tail.as_bytes();
    let digits = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    let ok = digits.iter().all(|i| b[*i].is_ascii_digit())
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'_'
        && b[13] == b'-'
        && b[16] == b'-';
    ok.then(|| {
        format!(
            "{}T{}:{}:{}",
            &tail[..10],
            &tail[11..13],
            &tail[14..16],
            &tail[17..19]
        )
    })
}

fn merge(base: Option<String>, over: Option<String>) -> Option<String> {
    over.map(|s| s.trim().chars().take(120).collect::<String>())
        .filter(|s| !s.is_empty())
        .or(base)
}

/// Lê, valida e analisa um log; não grava nada.
pub fn process(raw: &[u8], args: ProcessArgs) -> Result<Session, String> {
    let parsed = parser::parse(raw).map_err(|e| e.to_string())?;
    let analysis = metrics::analyze(&parsed);
    let sys = &parsed.system;
    // O perfil atual só descreve uma captura feita agora pelo FRX. Importar
    // um CSV antigo não confirma o Proton, driver ou hardware usados nele.
    let unknown = Conditions::default();
    let k = if args.source == Some(SessionSource::Assisted) {
        &args.known
    } else {
        &unknown
    };
    let i = args.input.conditions;
    // Ordem: o que o usuário informou > o que está no log > o que o FRX sabe agora.
    let conditions = Conditions {
        game_build_id: merge(k.game_build_id.clone(), i.game_build_id),
        proton: merge(k.proton.clone(), i.proton),
        gpu: merge(sys.gpu.clone().or(k.gpu.clone()), i.gpu),
        driver: merge(sys.driver.clone().or(k.driver.clone()), i.driver),
        kernel: merge(sys.kernel.clone().or(k.kernel.clone()), i.kernel),
        os: merge(sys.os.clone().or(k.os.clone()), i.os),
        cpu: merge(sys.cpu.clone().or(k.cpu.clone()), i.cpu),
        cpu_scheduler: merge(sys.cpu_scheduler.clone(), i.cpu_scheduler),
        resolution: merge(None, i.resolution),
        preset: merge(None, i.preset),
        fps_limit: merge(None, i.fps_limit),
        vsync: i.vsync,
    };
    let original_name = args
        .original_path
        .as_deref()
        .and_then(|p| Path::new(p).file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let captured_at = original_name.as_deref().and_then(captured_at_from_name);
    let game = args
        .input
        .game_name
        .clone()
        .filter(|g| !g.trim().is_empty());
    let label = args
        .input
        .label
        .clone()
        .map(|l| l.trim().chars().take(120).collect::<String>())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| {
            let when = captured_at.clone().unwrap_or_else(now_iso);
            format!(
                "{} · {}",
                game.clone().unwrap_or_else(|| "Benchmark".into()),
                when[..16.min(when.len())].replace('T', " ")
            )
        });
    let meta = SessionMeta {
        schema_version: SCHEMA_VERSION,
        analysis_version: ANALYSIS_VERSION,
        id: persistence::new_id(),
        app_id: args.input.app_id.filter(|a| crate::art::valid_app_id(a)),
        game_name: game,
        label,
        notes: args
            .input
            .notes
            .unwrap_or_default()
            .chars()
            .take(4000)
            .collect(),
        created_at: now_iso(),
        captured_at,
        source: args.source.unwrap_or(SessionSource::Import),
        original_name,
        original_path: args.original_path,
        raw_hash: persistence::content_hash(raw),
        raw_bytes: raw.len() as u64,
        format: parsed.format,
        mangohud_version: parsed.mangohud_version.clone(),
        conditions,
        stats: analysis.stats,
        quality: analysis.quality,
        fix_ids: vec![],
        active_fixes: None,
        smart_pack: None,
    };
    Ok(Session {
        meta,
        series: analysis.series,
    })
}

/// Lê um arquivo escolhido pelo usuário com as proteções necessárias.
pub fn read_log_file(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let real = std::fs::canonicalize(path).map_err(|_| "Arquivo não encontrado.".to_owned())?;
    let s = real.to_string_lossy();
    if s.starts_with("/proc/") || s.starts_with("/sys/") || s.starts_with("/dev/") {
        return Err("Esse caminho não pode ser importado.".into());
    }
    let meta = std::fs::metadata(&real).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("Selecione um arquivo, não uma pasta.".into());
    }
    let ext = real
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(ext.as_str(), "csv" | "txt" | "log") {
        return Err("Formato não suportado: escolha um .csv do MangoHud.".into());
    }
    if meta.len() as usize > parser::MAX_BYTES {
        return Err(parser::ParseError::TooLarge.to_string());
    }
    let mut raw = Vec::new();
    std::fs::File::open(&real)
        .map_err(|e| e.to_string())?
        .take(parser::MAX_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > parser::MAX_BYTES {
        return Err(parser::ParseError::TooLarge.to_string());
    }
    Ok(raw)
}

/// Exporta uma sessão processada (JSON completo ou CSV de estatísticas).
pub fn export(session: &Session, dest: &Path) -> Result<(), String> {
    let ext = dest
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if dest.is_dir() || !dest.parent().is_some_and(|p| p.is_dir()) {
        return Err("Destino inválido.".into());
    }
    let bytes = match ext.as_str() {
        "json" => serde_json::to_vec_pretty(session).map_err(|e| e.to_string())?,
        "csv" => export_csv(&session.meta).into_bytes(),
        _ => return Err("Use a extensão .json ou .csv.".into()),
    };
    std::fs::write(dest, bytes).map_err(|e| e.to_string())
}

fn export_csv(m: &SessionMeta) -> String {
    let mut out = String::from("metrica,valor,unidade\n");
    let mut row = |k: &str, v: Option<f64>, u: &str| {
        if let Some(v) = v {
            out.push_str(&format!("{k},{v:.3},{u}\n"));
        }
    };
    let s = &m.stats;
    row("duracao", Some(s.duration_s), "s");
    row("quadros", s.frames.map(|f| f as f64), "");
    row("fps_medio", s.fps.avg, "FPS");
    row("fps_mediano", s.fps.median, "FPS");
    row("low_1", s.fps.low1, "FPS");
    row("low_0_1", s.fps.low01, "FPS");
    row("frametime_medio", s.frametime.avg, "ms");
    row("frametime_p99", s.frametime.p99, "ms");
    row("frametime_p99_9", s.frametime.p999, "ms");
    row("picos", Some(s.stability.spikes as f64), "");
    for (metric, r) in &s.resources {
        row(
            &format!("{}_medio", metrics::label(*metric).replace(' ', "_")),
            Some(r.avg),
            &r.unit,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parser::fixtures::per_frame;
    use super::*;

    #[test]
    fn captured_at_comes_from_mangohud_file_name() {
        assert_eq!(
            captured_at_from_name("BatmanAC_2026-10-24_18-01-29.csv").as_deref(),
            Some("2026-10-24T18:01:29")
        );
        assert_eq!(captured_at_from_name("x.csv"), None);
        assert_eq!(
            captured_at_from_name("vkcube_2026-10-24_18-01-29_summary.csv"),
            None
        );
    }

    #[test]
    fn process_merges_conditions_in_the_right_order() {
        let raw = per_frame(&vec![10.0; 3000], 90.0);
        let s = process(
            raw.as_bytes(),
            ProcessArgs {
                input: SessionInput {
                    app_id: Some("200260".into()),
                    game_name: Some("Batman".into()),
                    conditions: Conditions {
                        preset: Some("Alto".into()),
                        kernel: Some(" ".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                known: Conditions {
                    proton: Some("GE-Proton9-20".into()),
                    gpu: Some("outra".into()),
                    ..Default::default()
                },
                original_path: Some("/tmp/BatmanAC_2026-10-24_18-01-29.csv".into()),
                source: Some(SessionSource::Assisted),
            },
        )
        .unwrap();
        let c = &s.meta.conditions;
        assert_eq!(
            c.gpu.as_deref(),
            Some("NVIDIA GeForce RTX 4060"),
            "o log vence o perfil atual"
        );
        assert_eq!(
            c.kernel.as_deref(),
            Some("7.3.0-4-cachyos"),
            "campo em branco do usuário não apaga"
        );
        assert_eq!(c.preset.as_deref(), Some("Alto"));
        assert_eq!(c.proton.as_deref(), Some("GE-Proton9-20"));
        assert_eq!(s.meta.captured_at.as_deref(), Some("2026-10-24T18:01:29"));
        assert!(s.meta.label.starts_with("Batman · 2026-10-24 18:01"));
    }

    #[test]
    fn imported_logs_never_inherit_the_current_computer_or_proton() {
        // Explicitly synthetic input to test import semantics, not real evidence.
        let raw = "frametime\n10\n10\n10\n";
        for source in [
            None,
            Some(SessionSource::Import),
            Some(SessionSource::Synthetic),
        ] {
            let s = process(
                raw.as_bytes(),
                ProcessArgs {
                    source,
                    known: Conditions {
                        proton: Some("Current-Proton".into()),
                        gpu: Some("Current GPU".into()),
                        driver: Some("Current driver".into()),
                        cpu: Some("Current CPU".into()),
                        ..Default::default()
                    },
                    input: SessionInput {
                        conditions: Conditions {
                            preset: Some("Explicit preset".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(s.meta.conditions.proton.is_none());
            assert!(s.meta.conditions.gpu.is_none());
            assert!(s.meta.conditions.driver.is_none());
            assert!(s.meta.conditions.cpu.is_none());
            assert_eq!(s.meta.conditions.preset.as_deref(), Some("Explicit preset"));
        }
    }

    #[test]
    fn read_log_file_guards() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_log_file(&tmp.path().join("nada.csv")).is_err());
        assert!(read_log_file(tmp.path()).is_err());
        let bin = tmp.path().join("x.exe");
        std::fs::write(&bin, "a").unwrap();
        assert!(read_log_file(&bin).is_err());
        assert!(read_log_file(Path::new("/proc/self/status")).is_err());
        let ok = tmp.path().join("log.csv");
        std::fs::write(&ok, "fps,frametime\n1,2\n").unwrap();
        assert!(read_log_file(&ok).is_ok());
    }

    #[test]
    fn export_json_and_csv() {
        let tmp = tempfile::tempdir().unwrap();
        let raw = per_frame(&vec![10.0; 3000], 90.0);
        let s = process(raw.as_bytes(), ProcessArgs::default()).unwrap();
        export(&s, &tmp.path().join("s.json")).unwrap();
        export(&s, &tmp.path().join("s.csv")).unwrap();
        let csv = std::fs::read_to_string(tmp.path().join("s.csv")).unwrap();
        assert!(csv.contains("fps_medio,100.000,FPS"));
        assert!(export(&s, &tmp.path().join("s.exe")).is_err());
        assert!(export(&s, &tmp.path().join("nao/existe.json")).is_err());
    }
}

/// Diagnóstico manual com um log real:
/// `FRX_LOG=/caminho/log.csv cargo test real_log -- --ignored --nocapture`
#[cfg(test)]
mod real {
    #[test]
    #[ignore]
    fn real_log() {
        let path = std::env::var("FRX_LOG").expect("defina FRX_LOG");
        let raw = super::read_log_file(std::path::Path::new(&path)).unwrap();
        let s = super::process(
            &raw,
            super::ProcessArgs {
                original_path: Some(path),
                ..Default::default()
            },
        )
        .unwrap();
        let m = &s.meta;
        println!(
            "{}",
            serde_json::to_string_pretty(&(
                &m.stats.fps,
                &m.stats.frametime.p99,
                &m.stats.duration_s,
                &m.stats.mode,
                &m.stats.stability.spikes,
                &m.quality,
                &m.conditions
            ))
            .unwrap()
        );
        println!(
            "series: {:?}",
            s.series
                .iter()
                .map(|x| (&x.key, x.points.len()))
                .collect::<Vec<_>>()
        );
    }
}
