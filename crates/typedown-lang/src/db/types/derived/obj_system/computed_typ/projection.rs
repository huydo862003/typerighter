use crate::db::TypedownDatabase;
use crate::db::types::{LitValue, TdBoundVariableTyp, TdFreeVariableTyp, TdTypEnum};

use super::super::TdStaticTyp;

// Projection<T, K>: look up field K on type T
// K must be lit<"field_name"> for resolution
// If T is a variable, best-effort reduction using upper bound
pub fn resolve_projection<'db>(
  db: &'db TypedownDatabase,
  object_typ: &TdTypEnum<'db>,
  key_typ: &TdTypEnum<'db>,
) -> Option<TdTypEnum<'db>> {
  let key = match key_typ {
    TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
      LitValue::String(string) => string,
      _ => return None,
    },
    _ => return None,
  };

  // Try direct lookup first
  if let Some(result) = object_typ.lookup_field_typ(db, &key) {
    return Some(result);
  }

  // Best-effort: if object is a variable, try its upper bound
  match object_typ {
    TdTypEnum::TdBoundVariableTyp(bound) => TdBoundVariableTyp::upper_bound(*bound, db)
      .resolve(db)?
      .lookup_field_typ(db, &key),
    TdTypEnum::TdFreeVariableTyp(free) => {
      TdBoundVariableTyp::upper_bound(TdFreeVariableTyp::bound_origin(*free, db), db)
        .resolve(db)?
        .lookup_field_typ(db, &key)
    }
    _ => None,
  }
}
