use std::{
    mem,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use windows::{
    core::{Free, PCWSTR},
    Win32::{
        Devices::{DeviceAndDriverInstallation, HumanInterfaceDevice},
        Foundation,
        Storage::FileSystem,
    },
};

pub struct DeviceInfo {
    location: String,
    product_string: Option<String>,
    product_id: u16,
    vendor_id: u16,
    h: OwnedHandle,
}

impl DeviceInfo {
    pub fn enumerate() -> Result<Vec<Self>, super::Error> {
        let guid = unsafe { HumanInterfaceDevice::HidD_GetHidGuid() };
        let info = unsafe {
            DeviceAndDriverInstallation::SetupDiGetClassDevsW(
                Some(&guid),
                None,
                None,
                DeviceAndDriverInstallation::DIGCF_PRESENT
                    | DeviceAndDriverInstallation::DIGCF_DEVICEINTERFACE,
            )
        }?;
        let info = OwnedDeviceInfo(info);

        let mut i = 0;
        let mut d_data = DeviceAndDriverInstallation::SP_DEVICE_INTERFACE_DATA::default();
        d_data.cbSize = mem::size_of_val(&d_data) as _;

        let re_pid = regex::Regex::new("[Pp][Ii][Dd]_([A-Fa-f0-9]+)").unwrap();
        let re_vid = regex::Regex::new("[Vv][Ii][Dd]_([A-Fa-f0-9]+)").unwrap();

        let mut info_list = vec![];

        while let Ok(_) = unsafe {
            DeviceAndDriverInstallation::SetupDiEnumDeviceInterfaces(
                info.0,
                None,
                &guid,
                i,
                &mut d_data,
            )
        } {
            i += 1;

            let mut required_size = 0;
            let _ = unsafe {
                DeviceAndDriverInstallation::SetupDiGetDeviceInterfaceDetailW(
                    info.0,
                    &d_data,
                    None,
                    0,
                    Some(&mut required_size),
                    None,
                )
            };
            if required_size < 1 {
                continue;
            }

            let mut detail: Vec<u8> = Vec::with_capacity(required_size as _);
            let p = unsafe {
                &mut *(detail.as_mut_ptr()
                    as *mut DeviceAndDriverInstallation::SP_DEVICE_INTERFACE_DETAIL_DATA_W)
            };
            p.cbSize = mem::size_of_val(p) as _;
            if unsafe {
                DeviceAndDriverInstallation::SetupDiGetDeviceInterfaceDetailW(
                    info.0,
                    &d_data,
                    Some(p),
                    required_size,
                    Some(&mut required_size),
                    None,
                )
            }
            .is_err()
            {
                continue;
            }

            let path = PCWSTR::from_raw(ptr::addr_of!(p.DevicePath) as _);
            let r = unsafe {
                FileSystem::CreateFileW(
                    path,
                    (FileSystem::FILE_GENERIC_READ | FileSystem::FILE_GENERIC_WRITE).0,
                    FileSystem::FILE_SHARE_READ | FileSystem::FILE_SHARE_WRITE,
                    None,
                    FileSystem::OPEN_EXISTING,
                    FileSystem::FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            };
            let Ok(h) = r else {
                continue;
            };
            let h = unsafe { OwnedHandle::from_raw_handle(h.0) };

            let location = String::from_utf16_lossy(unsafe { path.as_wide() });
            let product_id = re_pid
                .captures(&location)
                .and_then(|caps| caps.get(1))
                .and_then(|m| u16::from_str_radix(m.as_str(), 16).ok())
                .unwrap_or_default();
            let vendor_id = re_vid
                .captures(&location)
                .and_then(|caps| caps.get(1))
                .and_then(|m| u16::from_str_radix(m.as_str(), 16).ok())
                .unwrap_or_default();

            let mut info = DeviceInfo {
                location,
                product_string: None,
                product_id,
                vendor_id,
                h,
            };

            let mut name_buffer = vec![0u16; 256];
            if unsafe {
                HumanInterfaceDevice::HidD_GetProductString(
                    Foundation::HANDLE(info.h.as_raw_handle()),
                    name_buffer.as_mut_ptr() as _,
                    name_buffer.len() as _,
                )
            }
            .as_bool()
            {
                let end = name_buffer
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(name_buffer.len());
                info.product_string = Some(String::from_utf16_lossy(&name_buffer[..end]));
            }

            info_list.push(info);
        }

        Ok(info_list)
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn product_string(&self) -> Option<&str> {
        self.product_string.as_deref()
    }

    pub fn product_id(&self) -> u16 {
        self.product_id
    }

    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub fn open(self) -> Result<Device, super::Error> {
        Ok(Device { parent: self })
    }
}

pub struct Device {
    parent: DeviceInfo,
}

impl Device {
    pub fn info(&self) -> &DeviceInfo {
        &self.parent
    }

    pub fn get_input_report(&self, buffer: &mut [u8]) -> Result<(), super::Error> {
        let r = unsafe {
            HumanInterfaceDevice::HidD_GetInputReport(
                Foundation::HANDLE(self.parent.h.as_raw_handle()),
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
            )
        }
        .as_bool();
        if !r {
            return Err(super::Error::WinError(windows::core::Error::from_win32()));
        }

        Ok(())
    }

    pub fn set_output_report(&self, buffer: &[u8]) -> Result<(), super::Error> {
        let r = unsafe {
            HumanInterfaceDevice::HidD_SetOutputReport(
                Foundation::HANDLE(self.parent.h.as_raw_handle()),
                buffer.as_ptr() as _,
                buffer.len() as _,
            )
        }
        .as_bool();
        if !r {
            return Err(super::Error::WinError(windows::core::Error::from_win32()));
        }

        Ok(())
    }
}

struct OwnedDeviceInfo(DeviceAndDriverInstallation::HDEVINFO);

impl Drop for OwnedDeviceInfo {
    fn drop(&mut self) {
        unsafe {
            self.0.free();
        }
    }
}
