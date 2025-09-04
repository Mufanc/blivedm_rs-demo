use std::fmt;
use std::fmt::{Display, Formatter};

pub struct Credential {
    sessdata: String
}

impl Credential {
    pub fn from_sessdata(sessdata: &str) -> Self {
        Self { sessdata: sessdata.into() }
    }
}

impl Display for Credential {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "SESSDATA={}", self.sessdata)
    }
}
