use crate::db::TypedownDatabase;
use crate::db::types::{LitValue, TdObjEnum, TdRuntimeObj, TdTypEnum};

use super::is_subtyp_of;

/// Check if object `obj` is an instance of type `typ`
pub fn is_instance_of<'db>(
  db: &'db TypedownDatabase,
  obj: &TdObjEnum<'db>,
  typ: &TdTypEnum<'db>,
) -> bool {
  match typ {
    // Everything is an instance of obj
    TdTypEnum::TdObjTyp(_) => true,
    // Only types are instances of type
    TdTypEnum::TdTypTyp(_) => obj.is_td_typ_obj(),
    // Nothing is an instance of never
    TdTypEnum::TdNeverTyp(_) => false,

    // Primitive type checks
    TdTypEnum::TdNullTyp(_) => obj.is_td_null_obj(),
    TdTypEnum::TdBoolTyp(_) => obj.is_td_bool_obj(),
    TdTypEnum::TdNumberTyp(_) => obj.is_td_number_obj(),
    TdTypEnum::TdMathTyp(_) => obj.is_td_math_obj(),
    TdTypEnum::TdIconTyp(_) => obj.is_td_icon_obj(),
    // String subsumes datetime, date, time, blob at runtime
    TdTypEnum::TdStringTyp(_) => {
      obj.is_td_string_obj()
        || obj.is_td_date_time_obj()
        || obj.is_td_date_obj()
        || obj.is_td_time_obj()
        || obj.is_td_blob_obj()
    }
    TdTypEnum::TdDateTimeTyp(_) => obj.is_td_date_time_obj(),
    TdTypEnum::TdDateTyp(_) => obj.is_td_date_obj(),
    TdTypEnum::TdTimeTyp(_) => obj.is_td_time_obj(),
    TdTypEnum::TdBlobTyp(_) => obj.is_td_blob_obj() || obj.is_td_string_obj(),

    // Compound types: delegate to subtyping for structural/parametric checks
    TdTypEnum::TdFuncTyp(_)
    | TdTypEnum::TdListTyp(_)
    | TdTypEnum::TdDictTyp(_)
    | TdTypEnum::TdProductTyp(_)
    | TdTypEnum::TdSchemaTyp(_) => is_subtyp_of(db, &obj.get_typ(db), typ),

    // obj is a schema type value (e.g. Person is an instance of schema_meta)
    TdTypEnum::TdSchemaMetaTyp(_) => match obj.as_td_typ_obj() {
      Some(TdTypEnum::TdSchemaTyp(_)) => true,
      Some(TdTypEnum::TdLitTyp(lit)) => {
        matches!(lit.underlying_typ(db), TdTypEnum::TdSchemaMetaTyp(_))
      }
      _ => false,
    },

    // Instance of a sum if instance of any member
    TdTypEnum::TdSumTyp(sum) => sum
      .members(db)
      .iter()
      .filter_map(|member| member.resolve(db))
      .any(|member| is_instance_of(db, obj, &member)),

    // Singleton: obj value must match the literal exactly
    TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
      LitValue::String(string) => obj
        .as_td_string_obj()
        .is_some_and(|obj| obj.value(db) == string),
      LitValue::Number(number) => obj
        .as_td_number_obj()
        .is_some_and(|obj| obj.value(db).to_string() == number),
      LitValue::Bool(bool) => obj
        .as_td_bool_obj()
        .is_some_and(|obj| obj.value(db) == bool),
      LitValue::Typ(typ) => obj.as_td_typ_obj().is_some_and(|obj| obj == &typ),
    },

    // metatype_instance<T>: inner must be a metatype
    TdTypEnum::TdMetatypeInstanceTyp(instance_typ) => match &instance_typ.inner(db) {
      // metatype_instance<lit<X>>: unwrap to X
      TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
        LitValue::Typ(unwrapped) => is_instance_of(db, obj, &unwrapped),
        _ => is_instance_of(db, obj, &lit.underlying_typ(db)),
      },
      // metatype_instance<schema_meta>: obj must be a schema object
      TdTypEnum::TdSchemaMetaTyp(_) => obj.is_td_schema_obj(),
      // metatype_instance<type>: all values are instances of some type
      TdTypEnum::TdTypTyp(_) => true,
      // metatype_instance<obj>: everything
      TdTypEnum::TdObjTyp(_) => true,
      // metatype_instance<never>: nothing
      TdTypEnum::TdNeverTyp(_) => false,
      // metatype_instance<instance<T>>: recurse into inner
      TdTypEnum::TdMetatypeInstanceTyp(_) => is_instance_of(db, obj, &instance_typ.inner(db)),
      // Cannot resolve variables statically
      TdTypEnum::TdVariableTyp(_) | TdTypEnum::TdExistentialTyp(_) => {
        is_subtyp_of(db, &obj.get_typ(db), typ)
      }
      // metatype_instance of non-metatype is never, nothing is an instance
      _ => false,
    },

    // Delegate to subtyping for type variables
    TdTypEnum::TdVariableTyp(_) | TdTypEnum::TdExistentialTyp(_) => {
      is_subtyp_of(db, &obj.get_typ(db), typ)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
  use crate::db::derived::get_builtin_typs::{
    get_lit_typ, get_never_typ, get_null_typ, get_number_typ, get_obj_typ, get_schema_meta_typ,
    get_typ_typ,
  };
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::{load_vault_fixture, make_number_obj};
  use crate::db::types::{TdMetatypeInstanceTyp, TdNullObj};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  /* Primitive instance checks */

  #[test]
  fn null_is_instance_of_null() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    assert!(is_instance_of(&db, &obj, &get_null_typ(&db).into()));
  }

  #[test]
  fn null_not_instance_of_number() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    assert!(!is_instance_of(&db, &obj, &get_number_typ(&db).into()));
  }

  #[test]
  fn number_is_instance_of_number() {
    let db = make_db();
    let obj: TdObjEnum = make_number_obj(&db, 42.0_f64.to_bits()).into();
    assert!(is_instance_of(&db, &obj, &get_number_typ(&db).into()));
  }

  #[test]
  fn everything_is_instance_of_obj() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    assert!(is_instance_of(&db, &obj, &get_obj_typ(&db).into()));
  }

  #[test]
  fn nothing_is_instance_of_never() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    assert!(!is_instance_of(&db, &obj, &get_never_typ(&db).into()));
  }

  /* Type-as-value instance checks */

  #[test]
  fn type_is_instance_of_type() {
    let db = make_db();
    let obj: TdObjEnum = get_number_typ(&db).into();
    assert!(is_instance_of(&db, &obj, &get_typ_typ(&db).into()));
  }

  #[test]
  fn non_type_not_instance_of_type() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    assert!(!is_instance_of(&db, &obj, &get_typ_typ(&db).into()));
  }

  /* Schema meta instance checks */

  #[test]
  fn schema_typ_is_instance_of_schema_meta() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();
    let obj: TdObjEnum = person_typ.into();
    assert!(is_instance_of(&db, &obj, &get_schema_meta_typ(&db).into()));
  }

  #[test]
  fn lit_schema_is_instance_of_schema_meta() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();
    let lit_person = get_lit_typ(&db, LitValue::Typ(person_typ));
    let obj: TdObjEnum = lit_person.into();
    assert!(is_instance_of(&db, &obj, &get_schema_meta_typ(&db).into()));
  }

  #[test]
  fn number_not_instance_of_schema_meta() {
    let db = make_db();
    let obj: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    assert!(!is_instance_of(&db, &obj, &get_schema_meta_typ(&db).into()));
  }

  /* InstanceType checks */

  #[test]
  fn schema_obj_is_instance_of_instance_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();

    let lit_person: TdTypEnum = get_lit_typ(&db, LitValue::Typ(person_typ)).into();
    let instance_typ: TdTypEnum = TdMetatypeInstanceTyp::new(&db, lit_person).into();

    let person_file = *project
      .files(&db)
      .iter()
      .find(|(p, _)| p.ends_with("valid_person.td"))
      .unwrap()
      .1;
    let person_symbol = file_symbol(&db, project, person_file).value(&db).unwrap();
    let person_obj = evaluate_resource(&db, person_symbol).value(&db).unwrap();

    assert!(is_instance_of(&db, &person_obj, &instance_typ));
  }

  #[test]
  fn number_not_instance_of_instance_typ() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();

    let lit_person: TdTypEnum = get_lit_typ(&db, LitValue::Typ(person_typ)).into();
    let instance_typ: TdTypEnum = TdMetatypeInstanceTyp::new(&db, lit_person).into();

    let obj: TdObjEnum = make_number_obj(&db, 42.0_f64.to_bits()).into();
    assert!(!is_instance_of(&db, &obj, &instance_typ));
  }

  /* instance<lit<P>> and P are equivalent via subtyping */

  #[test]
  fn instance_lit_schema_subtype_of_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();

    let lit_person: TdTypEnum = get_lit_typ(&db, LitValue::Typ(person_typ.clone())).into();
    let instance_typ: TdTypEnum = TdMetatypeInstanceTyp::new(&db, lit_person).into();

    assert!(is_subtyp_of(&db, &instance_typ, &person_typ));
  }

  #[test]
  fn schema_subtype_of_instance_lit_schema() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let symbol = file_symbol(&db, project, file).value(&db).unwrap();
    let person_typ = evaluate_typ(&db, symbol).typ(&db).unwrap();

    let lit_person: TdTypEnum = get_lit_typ(&db, LitValue::Typ(person_typ.clone())).into();
    let instance_typ: TdTypEnum = TdMetatypeInstanceTyp::new(&db, lit_person).into();

    assert!(is_subtyp_of(&db, &person_typ, &instance_typ));
  }

  /* instance<schema_meta> checks */

  #[test]
  fn schema_obj_is_instance_of_instance_schema_meta() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "_types/Person.td");
    let person_file = *project
      .files(&db)
      .iter()
      .find(|(p, _)| p.ends_with("valid_person.td"))
      .unwrap()
      .1;
    let person_symbol = file_symbol(&db, project, person_file).value(&db).unwrap();
    let person_obj = evaluate_resource(&db, person_symbol).value(&db).unwrap();

    let instance_schema_meta: TdTypEnum =
      TdMetatypeInstanceTyp::new(&db, get_schema_meta_typ(&db).into()).into();
    assert!(is_instance_of(&db, &person_obj, &instance_schema_meta));
  }

  #[test]
  fn number_not_instance_of_instance_schema_meta() {
    let db = make_db();
    let obj: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let instance_schema_meta: TdTypEnum =
      TdMetatypeInstanceTyp::new(&db, get_schema_meta_typ(&db).into()).into();
    assert!(!is_instance_of(&db, &obj, &instance_schema_meta));
  }

  /* instance<type> checks */

  #[test]
  fn anything_is_instance_of_instance_type() {
    let db = make_db();
    let obj: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let instance_type: TdTypEnum = TdMetatypeInstanceTyp::new(&db, get_typ_typ(&db).into()).into();
    assert!(is_instance_of(&db, &obj, &instance_type));
  }

  #[test]
  fn null_is_instance_of_instance_type() {
    let db = make_db();
    let obj: TdObjEnum = TdNullObj::get(&db).into();
    let instance_type: TdTypEnum = TdMetatypeInstanceTyp::new(&db, get_typ_typ(&db).into()).into();
    assert!(is_instance_of(&db, &obj, &instance_type));
  }

  /* instance<never> checks */

  #[test]
  fn nothing_is_instance_of_instance_never() {
    let db = make_db();
    let obj: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let instance_never: TdTypEnum =
      TdMetatypeInstanceTyp::new(&db, get_never_typ(&db).into()).into();
    assert!(!is_instance_of(&db, &obj, &instance_never));
  }
}
