/* This file is generated. Do not edit manually. */
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum CatVariant {
    Tabby,
    Black,
    Red,
    Siamese,
    BritishShorthair,
    Calico,
    Persian,
    Ragdoll,
    White,
    Jellie,
    AllBlack,
}
impl CatVariant {
    #[doc = "Returns the cat variant from a resource name (bare or namespaced)."]
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "minecraft:tabby" | "tabby" => Some(Self::Tabby),
            "minecraft:black" | "black" => Some(Self::Black),
            "minecraft:red" | "red" => Some(Self::Red),
            "minecraft:siamese" | "siamese" => Some(Self::Siamese),
            "minecraft:british_shorthair" | "british_shorthair" => Some(Self::BritishShorthair),
            "minecraft:calico" | "calico" => Some(Self::Calico),
            "minecraft:persian" | "persian" => Some(Self::Persian),
            "minecraft:ragdoll" | "ragdoll" => Some(Self::Ragdoll),
            "minecraft:white" | "white" => Some(Self::White),
            "minecraft:jellie" | "jellie" => Some(Self::Jellie),
            "minecraft:all_black" | "all_black" => Some(Self::AllBlack),
            _ => None,
        }
    }
    #[doc = "Returns the numeric ID of the cat variant in the synced registry."]
    #[must_use]
    pub const fn id(&self) -> u8 {
        *self as u8
    }
    #[doc = "Returns the bare string name of the cat variant."]
    #[must_use]
    pub const fn to_name(&self) -> &'static str {
        match self {
            Self::Tabby => "tabby",
            Self::Black => "black",
            Self::Red => "red",
            Self::Siamese => "siamese",
            Self::BritishShorthair => "british_shorthair",
            Self::Calico => "calico",
            Self::Persian => "persian",
            Self::Ragdoll => "ragdoll",
            Self::White => "white",
            Self::Jellie => "jellie",
            Self::AllBlack => "all_black",
        }
    }
    #[doc = "Returns the fully-qualified asset id of the cat variant."]
    #[must_use]
    pub const fn asset_id(&self) -> &'static str {
        match self {
            Self::Tabby => "minecraft:tabby",
            Self::Black => "minecraft:black",
            Self::Red => "minecraft:red",
            Self::Siamese => "minecraft:siamese",
            Self::BritishShorthair => "minecraft:british_shorthair",
            Self::Calico => "minecraft:calico",
            Self::Persian => "minecraft:persian",
            Self::Ragdoll => "minecraft:ragdoll",
            Self::White => "minecraft:white",
            Self::Jellie => "minecraft:jellie",
            Self::AllBlack => "minecraft:all_black",
        }
    }
    #[doc = "Returns the texture asset path for the cat variant."]
    #[must_use]
    pub const fn texture(&self) -> &'static str {
        match self {
            Self::Tabby => "minecraft:textures/entity/cat/tabby.png",
            Self::Black => "minecraft:textures/entity/cat/black.png",
            Self::Red => "minecraft:textures/entity/cat/red.png",
            Self::Siamese => "minecraft:textures/entity/cat/siamese.png",
            Self::BritishShorthair => "minecraft:textures/entity/cat/british_shorthair.png",
            Self::Calico => "minecraft:textures/entity/cat/calico.png",
            Self::Persian => "minecraft:textures/entity/cat/persian.png",
            Self::Ragdoll => "minecraft:textures/entity/cat/ragdoll.png",
            Self::White => "minecraft:textures/entity/cat/white.png",
            Self::Jellie => "minecraft:textures/entity/cat/jellie.png",
            Self::AllBlack => "minecraft:textures/entity/cat/all_black.png",
        }
    }
    #[doc = "Returns all vanilla cat variants."]
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Tabby,
            Self::Black,
            Self::Red,
            Self::Siamese,
            Self::BritishShorthair,
            Self::Calico,
            Self::Persian,
            Self::Ragdoll,
            Self::White,
            Self::Jellie,
            Self::AllBlack,
        ]
    }
}
