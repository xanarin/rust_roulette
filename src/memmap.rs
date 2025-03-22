use anyhow::{Context, Result, anyhow};
use nix::sys::mman::{ProtFlags, mprotect};
use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ptr::NonNull;

#[derive(Clone)]
pub struct Mapping {
    pub start_addr: u64,
    pub end_addr: u64,
    pub permissions: PermissionSet,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionSet {
    readable: bool,
    writeable: bool,
    executable: bool,
}

impl PermissionSet {
    pub fn from(s: &str) -> PermissionSet {
        let mut result = PermissionSet {
            readable: false,
            writeable: false,
            executable: false,
        };
        for letter in s.chars() {
            match letter {
                'r' => result.readable = true,
                'w' => result.writeable = true,
                'x' => result.executable = true,
                _ => continue,
            };
        }
        result
    }

    pub fn and(&self, mask: &PermissionSet) -> bool {
        return (self.readable && (self.readable == mask.readable))
            || (self.writeable && (self.writeable == mask.writeable))
            || (self.executable && (self.executable == mask.executable));
    }
}

impl Into<ProtFlags> for PermissionSet {
    fn into(self) -> ProtFlags {
        let mut result = ProtFlags::empty();
        result.set(ProtFlags::PROT_READ, self.readable);
        result.set(ProtFlags::PROT_WRITE, self.writeable);
        result.set(ProtFlags::PROT_EXEC, self.executable);

        result
    }
}

impl Display for PermissionSet {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.readable {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }

        if self.writeable {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }

        if self.executable {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        Ok(())
    }
}

impl Mapping {
    pub fn new(start_addr: u64, end_addr: u64, permissions: String, path: String) -> Mapping {
        Mapping {
            start_addr,
            end_addr,
            permissions: PermissionSet::from(&permissions),
            path,
        }
    }

    // Get the size of the mapping in bytes
    pub fn size(&self) -> u64 {
        return self.end_addr - self.start_addr;
    }

    pub fn set_permissions(&mut self, new_perms: PermissionSet) -> Result<()> {
        let ptr = NonNull::new(self.start_addr as *mut c_void).ok_or(anyhow!(format!(
            "Failed to cast address 0x{:x} as it was null",
            self.start_addr
        )))?;
        unsafe {
            mprotect(ptr, self.size() as usize, new_perms.into())
                .context(format!("Failed to set new page permissions on 0x{:x}", self.start_addr))
        }
    }
}

impl Display for Mapping {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Mapping(0x{:x}-0x{:x}, perms={}, path={})",
            self.start_addr, self.end_addr, self.permissions, self.path
        )
    }
}

pub fn get_memmap(pid: u32) -> Result<Vec<Mapping>> {
    let mut results: Vec<Mapping> = Vec::new();

    let file = File::open(format!("/proc/{}/maps", pid))
        .context("Failed to open process memory map in procfs")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Err(e) => {
                return Err(e).context("Failed to read line in procfs maps file");
            }
            Ok(line) => line,
        };
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            println!(
                "Failed to parse bad maps entry because only {} fields were found: '{}'",
                fields.len(),
                line
            );
            continue;
        }
        let addresses = fields[0]
            .split("-")
            .map(|s| u64::from_str_radix(s, 16))
            .collect::<Result<Vec<u64>, _>>()
            .context(format!("Failed to parse addresses '{}'", fields[0]))?;
        if addresses.len() != 2 {
            println!(
                "Failed to parse addressess: {}, got {:?}",
                fields[0], addresses
            );
            continue;
        }
        let perms = fields[1];
        let path = {
            if fields.len() > 5 {
                fields[5..].join(" ")
            } else {
                "".to_string()
            }
        };
        results.push(Mapping::new(
            addresses[0],
            addresses[1],
            perms.to_string(),
            path.to_string(),
        ));
    }

    Ok(results)
}
