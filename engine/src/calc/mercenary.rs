//! Mercenary catalog, shared by native and transport-backed workflows.
use serde::Deserialize;
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
