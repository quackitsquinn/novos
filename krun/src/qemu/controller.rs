use std::{
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct QemuCtl {
    inner: Arc<RwLock<QemuInner>>,
}

impl QemuCtl {
    pub fn new(qemu: Child) -> Self {
        QemuCtl {
            inner: Arc::new(RwLock::new(QemuInner::new(qemu))),
        }
    }

    pub fn try_shutdown(&self) -> Result<(), std::io::Error> {
        let mut qemu = self.inner.write().unwrap();
        if qemu.try_wait()?.is_none() {
            qemu.kill()?;
        }
        Ok(())
    }

    pub fn kill(&self) -> Result<(), std::io::Error> {
        let mut qemu = self.inner.write().unwrap();
        qemu.kill()
    }

    pub fn died(&self) -> bool {
        let mut qemu = self.inner.write().unwrap();
        match qemu.try_wait() {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => true,
        }
    }
    // .. other methods
}

impl Clone for QemuCtl {
    fn clone(&self) -> Self {
        QemuCtl {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
struct QemuInner {
    qemu: Child,
}

impl QemuInner {
    fn new(qemu: Child) -> Self {
        QemuInner { qemu }
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        self.qemu.try_wait()
    }

    fn kill(&mut self) -> Result<(), std::io::Error> {
        self.qemu.kill()
    }
}
