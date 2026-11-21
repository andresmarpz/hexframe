use std::path::Path;

use crate::outcome::{CaseOutcome, Shot};

#[cfg(test)]
mod tests;

pub(crate) struct Totals {
    pub(crate) cases: usize,
    pub(crate) failed: usize,
    pub(crate) new: usize,
}

pub(crate) fn totals(outcomes: &[CaseOutcome]) -> Totals {
    Totals {
        cases: outcomes.len(),
        failed: outcomes.iter().filter(|c| c.failed()).count(),
        new: outcomes
            .iter()
            .flat_map(|c| &c.checks)
            .filter(|c| c.shot.as_ref().is_some_and(|s| s.snap.is_new()))
            .count(),
    }
}

fn rel(out: &Path, path: &Path) -> String {
    path.strip_prefix(out)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn markdown(out: &Path, outcomes: &[CaseOutcome]) -> String {
    let t = totals(outcomes);
    let mut md = format!(
        "# Greyscreen QA\n\n{} cases, {} failed, {} new snapshots.\n\n",
        t.cases, t.failed, t.new
    );
    for case in outcomes {
        let mark = if case.failed() { "FAIL" } else { "ok" };
        md.push_str(&format!("## {mark} {}\n\n", case.name));
        if !case.about.is_empty() {
            md.push_str(&format!("{}\n\n", case.about));
        }
        if let Some(err) = &case.error {
            md.push_str(&format!("- error: {err}\n"));
        }
        for check in &case.checks {
            let shot = check
                .shot
                .as_ref()
                .map(|s| format!(", snapshot {}", s.snap.word()))
                .unwrap_or_default();
            let state = if check.failures.is_empty() {
                "passes"
            } else {
                "fails"
            };
            md.push_str(&format!("- `{}` {state}{shot}\n", check.name));
            for failure in &check.failures {
                md.push_str(&format!("  - {failure}\n"));
            }
            if let Some(diff) = check.shot.as_ref().and_then(|s| s.diff.as_ref()) {
                md.push_str(&format!("  - diff: {}\n", rel(out, diff)));
            }
        }
        md.push('\n');
    }
    md
}

const STYLE: &str = r#"
:root{--bg:#fafafa;--fg:#18181b;--muted:#71717a;--card:#fff;--line:#e4e4e7;--bad:#dc2626;--good:#16a34a;--new:#2563eb}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){--bg:#111113;--fg:#ececee;--muted:#a1a1aa;--card:#1b1b1f;--line:#2e2e33;--bad:#f87171;--good:#4ade80;--new:#60a5fa}}
body{margin:0;padding:24px 16px;background:var(--bg);color:var(--fg);font:14px/1.5 system-ui,sans-serif}
main{max-width:1400px;margin:0 auto}
h1{font-size:20px;margin:0 0 4px}
section{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:16px;margin:16px 0}
h2{font-size:16px;margin:0}
.muted{color:var(--muted)}.bad{color:var(--bad)}.good{color:var(--good)}.new{color:var(--new)}
.check{border-top:1px solid var(--line);margin-top:12px;padding-top:12px}
.shots{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:8px;margin-top:8px}
figure{margin:0}figcaption{font-size:12px;color:var(--muted)}
img{width:100%;height:auto;border:1px solid var(--line);border-radius:6px}
ul{margin:4px 0;padding-left:20px}
"#;

pub(crate) fn html(out: &Path, outcomes: &[CaseOutcome]) -> String {
    let t = totals(outcomes);
    let mut h = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Greyscreen QA</title><style>{STYLE}</style></head><body><main><h1>Greyscreen QA</h1><p class=\"muted\">{} cases, <span class=\"{}\">{} failed</span>, {} new snapshots. Headless: no window, no input, no sound.</p>",
        t.cases,
        if t.failed > 0 { "bad" } else { "good" },
        t.failed,
        t.new
    );
    for case in outcomes {
        let (class, word) = if case.failed() {
            ("bad", "fail")
        } else {
            ("good", "pass")
        };
        h.push_str(&format!(
            "<section><h2><span class=\"{class}\">{word}</span> {}</h2>",
            escape(&case.name)
        ));
        if !case.about.is_empty() {
            h.push_str(&format!("<p class=\"muted\">{}</p>", escape(&case.about)));
        }
        if let Some(err) = &case.error {
            h.push_str(&format!("<p class=\"bad\">{}</p>", escape(err)));
        }
        for check in &case.checks {
            h.push_str(&format!(
                "<div class=\"check\"><strong>{}</strong>",
                escape(&check.name)
            ));
            if let Some(shot) = &check.shot {
                let class = if shot.snap.failed() {
                    "bad"
                } else if shot.golden.is_none() {
                    "new"
                } else {
                    "muted"
                };
                h.push_str(&format!(
                    " <span class=\"{class}\">snapshot {}</span>",
                    escape(&shot.snap.word())
                ));
            }
            if !check.failures.is_empty() {
                h.push_str("<ul>");
                for failure in &check.failures {
                    h.push_str(&format!("<li class=\"bad\">{}</li>", escape(failure)));
                }
                h.push_str("</ul>");
            }
            if let Some(shot) = &check.shot {
                shots(&mut h, out, shot);
            }
            h.push_str("</div>");
        }
        h.push_str("</section>");
    }
    h.push_str("</main></body></html>");
    h
}

fn shots(h: &mut String, out: &Path, shot: &Shot) {
    h.push_str("<div class=\"shots\">");
    let figure = |h: &mut String, path: &Path, caption: &str| {
        let src = escape(&rel(out, path));
        h.push_str(&format!("<figure><a href=\"{src}\"><img src=\"{src}\" alt=\"{caption}\" loading=\"lazy\"></a><figcaption>{caption}</figcaption></figure>"
        ));
    };
    if let Some(golden) = &shot.golden {
        figure(h, golden, "golden");
    }
    figure(h, &shot.actual, "this run");
    if let Some(diff) = &shot.diff {
        figure(h, diff, "diff (red: changed)");
    }
    h.push_str("</div>");
}
