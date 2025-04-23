# Epic Games plugin

This plugin allows to interact with the Epic Games Store via [Legendary](https://github.com/derrod/legendary) and a DBus service.

The `epic/service.rs` module is responsible for bridging Legendary and DBus.
Functions in this module should return `fdo::Result<T>`.
Functionality should be implemented in this service, and the DBus interface
defined in the `plugin` module should only serve as a thin wrapper around
this service. This allows swapping one service with another easilly.


## TODO

 * Sync installed apps
 * Pause install
 * Get install options 
