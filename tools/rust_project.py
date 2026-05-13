# Generates a rust-project.json file for the current workspace, used because rust-analyzer currently doesn't have great
# support for workspaces with multiple targets (e.g. a kernel + host tools + userspace software)

import os
import subprocess
import json
from pprint import pprint
from typing import Any

from enum import Enum
from dataclasses import dataclass


def rustc(*args) -> str | None:
    proc = subprocess.run(["rustc", *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return proc.stdout.decode("utf-8") if proc.returncode == 0 else None


def cargo(*args) -> str | None:
    proc = subprocess.run(["cargo", *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return proc.stdout.decode("utf-8") if proc.returncode == 0 else None


def canonicalize_crate_name(name: str) -> str:
    return name.replace("-", "_")


sysroot = rustc("--print", "sysroot")
print(sysroot)

meta_raw = cargo("metadata", "--format-version=1")
if meta_raw is None:
    print("Error: cargo metadata failed")
    exit(-1)
metadata = json.loads(meta_raw)


class DepType(Enum):
    Normal = "normal"
    Build = "build"
    Dev = "dev"


class TargetType(Enum):
    Lib = "lib"
    Bin = "bin"
    RLib = "rlib"
    Dylib = "dylib"
    ProcMacro = "proc-macro"
    Test = "test"
    Example = "example"
    Bench = "bench"
    CustomBuild = "custom-build"


@dataclass
class Target:
    name: str
    kind: list[TargetType]
    crate_types: list[TargetType]
    src_path: str
    edition: str
    required_features: list[str] | None
    doc: bool
    doctest: bool
    test: bool

    @staticmethod
    def parse(data: dict) -> Target:
        return Target(
            name=canonicalize_crate_name(data["name"]),
            kind=[TargetType(k) for k in data["kind"]],
            crate_types=[TargetType(ct) for ct in data["crate_types"]],
            src_path=data["src_path"],
            edition=data["edition"],
            required_features=(data["required_features"] if "required_features" in data else None),
            doc=data["doc"],
            doctest=data["doctest"],
            test=data["test"],
        )


@dataclass
class DependencyInfo:
    name: str
    effective_name: str
    req: str
    kind: DepType
    optional: bool
    uses_default_features: bool
    features: list[str]
    target: str | None
    path: str | None
    registry: str | None
    local_name: str | None = None

    @staticmethod
    def parse(data: dict) -> DependencyInfo:
        local_name = data["rename"] if "rename" in data else None
        effective_name = canonicalize_crate_name(local_name) if local_name is not None else data["name"]
        effective_name = canonicalize_crate_name(effective_name)
        return DependencyInfo(
            name=canonicalize_crate_name(data["name"]),
            effective_name=effective_name,
            req=data["req"],
            kind=DepType(data["kind"]) if data["kind"] is not None else DepType.Normal,
            optional=data["optional"],
            uses_default_features=data["uses_default_features"],
            features=data["features"],
            target=data["target"],
            path=data["path"] if "path" in data else None,
            registry=data["registry"],
            local_name=local_name,
        )


@dataclass
class UnresolvedDependency:
    info: DependencyInfo

    @staticmethod
    def parse(dep: dict) -> UnresolvedDependency:
        info = DependencyInfo.parse(dep)
        return UnresolvedDependency(info)


@dataclass
class ResolvedDependency:
    info: DependencyInfo
    crate_id: str
    idx: int


@dataclass
class Crate:
    name: str
    ver: str
    id: str
    dependencies: list[UnresolvedDependency | ResolvedDependency]
    targets: list[Target]

    @staticmethod
    def parse(data: dict) -> Crate:
        deps = []
        for dep in data["dependencies"]:
            deps.append(UnresolvedDependency.parse(dep))
        targets = []
        for target in data["targets"]:
            targets.append(Target.parse(target))
        return Crate(
            canonicalize_crate_name(data["name"]),
            data["version"],
            data["id"],
            deps,
            targets,
        )
    
    def dependencies_to_rust_project(self) -> list[dict]:
        deps = []
        for dep in self.dependencies:
            if isinstance(dep, UnresolvedDependency):
                continue
            deps.append({
                "crate": dep.idx + slide,
                "name": dep.info.name,
            })
            
        return deps
    
    def to_rust_project(self) -> dict[str, Any]:
        for target in self.targets:
            if TargetType.Lib in target.kind or TargetType.ProcMacro in target.kind:
                crate = {
                    "root_module": target.src_path,
                    "edition": target.edition,
                    "version": self.ver,
                    "deps": self.dependencies_to_rust_project(),
                    # TODO: cfg, env, crate_attrs, 
                    "is_proc_macro": TargetType.ProcMacro in target.kind,
                    # TODO: proc macro .so path
                }
                return crate
        raise Exception(f"Crate {self.name} {self.ver} has no library or proc macro target, unable to represent in rust-project.json")





crates: list[Crate] = []
for pkg in metadata["packages"]:
    crates.append(Crate.parse(pkg))

crates_by_id = {crate[1].id: crate for crate in enumerate(crates)}

# {(name, type): crate index}
targets: dict[tuple[str, TargetType], int] = {}
for crate_idx, crate in enumerate(crates):
    for target in crate.targets:
        for kind in target.kind:
            targets[(target.name, kind)] = crate_idx

pprint(targets)


def lookup_target(name: str, kind: TargetType) -> int | None:
    index = (name, kind)
    if index in targets:
        return targets[index]
    return None


def lookup_crate(name: str) -> int | None:
    idx = lookup_target(name, TargetType.Bin)
    if idx is not None:
        return idx
    idx = lookup_target(name, TargetType.ProcMacro)
    if idx is not None:
        return idx
    return None


print(f"Found {len(crates)} crates in metadata")
print(f"Found {len(targets)} targets in metadata")

failed_deps = []
total_node_deps = 0


@dataclass
class FailedResolution:
    crate_name: str
    crate_ver: str
    dep_name: str
    reason: str
    ctx: Any


class CrateDependencies:
    def __init__(self, resolver: Resolver, crate: Crate):
        self.crate = crate
        self.resolve = resolver
        self.deps = crate.dependencies
        self.deps_by_name = {canonicalize_crate_name(dep[1].info.name): dep for dep in enumerate(crate.dependencies)}
        self.deps_by_effective_name = {canonicalize_crate_name(dep[1].info.effective_name): dep for dep in enumerate(crate.dependencies)}

    def find_dep(self, name: str) -> tuple[int, UnresolvedDependency | ResolvedDependency] | None:
        if dep := self.deps_by_name.get(name):
            return dep
        if dep := self.deps_by_effective_name.get(name):
            return dep
        # if we get here, it might be an overridden [lib] target, so attempt to look it up by name
        print(f"resolving {name} for crate {self.crate.name} {self.crate.ver}... ", end="")
        if dep_crate_idx := self.resolve.resolve_crate(name):
            print(f"found crate index {dep_crate_idx} for {name}")
            dep_crate = self.resolve.crates[dep_crate_idx][1]
            if dep := self.deps_by_name.get(dep_crate.name):
                return dep
        else:
            print(f"failed to find crate index for {name}")
        return None


class Resolver:
    def __init__(self, crates: list[Crate], target_index: dict[tuple[str, TargetType], int]):
        self.crates = tuple([(i, crate, CrateDependencies(self, crate)) for i, crate in enumerate(crates)])
        self.crates_by_id = {crate[1].id: crate for crate in self.crates}
        self.resolved_deps = 0
        self.total_deps = 0
        self.failed_deps: list[FailedResolution] = []
        self.target_index = target_index

    def resolve_target(self, name: str, kind: TargetType) -> int | None:
        index = (name, kind)
        if index in self.target_index:
            return self.target_index[index]
        return None

    def resolve_crate(self, name: str) -> int | None:
        idx = self.resolve_target(name, TargetType.Lib)
        if idx is not None:
            return idx
        idx= self.resolve_target(name, TargetType.ProcMacro)
        if idx is not None:
            return idx
        return None

    def resolve_dep(self, data: dict, crate: Crate, resolver: CrateDependencies) -> tuple[int, ResolvedDependency] | None:
        self.total_deps += 1
        node_name = canonicalize_crate_name(data["name"])
        print(f"Resolving dependency {node_name} for crate {crate.name} {crate.ver}")
        maybe_dep = resolver.find_dep(node_name)
        if maybe_dep is None:
            print(f"Failed to find dependency {node_name} for crate {crate.name} {crate.ver}")
            self.failed_deps.append(
                FailedResolution(
                    crate.name,
                    crate.ver,
                    node_name,
                    "unable to find dependency in crate's dependency list (considering both original and effective names, as well as [lib] target overrides)",
                    (data, resolver.deps_by_name),
                )
            )
            return
        local_dep_id, local_dep = maybe_dep
        global_name = local_dep.info.name
        dep_crate = self.crates_by_id.get(data["pkg"]) 
        if dep_crate is not None:
            dep_crate_idx, dep_crate, _ = dep_crate
            return local_dep_id, ResolvedDependency(
                info=local_dep.info,
                crate_id=data["pkg"],
                idx=dep_crate_idx,
            )
        print(f"Failed to resolve dependency {global_name} for crate {crate.name} {crate.ver}")
        self.failed_deps.append(
            FailedResolution(
                crate.name,
                crate.ver,
                global_name,
                "failed to resolve crate for dependency",
                (data, resolver.deps_by_name),
            )
        )
        return None

    def resolve(self, resolve: dict) -> list[Crate]:
        for node in resolve["nodes"]:
            crate_idx, crate, resolver = self.crates_by_id[node["id"]]
            for dep in node["deps"]:
                resolved = self.resolve_dep(dep, crate, resolver)
                if resolved is not None:
                    local_dep_id, resolved_dep = resolved
                    resolver.deps[local_dep_id] = resolved_dep
                    self.resolved_deps += 1
        print(f"Resolved {self.resolved_deps} out of {self.total_deps} dependencies")
        for failed in self.failed_deps:
            print(f"Failed to resolve dependency {failed.dep_name} for crate {failed.crate_name} {failed.crate_ver}: {failed.reason} ", end="")
            pprint(failed.ctx)
        return [crate for _, crate, _ in self.crates]


resolver = Resolver(crates, targets)
crates = resolver.resolve(metadata["resolve"])
pprint(crates)

total = 0
resolved = 0
for crate in crates:
    print(f"{crate.name} {crate.ver}")
    for dep in crate.dependencies:
        print(f"  {dep.info.name} {dep.info.req} ({dep.info.kind.value})", end="")
        total += 1
        if isinstance(dep, ResolvedDependency):
            print(f" -> {dep.crate_id}", end="")
            if dep.info.local_name is not None:
                print(f" (renamed to {dep.info.local_name})", end="")
            print()
            resolved += 1
        else:
            print(" (unresolved)")
    print(f"targets for {crate.name} {crate.ver}:")
    for target in crate.targets:
        print(f"  {target.name} ({', '.join([k.value for k in target.kind])})")


print(f"Resolved {resolved} out of {total} dependencies")
if len(resolver.failed_deps) > 0:
    print(f"Failed to resolve {len(resolver.failed_deps)} dependencies:")
    for dep in failed_deps:
        print(f"Failed to resolve dependency {dep[2]} for crate {dep[0]} {dep[1]}")
    exit(-1)

print("All dependencies resolved successfully, generating rust-project.json...")

rust_crates = []
slide = 0
for crate in crates:
    crate_repr = crate.to_rust_project()
    rust_crates.append(crate_repr)

rust_project = {
    "crates": rust_crates,
    "sysroot": sysroot,
}

with open("rust-project.json", "w") as f:
    json.dump(rust_project, f, indent=4)