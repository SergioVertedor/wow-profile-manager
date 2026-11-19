# Addon configuration adapters

An adapter says which part of an addon's data travels in a `.wowprofile` and how it is applied on the target
character. It is a **JSON file**, not code: anyone can propose a new one without knowing how to program. It is
validated with [`adaptador.schema.json`](adaptador.schema.json). The pieces it produces are package entries
([`wowprofile-format.md`](wowprofile-format.md)) of type `adaptador` or `perfil-acedb`.

Marks: **[V]** verified, **[I]** inference, **[H]** hypothesis.

## What AceDB is

AceDB-3.0 is a library from the Ace3 collection that many addons embed to store their configuration. It organizes
the addon's `SavedVariables` into **named profiles**, which is why the configuration can be carried from one
character to another without knowing the addon:

```lua
DominosDB = {
  ["profileKeys"] = {                       -- which profile each character uses
    ["Nombre Apellido"] = "Default",
    ["Otro Personaje"] = "Tanque",
  },
  ["profiles"] = {                          -- the configuration of each profile
    ["Default"] = { --[[ bars, positions, options… ]] },
    ["Tanque"] = { --[[ … ]] },
  },
  ["char"] = {                              -- per-character state: does not travel
    ["Nombre Apellido"] = { --[[ … ]] },
  },
}
```

Export = copy the profile the character uses (`profiles["Default"]`) and, if the addon has modules (`namespaces`,
measured in Prat, Dominos and Bazooka), the profile with that same name in each module. Next to a module's
`profiles` there may be `char` or `realm` with state and names of other players: that does not travel. Import = add
it to the target's file and point **only** the target character's `profileKeys` to that profile. The other
characters on the target are not affected [V: tested in game].

## Why they are needed

Measured on a Forever installation, with 45 addons that store data [V]:

| pattern | addons | consequence |
|---|---|---|
| AceDB (`profileKeys` and `profiles`) | Dominos, Prat, Mapster, HandyNotes, RareScanner, Spy, APR, Bazooka, Broker_Everything… (10) | the generic `perfil-acedb` rule is enough |
| preferences and progress in the same table | ForeverDungeonJournal (`journalMode` next to `rpgKills`, `runLog`…) | keys must be chosen |
| **other players' data** | WIM (`WIM3_History`, `WIM3_Cache`), LFGForever (`recentPartyMembers`), Spy per character (`KOSData`, `PlayerData`) | cannot travel |
| configuration and state in separate variables | Auctionator (`AUCTIONATOR_CONFIG` vs. `AUCTIONATOR_PRICE_DATABASE`), WIM | choosing variables is enough |
| account file with per-character data, without AceDB | Pawn (`PerCharacterOptions`), EllesmereUI (`chars`: gold, class, race) | only the source character must be kept |
| `Nombre-Reino` key **without surname** | SkillUpForever, TalentsForeverBook | two characters with the same name share a key: there is no way to tell whose it is |
| state only | error log, session data, learned recipes | not exported |

## Decision order

The app works for **any addon**, including ones nobody has ever looked at: rules 2 to 5 are generic and need no
adapter. Written adapters (rule 1) are only the **exceptions**: addons where the generic rules would get it wrong,
for example because they hold progress or other players' data.

For each addon with data, the first rule that applies is used:

The decision is made **per piece**: each account variable and each per-character file. The same addon can have
pieces that fall under different rules. Only **installed** addons are considered; the variables come from the file,
not from the `.toc`.

1. **Written adapter** for that `addon`.
2. **AceDB detected**: the account variable has `profileKeys` and `profiles`. The profile the character uses
   travels (`perfil-acedb`).
3. **Generic account**: account file that does not mention any character on the account. It is offered whole,
   **unchecked**, with the base warning. On the target it affects every character: the app shows what it overwrites
   and only applies it if the user accepts.
4. **Generic per character**: per-character file with no adapter. It is offered **unchecked** and with the base
   warning.
5. **Generic account with characters**: account file that mentions characters and is not AceDB. It is offered
   whole, **unchecked**, with the base warning and "mentions characters on this account". Like rule 3, on the target
   it affects every character and asks for confirmation.

Base warning for rules 3, 4 and 5: "may contain progress, state or other players' data".

**Hard rule:** no written adapter includes other players' data (names, GUIDs, conversations, enemy lists or
group-member lists). Generic rules 3, 4 and 5 cannot guarantee this, which is why they are unchecked and come with
a warning: the user decides.

Why rule 3 cannot either: a lone name of another player is a string like any other, and no check on the text can
ensure it is not there. Measured on an installation [V]: of 31 account files in which no character on the account
appeared in its usual format, 3 did store characters (a player GUID and `Nombre-Reino` keys without surname).

**Hints.** On export, the app searches the files of the generic rules for what can be recognized: player GUIDs
(`Player-<server>-<8 hex digits>`) and keys shaped like `Nombre-Reino` or `Nombre - Reino` (a one-word name, capitalized,
2 to 12 letters). The `Nombre Apellido` form is not a hint: it matches many texts that are not characters. If it finds
any, the warning says so ("seems to contain other players").
Hints **never check an entry**: the absence of hints proves nothing.

To avoid going addon by addon, the selection has a "check all" action for each type of generic rule. It does not
check pieces with hints: those are checked one by one, after seeing their warning.

## How to write an adapter

```json
{
  "id": "nombre-del-adaptador",
  "version": 1,
  "addon": "CarpetaDelAddon",
  "versionesAddon": ["1.2.3"],
  "medido": "Qué claves se vieron y cuándo.",
  "reglas": [
    { "ambito": "personaje", "variable": "MiAddonCharDB", "incluir": ["opciones", "colores"] }
  ]
}
```

- `reglas` (rules): one per variable (those in `## SavedVariables` or `## SavedVariablesPerCharacter` of the `.toc`).
  **A variable that does not appear in the rules does not travel.** It is an allowlist: if the addon adds a state
  variable tomorrow, it does not leak by accident.
- Each rule chooses **one** way to select:

| field | what travels |
|---|---|
| `"entera": true` (whole) | the complete variable. **Requires `versionesAddon`** (addon versions; see below) |
| `"incluir": [paths]` (include) | only those keys |
| `"excluir": [paths]` (exclude) | everything except those keys. **Requires `versionesAddon`** (see below) |
| `"perfilAcedb": true` (AceDB profile) | the character's AceDB profile (account variables only) |

- **Rule for any adapter: what has not been named does not travel without a measured version.**
  - `incluir` and `perfilAcedb` only let through what they name (specific keys, or the profile). A key the addon
    adds in an update does not travel until someone adds it to the adapter.
  - `entera` and `excluir` let through keys nobody has listed. That is why the schema rejects them if the adapter
    does not declare `versionesAddon`, and the app, with a different version installed, offers the entry unchecked
    and with a warning.
  - `incluir` is preferred: it does not go stale with each addon update.
- `soloPersonaje` (character only; optional, alongside `entera`, `incluir` or `excluir`): paths of tables indexed
  by character. Only the source character's entry travels and, on import, it is written with the target's key.
- `clavePersonaje` (character key): how the addon writes that key. `nombre-apellido` (`"Nombre Apellido"`, like
  AceDB in Forever) is the default value.
- `noExportar: true` (do not export) instead of `reglas`: the addon only stores state or other people's data, and
  is never offered.
- **Paths**: keys separated by `/`. A segment is a literal name, `*` (any key at that level) or `$PJ` (the
  character's key). Example: `Scales/*/PerCharacterOptions/$PJ`.

### How it is applied on the target

- It is **merged by key**: in the target's file only the keys that travel are replaced. The rest of the target's
  file is not touched.
  - The rule in the app's catalog is what applies, not what the piece says: a key the rule does not name does not
    arrive even if it comes in the package.
  - What the rule lets through but the piece does not bring stays as it is on the target: nothing is deleted. `entera`
    replaces the whole variable.
  - In `soloPersonaje`, the marker's entry is written with the target character's key; those of the other
    characters on the target do not change.
- Rules with `ambito: "cuenta"` (scope: account) change a file shared by every character on the target. If they only carry
  `soloPersonaje`, they only change the target character's entry and no confirmation is needed. If they carry
  more, the app shows what it overwrites and asks for confirmation.
- If the installed addon version is not in `versionesAddon` and the rule uses `entera` or `excluir`, the preview
  warns about it.
- Files are read and written as data (Lua is not executed), with minimal changes.

## Written adapters (measured exceptions)

Each adapter is a JSON file in [`src-tauri/adaptadores/`](../src-tauri/adaptadores/), which is bundled into the
executable: that is the only copy. Each one is explained here.

Addons where the generic rules would get it wrong, measured on 2026-11-17 in Forever 1.60.1. The list grows with
each addon that is measured (the popular ones, in Phase 5) and with those anyone proposes via PR. The classification of each key as preference or state is **[I]**
unless stated otherwise: the name and the content are read, not the addon's code.

### ForeverDungeonJournal

File: [`forever-dungeon-journal.json`](../src-tauri/adaptadores/forever-dungeon-journal.json).

`journalMode` is the Gamer/RPG choice the addon asks for again if it is missing [V].

### LFGForever

File: [`lfg-forever.json`](../src-tauri/adaptadores/lfg-forever.json).

The user's two messages travel (`inviteMessage`, `askToJoinMessage`); `recentPartyMembers` stores other players and
does not travel.

### WIM

File: [`wim.json`](../src-tauri/adaptadores/wim.json).

Options and filters travel (`WIM3_Data`, `WIM3_Filters`, `WIM3_ChatFilters`), whole, only with the measured
version. Conversations and the player cache do not travel.

### Auctionator

File: [`auctionator.json`](../src-tauri/adaptadores/auctionator.json).

Configuration, shopping lists and selling groups travel, whole, only with the measured version. Prices, history
and recent searches do not travel. Measured on 2026-11-18: a list's name is set by the user (or by an addon, such as
`SkillUp: <personaje>: Cocina`) and travels as is, like the text of a macro.

### Pawn

File: [`pawn.json`](../src-tauri/adaptadores/pawn.json).

In `PawnCommon`, `soloPersonaje` makes only the source character's entry of each `PerCharacterOptions` travel; those
of the other characters on the account are left out.

### EllesmereUI

File: [`ellesmereui.json`](../src-tauri/adaptadores/ellesmereui.json).

Affects the whole account: asks for confirmation. Version 3 (2026-11-18): `restoreOnUninstall` is removed from the
list; it is a copy of the addon's whole database, with `chars` and character GUIDs, to restore it on uninstall.

### TalentsForeverBook

File: [`talents-forever-book.json`](../src-tauri/adaptadores/talents-forever-book.json).

`settings` and `builds` travel. Per-character plans do not, because their key does not distinguish characters with
the same name.

### Spy

File: [`spy.json`](../src-tauri/adaptadores/spy.json).

Without this adapter, the AceDB rule would cover `SpyDB`, but the generic per-character rule would offer
`SpyPerCharDB`. The adapter closes that gap: since `SpyPerCharDB` is not in the rules, it does not travel.

### Addons that are not exported

SkillUpForever (per-character recipes), LayerSwap (session data) and !BugGrabber (captured errors) only store
state: they are never offered.

File: [`skillup-forever.json`](../src-tauri/adaptadores/skillup-forever.json).

File: [`layerswap.json`](../src-tauri/adaptadores/layerswap.json).

File: [`buggrabber.json`](../src-tauri/adaptadores/buggrabber.json).

## Internal adapters

Blizzard's **Edit Mode** is not a `SavedVariables`: it is `edit-mode-cache-account.txt` (the layouts) and
`edit-mode-cache-character.txt` (which layout each specialization uses). It is handled by an internal adapter in
the app, `modo-edicion` (version 1), because it cannot be described with table paths. It exports only the layout the
character uses and, on import, adds it to the account's layouts and points the character to it. Format measured in
[`wow-forever-compatibility.md`](wow-forever-compatibility.md#edit-mode-v).

## Pending

- Adapters for popular addons not measured here (WeakAuras, Plater, Details!, Baganator, Questie, Bartender4,
  TradeSkillMaster…).
- Verify each written adapter in game, as was done with AceDB.
