//! Integration certificate binding admission owned by a proved predicate.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Borrow the first producer's value when every observed output is admissible.
pub fn agreed_output<'a, T, V: Fn(usize) -> bool, E: Fn(usize, usize) -> bool>(
    values: &[&'a T], valid: V, equal: E,
)
    -> (result: Option<&'a T>)
    requires
        forall|i: usize| i < values.len() ==> valid.requires((i,)),
        forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.requires((a, b)),
    ensures match result {
        Some(value) => values.len() > 0 && value == values@[0] &&
            (forall|i: usize| i < values.len() ==> valid.ensures((i,), true)) &&
            (forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.ensures((a, b), true)),
        None => values.len() == 0 ||
            (exists|i: usize| i < values.len() && valid.ensures((i,), false)) ||
            (exists|a: usize, b: usize| a + 1 == b && b < values.len() && #[trigger] equal.ensures((a, b), false)),
    },
{
    if values.len() == 0 { return None; }
    let mut i = 0;
    while i < values.len()
        invariant i <= values.len(),
            forall|k: usize| k < values.len() ==> valid.requires((k,)),
            forall|a: usize, b: usize| a + 1 == b && b < values.len() ==> #[trigger] equal.requires((a, b)),
            forall|k: usize| k < i ==> valid.ensures((k,), true),
            forall|a: usize, b: usize| a + 1 == b && b < i ==> #[trigger] equal.ensures((a, b), true),
        decreases values.len() - i,
    {
        if !valid(i) || (i > 0 && !equal(i - 1, i)) { return None; }
        i += 1;
    }
    Some(values[0])
}

/// A certificate binding is admissible only when its cardinality and exact
/// field-set checks both hold in the concrete adapter.
pub open spec fn binding_admissible(
    field_count: bool,
    exact_fields: bool,
) -> bool {
    field_count && exact_fields
}

/// Consume the adapted binding facts through the proved certificate rule.
pub fn admit_binding(
    field_count: bool,
    exact_fields: bool,
) -> (admitted: bool)
    ensures admitted == binding_admissible(field_count, exact_fields),
{
    field_count && exact_fields
}

}
