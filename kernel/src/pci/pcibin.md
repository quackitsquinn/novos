# PCIbin: pci.ids in a easy to parse binary format

pci.bin is intended to be embedded in a kernel binary so that offsets can be converted to addr(pci.bin) + offset.

## Format

### Header

`checksum: u32`: Checksum of the file. XOR all of the bytes in u32 chunks should equal zero.
`str_off: u32`: Offset to the start of the strings.

### Vendors

`vendors: [VendorDescriptor; nul]`: At the start of the file. Index is the PCI ID. Value is the offset to that PCI ID's entry. offset == 0 means no entry.

### VendorDescriptor

`id: u16`: PCI ID.
`offset: u32`: Offset to the vendor's entry.


### Vendor

`name_off: u32`: Offset to the vendor's name.
`devices: [Device];nul` NUL terminated list of devices.

### Device
`name_off: u32`: Offset to the device's name.
`sublen: u16`: Subdevice count.`
`subdevices: [Subdevice; sublen]`: List of subdevices.

### Subdevice
`name_off: u32`: Offset to the subdevice's name.
`subvendor: u16`: Subvendor ID.

### Strings

`strings: [u8]`: NUL terminated strings. Strings are UTF-8 encoded.


