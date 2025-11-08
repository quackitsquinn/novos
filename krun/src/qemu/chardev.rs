use std::{path::Path, rc::Rc};

/// Character device representation for QEMU virtual machines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharDev {
    /// Identifier for the character device. This is only used to check uniqueness and validity of character device IDs.
    pub(super) id: Rc<str>,
    /// Raw representation of the character device.
    repr: Rc<str>, // rc to allow cheap cloning
}

/// Reference to a character device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharDevRef(Rc<str>);

impl CharDevRef {
    /// Returns the raw representation of the character device reference.
    pub(super) fn id(&self) -> Rc<str> {
        self.0.clone()
    }
}

impl CharDev {
    /// Creates a new `CharDev` from a raw string representation.
    pub fn from_raw(repr: &str) -> Self {
        let id = try_find_id(repr);
        if id.is_none() {
            panic!(
                "Character device representation must contain an 'id' field, {}",
                repr
            );
        }

        Self {
            id: Rc::from(id.unwrap()),
            repr: repr.to_string().into(),
        }
    }

    fn from_raw_parts(id: &str, repr: &str) -> Self {
        Self {
            id: Rc::from(id),
            repr: Rc::from(repr),
        }
    }

    /// Creates a new Unix socket character device.
    ///
    /// `path` is the file system path to the Unix socket.
    ///
    /// `id` is the identifier for the character device (max 127 characters).
    ///
    /// `server` specifies whether the socket should operate in server mode.
    /// The contained bool controls if it should wait for a connection (true) or not (false). If `None`, defaults to `false`.
    pub fn unix_socket(id: &str, path: &Path, server: Option<bool>) -> Self {
        if id.len() > 127 {
            panic!("Character device ID exceeds maximum allowed value of 127");
        }
        let server = server.unwrap_or(false);
        let wait = if server { ",wait=on" } else { "" };
        let repr = format!(
            "socket,path={},server={}{},id={}",
            path.display(),
            server,
            wait,
            id
        );
        Self::from_raw_parts(id, &repr)
    }

    /// Creates a new PTY character device.
    ///
    /// `id` is the identifier for the character device (max 127 characters).
    ///
    /// `link` is an optional tuple containing a path to create a symlink to the PTY
    /// and a boolean indicating whether to remove any existing symlink.
    pub fn pty(id: &str, link: Option<(&Path, bool)>) -> Self {
        if id.len() > 127 {
            panic!("Character device ID exceeds maximum allowed value of 127");
        }

        if let Some((link_path, remove_existing)) = link {
            if remove_existing {
                let _ = std::fs::remove_file(link_path);
            }
            let repr = format!("pty,path={},id={}", link_path.display(), id);
            Self::from_raw_parts(id, &repr)
        } else {
            let repr = format!("pty,id={}", id);
            Self::from_raw_parts(id, &repr)
        }
    }

    /// Returns the identifier of the character device.
    pub fn as_parameter(&self) -> Rc<str> {
        self.repr.clone()
    }

    /// Returns a reference to the character device.
    pub(super) fn dev_ref(&self) -> CharDevRef {
        CharDevRef(self.id.clone())
    }
}

fn try_find_id(repr: &str) -> Option<String> {
    for part in repr.split(',') {
        if let Some(id_part) = part.strip_prefix("id=") {
            if id_part.len() > 127 {
                panic!("Character device ID exceeds maximum allowed value of 127");
            }
            return Some(id_part.to_string());
        }
    }
    None
}
