use derive_discriminant::Discriminant;
use serde::{Deserialize, Serialize};

#[derive(Discriminant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Client {
    /// Send an instruction. This initiates a question-answer session.
    Instruction { instruction: String },
    /// Answer a question.
    Answer { answer: String },
}

impl From<Instruction> for String {
    fn from(instruction: Instruction) -> Self {
        instruction.instruction
    }
}
