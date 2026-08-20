// ---------- rule-building macros ----------

macro_rules! mod_rule {
    ($pattern:expr, $key:literal $(,)?) => {
        ParseRule {
            test: Regex::new($pattern).unwrap(),
            build: |m| Some(ParsedMod {
                key: $key.to_string(),
                value: num(&m[1]),
                self_condition: None,
            }),
        }
    };
    ($pattern:expr, $base:literal, $more:literal $(,)?) => {
        ParseRule {
            test: Regex::new($pattern).unwrap(),
            build: |m| Some(ParsedMod {
                key: if m.get(2).is_some() {
                    $more.to_string()
                } else {
                    $base.to_string()
                },
                value: num(&m[1]),
                self_condition: None,
            }),
        }
    };
}

macro_rules! fixed_rule {
    ($pattern:expr, $key:literal, $value:expr $(,)?) => {
        ParseRule {
            test: Regex::new($pattern).unwrap(),
            build: |_| Some(ParsedMod {
                key: $key.to_string(),
                value: $value,
                self_condition: None,
            }),
        }
    };
}

macro_rules! null_rule {
    ($pattern:expr $(,)?) => {
        ParseRule {
            test: Regex::new($pattern).unwrap(),
            build: |_| None,
        }
    };
}

macro_rules! cond_rule {
    ($pattern:expr, $key:literal, $cond:expr $(,)?) => {
        ParseRule {
            test: Regex::new($pattern).unwrap(),
            build: |m| Some(ParsedMod {
                key: $key.to_string(),
                value: num(&m[1]),
                self_condition: Some($cond),
            }),
        }
    };
}

