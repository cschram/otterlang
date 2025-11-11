use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use super::metadata::DependencyConfig;
use super::metadata::{CrateSpec, FnSig, PublicItem, RustPath, RustTypeRef};
use cache::path::cache_root;

pub fn generate_rustdoc_json(dep: &DependencyConfig) -> Result<PathBuf> {
    let root = match cache_root() {
        Ok(path) => path.join("ffi").join("rustdoc").join(&dep.name),
        Err(_) => return Err(anyhow!("Failed to get cache root")),
    };
    fs::create_dir_all(&root)
        .with_context(|| format!("failed to create rustdoc cache dir {}", root.display()))?;
    let manifest = root.join("Cargo.toml");
    let src_dir = root.join("src");
    let lib_rs = src_dir.join("lib.rs");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    let manifest_contents = format!(
        "[package]\nname = \"otter_rustdoc_{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{} = {}\n",
        dep.name,
        dep.name,
        dep.manifest_entry()
    );
    fs::write(&manifest, manifest_contents)
        .with_context(|| format!("failed writing {}", manifest.display()))?;
    fs::write(&lib_rs, "")?;

    let target_dir = root.join("target");
    fs::create_dir_all(&target_dir)?;

    let try_nightly_doc = || {
        duct::cmd(
            "cargo",
            vec![
                "+nightly",
                "doc",
                "-p",
                &dep.name,
                "--manifest-path",
                manifest.to_str().unwrap(),
            ],
        )
        .dir(&root)
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
        .run()
    };

    let mut ran = try_nightly_doc();
    if ran.is_err() || !ran.as_ref().unwrap().status.success() {
        ran = duct::cmd(
            "cargo",
            vec![
                "+nightly",
                "rustdoc",
                "-p",
                &dep.name,
                "--manifest-path",
                manifest.to_str().unwrap(),
                "--",
                "-Z",
                "unstable-options",
                "--output-format",
                "json",
            ],
        )
        .dir(&root)
        .env("CARGO_TARGET_DIR", &target_dir)
        .run();
    }

    if ran.is_err() || !ran.as_ref().unwrap().status.success() {
        return Err(anyhow!("failed to produce rustdoc JSON for `{}`", dep.name));
    }

    let doc_dir = target_dir.join("doc");
    let json_path = fs::read_dir(&doc_dir)
        .with_context(|| format!("failed to read {}", doc_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|ext| ext == "json").unwrap_or(false)
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s == dep.name || s.starts_with(&dep.name))
                    .unwrap_or(false)
        })
        .next()
        .ok_or_else(|| anyhow!("rustdoc JSON for crate `{}` not found under {}", dep.name, doc_dir.display()))?;

    Ok(json_path)
}

pub fn extract_crate_spec_from_json(
    crate_name: &str,
    version: Option<String>,
    rustdoc_json_path: &Path,
) -> Result<CrateSpec> {
    let data = fs::read_to_string(rustdoc_json_path).with_context(|| {
        format!(
            "failed to read rustdoc JSON from {}",
            rustdoc_json_path.display()
        )
    })?;
    let doc: Rustdoc = serde_json::from_str(&data).with_context(|| {
        format!(
            "failed to parse rustdoc JSON at {}",
            rustdoc_json_path.display()
        )
    })?;
    Ok(normalize(crate_name.to_string(), version, doc))
}

pub fn extract_crate_spec(_dep: &DependencyConfig) -> Result<CrateSpec> {
    let json = generate_rustdoc_json(_dep)?;
    extract_crate_spec_from_json(&_dep.name, _dep.version.clone(), &json)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Rustdoc {
    index: serde_json::Map<String, serde_json::Value>,
    paths: serde_json::Map<String, serde_json::Value>,
    crate_version: Option<String>,
    #[serde(default)]
    crate_id: Option<usize>,
    root: Option<serde_json::Value>,
    #[serde(default)]
    external_crates: serde_json::Map<String, serde_json::Value>,
}

fn normalize(name: String, version: Option<String>, doc: Rustdoc) -> CrateSpec {
    use std::collections::HashSet;
    
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    let _target_crate_id = doc.external_crates
        .iter()
        .find_map(|(id_str, crate_info)| {
            crate_info.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|n| n.as_str())
                .filter(|n| *n == name)
                .and_then(|_| id_str.parse::<usize>().ok())
        })
        .or_else(|| {
            if let Some(root_val) = doc.root.as_ref() {
                if let Some(root_obj) = root_val.as_object() {
                    root_obj.get("crate_id")
                        .and_then(|id| id.as_u64())
                        .map(|id| id as usize)
                } else if let Some(root_id) = root_val.as_u64() {
                    Some(root_id as usize)
                } else {
                    None
                }
            } else {
                None
            }
        });

    for (path_id, path_value) in &doc.paths {
        let path_obj = match path_value.as_object() {
            Some(obj) => obj,
            None => continue,
        };

        if path_obj.get("kind").and_then(|k| k.as_str()) != Some("function") {
            continue;
        }

        let path_array = match path_obj.get("path").and_then(|p| p.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => continue,
        };

        let path_segments: Vec<String> = path_array
            .iter()
            .filter_map(|seg| {
                if let Some(s) = seg.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = seg.as_object() {
                    obj.get("name")?.as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        if path_segments.is_empty() {
            continue;
        }

        let first_segment = match path_segments.first() {
            Some(seg) => seg,
            None => continue,
        };

        if first_segment != &name {
            continue;
        }

        let _func_name = match path_segments.last() {
            Some(name) => name.clone(),
            None => continue,
        };

        let item_id = path_id;
        let item_value = match doc.index.get(item_id) {
            Some(val) => val,
            None => continue,
        };

        let item_obj = match item_value.as_object() {
            Some(obj) => obj,
            None => continue,
        };

        if !item_obj.get("inner")
            .and_then(|i| i.as_object())
            .map_or(false, |inner| inner.contains_key("function"))
        {
            continue;
        }

        if is_trait_method(item_obj) {
            continue;
        }

        if requires_type_parameters(item_obj) {
            continue;
        }

        if let Some(function_item) = extract_function(item_obj, &path_segments) {
            if let PublicItem::Function { path, sig, .. } = &function_item {
                let key = (
                    path.segments.clone(),
                    sig.name.clone(),
                );
                if seen.insert(key) {
                    items.push(function_item);
                }
            }
        }
    }

    CrateSpec {
        name,
        version,
        items,
    }
}


fn is_trait_method(item: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(inner) = item.get("inner").and_then(|i| i.as_object()) {
        if let Some(func) = inner.get("function").and_then(|f| f.as_object()) {
            if let Some(sig) = func.get("sig").and_then(|s| s.as_object()) {
                if let Some(decl) = sig.get("decl").and_then(|d| d.as_object()) {
                    if let Some(inputs) = decl.get("inputs").and_then(|i| i.as_array()) {
                        if let Some(first_input) = inputs.first().and_then(|i| i.as_object()) {
                            if let Some(name) = first_input.get("name").and_then(|n| n.as_str()) {
                                if name == "self" || (name.starts_with("&") && name.contains("self")) || name == "&mut self" {
                                    return true;
                                }
                            }
                            if let Some(ty) = first_input.get("type") {
                                if let Some(ty_obj) = ty.as_object() {
                                    if let Some(ty_name) = ty_obj.get("name").and_then(|n| n.as_str()) {
                                        if ty_name == "Self" || ty_name.contains("self") {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn requires_type_parameters(item: &serde_json::Map<String, serde_json::Value>) -> bool {
    if let Some(inner) = item.get("inner").and_then(|i| i.as_object()) {
        if let Some(func) = inner.get("function").and_then(|f| f.as_object()) {
            if let Some(generics) = func.get("generics") {
                if let Some(generics_obj) = generics.as_object() {
                    if let Some(params) = generics_obj.get("params").and_then(|p| p.as_array()) {
                        if !params.is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn extract_function(
    item: &serde_json::Map<String, serde_json::Value>,
    path_segments: &[String],
) -> Option<PublicItem> {
    let name = item.get("name")?.as_str()?.to_string();

    let mut params = Vec::new();
    let return_type = if let Some(sig) = item.get("sig").and_then(|s| s.as_object()) {
        if let Some(decl) = sig.get("decl").and_then(|d| d.as_object()) {
            if let Some(inputs) = decl.get("inputs").and_then(|i| i.as_array()) {
                for input in inputs {
                    if let Some(input_obj) = input.as_object() {
                        if let Some(ty) = input_obj.get("type") {
                            params.push(parse_rust_type(ty).unwrap_or(RustTypeRef::Opaque));
                        } else {
                            params.push(RustTypeRef::Opaque);
                        }
                    } else {
                        params.push(RustTypeRef::Opaque);
                    }
                }
            }
            let ret_ty = if let Some(output) = decl.get("output") {
                parse_rust_type(output).unwrap_or(RustTypeRef::Opaque)
            } else {
                RustTypeRef::Unit
            };
            Some(ret_ty)
        } else {
            None
        }
    } else {
        None
    };

    let is_async = item
        .get("sig")
        .and_then(|s| s.as_object())
        .and_then(|s| s.get("asyncness"))
        .is_some();

    let doc = item
        .get("docs")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    let path = RustPath {
        segments: path_segments.to_vec(),
    };

    Some(PublicItem::Function {
        sig: FnSig {
            name,
            params,
            return_type,
            is_async,
        },
        path,
        doc,
    })
}

fn parse_rust_type(ty_value: &serde_json::Value) -> Option<RustTypeRef> {
    if let Some(ty_str) = ty_value.as_str() {
        match ty_str {
            "()" => return Some(RustTypeRef::Unit),
            "bool" => return Some(RustTypeRef::Bool),
            "i32" => return Some(RustTypeRef::I32),
            "i64" => return Some(RustTypeRef::I64),
            "f64" => return Some(RustTypeRef::F64),
            "f32" => return Some(RustTypeRef::F64),
            "&str" | "str" | "String" => return Some(RustTypeRef::Str),
            _ => {}
        }
    }

    if let Some(obj) = ty_value.as_object() {
        if let Some(ty_name) = obj.get("name").and_then(|n| n.as_str()) {
            if ty_name == "Option" {
                if let Some(inner) = obj
                    .get("inner")
                    .and_then(|i| i.as_array())
                    .and_then(|a| a.first())
                {
                    return Some(RustTypeRef::Option {
                        inner: Box::new(parse_rust_type(inner).unwrap_or(RustTypeRef::Opaque)),
                    });
                }
            }
            if ty_name == "Result" {
                if let Some(inner) = obj.get("inner").and_then(|i| i.as_array()) {
                    let ok = inner
                        .get(0)
                        .and_then(|t| parse_rust_type(t))
                        .unwrap_or(RustTypeRef::Opaque);
                    let err = inner
                        .get(1)
                        .and_then(|t| parse_rust_type(t))
                        .unwrap_or(RustTypeRef::Opaque);
                    return Some(RustTypeRef::Result {
                        ok: Box::new(ok),
                        err: Box::new(err),
                    });
                }
            }
            if ty_name == "Future" {
                if let Some(inner) = obj
                    .get("inner")
                    .and_then(|i| i.as_array())
                    .and_then(|a| a.first())
                {
                    return Some(RustTypeRef::Future {
                        output: Box::new(parse_rust_type(inner).unwrap_or(RustTypeRef::Opaque)),
                    });
                }
            }
        }
    }

    Some(RustTypeRef::Opaque)
}

#[allow(dead_code)]
fn fq_path(parts: &[&str]) -> RustPath {
    RustPath {
        segments: parts.iter().map(|s| s.to_string()).collect(),
    }
}

#[allow(dead_code)]
fn fn_item(
    path: RustPath,
    name: &str,
    params: Vec<RustTypeRef>,
    return_type: Option<RustTypeRef>,
    is_async: bool,
    doc: Option<String>,
) -> PublicItem {
    PublicItem::Function {
        sig: FnSig {
            name: name.to_string(),
            params,
            return_type,
            is_async,
        },
        path,
        doc,
    }
}
