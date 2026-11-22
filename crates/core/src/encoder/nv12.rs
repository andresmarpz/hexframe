use serde::Serialize;

const EXPECTED_LUMA_MEAN: f64 = 125.5;
const LUMA_TOLERANCE: f64 = 12.0;
const CHROMA_NEUTRAL: f64 = 128.0;
const CHROMA_TOLERANCE: f64 = 6.0;
const MIN_RAMP_SPAN: f64 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Nv12Means {
    pub luma: f64,
    pub luma_left: f64,
    pub luma_right: f64,
    pub chroma_u: f64,
    pub chroma_v: f64,
}

#[derive(Debug, Serialize)]
pub struct Nv12Report {
    pub status: &'static str,
    pub detail: String,
    pub means: Option<Nv12Means>,
}

impl Nv12Report {
    pub fn summary(&self) -> String {
        match self.status {
            "ok" => "nv12 ok".to_owned(),
            "suspect" => format!("nv12 suspect: {}", self.detail),
            _ => "nv12 unavailable".to_owned(),
        }
    }
}

pub fn nv12_report(means: Option<Nv12Means>) -> Nv12Report {
    let Some(m) = means else {
        return Nv12Report {
            status: "unavailable",
            detail: "the frame could not be read back".to_owned(),
            means: None,
        };
    };
    let mut problems = Vec::new();
    if (m.luma - EXPECTED_LUMA_MEAN).abs() > LUMA_TOLERANCE {
        problems.push(format!(
            "luma mean {:.1}, expected about {EXPECTED_LUMA_MEAN}",
            m.luma
        ));
    }
    if m.luma_right - m.luma_left < MIN_RAMP_SPAN {
        problems.push(format!(
            "luma ramp spans {:.1}, expected over {MIN_RAMP_SPAN}",
            m.luma_right - m.luma_left
        ));
    }
    for (name, value) in [("U", m.chroma_u), ("V", m.chroma_v)] {
        if (value - CHROMA_NEUTRAL).abs() > CHROMA_TOLERANCE {
            problems.push(format!("{name} mean {value:.1}, expected {CHROMA_NEUTRAL}"));
        }
    }
    Nv12Report {
        status: if problems.is_empty() { "ok" } else { "suspect" },
        detail: problems.join("; "),
        means,
    }
}
