# KSerial: The serial communication library for Novos.

All structs defined in this document are implied to be defined with `#[repr(C)]` unless otherwise specified.

All commands and actions are expected to only work in emulated environments, unless you are able to physically connect to the host machine.

## Architecture

This system uses const-sized packets to communicate between devices. This is to ensure that KSerial can be easily used in no_std environments, even without dynamic memory allocation. The size of each packet is determined by the command that is sent.

The system can exist in two states: `Raw` and `Packet`.

The system will be in `Raw` mode by default. In this mode, the system will receive raw bytes from the serial port and write them to stdout. This mode is indented for bootstrapping the system before the serial communication system is fully initialized.

Raw mode is switched back into when the system receive any invalid bytes.

`Packet` mode is the mode where the system will receive packets from the serial port and process them. This mode is intended for normal operation.

### Switching into Packet Mode

To switch into `Packet` mode, the system must receive the following bytes:

```rust
pub const PACKET_MODE_ENTRY: [u8; 8] = b"KSP\0\0ENTER"
```

TODO: Add a challenge if 2-way comms are possible

### Packet Structure

```rust
pub struct Packet {
    // The command to execute. This field determines the size of the data field.
    pub command: u8,
    // TODO: Implement this? Maybe?
    pub checksum: u8,
    pub data: [u8; /* Size specified by the command */],
}
```

If 2-way communication is possible, the command will be echoed back to the sender, with the data field being the response. It is not guaranteed that the response will be one packet.

### Implementation

All packets will implement the `Pod` trait. This is to ensure that the packets can be safely transmuted to and from byte arrays.

## Commands

TODO: Determine if 2-way communication will even work because serial has always been weird with that.

### 0x00: SendString

Size: 4098 bytes

```rust
pub struct SendString {
    // The length of the string to send.
    pub len: u16,
    // The string to send.
    pub data: [u8; 4096],
}
```

TODO: If 2-way communication works consistently, 0x01 - 0x05 will be file manipulation commands. This should mirror native file manipulation commands to some extent. (e.g. the command would return a file descriptor that can be used to read(?) and write to the file)

### 0x06: CreateIncrementalFileChannel

Creates a new file channel that will be used with the `IncrementalFile` struct.

```rust
pub struct CreateIncrementalFileChannel {
    // The name of the channel. null-terminated.
    pub name: [u8; 16],
    // The template of the file name to create. 
    // All occurrences of `{{ID}}` will be replaced with the ID of the file.
    pub file_template: [u8; 32],
}
```

### 0x07: IncrementalFile

```rust
pub struct IncrementalFile {
    /// The name of the channel. null-terminated.
    pub name: [u8; 16],
    /// Is there more data to send?
    pub is_done: bool,
    /// The length of the data.
    /// TODO: Maybe make 0 indicate `is_done`?
    pub len: u16,
    /// The data itself.
    pub data: [u8; 4096],
}
```

### 0x08: CloseIncrementalFileChannel

Closes the file channel.

```rust
pub struct CloseIncrementalFileChannel {
    /// The name of the channel. null-terminated.
    pub name: [u8; 16],
}
```

### 0x09: Shutdown

Shuts down the device. 

```rust
pub struct Shutdown {
    /// Shutdown with a specific code.
    pub code: i32,
}
```
