//! An enum tree representing the PCI device class and subclass. This file is kinda gross, and manually done.

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] //
pub enum PciDeviceClass {
    Unknown,
    MassStorage(MassStorageId),
    NetworkController(NetworkControllerId),
    DisplayController(DisplayControllerId),
    MultimediaController(MultimediaControllerId),
    MemoryController(MemoryControllerId),
    BridgeController(BridgeControllerId),
    SimpleCommunicationController(SimpleCommunicationControllerId),
    BaseSystemPeripheral(BaseSystemPeripheralId),
    InputDeviceController(InputDeviceControllerId),
    DockingStation(DockingStationId),
    Processor(ProcessorId),
    SerialBusController(SerialBusControllerId),
    WirelessController(WirelessControllerId),
    IntelligentController,
    SatelliteCommunicationController(SatelliteCommunicationControllerId),
    EncryptionController(EncryptionControllerId),
    SignalProcessingController(SignalProcessingControllerId),
    ProcessingAccelerator,
    NonEssentialInstrumentation,
    CoProcessor,
    Unassigned(u8, u8),
}

impl PciDeviceClass {
    pub fn new(class: u8, subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match class {
            0x01 => Ok(Self::MassStorage(MassStorageId::new(subclass, prog_if)?)),
            0x02 => Ok(Self::NetworkController(NetworkControllerId::new(
                subclass, prog_if,
            )?)),
            0x03 => Ok(Self::DisplayController(DisplayControllerId::new(
                subclass, prog_if,
            )?)),
            0x04 => Ok(Self::MultimediaController(MultimediaControllerId::new(
                subclass, prog_if,
            )?)),
            0x05 => Ok(Self::MemoryController(MemoryControllerId::new(
                subclass, prog_if,
            )?)),
            0x06 => Ok(Self::BridgeController(BridgeControllerId::new(
                subclass, prog_if,
            )?)),
            0x07 => Ok(Self::SimpleCommunicationController(
                SimpleCommunicationControllerId::new(subclass, prog_if)?,
            )),
            0x08 => Ok(Self::BaseSystemPeripheral(BaseSystemPeripheralId::new(
                subclass, prog_if,
            )?)),
            0x09 => Ok(Self::InputDeviceController(InputDeviceControllerId::new(
                subclass, prog_if,
            )?)),
            0x0A => Ok(Self::DockingStation(DockingStationId::new(
                subclass, prog_if,
            )?)),
            0x0B => Ok(Self::Processor(ProcessorId::new(subclass, prog_if)?)),
            0x0C => Ok(Self::SerialBusController(SerialBusControllerId::new(
                subclass, prog_if,
            )?)),
            0x0D => Ok(Self::WirelessController(WirelessControllerId::new(
                subclass, prog_if,
            )?)),
            0x0E => Ok(Self::IntelligentController),
            0x0F => Ok(Self::SatelliteCommunicationController(
                SatelliteCommunicationControllerId::new(subclass, prog_if)?,
            )),
            0x10 => Ok(Self::EncryptionController(EncryptionControllerId::new(
                subclass, prog_if,
            )?)),
            0x11 => Ok(Self::SignalProcessingController(
                SignalProcessingControllerId::new(subclass, prog_if)?,
            )),
            0x12 => Ok(Self::ProcessingAccelerator),
            0x13 => Ok(Self::NonEssentialInstrumentation),
            0x40 => Ok(Self::CoProcessor),
            0xFF => Ok(Self::Unassigned(subclass, prog_if)),
            _ => Err(IdError(class)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MassStorageId {
    Scsi,
    Ide(u8),
    FloppyDisk,
    IpiBus,
    Raid,
    Ata(u8),
    Sata(u8),
    SerialScsi(u8),
    NonVolatileMemory(u8),
    Unknown,
}

impl MassStorageId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Scsi),
            0x01 => Ok(Self::Ide(prog_if)),
            0x02 => Ok(Self::FloppyDisk),
            0x03 => Ok(Self::IpiBus),
            0x04 => Ok(Self::Raid),
            0x05 => Ok(Self::Ata(prog_if)),
            0x06 => Ok(Self::Sata(prog_if)),
            0x07 => Ok(Self::SerialScsi(prog_if)),
            0x08 => Ok(Self::NonVolatileMemory(prog_if)),
            0x80 => Ok(Self::Unknown),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NetworkControllerId {
    Ethernet,
    TokenRing,
    Fddi,
    Atm,
    Isdn,
    WordFlip,
    Picmg24MutiComputerController,
    Infiniband,
    Fabric,
    Unknown,
}

impl NetworkControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Ethernet),
            0x01 => Ok(Self::TokenRing),
            0x02 => Ok(Self::Fddi),
            0x03 => Ok(Self::Atm),
            0x04 => Ok(Self::Isdn),
            0x05 => Ok(Self::WordFlip),
            0x06 => Ok(Self::Picmg24MutiComputerController),
            0x07 => Ok(Self::Infiniband),
            0x08 => Ok(Self::Fabric),
            0x80 => Ok(Self::Unknown),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayControllerId {
    VgaCompatible(u8),
    XgaCompatible,
    Controller3d,
    Other,
}

impl DisplayControllerId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::VgaCompatible(prog_if)),
            0x02 => Ok(Self::XgaCompatible),
            0x03 => Ok(Self::Controller3d),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MultimediaControllerId {
    Video,
    Audio,
    Telephone,
    AudioDevice,
    Other,
}

impl MultimediaControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Video),
            0x01 => Ok(Self::Audio),
            0x02 => Ok(Self::Telephone),
            0x03 => Ok(Self::AudioDevice),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryControllerId {
    Ram,
    Flash,
    Other,
}

impl MemoryControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Ram),
            0x01 => Ok(Self::Flash),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BridgeControllerId {
    HostBridge,
    IsaBridge,
    EisaBridge,
    McaBridge,
    PciPciBridge(u8),
    PcmciaBridge,
    NuBusBridge,
    CardbusBridge,
    RacewayBridge(u8),
    PciPciBridge2(u8),
    InfinibandToPciHostBridge,
    Other,
}

impl BridgeControllerId {
    pub fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::HostBridge),
            0x01 => Ok(Self::IsaBridge),
            0x02 => Ok(Self::EisaBridge),
            0x03 => Ok(Self::McaBridge),
            0x04 => Ok(Self::PciPciBridge(prog_if)),
            0x05 => Ok(Self::PcmciaBridge),
            0x06 => Ok(Self::NuBusBridge),
            0x07 => Ok(Self::CardbusBridge),
            0x08 => Ok(Self::RacewayBridge(prog_if)),
            0x09 => Ok(Self::PciPciBridge2(prog_if)),
            0x0A => Ok(Self::InfinibandToPciHostBridge),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SimpleCommunicationControllerId {
    Serial(u8),
    Parallel(u8),
    MultiportSerial,
    Modem(u8),
    Gpib,
    SmartCard,
    Other,
}

impl SimpleCommunicationControllerId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Serial(prog_if)),
            0x01 => Ok(Self::Parallel(prog_if)),
            0x02 => Ok(Self::MultiportSerial),
            0x03 => Ok(Self::Modem(prog_if)),
            0x04 => Ok(Self::Gpib),
            0x05 => Ok(Self::SmartCard),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BaseSystemPeripheralId {
    Pic(u8),
    Dma(u8),
    Timer(u8),
    Rtc(u8),
    PciHotPlug,
    SdHost,
    Iommu,
    Other,
}

impl BaseSystemPeripheralId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Pic(prog_if)),
            0x01 => Ok(Self::Dma(prog_if)),
            0x02 => Ok(Self::Timer(prog_if)),
            0x03 => Ok(Self::Rtc(prog_if)),
            0x04 => Ok(Self::PciHotPlug),
            0x05 => Ok(Self::SdHost),
            0x06 => Ok(Self::Iommu),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InputDeviceControllerId {
    Keyboard,
    DigitizerPen,
    Mouse,
    Scanner,
    GamePort(u8),
    Other,
}

impl InputDeviceControllerId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Keyboard),
            0x01 => Ok(Self::DigitizerPen),
            0x02 => Ok(Self::Mouse),
            0x03 => Ok(Self::Scanner),
            0x04 => Ok(Self::GamePort(prog_if)),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DockingStationId {
    Generic,
    Other,
}

impl DockingStationId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Generic),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessorId {
    Type386,
    Type486,
    Pentium,
    PentiumPro,
    Alpha,
    PowerPc,
    Mips,
    CoProcessor,
    Other,
}

impl ProcessorId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Type386),
            0x01 => Ok(Self::Type486),
            0x02 => Ok(Self::Pentium),
            0x03 => Ok(Self::PentiumPro),
            0x10 => Ok(Self::Alpha),
            0x20 => Ok(Self::PowerPc),
            0x30 => Ok(Self::Mips),
            0x40 => Ok(Self::CoProcessor),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SerialBusControllerId {
    FireWire(u8),
    AccessBus,
    Ssa,
    Usb(u8),
    Fibre,
    SmBus,
    Infiniband,
    Ipmi(u8),
    Sercos,
    CanBus,
    Other,
}

impl SerialBusControllerId {
    fn new(subclass: u8, prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::FireWire(prog_if)),
            0x01 => Ok(Self::AccessBus),
            0x02 => Ok(Self::Ssa),
            0x03 => Ok(Self::Usb(prog_if)),
            0x04 => Ok(Self::Fibre),
            0x05 => Ok(Self::SmBus),
            0x06 => Ok(Self::Infiniband),
            0x07 => Ok(Self::Ipmi(prog_if)),
            0x08 => Ok(Self::Sercos),
            0x09 => Ok(Self::CanBus),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WirelessControllerId {
    Irda,
    Ir,
    Rf,
    Bluetooth,
    Broadband,
    Ethernet1a,
    Ethernet1b,
    Other,
}

impl WirelessControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Irda),
            0x01 => Ok(Self::Ir),
            0x10 => Ok(Self::Rf),
            0x11 => Ok(Self::Bluetooth),
            0x12 => Ok(Self::Broadband),
            0x20 => Ok(Self::Ethernet1a),
            0x21 => Ok(Self::Ethernet1b),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SatelliteCommunicationControllerId {
    Tv,
    Audio,
    Voice,
    Data,
    Other,
}

impl SatelliteCommunicationControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Tv),
            0x01 => Ok(Self::Audio),
            0x02 => Ok(Self::Voice),
            0x03 => Ok(Self::Data),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EncryptionControllerId {
    NetworkComputingEncryptionDecryption,
    EntertainmentDrmEncryption,
    Other,
}

impl EncryptionControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::NetworkComputingEncryptionDecryption),
            0x10 => Ok(Self::EntertainmentDrmEncryption),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SignalProcessingControllerId {
    Dpio,
    PerformanceCounters,
    CommunicationSynchronizer,
    SignalProcessingManagement,
    Other,
}

impl SignalProcessingControllerId {
    fn new(subclass: u8, _prog_if: u8) -> Result<Self, IdError> {
        match subclass {
            0x00 => Ok(Self::Dpio),
            0x01 => Ok(Self::PerformanceCounters),
            0x10 => Ok(Self::CommunicationSynchronizer),
            0x20 => Ok(Self::SignalProcessingManagement),
            0x80 => Ok(Self::Other),
            _ => Err(IdError(subclass)),
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Invalid PCI ID")]
pub struct IdError(u8);
