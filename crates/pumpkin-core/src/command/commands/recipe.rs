use crate::command::argument_builder::{ArgumentBuilder, argument, command, literal};
use crate::command::argument_types::entity::EntityArgumentType;
use crate::command::argument_types::resource_key::{RECIPE_REGISTRY, ResourceKeyArgument};
use crate::command::context::command_context::CommandContext;
use crate::command::context::command_source::CommandSource;
use crate::command::errors::command_syntax_error::CommandSyntaxError;
use crate::command::errors::error_types::CommandErrorType;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::command::suggestion::provider::{SuggestionProvider, SuggestionProviderResult};
use crate::command::suggestion::suggestions::SuggestionsBuilder;
use crate::entity::EntityBase;
use crate::entity::player::Player;
use pumpkin_data::translation;
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;
use std::sync::Arc;

const NAME: &str = "recipe";
const DESCRIPTION: &str = "unlock or lock recipes for players";
const PERMISSION: &str = "minecraft:command.recipe";

const ARG_TARGETS: &str = "targets";
const ARG_RECIPE: &str = "recipe";

const ERROR_GIVE_FAILED: CommandErrorType<0> = CommandErrorType::new(
    translation::java::COMMANDS_RECIPE_GIVE_FAILED,
    translation::java::COMMANDS_RECIPE_GIVE_FAILED,
);
const ERROR_TAKE_FAILED: CommandErrorType<0> = CommandErrorType::new(
    translation::java::COMMANDS_RECIPE_TAKE_FAILED,
    translation::java::COMMANDS_RECIPE_TAKE_FAILED,
);

/// Vanilla `RecipeCommand#giveRecipes`/`takeRecipes`: the feedback counts the
/// requested recipes, the return value the ones that actually changed.
fn perform(
    context: &CommandSource,
    targets: &[Arc<Player>],
    give: bool,
    ids: &[String],
) -> Result<i32, CommandSyntaxError> {
    let mut changed = 0;
    for player in targets {
        changed += if give {
            player.award_recipes(ids)
        } else {
            player.reset_recipes(ids)
        };
    }
    if changed == 0 {
        return Err(if give {
            &ERROR_GIVE_FAILED
        } else {
            &ERROR_TAKE_FAILED
        }
        .create_without_context());
    }

    let feedback = if let [player] = targets {
        TextComponent::translate(
            if give {
                translation::java::COMMANDS_RECIPE_GIVE_SUCCESS_SINGLE
            } else {
                translation::java::COMMANDS_RECIPE_TAKE_SUCCESS_SINGLE
            },
            [
                TextComponent::text(ids.len().to_string()),
                player.get_display_name(),
            ],
        )
    } else {
        TextComponent::translate(
            if give {
                translation::java::COMMANDS_RECIPE_GIVE_SUCCESS_MULTIPLE
            } else {
                translation::java::COMMANDS_RECIPE_TAKE_SUCCESS_MULTIPLE
            },
            [
                TextComponent::text(ids.len().to_string()),
                TextComponent::text(targets.len().to_string()),
            ],
        )
    };
    context.send_feedback(feedback, true);
    Ok(changed)
}

struct RecipeExecutor {
    give: bool,
    all: bool,
}

impl CommandExecutor for RecipeExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let targets = EntityArgumentType::get_players(context, ARG_TARGETS)?;
        let ids = if self.all {
            context.server().recipe_manager.all_ids()
        } else {
            vec![ResourceKeyArgument::get_recipe_id(context, ARG_RECIPE)?]
        };
        perform(&context.source, &targets, self.give, &ids)
    }
}

/// Suggests every registered recipe key.
struct RecipeSuggestionProvider;

impl SuggestionProvider for RecipeSuggestionProvider {
    fn suggest(
        &self,
        context: &CommandContext,
        builder: SuggestionsBuilder,
    ) -> SuggestionProviderResult {
        let ids = context.server().recipe_manager.all_ids();
        builder.filter_and_suggest_iter(ids).build()
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    macro_rules! build_action {
        ($name:expr, $give:expr) => {
            literal($name).then(
                argument(ARG_TARGETS, EntityArgumentType::Players)
                    .then(
                        argument(ARG_RECIPE, ResourceKeyArgument(&RECIPE_REGISTRY))
                            .suggests(RecipeSuggestionProvider)
                            .executes(RecipeExecutor {
                                give: $give,
                                all: false,
                            }),
                    )
                    .then(literal("*").executes(RecipeExecutor {
                        give: $give,
                        all: true,
                    })),
            )
        };
    }

    dispatcher.register(
        command(NAME, DESCRIPTION)
            .requires(PERMISSION)
            .then(build_action!("give", true))
            .then(build_action!("take", false)),
    );
}
