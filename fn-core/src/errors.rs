use failure::Fail;

#[derive(Debug, Fail)]
pub enum Errors {
    #[fail(display = "Failed to serialize the value: {}", _0)]
    SerializationError(String),

    #[fail(display = "Failed to deserialize the value: {}", _0)]
    DeserializationError(String),
}
