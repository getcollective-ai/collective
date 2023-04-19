use derive_discriminant::Discriminant;
use serde::{Deserialize, Serialize};

#[derive(Discriminant)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Server {
    Question {
        question: String,
        is_first_word: bool,
        is_last_word: bool,
    },
}
