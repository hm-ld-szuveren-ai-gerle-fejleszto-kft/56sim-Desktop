//! Interrogation Room 407 — engine, verdict parsing, stub + LLM trait.
//!
//! The four contract items below are FROZEN (sim-app `llm.rs` codes against
//! them exactly); engine internals are ported verbatim from
//! `~/Desktop/56sim-iOS/56sim/InterrogationEngine.swift`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChatTurn {
    pub role: ChatRole,
    pub text: String,
}

pub enum AskEvent {
    Delta(String),
    Finished,
    Error(String),
}

pub trait Interrogator: Send {
    /// Streams one interrogator reply for the given history (system prompt excluded).
    fn ask(&mut self, history: &[ChatTurn]) -> Box<dyn Iterator<Item = AskEvent> + Send>;
}
