use rusqlite::Connection;

pub mod log;
pub mod message;

pub trait Model {
    fn init_table(connection: &Connection) -> Result<(), anyhow::Error>;
}
