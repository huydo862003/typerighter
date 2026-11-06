use crate::db::TypedownDatabase;
use crate::db::types::{TdBoundVariableTyp, TdFreeVariableTyp, TdTypEnum};

// Unwrap<T>: strip nullable and list wrappers to extract the underlying schema type
// null -> never (no element)
// Schema -> Schema (identity)
// Schema | null -> Schema (strip nullable)
// list[Schema] -> Schema (unwrap list)
// list[Schema] | null -> Schema (strip nullable, unwrap list)
pub fn resolve_unwrap<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
) -> Option<TdTypEnum<'db>> {
  match typ {
    // Already a concrete non-wrapper type: identity
    TdTypEnum::TdSchemaTyp(_)
    | TdTypEnum::TdStringTyp(_)
    | TdTypEnum::TdNumberTyp(_)
    | TdTypEnum::TdBoolTyp(_)
    | TdTypEnum::TdMathTyp(_)
    | TdTypEnum::TdIconTyp(_)
    | TdTypEnum::TdDateTimeTyp(_)
    | TdTypEnum::TdDateTyp(_)
    | TdTypEnum::TdTimeTyp(_)
    | TdTypEnum::TdBlobTyp(_)
    | TdTypEnum::TdProductTyp(_)
    | TdTypEnum::TdObjTyp(_) => Some(typ.clone()),

    // list[T] -> unwrap element
    TdTypEnum::TdListTyp(list) => {
      let element = list.element(db)?.resolve(db)?;
      resolve_unwrap(db, &element)
    }

    // T | null -> strip null members, unwrap the rest
    TdTypEnum::TdSumTyp(sum) => {
      let non_null: Vec<TdTypEnum<'db>> = sum
        .members(db)
        .iter()
        .filter_map(|member| member.resolve(db))
        .filter(|member| member.as_td_null_typ().is_none())
        .collect();
      match non_null.len() {
        0 => None,
        1 => resolve_unwrap(db, &non_null[0]),
        _ => Some(typ.clone()),
      }
    }

    // null alone -> no element
    TdTypEnum::TdNullTyp(_) => None,

    // Computed types: try resolving first, then unwrap the result
    TdTypEnum::TdComputedTyp(computed) => {
      let resolved = computed.try_resolve(db)?;
      resolve_unwrap(db, &resolved)
    }

    TdTypEnum::TdMetatypeInstanceTyp(instance) => {
      let resolved = instance.resolve_to_typ(db)?;
      resolve_unwrap(db, &resolved)
    }

    // Bound/free variables: best-effort reduction using upper bound
    TdTypEnum::TdBoundVariableTyp(bound) => TdBoundVariableTyp::upper_bound(*bound, db)
      .resolve(db)
      .and_then(|upper| resolve_unwrap(db, &upper)),
    TdTypEnum::TdFreeVariableTyp(free) => {
      TdBoundVariableTyp::upper_bound(TdFreeVariableTyp::bound_origin(*free, db), db)
        .resolve(db)
        .and_then(|upper| resolve_unwrap(db, &upper))
    }

    // Everything else: can't unwrap
    _ => None,
  }
}
