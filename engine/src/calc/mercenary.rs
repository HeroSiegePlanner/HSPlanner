//! Mercenary catalog, shared by native and transport-backed workflows.
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MercenaryData {
    pub max_skill_rank: u32,
    pub slots: Vec<String>,
    pub classes: Vec<MercenaryClass>,
}
#[derive(Debug, Deserialize)]
pub struct MercenaryClass {
    pub id: String,
    pub name: String,
    pub role: String,
    pub location: String,
    pub skills: Vec<MercenarySkill>,
}
#[derive(Debug, Deserialize)]
pub struct MercenarySkill {
    pub id: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub damage_type: Option<String>,
    pub shared: bool,
    pub description: String,
}
static DATA: LazyLock<MercenaryData> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/mercenaries.json"))
        .expect("valid mercenary catalog")
});
pub fn data() -> &'static MercenaryData {
    &DATA
}

impl MercenaryClass {
    /// Ranks saved for skills the class no longer has, like Aura Expert, are not spent points.
    pub fn spent_points(&self, ranks: &HashMap<String, u32>) -> u32 {
        self.skills
            .iter()
            .map(|skill| ranks.get(&skill.id).copied().unwrap_or(0))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_skills_neither_appear_nor_count_as_spent_points() {
        let knight = data()
            .classes
            .iter()
            .find(|class| class.id == "merc_knight")
            .unwrap();
        let ranks = HashMap::from([
            (knight.skills[0].id.clone(), 3),
            ("aura_expert".to_string(), 5),
        ]);
        assert_eq!(knight.spent_points(&ranks), 3);
        assert!(data()
            .classes
            .iter()
            .flat_map(|class| &class.skills)
            .all(|skill| skill.id != "aura_expert"));
    }
}
