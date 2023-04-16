#![feature(unsize)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{client::Client, server::Server};

pub mod client;
pub mod server;

pub type PacketId = Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Packet<T> {
    pub id: PacketId,
    pub data: T,
}

pub type ClientPacket = Packet<Client>;
pub type ServerPacket = Packet<Server>;

impl<T> Packet<T> {
    pub fn new(data: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            data,
        }
    }
}

impl Packet<Server> {
    pub fn server(data: impl Into<Server>) -> Self {
        Self::new(data.into())
    }
}

impl Packet<Client> {
    pub fn client(data: impl Into<Client>) -> Self {
        Self::new(data.into())
    }
}
