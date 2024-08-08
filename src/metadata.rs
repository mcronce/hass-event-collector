use std::collections::HashMap;

use hass_rs::HassArea;
use hass_rs::HassClient;
use hass_rs::HassDevice;
use hass_rs::HassEntity;
use hass_rs::HassError;
use tracing::instrument;

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
	pub fn new(areas: Vec<HassArea>, devices: Vec<HassDevice>, entities: Vec<HassEntity>) -> Self {
		Self {
			entities_by_id: entities.into_iter().map(|e| (e.entity_id.clone(), e)).collect(),
			devices_by_id: devices.into_iter().map(|d| (d.id.clone(), d)).collect(),
			areas_by_id: areas.into_iter().map(|a| (a.id.clone(), a)).collect()
		}
	}

	#[instrument(level = "debug", skip_all, err)]
	pub async fn load(client: &mut HassClient) -> Result<Self, HassError> {
		let areas = client.get_area_registry().await?;
		let devices = client.get_device_registry().await?;
		let entities = client.get_entity_registry().await?;

		// TODO:  Filter these somehow
		let meta = Self::new(areas, devices, entities);
		Ok(meta)
	}

	pub fn find(&self, entity_id: &str) -> Option<MetadataResult<'_>> {
		let entity = self.entities_by_id.get(entity_id)?;
		let device = self.devices_by_id.get(entity.device_id.as_ref()?)?;
		let area = device.area_id.as_ref().and_then(|id| self.areas_by_id.get(id));
		Some(MetadataResult { entity, device, area })
	}
}
