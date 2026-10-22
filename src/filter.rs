use crate::model::Process;
use regex::RegexBuilder;
#[derive(Default)]
pub struct Query {
    groups: Vec<Vec<Term>>,
    pub error: Option<String>,
}
struct Term {
    field: String,
    value: String,
    negated: bool,
    regex: Option<regex::Regex>,
    compare: Option<(char, f64)>,
}
impl Query {
    pub fn parse(text: &str) -> Self {
        let mut query = Self::default();
        for group in text.split(" | ") {
            let mut terms = vec![];
            // Quoted terms preserve spaces; all terms in one group are ANDed.
            let mut words = vec![];
            let mut word = String::new();
            let mut quoted = false;
            for c in group.chars() {
                if c == '"' {
                    quoted = !quoted;
                } else if c.is_whitespace() && !quoted {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                } else {
                    word.push(c);
                }
            }
            if !word.is_empty() {
                words.push(word);
            }
            if quoted {
                query.error = Some("Unclosed quoted term".into());
            }
            for word in words {
                let (negated, word) = word
                    .strip_prefix('!')
                    .map(|s| (true, s))
                    .unwrap_or((false, word.as_str()));
                let (field, value) = word.split_once(':').unwrap_or(("", word));
                if ![
                    "", "pid", "user", "name", "state", "cmd", "command", "cpu", "mem", "memory",
                    "gpu", "vram", "tag",
                ]
                .contains(&field.to_lowercase().as_str())
                {
                    query.error = Some(format!("Unknown field: {field}"));
                }
                let mut regex = None;
                let mut compare = None;
                if value.starts_with('/') && value.ends_with('/') && value.len() >= 2 {
                    match RegexBuilder::new(&value[1..value.len() - 1])
                        .case_insensitive(true)
                        .size_limit(256 * 1024)
                        .build()
                    {
                        Ok(re) => regex = Some(re),
                        Err(error) => query.error = Some(error.to_string()),
                    }
                } else if let Some(first) =
                    value.chars().next().filter(|c| ['>', '<', '='].contains(c))
                    && let Ok(number) = value[1..].parse::<f64>()
                {
                    compare = Some((first, number));
                }
                terms.push(Term {
                    field: field.to_lowercase(),
                    value: value.to_lowercase(),
                    negated,
                    regex,
                    compare,
                });
            }
            query.groups.push(terms);
        }
        query
    }
    pub fn matches(&self, p: &Process) -> bool {
        if self.error.is_some() {
            return false;
        }
        self.groups.iter().any(|group| {
            group.iter().all(|term| {
                let value = match term.field.as_str() {
                    "pid" => p.pid.to_string(),
                    "user" => p.user.clone(),
                    "name" => p.name.clone(),
                    "state" => p.state.clone(),
                    "cmd" | "command" => p.command.clone(),
                    "cpu" => p.cpu.to_string(),
                    "mem" | "memory" => (p.memory as f64 / 1024.0 / 1024.0).to_string(),
                    "gpu" => p
                        .gpu_percent
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "—".into()),
                    "vram" => p
                        .gpu_memory
                        .map(|v| (v as f64 / 1024.0 / 1024.0).to_string())
                        .unwrap_or_else(|| "—".into()),
                    "tag" => p
                        .tags
                        .iter()
                        .map(|t| format!("{} {}", t.key, t.display()))
                        .collect::<Vec<_>>()
                        .join(" "),
                    "" => format!(
                        "{} {} {} {} {}",
                        p.pid,
                        p.user,
                        p.name,
                        p.command,
                        crate::tags::text(&p.tags)
                    ),
                    _ => String::new(),
                };
                let matched = if let Some((op, number)) = term.compare {
                    value.parse::<f64>().ok().is_some_and(|v| match op {
                        '>' => v > number,
                        '<' => v < number,
                        _ => v == number,
                    })
                } else if let Some(re) = &term.regex {
                    re.is_match(&value)
                } else {
                    value.to_lowercase().contains(&term.value)
                };
                matched != term.negated
            })
        })
    }
}

pub fn event_matches(event: &crate::events::Event, query: &str) -> bool {
    format!(
        "{} {} {}",
        event.timestamp,
        if event.active { "active" } else { "recovered" },
        event.condition
    )
    .to_lowercase()
    .contains(&query.to_lowercase())
}
