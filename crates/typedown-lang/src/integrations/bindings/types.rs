//! Typed serialization output for the bindings layer
//!
//! These structs define the exact JSON shape produced by `serialize_value`
//! and can be used with ts-rs to auto-generate TypeScript definitions

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A finite f64 value (guaranteed not NaN or Inf)
#[derive(Debug, Clone, Copy)]
pub struct FiniteF64(f64);

impl FiniteF64 {
  /// Create a new FiniteF64, panics if value is non-finite
  pub fn new(value: f64) -> Self {
    debug_assert!(value.is_finite(), "FiniteF64 must be finite, got {value}");
    Self(value)
  }

  pub fn value(self) -> f64 {
    self.0
  }
}

impl Serialize for FiniteF64 {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64(self.0)
  }
}

impl<'de> Deserialize<'de> for FiniteF64 {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let value = f64::deserialize(deserializer)?;
    if !value.is_finite() {
      return Err(serde::de::Error::custom("expected finite f64"));
    }
    Ok(Self(value))
  }
}

/// A serialized Typedown value - the output of `serialize_value`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TdValue {
  Null,
  Bool(bool),
  Number(FiniteF64),
  String(String),
  Array(Vec<TdValue>),
  Tagged(TdTaggedValue),
}

/// A $kind-tagged value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$kind")]
pub enum TdTaggedValue {
  // Non-finite numbers
  #[serde(rename = "nan")]
  Nan,
  #[serde(rename = "inf")]
  Inf,
  #[serde(rename = "-inf")]
  NegInf,

  // Typed strings
  #[serde(rename = "math")]
  Math { value: String },
  #[serde(rename = "date")]
  Date { value: String },
  #[serde(rename = "datetime")]
  DateTime { value: String },
  #[serde(rename = "time")]
  Time { value: String },

  // Structured objects
  #[serde(rename = "icon")]
  Icon { name: String, lucide_name: String },
  #[serde(rename = "blob")]
  Blob { format: String, handle: TdHandle },
  #[serde(rename = "schema")]
  Schema {
    schema: String,
    fields: BTreeMap<String, TdValue>,
  },
  #[serde(rename = "product")]
  Product { fields: BTreeMap<String, TdValue> },
  #[serde(rename = "dict")]
  Dict { entries: BTreeMap<String, TdValue> },

  // File reference (nested file-scoped objects)
  #[serde(rename = "ref")]
  Ref(TdRef),

  // Type descriptors
  #[serde(rename = "type:null")]
  TypeNull,
  #[serde(rename = "type:bool")]
  TypeBool,
  #[serde(rename = "type:number")]
  TypeNumber,
  #[serde(rename = "type:string")]
  TypeString,
  #[serde(rename = "type:math")]
  TypeMath,
  #[serde(rename = "type:date")]
  TypeDate,
  #[serde(rename = "type:datetime")]
  TypeDateTime,
  #[serde(rename = "type:time")]
  TypeTime,
  #[serde(rename = "type:array")]
  TypeArray {
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<TdValue>>,
  },
  #[serde(rename = "type:dict")]
  TypeDict {
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Box<TdValue>>,
  },
  #[serde(rename = "type:union")]
  TypeUnion { members: Vec<TdValue> },
  #[serde(rename = "type:schema")]
  TypeSchema {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<BTreeMap<String, TdValue>>,
  },
  #[serde(rename = "type:product")]
  TypeProduct {
    name: Option<String>,
    fields: BTreeMap<String, TdValue>,
  },
  #[serde(rename = "type:unknown")]
  TypeUnknown,
}

/// Resolved file reference for nested file-scoped objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdRef {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  #[serde(rename = "type")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub type_name: Option<String>,
  #[serde(rename = "$ref")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolved: Option<TdResolvedLink>,
}

/// Resolved link target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdResolvedLink {
  pub url: String,
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<TdRefIcon>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<String>,
}

/// Icon in a resolved reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdRefIcon {
  pub name: String,
}

/// Serialized file handle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TdHandle {
  #[serde(rename = "path")]
  Path {
    path: String,
    metadata: TdHandleMetadata,
  },
  #[serde(rename = "content")]
  Content {
    path: String,
    content: String,
    metadata: TdHandleMetadata,
  },
}

/// File handle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TdHandleMetadata {
  pub mtime: u64,
  pub ctime: u64,
}
