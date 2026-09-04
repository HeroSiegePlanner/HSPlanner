use std::sync::LazyLock;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

use super::data::{patched_value, PatchKind};
use super::season;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StarScaleConfig {
    Percent { per_star: f64 },
    Flat { per_star: f64 },
    None,
}

// Single source of truth shared with the TS calc layer (data/star-scaling.json).
const STAR_SCALING_JSON: &str = include_str!("../../../data/star-scaling.json");

#[derive(Deserialize, Clone, Copy)]
#[serde(tag = "kind")]
enum StarScaleConfigDto {
    #[serde(rename = "percent")]
    Percent {
        #[serde(rename = "perStar")]
        per_star: f64,
    },
    #[serde(rename = "flat")]
    Flat {
        #[serde(rename = "perStar")]
        per_star: f64,
    },
    #[serde(rename = "none")]
    None,
}

impl From<StarScaleConfigDto> for StarScaleConfig {
    fn from(v: StarScaleConfigDto) -> Self {
        match v {
            StarScaleConfigDto::Percent { per_star } => StarScaleConfig::Percent { per_star },
            StarScaleConfigDto::Flat { per_star } => StarScaleConfig::Flat { per_star },
            StarScaleConfigDto::None => StarScaleConfig::None,
        }
    }
}

#[derive(Deserialize)]
struct StarScalingData {
    #[serde(rename = "maxStars")]
    max_stars: u32,
    map: HashMap<String, StarScaleConfigDto>,
}

struct StarScaling {
    max_stars: u32,
    map: HashMap<String, StarScaleConfig>,
}

static STAR_SCALING_BY_SEASON: LazyLock<Mutex<HashMap<String, &'static StarScaling>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// On patch error, log and fall back to base; same season normalization as data::data_for.
fn load_for(season_id: &str) -> StarScaling {
    let patches = season::patches_for(season_id);
    let base: serde_json::Value =
        serde_json::from_str(STAR_SCALING_JSON).expect("data/star-scaling.json must be valid");
    let value = patched_value(base, &patches, "star-scaling", PatchKind::RecordMerge);
    let dto: StarScalingData =
        serde_json::from_value(value).expect("invalid star-scaling shape after patch");
    StarScaling {
        max_stars: dto.max_stars,
        map: dto.map.into_iter().map(|(k, v)| (k, v.into())).collect(),
    }
}

fn configs_for(season_id: &str) -> &'static StarScaling {
    season::cached_per_season(&STAR_SCALING_BY_SEASON, season_id, load_for)
}

thread_local! {
    static LAST_SCALING: RefCell<Option<(String, &'static StarScaling)>> =
        const { RefCell::new(None) };
}

fn configs() -> &'static StarScaling {
    season::memoized_current_season(&LAST_SCALING, configs_for)
}

pub fn max_stars() -> u32 {
    configs().max_stars
}

pub fn get_star_scale_config(stat_key: Option<&str>) -> StarScaleConfig {
    let Some(key) = stat_key else {
        return StarScaleConfig::None;
    };
    configs()
        .map
        .get(key)
        .copied()
        .unwrap_or(StarScaleConfig::None)
}

pub fn is_stat_star_immune(stat_key: Option<&str>) -> bool {
    matches!(get_star_scale_config(stat_key), StarScaleConfig::None)
}

// The game caps item field `p` at 5; a build carrying more must not out-scale it.
fn capped(stars: Option<u32>) -> u32 {
    stars.unwrap_or(0).min(max_stars())
}

pub fn stat_star_percent_multiplier(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = capped(stars);
    if s == 0 {
        return 1.0;
    }
    match get_star_scale_config(stat_key) {
        StarScaleConfig::Percent { per_star } => 1.0 + (s as f64 * per_star) / 100.0,
        _ => 1.0,
    }
}

pub fn stat_star_flat_bonus(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = capped(stars);
    if s == 0 {
        return 0.0;
    }
    match get_star_scale_config(stat_key) {
        StarScaleConfig::Flat { per_star } => s as f64 * per_star,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_treated_as_none() {
        assert_eq!(get_star_scale_config(None), StarScaleConfig::None);
        assert_eq!(
            get_star_scale_config(Some("unknown_stat_key")),
            StarScaleConfig::None
        );
    }

    #[test]
    fn percent_kind_lookup() {
        assert_eq!(
            get_star_scale_config(Some("to_strength")),
            StarScaleConfig::Percent { per_star: 5.0 }
        );
        assert_eq!(
            get_star_scale_config(Some("increased_all_attributes")),
            StarScaleConfig::Percent { per_star: 10.0 }
        );
    }

    #[test]
    fn flat_kind_lookup() {
        assert_eq!(
            get_star_scale_config(Some("physical_skills")),
            StarScaleConfig::Flat { per_star: 0.4 }
        );
        assert_eq!(
            get_star_scale_config(Some("fire_skills")),
            StarScaleConfig::Flat { per_star: 0.4 }
        );
        assert_eq!(
            get_star_scale_config(Some("item_granted_skill_rank")),
            StarScaleConfig::Flat { per_star: 0.6 }
        );
    }

    #[test]
    fn stats_the_game_never_star_scales() {
        for key in [
            "enhanced_defense",
            "crit_chance",
            "all_skills",
            "experience_gain",
            "max_all_resistances",
            "crushing_blow_chance",
        ] {
            assert_eq!(get_star_scale_config(Some(key)), StarScaleConfig::None);
            assert!(is_stat_star_immune(Some(key)));
        }
    }

    #[test]
    fn stars_zero_or_missing_returns_identity() {
        assert_eq!(stat_star_percent_multiplier(Some("to_strength"), None), 1.0);
        assert_eq!(
            stat_star_percent_multiplier(Some("to_strength"), Some(0)),
            1.0
        );
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(0)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), None), 0.0);
    }

    #[test]
    fn percent_multiplier_math() {
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(3)) - 1.15).abs() < 1e-12);
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(5)) - 1.25).abs() < 1e-12);
        assert!(
            (stat_star_percent_multiplier(Some("increased_all_attributes"), Some(5)) - 1.5).abs()
                < 1e-12
        );
    }

    #[test]
    fn flat_bonus_math() {
        assert!((stat_star_flat_bonus(Some("fire_skills"), Some(3)) - 1.2).abs() < 1e-12);
        assert!((stat_star_flat_bonus(Some("fire_skills"), Some(5)) - 2.0).abs() < 1e-12);
        let rank = stat_star_flat_bonus(Some("item_granted_skill_rank"), Some(4));
        assert!((rank - 2.4).abs() < 1e-12);
    }

    #[test]
    fn stars_beyond_the_game_cap_are_clamped() {
        assert_eq!(max_stars(), 5);
        assert_eq!(
            stat_star_percent_multiplier(Some("to_strength"), Some(99)),
            stat_star_percent_multiplier(Some("to_strength"), Some(5))
        );
        assert_eq!(
            stat_star_flat_bonus(Some("fire_skills"), Some(99)),
            stat_star_flat_bonus(Some("fire_skills"), Some(5))
        );
    }

    #[test]
    fn kinds_do_not_bleed_into_each_other() {
        assert_eq!(stat_star_flat_bonus(Some("to_strength"), Some(5)), 0.0);
        assert_eq!(
            stat_star_percent_multiplier(Some("fire_skills"), Some(5)),
            1.0
        );
    }

    #[test]
    fn none_kind_never_scales() {
        assert_eq!(
            stat_star_percent_multiplier(Some("enhanced_defense"), Some(5)),
            1.0
        );
        assert_eq!(stat_star_flat_bonus(Some("enhanced_defense"), Some(5)), 0.0);
    }

    #[test]
    fn star_immune_classification() {
        assert!(is_stat_star_immune(Some("attacks_per_second")));
        assert!(is_stat_star_immune(None));
        assert!(!is_stat_star_immune(Some("to_strength")));
        assert!(!is_stat_star_immune(Some("fire_skills")));
    }

    #[test]
    fn flat_magic_skill_damage_scales_like_the_other_flats() {
        assert_eq!(
            stat_star_percent_multiplier(Some("flat_magic_skill_damage"), Some(5)),
            1.25
        );
    }
}
