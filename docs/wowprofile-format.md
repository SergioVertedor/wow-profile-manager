# The `.wowprofile` format 1.1

A `.wowprofile` carries the configuration of one WoW character to another, usually another player's on another
computer, and optionally the addons it uses. This document is the contract between export and import. Version
`1.1` adds [addon pieces](#addon-pieces-11) to `1.0`; a package without addons is still written as `1.0`. The manifest
is validated with [`wowprofile.schema.json`](wowprofile.schema.json) (JSON Schema 2020-12). The measurements behind it
are in [`wow-forever-compatibility.md`](wow-forever-compatibility.md).

## Principles

1. **Pieces, not whole files.** Account `SavedVariables` hold data for every character on the account. Only what
   belongs to the character is exported: the profile it uses, its macros, its key bindings.
2. **Configuration, not state.** Progress, tracked quests and AceDB `char` tables stay out.
3. **Nothing private.** No account number, no realm, no source character name and no disk paths. The only name is the
   one the user gives the profile.

The first three are guaranteed by AceDB and adapter pieces. A piece from a generic rule in
[`adapters.md`](adapters.md#decision-order) travels as a whole file and can break them (state, names of the source
character or of other players). That is why it starts unchecked, with a warning, and only travels if the user checks it.
4. **The package does not decide where things are written.** It carries logical paths. The importing app maps them to
   the character the user picks.
5. **Lua is data.** No piece is executed: it is read and rewritten as text.

## Container

- A ZIP (`deflate` or `stored` method) with the `.wowprofile` extension. The file name is free.
- Contents:

```
manifest.json        ← UTF-8, no BOM
datos/<id>           ← one piece per manifest entry, bytes as is
```

- Nothing else. Any other ZIP entry, folders included, makes the package be rejected.
- The order of ZIP entries does not matter. The app writes `manifest.json` last: the hash of an addon piece is only
  known once it is packed.

## Manifest

| field | required | what it is |
|---|---|---|
| `formato.nombre` | yes | always `"wowprofile"` |
| `formato.version` | yes | `"MAYOR.MENOR"` (major.minor); this document is `1.1` (`1.0` if there are no addon pieces) |
| `perfil.nombre` | yes | descriptive name set by the user (1–80 characters) |
| `perfil.descripcion` | no | free text (up to 500) |
| `perfil.creado` | yes | export date, RFC 3339 in UTC (`…Z`) |
| `origen.producto` | yes | `wowf` (Forever) or `wow` (Retail), from `.build.info` |
| `origen.build` | yes | client version, e.g. `1.60.1.70938` |
| `origen.interfaz` | yes | interface number, e.g. `16001` |
| `origen.clase` | no | character class (`DRUID`…), only if it could be determined; used to warn, never to block. The app does not write it yet: it cannot be read reliably from the files |
| `addons[]` | yes | addons the package mentions: those of its `config-addon` pieces and the installed ones its `AddOns.txt` leaves enabled (may be empty) |
| `entradas[]` | yes | one per piece in `datos/` |

### `addons[]`

The addon description: the importer sees what is missing and where to get it. The addon code only travels if there is
also an [addon piece](#addon-pieces-11) for it.

| field | required | where it comes from |
|---|---|---|
| `id` | yes | folder name in `Interface/AddOns` |
| `titulo` | yes | `## Title` from the `.toc` |
| `version` | no | `## Version` (not all addons declare it) |
| `enlace` | no | `## X-Website` or similar, `https://` only |
| `dependencias` | no | `## Dependencies` / `## RequiredDeps`, as `id` |

### `entradas[]`

| field | required | what it is |
|---|---|---|
| `id` | yes | `^[a-z0-9][a-z0-9-]{0,63}$`; the piece is at `datos/<id>` |
| `categoria` | yes | what the user sees when choosing what to import (table below) |
| `tipo` | yes | how the piece was extracted and how it is applied (table below) |
| `ambito` | yes | where it was in the source: `personaje` (character) or `cuenta` (account); `instalacion` (installation) for an addon piece |
| `destino` | yes | logical path where it is applied by default: `personaje/<fichero>` or `cuenta/[SavedVariables/]<fichero>`; `addons/<carpeta>` for an addon piece |
| `addon` | depends on type | `id` of the addon it belongs to |
| `sha256` | yes | SHA-256 of the piece, in lowercase hexadecimal |
| `bytes` | yes | exact size of the piece |
| `marcadores` | no | `PERSONAJE` (character) and/or `REINO` (realm) if the piece contains them |

`ambito` and `destino` may differ. Example: a character without its own key bindings uses the account ones
(`ambito: cuenta`), and they are imported as key bindings of the target character
(`destino: personaje/bindings-cache.wtf`) so the other characters on that account are not touched.

### Categories

A minor version may add more. The contents of action bar buttons will not be one: they are not in any file, the server
stores them.

| category | type | `destino` |
|---|---|---|
| `macros` | `fichero` (file) | `personaje/macros-cache.txt` or `cuenta/macros-cache.txt` (general macros) |
| `teclas` (key bindings) | `fichero` | `personaje/bindings-cache.wtf` (also if they were account-wide in the source) |
| `teclas-clic` (click bindings) | `fichero` | `personaje/click-bindings-cache.txt` |
| `opciones` (options) | `fichero-filtrado` (filtered file) | `personaje/config-cache.wtf` or `cuenta/config-cache.wtf` |
| `chat` | `fichero` | `personaje/chat-cache.txt` |
| `ventanas` (windows) | `fichero` | `personaje/layout-local.txt` |
| `modo-edicion` (Edit Mode) | `adaptador` (adapter) `modo-edicion` (version 1): an `edit-mode-cache-account.txt` with a single layout, the one the character uses; `perfil` holds its name | `cuenta/edit-mode-cache-account.txt` |
| `addons-activos` (enabled addons) | `fichero-filtrado` | `personaje/AddOns.txt` |
| `config-addon` (addon configuration) | `perfil-acedb` (AceDB profile), `fichero` or `adaptador` | `cuenta/SavedVariables/<addon>.lua` or `personaje/SavedVariables/<addon>.lua`, with `<addon>` = the addon folder (this is how the game stores its variables) |
| `marcos-banda` (raid frames) | `fichero` | `personaje/CUFProfiles.txt` |
| `addon` (1.1) | `carpeta` (folder) | `addons/<carpeta>`, with `<carpeta>` = `addon` |

Rules for file categories:

- **Key bindings.** If the character has `bindings-cache.wtf`, that one travels (`ambito: personaje`). If not, it uses
  the account ones [H] and the account file travels (`ambito: cuenta`), always to `personaje/bindings-cache.wtf`.
- **Options** (`config-cache.wtf`, character and account). The state `SET <clave>` lines and the security one are
  removed; the rest travel, because they are client options. Case-insensitive, like CVars:
  - exact: `AllowDangerousScripts` (it would remove the importer's warning when running scripts), `lastSelectedClubId`,
    `EJLootClass`, `flaggedTutorials`, `closedInfoFrames`, `closedInfoFramesAccountWide`, `numReputationHeaders`,
    `numCurrencyCategories`;
  - starting with `tracked`, `hardTracked`, `cameraSaved`, `perksActivities` or `petJournal`;
  - containing `collapsed`.
- **Enabled addons** (`AddOns.txt`). Lines for addons not installed in the source are removed. `Blizzard_*` lines always
  travel: they are client modules, they are not in `Interface/AddOns` and they can be disabled.
- **Chat** travels whole: it only holds windows, colors, message types and channels [V].
- **On import**, options and enabled addons are merged line by line with what the target already has, and the list of
  option keys that do not travel is applied again: a handcrafted package could bring them. The other file categories
  replace the whole file ([`flows.md`](flows.md#how-each-category-is-written)).

`config-addon` rules (the decision order is in [`adapters.md`](adapters.md#decision-order)):

- Only from addons **installed** in the source. `.lua` files of uninstalled addons and the client's `Blizzard_*.lua`
  do not travel.
- Variables come from what is in the file (`Nombre = <valor>`), not from the `.toc`.
- Account file of an addon without an adapter: each AceDB variable is a `perfil-acedb` entry. What remains is **one**
  generic entry: `fichero` if nothing was removed, or `fichero-filtrado` with the AceDB variables in `claves_quitadas`
  (they travel separately, as their profile). If nothing remains, there is no generic entry.
- Character file of an addon without an adapter: `fichero`, whole.

These do not travel: `flagged-cache-*` (tutorials seen), `character-list-order.txt` (order of the character selection
screen), `tts-cache-*` and `cooldownmanager.txt` (not measured), `cache.md5` and the game's `.old` and `.bak` copies.

### Entry types

| type | what the piece holds | extra fields | example |
|---|---|---|---|
| `fichero` | the whole file | — | the character's `macros-cache.txt` |
| `fichero-filtrado` | the file without the state keys or the lines that must not travel | `claves_quitadas` | `config-cache.wtf` without `hardTrackedQuests`; `AddOns.txt` without the addons that were not installed |
| `perfil-acedb` | the character's profile and that of each module (namespace), as Lua text (see below) | `addon`, `variable`, `perfil` | the `Default` profile of `DominosDB` |
| `adaptador` | `<variable> = <tabla>` as Lua text, only with what the adapter's rule lets through | `adaptador.id`, `adaptador.version`, `addon` if it belongs to an addon, `variable` | preferences of an addon that mixes in progress |
| `carpeta` (1.1) | a ZIP with the files of an addon's folder | `addon` | the `Interface/AddOns/Dominos` folder |

`perfil-acedb` piece. Many AceDB addons store part of their configuration in modules (`namespaces`), each with its own
profiles; that is why the piece holds the profile and that of each module with the same name:

```lua
{
["profile"] = { --[[ <variable>.profiles["<perfil>"] ]] },
["namespaces"] = {
["<módulo>"] = { --[[ <variable>.namespaces["<módulo>"].profiles["<perfil>"] ]] },
},
}
```

`["namespaces"]` is only present if some module has that profile. It never holds `profileKeys`, `char`, `realm`,
`global` or other profiles: that is where state and other characters' names live. Each table is the source bytes as is.
If the profile is named after the source character, `perfil` is `@@WOWPROFILE:PERSONAJE@@` and the entry has
`marcadores: ["PERSONAJE"]`.

`adaptador` piece. What travels is copied byte for byte from the source; only the braces of trimmed tables are written
anew. A character key from `soloPersonaje` is written as `["@@WOWPROFILE:PERSONAJE@@"]`.

`adaptador` `modo-edicion` piece: the header of the source account file and the layout the character uses, byte for
byte, ending in the null byte like the game file. It only travels if the character uses its own layout (not a Blizzard
one). The character file does not travel: the index is recomputed on the target.

How each type is applied on import:
- `fichero` and `fichero-filtrado`: written to `destino`, with WoW closed, without touching the target's `cache.md5`.
  In `config-addon`, the character one replaces the file; the account one only changes its variables and keeps the
  rest of the file (those that travel as a profile, for example). An account generic entry neither holds nor touches an
  AceDB base (`profileKeys` and `profiles`): that only comes in as `perfil-acedb`.
- `adaptador` `modo-edicion`: the layout is **appended** to the end of the target's `edit-mode-cache-account.txt`
  without changing anything that was there (as `<nombre> 2`, `3`… if the name already exists), and the character's
  `edit-mode-cache-character.txt` is pointed at it. If the account already has an identical layout, that one is used and
  nothing is added. It does not affect the other characters on the account, so it needs no account decision. If a
  target file cannot be understood, it is not touched.
- `perfil-acedb`: added to `<variable>.profiles["<perfil>"]` in the target account file (and each module to
  `<variable>.namespaces["<módulo>"].profiles["<perfil>"]`), and **only** the target character's `profileKeys` is
  pointed at that profile. If a profile with that name already exists and is different, the app
  asks (rename or overwrite). With the marker, the profile is named after the target character. If the variable or the
  file does not exist, it is created.
- Everything else in the file stays byte for byte: only the values that change are changed, and new ones are appended to
  the end of their table. The exception is entries without a key (lists, which go by position): if they change, that
  table is written anew, with its keyed entries as is. If the piece brings none, the target's ones stay.
- Before writing, the resulting file is read back: if it cannot be understood as data, that entry is not applied to that
  character and the preview says so.
- `adaptador`: applied by the adapter with that `id`, using the rule from the app's catalog: the piece is trimmed again
  with it, so a key the rule does not name does not get through even if it is in the piece. If the app does not know it,
  it belongs to another addon, or the package has a newer version than the app's, the entry is not imported.

`cuenta` entries that cannot be applied to the target character alone (general macros, account options) are shown with
what they will overwrite and are only applied if the user accepts.

## Addon pieces (1.1)

An `addon` piece holds an addon's folder, with its code. **It is code the game runs**: the importer has to trust the
sharer, and many addons do not allow redistribution (that is the sharer's responsibility). The app warns about it on
export and on import.

- **What travels.** On export, the app offers the folders of the installed addons the character uses: those its
  `AddOns.txt` enables and those with configuration in the package. They start checked, with the warning.
- **The piece.** A ZIP (`deflate`) with the folder's files and paths relative to it, separated by `/`. It goes inside
  the `.wowprofile` without recompressing (`stored`). No empty folders or links: when packing, a link or a Windows
  junction keeps that folder from travelling.
- **Each path** is relative, without `.` or `..`, without a root or drive, without `\`, with segments Windows writes
  as is (no `CON`, `AUX`…, `:`, nor ending in a dot or space), at most 32 nested folders. Two paths that only
  differ in case are the same for Windows: the ZIP is rejected.
- **Limits.** 64 MiB per addon, uncompressed and also as a ZIP; 4,096 files per addon; 512 MiB of addon pieces per
  package, on top of the 128 MiB for the rest. An addon that does not fit does not travel: it stays in `addons[]`, with
  its link if it has one.
- **On import**, the addon is installed in `Interface/AddOns/<addon>` **only if that folder does not exist**
  (case-insensitive). One already installed is not touched, even if it is another version: the app warns. Undoing the
  import deletes the folders it created.

## Markers

Where a piece holds the source character's name, it is replaced on export by `@@WOWPROFILE:PERSONAJE@@`.
On import it is changed to the target's name, in the format that data uses (`Nombre Apellido`, first name and surname,
in Forever). For addons that store the realm, `@@WOWPROFILE:REINO@@` is used.

- The replacement is done by the AceDB rule or the adapter, which know which text is the name. It is not a find and
  replace over the file.
- If the source already contains `@@WOWPROFILE:`, that piece is not exported.
- Macros are not rewritten: if a macro names a character, that is up to the user.

## Reading a package safely

Implementation requirements, in this order, **before** writing anything:

1. Reject the file if it exceeds 644 MiB (128 for pieces, 512 for addons, plus margin) and the ZIP if it has more than
   257 entries (`manifest.json` + 256 pieces).
2. Read `manifest.json` (at most 1 MiB) and validate it against the schema for its major version.
   - Unknown major version: reject and ask the user to update the app.
   - Same major and newer minor: ignore unknown fields and values; entries with an unknown category or type are offered
     unchecked.
3. Entry `id`s are unique. The set of ZIP entries must be exactly `manifest.json` plus `datos/<id>` for each entry. A
   ZIP entry name is never used as a path.
4. Decompress each piece to memory, stopping at `bytes` + 1. If it exceeds `bytes`, or the total exceeds 128 MiB (512 MiB
   for addon pieces), reject (ZIP bomb). Addon pieces are checked one at a time and are not kept in memory:
   they are read again from the file when installing them.
5. Check `bytes` and `sha256` of each piece. If any fails, the whole package is rejected.
6. Map each `destino` to a real path **inside** the folder of the chosen character or account, and check that the
   resulting path is still inside that folder.
   - A file-category entry can only go to its category's `destino` in the [Categories](#categories) table (for
     example, `macros` only to `personaje/macros-cache.txt` or `cuenta/macros-cache.txt`). Otherwise it is not imported,
     even if it matches the schema pattern: a package cannot send a file to a place its category does not account for.
   - The real path comes from that table, not from the `destino` text.

7. Each addon piece must be a ZIP that meets [its rules](#addon-pieces-11) when decompressed, stopping at its limits.
   If one does not, the whole package is rejected: someone has crafted it to write outside its place.
8. Each `config-addon` piece must be **data only**: `Nombre = <valor>` statements (or a bare table in `perfil-acedb`)
   with tables, strings, numbers, `true`, `false` and `nil`. A function call, a variable, a function or an operator
   means the entry is not imported: the game runs `SavedVariables` when loading them.

Then come the preview, the backup and the write, with WoW closed.

**Integrity, not authorship.** The hashes detect a damaged or partially modified package, but they do not say who made
it. The format has no signature: packages are passed between players and there are no keys to verify. Protection
comes from treating everything as data, from logical paths, from the preview and from the backup.

## Versions

- `MAYOR.MENOR` (major.minor). This document defines `1.1`; `1.0` is the same without addon pieces.
- **Minor**: adding categories, types or optional fields. An app that reads `1.0` can open a `1.3`.
- **Major**: changing the meaning of a field, removing a required one or changing the container.
- The limits (256 entries, 32 MiB per piece, 128 MiB in total) may be raised in a minor version.
- An app that only reads `1.0` does not open a `1.1` with addons: their `destino` and size do not fit it. None has been
  published: `1.1` arrives before the app's first release.

## Example

A character exports macros, key bindings (its account ones), options, enabled addons and the Dominos profile:

```
mi-druida.wowprofile
├── manifest.json
└── datos/
    ├── macros
    ├── teclas
    ├── opciones
    ├── addons-activos
    └── dominos-perfil
```

```json
{
  "formato": { "nombre": "wowprofile", "version": "1.0" },
  "perfil": {
    "nombre": "Druida feral para la hermandad",
    "descripcion": "Barras de Dominos y macros de forma.",
    "creado": "2026-11-17T18:30:00Z"
  },
  "origen": { "producto": "wowf", "build": "1.60.1.70938", "interfaz": 16001 },
  "addons": [
    {
      "id": "Dominos",
      "titulo": "Dominos",
      "version": "12.0.3",
      "enlace": "https://github.com/tullamods/Dominos"
    }
  ],
  "entradas": [
    {
      "id": "macros",
      "categoria": "macros",
      "tipo": "fichero",
      "ambito": "personaje",
      "destino": "personaje/macros-cache.txt",
      "sha256": "3b1f0c9d6e2a4f8b7c5d1e0a9f8e7d6c5b4a39281706f5e4d3c2b1a09f8e7d6c",
      "bytes": 1995
    },
    {
      "id": "teclas",
      "categoria": "teclas",
      "tipo": "fichero",
      "ambito": "cuenta",
      "destino": "personaje/bindings-cache.wtf",
      "sha256": "a7c4e2f19b3d5c8e0f6a2b4d6e8f0a1c3e5b7d9f1a3c5e7b9d1f3a5c7e9b1d3f",
      "bytes": 1612
    },
    {
      "id": "opciones",
      "categoria": "opciones",
      "tipo": "fichero-filtrado",
      "ambito": "personaje",
      "destino": "personaje/config-cache.wtf",
      "sha256": "0d9e8f7a6b5c4d3e2f1a0b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e",
      "bytes": 1065,
      "claves_quitadas": ["hardTrackedQuests"]
    },
    {
      "id": "addons-activos",
      "categoria": "addons-activos",
      "tipo": "fichero-filtrado",
      "ambito": "personaje",
      "destino": "personaje/AddOns.txt",
      "sha256": "5e4d3c2b1a0f9e8d7c6b5a49382716f5e4d3c2b1a0f9e8d7c6b5a4938271605f",
      "bytes": 412,
      "claves_quitadas": ["Carbonite", "AtlasLoot"]
    },
    {
      "id": "dominos-perfil",
      "categoria": "config-addon",
      "tipo": "perfil-acedb",
      "ambito": "cuenta",
      "destino": "cuenta/SavedVariables/Dominos.lua",
      "addon": "Dominos",
      "variable": "DominosDB",
      "perfil": "Default - Druid",
      "sha256": "c2b1a0f9e8d7c6b5a4938271605f4e3d2c1b0a9f8e7d6c5b4a3928170f6e5d4c",
      "bytes": 18734
    }
  ]
}
```

The hashes and sizes in the example are illustrative.
