use core::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Value(pub f64);

impl FromStr for Value {
	type Err = ParseError;
	fn from_str(input: &str) -> Result<Self, Self::Err> {
		if let Ok(v) = input.parse::<f64>() {
			return Ok(Self(v));
		}

		if let Ok(v) = input.parse::<i64>() {
			return Ok(Self(v as f64));
		}

		Err(ParseError)
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to parse value")]
pub struct ParseError;

