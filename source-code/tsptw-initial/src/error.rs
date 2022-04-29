use std::num::{ParseIntError, ParseFloatError};


#[derive(Debug, thiserror::Error)]
pub enum TsptwError {
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
    #[error("n_cities is not a valid number {0}")]
    NbCities(ParseIntError),
    #[error("matrix coefficient ({0},{1}) = {2}")]
    MatrixCoeff(usize, usize, ParseFloatError),
    #[error("start of time window {0}")]
    TwStart(ParseFloatError),
    #[error("stop of time window {0}")]
    TwStop(ParseFloatError),
    #[error("no tw start (line: {0})")]
    NoTwStart(usize),
    #[error("no tw stop (line: {0})")]
    NoTwStop(usize),
}