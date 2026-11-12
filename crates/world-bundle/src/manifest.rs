//! Strict wire extension: keep existing authored Manifest literals source compatible.
use crate::{Error, bundle::Manifest, layers::Layer};
#[cfg(feature = "writer")]
use serde::Serialize;
use serde::{
    Deserialize,
    de::{Error as _, MapAccess, Visitor},
};
use serde_json::value::RawValue;
use std::{collections::BTreeSet, fmt};

pub(super) fn decode(bytes: &[u8]) -> Result<(Manifest, Vec<Layer>), Error> {
    if bytes.len() > crate::bundle::MAX_MANIFEST_BYTES {
        return Err(Error::Limit("manifest too large".into()));
    }
    struct Fields(Vec<(String, Box<RawValue>)>);
    impl<'de> Deserialize<'de> for Fields {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct FieldsVisitor;
            impl<'de> Visitor<'de> for FieldsVisitor {
                type Value = Fields;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("unique manifest fields")
                }
                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Fields, A::Error> {
                    let mut fields = Vec::new();
                    let mut names = BTreeSet::new();
                    while let Some((name, value)) = map.next_entry::<String, Box<RawValue>>()? {
                        if !names.insert(name.clone()) {
                            return Err(A::Error::custom("duplicate manifest field"));
                        }
                        fields.push((name, value));
                    }
                    Ok(Fields(fields))
                }
            }
            d.deserialize_map(FieldsVisitor)
        }
    }
    let fields: Fields = serde_json::from_slice(bytes).map_err(format)?;
    let mut manifest = String::from("{");
    let mut layers = Vec::new();
    for (key, value) in fields.0 {
        if key == "layers" {
            layers = serde_json::from_str(value.get()).map_err(format)?;
        } else {
            if manifest.len() > 1 {
                manifest.push(',');
            }
            manifest.push_str(&serde_json::to_string(&key).map_err(format)?);
            manifest.push(':');
            manifest.push_str(value.get());
        }
    }
    manifest.push('}');
    Ok((serde_json::from_str(&manifest).map_err(format)?, layers))
}
fn format(error: serde_json::Error) -> Error {
    Error::Format(format!("manifest: {error}"))
}

#[cfg(feature = "writer")]
pub(super) fn encode(manifest: &Manifest, layers: &[Layer]) -> Result<Vec<u8>, Error> {
    #[derive(Serialize)]
    struct Envelope<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        layers: Option<&'a [Layer]>,
        #[serde(flatten)]
        manifest: &'a Manifest,
    }
    serde_json::to_vec(&Envelope {
        layers: (!layers.is_empty()).then_some(layers),
        manifest,
    })
    .map_err(format)
}

#[cfg(all(test, feature = "writer"))]
mod tests {
    use super::*;
    use crate::bundle::{BuildClass, Extent, FORMAT, Identity, VERSION};
    #[test]
    fn envelope_preserves_literal_api_and_rejects_duplicate_or_unknown_fields() -> Result<(), Error>
    {
        let manifest = Manifest {
            subareas: Default::default(),
            format: FORMAT.into(),
            min_reader_version: VERSION,
            projection_class: "server".into(),
            compiler_version: "test".into(),
            build_class: BuildClass::Production,
            identity: Identity::default(),
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 1,
                max_y: 1,
                floors: vec![-7],
            },
            palette: vec![],
            draft_areas: vec![],
            skipped_provisional_keys: vec![],
            dropped_teleports: vec![],
            spawns: Default::default(),
            npcs: Default::default(),
        };
        let bytes = encode(&manifest, &[])?;
        let (decoded, layers) = decode(&bytes)?;
        assert_eq!(decoded, manifest);
        assert!(layers.is_empty());
        let text = String::from_utf8(bytes).map_err(|e| Error::Format(e.to_string()))?;
        let duplicate = format!("{{\"format\":\"duplicate\",{}", &text[1..]);
        assert!(decode(duplicate.as_bytes()).is_err());
        let duplicate = format!("{{\"layers\":[],\"layers\":[],{}", &text[1..]);
        assert!(decode(duplicate.as_bytes()).is_err());
        let unknown = format!("{{\"unknown\":true,{}", &text[1..]);
        assert!(decode(unknown.as_bytes()).is_err());
        let nested = text.replace(
            "\"project_format_version\":",
            "\"unknown_nested\":true,\"project_format_version\":",
        );
        assert!(decode(nested.as_bytes()).is_err());
        Ok(())
    }
}
