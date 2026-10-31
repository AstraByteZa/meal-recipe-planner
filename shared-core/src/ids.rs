//! Deterministic ids (UUIDv5). Two devices that create the same logical row get the same id, so
//! the server merges them instead of storing duplicates. Changing the namespace or any name
//! format would orphan ids already on other devices, so the tests pin known values.

use uuid::Uuid;

/// Namespace for deterministic ids.
const ID_NAMESPACE: Uuid = Uuid::from_u128(0x6b1f_7a2e_9c4d_4f0a_8e3b_5d2c_1a9f_0e71);

/// Deterministic id for a tag. Tag names are case-insensitive, so the id uses the lowercase name.
pub fn tag_id(name: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("tag|{}", name.to_lowercase()).as_bytes()).to_string()
}

/// Deterministic id for a recipe-tag link.
pub fn tag_link_id(recipe_id: &str, tag_id: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("recipe_tag|{recipe_id}|{tag_id}").as_bytes()).to_string()
}

/// Deterministic id for a schedule slot. A slot is one row, however often it is set and cleared.
pub fn slot_id(date: &str, track_id: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("slot|{date}|{track_id}").as_bytes()).to_string()
}

/// Deterministic id for a config key.
pub fn config_id(key: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("config|{key}").as_bytes()).to_string()
}

/// Deterministic id for a dietary need.
pub fn diet_id(diet: &str) -> String {
    Uuid::new_v5(&ID_NAMESPACE, format!("diet|{diet}").as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These values were computed before the helpers moved here. If one changes, existing
    // devices would create new rows instead of matching the old ones.
    #[test]
    fn ids_match_the_values_already_in_use() {
        assert_eq!(tag_id("Breakfast"), "56789a96-b0e8-5a47-a2dc-b890ed3b5593");
        assert_eq!(
            tag_link_id("7f0c1e2a-0000-4000-8000-000000000001", "a1b2c3d4-0000-4000-8000-000000000002"),
            "4f314d77-6afb-5ef3-b0e2-a3258b3990d4"
        );
        assert_eq!(
            slot_id("2026-10-31", "00000000-0000-4000-8000-000000000003"),
            "95429df5-f91a-5545-a77d-80143b9d6269"
        );
        assert_eq!(config_id("cook-keep-awake"), "48877a60-7b0e-5aaa-9e35-6f19e6df4f03");
        assert_eq!(diet_id("vegan"), "46940cf6-eca8-5a84-a0dc-8abcbb1ec0a7");
    }

    #[test]
    fn tag_ids_ignore_case() {
        assert_eq!(tag_id("Breakfast"), tag_id("breakfast"));
        assert_ne!(tag_id("breakfast"), tag_id("lunch"));
    }
}
