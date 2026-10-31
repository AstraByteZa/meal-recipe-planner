use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A saved recipe as shown in the recipe library. The usage fields are derived in SQL.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SavedRecipe {
    pub id: String,
    pub name: String,
    pub source_domain: String,
    pub rating: Option<i64>,
    pub saved_days_ago: i64,
    pub last_used_days_ago: Option<i64>,
    pub planned_count: i64,
    #[sqlx(skip)]
    pub tags: Vec<String>,
}

/// One planned use of a recipe, for the plan history on the detail view.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    pub date: String,
    pub track_key: String,
    pub track_label: String,
}

/// A single recipe with everything the detail and edit views need.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDetail {
    pub id: String,
    pub name: String,
    pub url: String,
    pub source_domain: String,
    pub rating: Option<i64>,
    pub minutes: Option<i64>,
    pub servings: Option<i64>,
    pub notes: Option<String>,
    pub usual_track_key: Option<String>,
    pub image_url: Option<String>,
    #[sqlx(json)]
    pub ingredients: Vec<String>,
    #[sqlx(json)]
    pub instructions: Vec<String>,
    pub saved_days_ago: i64,
    pub last_used_days_ago: Option<i64>,
    pub planned_count: i64,
    #[sqlx(skip)]
    pub tags: Vec<String>,
    #[sqlx(skip)]
    pub history: Vec<PlanEntry>,
}

/// Payload for creating or replacing a recipe.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeInput {
    pub name: String,
    pub url: String,
    pub source_domain: String,
    pub minutes: Option<i64>,
    pub rating: Option<i64>,
    #[serde(default)]
    pub servings: Option<i64>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub usual_track_key: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub ingredients: Vec<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub key: String,
    pub label: String,
    pub color: String,
    pub position: i64,
    pub default_time: Option<String>,
    pub visible: bool,
    pub repeats_on: String,
}

/// One track in the editor. The list order becomes the schedule order.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInput {
    pub key: String,
    pub label: String,
    pub color: String,
    pub default_time: Option<String>,
    pub visible: bool,
    pub repeats_on: String,
}

/// One planned meal in a schedule slot, with the recipe summary joined in.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PlannedMeal {
    pub date: String,
    pub track_key: String,
    pub recipe_id: String,
    pub name: String,
    pub minutes: Option<i64>,
    pub rating: Option<i64>,
}

/// Household settings and onboarding state. Matches the `settings_get` command.
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub household_size: i64,
    pub week_starts_on: String,
    pub onboarded: bool,
    #[sqlx(skip)]
    pub dietary: Vec<String>,
}

/// Payload for `settings_save`. Replaces the household settings and dietary needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub household_size: i64,
    pub week_starts_on: String,
    pub dietary: Vec<String>,
}
