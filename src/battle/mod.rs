#[cfg(test)]
mod tests;

use crate::creature::CreatureId;
use crate::encounter::Encounter;
use crate::party::Party;

/// Represents the phases of a battle turn
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleState {
    StartTurn,
    SelectActions,
    ResolveActions,
    EndTurn,
    Finished,
}

/// Represents a single turn and the actions selected for it
#[derive(Debug)]
pub struct Turn {
    pub turn_number: u32,
    pub actions: Vec<BattleAction>,
}

impl Turn {
    pub fn new(turn_number: u32) -> Self {
        Self {
            turn_number,
            actions: Vec::new(),
        }
    }

    pub fn add_action(&mut self, action: BattleAction) {
        self.actions.push(action);
    }
}

/// Represents a possible action a creature can take in battle
#[derive(Debug, Clone)]
pub enum BattleAction {
    Attack {
        attacker_id: CreatureId,
        target_id: CreatureId,
    },
    Switch {
        out_id: CreatureId,
        in_id: CreatureId,
    },
    UseItem {
        user_id: CreatureId,
        item_id: u32,
    },
    Pass,
}

/// The Battle struct itself, managing parties and turn state
pub struct Battle {
    pub parties: [Party; 2],
    pub state: BattleState,
    pub current_turn: Turn,
}

impl Battle {
    pub fn new(party1: Party, party2: Party) -> Self {
        Self {
            parties: [party1, party2],
            state: BattleState::StartTurn,
            current_turn: Turn::new(1),
        }
    }

    /// Advance to the next state in the turn cycle
    pub fn advance_state(&mut self) {
        self.state = match self.state {
            BattleState::StartTurn => BattleState::SelectActions,
            BattleState::SelectActions => BattleState::ResolveActions,
            BattleState::ResolveActions => BattleState::EndTurn,
            BattleState::EndTurn => {
                self.current_turn = Turn::new(self.current_turn.turn_number + 1);
                BattleState::StartTurn
            }
            BattleState::Finished => BattleState::Finished,
        };
    }
}

impl Encounter for Battle {
    fn process_turn(&mut self) {
        if self.state == BattleState::Finished {
            return;
        }

        if self.parties.iter().any(|party| party.all_fainted()) {
            self.state = BattleState::Finished;
            return;
        }

        // Placeholder: future logic to collect and execute actions
        self.advance_state();

        if self.parties.iter().any(|party| party.all_fainted()) {
            self.state = BattleState::Finished;
        }
    }

    fn is_over(&self) -> bool {
        self.state == BattleState::Finished
    }
}
