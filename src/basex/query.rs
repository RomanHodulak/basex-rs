use super::{Result, Connection};

pub struct Query {
    id: String,
    connection: Connection,
}

impl Query {
    const CLOSE_CODE: u8 = 2;
    const BIND_CODE: u8 = 3;
    const RESULTS_CODE: u8 = 4;
    const EXECUTE_CODE: u8 = 5;
    const INFO_CODE: u8 = 6;
    const OPTIONS_CODE: u8 = 7;
    const CONTEXT_CODE: u8 = 0x0e;
    const UPDATING_CODE: u8 = 0x1e;

    pub fn new(id: String, connection: Connection) -> Self {
        Self { id, connection }
    }

    pub fn close(&mut self) -> Result<&mut Self> {
        self.connection.send_cmd(Self::CLOSE_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn bind(&mut self, name: &str, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Self::BIND_CODE, vec![Some(&self.id), Some(name), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn execute(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::EXECUTE_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn info(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::INFO_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn options(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::OPTIONS_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }

    pub fn context(&mut self, value: Option<&str>, value_type: Option<&str>) -> Result<&mut Self> {
        self.connection.send_cmd(Self::CONTEXT_CODE, vec![Some(&self.id), value, value_type])?;
        self.connection.get_response()?;
        Ok(self)
    }

    pub fn updating(&mut self) -> Result<String> {
        self.connection.send_cmd(Self::UPDATING_CODE, vec![Some(&self.id)])?;
        self.connection.get_response()
    }
}
