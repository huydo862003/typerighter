use crate::DepId;

use super::{DerivedFieldIngredient, DerivedQueryIngredient, InputIngredient, InternedIngredient};

/// Factory callbacks for each ingredient kind
pub type InputFactory = fn(DepId) -> Box<dyn InputIngredient>;
pub type InternedFactory = fn(DepId) -> Box<dyn InternedIngredient>;
pub type QueryFactory = fn(DepId) -> Box<dyn DerivedQueryIngredient>;
pub type FieldFactory = fn(DepId) -> Box<dyn DerivedFieldIngredient>;

pub struct InputInventory {
  pub register: fn(&mut Vec<InputFactory>),
}

pub struct InternedInventory {
  pub register: fn(&mut Vec<InternedFactory>),
}

pub struct QueryInventory {
  pub register: fn(&mut Vec<QueryFactory>),
}

pub struct FieldInventory {
  pub register: fn(&mut Vec<FieldFactory>),
}

inventory::collect!(InputInventory);
inventory::collect!(InternedInventory);
inventory::collect!(QueryInventory);
inventory::collect!(FieldInventory);
