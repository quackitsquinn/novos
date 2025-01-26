use core::sync::atomic::Ordering;

use log::info;

use crate::{serial, sprintln};

use super::IN_TEST_FRAMEWORK;

#[derive(Debug, Clone, Copy)]
pub struct TestFunction {
    /// The function to run.
    pub function: fn(),
    /// The name of the function.
    pub function_name: &'static str,
    /// The name of the test that will be displayed to the user
    /// This should be a human readable name.
    pub human_name: &'static str,
    /// If this test fails/panics, should we continue running tests?
    /// This should be false for tests that test the kernel's core functionality.
    pub can_recover: bool,
    /// This test needs to panic to pass.
    pub should_panic: bool,
    /// The number of times this test should be run.
    pub bench_count: Option<usize>,
}

impl Default for TestFunction {
    fn default() -> Self {
        Self::const_default()
    }
}

impl TestFunction {
    pub const fn const_default() -> Self {
        Self {
            function: || {},
            function_name: "",
            human_name: "",
            can_recover: false,
            bench_count: None,
            should_panic: false,
        }
    }
    pub fn run(&self) {
        #[allow(unused_unsafe)]
        unsafe {
            #[cfg(test)]
            crate::memory::allocator::TEST_ALLOCATOR
                .get()
                .blocks
                .clear();
        }
        if let Some(count) = self.bench_count {
            self.do_bench(count);
        } else {
            *IN_TEST_FRAMEWORK.lock() = false;
            (self.function)();
            *IN_TEST_FRAMEWORK.lock() = true;
        }
        if self.should_panic {
            self.failed();
        } else {
            self.passed();
        }
    }

    fn do_bench(&self, count: usize) {
        let log_level = serial::LOG_LEVEL;
        info!("Reducing log level to error for benchmarking");
        log::set_max_level(log::LevelFilter::Error);
        // Don't bother to set / unset IN_TEST_FRAMEWORK as we're not going to panic on a for loop
        *IN_TEST_FRAMEWORK.lock() = false;
        for _ in 0..count {
            (self.function)();
        }
        *IN_TEST_FRAMEWORK.lock() = true;
        log::set_max_level(log_level.to_level_filter());
        info!("Restored log level to {}", log_level);
    }

    pub(super) fn passed(&self) {
        sprintln!(
            "\x1b[32m[PASSED]\x1b[0m {} ({})",
            self.human_name,
            self.function_name
        );
    }

    pub(super) fn failed(&self) {
        sprintln!(
            "\x1b[31m[FAILED]\x1b[0m {} ({})",
            self.human_name,
            self.function_name
        );
    }
}
