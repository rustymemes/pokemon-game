use crate::experience::Level;
use crate::moves::MoveId;
use crate::species::{Species, SpeciesId};
use crate::species_registry::SpeciesRegistry;
use crate::stats::{IndividualStats, Stat};
use uuid::Uuid;

/// Globally unique identifier for each persistent creature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CreatureId(Uuid);

impl Default for CreatureId {
    fn default() -> Self {
        Self::new()
    }
}

impl CreatureId {
    pub fn new() -> Self {
        CreatureId(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        CreatureId(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

/// Persistent creature instance
#[derive(Debug, Clone)]
pub struct Creature {
    pub id: CreatureId,
    pub species_id: SpeciesId,
    pub name: String,
    pub level: Level,
    pub experience: u32,
    pub individual_stats: IndividualStats,
    pub current_hp: u16, // Plain u16, allows 0 for fainted state
    pub moves: MoveSlots,
}

impl Creature {
    pub fn new(species: &Species, starting_level: u8) -> Option<Self> {
        let id = CreatureId::new();
        let species_id = species.id;
        let name = species.name.clone().to_string();
        let individual_stats = IndividualStats::from_base(&species.base_stats);
        let level = Level::new(starting_level)?;
        let experience = species.growth_rate.exp_for_level(level);

        let mut creature = Self {
            id,
            species_id,
            name,
            level,
            experience,
            individual_stats,
            current_hp: 0,
            moves: [None, None, None, None],
        };

        creature.calculate_stats(species);
        creature.current_hp = creature.individual_stats.max_hp.get();
        Some(creature)
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn modify_hp(&mut self, amount: i16) {
        let hp = self.current_hp as i16 + amount;
        let max_hp = self.individual_stats.max_hp.get();
        self.current_hp = hp.clamp(0, max_hp as i16) as u16;
    }

    pub fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }

    pub fn gain_exp<S: SpeciesRegistry>(
        &mut self,
        amount: u32,
        species_registry: &S,
    ) -> Vec<LevelUpEvent> {
        let mut events = Vec::new();

        let growth_rate = match species_registry.get_growth_rate(self.species_id) {
            Some(gr) => gr,
            None => panic!("things are broken with your registry"), // things are wonky if this happens
        };
        let max_exp = growth_rate.exp_for_level(Level::new(Level::MAX).expect("Max level valid"));
        self.experience = self.experience.saturating_add(amount).min(max_exp);

        while let Some(next_level) = self.level.next() {
            let next_level_exp = growth_rate.exp_for_level(next_level);
            if self.experience >= next_level_exp {
                let mut level_events = self.level_up(next_level, species_registry);
                events.append(&mut level_events);
            } else {
                break;
            }
        }
        events
    }

    fn level_up<S: SpeciesRegistry>(
        &mut self,
        next_level: Level,
        species_registry: &S,
    ) -> Vec<LevelUpEvent> {
        let events = self.on_level_up(next_level, species_registry);
        self.level = next_level;
        events
    }

    fn on_level_up<S: SpeciesRegistry>(
        &mut self,
        level: Level,
        species_registry: &S,
    ) -> Vec<LevelUpEvent> {
        let mut events = Vec::new();
        let species = match species_registry.get_species(self.species_id) {
            Some(s) => s,
            None => panic!(),
        };

        self.calculate_stats(species);

        for m in species.learnset.iter().filter(|m| m.level == level) {
            events.push(LevelUpEvent::CanLearnMove {
                move_id: m.move_id.clone(),
            });
        }

        //if let Some(new_species_id) = species.check_evolution(level) {
        //    events.push(LevelUpEvent::CanEvolve { species_id: new_species_id })
        //}

        events
    }

    fn calculate_stats(&mut self, species: &Species) {
        let stats = &species.base_stats;
        self.individual_stats.max_hp = Self::calculate_hp(stats.max_hp, self.level);
        self.individual_stats.attack = Self::calculate_stat(stats.attack, self.level);
        self.individual_stats.defense = Self::calculate_stat(stats.defense, self.level);
        self.individual_stats.speed = Self::calculate_stat(stats.speed, self.level);
    }

    fn calculate_hp(hp: Stat, level: Level) -> Stat {
        let hp = (2 * hp.get() * level.get() as u16) / 100 + level.get() as u16 + 10;
        Stat::new(hp).expect("HP within bounds")
    }

    fn calculate_stat(stat: Stat, level: Level) -> Stat {
        let stat = (2 * stat.get() * level.get() as u16) / 100 + 5;
        Stat::new(stat).expect("Stat within bounds")
    }

    pub fn try_learn_move(&mut self, move_id: MoveId, max_pp: u8) -> LearnMoveResult {
        if self
            .moves
            .iter()
            .any(|m| m.as_ref().is_some_and(|m| m.move_id == move_id))
        {
            return LearnMoveResult::AlreadyKnown;
        }
        if let Some((_index, slot)) = self
            .moves
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        {
            *slot = Some(CreatureMove {
                move_id,
                pp: MovePP {
                    current: max_pp,
                    max: max_pp,
                },
            });
            return LearnMoveResult::Learned;
        }
        LearnMoveResult::MustForgetOldMove
    }

    pub fn forget_move(&mut self, slot: usize) -> Option<CreatureMove> {
        self.moves.get_mut(slot)?.take()
    }
}

#[derive(Debug, Clone)]
pub enum LevelUpEvent {
    CanEvolve { species_id: SpeciesId },
    CanLearnMove { move_id: MoveId },
}
#[derive(Debug, Clone)]
pub enum LearnMoveResult {
    AlreadyKnown,
    Learned,
    MustForgetOldMove,
}

#[derive(Debug, Clone, Copy)]
pub struct MovePP {
    pub current: u8,
    pub max: u8,
}

#[derive(Debug, Clone)]
pub struct CreatureMove {
    pub move_id: MoveId,
    pub pp: MovePP,
}

pub type MoveSlots = [Option<CreatureMove>; 4];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::GrowthRate;
    use crate::species::SpeciesId;
    use crate::tests::helpers::MockRegistry;

    fn test_creature(level: u8, registry: &MockRegistry) -> Creature {
        Creature::new(registry.get_species(SpeciesId(1)).unwrap(), level).unwrap()
    }

    #[test]
    fn single_level_up() {
        let registry = MockRegistry::new();
        let growth_rate = registry.get_growth_rate(SpeciesId(1)).unwrap();
        let mut creature = test_creature(5, &registry);

        let target_level = creature.level.next().unwrap();
        let required_exp = growth_rate.exp_for_level(target_level);

        creature.gain_exp(required_exp, &registry);

        assert_eq!(creature.level, Level::new(6).unwrap());
    }

    #[test]
    fn reach_max_level() {
        let registry = MockRegistry::new();
        let growth_rate = registry.get_growth_rate(SpeciesId(1)).unwrap();
        let mut creature = test_creature(95, &registry);
        let exp_needed =
            growth_rate.exp_for_level(Level::new(Level::MAX).unwrap()) - creature.experience;
        creature.gain_exp(exp_needed, &registry);

        assert_eq!(creature.level.get(), Level::MAX);
        assert_eq!(
            creature.experience,
            growth_rate.exp_for_level(Level::new(Level::MAX).unwrap())
        );
    }

    #[test]
    fn multiple_level_ups() {
        let registry = MockRegistry::new();
        let growth_rate = registry.get_growth_rate(SpeciesId(1)).unwrap();
        let mut creature = test_creature(5, &registry);

        let exp_to_gain = growth_rate.exp_for_level(Level::new(8).unwrap()) - creature.experience;
        creature.gain_exp(exp_to_gain, &registry);

        assert_eq!(creature.level, Level::new(8).unwrap());
        assert_eq!(
            creature.experience,
            growth_rate.exp_for_level(Level::new(8).unwrap())
        );
    }

    #[test]
    fn creature_creation() {
        let registry = MockRegistry::new();
        let c = test_creature(5, &registry);

        assert_eq!(c.current_hp, 20);
        assert_eq!(c.level.get(), 5);
        assert!(c.name().contains("Bulby"));
        assert_eq!(c.individual_stats.attack.get(), 10);
    }

    #[test]
    fn hp_modification() {
        let registry = MockRegistry::new();
        let mut c = test_creature(5, &registry);

        c.modify_hp(-10);
        assert_eq!(c.current_hp, 10);

        c.modify_hp(5);
        assert_eq!(c.current_hp, 15);

        c.modify_hp(100);
        assert_eq!(c.current_hp, 20);

        c.modify_hp(-100);
        assert_eq!(c.current_hp, 0);
        assert!(c.is_fainted());
    }

    #[test]
    fn level_up_triggers_learn_move_event() {
        let species_registry = MockRegistry::new(); // learnset in mock: lvl 5, 10, 15

        let mut creature =
            Creature::new(species_registry.get_species(SpeciesId(1)).unwrap(), 4).unwrap();

        let events = creature.level_up(Level::new(5).unwrap(), &species_registry);

        assert_eq!(events.len(), 1);

        match &events[0] {
            LevelUpEvent::CanLearnMove { move_id } => {
                assert_eq!(*move_id, MoveId(1)); // corresponds to the first move in MockMoveRegistry
            }
            _ => panic!("Expected CanLearnMove event at level 5"),
        }
    }

    #[test]
    fn gain_exp_triggers_multiple_learn_move_events() {
        let species_registry = MockRegistry::new(); // learnset: lvl 5, 10, 15

        let mut creature =
            Creature::new(species_registry.get_species(SpeciesId(1)).unwrap(), 9).unwrap();
        let gr = GrowthRate::Fast;

        let needed_exp =
            gr.exp_for_level(Level::new(15).unwrap()) - gr.exp_for_level(Level::new(9).unwrap());

        // Give enough EXP to jump to level 15
        let events = creature.gain_exp(needed_exp, &species_registry);

        // We expect two learn-move events: for levels 10 and 15
        let learn_move_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, LevelUpEvent::CanLearnMove { .. }))
            .collect();

        assert_eq!(learn_move_events.len(), 2);

        // Assert the move IDs directly using pattern matching
        match learn_move_events[0] {
            LevelUpEvent::CanLearnMove { move_id } => assert_eq!(*move_id, MoveId(2)), // level 10
            _ => panic!("Expected CanLearnMove event at level 10"),
        }

        match learn_move_events[1] {
            LevelUpEvent::CanLearnMove { move_id } => assert_eq!(*move_id, MoveId(3)), // level 15
            _ => panic!("Expected CanLearnMove event at level 15"),
        }
    }

    #[test]
    fn gain_exp_saturates_to_max_level_experience() {
        let registry = MockRegistry::new();
        let growth_rate = registry.get_growth_rate(SpeciesId(1)).unwrap();
        let mut creature = test_creature(5, &registry);

        let events = creature.gain_exp(u32::MAX, &registry);

        assert_eq!(creature.level.get(), Level::MAX);
        assert_eq!(
            creature.experience,
            growth_rate.exp_for_level(Level::new(Level::MAX).unwrap())
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, LevelUpEvent::CanLearnMove { .. }))
        );
    }
}
