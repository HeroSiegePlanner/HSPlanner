use once_cell::sync::Lazy;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

use super::data::{patched_value, PatchKind};
use super::season;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StarScaleConfig {
    Percent { per_star: f64 },
    FlatSkillStaircase,
    ItemSpecificStaircase,
    None,
    Unknown,
    Glitch,
}

// Single source of truth shared with the TS calc layer (src/data/star-scaling.json).
const STAR_SCALING_JSON: &str = include_str!("../../../src/data/star-scaling.json");

#[derive(Deserialize, Clone, Copy)]
#[serde(tag = "kind")]
enum StarScaleConfigDto {
    #[serde(rename = "percent")]
    Percent {
        #[serde(rename = "perStar")]
        per_star: f64,
    },
    #[serde(rename = "flat-skill-staircase")]
    FlatSkillStaircase,
    #[serde(rename = "item-specific-staircase")]
    ItemSpecificStaircase,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "glitch")]
    Glitch,
}

impl From<StarScaleConfigDto> for StarScaleConfig {
    fn from(v: StarScaleConfigDto) -> Self {
        match v {
            StarScaleConfigDto::Percent { per_star } => StarScaleConfig::Percent { per_star },
            StarScaleConfigDto::FlatSkillStaircase => StarScaleConfig::FlatSkillStaircase,
            StarScaleConfigDto::ItemSpecificStaircase => StarScaleConfig::ItemSpecificStaircase,
            StarScaleConfigDto::None => StarScaleConfig::None,
            StarScaleConfigDto::Unknown => StarScaleConfig::Unknown,
            StarScaleConfigDto::Glitch => StarScaleConfig::Glitch,
        }
    }
}

#[derive(Deserialize)]
struct StarScalingData {
    #[serde(rename = "flatSkillStaircase")]
    flat_skill_staircase: Vec<f64>,
    #[serde(rename = "itemSpecificStaircase")]
    item_specific_staircase: Vec<f64>,
    map: HashMap<String, StarScaleConfigDto>,
}

struct StarScaling {
    flat_skill_staircase: Vec<f64>,
    item_specific_staircase: Vec<f64>,
    map: HashMap<String, StarScaleConfig>,
}

static STAR_SCALING_BY_SEASON: Lazy<Mutex<HashMap<String, &'static StarScaling>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// On patch error, log and fall back to base; same season normalization as data::data_for.
fn load_for(season_id: &str) -> StarScaling {
    let patches = season::patches_for(season_id);
    let base: serde_json::Value =
        serde_json::from_str(STAR_SCALING_JSON).expect("src/data/star-scaling.json must be valid");
    let value = patched_value(base, &patches, "star-scaling", PatchKind::RecordMerge);
    let dto: StarScalingData =
        serde_json::from_value(value).expect("invalid star-scaling shape after patch");
    StarScaling {
        flat_skill_staircase: dto.flat_skill_staircase,
        item_specific_staircase: dto.item_specific_staircase,
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
    matches!(
        get_star_scale_config(stat_key),
        StarScaleConfig::None | StarScaleConfig::Unknown | StarScaleConfig::Glitch
    )
}

pub fn stat_star_percent_multiplier(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 1.0;
    }
    match get_star_scale_config(stat_key) {
        StarScaleConfig::Percent { per_star } => 1.0 + (s as f64 * per_star) / 100.0,
        _ => 1.0,
    }
}

pub fn stat_star_flat_bonus(stat_key: Option<&str>, stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 0.0;
    }
    let c = configs();
    match get_star_scale_config(stat_key) {
        StarScaleConfig::FlatSkillStaircase => {
            c.flat_skill_staircase.get(s as usize).copied().unwrap_or(0.0)
        }
        StarScaleConfig::ItemSpecificStaircase => {
            c.item_specific_staircase.get(s as usize).copied().unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

pub fn item_granted_skill_rank_flat_bonus(stars: Option<u32>) -> f64 {
    let s = stars.unwrap_or(0);
    if s == 0 {
        return 0.0;
    }
    configs()
        .item_specific_staircase
        .get(s as usize)
        .copied()
        .unwrap_or(0.0)
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
        assert_eq!(
            get_star_scale_config(Some("physical_skills")),
            StarScaleConfig::Percent { per_star: 8.0 }
        );
    }

    #[test]
    fn staircase_and_special_kinds() {
        assert_eq!(
            get_star_scale_config(Some("fire_skills")),
            StarScaleConfig::FlatSkillStaircase
        );
        assert_eq!(
            get_star_scale_config(Some("cold_skills")),
            StarScaleConfig::FlatSkillStaircase
        );
        assert_eq!(
            get_star_scale_config(Some("max_fire_resistance")),
            StarScaleConfig::Unknown
        );
        assert_eq!(
            get_star_scale_config(Some("magic_damage_reduction")),
            StarScaleConfig::Glitch
        );
        assert_eq!(
            get_star_scale_config(Some("attacks_per_second")),
            StarScaleConfig::None
        );
    }

    #[test]
    fn stars_zero_or_missing_returns_identity() {
        assert_eq!(stat_star_percent_multiplier(Some("to_strength"), None), 1.0);
        assert_eq!(stat_star_percent_multiplier(Some("to_strength"), Some(0)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(0)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(0)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(None), 0.0);
    }

    #[test]
    fn percent_multiplier_math() {
        // to_strength = percent 5, 3 stars => 1 + 3*5/100 = 1.15
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(3)) - 1.15).abs() < 1e-12);
        // to_strength, 5 stars => 1.25
        assert!((stat_star_percent_multiplier(Some("to_strength"), Some(5)) - 1.25).abs() < 1e-12);
        // increased_all_attributes = percent 10, 5 stars => 1.5
        assert!(
            (stat_star_percent_multiplier(Some("increased_all_attributes"), Some(5)) - 1.5).abs()
                < 1e-12
        );
        // physical_skills = percent 8, 5 stars => 1.4
        assert!(
            (stat_star_percent_multiplier(Some("physical_skills"), Some(5)) - 1.4).abs() < 1e-12
        );
    }

    #[test]
    fn flat_skill_staircase_values() {
        // 1*=0, 2*=0, 3*=+1, 4*=+1, 5*=+2
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(1)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(2)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(3)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(4)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(5)), 2.0);
    }

    #[test]
    fn item_specific_staircase_values() {
        // 1*=0, 2*=+1, 3*=+1, 4*=+2, 5*=+3
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(1)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(2)), 1.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(3)), 1.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(4)), 2.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(5)), 3.0);
    }

    #[test]
    fn out_of_range_stars_returns_zero() {
        assert_eq!(stat_star_flat_bonus(Some("fire_skills"), Some(99)), 0.0);
        assert_eq!(item_granted_skill_rank_flat_bonus(Some(42)), 0.0);
    }

    #[test]
    fn percent_kind_yields_no_flat_bonus() {
        // percent stats use multiplier, not flat staircase
        assert_eq!(stat_star_flat_bonus(Some("to_strength"), Some(5)), 0.0);
        assert_eq!(stat_star_flat_bonus(Some("fire_skill_damage"), Some(5)), 0.0);
    }

    #[test]
    fn none_unknown_glitch_no_scaling() {
        // None
        assert_eq!(stat_star_percent_multiplier(Some("attacks_per_second"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("attacks_per_second"), Some(5)), 0.0);
        // Unknown
        assert_eq!(stat_star_percent_multiplier(Some("max_all_resistances"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("max_all_resistances"), Some(5)), 0.0);
        // Glitch
        assert_eq!(stat_star_percent_multiplier(Some("magic_damage_reduction"), Some(5)), 1.0);
        assert_eq!(stat_star_flat_bonus(Some("magic_damage_reduction"), Some(5)), 0.0);
    }

    #[test]
    fn star_immune_classification() {
        assert!(is_stat_star_immune(Some("attacks_per_second"))); // None
        assert!(is_stat_star_immune(Some("max_all_resistances"))); // Unknown
        assert!(is_stat_star_immune(Some("magic_damage_reduction"))); // Glitch
        assert!(is_stat_star_immune(None)); // missing
        assert!(!is_stat_star_immune(Some("to_strength"))); // Percent
        assert!(!is_stat_star_immune(Some("fire_skills"))); // FlatSkillStaircase
    }
}
