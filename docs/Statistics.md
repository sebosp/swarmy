# Statistics

The snapshot written by Optimize contains the s2protocol-rs `FlatRow` structs:
`PlayerLobbyDetailsFlatRow` (details.arrow), `PlayerStatsFlatRow` (stats.arrow),
`UpgradeEventFlatRow` (upgrades.arrow), `UnitBornEventFlatRow` (unit_born.arrow),
`UnitDiedEventFlatRow` (unit_died.arrow), `CmdTargetPointEventFlatRow` (cmd_target_point.arrow) and
`CmdTargetUnitEventFlatRow` (cmd_target_unit.arrow). `UserInitDataFlatRow` exists but is not written
to the snapshot yet. Ideas of what could be shown with them:

Most of them are in the Player view, from the analysis of every game of the player, and in the
Replay Detail view, see the Help view for how each one is calculated.

## For players that want to improve
- [x] Benchmarks: workers, supply, income and army value at 4, 6, 8 and 10 minutes against the snapshot average of the same matchup.
- [x] Supply blocks: time with `food_used` reaching `food_made` below 200.
- [x] Floating resources: time with more than 1000 minerals and vespene unspent.
- [x] Expansion timings (Replay Detail) and upgrade priorities (Player). Comparing them to the pros of the same map and matchup is still to do.
- [x] Expansion and upgrade timings compared to the average of the games of the same patch (Replay Detail), i.e. "Metabolic Boost 5s faster than average".
- [x] Map resources (Map Details): minerals and vespene of the map, gold minerals and rich vespene, and the distance from each start location to its main base mineral fields and geysers, from the Objects of the map cache, over the Minimap.tga of the map cache.
- [x] Trades: engagements with the resources and mining time lost by each side, and the trade ratio of each unit type.
- [x] What kills my units: losses by killer unit type, per game and per player.
- [x] Where I lose my units on each map: the Deaths layer of Map Scout.
- [x] Idle production: time between units of the same production structure.
- [x] Macro cadence: injects, chrono boosts and MULEs per game. The intervals between them are still to do.
- [x] Commands per minute through the game, per minute in Replay Detail and on average in Player.
- [x] Player focus: the selections and commands of each minute by army, macro, economy, expand and upgrades, and the longest time with only army actions (Replay Detail). Read from the replay files, selections are not in the snapshot, so it is not averaged across games in the Player view yet.
- [x] Win rates by matchup, map and game length.
- [x] Rating progression, read from the replays as `UserInitDataFlatRow` is not in the snapshot. Pro games played on LAN have no rating.

## For casters of online tournaments
- [x] Openings: the first three tech structures and the first army unit, by matchup and map, with the share of games and win rate.
- [x] Head to head records and recent form (Player, opponents and the last ten games).
- [x] Proxies: production structures far from the start location before 04:00.
- [x] Timing of the first attack: the first enemy unit killed.
- [x] Favourite army composition at 10 minutes and the most produced army unit by map.
- [x] Comebacks: games won after being below 70% of the enemy army value.
- [x] Bases at 10 minutes and how often games end in a base trade.
- [x] Upgrade priorities: the first upgrade researched.
- [x] Map veto help: win rate of each player per map.
- [x] Scouting habits: when and with which unit the enemy start location is first scouted, and Map Scout.

## Missing data
- [ ] Camera positions are only read from the replay files, adding them to the snapshot would allow camera based statistics (where the player looks) across games.
- [x] The chat messages are read once into the personal database, the Replay List searches them.
- [x] The protocol version of each replay is in details.arrow (`ext_protocol_version`), with the s2protocol-rs `fix-snapshot-ipc-schemas` branch.
- [x] metadata.json in the snapshot with the s2protocol-rs version that wrote it, shown in the Scan statistics with whether it is compatible.
- [x] The snapshot is partitioned by day (`ipcs/year=YYYY/month=MM/day=DD/`), Optimize only adds the replays that are not in it yet and Rebuild generates it from scratch.
- [ ] Selections (control groups and selection deltas) are not in the snapshot, the focus statistics need the replay files.
- [ ] The `ext_replay_seconds` column is computed by s2protocol-rs with 68.58 ms per game loop, about 14.6 loops per second, Faster speed is 22.4. Swarmy computes the seconds from `ext_replay_loop` instead.
