//! Native explanation rows captured beside the calculations they describe.
use super::{rg, Ranged, StatMap};

#[derive(Debug, Clone, PartialEq)]
pub struct CalculationStep {
    label: String,
    expression: String,
    value: Ranged,
    stat_key: Option<String>,
}

impl CalculationStep {
    pub(crate) fn new(
        label: impl Into<String>,
        expression: impl Into<String>,
        value: Ranged,
    ) -> Self {
        Self {
            label: label.into(),
            expression: expression.into(),
            value,
            stat_key: None,
        }
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn expression(&self) -> &str {
        &self.expression
    }
    pub fn value(&self) -> Ranged {
        self.value
    }
    pub fn stat_key(&self) -> Option<&str> {
        self.stat_key.as_deref()
    }
}

pub(crate) fn number(value: f64) -> String {
    let text = format!("{value:.6}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}
pub(crate) fn range(value: Ranged) -> String {
    if value.0 == value.1 {
        number(value.0)
    } else {
        format!("[{} … {}]", number(value.0), number(value.1))
    }
}
pub(crate) fn scalar(value: f64) -> Ranged {
    (value, value)
}

pub(crate) fn stat_inputs(
    trace: &mut Vec<CalculationStep>,
    stats: &StatMap,
    keys: impl IntoIterator<Item = impl AsRef<str>>,
) {
    for key in keys {
        let key = key.as_ref();
        let value = rg(stats, key);
        if value == (0.0, 0.0) || trace.iter().any(|step| step.stat_key() == Some(key)) {
            continue;
        }
        let mut step = CalculationStep::new(
            key.replace('_', " "),
            "Build stat · open for sources",
            value,
        );
        step.stat_key = Some(key.to_owned());
        trace.push(step);
    }
}

pub(crate) fn scoped_inputs(trace: &mut Vec<CalculationStep>, stats: &StatMap) {
    let mut keys: Vec<_> = stats.keys().collect();
    keys.sort();
    for key in keys {
        if stats[key] != (0.0, 0.0) {
            trace.push(CalculationStep::new(
                format!("Subtree · {}", key.replace('_', " ")),
                "Applies only to this skill",
                stats[key],
            ));
        }
    }
}

/// Human-readable minimum and maximum without compact-number rounding.
pub fn display_range(value: Ranged) -> String {
    range(value)
}
