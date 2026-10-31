use serde::{Deserialize, Serialize};

/// The synced tables. The snake_case name is the wire name and the `entity_type` column value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Track,
    Tag,
    Recipe,
    RecipeTag,
    ScheduleEntry,
    Settings,
    HouseholdDiet,
    Config,
}

impl EntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityType::Track => "track",
            EntityType::Tag => "tag",
            EntityType::Recipe => "recipe",
            EntityType::RecipeTag => "recipe_tag",
            EntityType::ScheduleEntry => "schedule_entry",
            EntityType::Settings => "settings",
            EntityType::HouseholdDiet => "household_diet",
            EntityType::Config => "config",
        }
    }

    /// Whether rows of this type can be shared with a family. Per-user settings, diets and
    /// config always stay private, so the server rejects them with a family id.
    pub fn is_family_shareable(self) -> bool {
        matches!(
            self,
            EntityType::Track | EntityType::Tag | EntityType::Recipe | EntityType::RecipeTag | EntityType::ScheduleEntry
        )
    }
}

/// What a change does to its row. Upserts carry the full row, and deletes are soft deletes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operation {
    Upsert,
    Delete,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Upsert => "UPSERT",
            Operation::Delete => "DELETE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_match_the_database_names() {
        let entities = [
            EntityType::Track,
            EntityType::Tag,
            EntityType::Recipe,
            EntityType::RecipeTag,
            EntityType::ScheduleEntry,
            EntityType::Settings,
            EntityType::HouseholdDiet,
            EntityType::Config,
        ];
        for entity in entities {
            let json = serde_json::to_string(&entity).unwrap();
            assert_eq!(json, format!("\"{}\"", entity.as_str()));
        }
        for operation in [Operation::Upsert, Operation::Delete] {
            let json = serde_json::to_string(&operation).unwrap();
            assert_eq!(json, format!("\"{}\"", operation.as_str()));
        }
    }
}
