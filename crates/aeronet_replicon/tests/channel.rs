#![expect(missing_docs, reason = "testing")]
#![cfg(test)]

use {
    aeronet_channel::{ChannelIo, ChannelIoPlugin},
    aeronet_io::server::Server,
    aeronet_replicon::{
        client::{AeronetRepliconClient, AeronetRepliconClientPlugin},
        server::{AeronetRepliconServer, AeronetRepliconServerPlugin},
    },
    bevy_app::{TaskPoolPlugin, prelude::*},
    bevy_ecs::prelude::*,
    bevy_platform::time::Instant,
    bevy_replicon::{prelude::*, shared::server_entity_map::ServerEntityMap},
    bevy_state::app::StatesPlugin,
    bevy_time::{TimePlugin, TimeUpdateStrategy},
    core::time::Duration,
    serde::{Deserialize, Serialize},
};

// <https://github.com/aecsocket/aeronet/pull/76>
#[test]
fn channel_io_with_replicon() {
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        TimePlugin,
        StatesPlugin,
        ChannelIoPlugin,
        RepliconPlugins,
        AeronetRepliconClientPlugin,
        AeronetRepliconServerPlugin,
    ))
    .finish();

    let a = app.world_mut().spawn_empty().id();
    let b = app.world_mut().spawn_empty().id();
    ChannelIo::open(a, b).apply(app.world_mut());

    app.update();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Serialize, Deserialize)]
struct Counter(u32);

#[test]
fn channel_io_replicates_components() {
    let mut server = App::new();
    server
        .add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            StatesPlugin,
            ChannelIoPlugin,
            RepliconPlugins,
            AeronetRepliconServerPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .replicate::<Counter>()
        .finish();

    let mut client = App::new();
    client
        .add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            StatesPlugin,
            ChannelIoPlugin,
            RepliconPlugins,
            AeronetRepliconClientPlugin,
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .replicate::<Counter>()
        .finish();

    let endpoint = server
        .world_mut()
        .spawn((AeronetRepliconServer, Server::new(Instant::now())))
        .id();
    let (client_io, server_io) = ChannelIo::new();
    server.world_mut().spawn((ChildOf(endpoint), server_io));
    client.world_mut().spawn((AeronetRepliconClient, client_io));

    let entity = server.world_mut().spawn((Replicated, Counter(7))).id();
    for _ in 0..16 {
        server.update();
        client.update();
    }

    let remote = *client
        .world()
        .resource::<ServerEntityMap>()
        .to_client()
        .get(&entity)
        .expect("server entity reaches the client");
    assert_eq!(client.world().get::<Counter>(remote), Some(&Counter(7)));

    server.world_mut().entity_mut(entity).insert(Counter(11));
    for _ in 0..16 {
        server.update();
        client.update();
    }
    assert_eq!(client.world().get::<Counter>(remote), Some(&Counter(11)));

    server.world_mut().entity_mut(entity).despawn();
    for _ in 0..16 {
        server.update();
        client.update();
    }
    assert!(client.world().get_entity(remote).is_err());
}
