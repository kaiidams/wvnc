#[derive(Debug)]
pub enum ErrorKind {
    Generic,
    FromUtf8String,
    Protocol,
    IO,
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
}

impl Error {
    pub const GENERIC: Self = Self {
        kind: ErrorKind::Generic,
    };
    pub const FROM_UTF8_STRING: Self = Self {
        kind: ErrorKind::FromUtf8String,
    };
    pub const PROTOCOL: Self = Self {
        kind: ErrorKind::Protocol,
    };
    pub const IO: Self = Self {
        kind: ErrorKind::IO,
    };
}

unsafe impl Send for Error {}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_value: std::string::FromUtf8Error) -> Self {
        Error::FROM_UTF8_STRING
    }
}

impl From<std::io::Error> for Error {
    fn from(_value: std::io::Error) -> Self {
        Error::IO
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(_value: std::array::TryFromSliceError) -> Self {
        Error::IO
    }
}
