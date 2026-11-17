//! What the preview shows over the uses: which uses play when, the numbers rising over the dummy
//! and the player, the dummy's frostbite, the player's buff and pulls.
use crate::skill_sim::Timeline;
use hsplanner_engine::calc::skill_cost;
use hsplanner_engine::calc::skills::{Ranged, StatMap};
use hsplanner_engine::calc::types::SkillSpec;

/// Frames a number rises before it is gone.
pub(crate) const RISE: u32 = 36;
/// Game px a number rises over its life.
const RISE_HEIGHT: f32 = 24.;
/// Game px above the dummy and the player at which numbers start.
const ABOVE_DUMMY: f32 = 34.;
const ABOVE_PLAYER: f32 = 48.;
/// Uses drawn at once at most, the newest ones, however fast the skill is used.
pub(crate) const MAX_OVERLAP: usize = 8;
/// Side steps that keep numbers of the same frame apart.
const SPREAD: [f32; 3] = [0., -14., 14.];

/// The uses on screen `t` frames into the loop, oldest first, each with its frame. Use `k` starts
/// at `k * period` and plays `lengths[k % lengths.len()]` frames, so uses overlap when the period
/// is shorter than a use.
pub(crate) fn visible(lengths: &[usize], period: usize, t: usize) -> Vec<(usize, usize)> {
    let period = period.max(1);
    let longest = lengths.iter().copied().max().unwrap_or(0);
    let mut shown: Vec<(usize, usize)> = (0..=t / period)
        .rev()
        .map(|k| (k, t - k * period))
        .take_while(|&(_, frame)| frame < longest)
        .filter(|&(k, frame)| frame < lengths[k % lengths.len()])
        .take(MAX_OVERLAP)
        .collect();
    shown.reverse();
    shown
}

/// Frames until the next use starts.
pub(crate) fn until_next(period: usize, t: usize) -> usize {
    let period = period.max(1);
    period - t % period
}

/// Uses per second a skill plays at: the game's use rate, or the pulse rate of a damaging aura,
/// whose timer makes each pulse (skillsAura).
pub(crate) fn use_rate(skill: &SkillSpec, stats: &StatMap, scoped: &StatMap) -> Option<Ranged> {
    skill_cost::game_use_rate(skill, stats, scoped)
        .or_else(|| skill_cost::pulse_rate(skill, scoped))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    Hit,
    Frost,
    Mana,
}

/// A number rising from the dummy or the player.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Rising {
    /// Game px of the number's centre.
    pub at: [f32; 2],
    pub text: String,
    /// Share of its rise already gone, 0 to 1.
    pub gone: f32,
    pub kind: Kind,
}

/// Damage numbers over the dummy, from the skill's `hit`, and mana numbers over the player,
/// from the player's maximum mana.
pub(crate) fn rising(moment: &[(&Timeline, usize)], hit: f64, mana: f64) -> Vec<Rising> {
    let mut numbers = Vec::new();
    for &(timeline, frame) in moment {
        let now = frame as u32;
        let recent = |at: u32| at <= now && now < at + RISE;
        let gone = |at: u32| (now - at) as f32 / RISE as f32;
        let [x, y] = timeline.dummy;
        let damage = (timeline.effects.damage.iter()).filter(|d| d.share > 0. && recent(d.frame));
        for (i, d) in damage.enumerate() {
            let gone = gone(d.frame);
            numbers.push(Rising {
                at: [x + SPREAD[i % 3], y - ABOVE_DUMMY - gone * RISE_HEIGHT],
                text: format!("{:.0}", hit * f64::from(d.share)),
                gone,
                kind: if d.frost { Kind::Frost } else { Kind::Hit },
            });
        }
        for &(at, share) in timeline.effects.mana.iter().filter(|(at, _)| recent(*at)) {
            let gone = gone(at);
            numbers.push(Rising {
                at: [0., -ABOVE_PLAYER - gone * RISE_HEIGHT],
                text: format!("-{:.0}", mana * f64::from(share)),
                gone,
                kind: Kind::Mana,
            });
        }
    }
    numbers
}

pub(crate) fn frostbitten(moment: &[(&Timeline, usize)]) -> bool {
    moment
        .iter()
        .any(|&(timeline, frame)| timeline.effects.frostbitten_at(frame as u32))
}

/// The player's buff percent while a use has it on.
pub(crate) fn buff(moment: &[(&Timeline, usize)]) -> Option<f32> {
    moment.iter().find_map(|&(timeline, frame)| {
        let ([from, to], percent) = timeline.effects.buffed?;
        (from..to).contains(&(frame as u32)).then_some(percent)
    })
}

/// Points the dummy is being pulled toward.
pub(crate) fn pulls(moment: &[(&Timeline, usize)]) -> Vec<[f32; 2]> {
    moment
        .iter()
        .flat_map(|&(timeline, frame)| {
            timeline
                .effects
                .pulls
                .iter()
                .filter(move |([from, to], _)| (*from..*to).contains(&(frame as u32)))
                .map(|(_, toward)| *toward)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Kind, RISE, rising, until_next, visible};
    use crate::skill_effects::{Damage, Effects};
    use crate::skill_sim::Timeline;

    #[test]
    fn a_long_pause_shows_one_use_and_a_short_period_overlaps_them() {
        // The old loop: a 100-frame use and 30 frames of floor.
        assert_eq!(visible(&[100], 130, 50), [(0, 50)]);
        assert_eq!(visible(&[100], 130, 120), []);
        assert_eq!(visible(&[100], 130, 140), [(1, 10)]);
        assert_eq!(until_next(130, 120), 10);
        // Casting every 60 frames: two uses at once, the older one first.
        assert_eq!(visible(&[100, 100], 60, 70), [(0, 70), (1, 10)]);
    }

    #[test]
    fn at_most_a_few_uses_overlap_at_a_fast_period() {
        let shown = visible(&[300], 1, 1000);
        assert_eq!(shown.len(), super::MAX_OVERLAP);
        assert_eq!(shown.last(), Some(&(1000, 0)));
    }

    #[test]
    fn a_damaging_aura_plays_at_its_pulse_rate_and_other_skills_at_their_use_rate() {
        use hsplanner_engine::calc::{data, skill_cost};
        let skills = data::get_skills_by_class("pyromancer");
        let skill = |id: &str| skills.iter().find(|s| s.id == id).expect(id);
        let none = std::collections::HashMap::new();
        let aura = super::use_rate(skill("scorching_aura"), &none, &none);
        assert_eq!(aura, Some((60. / 61., 60. / 61.)));
        let rapid = [("aura_pulse_speed".to_owned(), (75., 75.))].into();
        let fast = super::use_rate(skill("scorching_aura"), &none, &rapid);
        assert_eq!(fast, Some((60. / 16., 60. / 16.)));
        let fireball = skill("fireball");
        assert_eq!(
            super::use_rate(fireball, &none, &none),
            skill_cost::game_use_rate(fireball, &none, &none)
        );
    }

    #[test]
    fn the_buff_and_the_pulls_show_only_inside_their_frames() {
        use super::{buff, pulls};
        let timeline = Timeline {
            dummy: [160., 0.],
            frames: Vec::new(),
            hits: Vec::new(),
            effects: Effects {
                buffed: Some(([0, 270], 75.)),
                pulls: vec![([10, 70], [160., -30.])],
                ..Effects::default()
            },
        };
        assert_eq!(buff(&[(&timeline, 269)]), Some(75.));
        assert_eq!(buff(&[(&timeline, 270)]), None);
        assert_eq!(pulls(&[(&timeline, 10)]), [[160., -30.]]);
        assert!(pulls(&[(&timeline, 70)]).is_empty());
    }

    #[test]
    fn hits_rise_over_the_dummy_and_drains_over_the_player() {
        let timeline = Timeline {
            dummy: [160., 0.],
            frames: Vec::new(),
            hits: vec![10],
            effects: Effects {
                damage: vec![
                    Damage {
                        frame: 10,
                        share: 1.5,
                        frost: false,
                    },
                    Damage {
                        frame: 10,
                        share: 0.25,
                        frost: true,
                    },
                ],
                mana: vec![(10, 0.03)],
                ..Effects::default()
            },
        };
        let numbers = rising(&[(&timeline, 10 + RISE as usize / 2)], 200., 1000.);
        let texts: Vec<(&str, Kind)> = numbers.iter().map(|n| (n.text.as_str(), n.kind)).collect();
        assert_eq!(
            texts,
            [("300", Kind::Hit), ("50", Kind::Frost), ("-30", Kind::Mana)]
        );
        assert!(numbers.iter().all(|n| (n.gone - 0.5).abs() < 1e-6));
        assert!(rising(&[(&timeline, 10 + RISE as usize)], 200., 1000.).is_empty());
    }

    #[test]
    fn a_hit_that_deals_nothing_raises_no_number() {
        let timeline = Timeline {
            dummy: [160., 0.],
            frames: Vec::new(),
            hits: vec![10],
            effects: Effects {
                damage: vec![Damage {
                    frame: 10,
                    share: 0.,
                    frost: false,
                }],
                ..Effects::default()
            },
        };
        assert!(rising(&[(&timeline, 12)], 200., 1000.).is_empty());
    }
}
