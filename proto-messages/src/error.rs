#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid coins: `{0}`")]
    Coins(String),
    #[error("invalid coin: `{0}`")]
    Coin(String),
    #[error(transparent)]
    Decode(#[from] prost::DecodeError),
    #[error("{0}")]
    DecodeProtobuf(String),
    #[error("decode error: `{0}`")]
    DecodeAny(String),
    #[error("missing field: `{0}`")]
    MissingField(String),
    #[error("decode error: `{0}`")]
    DecodeAddress(String),
    #[error("decode error: `{0}`")]
    DecodeGeneral(String),
    #[error("{0}")]
    SerdeSerialize(#[from] serde_json::Error),
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    Custom(String),
    #[error("invalid sign mode: `{0}`")]
    InvalidSignMode(i32),
}
