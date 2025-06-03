use super::{TypeGeneric, UnionTypeGeneric};

impl<T: std::fmt::Debug + Default> UnionTypeGeneric<T> {
    // disallow construction so people have to use:
    // FieldType::union(vec![...]) which does a simplify() default
    fn new(types: Vec<TypeGeneric<T>>) -> Self {
        Self {
            types,
            null_type: Box::new(TypeGeneric::null()),
        }
    }

    pub(crate) fn new_unsafe(types: Vec<TypeGeneric<T>>) -> Self {
        if types.iter().all(|t| t.is_null()) {
            panic!("FATAL, please report this bug: Union type must have at least one non-null type. Got {:?}", types);
        }
        Self {
            types,
            null_type: Box::new(TypeGeneric::null()),
        }
    }
}

impl<T: std::fmt::Debug + Default + Clone + Eq + std::hash::Hash> TypeGeneric<T> {
    pub fn union(choices: Vec<TypeGeneric<T>>) -> TypeGeneric<T> {
        TypeGeneric::Union(UnionTypeGeneric::new(choices), T::default()).simplify()
    }

    pub fn union_with_meta(choices: Vec<TypeGeneric<T>>, meta: T) -> TypeGeneric<T> {
        TypeGeneric::Union(UnionTypeGeneric::new_unsafe(choices), meta).simplify()
    }
}
