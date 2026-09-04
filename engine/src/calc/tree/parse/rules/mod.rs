use super::*;

mod base;
mod s10;

pub(crate) static RULES: LazyLock<Vec<ParseRule>> = LazyLock::new(|| {
    let mut rules = base::rules();
    rules.extend(s10::rules());
    rules
});
