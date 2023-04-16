use derive_discriminant::Discriminant;
use serde::{Deserialize, Serialize};

#[derive(Discriminant)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Server {
    Question { question: String },
}
