use super::{Controls, Goal};

use crate::entity::ai::goal::melee_attack::MeleeAttackGoal;
use crate::entity::ai::goal::revenge::MobFilter;
use crate::entity::mob::Mob;

pub struct ZombieAttackGoal {
    melee_attack_goal: MeleeAttackGoal,
    ticks: i32,
    /// A reason to stay off the mob's melee, for the mobs that gate it.
    gate: Option<MobFilter>,
}

impl ZombieAttackGoal {
    #[must_use]
    pub fn new(speed: f64, pause_when_mob_idle: bool) -> Box<Self> {
        Box::new(Self {
            melee_attack_goal: MeleeAttackGoal::new(speed, pause_when_mob_idle),
            ticks: 0,
            gate: None,
        })
    }

    /// Vanilla subclasses the zombie's melee to add a condition, as
    /// `Drowned.DrownedAttackGoal` does with `okTarget`; this is that hook.
    #[must_use]
    pub fn gated_by(mut self: Box<Self>, gate: MobFilter) -> Box<Self> {
        self.gate = Some(gate);
        self
    }
}

impl Goal for ZombieAttackGoal {
    fn can_start(&mut self, mob: &dyn Mob) -> bool {
        self.gate.is_none_or(|gate| gate(mob)) && self.melee_attack_goal.can_start(mob)
    }

    fn should_continue(&mut self, mob: &dyn Mob) -> bool {
        if self.gate.is_some_and(|gate| !gate(mob)) {
            return false;
        }

        self.melee_attack_goal.should_continue(mob)
    }

    fn start(&mut self, mob: &dyn Mob) {
        self.melee_attack_goal.start(mob);
        self.ticks = 0;
    }

    fn stop(&mut self, mob: &dyn Mob) {
        self.melee_attack_goal.stop(mob);
        mob.get_mob_entity().set_attacking(false);
    }

    fn tick(&mut self, mob: &dyn Mob) {
        self.melee_attack_goal.tick(mob);
        self.ticks += 1;
        if self.ticks >= 5
            && self.melee_attack_goal.cooldown < self.melee_attack_goal.get_max_cooldown() / 2
        {
            mob.get_mob_entity().set_attacking(true);
        } else {
            mob.get_mob_entity().set_attacking(false);
        }
    }

    fn should_run_every_tick(&self) -> bool {
        self.melee_attack_goal.should_run_every_tick()
    }

    fn controls(&self) -> Controls {
        self.melee_attack_goal.controls()
    }
}
