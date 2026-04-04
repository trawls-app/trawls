use gpui::{AssetSource, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "examples/**"]
#[include = "*.png"]
struct TrawlsAssets;

/// Composite asset source that delegates to gpui-component's bundled icons
/// and falls back to trawls-specific assets (examples, logo).
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        // Try gpui-component icons first
        if let Ok(Some(data)) = gpui_component_assets::Assets.load(path) {
            return Ok(Some(data));
        }

        // Fall back to trawls-specific assets
        Ok(TrawlsAssets::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut result: Vec<SharedString> = gpui_component_assets::Assets
            .list(path)
            .unwrap_or_default();

        result.extend(
            TrawlsAssets::iter().filter_map(|p| p.starts_with(path).then(|| p.into())),
        );

        Ok(result)
    }
}
