use super::*;

flat! {
	#[cfg(feature = "bsp")]
	bsp;
}

/// The prefix used by base classes provided by bevy_trenchbroom.
///
/// You should not use this prefix in your base classes to avoid conflicts.
pub const BUILTIN_BASE_CLASS_PREFIX: &str = "__";

#[derive(Default)]
pub struct BaseClassesPlugin;
impl Plugin for BaseClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type_data::<Transform, ReflectQuakeClass>()
		;

		#[cfg(feature = "client")]
		#[rustfmt::skip]
		app
			.register_type_data::<Visibility, ReflectQuakeClass>()
		;
	}
}

impl QuakeClass for Transform {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__transform",
		description: None,
		base: &[],

		model: None,
		color: None,
		iconsprite: None,
		size: None,
		decal: false,

		properties: &[
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "origin",
				title: Some("Translation/Origin"),
				description: None,
				default_value: Some(|| Vec3::ZERO.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "angles",
				title: Some("Rotation (pitch yaw roll) in degrees"),
				description: None,
				default_value: Some(|| Vec3::ZERO.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "scale",
				title: Some("Scale"),
				description: None,
				default_value: Some(|| Vec3::ONE.fgd_to_string()),
			},
		],
	};

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		view.world.entity_mut(view.entity).insert(Transform {
			translation: read_translation_from_entity(view.src_entity, view.tb_config)?,
			rotation: read_rotation_from_entity(view.src_entity)?,
			scale: match view.src_entity.get::<f32>("scale") {
				Ok(scale) => Vec3::splat(scale),
				Err(QuakeEntityError::RequiredPropertyNotFound { .. }) => Vec3::ONE,
				Err(_) => view.src_entity.get::<Vec3>("scale")?.xzy(),
			},
		});
		Ok(())
	}
}

#[cfg(feature = "client")]
impl QuakeClass for Visibility {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__visibility",
		description: None,
		base: &[],

		model: None,
		color: None,
		iconsprite: None,
		size: None,
		decal: false,

		properties: &[QuakeClassProperty {
			#[rustfmt::skip]
			ty: QuakeClassPropertyType::Choices(&[
				(ChoicesKey::String("Inherited"), "Uses the visibility of its parents. If its a root-level entity, it will be visible."),
				(ChoicesKey::String("Hidden"), "Always not rendered, regardless of its parent's visibility."),
				(ChoicesKey::String("Visible"), "Always rendered, regardless of its parent's visibility."),
			]),
			name: "visibility",
			title: Some("Visibility"),
			description: None,
			default_value: Some(|| "\"Inherited\"".s()),
		}],
	};

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let visibility = match view.src_entity.properties.get("visibility").map(String::as_str) {
			Some("Inherited") => Visibility::Inherited,
			Some("Hidden") => Visibility::Hidden,
			Some("Visible") => Visibility::Visible,
			None => Visibility::default(),
			Some(value) => Err(qmap::QuakeEntityError::PropertyParseError {
				property: "visibility".s(),
				value: value.s(),
				required_type: "Visibility",
				error: "Must be either `Inherited`, `Hidden`, or `Visible`".s(),
			})?,
		};

		view.world.entity_mut(view.entity).insert(visibility);

		Ok(())
	}
}

/// Quake entity IO - Able to target entities with the [`Targetable`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[base_class(classname("__target"))]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct Target {
	/// If [`Some`], when this entity's IO fires, it will activate all entities with its [`Targetable::targetname`] set to this, with whatever input that functionality that entity has set up.
	pub target: Option<String>,
	/// If [`Some`], when this entity's IO fires, it will kill all entities with its [`Targetable::targetname`] set to this.
	pub killtarget: Option<String>,
}

/// Quake entity IO - Able to be targeted from a [`Target`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[base_class(classname("__targetable"))]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct Targetable {
	/// The name for entities with [`Target`] components to point to.
	pub targetname: Option<String>,
}
