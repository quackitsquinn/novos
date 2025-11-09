# Specification for Kernel Independent Driver Interface (KIDI) (DRAFT)

## Introduction

KIDI (ipa /kaɪ daɪ/) stands for Kernel Independent Driver Interface. It is a proposed standard for writing drivers that can be used in any kernel. The goal of KIDI is to create a standard API for drivers that can be implemented in any kernel, allowing drivers to be written once and used in multiple kernels without modification. This specification is aimed at creating a common interface that any kernel of any architecture can implement, allowing for greater compatibility and ease of driver development.

This specification is aimed at reducing what has historically been a massive pain point in using an alternate operating system/kernel: the lack of driver support. By creating a standard for writing drivers that can be used in any kernel, we can help to alleviate this problem and make it easier for users to switch between different kernels without worrying about driver compatibility.


## Goals

Right now I have a few goals for this standard:
1. Create a solution to the current problem of cross kernel driver compatibility.
    - This is the main goal of the standard. See Nvidia/AMD drivers for an example of the current problem. Lacking documentation and support for multiple kernels mean that drivers are often only made for one kernel and often take the span of many years if there is no manufacturer support.
2. Create a standard that is adaptable to many different styles of kernels but with a standard API.
    - This is important because there are many different styles of kernels. Monolithic, Microkernel, Hybrid, Real Time, etc. The standard should be able to adapt to all of these styles.
3. Maintaining simplicity versus complexity.
    - The standard should be simple enough for small drivers but complex enough for large drivers. This is a balancing act and will require careful consideration.
4. Linux translation layer compatibility. (Not really part of the standard but a goal)
    - This is a big one. If we can write a compatibility layer that can translate Linux driver calls to KIDI calls, we can leverage the massive amount of existing Linux drivers. This will be a big task but it is worth it and will help with adoption of the standard.
5. Adoption!
    - This is a very very important and very very *very* long term goal. The standard is useless if no one uses it. Adoption will require outreach to kernel developers and driver developers. This will be a long term goal and will require patience and persistence.

I am planning on reaching out to kernel developers and driver developers once I have a more solid draft of the standard. If you are interested in helping with this project or have any feedback, please reach out to me on GitHub or via email.

## Design

KIDI will be split into multiple modules, each module will handle a specific aspect of driver development. Modules will contain functions, structs, and constants that are relevant to that module. Modules will be somewhat specialized for specific types of drivers but will also contain general purpose functions that can be used by any driver.

The main modules will be:
1. Core Module
    - This module will handle the basic functionality of the driver that almost all drivers will need. This includes device initialization, memory management, and I/O operations.
2. USB Module
    - This module will handle USB specific functionality. Contents TBD.
3. PCI Module
    - This module will handle PCI specific functionality. Contents TBD.
4. Network Module
    - This module will handle network specific functionality. Contents TBD.
5. Storage Module
    - This module will handle storage specific functionality. Contents TBD.
6. GPU Module
    - This module will handle GPU specific functionality. Contents TBD.

## Functions

There is currently not a defined set of functions for KIDI because it's so early in development. However, here are some ideas for functions that could be included in the standard:


Functions are currently written in rust style pseudocode. All functions can be assumed to be `extern "C"` unless otherwise specified.

## Error Handling / Management

Each module will define it's own error types, but all modules will use the same system for handling errors.

This is a part of the core module, but is important enough to be highlighted on its own.

TODO: More elegant function params and return types.

### Catastrophic Errors

The parameter `include_last_err: bool` indicates whether the last error code should be included in the panic/halt/init failed message. If true, the last error code will be retrieved using `kidi_last_error()` and included in the message. If false, no error code will be included.

- `fn kidi_kernel_panic(message: *const char, include_last_err: bool) -> !`
    - This function is called when the driver encounters a fatal error that it cannot recover from. This function will cause the kernel to panic and halt execution.
    - This should only be used in extreme cases where data corruption or other serious issues may occur.
- `fn kidi_driver_halt(message: *const char, include_last_err: bool) -> !`
    - This function is called when the driver encounters a non-fatal error that it cannot recover from. This function will halt the driver and prevent it from continuing execution.
    - This should be used when the driver cannot continue to function properly but the kernel can continue to run.
    - This should be preferred over `kidi_kernel_panic` whenever possible.
    - Ideally the kernel should forward a message to the user indicating that the driver has halted.
- `fn kidi_init_failed(message: *const char, include_last_err: bool) -> !`
    - This function is called when the driver fails to initialize. This function will return an error code to the kernel indicating that the driver failed to initialize.
    - This should be used when the driver cannot initialize properly due to missing resources or other issues.
    - The kernel should instantly halt the driver and prevent it from continuing execution.

### Non-Catastrophic Errors

- `type KidiError = u32`
    - A type alias for error codes used in KIDI. The lower 16 bits is the error code, the upper 16 bits is the module ID.
- `fn kidi_last_error() -> KidiError`
    - This function returns the last error code that occurred in the driver. This function can be used to retrieve the error code after a function call that returns an error.
    - Ideal for working decently in most languages.
- `fn kidi_get_error_message(error: *mut KidiError) -> *const char`
    - This function returns a human readable error message for the given error code. This function can be used to retrieve a string that describes the error.
    - Ideal for logging and debugging purposes.


## Core Module

### Error Types


```rust
#[repr(C, u16)]
enum KidiCoreError {

}
```

### Device Initialization

- `fn kidi_init_driver(/* PARAMS TODO */) -> /* RETURN TYPE TODO */`
    - Initializes the driver. This function is called when the driver is loaded.
- `fn kidi_register_device(/* PARAMS TODO */) -> KidiResult`
    - Registers a device with the kernel. This function should be called for each device that the driver supports.
- `fn kidi_unregister_device(/* PARAMS TODO */) -> KidiResult`
    - Unregister the device from the kernel. Should only be used for hotplug capable devices.
- `fn kidi_request_module(/* PARAMS TODO */) -> KidiResult`
    - Requests a kernel module to be loaded. This function is called when the driver needs a specific kernel module to be loaded.
    - TODO: Should this only be permitted in `kidi_init_driver`?

### Memory Management

- `fn kidi_malloc(size: usize, align: usize) -> *mut u8`
    - Allocates memory for the driver to use.
    - `size`: The size of the memory to allocate in bytes.
    - `align`: The alignment of the memory to allocate in bytes.
    - Returns a pointer to the allocated memory. The pointer may be null if the allocation failed.
- `fn kidi_calloc(num: usize, size: usize, align: usize) -> *mut u8`
    - Allocates zeroed memory for the driver to use.
    - `num`: The number of elements to allocate.
    - `size`: The size of each element in bytes.
    - `align`: The alignment of the memory to allocate in bytes.
    - Returns a pointer to the allocated memory. The pointer may be null if the allocation failed.
- `fn kidi_free(ptr: *mut u8)`
    - Frees memory that was allocated by the driver.
- `fn kidi_realloc(ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8`
    - Reallocates memory that was allocated by the driver.

### PCI

- `fn kidi_pci_enumerate_devices(/* PARAMS TODO */) -> KidiResult`
    - Enumerates all PCI devices on the system.
    - This function will skip devices that are already registered by other drivers.


## Required Functions

KIDI drivers must implement a set of required functions in order to be compatible with the standard. These functions can be detected in 3 ways:

1. Having a symbol export with the exact name of the function.
2. Having a specific section in the binary that contains a table of function pointers to the required functions.
3. A `KIDI_INTERFACE` static variable that contains a struct with function pointers to the required functions. This variable must be exported with the exact name `KIDI_INTERFACE`.

Each driver must implement the following functions:

- `fn kidi_driver_load()`
    - This is the main entry point for the driver. This function is called when the driver is loaded.
    - This function should initialize the driver and register any devices with the kernel.