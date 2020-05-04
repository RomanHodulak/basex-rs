use super::{Result, Connection};

/// Represents database command code in the [query mode](https://docs.basex.org/wiki/Query_Mode).
pub enum Command {
    Close = 2,
    Bind = 3,
    Results = 4,
    Execute = 5,
    Info = 6,
    Options = 7,
    Context = 0x0e,
    Updating = 0x1e,
}

pub struct Query {
    id: String,
    connection: Connection,
}

impl Query {

    pub fn new(id: String, connection: Connection) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Close as u8, vec![Some(&self.id)])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Bind as u8, vec![Some(&self.id), Some(name), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Execute as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Info as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Options as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Command::Context as u8, vec![Some(&self.id), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Command::Updating as u8, vec![Some(&self.id)])?;
        self.connection.get_response()
    }
}
