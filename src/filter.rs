use core::str::FromStr;

use compact_str::CompactString;
use hass_rs::MqttEvent;
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EntityFilter {
	kind: CompactString,
	#[serde(with = "serde_regex", default)]
	name: Option<Regex>
}

impl EntityFilter {
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

impl FromStr for EntityFilter {
	type Err = serde_json::Error;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		serde_json::from_str(input)
	}
}
