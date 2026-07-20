use crate::domain::app::entities::KnownDevice;
use crate::domain::shared::{DeviceId, StorageError};
use crate::infrastructure::sqlite::app::entities::known_device::Model;

use super::{fixed_key, string_to_ts, string_to_ts_opt, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<KnownDevice, StorageError> {
    Ok(KnownDevice {
        device_id: DeviceId::from_raw(model.device_id),
        display_name: model.display_name,
        public_key: fixed_key(&model.public_key, "known_device.public_key")?,
        first_seen: string_to_ts(&model.first_seen)?,
        last_seen: string_to_ts_opt(model.last_seen.as_deref())?,
    })
}

pub fn domain_to_model(device: &KnownDevice) -> Model {
    Model {
        device_id: device.device_id.as_str().to_owned(),
        display_name: device.display_name.clone(),
        public_key: device.public_key.to_vec(),
        first_seen: ts_to_string(&device.first_seen),
        last_seen: device.last_seen.as_ref().map(ts_to_string),
    }
}
