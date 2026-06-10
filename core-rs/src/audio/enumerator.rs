use cpal::traits::{DeviceTrait, HostTrait};

use crate::audio::traits::DeviceEnumerator;
use crate::audio::types::{AudioError, DeviceInfo};

/// Device enumerator backed by cpal.
pub struct Enumerator;

impl DeviceEnumerator for Enumerator {
    fn list_input_devices(&self) -> Result<Vec<DeviceInfo>, AudioError> {
        let host = cpal::default_host();
        let default_dev = host.default_input_device();
        let default_name = default_dev.as_ref().and_then(|d| d.name().ok());

        let devices: Vec<cpal::Device> = host
            .input_devices()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?
            .collect();

        let mut result = Vec::with_capacity(devices.len());
        for device in devices {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let is_default = default_name.as_deref() == Some(name.as_str());

            let mut channels = 1u32;
            let mut sample_rates: Vec<u32> = Vec::new();
            if let Ok(configs) = device.supported_input_configs() {
                for cfg in configs {
                    channels = channels.max(cfg.channels().into());
                    let min = cfg.min_sample_rate().0;
                    let max = cfg.max_sample_rate().0;
                    sample_rates.push(min);
                    if max > min {
                        sample_rates.push(max);
                    }
                }
            }
            sample_rates.sort();
            sample_rates.dedup();

            result.push(DeviceInfo {
                id: name.clone(),
                name,
                is_default,
                channels,
                sample_rates,
            });
        }

        Ok(result)
    }

    fn default_input_device(&self) -> Result<DeviceInfo, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoAudioHardware)?;
        let name = device
            .name()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        let devices = self.list_input_devices()?;
        devices.into_iter().find(|d| d.id == name).ok_or_else(|| {
            AudioError::DeviceError("default device not found in device list".to_string())
        })
    }
}
