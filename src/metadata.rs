use std::collections::HashMap;
use std::sync::Arc;

use hass_rs::HassArea;
use hass_rs::HassDevice;
use hass_rs::HassEntity;

pub struct MetadataTree {
	entities_by_id: HashMap<String, HassEntity>,
	devices_by_id: HashMap<String, HassDevice>,
	areas_by_id: HashMap<String, HassArea>
}

#[derive(Debug)]
pub struct MetadataResult<'a> {
	pub entity: &'a HassEntity,
	pub device: &'a HassDevice,
	pub area: Option<&'a HassArea>
}

impl MetadataTree {
	pub fn new(areas: Vec<HassArea>, devices: Vec<HassDevice>, entities: Vec<HassEntity>) -> Arc<Self> {
		Arc::new(Self {
			entities_by_id: entities.into_iter().map(|e| (e.entity_id.clone(), e)).collect(),
			devices_by_id: devices.into_iter().map(|d| (d.id.clone(), d)).collect(),
			areas_by_id: areas.into_iter().map(|a| (a.id.clone(), a)).collect()
		})
	}

	pub fn find(&self, entity_id: &str) -> Option<MetadataResult<'_>> {
		let entity = self.entities_by_id.get(entity_id)?;
		let device = self.devices_by_id.get(entity.device_id.as_ref()?)?;
		let area = device.area_id.as_ref().and_then(|id| self.areas_by_id.get(id));
		Some(MetadataResult { entity, device, area })
	}
}
