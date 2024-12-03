use bevy_internal::asset::io::Reader;
use bevy_internal::asset::{AssetLoader, LoadContext};
use bevy_internal::prelude::*;
use bevy_internal::tasks::block_on;
use serde::{Deserialize, Serialize};

//use crate::bevy::Effects;
use crate::prelude::*;

impl BinaryAsset {
    async fn recurse_nodes(
        &self, _world: &mut World, _context: &mut LoadContext<'_>, _assets: &mut PandaAsset,
        _node_index: usize,
    ) -> Result<(), bam::Error> {
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoadSettings {}

#[derive(Debug, Default)]
pub struct BamLoader;

#[derive(Asset, TypePath, Debug, Default)]
pub struct PandaAsset {
    scene: Handle<Scene>,
}

impl AssetLoader for BamLoader {
    type Asset = PandaAsset;
    type Error = bam::Error;
    type Settings = LoadSettings;

    async fn load(
        &self, reader: &mut dyn Reader, _settings: &Self::Settings, load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // let start_time = bevy_internal::utils::Instant::now();

        // First, let's parse the data into something we can work with. TODO: take the Reader directly?
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Then, let's parse out our scene graph. TODO: make an async function?
        let bam = BinaryAsset::load(bytes)?;

        // Now we need to post-process it into a scene we can actually spawn
        let mut assets = Self::Asset::default();
        assets.scene = load_context.labeled_asset_scope("Scene0".to_string(), |context| {
            let mut world = World::default();

            block_on(bam.recurse_nodes(&mut world, context, &mut assets, 1)).unwrap();

            Scene::new(world)
        });

        Ok(assets)
    }

    fn extensions(&self) -> &[&str] {
        &["bam", "boo"]
    }
}
