use std::time::Duration;

use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::Url;
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::Value;

/// Sites often refuse requests without a browser-like agent, so the scraper presents as Chrome.
const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// A recipe read from a page's Schema.org `Recipe` data. Fields are cleaned but not otherwise edited.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScrapedRecipe {
    pub name: String,
    /// The page address after redirects.
    pub source_url: String,
    pub image_url: Option<String>,
    pub ingredients: Vec<String>,
    pub instructions: Vec<String>,
}

/// Downloads the page at `url` and reads its Schema.org recipe data.
pub async fn scrape(url: &str) -> Result<ScrapedRecipe, String> {
    let address = Url::parse(url).map_err(|_| "That is not a valid web address.".to_string())?;
    if !matches!(address.scheme(), "http" | "https") {
        return Err("Only http and https links can be imported.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(address)
        .header(USER_AGENT, BROWSER_USER_AGENT)
        .header(ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
        .map_err(|error| format!("Could not reach the site: {error}."))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("The site refused the request ({status})."));
    }

    let page_url = response.url().to_string();
    let html = response
        .text()
        .await
        .map_err(|error| format!("Could not read the page: {error}."))?;
    parse(&html, &page_url)
}

/// Finds the first Schema.org `Recipe` in the page's JSON-LD blocks and cleans it up.
pub fn parse(html: &str, page_url: &str) -> Result<ScrapedRecipe, String> {
    let document = Html::parse_document(html);
    let scripts = Selector::parse(r#"script[type*="ld+json"]"#).expect("valid selector");

    let blocks: Vec<Value> = document
        .select(&scripts)
        .filter_map(|script| serde_json::from_str(&script.text().collect::<String>()).ok())
        .collect();

    let recipe = blocks
        .iter()
        .find_map(find_recipe)
        .ok_or_else(|| "This page has no recipe data to import. Enter the details by hand.".to_string())?;

    let name = text_of(recipe.get("name")).ok_or_else(|| "The recipe data on this page has no title.".to_string())?;

    let mut instructions = Vec::new();
    if let Some(value) = recipe.get("recipeInstructions") {
        collect_steps(value, &mut instructions);
    }

    Ok(ScrapedRecipe {
        name,
        source_url: page_url.to_string(),
        image_url: recipe.get("image").and_then(image_url).and_then(|image| absolute(page_url, &image)),
        ingredients: strings_of(recipe.get("recipeIngredient")),
        instructions,
    })
}

/// The recipe object in a JSON-LD block. Recipes sit at the top level, in an array, or under `@graph`.
fn find_recipe(node: &Value) -> Option<&Value> {
    match node {
        Value::Array(items) => items.iter().find_map(find_recipe),
        Value::Object(_) if has_type(node, "Recipe") => Some(node),
        Value::Object(_) => node.get("@graph").and_then(find_recipe),
        _ => None,
    }
}

/// `@type` is either a string or a list of strings.
fn has_type(node: &Value, wanted: &str) -> bool {
    match node.get("@type") {
        Some(Value::String(found)) => found == wanted,
        Some(Value::Array(found)) => found.iter().any(|item| item.as_str() == Some(wanted)),
        _ => false,
    }
}

/// Instructions come as plain text, a list of text, `HowToStep` objects, or `HowToSection` objects
/// that hold their own steps. Every shape is flattened into one list of steps.
fn collect_steps(value: &Value, steps: &mut Vec<String>) {
    match value {
        Value::String(text) => steps.extend(clean(text)),
        Value::Array(items) => items.iter().for_each(|item| collect_steps(item, steps)),
        Value::Object(_) => match value.get("itemListElement") {
            Some(nested) => collect_steps(nested, steps),
            None => steps.extend(text_of(value.get("text").or(value.get("name")))),
        },
        _ => {}
    }
}

/// An image is a URL, a list of URLs, or an `ImageObject` with `url` or `contentUrl`.
fn image_url(value: &Value) -> Option<String> {
    match value {
        Value::String(url) => clean(url),
        Value::Array(items) => items.iter().find_map(image_url),
        Value::Object(_) => value.get("url").or(value.get("contentUrl")).and_then(image_url),
        _ => None,
    }
}

/// Resolves a relative image path against the page address.
fn absolute(page_url: &str, candidate: &str) -> Option<String> {
    let base = Url::parse(page_url).ok()?;
    base.join(candidate).ok().map(|url| url.to_string())
}

fn strings_of(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(|item| text_of(Some(item))).collect(),
        other => text_of(other).into_iter().collect(),
    }
}

fn text_of(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).and_then(clean)
}

/// Trims whitespace and drops empty text.
fn clean(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "https://example.com/recipes/pancakes";

    fn page(json_ld: &str) -> String {
        format!(r#"<html><head><script type="application/ld+json">{json_ld}</script></head><body></body></html>"#)
    }

    #[test]
    fn reads_a_top_level_recipe() {
        let html = page(r#"{
            "@type": "Recipe",
            "name": "  Pancakes ",
            "image": "/img/pancakes.jpg",
            "recipeIngredient": ["1 cup flour", " ", "2 eggs"],
            "recipeInstructions": ["Mix.", "Fry."]
        }"#);

        let recipe = parse(&html, PAGE).unwrap();
        assert_eq!(recipe.name, "Pancakes");
        assert_eq!(recipe.source_url, PAGE);
        assert_eq!(recipe.image_url.as_deref(), Some("https://example.com/img/pancakes.jpg"));
        assert_eq!(recipe.ingredients, vec!["1 cup flour", "2 eggs"]);
        assert_eq!(recipe.instructions, vec!["Mix.", "Fry."]);
    }

    #[test]
    fn finds_recipe_inside_graph_with_list_types() {
        let html = page(r#"{
            "@context": "https://schema.org",
            "@graph": [
                {"@type": "WebPage", "name": "Not this"},
                {"@type": ["Thing", "Recipe"], "name": "Soup", "recipeIngredient": "water"}
            ]
        }"#);

        let recipe = parse(&html, PAGE).unwrap();
        assert_eq!(recipe.name, "Soup");
        assert_eq!(recipe.ingredients, vec!["water"]);
    }

    #[test]
    fn flattens_how_to_sections_and_steps() {
        let html = page(r#"{
            "@type": "Recipe",
            "name": "Cake",
            "recipeInstructions": [
                {"@type": "HowToSection", "name": "Batter", "itemListElement": [
                    {"@type": "HowToStep", "text": "Whisk eggs."},
                    {"@type": "HowToStep", "name": "Bake it."}
                ]},
                "Cool.",
                {"@type": "HowToStep", "text": "  "}
            ]
        }"#);

        let recipe = parse(&html, PAGE).unwrap();
        assert_eq!(recipe.instructions, vec!["Whisk eggs.", "Bake it.", "Cool."]);
    }

    #[test]
    fn reads_image_objects_and_absolute_urls() {
        let html = page(r#"{
            "@type": "Recipe",
            "name": "Rice",
            "image": [{"@type": "ImageObject", "url": "https://cdn.example.com/rice.jpg"}]
        }"#);

        let recipe = parse(&html, PAGE).unwrap();
        assert_eq!(recipe.image_url.as_deref(), Some("https://cdn.example.com/rice.jpg"));
    }

    #[test]
    fn skips_broken_blocks_and_reports_missing_recipe() {
        let html = r#"<script type="application/ld+json">{ not json</script>
            <script type="application/ld+json">{"@type": "Article", "name": "News"}</script>"#;

        assert!(parse(html, PAGE).unwrap_err().contains("no recipe data"));
    }

    #[test]
    fn recipe_without_title_is_an_error() {
        let html = page(r#"{"@type": "Recipe", "recipeIngredient": ["salt"]}"#);

        assert!(parse(&html, PAGE).unwrap_err().contains("no title"));
    }
}
