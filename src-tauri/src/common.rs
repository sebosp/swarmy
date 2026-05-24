use polars::prelude::*;
use swarmy_tauri_common::error::SwarmyTauriError;

/// Converts a Dataframe into a String, this is expensive but useful for small results.
pub fn convert_df_to_json_data(df: &DataFrame) -> Result<String, SwarmyTauriError> {
    let mut buf = Vec::new();
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df.clone())?;
    Ok(String::from_utf8(buf)?)
}
