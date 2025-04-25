use super::*;

impl TrenchBroomConfig {
	/// Creates a new TrenchBroom config. It is recommended to use this over [`TrenchBroomConfig::default`]
	pub fn new(name: impl Into<String>) -> Self {
		Self::default().name(name)
	}

	/// Inserts a new texture to auto-remove.
	pub fn auto_remove_texture(mut self, texture: impl ToString) -> Self {
		self.auto_remove_textures.insert(texture.to_string());
		self
	}

	/// Excludes "\*_normal", "\*_mr" (Metallic and roughness), "\*_emissive", and "\*_depth".
	pub fn default_texture_exclusions() -> Vec<String> {
		vec!["*_normal".into(), "*_mr".into(), "*_emissive".into(), "*_depth".into()]
	}

	/// (See documentation on [`TrenchBroomConfig::face_tags`])
	pub fn empty_face_tag() -> TrenchBroomTag {
		TrenchBroomTag::new("empty", "__TB_empty").attributes([TrenchBroomTagAttribute::Transparent])
	}

	/// A repeating, nearest-neighbor sampler.
	pub fn default_texture_sampler() -> ImageSampler {
		ImageSampler::nearest().repeat()
	}

	/// Switches to using linear (smooth) filtering on textures.
	pub fn linear_filtering(self) -> Self {
		self.texture_sampler(ImageSampler::linear().repeat())
	}

	pub fn default_compute_lightmap_settings() -> ComputeLightmapSettings {
		ComputeLightmapSettings {
			special_lighting_color: [75; 3],
			..default()
		}
	}

	/// Names the entity based on the classname, and `targetname` if the property exists. (See documentation on [`TrenchBroomConfig::global_spawner`])
	///
	/// If the entity is a brush entity, rotation is reset.
	pub fn default_global_spawner(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let classname = view.src_entity.classname()?.s();

		// For things like doors where the `angles` property means open direction.
		if let Some(mut transform) = view.entity.get_mut::<Transform>() {
			if view.config.get_class(&classname).map(|class| class.info.ty.is_solid()) == Some(true) {
				transform.rotation = Quat::IDENTITY;
			}
		}

		view.entity.insert(Name::new(
			view.src_entity
				.get::<String>("targetname")
				.map(|name| format!("{classname} ({name})"))
				.unwrap_or(classname),
		));

		Ok(())
	}

	/// Adds [`Visibility`] and [`Transform`] components if they aren't in the entity, as it is needed to clear up warnings for child meshes.
	///
	/// Also adds [`GenericMaterial3d`]s.
	pub fn default_global_geometry_provider(view: &mut GeometryProviderView) {
		let mut ent = view.world.entity_mut(view.entity);

		#[cfg(feature = "client")]
		if !ent.contains::<Visibility>() {
			ent.insert(Visibility::default());
		}
		if !ent.contains::<Transform>() {
			ent.insert(Transform::default());
		}

		for mesh_view in &view.meshes {
			view.world
				.entity_mut(mesh_view.entity)
				.insert(GenericMaterial3d(mesh_view.texture.material.clone()));
		}
	}

	pub fn load_embedded_texture_fn(mut self, provider: impl FnOnce(Arc<LoadEmbeddedTextureFn>) -> Arc<LoadEmbeddedTextureFn>) -> Self {
		self.load_embedded_texture.set(provider);
		self
	}
	pub fn default_load_embedded_texture<'a>(
		#[allow(unused_mut)] mut view: EmbeddedTextureLoadView<'a, '_>,
	) -> BoxedFuture<'a, Handle<GenericMaterial>> {
		Box::pin(async move {
			#[cfg(feature = "client")]
			let mut material = StandardMaterial {
				base_color_texture: Some(view.image_handle.clone()),
				perceptual_roughness: 1.,
				..default()
			};

			#[cfg(feature = "client")]
			if let Some(alpha_mode) = view.alpha_mode {
				material.alpha_mode = alpha_mode;
			}

			#[cfg(feature = "client")]
			let generic_material = match special_textures::load_special_texture(&mut view, &material) {
				Some(v) => v,
				None => GenericMaterial {
					handle: view.add_material(material).into(),
					properties: default(),
				},
			};

			#[cfg(not(feature = "client"))]
			let generic_material = GenericMaterial::default();

			view.parent_view
				.load_context
				.add_labeled_asset(format!("{GENERIC_MATERIAL_PREFIX}{}", view.name), generic_material)
		})
	}

	pub fn load_loose_texture_fn(mut self, provider: impl FnOnce(Arc<LoadLooseTextureFn>) -> Arc<LoadLooseTextureFn>) -> Self {
		self.load_loose_texture.set(provider);
		self
	}
	/// Tries to load a [`GenericMaterial`] with the [`generic_material_extension`](Self::generic_material_extension), as a fallback tries [`texture_extension`](Self::texture_extension).
	pub fn default_load_loose_texture<'a>(view: TextureLoadView<'a, '_>) -> BoxedFuture<'a, Handle<GenericMaterial>> {
		Box::pin(async move {
			let generic_material_path = view
				.tb_config
				.material_root
				.join(format!("{}.{}", view.name, view.tb_config.generic_material_extension));

			// Extract the asset source out of load_context without borrowing it.
			// This is hacky, but i can't think of a better way to keep the borrow checker pleased.
			// SAFETY: The other things load_context is used for in this function don't interact with asset_path at all.
			let source = unsafe { (*std::ptr::from_ref(view.load_context)).asset_path().source() };
			let generic_material_path = AssetPath::from_path(&generic_material_path).with_source(source);

			#[allow(clippy::unnecessary_to_owned)]
			match view
				.asset_server
				.get_source(view.load_context.asset_path().source())
				.expect("Could not find asset source")
				.reader()
				// Annoying clone, but the borrow checker demands it!
				.read(&generic_material_path.path().to_path_buf())
				.await
			{
				Ok(_) => {
					let texture_sampler = view.tb_config.texture_sampler.clone();
					view.load_context
						.loader()
						.with_settings(move |s: &mut ImageLoaderSettings| s.sampler = texture_sampler.clone())
						.load(generic_material_path)
				}
				Err(err) => match err {
					AssetReaderError::NotFound(_) => {
						let texture_sampler = view.tb_config.texture_sampler.clone();
						let image_path = view
							.tb_config
							.material_root
							.join(format!("{}.{}", view.name, view.tb_config.texture_extension));

						view.load_context
							.loader()
							.with_settings(move |s: &mut ImageLoaderSettings| s.sampler = texture_sampler.clone())
							.load(AssetPath::from_path(&image_path).with_source(source))
					}

					err => {
						error!("Loading map {}: {err}", view.load_context.asset_path());
						Handle::default()
					}
				},
			}
		})
	}

	/// Returns a copy of [`Self::entity_scale_expression`], replacing all instances of "%%scale%%" with this config's scale.
	pub fn get_entity_scale_expression(&self) -> Option<String> {
		self.entity_scale_expression
			.as_ref()
			.map(|s| s.replace("%%scale%%", &self.scale.to_string()))
	}

	/// Retrieves the entity class of `classname` from this config. If none is found and the `auto_register` feature is enabled, it'll try to find it in [`GLOBAL_CLASS_REGISTRY`](crate::class::GLOBAL_CLASS_REGISTRY).
	pub fn get_class(&self, classname: &str) -> Option<&ErasedQuakeClass> {
		#[cfg(not(feature = "auto_register"))]
		{
			self.entity_classes.get(classname).map(Cow::as_ref)
		}

		#[cfg(feature = "auto_register")]
		{
			self.entity_classes
				.get(classname)
				.map(Cow::as_ref)
				.or_else(|| class::GLOBAL_CLASS_REGISTRY.get(classname).copied())
		}
	}

	/// A list of all registered classes. If the `auto_register` feature is enabled, also includes [`GLOBAL_CLASS_REGISTRY`](crate::class::GLOBAL_CLASS_REGISTRY).
	pub fn class_iter(&self) -> impl Iterator<Item = &ErasedQuakeClass> {
		#[cfg(not(feature = "auto_register"))]
		{
			self.entity_classes
				.values()
				.map(Cow::as_ref)
				.sorted_by(|a, b| a.info.name.cmp(b.info.name))
		}

		#[cfg(feature = "auto_register")]
		{
			self.entity_classes
				.values()
				.map(Cow::as_ref)
				.chain(class::GLOBAL_CLASS_REGISTRY.values().copied())
				.sorted_by(|a, b| a.info.name.cmp(b.info.name))
		}
	}

	/// Registers a [`QuakeClass`] into this config. It will be outputted into the fgd, and will be used when loading entities into scenes.
	///
	/// If the `auto_register` feature is enabled, you don't have to do this, as it automatically puts classes into a global registry when [`QuakeClass`] is derived.
	pub fn register_class<T: QuakeClass>(mut self) -> Self {
		self.entity_classes.insert(T::CLASS_INFO.name, Cow::Borrowed(T::ERASED_CLASS));
		self
	}

	/// Register an owned [`ErasedQuakeClass`] directly for dynamic classes. You almost always want to be using [`Self::register_class`] instead.
	pub fn register_class_dynamic(mut self, class: ErasedQuakeClass) -> Self {
		self.entity_classes.insert(class.info.name, Cow::Owned(class));
		self
	}

	/// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
	pub fn to_bevy_space(&self, vec: Vec3) -> Vec3 {
		vec.z_up_to_y_up() / self.scale
	}

	/// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
	pub fn to_bevy_space_f64(&self, vec: DVec3) -> DVec3 {
		vec.z_up_to_y_up() / self.scale as f64
	}

	/// The opposite of [`Self::to_bevy_space`], converts from a y-up coordinate space to z-up, and scales everything up by this config's scale.
	pub fn from_bevy_space(&self, vec: Vec3) -> Vec3 {
		vec.y_up_to_z_up() * self.scale
	}

	/// The opposite of [`Self::to_bevy_space_f64`], converts from a y-up coordinate space to z-up, and scales everything up by this config's scale.
	pub fn from_bevy_space_f64(&self, vec: DVec3) -> DVec3 {
		vec.y_up_to_z_up() * self.scale as f64
	}
}