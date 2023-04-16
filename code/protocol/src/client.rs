use derive_discriminant::Discriminant;
use serde::{Deserialize, Serialize};

#[derive(Discriminant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Client {
    Instruction { instruction: String },
    Answer { answer: String },
}

impl From<Instruction> for String {
    fn from(instruction: Instruction) -> Self {
        instruction.instruction
    }
}
