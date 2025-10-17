//! This module defines the command IDs used in the KSerial protocol.
//!
//! Each command ID is associated with a specific packet type, including the response type.

/// The command ID for a string packet.
pub const STRING_PACKET_ID: u8 = 0x00;
/// The command ID for the `OpenFile` command.
pub const OPEN_FILE_ID: u8 = 0x01;
/// The command ID for the `WriteFile` command.
pub const WRITE_FILE_ID: u8 = 0x02;
/// The command ID for the `CloseFile` command.
pub const CLOSE_FILE_ID: u8 = 0x03;
// 4-5: reserved for future use
/// The command ID for creating an incremental file channel.
pub const CREATE_INCREMENTAL_FILE_CHANNEL_ID: u8 = 0x06;
/// The command ID for sending incremental file data.
pub const INCREMENTAL_FILE_ID: u8 = 0x07;
/// The command ID for closing an incremental file channel.
pub const CLOSE_INCREMENTAL_FILE_CHANNEL_ID: u8 = 0x08;
/// The command ID for shutting down the server.
pub const SHUTDOWN_ID: u8 = 0x09;
