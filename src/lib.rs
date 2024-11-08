#[cfg(target_os = "macos")]
pub use macos::DeviceInfo;

#[cfg(target_os = "macos")]
mod macos;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    IOReturn(io_kit_sys::ret::IOReturn),
}
