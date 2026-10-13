
/// Longest a facet value may be. Long enough for a real preference, short enough
/// that nothing can smuggle a paragraph of instructions into every system prompt.
pub(super) const VALUE_MAX: usize = 140;
