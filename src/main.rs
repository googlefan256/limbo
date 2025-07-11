#![allow(clippy::type_complexity)]

use std::env;

use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::prelude::*;

const SPAWN_Y: i32 = 64;

pub fn main() {
    let addr = env::var("VALENCE_ADDR").unwrap_or("0.0.0.0:25565".into());
    let secret = env::var("VALENCE_SECRET").unwrap_or("pasta".into());
    App::new()
        .insert_resource(NetworkSettings {
            address: addr.parse().expect("Invalid address"),
            connection_mode: ConnectionMode::Velocity {
                secret: secret.into(),
            },
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                toggle_gamemode_on_sneak,
                digging,
                place_blocks,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -50..50 {
        for x in -50..50 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, f64::from(SPAWN_Y) + 1.0, 0.0]);
        *game_mode = GameMode::Creative;

        client
            .send_chat_message("ロビーサーバーです。/serverでサーバー間移動をできます。".italic());
    }
}

fn toggle_gamemode_on_sneak(
    mut clients: Query<&mut GameMode>,
    mut events: EventReader<SneakEvent>,
) {
    for event in events.read() {
        let Ok(mut mode) = clients.get_mut(event.client) else {
            continue;
        };
        if event.state == SneakState::Start {
            *mode = match *mode {
                GameMode::Survival => GameMode::Creative,
                GameMode::Creative => GameMode::Survival,
                _ => GameMode::Creative,
            };
        }
    }
}

fn digging(
    clients: Query<&GameMode>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };

        if (*game_mode == GameMode::Creative && event.state == DiggingState::Start)
            || (*game_mode == GameMode::Survival && event.state == DiggingState::Stop)
        {
            layer.set_block(event.position, BlockState::AIR);
        }
    }
}

fn place_blocks(
    mut clients: Query<(&mut Inventory, &GameMode, &HeldItem)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((mut inventory, game_mode, held)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            continue;
        };

        if *game_mode == GameMode::Survival {
            if stack.count > 1 {
                let amount = stack.count - 1;
                inventory.set_slot_amount(slot_id, amount);
            } else {
                inventory.set_slot(slot_id, ItemStack::EMPTY);
            }
        }
        let real_pos = event.position.get_in_direction(event.face);
        let state = block_kind.to_state().set(
            PropName::Axis,
            match event.face {
                Direction::Down | Direction::Up => PropValue::Y,
                Direction::North | Direction::South => PropValue::Z,
                Direction::West | Direction::East => PropValue::X,
            },
        );
        layer.set_block(real_pos, state);
    }
}
