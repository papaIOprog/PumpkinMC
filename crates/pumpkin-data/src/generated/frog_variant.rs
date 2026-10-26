/* This file is generated. Do not edit manually. */
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u32)]
pub enum FrogVariant {
    Cold,
    #[default]
    Temperate,
    Warm,
}
impl FrogVariant {
    #[doc = "Returns the frog variant from a resource name (bare or namespaced)."]
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "minecraft:temperate" | "temperate" => Some(Self::Temperate),
            "minecraft:warm" | "warm" => Some(Self::Warm),
            "minecraft:cold" | "cold" => Some(Self::Cold),
            _ => None,
        }
    }
    #[doc = "Returns the numeric ID of the frog variant in the synced registry."]
    #[must_use]
    pub const fn id(&self) -> u32 {
        *self as u32
    }
    #[doc = "Returns the frog variant from numeric ID."]
    #[must_use]
    pub const fn from_id(id: u32) -> Self {
        match id {
            0 => Self::Cold,
            2 => Self::Warm,
            _ => Self::Temperate,
        }
    }
    #[doc = "Returns the bare string name of the frog variant."]
    #[must_use]
    pub const fn to_name(&self) -> &'static str {
        match self {
            Self::Temperate => "temperate",
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }
    #[doc = "Returns the fully-qualified asset id of the frog variant."]
    #[must_use]
    pub const fn asset_id(&self) -> &'static str {
        match self {
            Self::Temperate => "minecraft:temperate",
            Self::Warm => "minecraft:warm",
            Self::Cold => "minecraft:cold",
        }
    }
    #[doc = "Returns the texture asset path for the frog variant."]
    #[must_use]
    pub const fn texture(&self) -> &'static str {
        match self {
            Self::Temperate => "minecraft:textures/entity/frog/temperate_frog.png",
            Self::Warm => "minecraft:textures/entity/frog/warm_frog.png",
            Self::Cold => "minecraft:textures/entity/frog/cold_frog.png",
        }
    }
    #[doc = "Returns all vanilla frog variants."]
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Temperate, Self::Warm, Self::Cold]
    }
}
