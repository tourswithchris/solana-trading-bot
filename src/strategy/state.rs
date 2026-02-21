use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotState {
    Idle,
    Watch,
    Candidate,
    Simulate,
    Execute,
    Confirm,
    Cooldown,
}

#[derive(Debug)]
pub struct StateMachine {
    pub state: BotState,
    pub entered_at: Instant,
    pub cooldown_until: Option<Instant>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            state: BotState::Idle,
            entered_at: Instant::now(),
            cooldown_until: None,
        }
    }

    pub fn transition(&mut self, next: BotState) {
        self.state = next;
        self.entered_at = Instant::now();
    }

    pub fn set_cooldown(&mut self, d: Duration) {
        self.cooldown_until = Some(Instant::now() + d);
        self.transition(BotState::Cooldown);
    }

    pub fn cooldown_done(&self) -> bool {
        match self.cooldown_until {
            Some(t) => Instant::now() >= t,
            None => true,
        }
    }
}
