//! Expected Type Set (Bounded Beam)
//!
//! Represents uncertainty about which type we're parsing as a bounded set of candidates.
//! Uses beam search to avoid exponential AnyOf blowup.

use super::schema_index::{SchemaIndex, TypeId, TypeKind};

/// Maximum candidates to track (beam width)
pub const DEFAULT_BEAM_K: usize = 8;

/// Default score gap for auto-collapse
pub const DEFAULT_COLLAPSE_GAP: i32 = 20;

/// A candidate type interpretation with its score
#[derive(Debug, Clone)]
pub struct Candidate {
    pub type_id: TypeId,
    pub score: i32,
    pub dead: bool, // Eliminated by hard evidence
}

/// Bounded set of possible types at a parse position
#[derive(Debug, Clone)]
pub struct ExpectedTypeSet {
    candidates: Vec<Candidate>,
    max_k: usize,
}

impl ExpectedTypeSet {
    /// Create a set with a single known type
    pub fn single(type_id: TypeId) -> Self {
        ExpectedTypeSet {
            candidates: vec![Candidate {
                type_id,
                score: 0,
                dead: false,
            }],
            max_k: DEFAULT_BEAM_K,
        }
    }

    /// Create a set from union variants
    pub fn from_union(variant_ids: &[TypeId], max_k: usize) -> Self {
        let candidates = variant_ids
            .iter()
            .map(|&id| Candidate {
                type_id: id,
                score: 0,
                dead: false,
            })
            .collect();
        ExpectedTypeSet { candidates, max_k }
    }

    /// Create an empty set (represents impossible parse state)
    pub fn empty() -> Self {
        ExpectedTypeSet {
            candidates: Vec::new(),
            max_k: DEFAULT_BEAM_K,
        }
    }

    /// Check if the set is empty (all candidates eliminated)
    pub fn is_empty(&self) -> bool {
        self.candidates.iter().all(|c| c.dead)
    }

    /// Check if the set has a single resolved type
    pub fn is_resolved(&self) -> bool {
        self.candidates.iter().filter(|c| !c.dead).count() == 1
    }

    /// Get the best (highest scoring, non-dead) candidate
    pub fn best(&self) -> Option<TypeId> {
        self.candidates
            .iter()
            .filter(|c| !c.dead)
            .max_by_key(|c| c.score)
            .map(|c| c.type_id)
    }

    /// Get all active (non-dead) candidates
    pub fn all_candidates(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.candidates.iter().filter(|c| !c.dead).map(|c| c.type_id)
    }

    /// Get number of active candidates
    pub fn active_count(&self) -> usize {
        self.candidates.iter().filter(|c| !c.dead).count()
    }

    /// Narrow by structural token (saw `{` or `[`)
    ///
    /// - `saw_brace=true`: we saw `{`, keep only object-like types
    /// - `saw_brace=false`: we saw `[`, keep only array-like types
    pub fn narrow_by_structure(&mut self, schema: &SchemaIndex, saw_brace: bool) {
        for cand in &mut self.candidates {
            if cand.dead {
                continue;
            }

            let info = schema.get(cand.type_id);
            let is_compatible = match (saw_brace, info.map(|i| &i.kind)) {
                (true, Some(TypeKind::Class { .. })) => true,
                (true, Some(TypeKind::Map { .. })) => true,
                (false, Some(TypeKind::List { .. })) => true,
                (false, Some(TypeKind::Tuple { .. })) => true,
                // Optional types: check inner type
                (b, Some(TypeKind::Optional { inner })) => {
                    let inner_info = schema.get(*inner);
                    match (b, inner_info.map(|i| &i.kind)) {
                        (true, Some(TypeKind::Class { .. })) => true,
                        (true, Some(TypeKind::Map { .. })) => true,
                        (false, Some(TypeKind::List { .. })) => true,
                        (false, Some(TypeKind::Tuple { .. })) => true,
                        _ => false,
                    }
                }
                // Union types: at least one variant must be compatible
                (b, Some(TypeKind::Union { variants, .. })) => variants.iter().any(|&v| {
                    let v_info = schema.get(v);
                    match (b, v_info.map(|i| &i.kind)) {
                        (true, Some(TypeKind::Class { .. })) => true,
                        (true, Some(TypeKind::Map { .. })) => true,
                        (false, Some(TypeKind::List { .. })) => true,
                        (false, Some(TypeKind::Tuple { .. })) => true,
                        _ => false,
                    }
                }),
                _ => false,
            };

            if !is_compatible {
                cand.dead = true;
            }
        }
        self.prune_dead();
    }

    /// Score by observed key (soft narrowing during streaming)
    ///
    /// - If `streaming=true`: soft penalty for missing key (don't hard eliminate)
    /// - If `streaming=false`: hard eliminate candidates without the key
    pub fn observe_key(&mut self, schema: &SchemaIndex, key: &str, streaming: bool) {
        for cand in &mut self.candidates {
            if cand.dead {
                continue;
            }

            let has_key = type_has_key(schema, cand.type_id, key);

            if has_key {
                cand.score += 10; // Boost for matching key
            } else if streaming {
                cand.score -= 5; // Soft penalty during streaming
            } else {
                cand.dead = true; // Hard eliminate when not streaming
            }
        }

        self.prune_dead();
        self.keep_top_k();
    }

    /// After successful field value parse
    pub fn observe_value_success(&mut self, type_id: TypeId) {
        for cand in &mut self.candidates {
            if cand.type_id == type_id {
                cand.score += 5;
            }
        }
        self.keep_top_k();
    }

    /// After failed value parse for a type
    pub fn observe_value_failure(&mut self, type_id: TypeId) {
        for cand in &mut self.candidates {
            if cand.type_id == type_id {
                cand.score -= 10;
            }
        }
        self.keep_top_k();
    }

    /// Boost candidates that match primitive value structure
    pub fn narrow_by_primitive_value(&mut self, schema: &SchemaIndex, is_string: bool, is_number: bool, is_bool: bool, is_null: bool) {
        for cand in &mut self.candidates {
            if cand.dead {
                continue;
            }

            let matches = match schema.get(cand.type_id).map(|i| &i.kind) {
                Some(TypeKind::Primitive(p)) => {
                    use super::schema_index::PrimitiveKind;
                    match p {
                        PrimitiveKind::String => is_string,
                        PrimitiveKind::Int | PrimitiveKind::Float => is_number,
                        PrimitiveKind::Bool => is_bool,
                        PrimitiveKind::Null => is_null,
                        PrimitiveKind::Media => false,
                    }
                }
                Some(TypeKind::Enum { .. }) => is_string,
                Some(TypeKind::Literal(lit)) => {
                    use super::schema_index::LiteralKind;
                    match lit {
                        LiteralKind::String(_) => is_string,
                        LiteralKind::Int(_) => is_number,
                        LiteralKind::Bool(_) => is_bool,
                    }
                }
                Some(TypeKind::Optional { inner }) => {
                    // Check if null or if inner type matches
                    if is_null {
                        true
                    } else {
                        // Recursively check inner type
                        let mut inner_set = ExpectedTypeSet::single(*inner);
                        inner_set.narrow_by_primitive_value(schema, is_string, is_number, is_bool, is_null);
                        !inner_set.is_empty()
                    }
                }
                Some(TypeKind::Union { variants, .. }) => {
                    // At least one variant should match
                    variants.iter().any(|&v| {
                        let mut var_set = ExpectedTypeSet::single(v);
                        var_set.narrow_by_primitive_value(schema, is_string, is_number, is_bool, is_null);
                        !var_set.is_empty()
                    })
                }
                _ => false,
            };

            if !matches {
                cand.score -= 20;
            } else {
                cand.score += 5;
            }
        }
        self.keep_top_k();
    }

    /// Collapse to single best if gap is large enough
    pub fn maybe_collapse(&mut self, gap_threshold: i32) {
        if self.candidates.len() < 2 {
            return;
        }

        self.candidates.sort_by_key(|c| std::cmp::Reverse(c.score));

        let alive: Vec<_> = self.candidates.iter().filter(|c| !c.dead).collect();
        if alive.len() < 2 {
            return;
        }

        let gap = alive[0].score - alive[1].score;
        if gap >= gap_threshold {
            // Keep only the best candidate
            let best_id = alive[0].type_id;
            for cand in &mut self.candidates {
                if cand.type_id != best_id {
                    cand.dead = true;
                }
            }
            self.prune_dead();
        }
    }

    /// Remove dead candidates
    fn prune_dead(&mut self) {
        self.candidates.retain(|c| !c.dead);
    }

    /// Keep only top K candidates by score
    fn keep_top_k(&mut self) {
        if self.candidates.len() <= self.max_k {
            return;
        }

        self.candidates.sort_by_key(|c| std::cmp::Reverse(c.score));
        self.candidates.truncate(self.max_k);
    }

    /// Get the score gap between top two candidates
    pub fn score_gap(&self) -> Option<i32> {
        let alive: Vec<_> = self.candidates.iter().filter(|c| !c.dead).collect();
        if alive.len() < 2 {
            return None;
        }
        let mut scores: Vec<_> = alive.iter().map(|c| c.score).collect();
        scores.sort_by(|a, b| b.cmp(a));
        Some(scores[0] - scores[1])
    }
}

/// Check if a type has a given key (for classes and maps)
fn type_has_key(schema: &SchemaIndex, type_id: TypeId, key: &str) -> bool {
    match schema.get(type_id).map(|i| &i.kind) {
        Some(TypeKind::Class { fields, .. }) => {
            // Fields are keyed by rendered_name
            fields.contains_key(key)
        }
        Some(TypeKind::Map { .. }) => true, // Maps accept any key
        Some(TypeKind::Optional { inner }) => type_has_key(schema, *inner, key),
        Some(TypeKind::Union { variants, .. }) => {
            variants.iter().any(|&v| type_has_key(schema, v, key))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_type() {
        let set = ExpectedTypeSet::single(42);
        assert!(set.is_resolved());
        assert_eq!(set.best(), Some(42));
    }

    #[test]
    fn test_union_types() {
        let set = ExpectedTypeSet::from_union(&[1, 2, 3], 8);
        assert!(!set.is_resolved());
        assert_eq!(set.active_count(), 3);
    }

    #[test]
    fn test_empty_set() {
        let set = ExpectedTypeSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.best(), None);
    }

    #[test]
    fn test_score_collapse() {
        let mut set = ExpectedTypeSet::from_union(&[1, 2, 3], 8);

        // Boost candidate 1
        for _ in 0..5 {
            set.observe_value_success(1);
        }

        // Should collapse if gap is large enough
        set.maybe_collapse(20);

        // After collapse, should have single candidate
        assert!(set.is_resolved() || set.score_gap().map(|g| g < 20).unwrap_or(true));
    }

    #[test]
    fn test_keep_top_k() {
        let ids: Vec<_> = (0..20).collect();
        let mut set = ExpectedTypeSet::from_union(&ids, 5);

        // Assign different scores
        for (i, cand) in set.candidates.iter_mut().enumerate() {
            cand.score = i as i32;
        }

        set.keep_top_k();

        // Should only keep top 5
        assert_eq!(set.active_count(), 5);
    }
}
