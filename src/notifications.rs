use notify_rust::Notification;

pub fn notify_low_battery(percent: u8) {
    let _ = Notification::new()
        .summary("🔋 HyperX Headset — Low Battery")
        .body(&format!("Battery level is {}%. Please charge your headset.", percent))
        .icon("battery-caution")
        .timeout(notify_rust::Timeout::Never)
        .show();
}

pub fn notify_full_charge() {
    let _ = Notification::new()
        .summary("🔋 HyperX Headset — Fully Charged")
        .body("Your headset is fully charged.")
        .icon("battery-full")
        .timeout(notify_rust::Timeout::Never)
        .show();
}
