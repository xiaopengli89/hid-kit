#[cfg(windows)]
pub use self::windows::DeviceInfo;
#[cfg(target_os = "macos")]
pub use macos::DeviceInfo;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(target_os = "macos")]
    #[error("{0}")]
    IOReturn(io_kit_sys::ret::IOReturn),
    #[cfg(windows)]
    #[error("{0}")]
    WinError(#[from] ::windows::core::Error),
}
