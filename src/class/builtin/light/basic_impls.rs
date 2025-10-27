use super::*;

// Iconsprites are quite user-specific. I've opted to just put a pretty generic generic path of `sprites/light_*.png`. Should work for most users.

//////////////////////////////////////////////////////////////////////////////////
//// LightingWorkflow::DynamicOnly
//////////////////////////////////////////////////////////////////////////////////

/// [`LightingWorkflow::DynamicOnly`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(PointLight),
	classname("light_point"),
	iconsprite({ path: "sprites/light_point.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicOnlyPointLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("light_point"))]
#[reflect(no_auto_register)]
pub struct DynamicOnlyPointLight;

/// [`LightingWorkflow::DynamicOnly`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(SpotLight),
	classname("light_spot"),
	iconsprite({ path: "sprites/light_spot.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicOnlySpotLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("light_spot"))]
#[reflect(no_auto_register)]
pub struct DynamicOnlySpotLight;

/// [`LightingWorkflow::DynamicOnly`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(DirectionalLight),
	classname("light_directional"),
	iconsprite({ path: "sprites/light_directional.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicOnlyDirectionalLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("light_directional"))]
#[reflect(no_auto_register)]
pub struct DynamicOnlyDirectionalLight;

//////////////////////////////////////////////////////////////////////////////////
//// LightingWorkflow::BakedOnly
//////////////////////////////////////////////////////////////////////////////////

/// [`LightingWorkflow::BakedOnly`] and [`LightingWorkflow::DynamicAndBakedSeparate`] implementation.
#[cfg(feature = "bsp")]
#[point_class(
	base(BspLight),
	classname("light"),
	// TODO: switch to different models/sprites when spot or sun light
	iconsprite({ path: "sprites/light_point.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct BakedOnlyLight;

//////////////////////////////////////////////////////////////////////////////////
//// LightingWorkflow::MapDynamicBspBaked
//////////////////////////////////////////////////////////////////////////////////

/// [`LightingWorkflow::MapDynamicBspBaked`] implementation.
#[cfg(all(feature = "bsp", feature = "client"))]
#[point_class(
	base(MixedLight),
	classname("light"),
	iconsprite({ path: "sprites/light_point.png", scale: 0.1 }),
	hooks(SceneHooks::new().push(Self::spawn_hook)),
)]
#[reflect(no_auto_register)]
pub struct MapDynamicBspBakedLight;
#[cfg(all(feature = "bsp", feature = "client"))]
impl MapDynamicBspBakedLight {
	pub fn spawn_hook(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		if view.file_type.is_bsp() {
			return Ok(());
		}

		CombinedLight::spawn_hook(view)
	}
}

#[cfg(all(feature = "bsp", not(feature = "client")))]
#[point_class(classname("light"))]
#[reflect(no_auto_register)]
pub struct MapDynamicBspBakedLight;

//////////////////////////////////////////////////////////////////////////////////
//// LightingWorkflow::DynamicAndBakedCombined
//////////////////////////////////////////////////////////////////////////////////

/// [`LightingWorkflow::DynamicAndBakedCombined`] implementation.
#[cfg(all(feature = "bsp", feature = "client"))]
#[point_class(
	base(MixedLight),
	classname("light"),
	iconsprite({ path: "sprites/light_point.png", scale: 0.1 }),
	hooks(SceneHooks::new().push(Self::spawn_hook)),
)]
#[reflect(no_auto_register)]
pub struct CombinedLight;
#[cfg(all(feature = "bsp", feature = "client"))]
impl CombinedLight {
	pub fn spawn_hook(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let entity_ref = view.world.entity(view.entity);
		let bsp_light = entity_ref.get::<BspLight>().ok_or_else(|| anyhow!("No BspLight found for mixed light implementation during spawn hook!"))?;
		let mixed_light = entity_ref.get::<MixedLight>().ok_or_else(|| anyhow!("No MixedLight found for mixed light implementation during spawn hook!"))?;
		
		if let Some(light) = mixed_light.create_dynamic_light(bsp_light, view.tb_config) {
			light.insert(&mut view.world.entity_mut(view.entity));
		}

		Ok(())
	}
}

#[cfg(all(feature = "bsp", not(feature = "client")))]
#[point_class(classname("light"))]
#[reflect(no_auto_register)]
pub struct CombinedLight;

//////////////////////////////////////////////////////////////////////////////////
//// LightingWorkflow::DynamicAndBakedSeparate
//////////////////////////////////////////////////////////////////////////////////

/// [`LightingWorkflow::DynamicAndBakedSeparate`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(PointLight),
	classname("dynamiclight_point"),
	iconsprite({ path: "sprites/light_point.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicPointLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("dynamiclight_point"))]
#[reflect(no_auto_register)]
pub struct DynamicPointLight;

/// [`LightingWorkflow::DynamicAndBakedSeparate`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(SpotLight),
	classname("dynamiclight_spot"),
	iconsprite({ path: "sprites/light_spot.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicSpotLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("dynamiclight_spot"))]
#[reflect(no_auto_register)]
pub struct DynamicSpotLight;

/// [`LightingWorkflow::DynamicAndBakedSeparate`] implementation.
#[cfg(feature = "client")]
#[point_class(
	base(DirectionalLight),
	classname("dynamiclight_directional"),
	iconsprite({ path: "sprites/light_directional.png", scale: 0.1 }),
)]
#[reflect(no_auto_register)]
pub struct DynamicDirectionalLight;

#[cfg(not(feature = "client"))]
#[point_class(classname("dynamiclight_directional"))]
#[reflect(no_auto_register)]
pub struct DynamicDirectionalLight;
