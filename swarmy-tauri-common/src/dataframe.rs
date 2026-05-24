//! Some dataframe related utilities.
#[cfg(not(target_arch = "wasm32"))]
use polars::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::SwarmyTauriError;

#[cfg(not(target_arch = "wasm32"))]
pub fn col_ymd_to_naive_date(
    df: &DataFrame,
    col_name: &str,
) -> Result<chrono::NaiveDate, SwarmyTauriError> {
    let date_str = df.column(col_name)?.str()?.get(0).unwrap_or("1970-01-01");

    Ok(chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .unwrap_or(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()))
}
