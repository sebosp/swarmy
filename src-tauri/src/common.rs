use polars::prelude::*;
use swarmy_common::error::SwarmyError;

/// Converts a Dataframe into a String, this is expensive but useful for small results.
pub fn convert_df_to_json_data(df: &DataFrame) -> Result<String, SwarmyError> {
    let mut buf = Vec::new();
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df.clone())?;
    Ok(String::from_utf8(buf)?)
}

/// Runs a blocking query, i.e. a polars scan, without blocking the async runtime.
/// A panic in the query becomes a failure response instead of a request that never resolves.
pub async fn run_query<F>(query: F) -> Result<swarmy_common::ApiResponse, SwarmyError>
where
    F: FnOnce() -> swarmy_common::ApiResponse + Send + 'static,
{
    Ok(tauri::async_runtime::spawn_blocking(query)
        .await
        .unwrap_or_else(|err| {
            tracing::error!("Query task failed: {err:?}");
            swarmy_common::ApiResponseBuilder::new()
                .with_message(format!("The query failed: {err}"))
                .with_failure()
                .build()
        }))
}

/// Encodes RGBA8 pixels as a PNG data URL, to be shown in an `img` of the frontend.
pub fn rgba_png_data_url(width: u32, height: u32, pixels: &[u8]) -> Result<String, SwarmyError> {
    use base64::Engine;
    let mut png_bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| SwarmyError::Other(format!("PNG header: {err}")))?;
        writer
            .write_image_data(pixels)
            .map_err(|err| SwarmyError::Other(format!("PNG data: {err}")))?;
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png_bytes)
    ))
}
