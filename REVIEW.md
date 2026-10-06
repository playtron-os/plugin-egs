# Review checks

Cloud save methods and signals implement `one.playtron.plugin.CloudSaveProvider`.
Check the D-Bus payload as well as command exit handling when changing them.

Confirmed consumer: `playtron-os/playserve` decodes these signals in
`crates/plugins/src/models/zbus_generated_interfaces/cloud_save_provider.rs`,
defines their payloads in `crates/models/src/cloud_sync.rs`, and handles progress
and failure in `service/src/dbus/plugin.rs`. Its failure payload includes
`sync_state` after the quota field. Completion requires both a successful method
reply and terminal progress for the requested operation.

Run `cargo test` with `dbus-daemon` available. The cloud completion tests use a
private bus and a controlled Legendary executable; they do not contact Epic.
