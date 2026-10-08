use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// One vanilla potion mix: `from` potion + `ingredient` item -> `to` potion.
struct PotionMix {
    from: &'static str,
    ingredient: &'static str,
    to: &'static str,
}

/// Vanilla 1.21.1 potion mixes, hardcoded in `PotionBrewing` in the vanilla server.
/// The container item (potion, splash potion, lingering potion) carries over, so
/// every mix is valid for all three potion items.
const POTION_MIXES: &[PotionMix] = &[
    // Base conversions from water bottles
    PotionMix {
        from: "water",
        ingredient: "glowstone_dust",
        to: "thick",
    },
    PotionMix {
        from: "water",
        ingredient: "redstone",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "nether_wart",
        to: "awkward",
    },
    PotionMix {
        from: "water",
        ingredient: "magma_cream",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "rabbit_foot",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "sugar",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "glistering_melon_slice",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "spider_eye",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "ghast_tear",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "blaze_powder",
        to: "mundane",
    },
    PotionMix {
        from: "water",
        ingredient: "fermented_spider_eye",
        to: "weakness",
    },
    // Night vision and invisibility
    PotionMix {
        from: "awkward",
        ingredient: "golden_carrot",
        to: "night_vision",
    },
    PotionMix {
        from: "night_vision",
        ingredient: "redstone",
        to: "long_night_vision",
    },
    PotionMix {
        from: "night_vision",
        ingredient: "fermented_spider_eye",
        to: "invisibility",
    },
    PotionMix {
        from: "long_night_vision",
        ingredient: "fermented_spider_eye",
        to: "long_invisibility",
    },
    PotionMix {
        from: "invisibility",
        ingredient: "redstone",
        to: "long_invisibility",
    },
    // Fire resistance
    PotionMix {
        from: "awkward",
        ingredient: "magma_cream",
        to: "fire_resistance",
    },
    PotionMix {
        from: "fire_resistance",
        ingredient: "redstone",
        to: "long_fire_resistance",
    },
    // Leaping and slowness
    PotionMix {
        from: "awkward",
        ingredient: "rabbit_foot",
        to: "leaping",
    },
    PotionMix {
        from: "leaping",
        ingredient: "redstone",
        to: "long_leaping",
    },
    PotionMix {
        from: "leaping",
        ingredient: "glowstone_dust",
        to: "strong_leaping",
    },
    PotionMix {
        from: "leaping",
        ingredient: "fermented_spider_eye",
        to: "slowness",
    },
    PotionMix {
        from: "long_leaping",
        ingredient: "fermented_spider_eye",
        to: "long_slowness",
    },
    PotionMix {
        from: "slowness",
        ingredient: "redstone",
        to: "long_slowness",
    },
    PotionMix {
        from: "slowness",
        ingredient: "glowstone_dust",
        to: "strong_slowness",
    },
    // Turtle master
    PotionMix {
        from: "awkward",
        ingredient: "turtle_helmet",
        to: "turtle_master",
    },
    PotionMix {
        from: "turtle_master",
        ingredient: "redstone",
        to: "long_turtle_master",
    },
    PotionMix {
        from: "turtle_master",
        ingredient: "glowstone_dust",
        to: "strong_turtle_master",
    },
    // Swiftness and slowness
    PotionMix {
        from: "awkward",
        ingredient: "sugar",
        to: "swiftness",
    },
    PotionMix {
        from: "swiftness",
        ingredient: "redstone",
        to: "long_swiftness",
    },
    PotionMix {
        from: "swiftness",
        ingredient: "glowstone_dust",
        to: "strong_swiftness",
    },
    PotionMix {
        from: "swiftness",
        ingredient: "fermented_spider_eye",
        to: "slowness",
    },
    PotionMix {
        from: "long_swiftness",
        ingredient: "fermented_spider_eye",
        to: "long_slowness",
    },
    // Water breathing
    PotionMix {
        from: "awkward",
        ingredient: "pufferfish",
        to: "water_breathing",
    },
    PotionMix {
        from: "water_breathing",
        ingredient: "redstone",
        to: "long_water_breathing",
    },
    // Healing and harming
    PotionMix {
        from: "awkward",
        ingredient: "glistering_melon_slice",
        to: "healing",
    },
    PotionMix {
        from: "healing",
        ingredient: "glowstone_dust",
        to: "strong_healing",
    },
    PotionMix {
        from: "healing",
        ingredient: "fermented_spider_eye",
        to: "harming",
    },
    PotionMix {
        from: "strong_healing",
        ingredient: "fermented_spider_eye",
        to: "strong_harming",
    },
    PotionMix {
        from: "harming",
        ingredient: "glowstone_dust",
        to: "strong_harming",
    },
    PotionMix {
        from: "poison",
        ingredient: "fermented_spider_eye",
        to: "harming",
    },
    PotionMix {
        from: "long_poison",
        ingredient: "fermented_spider_eye",
        to: "harming",
    },
    PotionMix {
        from: "strong_poison",
        ingredient: "fermented_spider_eye",
        to: "strong_harming",
    },
    // Poison
    PotionMix {
        from: "awkward",
        ingredient: "spider_eye",
        to: "poison",
    },
    PotionMix {
        from: "poison",
        ingredient: "redstone",
        to: "long_poison",
    },
    PotionMix {
        from: "poison",
        ingredient: "glowstone_dust",
        to: "strong_poison",
    },
    // Regeneration
    PotionMix {
        from: "awkward",
        ingredient: "ghast_tear",
        to: "regeneration",
    },
    PotionMix {
        from: "regeneration",
        ingredient: "redstone",
        to: "long_regeneration",
    },
    PotionMix {
        from: "regeneration",
        ingredient: "glowstone_dust",
        to: "strong_regeneration",
    },
    // Strength
    PotionMix {
        from: "awkward",
        ingredient: "blaze_powder",
        to: "strength",
    },
    PotionMix {
        from: "strength",
        ingredient: "redstone",
        to: "long_strength",
    },
    PotionMix {
        from: "strength",
        ingredient: "glowstone_dust",
        to: "strong_strength",
    },
    // Weakness
    PotionMix {
        from: "weakness",
        ingredient: "redstone",
        to: "long_weakness",
    },
    // Slow falling
    PotionMix {
        from: "awkward",
        ingredient: "phantom_membrane",
        to: "slow_falling",
    },
    PotionMix {
        from: "slow_falling",
        ingredient: "redstone",
        to: "long_slow_falling",
    },
];

/// Every potion content that exists in vanilla 1.21.1 and can be carried by the
/// three potion items. Container conversions keep the potion content.
const POTION_CONTENTS: &[&str] = &[
    "water",
    "mundane",
    "thick",
    "awkward",
    "night_vision",
    "long_night_vision",
    "invisibility",
    "long_invisibility",
    "leaping",
    "long_leaping",
    "strong_leaping",
    "slowness",
    "long_slowness",
    "strong_slowness",
    "fire_resistance",
    "long_fire_resistance",
    "swiftness",
    "long_swiftness",
    "strong_swiftness",
    "turtle_master",
    "long_turtle_master",
    "strong_turtle_master",
    "water_breathing",
    "long_water_breathing",
    "healing",
    "strong_healing",
    "harming",
    "strong_harming",
    "poison",
    "long_poison",
    "strong_poison",
    "regeneration",
    "long_regeneration",
    "strong_regeneration",
    "strength",
    "long_strength",
    "strong_strength",
    "weakness",
    "long_weakness",
    "slow_falling",
    "long_slow_falling",
];

/// The three potion container items a mix can happen in.
const CONTAINERS: &[&str] = &["potion", "splash_potion", "lingering_potion"];

fn recipe_tokens(
    from_item: &str,
    from_potion: &str,
    ingredient: &str,
    to_item: &str,
    to_potion: &str,
) -> TokenStream {
    let from_item = format_ident!("{}", from_item.to_uppercase());
    let from_potion = format_ident!("{}", from_potion.to_uppercase());
    let ingredient = format_ident!("{}", ingredient.to_uppercase());
    let to_item = format_ident!("{}", to_item.to_uppercase());
    let to_potion = format_ident!("{}", to_potion.to_uppercase());

    quote! {
        BrewingRecipe {
            from_item: &Item::#from_item,
            from_potion: &Potion::#from_potion,
            ingredient: &Item::#ingredient,
            to_item: &Item::#to_item,
            to_potion: &Potion::#to_potion,
        },
    }
}

/// Generates the `TokenStream` for `BREWING_RECIPES`, `POTION_RECIPES` and `ITEM_RECIPES`.
pub fn build() -> TokenStream {
    let mut brewing_tokens = Vec::new();

    // Vanilla 1.21.1 hardcodes brewing in the server; the datapack gained brewing
    // recipes only later. Every potion mix applies to each potion container item.
    for mix in POTION_MIXES {
        for container in CONTAINERS {
            brewing_tokens.push(recipe_tokens(
                container,
                mix.from,
                mix.ingredient,
                container,
                mix.to,
            ));
        }
    }

    // Container conversions: gunpowder turns potions into splash potions and
    // dragon breath turns splash potions into lingering potions. The potion
    // content carries over.
    for potion in POTION_CONTENTS {
        brewing_tokens.push(recipe_tokens(
            "potion",
            potion,
            "gunpowder",
            "splash_potion",
            potion,
        ));
        brewing_tokens.push(recipe_tokens(
            "splash_potion",
            potion,
            "dragon_breath",
            "lingering_potion",
            potion,
        ));
    }

    let brewing_len = brewing_tokens.len();

    quote! {
        /* This file is generated. Do not edit manually. */
        #![allow(dead_code)]
        use crate::item::Item;
        use crate::potion::Potion;

        pub struct BrewingRecipe {
            pub from_item: &'static Item,
            pub from_potion: &'static Potion,
            pub ingredient: &'static Item,
            pub to_item: &'static Item,
            pub to_potion: &'static Potion,
        }

        impl BrewingRecipe {
            #[must_use]
            pub const fn from_item(&self) -> &'static Item { self.from_item }
            #[must_use]
            pub const fn from_potion(&self) -> &'static Potion { self.from_potion }
            #[must_use]
            pub const fn ingredient(&self) -> &'static Item { self.ingredient }
            #[must_use]
            pub const fn reagent(&self) -> &'static Item { self.ingredient }
            #[must_use]
            pub const fn to_item(&self) -> &'static Item { self.to_item }
            #[must_use]
            pub const fn to_potion(&self) -> &'static Potion { self.to_potion }
        }

        pub struct PotionRecipe {
            from: &'static Potion,
            ingredient: &'static [&'static Item],
            to: &'static Potion,
        }

        pub struct ItemRecipe {
            from: &'static Item,
            ingredient: &'static [&'static Item],
            to: &'static Item,
        }

        impl PotionRecipe {
            #[must_use]
            pub const fn from(&self) -> &'static Potion { self.from }
            #[must_use]
            pub const fn ingredient(&self) -> &'static [&'static Item] { self.ingredient }
            #[must_use]
            pub const fn to(&self) -> &'static Potion { self.to }
        }

        impl ItemRecipe {
            #[must_use]
            pub const fn from(&self) -> &'static Item { self.from }
            #[must_use]
            pub const fn ingredient(&self) -> &'static [&'static Item] { self.ingredient }
            #[must_use]
            pub const fn to(&self) -> &'static Item { self.to }
        }

        pub const BREWING_RECIPES: [BrewingRecipe; #brewing_len] = [#(#brewing_tokens)*];
        pub const ITEM_RECIPES: [ItemRecipe; 0] = [];
        pub const POTION_RECIPES: [PotionRecipe; 0] = [];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_vanilla_brewing_table() {
        let generated = build().to_string();

        // 55 potion mixes * 3 containers + 41 contents * 2 container conversions.
        assert_eq!(POTION_MIXES.len(), 55);
        assert_eq!(POTION_CONTENTS.len(), 41);
        assert!(generated.contains("247"));
    }
}
