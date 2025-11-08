use std::{
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, RwLock},
};

pub struct QemuCtl {
    inner: Arc<RwLock<QemuInner>>,
}

impl QemuCtl {
    pub fn new(qemu: Child, socket_path: &Path) -> Self {
        QemuCtl {
            inner: Arc::new(RwLock::new(QemuInner::new(qemu, socket_path.to_path_buf()))),
        }
    }

    pub fn try_shutdown(&self) -> Result<(), std::io::Error> {
        let mut qemu = self.inner.write().unwrap();
        if qemu.try_wait()?.is_none() {
            qemu.kill()?;
        }
        Ok(())
    }

    pub fn get_pty_path(&self) -> PathBuf {
        let qemu = self.inner.read().unwrap();
        qemu.pty_path.clone()
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

struct QemuInner {
    qemu: Child,
    pty_path: PathBuf,
}

impl QemuInner {
    fn new(qemu: Child, pty_path: PathBuf) -> Self {
        QemuInner { qemu, pty_path }
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        self.qemu.try_wait()
    }

    fn kill(&mut self) -> Result<(), std::io::Error> {
        self.qemu.kill()
    }
}
