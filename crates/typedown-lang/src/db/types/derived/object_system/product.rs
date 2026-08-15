use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use typedown_incremental::Id;
use typedown_types::either::Either;

use crate::db::types::{HirValue, InstResult, LazyType, MemberType, Symbol, TypeMember};

#[query_derived]
pub struct TdProductType {
  pub name: Option<String>,
  pub metatype: TdTypeEnum,
  pub supertype: Option<TdTypeEnum>,
  pub fields: HashMap<String, TypeMember>,
  pub vtable: HashMap<String, TdFuncObj>,
}

impl TdObjectLike for TdProductType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self.metatype(db)
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl TdTypeLike for TdProductType {
  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self
      .supertype(db)
      .unwrap_or_else(|| TdObjectType::get(db).into())
  }
  fn get_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    self.vtable(db)
  }
  fn get_owned_field_type_member(&self, db: &TypedownDatabase, name: &str) -> Option<TypeMember> {
    self.fields(db).get(name).cloned()
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn accepts(&self, _db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let dict = arg.as_td_dict_obj()?;
    let fields = dict.entries(db);
    Some(TdProductObj::new(db, (*self).into(), None, fields).into())
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    if let Some(name) = self.name(db) {
      return name;
    }
    // Structural fallback for anonymous product types
    let fields = self.fields(db);
    if fields.is_empty() {
      return "{}".to_string();
    }
    let mut parts: Vec<String> = fields
      .iter()
      .map(|(name, member)| {
        let type_name = member_type_display_name(db, &member.typ(db));
        format!("{}: {}", name, type_name)
      })
      .collect();
    parts.sort();
    format!("{{ {} }}", parts.join(", "))
  }
}

pub fn member_type_display_name(db: &TypedownDatabase, member: &MemberType) -> String {
  match member {
    MemberType::Simple(lazy) => match lazy.resolve(db) {
      Some(typ) => typ.display_name(db),
      None => "?".to_string(),
    },
    MemberType::Sum(members) => members
      .iter()
      .map(|m| member_type_display_name(db, &m.typ(db)))
      .collect::<Vec<_>>()
      .join(" | "),
    MemberType::ListOfSum(members) => {
      let inner = members
        .iter()
        .map(|m| member_type_display_name(db, &m.typ(db)))
        .collect::<Vec<_>>()
        .join(" | ");
      format!("list[{}]", inner)
    }
    MemberType::DictOfSum(members) => {
      let inner = members
        .iter()
        .map(|m| member_type_display_name(db, &m.typ(db)))
        .collect::<Vec<_>>()
        .join(" | ");
      format!("dict[{}]", inner)
    }
    MemberType::Literal(val) => format!("{:?}", val),
    MemberType::Structural(fields) => {
      if fields.is_empty() {
        return "{}".to_string();
      }
      let mut parts: Vec<String> = fields
        .iter()
        .map(|(name, member)| {
          let type_name = member_type_display_name(db, &member.typ(db));
          format!("{}: {}", name, type_name)
        })
        .collect();
      parts.sort();
      format!("{{ {} }}", parts.join(", "))
    }
    MemberType::Never => "never".to_string(),
  }
}

#[query_derived]
pub struct TdProductObj {
  pub schema: TdTypeEnum,
  pub file_symbol: Option<Symbol>,
  pub fields: HashMap<String, Either<HirValue, TdObjectEnum>>,
}

impl TdObjectLike for TdProductObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self.schema(db)
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match self.fields(db).get(key).cloned()? {
      Either::Left(hir) => evaluate_node(db, hir).value(db),
      Either::Right(obj) => Some(obj),
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
