#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I2c error")]
    I2cError(embassy_rp::i2c::Error),

    #[error("IO error: {0}")]
    FormattingError(#[from] core::fmt::Error),
}

impl From<embassy_rp::i2c::Error> for Error {
    fn from(value: embassy_rp::i2c::Error) -> Self {
        Error::I2cError(value)
    }
}
