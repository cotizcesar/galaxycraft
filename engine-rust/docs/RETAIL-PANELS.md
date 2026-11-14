# Retail panels, unit by unit

Every placeable `units.dat` type (190 of 228: turret subunits, spell
effects, powerups, unused rows and the start marker excluded) was placed for
player 1 on a generated Use-Map-Settings map, opened in the retail game
(StarCraft Remastered, 2026-11-14) and clicked one by one.  For each unit the
panel name was verified, then its health/shields line and the tooltips of its
armor and weapon icons were captured and read by OCR; rows the OCR could not
parse were read by eye from the same captures.  The values below are the
executable's tables (`fixtures/oracle_*.tsv`, the same values the engine
uses); "match" means the retail panel shows exactly them.

Tooltip damage is the weapon's damage, doubled for two-blow weapons (Zealot
16, Firebat 16, Goliath air 20).  Protoss panels print shields then health.
Indestructible resources and doodads show no health line.

Tools: `scripts/retail_panels/` (`make_map.py` writes the map as an MPQ,
`read_units.py` drives the retail client, `compare.py` reads the captures).

**Result: 173 of 190 types match; 17 could not be selected: 15 doodad
buildings the retail game does not let you select (Cave, Cantina, Markers,
Doors...), the Kakaru (a flying critter that leaves at once) and a Larva
placed without a Hatchery (it does not stay on the map).  None differs.**
A third pass caged the Rhynadon and the Ragnasaur in rings of Pylons
(`CAGE=1`) and read them.  A second pass
placed only the unread types and read the Broodling (before its timer ran
out), the Scantid and the Ursadon.

| id | unit | hp | shields | armor | damage | retail panel |
|---|---|---|---|---|---|---|
| 0 | Terran Marine | 40 | - | 0 | 6, 6 | match (OCR) |
| 1 | Terran Ghost | 45 | - | 0 | 10, 10 | match (OCR) |
| 2 | Terran Vulture | 80 | - | 0 | 20 | match (OCR) |
| 3 | Terran Goliath | 125 | - | 1 | 12, 10 | match (OCR) |
| 5 | Terran Siege Tank | 150 | - | 1 | 30 | match (OCR) |
| 7 | Terran SCV | 60 | - | 0 | 5 | match (OCR) |
| 8 | Terran Wraith | 120 | - | 0 | 8, 20 | match (OCR) |
| 9 | Terran Science Vessel | 200 | - | 1 | - | match (OCR) |
| 10 | Gui Montag | 160 | - | 3 | 16 | match (checked by eye) |
| 11 | Terran Dropship | 150 | - | 1 | - | match (OCR) |
| 12 | Terran Battlecruiser | 500 | - | 3 | 25, 25 | match (OCR) |
| 13 | Vulture Spider Mine | 20 | - | 0 | 125 | match (checked by eye) |
| 15 | Terran Civilian | 40 | - | 0 | - | match (OCR) |
| 16 | Sarah Kerrigan | 250 | - | 3 | 30, 30 | match (OCR) |
| 17 | Alan Schezar | 300 | - | 3 | 24, 20 | match (OCR) |
| 19 | Jim Raynor | 300 | - | 3 | 30 | match (OCR) |
| 20 | Jim Raynor | 200 | - | 3 | 18, 18 | match (OCR) |
| 21 | Tom Kazansky | 500 | - | 4 | 16, 40 | match (OCR) |
| 22 | Magellan | 800 | - | 4 | - | match (OCR) |
| 23 | Edmund Duke | 400 | - | 3 | 70 | match (OCR) |
| 25 | Edmund Duke | 400 | - | 3 | 150 | match (OCR) |
| 27 | Arcturus Mengsk | 1000 | - | 4 | 50, 50 | match (OCR) |
| 28 | Hyperion | 850 | - | 4 | 30, 30 | match (OCR) |
| 29 | Norad II | 700 | - | 4 | 50, 50 | match (OCR) |
| 30 | Terran Siege Tank | 150 | - | 1 | 70 | match (OCR) |
| 32 | Terran Firebat | 50 | - | 1 | 8 | match (checked by eye) |
| 34 | Terran Medic | 60 | - | 1 | - | match (OCR) |
| 35 | Zerg Larva | 25 | - | 10 | - | not readable: a Larva placed without a Hatchery does not stay |
| 36 | Zerg Egg | 200 | - | 10 | - | match (checked by eye) |
| 37 | Zerg Zergling | 35 | - | 0 | 5 | match (OCR) |
| 38 | Zerg Hydralisk | 80 | - | 0 | 10, 10 | match (OCR) |
| 39 | Zerg Ultralisk | 400 | - | 1 | 20 | match (checked by eye) |
| 40 | Zerg Broodling | 30 | - | 0 | 4 | match (second pass: 30/30, armor 0, read at once) |
| 41 | Zerg Drone | 40 | - | 0 | 5 | match (checked by eye) |
| 42 | Zerg Overlord | 200 | - | 0 | - | match (OCR) |
| 43 | Zerg Mutalisk | 120 | - | 0 | 9, 9 | match (OCR) |
| 44 | Zerg Guardian | 150 | - | 2 | 20 | match (OCR) |
| 45 | Zerg Queen | 120 | - | 0 | - | match (OCR) |
| 46 | Zerg Defiler | 80 | - | 1 | - | match (OCR) |
| 47 | Zerg Scourge | 25 | - | 0 | 110 | match (OCR) |
| 48 | Torrasque | 800 | - | 4 | 50 | match (OCR) |
| 49 | Matriarch | 300 | - | 3 | - | match (OCR) |
| 50 | Infested Terran | 60 | - | 0 | 500 | match (OCR) |
| 51 | Infested Kerrigan | 400 | - | 2 | 50 | match (OCR) |
| 52 | Unclean One | 250 | - | 3 | - | match (OCR) |
| 53 | Hunter Killer | 160 | - | 2 | 20, 20 | match (OCR) |
| 54 | Devouring One | 120 | - | 3 | 10 | match (OCR) |
| 55 | Kukulza | 300 | - | 3 | 18, 18 | match (OCR) |
| 56 | Kukulza | 400 | - | 4 | 40 | match (OCR) |
| 57 | Yggdrasill | 1000 | - | 4 | - | match (OCR) |
| 58 | Terran Valkyrie | 200 | - | 2 | 6 | match (OCR) |
| 59 | Cocoon | 200 | - | 0 | - | match (checked by eye) |
| 60 | Protoss Corsair | 100 | 80 | 1 | 5 | match (checked by eye) |
| 61 | Protoss Dark Templar | 80 | 40 | 1 | 40 | match (checked by eye) |
| 62 | Zerg Devourer | 250 | - | 2 | 25 | match (OCR) |
| 63 | Protoss Dark Archon | 25 | 200 | 1 | - | match (checked by eye) |
| 64 | Protoss Probe | 20 | 20 | 0 | 5 | match (checked by eye) |
| 65 | Protoss Zealot | 100 | 60 | 1 | 8 | match (checked by eye) |
| 66 | Protoss Dragoon | 100 | 80 | 1 | 20, 20 | match (checked by eye) |
| 67 | Protoss High Templar | 40 | 40 | 0 | - | match (checked by eye) |
| 68 | Protoss Archon | 10 | 350 | 0 | 30, 30 | match (checked by eye) |
| 69 | Protoss Shuttle | 80 | 60 | 1 | - | match (checked by eye) |
| 70 | Protoss Scout | 150 | 100 | 0 | 8, 14 | match (checked by eye) |
| 71 | Protoss Arbiter | 200 | 150 | 1 | 10, 10 | match (checked by eye) |
| 72 | Protoss Carrier | 300 | 150 | 4 | - | match (checked by eye) |
| 74 | Dark Templar | 40 | 80 | 0 | 45 | match (checked by eye) |
| 75 | Zeratul | 60 | 400 | 0 | 100 | match (checked by eye) |
| 76 | Tassadar/Zeratul | 100 | 800 | 3 | 60, 60 | match (checked by eye) |
| 77 | Fenix | 240 | 240 | 2 | 20 | match (checked by eye) |
| 78 | Fenix | 240 | 240 | 3 | 45, 45 | match (checked by eye) |
| 79 | Tassadar | 80 | 300 | 2 | 20 | match (checked by eye) |
| 80 | Mojo | 400 | 400 | 3 | 20, 28 | match (checked by eye) |
| 81 | Warbringer | 200 | 400 | 3 | - | match (checked by eye) |
| 82 | Gantrithor | 800 | 500 | 4 | - | match (checked by eye) |
| 83 | Protoss Reaver | 100 | 80 | 0 | - | match (checked by eye) |
| 84 | Protoss Observer | 40 | 20 | 0 | - | match (checked by eye) |
| 86 | Danimoth | 600 | 500 | 3 | 20, 20 | match (checked by eye) |
| 87 | Aldaris | 80 | 300 | 2 | 20 | match (checked by eye) |
| 88 | Artanis | 250 | 250 | 3 | 20, 28 | match (checked by eye) |
| 89 | Rhynadon | 60 | - | 0 | - | match (third pass, caged in Pylons: 60/60) |
| 90 | Bengalaas | 60 | - | 0 | - | match (OCR) |
| 93 | Scantid | 60 | - | 0 | - | match (second pass: 60/60) |
| 94 | Kakaru | 60 | - | 0 | - | not readable: flying critter, leaves at once |
| 95 | Ragnasaur | 60 | - | 0 | - | match (third pass, caged in Pylons: 60/60) |
| 96 | Ursadon | 60 | - | 0 | - | match (second pass: 60/60) |
| 97 | Zerg Lurker Egg | 200 | - | 10 | - | match (checked by eye) |
| 98 | Raszagal | 100 | 60 | 0 | 5 | match (checked by eye) |
| 99 | Samir Duran | 200 | - | 2 | 25, 25 | match (OCR) |
| 100 | Alexei Stukov | 250 | - | 3 | 30, 30 | match (OCR) |
| 102 | Gerard DuGalle | 700 | - | 4 | 50, 50 | match (OCR) |
| 103 | Zerg Lurker | 125 | - | 1 | 20 | match (OCR) |
| 104 | Infested Duran | 300 | - | 3 | 25, 25 | match (OCR) |
| 106 | Terran Command Center | 1500 | - | 1 | - | match (checked by eye) |
| 107 | Terran Comsat Station | 500 | - | 1 | - | match (OCR) |
| 108 | Terran Nuclear Silo | 600 | - | 1 | - | match (OCR) |
| 109 | Terran Supply Depot | 500 | - | 1 | - | match (OCR) |
| 110 | Terran Refinery | 750 | - | 1 | - | match (OCR) |
| 111 | Terran Barracks | 1000 | - | 1 | - | match (checked by eye) |
| 112 | Terran Academy | 600 | - | 1 | - | match (OCR) |
| 113 | Terran Factory | 1250 | - | 1 | - | match (checked by eye) |
| 114 | Terran Starport | 1300 | - | 1 | - | match (OCR) |
| 115 | Terran Control Tower | 500 | - | 1 | - | match (OCR) |
| 116 | Terran Science Facility | 850 | - | 1 | - | match (OCR) |
| 117 | Terran Covert Ops | 750 | - | 1 | - | match (OCR) |
| 118 | Terran Physics Lab | 600 | - | 1 | - | match (OCR) |
| 120 | Terran Machine Shop | 750 | - | 1 | - | match (OCR) |
| 122 | Terran Engineering Bay | 850 | - | 1 | - | match (OCR) |
| 123 | Terran Armory | 750 | - | 1 | - | match (OCR) |
| 124 | Terran Missile Turret | 200 | - | 0 | 20 | match (checked by eye) |
| 125 | Terran Bunker | 350 | - | 1 | - | match (OCR) |
| 126 | Norad II | 700 | - | 1 | - | match (checked by eye) |
| 127 | Ion Cannon | 2000 | - | 1 | - | match (checked by eye) |
| 130 | Infested Command Center | 1500 | - | 1 | - | match (checked by eye) |
| 131 | Zerg Hatchery | 1250 | - | 1 | - | match (checked by eye) |
| 132 | Zerg Lair | 1800 | - | 1 | - | match (checked by eye) |
| 133 | Zerg Hive | 2500 | - | 1 | - | match (OCR) |
| 134 | Zerg Nydus Canal | 250 | - | 1 | - | match (OCR) |
| 135 | Zerg Hydralisk Den | 850 | - | 1 | - | match (OCR) |
| 136 | Zerg Defiler Mound | 850 | - | 1 | - | match (OCR) |
| 137 | Zerg Greater Spire | 1000 | - | 1 | - | match (checked by eye) |
| 138 | Zerg Queen's Nest | 850 | - | 1 | - | match (OCR) |
| 139 | Zerg Evolution Chamber | 750 | - | 1 | - | match (OCR) |
| 140 | Zerg Ultralisk Cavern | 600 | - | 1 | - | match (OCR) |
| 141 | Zerg Spire | 600 | - | 1 | - | match (OCR) |
| 142 | Zerg Spawning Pool | 750 | - | 1 | - | match (OCR) |
| 143 | Zerg Creep Colony | 400 | - | 0 | - | match (OCR) |
| 144 | Zerg Spore Colony | 400 | - | 0 | 15 | match (OCR) |
| 146 | Zerg Sunken Colony | 300 | - | 2 | 40 | match (OCR) |
| 147 | Zerg Overmind | 5000 | - | 1 | - | match (OCR) |
| 148 | Zerg Overmind | 2500 | - | 1 | - | match (checked by eye) |
| 149 | Zerg Extractor | 750 | - | 1 | - | match (OCR) |
| 150 | Mature Crysalis | 250 | - | 1 | - | match (OCR) |
| 151 | Zerg Cerebrate | 1500 | - | 1 | - | match (checked by eye) |
| 152 | Zerg Cerebrate Daggoth | 1500 | - | 1 | - | match (checked by eye) |
| 154 | Protoss Nexus | 750 | 750 | 1 | - | match (checked by eye) |
| 155 | Protoss Robotics Facility | 500 | 500 | 1 | - | match (checked by eye) |
| 156 | Protoss Pylon | 300 | 300 | 0 | - | match (checked by eye) |
| 157 | Protoss Assimilator | 450 | 450 | 1 | - | match (checked by eye) |
| 159 | Protoss Observatory | 250 | 250 | 1 | - | match (checked by eye) |
| 160 | Protoss Gateway | 500 | 500 | 1 | - | match (checked by eye) |
| 162 | Protoss Photon Cannon | 100 | 100 | 0 | 20, 20 | match (checked by eye) |
| 163 | Protoss Citadel of Adun | 450 | 450 | 1 | - | match (checked by eye) |
| 164 | Protoss Cybernetics Core | 500 | 500 | 1 | - | match (checked by eye) |
| 165 | Protoss Templar Archives | 500 | 500 | 1 | - | match (checked by eye) |
| 166 | Protoss Forge | 550 | 550 | 1 | - | match (checked by eye) |
| 167 | Protoss Stargate | 600 | 600 | 1 | - | match (checked by eye) |
| 168 | Stasis Cell/Prison | 2000 | - | 1 | - | match (OCR) |
| 169 | Protoss Fleet Beacon | 500 | 500 | 1 | - | match (checked by eye) |
| 170 | Protoss Arbiter Tribunal | 500 | 500 | 1 | - | match (checked by eye) |
| 171 | Protoss Robotics Support Bay | 450 | 450 | 1 | - | match (checked by eye) |
| 172 | Protoss Shield Battery | 200 | 200 | 1 | - | match (checked by eye) |
| 173 | Khaydarin Crystal Formation | indestructible | - | 1 | - | match (checked by eye) |
| 174 | Protoss Temple | 1500 | - | 1 | - | match (checked by eye) |
| 175 | Xel'Naga Temple | 5000 | - | 1 | - | match (OCR) |
| 176 | Mineral Field | indestructible | - | 0 | - | match (checked by eye) |
| 177 | Mineral Field | indestructible | - | 0 | - | match (checked by eye) |
| 178 | Mineral Field | indestructible | - | 0 | - | match (checked by eye) |
| 179 | Cave | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 180 | Cave-in | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 181 | Cantina | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 182 | Mining Platform | 800 | - | 0 | 7 | not readable: doodad building: not selectable in retail |
| 183 | Independent Command Center | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 184 | Independent Starport | 800 | - | 0 | 7 | match (OCR) |
| 185 | Jump Gate | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 186 | Ruins | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 187 | Kyadarin Crystal Formation | 800 | - | 0 | - | not readable: doodad building: not selectable in retail |
| 188 | Vespene Geyser | indestructible | - | 0 | - | match (checked by eye) |
| 189 | Warp Gate | 700 | - | 1 | - | match (OCR) |
| 190 | Psi Disrupter | 2000 | - | 1 | - | match (checked by eye) |
| 191 | Zerg Marker | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 192 | Terran Marker | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 193 | Protoss Marker | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 194 | Zerg Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 195 | Terran Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 196 | Protoss Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 197 | Zerg Flag Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 198 | Terran Flag Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 199 | Protoss Flag Beacon | indestructible | - | 0 | - | match (checked by eye) |
| 200 | Power Generator | 800 | - | 1 | - | match (OCR) |
| 201 | Overmind Cocoon | 2500 | - | 1 | - | match (checked by eye) |
| 203 | Floor Missile Trap | 50 | - | 0 | 10 | match (OCR) |
| 205 | Left Upper Level Door | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 206 | Right Upper Level Door | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 207 | Left Pit Door | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 208 | Right Pit Door | indestructible | - | 0 | - | not readable: doodad building: not selectable in retail |
| 209 | Floor Gun Trap | 50 | - | 0 | 10 | match (OCR) |
| 210 | Left Wall Missile Trap | 50 | - | 0 | 10 | match (OCR) |
| 211 | Left Wall Flame Trap | 50 | - | 0 | 8 | match (OCR) |
| 212 | Right Wall Missile Trap | 50 | - | 0 | 10 | match (OCR) |
| 213 | Right Wall Flame Trap | 50 | - | 0 | 8 | match (OCR) |
