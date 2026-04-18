use crate::domain::shared::{DeviceId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDevice {
    pub device_id: DeviceId,
    pub display_name: String,
    pub public_key: [u8; 32],
    pub first_seen: Timestamp,
    pub last_seen: Option<Timestamp>,
}
