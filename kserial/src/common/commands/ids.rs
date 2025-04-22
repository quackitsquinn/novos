//! This module defines the command IDs used in the KSerial protocol.
//!
//! Each command ID is associated with a specific packet type, including the response type.

pub const STRING_PACKET_ID: u8 = 0x00;
pub const OPEN_FILE_ID: u8 = 0x01;
pub const WRITE_FILE_ID: u8 = 0x02;
pub const CLOSE_FILE_ID: u8 = 0x03;
// 4-5: reserved for future use
pub const CREATE_INCREMENTAL_FILE_CHANNEL_ID: u8 = 0x06;
pub const INCREMENTAL_FILE_ID: u8 = 0x07;
pub const CLOSE_INCREMENTAL_FILE_CHANNEL_ID: u8 = 0x08;

pub const SHUTDOWN_ID: u8 = 0x09;
