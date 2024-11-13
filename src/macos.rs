use core_foundation::{array, base, base::TCFType, dictionary, number, set, string};
use io_kit_sys::{
    hid,
    hid::{device, keys, manager},
    ret,
};
use std::{ffi, ptr};

pub struct DeviceInfo {
    location: String,
    product_string: Option<String>,
    elements: Vec<Element>,
    _m: CFObjectMut<manager::__IOHIDManager>,
}

impl DeviceInfo {
    pub fn enumerate() -> Result<Vec<Self>, super::Error> {
        let m = unsafe {
            manager::IOHIDManagerCreate(base::kCFAllocatorDefault, manager::kIOHIDManagerOptionNone)
        };
        if m.is_null() {
            return Err(super::Error::Unknown);
        }
        let m = CFObjectMut(m);

        let r = unsafe { manager::IOHIDManagerOpen(m.0, manager::kIOHIDManagerOptionNone) };
        if r != ret::kIOReturnSuccess {
            return Err(super::Error::IOReturn(r));
        }

        unsafe { manager::IOHIDManagerSetDeviceMatching(m.0, ptr::null()) };

        let set = unsafe { manager::IOHIDManagerCopyDevices(m.0) };
        if set.is_null() {
            return Err(super::Error::Unknown);
        }
        let set: set::CFSet<hid::base::IOHIDDeviceRef> =
            unsafe { set::CFSet::wrap_under_create_rule(set) };

        extern "C" fn collect(value: *const ffi::c_void, context: *const ffi::c_void) {
            let ctx = unsafe {
                &mut *(context as *mut (Vec<DeviceInfo>, CFObjectMut<manager::__IOHIDManager>))
            };

            let raw = CFObjectMut(value as hid::base::IOHIDDeviceRef);
            let loc = {
                let prop: number::CFNumberRef = unsafe {
                    device::IOHIDDeviceGetProperty(
                        raw.0,
                        io_kit_sys::CFSTR(keys::kIOHIDLocationIDKey),
                    )
                } as _;
                if prop.is_null() {
                    return;
                }
                let prop = unsafe { number::CFNumber::wrap_under_get_rule(prop) };
                let Some(prop) = prop.to_i32() else {
                    return;
                };
                prop.to_string()
            };

            let mut usages = vec![];
            {
                let prop: array::CFArrayRef = unsafe {
                    device::IOHIDDeviceGetProperty(
                        raw.0,
                        io_kit_sys::CFSTR(keys::kIOHIDDeviceUsagePairsKey),
                    )
                } as _;
                if !prop.is_null() {
                    let len = unsafe { array::CFArrayGetCount(prop) };
                    for i in 0..len {
                        let v: dictionary::CFDictionaryRef =
                            unsafe { array::CFArrayGetValueAtIndex(prop, i) } as _;

                        let usage: number::CFNumberRef = unsafe {
                            dictionary::CFDictionaryGetValue(
                                v,
                                io_kit_sys::CFSTR(keys::kIOHIDDeviceUsageKey) as _,
                            )
                        } as _;
                        let Some(usage) =
                            unsafe { number::CFNumber::wrap_under_get_rule(usage) }.to_i32()
                        else {
                            continue;
                        };

                        let usage_page: number::CFNumberRef = unsafe {
                            dictionary::CFDictionaryGetValue(
                                v,
                                io_kit_sys::CFSTR(keys::kIOHIDDeviceUsagePageKey) as _,
                            )
                        } as _;
                        let Some(usage_page) =
                            unsafe { number::CFNumber::wrap_under_get_rule(usage_page) }.to_i32()
                        else {
                            continue;
                        };

                        usages.push((usage, usage_page));
                    }
                }
            }

            let ele = Element { usages, raw };

            if let Some(info) = ctx.0.iter_mut().find(|info| info.location == loc) {
                info.elements.push(ele);
            } else {
                let mut info = DeviceInfo {
                    location: loc,
                    product_string: None,
                    elements: vec![ele],
                    _m: ctx.1.clone(),
                };
                let product_string: string::CFStringRef = unsafe {
                    device::IOHIDDeviceGetProperty(
                        info.elements[0].raw.0,
                        io_kit_sys::CFSTR(keys::kIOHIDProductKey),
                    )
                } as _;
                if !product_string.is_null() {
                    let product_string =
                        unsafe { string::CFString::wrap_under_get_rule(product_string) };
                    info.product_string = Some(product_string.to_string());
                }
                ctx.0.push(info);
            }
        }
        let mut ctx: (Vec<Self>, _) = (vec![], m);
        unsafe {
            set::CFSetApplyFunction(
                set.as_concrete_TypeRef(),
                collect,
                &mut ctx as *const _ as _,
            )
        };
        Ok(ctx.0)
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn product_string(&self) -> Option<&str> {
        self.product_string.as_deref()
    }

    pub fn product_id(&self) -> u16 {
        let pid: number::CFNumberRef = unsafe {
            device::IOHIDDeviceGetProperty(
                self.elements[0].raw.0,
                io_kit_sys::CFSTR(keys::kIOHIDProductIDKey),
            )
        } as _;
        if pid.is_null() {
            return 0;
        }
        let pid = unsafe { number::CFNumber::wrap_under_get_rule(pid) };
        let Some(pid) = pid.to_i32() else {
            return 0;
        };
        pid as _
    }

    pub fn vendor_id(&self) -> u16 {
        let vid: number::CFNumberRef = unsafe {
            device::IOHIDDeviceGetProperty(
                self.elements[0].raw.0,
                io_kit_sys::CFSTR(keys::kIOHIDVendorIDKey),
            )
        } as _;
        if vid.is_null() {
            return 0;
        }
        let vid = unsafe { number::CFNumber::wrap_under_get_rule(vid) };
        let Some(vid) = vid.to_i32() else {
            return 0;
        };
        vid as _
    }

    pub fn usages(&self) -> impl Iterator<Item = &(i32, i32)> {
        self.elements.iter().flat_map(|ele| ele.usages.iter())
    }

    pub fn open(self) -> Result<Device, super::Error> {
        let r =
            unsafe { device::IOHIDDeviceOpen(self.elements[0].raw.0, keys::kIOHIDOptionsTypeNone) };
        if r != ret::kIOReturnSuccess {
            return Err(super::Error::IOReturn(r));
        }
        Ok(Device { parent: self })
    }
}

pub struct Device {
    parent: DeviceInfo,
}

impl Drop for Device {
    fn drop(&mut self) {
        let _r = unsafe {
            device::IOHIDDeviceClose(self.parent.elements[0].raw.0, keys::kIOHIDOptionsTypeNone)
        };
    }
}

impl Device {
    pub fn info(&self) -> &DeviceInfo {
        &self.parent
    }

    pub fn get_input_report(&self, buffer: &mut [u8]) -> Result<(), super::Error> {
        let report_id = buffer.get(0).copied().unwrap_or_default();
        let mut len = buffer.len() as isize;

        let r = unsafe {
            device::IOHIDDeviceGetReport(
                self.parent.elements[0].raw.0,
                keys::kIOHIDReportTypeInput,
                report_id as _,
                buffer.as_mut_ptr(),
                &mut len,
            )
        };
        if r != ret::kIOReturnSuccess {
            return Err(super::Error::IOReturn(r));
        }
        Ok(())
    }
}

struct Element {
    usages: Vec<(i32, i32)>,
    raw: CFObjectMut<hid::base::__IOHIDDevice>,
}

struct CFObject<T>(*const T);

unsafe impl<T> Send for CFObject<T> {}

unsafe impl<T> Sync for CFObject<T> {}

impl<T> Drop for CFObject<T> {
    fn drop(&mut self) {
        unsafe {
            base::CFRelease(self.0 as _);
        };
    }
}

impl<T> Clone for CFObject<T> {
    fn clone(&self) -> Self {
        unsafe { base::CFRetain(self.0 as _) };
        Self(self.0)
    }
}

struct CFObjectMut<T>(*mut T);

unsafe impl<T> Send for CFObjectMut<T> {}

unsafe impl<T> Sync for CFObjectMut<T> {}

impl<T> Drop for CFObjectMut<T> {
    fn drop(&mut self) {
        unsafe {
            base::CFRelease(self.0 as _);
        };
    }
}

impl<T> Clone for CFObjectMut<T> {
    fn clone(&self) -> Self {
        unsafe { base::CFRetain(self.0 as _) };
        Self(self.0)
    }
}
