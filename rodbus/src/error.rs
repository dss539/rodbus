use crate::tokio;

/// Top level error type for the client API
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RequestError {
    /// An I/O error occurred
    Io(::std::io::ErrorKind),
    /// A Modbus exception was returned by the server
    Exception(crate::exception::ExceptionCode),
    /// Request was not performed because it is invalid
    BadRequest(InvalidRequest),
    /// Unable to parse a frame from the server
    BadFrame(FrameParseError),
    /// Response ADU was invalid
    BadResponse(AduParseError),
    /// An internal error occurred in the library itself
    ///
    /// These errors should never happen, but are trapped here for reporting purposes in case they ever do occur
    Internal(InternalError),
    /// timeout occurred before receiving a response from the server
    ResponseTimeout,
    /// no connection could be made to the Modbus server
    NoConnection,
    /// the task processing requests has been shutdown
    Shutdown,
}

impl std::error::Error for RequestError {}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            RequestError::Io(kind) => std::io::Error::from(*kind).fmt(f),
            RequestError::Exception(err) => err.fmt(f),
            RequestError::BadRequest(err) => err.fmt(f),
            RequestError::BadFrame(err) => err.fmt(f),
            RequestError::BadResponse(err) => err.fmt(f),
            RequestError::Internal(err) => err.fmt(f),
            RequestError::ResponseTimeout => f.write_str("response timeout"),
            RequestError::NoConnection => f.write_str("no connection to server"),
            RequestError::Shutdown => f.write_str("channel shutdown"),
        }
    }
}

impl From<std::io::Error> for RequestError {
    fn from(err: std::io::Error) -> Self {
        RequestError::Io(err.kind())
    }
}

impl From<InvalidRequest> for RequestError {
    fn from(err: InvalidRequest) -> Self {
        RequestError::BadRequest(err)
    }
}

impl From<InternalError> for RequestError {
    fn from(err: InternalError) -> Self {
        RequestError::Internal(err)
    }
}

impl From<AduParseError> for RequestError {
    fn from(err: AduParseError) -> Self {
        RequestError::BadResponse(err)
    }
}

impl From<crate::exception::ExceptionCode> for RequestError {
    fn from(err: crate::exception::ExceptionCode) -> Self {
        RequestError::Exception(err)
    }
}

impl From<FrameParseError> for RequestError {
    fn from(err: FrameParseError) -> Self {
        RequestError::BadFrame(err)
    }
}

impl From<InvalidRange> for InvalidRequest {
    fn from(x: InvalidRange) -> Self {
        InvalidRequest::BadRange(x)
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for RequestError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        RequestError::Shutdown
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for RequestError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        RequestError::Shutdown
    }
}

impl From<InvalidRange> for RequestError {
    fn from(x: InvalidRange) -> Self {
        RequestError::BadRequest(x.into())
    }
}

/// errors that can be produced when validating start/count
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InvalidRange {
    /// count of zero not allowed
    CountOfZero,
    /// address in range overflows u16
    AddressOverflow(u16, u16),
    /// count too large for type
    CountTooLargeForType(u16, u16), // actual and limit
}

/// errors that indicate faulty logic in the library itself if they occur
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InternalError {
    /// Insufficient space for write operation
    InsufficientWriteSpace(usize, usize), // written vs remaining space
    /// ADU size is larger than the maximum allowed size
    AduTooBig(usize),
    /// The calculated frame size exceeds what is allowed by the spec
    FrameTooBig(usize, usize), // calculate size vs allowed maximum
    /// Attempted to read more bytes than present
    InsufficientBytesForRead(usize, usize), // requested vs remaining
    /// Cursor seek operation exceeded the bounds of the underlying buffer
    BadSeekOperation,
    /// Byte count would exceed maximum allowed size in the ADU of u8
    BadByteCount(usize),
}

impl std::error::Error for InternalError {}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            InternalError::InsufficientWriteSpace(written, remaining) => write!(
                f,
                "attempted to write {} bytes with {} bytes remaining",
                written, remaining
            ),
            InternalError::AduTooBig(size) => write!(
                f,
                "ADU length of {} exceeds the maximum allowed length",
                size
            ),
            InternalError::FrameTooBig(size, max) => write!(
                f,
                "Frame length of {} exceeds the maximum allowed length of {}",
                size, max
            ),
            InternalError::InsufficientBytesForRead(requested, remaining) => write!(
                f,
                "attempted to read {} bytes with only {} remaining",
                requested, remaining
            ),
            InternalError::BadSeekOperation => {
                f.write_str("Cursor seek operation exceeded the bounds of the underlying buffer")
            }
            InternalError::BadByteCount(size) => write!(
                f,
                "Byte count of in ADU {} exceeds maximum size of u8",
                size
            ),
        }
    }
}

/// errors that occur while parsing a frame off a stream (TCP or serial)
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum FrameParseError {
    /// Received TCP frame with the length field set to zero
    MbapLengthZero,
    /// Received TCP frame with length that exceeds max allowed size
    MbapLengthTooBig(usize, usize), // actual size and the maximum size
    /// Received TCP frame within non-Modbus protocol id
    UnknownProtocolId(u16),
}

impl std::error::Error for FrameParseError {}

impl std::fmt::Display for FrameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            FrameParseError::MbapLengthZero => {
                f.write_str("Received TCP frame with the length field set to zero")
            }
            FrameParseError::MbapLengthTooBig(size, max) => write!(
                f,
                "Received TCP frame with length ({}) that exceeds max allowed size ({})",
                size, max
            ),
            FrameParseError::UnknownProtocolId(id) => {
                write!(f, "Received TCP frame with non-Modbus protocol id: {}", id)
            }
        }
    }
}

/// errors that occur while parsing requests and responses
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum AduParseError {
    /// response is too short to be valid
    InsufficientBytes,
    /// byte count doesn't match the actual number of bytes present
    InsufficientBytesForByteCount(usize, usize), // count / remaining
    /// response contains extra trailing bytes
    TrailingBytes(usize),
    /// a parameter expected to be echoed in the reply did not match
    ReplyEchoMismatch,
    /// an unknown response function code was received
    UnknownResponseFunction(u8, u8, u8), // actual, expected, expected error
    /// Bad value for the coil state
    UnknownCoilState(u16),
}

impl std::error::Error for AduParseError {}

impl std::fmt::Display for AduParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            AduParseError::InsufficientBytes => f.write_str("response is too short to be valid"),
            AduParseError::InsufficientBytesForByteCount(count, remaining) => write!(
                f,
                "byte count ({}) doesn't match the actual number of bytes remaining ({})",
                count, remaining
            ),
            AduParseError::TrailingBytes(remaining) => {
                write!(f, "response contains {} extra trailing bytes", remaining)
            }
            AduParseError::ReplyEchoMismatch => {
                f.write_str("a parameter expected to be echoed in the reply did not match")
            }
            AduParseError::UnknownResponseFunction(actual, expected, error) => write!(
                f,
                "received unknown response function code: {}. Expected {} or {}",
                actual, expected, error
            ),
            AduParseError::UnknownCoilState(value) => write!(
                f,
                "received coil state with unspecified value: 0x{:04X}",
                value
            ),
        }
    }
}

/// errors that result because of bad request parameter
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InvalidRequest {
    /// Request contained an invalid range
    BadRange(InvalidRange),
    /// Count is too big to fit in a u16
    CountTooBigForU16(usize),
    /// Count too big for specific request
    CountTooBigForType(u16, u16),
}

impl std::error::Error for InvalidRequest {}

impl std::fmt::Display for InvalidRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            InvalidRequest::BadRange(err) => write!(f, "{}", err),

            InvalidRequest::CountTooBigForU16(count) => write!(
                f,
                "The requested count of objects exceeds the maximum value of u16: {}",
                count
            ),
            InvalidRequest::CountTooBigForType(count, max) => write!(
                f,
                "the request count of {} exceeds maximum allowed count of {} for this type",
                count, max
            ),
        }
    }
}

impl std::fmt::Display for InvalidRange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            InvalidRange::CountOfZero => f.write_str("range contains count == 0"),
            InvalidRange::AddressOverflow(start, count) => write!(
                f,
                "start == {} and count = {} would overflow u16 representation",
                start, count
            ),
            InvalidRange::CountTooLargeForType(x, y) => write!(
                f,
                "count of {} is too large for the specified type (max == {})",
                x, y
            ),
        }
    }
}
