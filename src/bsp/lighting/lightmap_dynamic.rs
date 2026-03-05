//! Dynamic lighting for objects based purely on lightmap data.

use bevy::{
	asset::RenderAssetUsages,
	ecs::{entity::EntityHashMap, lifecycle::HookContext, world::DeferredWorld},
	pbr::Lightmap,
	prelude::*,
};
use ndshape::ConstShape;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

pub struct BspLightmapDynamicLightingPlugin;
impl Plugin for BspLightmapDynamicLightingPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<FixedLightingPool>().add_systems(
			Update,
			(Self::add_fixed_lighting, Self::compute_fixed_lighting, Self::apply_lightmaps).chain(),
		);
	}
}
impl BspLightmapDynamicLightingPlugin {
	pub fn add_fixed_lighting(mut pool: ResMut<FixedLightingPool>) {}

	pub fn compute_fixed_lighting() {}

	pub fn apply_lightmaps(
		mut commands: Commands,
		pool: Res<FixedLightingPool>,
		query: Query<(Entity, &FixedLighting), (With<Mesh3d>, Without<Lightmap>)>,
	) {
		for (entity, lighting) in query {
			commands.entity(entity).insert(Lightmap {
				image: pool.texture.clone(),
				uv_rect: Rect::from_center_size(
					(UVec2::from_array(FixedLightingPoolShape::delinearize(lighting.pool_idx)).as_vec2() + 0.5) / FIXED_LIGHTING_POOL_SIZE as f32,
					Vec2::ZERO,
				),
				bicubic_sampling: false,
			});
		}
	}
}

// 128x128 = 16384 meshes, that's plenty.
const FIXED_LIGHTING_POOL_POWER: u32 = 7;
const FIXED_LIGHTING_POOL_SIZE: u32 = 2u32.pow(FIXED_LIGHTING_POOL_POWER);

type FixedLightingPoolShape = ndshape::ConstPow2Shape2u32<FIXED_LIGHTING_POOL_POWER, FIXED_LIGHTING_POOL_POWER>;

#[derive(Resource)]
struct FixedLightingPool {
	texture: Handle<Image>,
	// bindings: EntityHashMap<>,
	free: Vec<u32>,
	head: usize,
}
impl FromWorld for FixedLightingPool {
	fn from_world(world: &mut World) -> Self {
		let image = Image::new_fill(
			Extent3d {
				width: FIXED_LIGHTING_POOL_SIZE,
				height: FIXED_LIGHTING_POOL_SIZE,
				depth_or_array_layers: 1,
			},
			TextureDimension::D2,
			&[0; 4],
			TextureFormat::Rgba8UnormSrgb,
			RenderAssetUsages::all(),
		);

		let texture = world.resource_mut::<Assets<Image>>().add(image);
		Self {
			texture,
			free: Vec::new(),
			head: 0,
		}
	}
}

#[derive(Component)]
#[component(on_remove = Self::on_remove)]
pub struct FixedLighting {
	pool_idx: u32,
	pub color: Color,
}
impl FixedLighting {
	fn on_remove(mut world: DeferredWorld, ctx: HookContext) {
		let pool_idx = world.entity(ctx.entity).get::<Self>().unwrap().pool_idx;
		world.resource_mut::<FixedLightingPool>().free.push(pool_idx);
	}
}
