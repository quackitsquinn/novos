//! A module for resolving the `Resolve` graph from `cargo metadata` to a list of `Package`s with all dependencies resolved to their corresponding
//! `PackageID`s and `CrateName`s.
use std::{collections::HashMap, fmt::Display, sync::Arc};

use log::info;

use crate::meta::{self, TargetKind};

/// A resolver that takes the `Resolve` graph and resolves all dependencies to their corresponding `PackageID`s and `CrateName`s.
#[derive(Debug)]
pub struct Resolver {
    resolve: Option<meta::Resolve>,
    packages_by_id: HashMap<meta::PackageID, meta::Package>,
    errors: Vec<ResolveError>,
}

/// An error that occurred during the resolution process.
#[derive(Debug, Clone)]
pub struct ResolveError {
    package: Option<meta::Package>,
    reason: ResolveErrorReason,
}

impl ResolveError {
    /// Creates a new `ResolveError` with the given package name, package ID, and reason.
    pub fn new(package: Option<meta::Package>, reason: ResolveErrorReason) -> Self {
        Self { package, reason }
    }
}

impl Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error resolving package ")?;
        if let Some(pkg) = &self.package {
            write!(f, "{} (id: {})", pkg.name.as_str(), pkg.id.as_str())?;
        } else {
            write!(f, "<unknown>")?;
        }

        writeln!(f)?;

        writeln!(f, "{}", self.reason)?;

        Ok(())
    }
}

/// The reason for a resolution error.
#[derive(Debug, Clone)]
pub enum ResolveErrorReason {
    /// A package ID in the `Resolve` graph does not correspond to any package in the `packages` list.
    NodePackageNotFound {
        /// The package ID that was not found.
        package_id: meta::PackageID,
    },
    /// A dependency in the `Resolve` graph does not correspond to any package in the `packages` list.
    DependencyPackageNotFound {
        /// The package ID of the dependency that was not found.
        dependency_id: meta::PackageID,
    },
    /// A dependency in the `Resolve` graph does not have a `lib` or `proc-macro` target,
    /// so it cannot be defined as a dependency in the `Cargo.toml` file.
    DependencyHasNoLibTarget {
        /// The package of the dependency that has no `lib` or `proc-macro` target.
        dependency_package: meta::Package,
    },
    /// A dependency in the `Resolve` graph does not correspond to any dependency in the `Cargo.toml` file of the package that depends on it.
    DependencyNotFoundInPackage {
        /// The package of the dependency that was not found in the `Cargo.toml` file of the package that depends on it.
        dependency_package: meta::Package,
        /// The list of dependencies defined in the `Cargo.toml` file of the package that depends on the dependency that was not found.
        defined_dependencies: Arc<[meta::Dependency]>,
    },
}

impl Display for ResolveErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveErrorReason::NodePackageNotFound { package_id } => {
                write!(
                    f,
                    "Package ID {} not found in packages list",
                    package_id.as_str()
                )
            }
            ResolveErrorReason::DependencyPackageNotFound { dependency_id } => {
                write!(
                    f,
                    "Dependency package ID {} not found in packages list",
                    dependency_id.as_str()
                )
            }
            ResolveErrorReason::DependencyHasNoLibTarget { dependency_package } => {
                write!(
                    f,
                    "Dependency {} (ID: {}) has no lib or proc-macro target",
                    dependency_package.name.as_str(),
                    dependency_package.id.as_str()
                )
            }
            ResolveErrorReason::DependencyNotFoundInPackage {
                dependency_package,
                defined_dependencies,
            } => {
                write!(
                    f,
                    "Dependency {} (ID: {}) not found in package's Cargo.toml dependencies",
                    dependency_package.name.as_str(),
                    dependency_package.id.as_str()
                )?;
                for pkg in defined_dependencies.iter() {
                    write!(
                        f,
                        "\n  - {} ({}), optional: {}, features: {:?}",
                        pkg.info().effective_name().as_str(),
                        pkg.info().global_name.as_str(),
                        pkg.info().optional,
                        pkg.info().features
                    )?;
                }
                Ok(())
            }
        }
    }
}

impl Resolver {
    /// Creates a new `Resolver` with the given packages.
    pub fn new(packages: Vec<meta::Package>, resolve: meta::Resolve) -> Self {
        let packages_by_id = packages
            .iter()
            .map(|pkg| (pkg.id.clone(), pkg.clone()))
            .collect();
        Self {
            resolve: Some(resolve),
            packages_by_id,
            errors: Vec::new(),
        }
    }

    fn resolve_node(&mut self, node: &meta::ResolveNode) -> Result<meta::Package, ResolveError> {
        info!(
            "Resolving package {:?} ({} dependencies)",
            node.id,
            node.dependencies.len()
        );
        let mut package = self.packages_by_id.get(&node.id).cloned().ok_or_else(|| {
            ResolveError::new(
                None,
                ResolveErrorReason::NodePackageNotFound {
                    package_id: node.id.clone(),
                },
            )
        })?;
        info!(
            "Found package {:?} with {} targets",
            package.name,
            package.targets.len()
        );
        for dep in node.dependencies.iter() {
            let dep_package = self.packages_by_id.get(&dep.id).ok_or_else(|| {
                ResolveError::new(
                    Some(package.clone()),
                    ResolveErrorReason::DependencyPackageNotFound {
                        dependency_id: dep.id.clone(),
                    },
                )
            })?;

            let mut dep_package_name = None;
            for target in dep_package.targets.iter() {
                match target.kind {
                    TargetKind::Lib | TargetKind::ProcMacro => {
                        dep_package_name = Some(target.name.clone());
                        break;
                    }
                    _ => continue,
                }
            }

            let dep_package_name = match dep_package_name {
                Some(name) => name,
                None => {
                    return Err(ResolveError::new(
                        Some(package.clone()),
                        ResolveErrorReason::DependencyHasNoLibTarget {
                            dependency_package: dep_package.clone(),
                        },
                    ));
                }
            };

            info!(
                "Resolving dependency {:?} for package {:?} ",
                dep.id, dep_package_name
            );

            let dep_desc = package.dependencies.iter_mut().find(|d| {
                *d.info().effective_name() == dep_package_name
                    || d.info().global_name == dep_package_name
                    || *d.info().effective_name() == dep_package.name
            });

            let dep_desc = match dep_desc {
                Some(desc) => desc,
                None => {
                    return Err(ResolveError::new(
                        Some(package.clone()),
                        ResolveErrorReason::DependencyNotFoundInPackage {
                            dependency_package: dep_package.clone(),
                            defined_dependencies: Arc::from(package.dependencies.clone()),
                        },
                    ));
                }
            };

            *dep_desc = meta::Dependency::Resolved {
                info: dep_desc.info().clone(),
                resolved_id: dep.id.clone(),
            };
        }
        Ok(package)
    }

    /// Parses the `Resolve` graph and resolves all dependencies to their corresponding `PackageID`s and `CrateName`s.
    pub fn resolve(&mut self) -> Vec<meta::Package> {
        let resolve = self.resolve.take().unwrap();
        let mut resolved_packages = Vec::with_capacity(self.packages_by_id.len());
        for node in resolve.nodes.iter() {
            let res = self.resolve_node(node);
            match res {
                Ok(pkg) => {
                    resolved_packages.push(pkg);
                }
                Err(err) => {
                    self.errors.push(err);
                }
            }
        }
        println!(
            "Resolved {} packages with {} errors",
            self.packages_by_id.len(),
            self.errors.len()
        );

        for error in self.errors.iter() {
            eprintln!("{}", error);
        }
        resolved_packages
    }
}
