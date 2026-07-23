use bevy::prelude::*;
use std::collections::BTreeSet;
use swarm_engine_api::prelude::{
    API_VERSION, ActionDescriptor, ConfigFieldDescriptor, ConfigValidator, ConfigValueType,
    DESCRIPTOR_SCHEMA_VERSION, PluginDependency, PluginDescriptor, SystemDescriptor, TickPhase,
};
use swarm_engine_plugin_sdk::{
    buffers::{PendingSpecialAttack, SpecialAttackKind, StatusActionIntent},
    resources::{ActionRegistrationError, ActionRegistry},
    traits::SwarmPlugin,
};

#[derive(Resource, Debug, Clone)]
pub struct SpecialAttacksConfig {
    pub enabled: BTreeSet<SpecialAttackKind>,
    pub damage_multiplier: u32,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionRegistrationFailures {
    pub errors: Vec<ActionRegistrationError>,
}

impl Default for SpecialAttacksConfig {
    fn default() -> Self {
        Self {
            enabled: [
                SpecialAttackKind::Hack,
                SpecialAttackKind::Drain,
                SpecialAttackKind::Overload,
                SpecialAttackKind::Debilitate,
                SpecialAttackKind::Disrupt,
                SpecialAttackKind::Fortify,
                SpecialAttackKind::Leech,
                SpecialAttackKind::Fabricate,
            ]
            .into_iter()
            .collect(),
            damage_multiplier: 10_000,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SpecialAttacksModPlugin;

impl Plugin for SpecialAttacksModPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpecialAttacksConfig>()
            .init_resource::<PendingSpecialAttack>()
            .init_resource::<ActionRegistry>()
            .init_resource::<ActionRegistrationFailures>()
            .add_systems(Startup, register_special_actions);
    }
}

impl SwarmPlugin for SpecialAttacksModPlugin {
    fn descriptor() -> PluginDescriptor {
        PluginDescriptor {
            id: "special-attacks".to_string(),
            version: "0.1.0".to_string(),
            api_version: API_VERSION.to_string(),
            dependencies: vec![PluginDependency {
                id: "combat-core".to_string(),
                version_req: ">=0.1.0".to_string(),
            }],
            config: vec![
                ConfigFieldDescriptor {
                    key: "special_attacks_enabled".to_string(),
                    value_type: ConfigValueType::Bool,
                    default: true.into(),
                    required: false,
                    validator: None,
                },
                special_attack_set_config(
                    "enabled",
                    &[
                        "Hack",
                        "Drain",
                        "Overload",
                        "Debilitate",
                        "Disrupt",
                        "Fortify",
                        "Leech",
                        "Fabricate",
                    ],
                ),
                special_attack_set_config("tutorial_enabled", &["Hack", "Drain", "Fortify"]),
                special_attack_set_config(
                    "novice_enabled",
                    &["Hack", "Drain", "Overload", "Fortify"],
                ),
                ConfigFieldDescriptor {
                    key: "damage_multiplier".to_string(),
                    value_type: ConfigValueType::FixedBasisPoints,
                    default: 10_000_u32.into(),
                    required: false,
                    validator: Some(ConfigValidator::FixedBasisPoints),
                },
                ConfigFieldDescriptor {
                    key: "fabricate_allowed_output_structures".to_string(),
                    value_type: ConfigValueType::Array {
                        item_type: "StructureType".to_string(),
                    },
                    default: serde_json::json!(["Tower"]),
                    required: false,
                    validator: Some(ConfigValidator::NonEmptyArray),
                },
            ],
            systems: vec![SystemDescriptor {
                system_id: "special-attacks.register".to_string(),
                version: "0.1.0".to_string(),
                phase: TickPhase::Startup,
                order: 0,
                reads: vec!["SpecialAttacksConfig".to_string()],
                writes: vec![
                    "ActionRegistry".to_string(),
                    "ActionRegistrationFailures".to_string(),
                ],
                produces_buffers: Vec::new(),
                consumes_buffers: Vec::new(),
                deterministic_iteration: vec!["SpecialAttackKind".to_string()],
            }],
            actions: [
                "Hack",
                "Drain",
                "Overload",
                "Debilitate",
                "Disrupt",
                "Fortify",
                "Leech",
                "Fabricate",
            ]
            .into_iter()
            .map(|action_type| ActionDescriptor {
                action_type: action_type.to_string(),
                handler: action_type.to_lowercase(),
                payload_schema: if action_type == "Fabricate" {
                    fabricate_payload_schema()
                } else {
                    special_attack_payload_schema()
                },
                command_phase: TickPhase::Command,
                output_buffer: Some("PendingSpecialAttack".to_string()),
            })
            .collect(),
            descriptor_schema_version: DESCRIPTOR_SCHEMA_VERSION.to_string(),
        }
    }
}

fn special_attack_set_config(key: &str, defaults: &[&str]) -> ConfigFieldDescriptor {
    ConfigFieldDescriptor {
        key: key.to_string(),
        value_type: ConfigValueType::Array {
            item_type: "SpecialAttack".to_string(),
        },
        default: serde_json::json!(defaults),
        required: false,
        validator: None,
    }
}

fn special_attack_payload_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["object_id", "target_id"],
        "properties": {
            "object_id": { "type": "integer", "minimum": 0, "maximum": 18_446_744_073_709_551_615_u64 },
            "target_id": { "type": "integer", "minimum": 0, "maximum": 18_446_744_073_709_551_615_u64 },
            "resource": { "type": "string", "minLength": 1 },
            "amount": { "type": "integer", "minimum": 0, "maximum": 4_294_967_295_u64 },
            "range": { "type": "integer", "minimum": 0, "maximum": 4_294_967_295_u64 },
            "structure": { "type": "string", "minLength": 1 },
            "damage_type": { "type": "string", "minLength": 1 },
            "cooldown": { "type": "integer", "minimum": 0, "maximum": 4_294_967_295_u64 }
        }
    })
}

fn fabricate_payload_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["target_id"],
        "properties": {
            "target_id": { "type": "integer", "minimum": 0, "maximum": 18_446_744_073_709_551_615_u64 }
        }
    })
}

pub fn register_special_actions(
    config: Res<SpecialAttacksConfig>,
    mut registry: ResMut<ActionRegistry>,
    mut failures: ResMut<ActionRegistrationFailures>,
) {
    if let Err(error) = try_register_special_actions(&config, &mut registry) {
        failures.errors.push(error);
    }
}

fn try_register_special_actions(
    config: &SpecialAttacksConfig,
    registry: &mut ActionRegistry,
) -> Result<(), ActionRegistrationError> {
    for (kind, action_type, handler) in [
        (SpecialAttackKind::Hack, "Hack", "hack"),
        (SpecialAttackKind::Drain, "Drain", "drain"),
        (SpecialAttackKind::Overload, "Overload", "overload"),
        (SpecialAttackKind::Debilitate, "Debilitate", "debilitate"),
        (SpecialAttackKind::Disrupt, "Disrupt", "disrupt"),
        (SpecialAttackKind::Fortify, "Fortify", "fortify"),
        (SpecialAttackKind::Leech, "Leech", "leech"),
        (SpecialAttackKind::Fabricate, "Fabricate", "fabricate"),
    ] {
        if config.enabled.contains(&kind) {
            registry.register(action_type, handler)?;
        }
    }
    Ok(())
}

pub fn enqueue_special_attack(
    pending: &mut PendingSpecialAttack,
    kind: SpecialAttackKind,
    source: Entity,
    target: Entity,
    owner: u32,
    amount: u32,
) {
    pending.intents.push(StatusActionIntent {
        kind,
        source,
        target,
        owner,
        amount,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_enables_all_special_attack_kinds() {
        let config = SpecialAttacksConfig::default();

        assert_eq!(config.enabled.len(), 8);
        assert!(config.enabled.contains(&SpecialAttackKind::Hack));
        assert!(config.enabled.contains(&SpecialAttackKind::Fabricate));
        assert_eq!(config.damage_multiplier, 10_000);
    }

    #[test]
    fn descriptor_is_valid_and_declares_combat_dependency() {
        let descriptor = SpecialAttacksModPlugin::descriptor();
        swarm_engine_api::validation::assert_valid_descriptor(&descriptor);
        assert_eq!(descriptor.id, "special-attacks");
        assert_eq!(descriptor.dependencies[0].id, "combat-core");
    }

    #[test]
    fn descriptor_matches_canonical_config_and_action_contracts() {
        let descriptor = SpecialAttacksModPlugin::descriptor();
        let keys: Vec<_> = descriptor
            .config
            .iter()
            .map(|field| field.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "special_attacks_enabled",
                "enabled",
                "tutorial_enabled",
                "novice_enabled",
                "damage_multiplier",
                "fabricate_allowed_output_structures",
            ]
        );
        let fabricate = descriptor
            .config
            .iter()
            .find(|field| field.key == "fabricate_allowed_output_structures")
            .expect("fabricate output config descriptor");
        assert_eq!(fabricate.default, serde_json::json!(["Tower"]));
        assert_eq!(fabricate.validator, Some(ConfigValidator::NonEmptyArray));
        assert_eq!(descriptor.systems.len(), 1);
        assert_eq!(descriptor.actions.len(), 8);
        for action in &descriptor.actions {
            assert_eq!(action.payload_schema["additionalProperties"], false);
            let required = if action.action_type == "Fabricate" {
                serde_json::json!(["target_id"])
            } else {
                serde_json::json!(["object_id", "target_id"])
            };
            assert_eq!(action.payload_schema["required"], required);
            assert_eq!(
                action.output_buffer.as_deref(),
                Some("PendingSpecialAttack")
            );
        }
        let fabricate_action = descriptor
            .actions
            .iter()
            .find(|action| action.action_type == "Fabricate")
            .unwrap();
        assert_eq!(
            fabricate_action.payload_schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .collect::<Vec<_>>(),
            vec!["target_id"]
        );
    }

    #[test]
    fn duplicate_action_registration_is_typed_and_preserves_original_handler() {
        let mut registry = ActionRegistry::default();
        registry.register("Hack", "original").unwrap();

        assert_eq!(
            registry.register("Hack", "replacement"),
            Err(ActionRegistrationError::DuplicateActionType {
                action_type: "Hack".to_string(),
            })
        );
        assert_eq!(
            registry.handlers.get("Hack").map(String::as_str),
            Some("original")
        );
    }

    #[test]
    fn startup_records_duplicate_registration_without_replacing_handler() {
        let mut app = App::new();
        let mut registry = ActionRegistry::default();
        registry.register("Hack", "original").unwrap();
        app.insert_resource(registry)
            .add_plugins(SpecialAttacksModPlugin);

        app.update();

        assert_eq!(
            app.world().resource::<ActionRegistrationFailures>().errors,
            [ActionRegistrationError::DuplicateActionType {
                action_type: "Hack".to_string(),
            }]
        );
        assert_eq!(
            app.world()
                .resource::<ActionRegistry>()
                .handlers
                .get("Hack")
                .map(String::as_str),
            Some("original")
        );
    }
}
