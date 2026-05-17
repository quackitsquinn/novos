#![allow(missing_docs)] // All of these structs closely follow the structure of `cargo metadata`.
// Differences will be documented, but otherwise you are expected to refer to the `cargo metadata` documentation for details.
use std::{collections::HashMap, ops::Deref, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub resolve: Resolve,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Package {
    pub id: PackageID,
    pub name: CrateName,
    pub version: Arc<str>,
    pub dependencies: Vec<Dependency>,
    pub targets: Arc<[Target]>,
    pub features: HashMap<Arc<String>, Vec<String>>,
    pub manifest_path: Arc<Path>,
    pub metadata: Option<serde_json::Value>,
    pub edition: RustEdition,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct DependencyInfo {
    /// The name of the dependency as specified in the `Cargo.toml` file. This is the name that other packages will use to refer to this dependency.
    ///
    /// If let Some(name) = local_name, then the effective name of the dependency is `name`, otherwise it is `global_name`.
    #[serde(rename = "name")]
    pub global_name: CrateName,
    /// The name of the dependency as specified in the `Cargo.toml` file, if it is renamed.
    /// This is the name that this package will use to refer to this dependency.
    #[serde(rename = "rename")]
    pub local_name: Option<CrateName>,
    #[serde(rename = "req")]
    pub request: String,
    pub kind: DepType,
    pub optional: bool,
    pub uses_default_features: bool,
    pub features: Vec<String>,
    pub target: Option<String>,
    pub path: Option<String>,
    pub registry: Option<String>,
}

impl DependencyInfo {
    /// Returns the effective name of the dependency, which is the local name if it exists, otherwise the global name.
    pub fn effective_name(&self) -> &CrateName {
        self.local_name.as_ref().unwrap_or(&self.global_name)
    }
}

/// A dependency.
#[derive(Debug, Clone)]
pub enum Dependency {
    /// A dependency that has not been resolved yet.
    /// It contains the information from the `Cargo.toml` file, but does not have a resolved package ID or crate name.
    ///
    /// NOTE: Not all dependencies will be resolved. `cargo metadata` won't resolve optional dependencies that aren't enabled.
    Unresolved(DependencyInfo),
    /// A dependency that has been resolved.
    /// It contains the information from the `Cargo.toml` file, as well as the resolved package ID and crate name.
    Resolved {
        /// The information from the `Cargo.toml` file.
        info: DependencyInfo,
        /// The resolved package ID of the dependency. This is the package ID of the dependency that this package depends on.
        resolved_id: PackageID,
    },
}

impl Dependency {
    /// Returns the information from the `Cargo.toml` file for this dependency, regardless of whether it has been resolved or not.
    pub fn info(&self) -> &DependencyInfo {
        match self {
            Dependency::Unresolved(info) => info,
            Dependency::Resolved { info, .. } => info,
        }
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let info = DependencyInfo::deserialize(deserializer)?;
        Ok(Dependency::Unresolved(info))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DepType {
    Normal,
    Dev,
    Build,
}

impl<'de> Deserialize<'de> for DepType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <Option<String>>::deserialize(deserializer)?;
        match s.as_deref() {
            Some("dev") => Ok(DepType::Dev),
            Some("build") => Ok(DepType::Build),
            _ => Ok(DepType::Normal), // Default to Normal if not specified
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: CrateName,
    // this is technically a list, but in practice it seems to always be a list of length 1.
    // i think if this was more than 1 it'd introduce a lot of ambiguity in the resolution process,
    // so it's probably safe to assume that it's always 1.
    pub kind: TargetKind,
    pub src_path: Arc<Path>,
    pub edition: RustEdition,
    // for a language that loves consistency,
    // it's quite strange they mix kebab-case and snake_case in their JSON output...
    #[serde(rename = "required-features")]
    pub required_features: Option<Vec<String>>,
    pub doc: bool,
    pub test: bool,
}

#[derive(Debug, Deserialize)]
pub struct Resolve {
    pub nodes: Arc<[ResolveNode]>,
    pub root: Option<PackageID>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveNode {
    pub id: PackageID,
    #[serde(rename = "deps")]
    pub dependencies: Vec<ResolveDep>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveDep {
    pub name: CrateName,
    #[serde(rename = "pkg")]
    pub id: PackageID,
    pub dep_kinds: Vec<DepKind>,
}

#[derive(Debug, Deserialize)]
pub struct DepKind {
    pub kind: DepType,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TargetKind {
    Bin,
    Lib,
    RLib,
    Dylib,
    ProcMacro,
    Example,
    Test,
    Bench,
    CustomBuild,
}
impl<'de> Deserialize<'de> for TargetKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // This is a weird way to deserialize this, but the `kind` field in the JSON is an array.
        let s = <[Arc<str>; 1]>::deserialize(deserializer)?;
        match &*s[0] {
            "bin" => Ok(TargetKind::Bin),
            "lib" => Ok(TargetKind::Lib),
            "rlib" => Ok(TargetKind::RLib),
            "dylib" => Ok(TargetKind::Dylib),
            "proc-macro" => Ok(TargetKind::ProcMacro),
            "example" => Ok(TargetKind::Example),
            "test" => Ok(TargetKind::Test),
            "bench" => Ok(TargetKind::Bench),
            "custom-build" => Ok(TargetKind::CustomBuild),
            _ => Err(serde::de::Error::custom(format!(
                "Unknown target kind: {}",
                s[0]
            ))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[non_exhaustive]
pub enum RustEdition {
    #[serde(rename = "2015")]
    Edition2015,
    #[serde(rename = "2018")]
    Edition2018,
    #[serde(rename = "2021")]
    Edition2021,
    #[serde(rename = "2024")]
    Edition2024,
}

/// An opaque identifier for a package. This should be the preferred way to refer to a package.
#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct PackageID(Arc<str>);

impl<'de> Deserialize<'de> for PackageID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PackageID(Arc::from(s)))
    }
}

impl PackageID {
    pub fn new(name: &str) -> Self {
        PackageID(Arc::from(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A canonical name for a crate.
///
/// This is a normal string, but all occurrences of `-` are replaced with `_`, since Rust crate names are normalized in this way.
#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct CrateName(Arc<str>);

impl CrateName {
    pub fn new(name: &str) -> Self {
        CrateName(Arc::from(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for CrateName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CrateName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut s = String::deserialize(deserializer)?;
        // SAFETY: We just replace `-` with `_`, which is valid UTF-8, so the resulting string is still valid UTF-8.
        // This is needed to prevent a unnecessary allocation when the crate name contains `-`, which is common in Rust crate names.
        unsafe {
            s.as_bytes_mut().iter_mut().for_each(|b| {
                if *b == b'-' {
                    *b = b'_';
                }
            });
        }
        Ok(CrateName(Arc::from(s)))
    }
}
