use axum::body::Bytes;
use axum_typed_multipart::{FieldMetadata, TryFromChunks, TypedMultipartError};
use futures::Stream;
use iban::Iban;
use serde::de::DeserializeOwned;

pub async fn handle_chunks<T>(
    chunks: impl Stream<Item = Result<Bytes, TypedMultipartError>> + Send + Sync + Unpin,
    metadata: FieldMetadata,
) -> Result<T, TypedMultipartError>
where
    T: DeserializeOwned,
{
    let bytes = Bytes::try_from_chunks(chunks, metadata).await?;

    serde_json::from_slice(&bytes).map_err(|e| TypedMultipartError::Other { source: e.into() })
}

pub fn is_valid_iban(value: &str, _: &()) -> garde::Result {
    match value.parse::<Iban>() {
        Err(e) => Err(garde::Error::new(e)),
        _ => Ok(()),
    }
}

pub fn is_valid_phone_number(value: &str, _: &()) -> garde::Result {
    use phonenumber::country::Id;
    // Works if number is in international format
    if phonenumber::parse(None, value)
        .map(|number| number.is_valid())
        .unwrap_or(false)
    {
        return Ok(());
    }
    // Missing country code but number is otherwise valid, assume FI
    match phonenumber::parse(Some(Id::FI), value).map(|n| n.is_valid()) {
        Ok(true) => Ok(()),
        Err(e) => Err(garde::Error::new(format!("not a valid phone number: {e}"))),
        _ => Err(garde::Error::new("Invalid phone number")),
    }
}
