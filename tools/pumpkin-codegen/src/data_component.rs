use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use std::fs;

/// Generates the `TokenStream` for the `DataComponent` enum and its ID/name conversion methods.
pub fn build() -> TokenStream {
    let data_component: BTreeMap<String, u8> =
        serde_json::from_str(&fs::read_to_string("../../assets/data_component.json").unwrap())
            .expect("Failed to parse data_component.json");
    // Components added after 1.21.1 have no wire id. They stay in the enum as
    // internal variants for server-side item metadata and are never accepted
    // from the wire (try_from_id only covers the synced registry).
    let internal: BTreeMap<String, u8> = serde_json::from_str(
        &fs::read_to_string("../../assets/data_component_internal.json").unwrap(),
    )
    .expect("Failed to parse data_component_internal.json");

    let mut enum_variants = TokenStream::new();
    let mut id_to_enum = TokenStream::new();
    let mut enum_to_name = TokenStream::new();
    let mut name_to_enum = TokenStream::new();
    let mut data_component_vec = data_component.iter().collect::<Vec<_>>();
    data_component_vec.sort_by_key(|(_, i)| **i);

    let mut next_internal_id = data_component_vec
        .last()
        .map_or(0, |(_, i)| **i)
        .saturating_add(1);

    let mut internal_vec = internal.iter().collect::<Vec<_>>();
    internal_vec.sort_by_key(|(_, i)| **i);

    // (name, wire id when the component is part of the synced registry)
    let all_entries: Vec<(&String, Option<u8>)> = data_component_vec
        .iter()
        .map(|(n, i)| (*n, Some(**i)))
        .chain(internal_vec.iter().map(|(n, _)| (*n, None)))
        .collect();

    for (raw_name, wire_id) in all_entries {
        let on_wire = wire_id.is_some();
        let raw_value = wire_id.unwrap_or_else(|| {
            let id = next_internal_id;
            next_internal_id += 1;
            id
        });
        let strip_name = raw_name
            .strip_prefix("minecraft:")
            .unwrap()
            .replace('/', "_");
        let pascal_case = format_ident!("{}", strip_name.to_pascal_case());

        // Enum variant

        enum_variants.extend(quote! {
            #pascal_case = #raw_value,
        });

        if on_wire {
            id_to_enum.extend(quote! {
                #raw_value => Some(Self::#pascal_case),
            });
        }

        // TODO use phf
        name_to_enum.extend(quote! {
            #raw_name | #strip_name => Some(Self::#pascal_case),
        });

        // Enum -> &str
        enum_to_name.extend(quote! {
            Self::#pascal_case => #raw_name,
        });
    }

    quote! {
        use crate::data_component_impl::*;

        #[derive(Copy, Clone, Hash, PartialEq, Eq)]
        #[repr(u8)]
        pub enum DataComponent {
            #enum_variants
        }

        impl DataComponent {
            #[must_use]
            pub const fn to_id(self) -> u8 {
                self as u8
            }

            #[must_use]
            #[allow(clippy::too_many_lines)]
            pub const fn try_from_id(id: u8) -> Option<Self> {
                match id {
                    #id_to_enum
                    _ => None,
                }
            }

            #[must_use]
            #[allow(clippy::too_many_lines)]
            pub fn try_from_name(name: &str) -> Option<Self> {
                match name {
                    #name_to_enum
                    _ => None,
                }
            }

            #[must_use]
            #[allow(clippy::too_many_lines)]
            pub const fn to_name(self) -> &'static str {
                match self {
                    #enum_to_name
                }
            }
        }
    }
}
