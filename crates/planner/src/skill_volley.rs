//! How a use fans its volley out round the aim.
use crate::skill_sim::SkillFx;

impl SkillFx {
    /// Degrees the `i`-th of `count` projectiles turns from the aim: `fan` apart about the aim,
    /// then `skew` past it once there is more than one.
    pub(crate) fn fanned(&self, i: u32, count: u32) -> f32 {
        let centred = self.fan * (i as f32 - (count as f32 - 1.) / 2.);
        if count > 1 {
            centred + self.skew
        } else {
            centred
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SkillFx;

    fn angles(skew: f32, count: u32) -> Vec<f32> {
        let fx = SkillFx {
            dummy: 160.,
            fan: 12.,
            skew,
            random: false,
            spawn: Vec::new(),
        };
        (0..count).map(|i| fx.fanned(i, count)).collect()
    }

    #[test]
    fn a_skewed_volley_turns_past_its_centre_once_it_has_more_than_one() {
        assert_eq!(angles(6., 1), [0.]);
        assert_eq!(angles(6., 2), [0., 12.]);
        assert_eq!(angles(6., 3), [-6., 6., 18.]);
        assert_eq!(angles(0., 3), [-12., 0., 12.]);
        assert_eq!(angles(0., 2), [-6., 6.]);
    }
}
