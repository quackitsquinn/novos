use pci_id_parse::{download_and_parse_pci_ids, Device, SubDevice, Vendor};
use proc_macro::TokenStream;

mod pci_id_parse;

pub fn generate_pci_ids() -> TokenStream {
    let vendors = download_and_parse_pci_ids();
    let vendors = vendors.iter().map(generate_vendor).collect::<Vec<_>>();

    quote::quote! {
        const VENDORS: &[Vendor] = &[#(#vendors),*];
    }
    .into()
}

fn generate_subdevice(subdevice: &SubDevice) -> proc_macro2::TokenStream {
    let subvendor = subdevice.subvendor;
    let id = subdevice.id;
    let name = &subdevice.name;
    quote::quote! {
        SubDevice::new(#subvendor, #id, #name)
    }
}

fn generate_device(device: &Device) -> proc_macro2::TokenStream {
    let id = device.id;
    let name = &device.name;
    let subdevices = device
        .subdevices
        .iter()
        .map(generate_subdevice)
        .collect::<Vec<_>>();
    quote::quote! {
        Device::new(#id, #name, &[#(#subdevices),*])
    }
}

fn generate_vendor(vendor: &Vendor) -> proc_macro2::TokenStream {
    let id = vendor.id;
    let name = &vendor.name;
    let devices = vendor
        .devices
        .iter()
        .map(generate_device)
        .collect::<Vec<_>>();
    quote::quote! {
        Vendor::new(#id, #name, &[#(#devices),*])
    }
}
