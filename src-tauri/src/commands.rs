use std::collections::BTreeMap;

use serde_json::Value;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::keep_awake::KeepAwakeLock;
use crate::models::{PlannedMeal, RecipeDetail, RecipeInput, SavedRecipe, Settings, SettingsInput, Track, TrackInput};
use crate::repo::{config, recipes, schedule, settings};
use crate::scrape::{self, ScrapedRecipe};

#[tauri::command]
pub async fn recipes_list(db: State<'_, SqlitePool>) -> Result<Vec<SavedRecipe>, AppError> {
    recipes::list(&db).await
}

#[tauri::command]
pub async fn recipe_get(db: State<'_, SqlitePool>, id: String) -> Result<RecipeDetail, AppError> {
    recipes::get(&db, &id).await
}

#[tauri::command]
pub async fn recipe_create(
    db: State<'_, SqlitePool>,
    input: RecipeInput,
) -> Result<RecipeDetail, AppError> {
    let id = recipes::create(&db, &input).await?;
    recipes::get(&db, &id).await
}

#[tauri::command]
pub async fn recipe_update(
    db: State<'_, SqlitePool>,
    id: String,
    input: RecipeInput,
) -> Result<RecipeDetail, AppError> {
    recipes::update(&db, &id, &input).await?;
    recipes::get(&db, &id).await
}

#[tauri::command]
pub async fn recipe_delete(db: State<'_, SqlitePool>, id: String) -> Result<(), AppError> {
    recipes::delete(&db, &id).await
}

/// Reads a recipe from a web page's Schema.org data. The error is shown to the user as-is.
#[tauri::command]
pub async fn scrape_recipe_url(url: String) -> Result<ScrapedRecipe, String> {
    scrape::scrape(&url).await
}

#[tauri::command]
pub async fn tracks_list(db: State<'_, SqlitePool>) -> Result<Vec<Track>, AppError> {
    schedule::tracks(&db).await
}

#[tauri::command]
pub async fn tracks_save(db: State<'_, SqlitePool>, tracks: Vec<TrackInput>) -> Result<(), AppError> {
    schedule::save_tracks(&db, &tracks).await
}

#[tauri::command]
pub async fn schedule_week(
    db: State<'_, SqlitePool>,
    start: String,
    end: String,
) -> Result<Vec<PlannedMeal>, AppError> {
    schedule::week(&db, &start, &end).await
}

#[tauri::command]
pub async fn schedule_set_entry(
    db: State<'_, SqlitePool>,
    date: String,
    track_key: String,
    recipe_id: String,
) -> Result<(), AppError> {
    schedule::set_entry(&db, &date, &track_key, &recipe_id).await
}

#[tauri::command]
pub async fn schedule_clear_entry(
    db: State<'_, SqlitePool>,
    date: String,
    track_key: String,
) -> Result<(), AppError> {
    schedule::clear_entry(&db, &date, &track_key).await
}

#[tauri::command]
pub async fn settings_get(db: State<'_, SqlitePool>) -> Result<Settings, AppError> {
    settings::get(&db).await
}

#[tauri::command]
pub async fn settings_save(db: State<'_, SqlitePool>, input: SettingsInput) -> Result<(), AppError> {
    settings::save(&db, &input).await
}

#[tauri::command]
pub async fn settings_complete_onboarding(db: State<'_, SqlitePool>) -> Result<(), AppError> {
    settings::complete_onboarding(&db).await
}

#[tauri::command]
pub async fn config_list(db: State<'_, SqlitePool>) -> Result<BTreeMap<String, Value>, AppError> {
    config::list(&db).await
}

#[tauri::command]
pub async fn config_set(db: State<'_, SqlitePool>, key: String, value: Value) -> Result<(), AppError> {
    config::set(&db, &key, &value).await
}

#[tauri::command]
pub fn keep_awake_set(lock: State<'_, KeepAwakeLock>, enabled: bool) -> Result<(), AppError> {
    lock.set(enabled)
}
