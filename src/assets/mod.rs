//! Swarmy Game Asset functionality
use bevy::prelude::*;

pub mod mpq_events;
pub use mpq_events::*;

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct GameAssets {
    pub ike_scene: Handle<Scene>,
    pub cartman_scene: Handle<Scene>,
    pub kyle_scene: Handle<Scene>,
    pub kenny_scene: Handle<Scene>,
    pub tweek_scene: Handle<Scene>,
    pub sp_church_scene: Handle<Scene>,
    pub wendy_scene: Handle<Scene>,
}

pub fn asset_loading(mut commands: Commands, assets: ResMut<AssetServer>) {
    let ike_asset: Handle<Scene> = assets.load("south_park_canada_ike.glb#Scene0");
    let cartman_asset: Handle<Scene> = assets.load("cartman.glb#Scene0");
    let kyle_asset: Handle<Scene> = assets.load("south_park_kyle_broflovski.glb#Scene0");
    let kenny_asset: Handle<Scene> = assets.load("kenny.glb#Scene0");
    let tweek_asset: Handle<Scene> =
        assets.load("nintendo_64_-_south_park_rally_-_tweek.glb#Scene0");
    let sp_church_asset: Handle<Scene> = assets.load("sp_church.glb#Scene0");
    let wendy_asset: Handle<Scene> = assets.load("wendy_testaburger.glb#Scene0");
    commands.insert_resource(GameAssets {
        ike_scene: ike_asset,
        cartman_scene: cartman_asset,
        kyle_scene: kyle_asset,
        kenny_scene: kenny_asset,
        tweek_scene: tweek_asset,
        sp_church_scene: sp_church_asset,
        wendy_scene: wendy_asset,
    });
}
