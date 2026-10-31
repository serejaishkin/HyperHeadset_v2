//! Airoha RACE over USB HID, as used by NGenuity's `AirohaHidCoreLib.dll`
//! (Cloud III S / Mix 2 / Mix Buds 2 / II Core class headsets).
//!
//! Findings from disassembling AirohaHidCoreLib.dll (SDK 3.10.1.5):
//! * Transport: HID **report ID 0x05**; the packet is written as an output report, zero-padded to
//!   `OutputReportByteLength` (no extra report-ID byte is added: the 0x05 *is* the report ID).
//! * Frame: `05 | type | len16 LE | cmd16 LE | payload`, `len = 2 + payload.len()`.
//!   Request type 0x5A, response 0x5B, indication 0x5D.
//! * "Feature" commands: cmd 0x2C83 = GET, 0x2C82 = SET. Payload = `feature16 LE [| value...]`.
//!   Response to GET: `05 5B len 83 2C | status | feature16 | value`; to SET: same without value.
//! * Battery: cmd 0x0CD6, payload `role` (0 = agent). Response (type 0x5D):
//!   `05 5D 05 00 D6 0C | status | role | level%`.
//! * Relay (dongle -> headset) wraps a normal packet in cmd 0x0D01.
//!
//! Feature IDs recovered as constants: sidetone level 6, sidetone state 7, advanced passthrough 8,
//! voice prompts 9, game/chat mix ratio 0x0B, game mic volume 0x0C. Auto-power-off / auto
//! play-pause / touch take their feature id from a per-device table (not a constant in the DLL).

pub const REPORT_ID: u8 = 0x05;
pub const TYPE_REQUEST: u8 = 0x5A;
pub const TYPE_RESPONSE: u8 = 0x5B;
pub const TYPE_INDICATION: u8 = 0x5D;

pub const CMD_GET_FEATURE: u16 = 0x2C83;
pub const CMD_SET_FEATURE: u16 = 0x2C82;
pub const CMD_GET_BATTERY: u16 = 0x0CD6;
pub const CMD_GET_MMI_STATE: u16 = 0x2C80;
pub const CMD_RELAY: u16 = 0x0D01;

pub const FEATURE_SIDETONE_LEVEL: u16 = 6;
pub const FEATURE_SIDETONE_STATE: u16 = 7;
pub const FEATURE_ADV_PASSTHROUGH: u16 = 8;
pub const FEATURE_VOICE_PROMPTS: u16 = 9;
pub const FEATURE_GAME_CHAT_MIX: u16 = 0x0B;
pub const FEATURE_GAME_MIC_VOLUME: u16 = 0x0C;

pub fn race_packet(cmd: u16, payload: &[u8]) -> Vec<u8> {
    let len = 2u16 + payload.len() as u16;
    let mut p = Vec::with_capacity(6 + payload.len());
    p.push(REPORT_ID);
    p.push(TYPE_REQUEST);
    p.extend_from_slice(&len.to_le_bytes());
    p.extend_from_slice(&cmd.to_le_bytes());
    p.extend_from_slice(payload);
    p
}

/// Zero-pad to the HID output report length.
pub fn pad(mut packet: Vec<u8>, report_len: usize) -> Vec<u8> {
    if packet.len() < report_len {
        packet.resize(report_len, 0);
    }
    packet
}

pub fn get_feature(feature: u16) -> Vec<u8> {
    race_packet(CMD_GET_FEATURE, &feature.to_le_bytes())
}

pub fn set_feature(feature: u16, value: &[u8]) -> Vec<u8> {
    let mut payload = feature.to_le_bytes().to_vec();
    payload.extend_from_slice(value);
    race_packet(CMD_SET_FEATURE, &payload)
}

pub fn get_battery(role: u8) -> Vec<u8> {
    race_packet(CMD_GET_BATTERY, &[role])
}

pub fn relay(inner: &[u8], recipient: u8, flags: u8) -> Vec<u8> {
    let mut payload = vec![recipient, flags];
    payload.extend_from_slice(inner);
    race_packet(CMD_RELAY, &payload)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// GET feature reply: feature id and raw value bytes.
    Feature { feature: u16, value: Vec<u8> },
    /// SET feature acknowledgement.
    FeatureAck { feature: u16 },
    Battery { role: u8, level: u8 },
    Other { kind: u8, cmd: u16, status: u8, data: Vec<u8> },
}

/// Parse an incoming report (must start with report ID 0x05). Returns None for non-RACE data
/// or non-zero status.
pub fn parse_response(buf: &[u8]) -> Option<Response> {
    if buf.len() < 7 || buf[0] != REPORT_ID {
        return None;
    }
    let kind = buf[1];
    if kind != TYPE_RESPONSE && kind != TYPE_INDICATION {
        return None;
    }
    let len = u16::from_le_bytes([buf[2], buf[3]]) as usize;
    let end = (4 + len).min(buf.len());
    if end < 7 {
        return None;
    }
    let cmd = u16::from_le_bytes([buf[4], buf[5]]);
    let status = buf[6];
    let data = &buf[7..end];
    if status != 0 {
        return Some(Response::Other { kind, cmd, status, data: data.to_vec() });
    }
    match cmd {
        CMD_GET_BATTERY if data.len() >= 2 => Some(Response::Battery { role: data[0], level: data[1].min(100) }),
        CMD_GET_FEATURE if data.len() >= 2 => Some(Response::Feature {
            feature: u16::from_le_bytes([data[0], data[1]]),
            value: data[2..].to_vec(),
        }),
        CMD_SET_FEATURE if data.len() >= 2 => Some(Response::FeatureAck {
            feature: u16::from_le_bytes([data[0], data[1]]),
        }),
        _ => Some(Response::Other { kind, cmd, status, data: data.to_vec() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidetone_get_set() {
        assert_eq!(get_feature(FEATURE_SIDETONE_STATE), [0x05, 0x5A, 0x04, 0x00, 0x83, 0x2C, 0x07, 0x00]);
        assert_eq!(set_feature(FEATURE_SIDETONE_STATE, &[1]), [0x05, 0x5A, 0x05, 0x00, 0x82, 0x2C, 0x07, 0x00, 0x01]);
    }

    #[test]
    fn battery_roundtrip() {
        assert_eq!(get_battery(0), [0x05, 0x5A, 0x03, 0x00, 0xD6, 0x0C, 0x00]);
        let r = [0x05, 0x5D, 0x05, 0x00, 0xD6, 0x0C, 0x00, 0x00, 0x57];
        assert_eq!(parse_response(&r), Some(Response::Battery { role: 0, level: 0x57 }));
    }

    #[test]
    fn feature_reply() {
        let r = [0x05, 0x5B, 0x06, 0x00, 0x83, 0x2C, 0x00, 0x07, 0x00, 0x01];
        assert_eq!(parse_response(&r), Some(Response::Feature { feature: 7, value: vec![1] }));
    }
}
