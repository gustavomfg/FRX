//! Estatísticas, estabilidade, qualidade e séries para o gráfico.
//!
//! Metodologia:
//! - **Modo por quadro** (MangoHud com `log_interval=0`): cada linha é um
//!   quadro. Detectado quando a soma dos frametimes cobre o tempo do log.
//! - **FPS médio** = quadros ÷ tempo total (média ponderada pelo tempo), não a
//!   média dos FPS instantâneos.
//! - **1% low** = 1000 ÷ média dos 1% maiores frametimes; **0.1% low** idem com
//!   0,1%. Exigem pelo menos 100 e 1000 quadros, respectivamente.
//! - **Modo periódico** (`log_interval > 0`): cada linha é uma amostra do
//!   último quadro. Os lows viram estimativas (percentis das amostras) e ficam
//!   marcados como tal; o 0.1% low só é estimado com ≥ 1000 amostras.
//! - **Percentis** usam o método nearest-rank sobre os dados válidos originais.
//! - **Picos**: quadro com frametime ≥ 2× a mediana local (31 quadros) e pelo
//!   menos 8 ms acima dela. Um pico isolado não é tratado como problema.

use std::collections::BTreeMap;

use super::models::*;
use super::parser::ParsedLog;

const MAX_POINTS: usize = 1200;
const SPIKE_RATIO: f64 = 2.0;
const SPIKE_MIN_MS: f64 = 8.0;

fn mean(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn stdev(v: &[f64]) -> Option<f64> {
    let m = mean(v)?;
    Some((v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt())
}

/// Percentil nearest-rank (p em 0–100) sobre dados já ordenados.
pub fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    Some(sorted[rank.min(sorted.len()) - 1])
}

/// Média dos `fraction` piores frametimes (maiores).
fn worst_mean(sorted_ft: &[f64], fraction: f64) -> f64 {
    let n = ((sorted_ft.len() as f64 * fraction).ceil() as usize).max(1);
    let tail = &sorted_ft[sorted_ft.len() - n..];
    tail.iter().sum::<f64>() / n as f64
}

fn local_medians(ft: &[f64], half: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(ft.len());
    let mut win: Vec<f64> = Vec::with_capacity(half * 2 + 1);
    for i in 0..ft.len() {
        let a = i.saturating_sub(half);
        let b = (i + half + 1).min(ft.len());
        win.clear();
        win.extend_from_slice(&ft[a..b]);
        win.sort_by(f64::total_cmp);
        out.push(win[win.len() / 2]);
    }
    out
}

pub struct Analysis {
    pub stats: SessionStats,
    pub quality: Quality,
    pub series: Vec<Series>,
}

/// Limpa uma coluna de sensor: sempre zero → indisponível; zeros antes do
/// primeiro valor real → aquecimento (nulo); fora da faixa física → nulo.
fn clean_sensor(
    m: Metric,
    col: &[Option<f64>],
    issues: &mut Vec<Issue>,
) -> Option<Vec<Option<f64>>> {
    let first_real = col.iter().position(|v| v.is_some_and(|x| x != 0.0))?;
    let mut implausible = 0;
    let out: Vec<Option<f64>> = col
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let v = (*v)?;
            if i < first_real {
                return None;
            }
            if !m.plausible(v) {
                implausible += 1;
                return None;
            }
            Some(v)
        })
        .collect();
    if implausible > 0 {
        issues.push(Issue::new(
            Severity::Warning,
            "implausible-values",
            format!(
                "{implausible} valor(es) impossíveis de {} foram ignorados.",
                m.unit()
            ),
        ));
    }
    Some(out)
}

pub fn analyze(p: &ParsedLog) -> Analysis {
    let mut issues = p.issues.clone();
    let n = p.rows;

    // --- Frametime / FPS por linha -----------------------------------------
    let ft_col = p.columns.get(&Metric::Frametime);
    let fps_col = p.columns.get(&Metric::Fps);
    let mut ft_row: Vec<Option<f64>> = (0..n)
        .map(|i| {
            let ft = ft_col.and_then(|c| c[i]).or_else(|| {
                fps_col
                    .and_then(|c| c[i])
                    .filter(|f| *f > 0.0)
                    .map(|f| 1000.0 / f)
            });
            ft.filter(|v| Metric::Frametime.plausible(*v))
        })
        .collect();
    let invalid_ft = ft_row.iter().filter(|v| v.is_none()).count();

    // --- Eixo de tempo ------------------------------------------------------
    let mut nonmono = 0;
    let times: Vec<Option<f64>> = match &p.elapsed_s {
        Some(e) => {
            let mut last = f64::NEG_INFINITY;
            e.iter()
                .map(|t| {
                    let t = (*t)?;
                    if t < last {
                        nonmono += 1;
                        return None;
                    }
                    last = t;
                    Some(t)
                })
                .collect()
        }
        None => {
            let mut acc = 0.0;
            ft_row
                .iter()
                .map(|ft| {
                    let t = acc;
                    acc += ft.unwrap_or(0.0) / 1000.0;
                    Some(t)
                })
                .collect()
        }
    };
    if nonmono > 0 {
        issues.push(Issue::new(
            Severity::Warning,
            "time-order",
            format!("{nonmono} linha(s) com tempo fora de ordem foram ignoradas."),
        ));
        for (i, t) in times.iter().enumerate() {
            if t.is_none() {
                ft_row[i] = None;
            }
        }
    }
    if p.elapsed_s.is_none() {
        issues.push(Issue::new(Severity::Info, "no-elapsed", "O log não tem coluna de tempo; a linha do tempo foi reconstruída somando os frametimes."));
    }

    let valid: Vec<(f64, f64)> = ft_row
        .iter()
        .zip(&times)
        .filter_map(|(ft, t)| Some(((*t)?, (*ft)?)))
        .collect();
    let fts: Vec<f64> = valid.iter().map(|(_, f)| *f).collect();
    let t_first = valid.first().map(|(t, _)| *t).unwrap_or(0.0);
    let t_last = valid.last().map(|(t, _)| *t).unwrap_or(0.0);
    let sum_ft_s = fts.iter().sum::<f64>() / 1000.0;
    let span = (t_last - t_first).max(0.0);

    // Por quadro se os frametimes cobrem o tempo do log (ou se o tempo veio deles).
    let coverage = if span > 0.0 { sum_ft_s / span } else { 1.0 };
    let mode = if p.elapsed_s.is_none() || (0.6..=1.6).contains(&coverage) || fts.len() < 3 {
        CaptureMode::PerFrame
    } else {
        CaptureMode::Periodic
    };
    let duration_s = match mode {
        CaptureMode::PerFrame if p.elapsed_s.is_some() => {
            span + fts.first().copied().unwrap_or(0.0) / 1000.0
        }
        CaptureMode::PerFrame => sum_ft_s,
        CaptureMode::Periodic => span,
    };

    // --- FPS ----------------------------------------------------------------
    let mut sorted = fts.clone();
    sorted.sort_by(f64::total_cmp);
    let mut fps = FpsStats::default();
    let mut ft = FrametimeStats::default();
    if !fts.is_empty() {
        match mode {
            CaptureMode::PerFrame => {
                fps.avg = (sum_ft_s > 0.0).then(|| fts.len() as f64 / sum_ft_s);
                fps.median = percentile(&sorted, 50.0).map(|m| 1000.0 / m);
                fps.min = sorted.last().map(|m| 1000.0 / m);
                fps.max = sorted.first().map(|m| 1000.0 / m);
                fps.low1 = (fts.len() >= 100).then(|| 1000.0 / worst_mean(&sorted, 0.01));
                fps.low01 = (fts.len() >= 1000).then(|| 1000.0 / worst_mean(&sorted, 0.001));
            }
            CaptureMode::Periodic => {
                // Amostras de FPS (coluna fps quando existe; senão, 1000/frametime).
                let samples: Vec<(f64, f64)> = (0..n)
                    .filter_map(|i| {
                        let t = times[i]?;
                        let f = fps_col
                            .and_then(|c| c[i])
                            .filter(|f| Metric::Fps.plausible(*f) && *f > 0.0)
                            .or(ft_row[i].map(|x| 1000.0 / x))?;
                        Some((t, f))
                    })
                    .collect();
                let mut fs: Vec<f64> = samples.iter().map(|(_, f)| *f).collect();
                // média ponderada pelo intervalo até a próxima amostra
                let mut wsum = 0.0;
                let mut w = 0.0;
                for k in 0..samples.len().saturating_sub(1) {
                    let dt = samples[k + 1].0 - samples[k].0;
                    wsum += samples[k].1 * dt;
                    w += dt;
                }
                fps.avg = if w > 0.0 { Some(wsum / w) } else { mean(&fs) };
                fs.sort_by(f64::total_cmp);
                fps.median = percentile(&fs, 50.0);
                fps.min = fs.first().copied();
                fps.max = fs.last().copied();
                fps.low1 = (fs.len() >= 100).then(|| percentile(&fs, 1.0)).flatten();
                fps.low01 = (fs.len() >= 1000).then(|| percentile(&fs, 0.1)).flatten();
                fps.lows_estimated = true;
                issues.push(Issue::new(
                    Severity::Warning,
                    "periodic",
                    "Log com amostras periódicas (log_interval > 0): 1% e 0,1% low são estimativas pelas amostras, não pelos quadros reais. Para medir quadro a quadro, use log_interval=0.",
                ));
            }
        }
        ft.avg = mean(&fts);
        ft.median = percentile(&sorted, 50.0);
        ft.p90 = percentile(&sorted, 90.0);
        ft.p95 = percentile(&sorted, 95.0);
        ft.p99 = (sorted.len() >= 100)
            .then(|| percentile(&sorted, 99.0))
            .flatten();
        ft.p999 = (sorted.len() >= 1000)
            .then(|| percentile(&sorted, 99.9))
            .flatten();
        ft.max = sorted.last().copied();
        ft.stdev = stdev(&fts);
        ft.histogram = histogram(&fts);
    }

    // --- Estabilidade -------------------------------------------------------
    let stability = stability(&valid, mode, duration_s);

    // --- Sensores -----------------------------------------------------------
    let mut resources = BTreeMap::new();
    let mut available: Vec<Metric> = Vec::new();
    let mut sensor_cols: BTreeMap<Metric, Vec<Option<f64>>> = BTreeMap::new();
    let mut unavailable: Vec<&str> = Vec::new();
    for (m, col) in &p.columns {
        if !m.is_sensor() {
            continue;
        }
        match clean_sensor(*m, col, &mut issues) {
            Some(c) => {
                let mut vals: Vec<f64> = c.iter().flatten().copied().collect();
                if vals.is_empty() {
                    continue;
                }
                let avg = mean(&vals).unwrap_or(0.0);
                vals.sort_by(f64::total_cmp);
                resources.insert(
                    *m,
                    ResourceStat {
                        avg,
                        min: vals[0],
                        max: vals[vals.len() - 1],
                        p95: percentile(&vals, 95.0).unwrap_or(avg),
                        unit: m.unit().into(),
                        samples: vals.len(),
                    },
                );
                available.push(*m);
                sensor_cols.insert(*m, c);
            }
            None => unavailable.push(label(*m)),
        }
    }
    if !unavailable.is_empty() {
        issues.push(Issue::new(
            Severity::Info,
            "unavailable-metrics",
            format!("Sem dados (sempre 0 ou ausente) para: {}. Esses campos ficam vazios, não como zero.", unavailable.join(", ")),
        ));
    }
    if !fts.is_empty() {
        available.insert(0, Metric::Frametime);
        available.insert(0, Metric::Fps);
    }

    // --- Qualidade ----------------------------------------------------------
    let total = n;
    if invalid_ft > 0 {
        let share = invalid_ft as f64 / total.max(1) as f64;
        let sev = if share > 0.2 {
            Severity::Error
        } else {
            Severity::Warning
        };
        issues.push(Issue::new(sev, "invalid-frames", format!("{invalid_ft} linha(s) com frametime ausente ou impossível (≤ 0 ou > 10 s) ficaram fora das estatísticas ({}%).", format!("{:.1}", share * 100.0).replace('.', ","))));
    }
    if fts.is_empty() {
        issues.push(Issue::new(
            Severity::Error,
            "no-data",
            "Nenhum quadro válido no log.",
        ));
    } else {
        match mode {
            CaptureMode::PerFrame if fts.len() < 300 => issues.push(Issue::new(
                Severity::Error,
                "few-frames",
                format!(
                    "Só {} quadros válidos — pouco para estatísticas confiáveis (mínimo 300).",
                    fts.len()
                ),
            )),
            CaptureMode::Periodic if fts.len() < 30 => issues.push(Issue::new(
                Severity::Error,
                "few-samples",
                format!(
                    "Só {} amostras — pouco para comparar (mínimo 30).",
                    fts.len()
                ),
            )),
            _ => {}
        }
        // O MangoHud encerra entre quadros: uma captura de 10 s pode medir 9,99 s.
        // A tolerância só afeta a classificação, nunca os números registrados.
        if duration_s + 0.1 < 10.0 {
            issues.push(Issue::new(
                Severity::Error,
                "too-short",
                format!(
                    "A captura durou {} s; o mínimo para comparação é 10 s.",
                    format!("{duration_s:.1}").replace('.', ",")
                ),
            ));
        } else if duration_s + 0.1 < 30.0 {
            issues.push(Issue::new(
                Severity::Warning,
                "short",
                format!(
                    "Captura curta ({duration_s:.0} s). Benchmarks de 60 s ou mais variam menos."
                ),
            ));
        }
        if let Some((largest, count)) = gaps(&valid, mode) {
            issues.push(Issue::new(Severity::Warning, "gaps", format!("{count} lacuna(s) na linha do tempo; a maior tem {} s (jogo pausado, minimizado ou carregando).", format!("{largest:.1}").replace('.', ","))));
        }
    }
    let level = if issues.iter().any(|i| i.severity == Severity::Error) {
        QualityLevel::Unsuitable
    } else if issues.iter().any(|i| i.severity == Severity::Warning) {
        QualityLevel::Usable
    } else {
        QualityLevel::Good
    };

    let series = build_series(&valid, mode, &times, &sensor_cols, fps_col);
    Analysis {
        stats: SessionStats {
            mode,
            duration_s,
            rows: total,
            valid_rows: fts.len(),
            frames: (mode == CaptureMode::PerFrame).then_some(fts.len() as u64),
            fps,
            frametime: ft,
            stability,
            resources,
            available,
        },
        quality: Quality {
            level,
            issues,
            total_rows: total + p.malformed_rows,
            excluded_rows: invalid_ft + p.malformed_rows,
        },
        series,
    }
}

pub fn label(m: Metric) -> &'static str {
    match m {
        Metric::Fps => "FPS",
        Metric::Frametime => "frametime",
        Metric::CpuLoad => "uso de CPU",
        Metric::GpuLoad => "uso de GPU",
        Metric::CpuTemp => "temperatura da CPU",
        Metric::GpuTemp => "temperatura da GPU",
        Metric::CpuPower => "consumo da CPU",
        Metric::GpuPower => "consumo da GPU",
        Metric::GpuCoreClock => "clock da GPU",
        Metric::GpuMemClock => "clock da memória da GPU",
        Metric::CpuMhz => "clock da CPU",
        Metric::Vram => "VRAM",
        Metric::Ram => "RAM",
        Metric::Swap => "swap",
        Metric::ProcessRss => "memória do processo",
    }
}

fn histogram(fts: &[f64]) -> Vec<HistogramBucket> {
    let edges: [(&str, f64, Option<f64>); 5] = [
        ("≥ 120 FPS", 0.0, Some(1000.0 / 120.0)),
        ("60–120 FPS", 1000.0 / 120.0, Some(1000.0 / 60.0)),
        ("30–60 FPS", 1000.0 / 60.0, Some(1000.0 / 30.0)),
        ("20–30 FPS", 1000.0 / 30.0, Some(50.0)),
        ("< 20 FPS", 50.0, None),
    ];
    let total: f64 = fts.iter().sum();
    edges
        .iter()
        .map(|(label, from, to)| {
            let inb = |f: &f64| *f > *from && to.is_none_or(|t| *f <= t);
            let sum: f64 = fts.iter().filter(|f| inb(f)).sum();
            HistogramBucket {
                label: (*label).into(),
                from_ms: *from,
                to_ms: *to,
                // `+ 0.0` normaliza o -0.0 que a soma vazia de floats produz.
                time_share: if total > 0.0 { sum / total + 0.0 } else { 0.0 },
                count: fts.iter().filter(|f| inb(f)).count() as u64,
            }
        })
        .collect()
}

fn gaps(valid: &[(f64, f64)], mode: CaptureMode) -> Option<(f64, usize)> {
    if valid.len() < 3 {
        return None;
    }
    let mut deltas: Vec<f64> = valid.windows(2).map(|w| w[1].0 - w[0].0).collect();
    let mut sorted = deltas.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    let limit = match mode {
        CaptureMode::PerFrame => 1.0f64.max(median * 20.0),
        CaptureMode::Periodic => 2.0f64.max(median * 5.0),
    };
    deltas.retain(|d| *d > limit);
    let largest = deltas.iter().copied().fold(0.0, f64::max);
    (!deltas.is_empty()).then_some((largest, deltas.len()))
}

fn stability(valid: &[(f64, f64)], mode: CaptureMode, duration_s: f64) -> Stability {
    let mut s = Stability::default();
    if valid.len() < 3 {
        return s;
    }
    let fts: Vec<f64> = valid.iter().map(|(_, f)| *f).collect();
    let half = if mode == CaptureMode::PerFrame { 15 } else { 4 };
    let med = local_medians(&fts, half);
    let spike: Vec<bool> = fts
        .iter()
        .zip(&med)
        .map(|(f, m)| *f >= m * SPIKE_RATIO && f - m >= SPIKE_MIN_MS)
        .collect();
    s.spikes = spike.iter().filter(|x| **x).count() as u32;
    s.spikes_per_min = (duration_s >= 10.0).then(|| s.spikes as f64 / (duration_s / 60.0));
    s.cv = match (mean(&fts), stdev(&fts)) {
        (Some(m), Some(sd)) if m > 0.0 => Some(sd / m),
        _ => None,
    };
    if mode == CaptureMode::Periodic {
        s.resolution_note = Some(
            "Amostras periódicas: picos entre uma amostra e outra não aparecem nos dados.".into(),
        );
    }

    // Janelas de 1 s.
    let t0 = valid[0].0;
    let mut windows: BTreeMap<i64, (Vec<f64>, u32)> = BTreeMap::new();
    for ((t, f), sp) in valid.iter().zip(&spike) {
        let e = windows.entry(((t - t0).floor()) as i64).or_default();
        e.0.push(*f);
        if *sp {
            e.1 += 1;
        }
    }
    let mut per_sec_fps = Vec::new();
    let mut unstable: Vec<(i64, u32, f64, f64)> = Vec::new();
    for (sec, (w, spikes)) in &windows {
        let sum: f64 = w.iter().sum();
        let wfps = match mode {
            CaptureMode::PerFrame if sum >= 500.0 => Some(w.len() as f64 / (sum / 1000.0)),
            CaptureMode::Periodic => mean(&w.iter().map(|f| 1000.0 / f).collect::<Vec<_>>()),
            _ => None,
        };
        if let Some(f) = wfps {
            per_sec_fps.push(f);
        }
        // Oscilação medida sem o pior quadro, para um pico isolado não "contaminar" o segundo.
        let mut trimmed = w.clone();
        if trimmed.len() > 3 {
            let worst_i = trimmed
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            trimmed.swap_remove(worst_i);
        }
        let (cv, sd_ms) = match (mean(&trimmed), stdev(&trimmed)) {
            (Some(m), Some(sd)) if m > 0.0 => (sd / m, sd),
            _ => (0.0, 0.0),
        };
        let worst = w.iter().copied().fold(0.0, f64::max);
        let enough = w.len() >= if mode == CaptureMode::PerFrame { 10 } else { 2 };
        // Oscilação só conta quando é perceptível (≥ 2 ms de desvio), não em frametimes de 0,1 ms.
        if enough && (*spikes >= 2 || (cv >= 0.35 && sd_ms >= 2.0)) {
            unstable.push((*sec, *spikes, worst, cv));
        }
    }
    s.seconds = windows.len();
    s.per_second_fps_stdev = (per_sec_fps.len() >= 5)
        .then(|| stdev(&per_sec_fps))
        .flatten();

    // Junta segundos consecutivos em intervalos.
    let mut intervals: Vec<Interval> = Vec::new();
    for (sec, spikes, worst, cv) in unstable {
        let start = t0 + sec as f64;
        match intervals.last_mut() {
            Some(last) if (start - last.end_s).abs() < 1e-6 => {
                last.end_s = start + 1.0;
                last.spikes += spikes;
                last.worst_ms = last.worst_ms.max(worst);
                last.cv = last.cv.max(cv);
            }
            _ => intervals.push(Interval {
                start_s: start,
                end_s: start + 1.0,
                spikes,
                worst_ms: worst,
                cv,
            }),
        }
    }
    intervals.sort_by(|a, b| {
        b.spikes
            .cmp(&a.spikes)
            .then(b.worst_ms.total_cmp(&a.worst_ms))
    });
    intervals.truncate(8);
    intervals.sort_by(|a, b| a.start_s.total_cmp(&b.start_s));
    s.unstable_intervals = intervals;

    // Tendência: primeiro × último terço.
    if duration_s >= 15.0 {
        let third = (valid[valid.len() - 1].0 - t0) / 3.0;
        let fps_of = |a: f64, b: f64| {
            let w: Vec<f64> = valid
                .iter()
                .filter(|(t, _)| *t - t0 >= a && *t - t0 < b)
                .map(|(_, f)| *f)
                .collect();
            match mode {
                CaptureMode::PerFrame => {
                    (!w.is_empty()).then(|| w.len() as f64 / (w.iter().sum::<f64>() / 1000.0))
                }
                CaptureMode::Periodic => mean(&w.iter().map(|f| 1000.0 / f).collect::<Vec<_>>()),
            }
        };
        s.first_third_fps = fps_of(0.0, third);
        s.last_third_fps = fps_of(2.0 * third, f64::INFINITY);
        if let (Some(a), Some(b)) = (s.first_third_fps, s.last_third_fps) {
            s.trend_pct = (a > 0.0).then(|| (b - a) / a * 100.0);
        }
    }
    s
}

/// Reduz a série para o gráfico sem perder picos: por janela de tempo, FPS
/// médio real (quadros ÷ tempo), frametime médio e o pior frametime.
fn build_series(
    valid: &[(f64, f64)],
    mode: CaptureMode,
    times: &[Option<f64>],
    sensors: &BTreeMap<Metric, Vec<Option<f64>>>,
    fps_col: Option<&Vec<Option<f64>>>,
) -> Vec<Series> {
    let mut out = Vec::new();
    if valid.is_empty() {
        return out;
    }
    let t0 = valid[0].0;
    let t1 = valid[valid.len() - 1].0;
    // Colunas de ~0,25 s (no mínimo 4 quadros cada): FPS por intervalo legível,
    // sem serrilhado de quadro a quadro; o pior quadro de cada coluna é mantido.
    let span = (valid[valid.len() - 1].0 - valid[0].0).max(0.0);
    let buckets = ((span * 4.0).ceil() as usize)
        .clamp(1, MAX_POINTS)
        .min((valid.len() / 4).max(1));
    let width = ((t1 - t0) / buckets as f64).max(1e-6);
    let idx = |t: f64| (((t - t0) / width) as usize).min(buckets - 1);

    let mut fps_b: Vec<(f64, usize, f64)> = vec![(0.0, 0, 0.0); buckets]; // (soma ft, quadros, pior)
    for (t, f) in valid {
        let b = &mut fps_b[idx(*t)];
        b.0 += f;
        b.1 += 1;
        b.2 = b.2.max(*f);
    }
    let center = |i: usize| t0 + (i as f64 + 0.5) * width;
    let fps_points: Vec<[f64; 2]> = fps_b
        .iter()
        .enumerate()
        .filter(|(_, b)| b.1 > 0)
        .map(|(i, b)| {
            let v = match mode {
                CaptureMode::PerFrame => b.1 as f64 / (b.0 / 1000.0),
                CaptureMode::Periodic => 1000.0 / (b.0 / b.1 as f64),
            };
            [round(center(i)), round(v)]
        })
        .collect();
    // Na amostragem periódica, preferimos a própria coluna fps quando existe.
    let fps_points = match (mode, fps_col) {
        (CaptureMode::Periodic, Some(col)) => bucket_mean(col, times, t0, width, buckets, |v| {
            v > 0.0 && Metric::Fps.plausible(v)
        })
        .unwrap_or(fps_points),
        _ => fps_points,
    };
    out.push(Series {
        key: "fps".into(),
        metric: Metric::Fps,
        unit: "FPS".into(),
        points: fps_points,
    });
    out.push(Series {
        key: "frametime".into(),
        metric: Metric::Frametime,
        unit: "ms".into(),
        points: fps_b
            .iter()
            .enumerate()
            .filter(|(_, b)| b.1 > 0)
            .map(|(i, b)| [round(center(i)), round(b.0 / b.1 as f64)])
            .collect(),
    });
    out.push(Series {
        key: "frametimeMax".into(),
        metric: Metric::Frametime,
        unit: "ms".into(),
        points: fps_b
            .iter()
            .enumerate()
            .filter(|(_, b)| b.1 > 0)
            .map(|(i, b)| [round(center(i)), round(b.2)])
            .collect(),
    });
    for (m, col) in sensors {
        if let Some(points) = bucket_mean(col, times, t0, width, buckets, |_| true) {
            out.push(Series {
                key: serde_json::to_value(m)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                metric: *m,
                unit: m.unit().into(),
                points,
            });
        }
    }
    out
}

fn bucket_mean(
    col: &[Option<f64>],
    times: &[Option<f64>],
    t0: f64,
    width: f64,
    buckets: usize,
    ok: impl Fn(f64) -> bool,
) -> Option<Vec<[f64; 2]>> {
    let mut acc = vec![(0.0, 0usize); buckets];
    for (v, t) in col.iter().zip(times) {
        let (Some(v), Some(t)) = (v, t) else { continue };
        if !ok(*v) || *t < t0 {
            continue;
        }
        let i = (((t - t0) / width) as usize).min(buckets - 1);
        acc[i].0 += v;
        acc[i].1 += 1;
    }
    let pts: Vec<[f64; 2]> = acc
        .iter()
        .enumerate()
        .filter(|(_, a)| a.1 > 0)
        .map(|(i, a)| {
            [
                round(t0 + (i as f64 + 0.5) * width),
                round(a.0 / a.1 as f64),
            ]
        })
        .collect();
    (!pts.is_empty()).then_some(pts)
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::super::parser::{fixtures::per_frame, parse};
    use super::*;

    fn run(log: &str) -> Analysis {
        analyze(&parse(log.as_bytes()).unwrap())
    }

    #[test]
    fn constant_60fps_gives_exact_numbers() {
        let fts = vec![1000.0 / 60.0; 3600]; // 60 s a 60 FPS
        let a = run(&per_frame(&fts, 97.0));
        let s = &a.stats;
        assert_eq!(s.mode, CaptureMode::PerFrame);
        assert!((s.fps.avg.unwrap() - 60.0).abs() < 0.01);
        assert!((s.fps.low1.unwrap() - 60.0).abs() < 0.01);
        assert!((s.fps.low01.unwrap() - 60.0).abs() < 0.01);
        assert!((s.duration_s - 60.0).abs() < 0.05, "{}", s.duration_s);
        assert_eq!(s.frames, Some(3600));
        assert_eq!(s.stability.spikes, 0);
        assert_eq!(
            a.quality.level,
            QualityLevel::Good,
            "{:?}",
            a.quality.issues
        );
        assert!((s.resources[&Metric::GpuLoad].avg - 97.0).abs() < 1e-9);
    }

    #[test]
    fn duration_thresholds_allow_capture_to_finish_between_frames() {
        let a = run(&per_frame(&vec![10.0; 999], 90.0));
        assert!(!a.quality.issues.iter().any(|i| i.code == "too-short"));
        assert!(a.quality.issues.iter().any(|i| i.code == "short"));
        let b = run(&per_frame(&vec![10.0; 980], 90.0));
        assert!(b.quality.issues.iter().any(|i| i.code == "too-short"));
    }

    #[test]
    fn average_is_time_weighted_and_lows_use_worst_frames() {
        // 990 quadros de 10 ms + 10 quadros de 50 ms (1% do total).
        let mut fts = vec![10.0; 990];
        fts.extend(vec![50.0; 10]);
        let a = run(&per_frame(&fts, 90.0));
        let s = &a.stats.fps;
        // 1000 quadros em 9,9 s + 0,5 s = 10,4 s → 96,15 FPS (média dos FPS daria 99,2)
        assert!((s.avg.unwrap() - 1000.0 / 10.4).abs() < 0.01, "{:?}", s.avg);
        assert!((s.low1.unwrap() - 20.0).abs() < 1e-9, "1% low = 1000/50");
        assert!((s.low01.unwrap() - 20.0).abs() < 1e-9);
        assert!(
            (a.stats.frametime.p99.unwrap() - 10.0).abs() < 1e-9,
            "p99 nearest-rank = 990º valor"
        );
        assert!((a.stats.frametime.p999.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(s.median.map(|m| m.round()), Some(100.0));
        assert!(!s.lows_estimated);
    }

    #[test]
    fn lows_require_enough_frames() {
        let a = run(&per_frame(&vec![10.0; 99], 50.0));
        assert_eq!(a.stats.fps.low1, None);
        assert_eq!(a.stats.fps.low01, None);
        assert_eq!(a.quality.level, QualityLevel::Unsuitable);
        assert!(a.quality.issues.iter().any(|i| i.code == "few-frames"));
    }

    #[test]
    fn detects_spikes_and_unstable_interval() {
        let mut fts = vec![10.0; 3000];
        for i in 0..5 {
            fts[1500 + i * 20] = 60.0; // 5 picos concentrados por volta de 15 s
        }
        let a = run(&per_frame(&fts, 90.0));
        let st = &a.stats.stability;
        assert_eq!(st.spikes, 5);
        assert!(!st.unstable_intervals.is_empty());
        let iv = &st.unstable_intervals[0];
        assert!(iv.start_s >= 14.0 && iv.end_s <= 17.5, "{iv:?}");
        assert!((iv.worst_ms - 60.0).abs() < 1e-9);
    }

    #[test]
    fn single_isolated_spike_is_not_an_unstable_interval() {
        let mut fts = vec![10.0; 3000];
        fts[1000] = 80.0;
        let a = run(&per_frame(&fts, 90.0));
        assert_eq!(a.stats.stability.spikes, 1);
        assert!(a.stats.stability.unstable_intervals.is_empty());
    }

    #[test]
    fn tiny_frametime_jitter_is_not_instability() {
        // ~6800 FPS com variação relativa alta, mas de décimos de milissegundo.
        let fts: Vec<f64> = (0..60_000)
            .map(|i| if i % 3 == 0 { 0.3 } else { 0.1 })
            .collect();
        let a = run(&per_frame(&fts, 90.0));
        assert!(a.stats.stability.unstable_intervals.is_empty());
    }

    #[test]
    fn impossible_frames_are_excluded_and_reported() {
        let mut fts = vec![10.0; 3000];
        fts[10] = 1_459_160.0; // visto em log real: janela fora de foco
        let a = run(&per_frame(&fts, 90.0));
        assert_eq!(a.stats.valid_rows, 2999);
        assert!(a.quality.issues.iter().any(|i| i.code == "invalid-frames"));
        assert!((a.stats.fps.avg.unwrap() - 100.0).abs() < 0.01);
    }

    #[test]
    fn warmup_zeros_and_always_zero_columns_become_null() {
        let mut log = String::from(super::super::parser::fixtures::V1_HEADER);
        let mut t = 0u64;
        for i in 0..400 {
            t += 10_000_000;
            let gpu = if i < 100 { 0 } else { 80 };
            log.push_str(&format!("100,10,20,0,{gpu},0,0,0,0,0,0,8,0,0,4000,{t}\n"));
        }
        let a = run(&log);
        let r = &a.stats.resources;
        assert!(
            (r[&Metric::GpuLoad].avg - 80.0).abs() < 1e-9,
            "zeros iniciais = aquecimento"
        );
        assert_eq!(r[&Metric::GpuLoad].samples, 300);
        assert!(
            !r.contains_key(&Metric::CpuPower),
            "sempre zero = indisponível"
        );
        assert!(!a.stats.available.contains(&Metric::GpuTemp));
        assert!(a
            .quality
            .issues
            .iter()
            .any(|i| i.code == "unavailable-metrics"));
    }

    #[test]
    fn periodic_logs_are_flagged_and_lows_estimated() {
        // 1 amostra a cada 100 ms com frametime ~10 ms (cobertura de ~10%).
        let mut log = String::from("fps,frametime,elapsed\n");
        for i in 0..600u64 {
            let fps = if i % 50 == 0 { 50.0 } else { 100.0 };
            log.push_str(&format!(
                "{fps},{},{}\n",
                1000.0 / fps,
                (i + 1) * 100_000_000
            ));
        }
        let a = run(&log);
        assert_eq!(a.stats.mode, CaptureMode::Periodic);
        assert!(a.stats.fps.lows_estimated);
        assert_eq!(a.stats.fps.low01, None, "600 amostras não bastam para 0,1%");
        assert!((a.stats.fps.low1.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(a.stats.frames, None);
        assert!(a.quality.issues.iter().any(|i| i.code == "periodic"));
    }

    #[test]
    fn gaps_are_reported() {
        let mut log = String::from("fps,frametime,elapsed\n");
        let mut t = 0u64;
        for i in 0..2000 {
            t += 10_000_000;
            if i == 1000 {
                t += 5_000_000_000; // 5 s sem quadros
            }
            log.push_str(&format!("100,10,{t}\n"));
        }
        let a = run(&log);
        assert!(
            a.quality.issues.iter().any(|i| i.code == "gaps"),
            "{:?}",
            a.quality.issues
        );
    }

    #[test]
    fn downsampling_keeps_peaks_and_limits_points() {
        let mut fts = vec![5.0; 200_000];
        fts[123_456] = 400.0;
        let a = run(&per_frame(&fts, 90.0));
        let worst = a.series.iter().find(|s| s.key == "frametimeMax").unwrap();
        assert!(worst.points.len() <= MAX_POINTS);
        assert!(
            worst.points.iter().any(|p| (p[1] - 400.0).abs() < 1e-9),
            "o pico sobrevive à redução"
        );
        assert_eq!(
            a.stats.valid_rows, 200_000,
            "estatísticas usam todos os quadros"
        );
    }

    #[test]
    fn percentile_nearest_rank() {
        let v: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        assert_eq!(percentile(&v, 99.0), Some(99.0));
        assert_eq!(percentile(&v, 50.0), Some(50.0));
        assert_eq!(percentile(&v, 0.1), Some(1.0));
        assert_eq!(percentile(&[], 50.0), None);
    }
}
