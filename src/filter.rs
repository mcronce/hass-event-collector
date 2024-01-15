use core::fmt;
use core::str::FromStr;

use compact_str::CompactString;
use hass_rs::MqttEvent;
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EntityFilter(Vec<IndividualEntityFilter>);

impl EntityFilter {
	pub fn matches_event(&self, ev: &MqttEvent) -> bool {
		self.0.iter().any(|f| f.matches_event(ev))
	}
}

impl FromStr for EntityFilter {
	type Err = serde_json::Error;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		serde_json::from_str(input)
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndividualEntityFilter {
	kind: CompactString,
	#[serde(with = "serde_regex", default)]
	name: Option<Regex>
}

impl IndividualEntityFilter {
	pub fn matches_event(&self, ev: &MqttEvent) -> bool {
		let Some((kind, name)) = ev.event_data.entity_id.split_once('.') else {
			return false;
		};

		if (kind != self.kind) {
			return false;
		}

		if let Some(name_filter) = self.name.as_ref() {
			if (name_filter.is_match(name)) {
				return true;
			}
			return false;
		}

		true
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DefaultFilter {
	Deny,
	Allow
}

impl FromStr for DefaultFilter {
	type Err = InvalidDefaultFilter;
	fn from_str(input: &str) -> Result<Self, Self::Err> {
		match input {
			"deny" => Ok(Self::Deny),
			"allow" => Ok(Self::Allow),
			s => Err(InvalidDefaultFilter(s.to_owned()))
		}
	}
}

impl fmt::Display for DefaultFilter {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Deny => f.write_str("deny"),
			Self::Allow => f.write_str("allow")
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid default filter '{0}'; must be 'deny' or 'allow'")]
pub struct InvalidDefaultFilter(String);

