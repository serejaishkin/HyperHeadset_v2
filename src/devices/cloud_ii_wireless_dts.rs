use crate::{
    debug_println,
    devices::{ChargingStatus, Color, Device, DeviceEvent, DeviceState},
};
use std::time::Duration;

const HP: u16 = 0x03F0;
pub const VENDOR_IDS: [u16; 1] = [HP];
// Possible Cloud II Wireless product IDs.
// 018B/0696 = dongles (HID MI_03 Col03), 058B/038B = headset side (HID Col02),
// per NGenuity2Helper HeadsetCNXTAvneraUpdater / HeadsetCloudIIWirelessDTS.
pub const PRODUCT_IDS: [u16; 6] = [0x1718, 0x018B, 0x0D93, 0x0696, 0x058B, 0x038B];

const BASE_PACKET: [u8; 20] = {
    let mut packet = [0; 20];
    (packet[0], packet[1], packet[2]) = (0x06, 0xff, 0xbb);
    packet
};

// "Extended" command header used by NGenuity for voice prompt / product colour:
// buf[0]=0x21, buf[1]=0xBB, buf[2]=cmd, buf[3]=payload
const BASE_PACKET2: [u8; 20] = {
    let mut packet = [0; 20];
    (packet[0], packet[1]) = (33, 187);
    packet
};

const RESPONSE_BUFFER_SIZE: usize = 256;

const GET_CHARGING_CMD_ID: u8 = 3;
const GET_MIC_CONNECTED_CMD_ID: u8 = 8;
const GET_BATTERY_CMD_ID: u8 = 2;
const GET_AUTO_SHUTDOWN_CMD_ID: u8 = 7;
const SET_AUTO_SHUTDOWN_CMD_ID: u8 = 34;
const GET_MUTE_CMD_ID: u8 = 5;
const SET_MUTE_CMD_ID: u8 = 32;
const GET_PAIRING_CMD_ID: u8 = 9;
const GET_PRODUCT_COLOR_CMD_ID: u8 = 14;
const GET_SIDE_TONE_ON_CMD_ID: u8 = 6;
const SET_SIDE_TONE_ON_CMD_ID: u8 = 33;
const GET_SIDE_TONE_VOLUME_CMD_ID: u8 = 11;
const SET_SIDE_TONE_VOLUME_CMD_ID: u8 = 35;
const GET_VOICE_PROMPT_CMD_ID: u8 = 9;
const SET_VOICE_PROMPT_CMD_ID: u8 = 19;
const GET_WIRELESS_STATUS_CMD_ID: u8 = 1;

pub struct CloudIIWirelessDTS {
    state: DeviceState,
}

impl CloudIIWirelessDTS {
    pub fn new_from_state(state: DeviceState) -> Self {
        let mut state = state;
        state.device_properties.connected = Some(true);
        CloudIIWirelessDTS { state }
    }
}

impl Device for CloudIIWirelessDTS {
    fn get_response_buffer(&self) -> Vec<u8> {
        let mut tmp = [0u8; RESPONSE_BUFFER_SIZE].to_vec();
        tmp[0] = 33;
        tmp
    }

    fn get_charging_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_CHARGING_CMD_ID;
        Some(tmp)
    }

    fn get_battery_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_BATTERY_CMD_ID;
        Some(tmp)
    }

    fn set_automatic_shut_down_packet(&self, shutdown_after: Duration) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = SET_AUTO_SHUTDOWN_CMD_ID;
        tmp[4] = (shutdown_after.as_secs() / 60) as u8;
        Some(tmp)
    }

    fn get_automatic_shut_down_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_AUTO_SHUTDOWN_CMD_ID;
        Some(tmp)
    }

    fn get_mute_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_MUTE_CMD_ID;
        Some(tmp)
    }

    fn set_mute_packet(&self, mute: bool) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = SET_MUTE_CMD_ID;
        tmp[4] = mute as u8;
        Some(tmp)
    }

    fn get_surround_sound_packet(&self) -> Option<Vec<u8>> {
        None
    }

    fn set_surround_sound_packet(&self, _surround_sound: bool) -> Option<Vec<u8>> {
        None
    }

    fn get_mic_connected_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_MIC_CONNECTED_CMD_ID;
        Some(tmp)
    }

    fn get_pairing_info_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_PAIRING_CMD_ID;
        Some(tmp)
    }

    fn get_product_color_packet(&self) -> Option<Vec<u8>> {
        // NGenuity: [0]=0x21 [1]=0xBB [2]=0x0E. Earlier "doesn't work" was most likely the
        // response parser dropping 0x21/0xBB-headed replies (see get_event_from_device_response).
        let mut tmp = BASE_PACKET2.to_vec();
        tmp[2] = GET_PRODUCT_COLOR_CMD_ID;
        Some(tmp)
    }

    fn get_side_tone_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_SIDE_TONE_ON_CMD_ID;
        Some(tmp)
    }

    fn set_side_tone_packet(&self, side_tone_on: bool) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = SET_SIDE_TONE_ON_CMD_ID;
        tmp[4] = side_tone_on as u8;
        Some(tmp)
    }

    fn get_side_tone_volume_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = GET_SIDE_TONE_VOLUME_CMD_ID;
        Some(tmp)
    }

    fn set_side_tone_volume_packet(&self, volume: u8) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET.to_vec();
        tmp[3] = SET_SIDE_TONE_VOLUME_CMD_ID;
        tmp[4] = volume;
        Some(tmp)
    }

    fn get_voice_prompt_packet(&self) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET2.to_vec();
        tmp[2] = GET_VOICE_PROMPT_CMD_ID;
        Some(tmp)
    }

    fn set_voice_prompt_packet(&self, enable: bool) -> Option<Vec<u8>> {
        let mut tmp = BASE_PACKET2.to_vec();
        tmp[2] = SET_VOICE_PROMPT_CMD_ID;
        tmp[3] = enable as u8;
        Some(tmp)
    }

    fn get_wireless_connected_status_packet(&self) -> Option<Vec<u8>> {
        // works but causes state to reset e.g. unmutes the headset
        // let mut tmp = BASE_PACKET.to_vec();
        // tmp[3] = GET_WIRELESS_STATUS_CMD_ID;
        // Some(tmp)
        None
    }

    fn get_sirk_packet(&self) -> Option<Vec<u8>> {
        None
    }

    fn reset_sirk_packet(&self) -> Option<Vec<u8>> {
        None
    }

    fn get_silent_mode_packet(&self) -> Option<Vec<u8>> {
        None
    }

    fn set_silent_mode_packet(&self, _silence: bool) -> Option<Vec<u8>> {
        None
    }

    fn get_event_from_device_response(&self, response: &[u8]) -> Option<Vec<DeviceEvent>> {
        debug_println!("Read packet: {:?}", response);
        if response.len() < 7 {
            return None;
        }
        // Extended replies: 21 BB <cmd> <status>
        if response[0] == 0x21 && response[1] == 0xBB {
            return match (response[2], response[3]) {
                (GET_VOICE_PROMPT_CMD_ID, status) => {
                    Some(vec![DeviceEvent::VoicePrompt(status == 1)])
                }
                (GET_PRODUCT_COLOR_CMD_ID, status) => {
                    Some(vec![DeviceEvent::ProductColor(Color::from(status))])
                }
                _ => {
                    debug_println!("Unknown extended device event: {:?}", response);
                    None
                }
            };
        }
        // Standard replies: 06 FF BB <cmd> <data...>
        if response[0] != 6 || response[1] != 255 || response[2] != 187 {
            return None;
        }
        match (response[2], response[3], response[4], response[7]) {
            (_, GET_CHARGING_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::Charging(ChargingStatus::from(status))])
            }
            (_, GET_MIC_CONNECTED_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::MicConnected(status == 1)])
            }
            (_, GET_BATTERY_CMD_ID, _, level) => Some(vec![DeviceEvent::BatterLevel(level)]),
            (_, GET_AUTO_SHUTDOWN_CMD_ID, time, _) => {
                Some(vec![DeviceEvent::AutomaticShutdownAfter(
                    Duration::from_secs(time as u64 * 60),
                )])
            }
            (_, SET_MUTE_CMD_ID, status, _) | (_, GET_MUTE_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::Muted(status == 1)])
            }
            (_, GET_PAIRING_CMD_ID, status, _) => Some(vec![DeviceEvent::PairingInfo(status)]),
            (_, GET_SIDE_TONE_ON_CMD_ID, status, _) | (_, SET_SIDE_TONE_ON_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::SideToneOn(status == 1)])
            }
            (_, GET_SIDE_TONE_VOLUME_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::SideToneVolume(status)])
            }
            (_, GET_WIRELESS_STATUS_CMD_ID, status, _) => {
                Some(vec![DeviceEvent::WirelessConnected(
                    status == 1 || status == 4,
                )])
            }
            _ => {
                debug_println!("Unknown device event: {:?}", response);
                None
            }
        }
    }

    fn allow_passive_refresh(&mut self) -> bool {
        true
    }

    fn get_device_state(&self) -> &DeviceState {
        &self.state
    }

    fn get_device_state_mut(&mut self) -> &mut DeviceState {
        &mut self.state
    }
}
