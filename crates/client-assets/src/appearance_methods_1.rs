impl AppearanceIndex {
    /// Loads the `appearances` file the catalogue names, verified against its manifest entry.
    pub fn load(store: &AssetStore, catalog: &Catalog) -> Result<Self, AssetError> {
        let file = catalog.appearances_file();
        let bytes = store.read_verified(file, MAX_APPEARANCES_BYTES)?;
        let message =
            AppearancesMsg::decode(bytes.as_slice()).map_err(|error| AssetError::Malformed {
                file: file.to_owned(),
                reason: error.to_string(),
            })?;
        let convert_all = |entries: Vec<AppearanceMsg>| {
            let mut converted = HashMap::with_capacity(entries.len());
            for entry in entries {
                let Some(id) = entry.id.filter(|id| (1..=MAX_APPEARANCE_ID).contains(id)) else {
                    return Err(AssetError::Malformed {
                        file: file.to_owned(),
                        reason: "object without a valid id".into(),
                    });
                };
                if let Some(appearance) = convert(id, entry) {
                    converted.insert(id, appearance);
                }
            }
            Ok(converted)
        };
        Ok(Self {
            objects: convert_all(message.object)?,
            outfits: convert_all(message.outfit)?,
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    #[must_use]
    pub fn max_id(&self) -> Option<u32> {
        self.objects.keys().copied().max()
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Appearance> {
        self.objects.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Appearance> {
        self.objects.values()
    }

    /// Frame group 0 (standing) of an outfit (look type).
    #[must_use]
    pub fn outfit(&self, look_type: u32) -> Option<&Appearance> {
        self.outfits.get(&look_type)
    }

    /// The cells of an outfit's base layer, standing, facing `direction` (0 north, 1 east,
    /// 2 south, 3 west), without addons or mount. The colour-mask layer is not drawn.
    pub fn resolve_outfit(
        &self,
        sheets: &SpriteSheets,
        look_type: u32,
        direction: u32,
    ) -> Result<ResolvedEntry, AssetError> {
        let appearance = self
            .outfits
            .get(&look_type)
            .ok_or(AssetError::UnknownAppearance { id: look_type })?;
        let pattern_x = direction % appearance.pattern_width;
        entry(appearance, sheets, (pattern_x, 0, 0), 0..1)
    }

    /// The cells to draw for one entry, chosen as the Tibia client does. `floor` is the native
    /// floor `-15..=0`; the depth pattern is selected from `z = -floor`, never from a negative
    /// index.
    pub fn resolve(
        &self,
        sheets: &SpriteSheets,
        appearance_id: u32,
        placement: Placement,
    ) -> Result<ResolvedEntry, AssetError> {
        if !(1..=MAX_APPEARANCE_ID).contains(&appearance_id) {
            return Err(AssetError::InvalidAppearanceId { id: appearance_id });
        }
        if !(-15..=0).contains(&placement.floor) {
            return Err(AssetError::InvalidFloor {
                floor: placement.floor,
            });
        }
        let appearance = self
            .objects
            .get(&appearance_id)
            .ok_or(AssetError::UnknownAppearance { id: appearance_id })?;
        let (pattern_x, pattern_y, pattern_z) = pattern(appearance, placement);
        entry(
            appearance,
            sheets,
            (pattern_x, pattern_y, pattern_z),
            0..appearance.layers,
        )
    }
}
