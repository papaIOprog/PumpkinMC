use std::{collections::BTreeMap, fs};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

fn canonicalize_type_name(name: &str) -> String {
    match name {
        "integer" | "int" => "int".to_string(),
        "entity_pose" | "pose" => "pose".to_string(),
        "facing" | "direction" => "direction".to_string(),
        "text_component" | "component" => "component".to_string(),
        "optional_text_component" | "optional_component" => "optional_component".to_string(),
        "optional_int" | "optional_unsigned_int" => "optional_unsigned_int".to_string(),
        "vector_3f" | "vector3f" | "vector3" => "vector3".to_string(),
        "quaternion_f" | "quaternionf" | "quaternion" => "quaternion".to_string(),
        "rotation" | "rotations" => "rotations".to_string(),
        "particle_list" | "particles" => "particles".to_string(),
        "copper_golem_state" | "weathering_copper_state" => "weathering_copper_state".to_string(),
        "profile" | "resolvable_profile" => "resolvable_profile".to_string(),
        "arm" | "humanoid_arm" => "humanoid_arm".to_string(),
        other => other.to_string(),
    }
}

/// Generates the `TokenStream` for the `MetaDataType` struct for Minecraft 26.3.
pub fn build() -> TokenStream {
    let path = "../../assets/meta_data_type.json";
    let parsed: BTreeMap<String, i32> = serde_json::from_str(
        &fs::read_to_string(path).expect("Failed to read meta_data_type.json"),
    )
    .expect("Failed to parse meta_data_type.json");

    let mut handlers_map: BTreeMap<String, i32> = BTreeMap::new();
    for (name, id) in parsed {
        let canonical = canonicalize_type_name(&name);
        handlers_map.insert(canonical, id);
    }

    let mut variants = TokenStream::new();
    for (name, id) in &handlers_map {
        let ident = format_ident!("{}", name.to_uppercase());
        variants.extend(quote! {
            pub const #ident: MetaDataType = MetaDataType { id: #id };
        });
    }

    // Old names kept as aliases; only emitted when the canonical type exists
    // in the dumped version.
    const ALIAS_TARGETS: &[(&str, &str)] = &[
        ("INTEGER", "int"),
        ("ENTITY_POSE", "pose"),
        ("FACING", "direction"),
        ("TEXT_COMPONENT", "component"),
        ("OPTIONAL_TEXT_COMPONENT", "optional_component"),
        ("OPTIONAL_INT", "optional_unsigned_int"),
        ("VECTOR_3F", "vector3"),
        ("QUATERNION_F", "quaternion"),
        ("ROTATION", "rotations"),
        ("PARTICLE_LIST", "particles"),
        ("COPPER_GOLEM_STATE", "weathering_copper_state"),
        ("PROFILE", "resolvable_profile"),
        ("ARM", "humanoid_arm"),
    ];
    let mut aliases = TokenStream::new();
    for (alias, target) in ALIAS_TARGETS {
        if handlers_map.contains_key(*target) {
            let alias_ident = format_ident!("{}", alias);
            let target_ident = format_ident!("{}", target.to_uppercase());
            aliases.extend(quote! {
                pub const #alias_ident: MetaDataType = Self::#target_ident;
            });
        }
    }

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct MetaDataType {
            pub id: i32,
        }

        impl MetaDataType {
            #variants

            #aliases

            #[must_use]
            pub const fn id(&self, _version: pumpkin_util::version::JavaMinecraftVersion) -> i32 {
                self.id
            }
        }
    }
}
