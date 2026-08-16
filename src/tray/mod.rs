#[cfg(not(target_os = "linux"))]
pub mod icon;
#[cfg(not(target_os = "linux"))]
pub mod windows;

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ShowWindow,
    ToggleMute,
    Quit,
}