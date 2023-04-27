use thiserror::Error as ThisError;
use std::result::Result as StdResult;

#[derive(Debug, ThisError)]
pub enum Error {}

pub type Result<T> = StdResult<T, Error>;
