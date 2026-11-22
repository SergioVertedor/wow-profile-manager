# WoW Forever compatibility: what can be exported and imported

Result of Phase 1 (research). Measured in November 2026 on a real WoW Forever installation, client `wowf`
**1.60.1.70938** (Interface 16001), copying the configuration of a "source" character to a "target" character on the
same account and checking it in game.

Marks:
- **[V]** verified: measured in the files or tested in game.
- **[I]** inference from what was measured.
- **[H]** hypothesis, not tested yet.

No official Blizzard documentation describes these files: everything here comes from measuring them.

## Where Forever stores configuration [V]

- Installation: `_forever_` folder, product `wowf` in `.build.info`. Retail (`_retail_`, product `wow`) can live in the
  same installation, with its own `WTF/` and `Interface/`.
- Character: `WTF/Account/<cuenta>/<n>/Nombre-Apellido/`. Forever characters have a first name and a surname; in the
  folder they are joined by a hyphen.
- Account: `WTF/Account/<cuenta>/` and `WTF/Account/<cuenta>/SavedVariables/`.
- Global: `WTF/Config.wtf`, with graphics and hardware settings.
- **What is on each action bar button is not in any `WTF` file**: the server stores it.

## `cache.md5`: server synchronization [V]

- 240 bytes: **10 slots of 24 bytes**. Each slot is the **MD5 of the exact bytes** of a file (16 bytes) plus a Unix
  `uint64` little-endian timestamp (8 bytes). Empty slot = all-zero MD5.
- The slot → file mapping is fixed:

| slot | account | character |
|---|---|---|
| 0 | `config-cache.wtf` | `config-cache.wtf` |
| 1 | `bindings-cache.wtf` | `bindings-cache.wtf` |
| 2 | `macros-cache.txt` | `macros-cache.txt` |
| 3 | `tts-cache-account.txt` | `layout-local.txt` |
| 4 | `flagged-cache-account.txt` | `chat-cache.txt` |
| 5 | `edit-mode-cache-account.txt` | `tts-cache-character.txt` |
| 6 | empty | `flagged-cache-character.txt` |
| 7 | `character-list-order.txt` | `click-bindings-cache.txt` |
| 8 | — | `edit-mode-cache-character.txt` |
| 9 | — | `cooldownmanager.txt` |

- `AddOns.txt`, `CUFProfiles.txt` and `SavedVariables/` have no slot: they are not synchronized.
- **Measured import rule**: with WoW closed, the file is copied to the target character **without touching its
  `cache.md5`**. On login, the client adopts the local file and updates the slot. Tested with the slot empty, with the
  slot holding other content, in slots 0, 1, 2, 4 and 8, and in account slot 5 (an Edit Mode layout added by hand
  stays active after logging out and back in, 2026-11-19).
- In 1.60.1 **the local file wins** even if the slot has a different MD5. Other tools, measured on the beta, describe
  the opposite.

## Edit Mode [V]

Measured on 1.60.1 on 2026-11-19 (#71), by creating, activating, renaming and deleting layouts in game.

- Both files are **one line** of space-separated values ending in a **null byte**.
- `edit-mode-cache-account.txt`: header `5 <n>` and `n` numbers (it does not change with the layouts); then the custom
  layouts one after another, each one complete: `<largo> <nombre> <tipo> <nº de marcos>` (length, name, type, number of
  frames) and 10 values per frame. The length is **in UTF-8 bytes** (`Diseño ñ` → `10`) and the game cuts the name by
  it.
- `edit-mode-cache-character.txt`: `5` and ten values; the first is the active layout. 0–2 are the Blizzard ones
  (Modern and Classic among them) and custom ones start at 3, in the order of the account file. Renaming does not change
  the index; deleting the active layout sets it to `0`.
- Without dual specialization it has not been possible to measure which slot each specialization uses.

## Addon configuration [V]

- Addons store their configuration in `SavedVariables/*.lua`, account-wide or per character. They are Lua tables: they
  are read and rewritten without running them.
- **AceDB in Forever** uses `"Nombre Apellido"` as the character key, with no realm. Moving a profile to another
  character means adding the profile to `profiles` and pointing `profileKeys["<destino>"]` at it.
- Most measured addons also use `"Nombre Apellido"`. Some have their own format (`"Nombre Apellido-Reino"`,
  `"Nombre Apellido - Reino"`, or the name as a value) and need an adapter.
- Some addons mix preferences and progress in the same table: copying the whole file drags the progress along.
- An addon's `.toc` is a hint, not the truth: there are `SavedVariables` from addons already uninstalled and `.toc`
  files that declare files that do not exist.
- Some `SavedVariables` are not valid UTF-8 (binary data inside Lua strings). They are handled as bytes.

## Compatibility matrix

Scope: P = character, C = account, G = global.

| category | file(s) | scope | slot | how it is imported | status |
|---|---|---|---|---|---|
| Character macros | `macros-cache.txt` | P | 2 | copy, `cache.md5` untouched | **[V]** |
| General macros | `macros-cache.txt` | C | 2 | affects the whole account: warning and confirmation | [H] |
| Key bindings | `bindings-cache.wtf` | P or C | 1 | copy to the target character | **[V]** |
| Click bindings | `click-bindings-cache.txt` | P | 7 | copy | [H] |
| Character options | `config-cache.wtf` | P | 0 | copy, removing state keys | **[V]** |
| Account options | `config-cache.wtf` | C | 0 | affects the whole account: warning and confirmation | [H] |
| Chat | `chat-cache.txt` | P | 4 | copy | **[V]** |
| Window layout | `layout-local.txt` | P | 3 | copy | [H] |
| Edit Mode: layout in use | `edit-mode-cache-character.txt` | P | 8 | point to the added layout | **[V]** |
| Edit Mode: layouts | `edit-mode-cache-account.txt` | C | 5 | append the layout at the end | **[V]** same account |
| Enabled addons | `AddOns.txt` | P | — | copy only installed addons | **[V]** |
| AceDB configuration | `SavedVariables/*.lua` | C | — | profiles only (`profiles` and `profileKeys`) | **[V]** |
| Per-character configuration | `SavedVariables/*.lua` | P | — | copy with the target's key | **[V]** |
| Non-AceDB addons | specific to each addon | C and P | — | adapter per addon | [H] |
| Raid frames | `CUFProfiles.txt` | P | — | copy | [H] |
| Graphics | `Config.wtf` | G | — | not exported: depends on the hardware | [I] |
| Action bars | none | server | — | not possible through files | [V] |

Macros from a Retail character copied to a Forever one worked [V]. Addon configuration across versions would need key
translation (`"Nombre - Reino"` in Retail, `"Nombre Apellido"` in Forever). Even so, the app does not mix versions: a
profile is only imported into the version it came from (see
[`flows.md`](flows.md#2-choose-the-destination)).

## Import criteria

- Everything is written **with WoW closed**, with a backup first and the option to undo.
- From AceDB addons **only profiles** are imported, never the `char` section, which holds character state.
- **State keys** inside option files (for example `hardTrackedQuests`, the tracked quests) are removed on export.
- What the character uses at account level is applied **only to the target character** when possible (key bindings,
  AceDB profiles, Edit Mode layout). When it is not (general macros, account options), the app shows what will be
  overwritten and only applies it if the user accepts.
- Copying between different classes is the user's decision: at most, the app warns. The class cannot be read reliably
  from the client files.

## Pending tests

- Importing on **another machine** and into **another account**: all tests were done between two characters on the
  same account.
- Edit Mode layouts across accounts and with dual specialization.
- `layout-local.txt`, `click-bindings-cache.txt`, `CUFProfiles.txt` and `cooldownmanager.txt` separately.
- Adapters for non-AceDB addons and for popular addons not measured (WeakAuras, Plater, Details!, Baganator, Questie,
  Bartender4, TradeSkillMaster…).
- Whether a first name or surname with a hyphen makes the `Nombre-Apellido` folder ambiguous.
