# Adaptadores de configuración de addons

Un adaptador dice qué parte de los datos de un addon viaja en un `.wowprofile` y cómo se aplica en el personaje
destino. Es un **fichero JSON**, no código: cualquiera puede proponer uno nuevo sin saber programar. Se valida
con [`adaptador.schema.json`](adaptador.schema.json). Las piezas que produce son entradas del paquete
([`formato-wowprofile.md`](formato-wowprofile.md)) de tipo `adaptador` o `perfil-acedb`.

Marcas: **[V]** verificado, **[I]** inferencia, **[H]** hipótesis.

## Qué es AceDB

AceDB-3.0 es una librería de la colección Ace3 que muchos addons llevan dentro para guardar su configuración.
Organiza los `SavedVariables` del addon en **perfiles con nombre**, y por eso se puede llevar la configuración de
un personaje a otro sin conocer el addon:

```lua
DominosDB = {
  ["profileKeys"] = {                       -- qué perfil usa cada personaje
    ["Nombre Apellido"] = "Default",
    ["Otro Personaje"] = "Tanque",
  },
  ["profiles"] = {                          -- la configuración de cada perfil
    ["Default"] = { --[[ barras, posiciones, opciones… ]] },
    ["Tanque"] = { --[[ … ]] },
  },
  ["char"] = {                              -- estado de cada personaje: no viaja
    ["Nombre Apellido"] = { --[[ … ]] },
  },
}
```

Exportar = copiar el perfil que usa el personaje (`profiles["Default"]`). Importar = añadirlo al fichero del
destino y apuntar **solo** el `profileKeys` del personaje destino a ese perfil. Los demás personajes del destino
no se enteran [V: probado en el juego].

## Por qué hacen falta

Medido en una instalación de Forever, con 45 addons que guardan datos [V]:

| patrón | addons | consecuencia |
|---|---|---|
| AceDB (`profileKeys` y `profiles`) | Dominos, Prat, Mapster, HandyNotes, RareScanner, Spy, APR, Bazooka, Broker_Everything… (10) | basta la regla genérica `perfil-acedb` |
| preferencias y progreso en la misma tabla | ForeverDungeonJournal (`journalMode` junto a `rpgKills`, `runLog`…) | hay que elegir claves |
| **datos de otros jugadores** | WIM (`WIM3_History`, `WIM3_Cache`), LFGForever (`recentPartyMembers`), Spy por personaje (`KOSData`, `PlayerData`) | no pueden viajar |
| configuración y estado en variables distintas | Auctionator (`AUCTIONATOR_CONFIG` frente a `AUCTIONATOR_PRICE_DATABASE`), WIM | basta con elegir variables |
| fichero de cuenta con datos por personaje, sin AceDB | Pawn (`PerCharacterOptions`), EllesmereUI (`chars`: oro, clase, raza) | hay que quedarse solo con el personaje de origen |
| clave `Nombre-Reino` **sin apellido** | SkillUpForever, TalentsForeverBook | dos personajes con el mismo nombre comparten clave: no se puede saber de quién es |
| solo estado | registro de errores, datos de sesión, recetas aprendidas | no se exportan |

## Orden de decisión

La app sirve para **cualquier addon**, también los que nadie ha mirado nunca: las reglas 2 a 5 son genéricas y
no necesitan adaptador. Los adaptadores escritos (regla 1) son solo las **excepciones**: addons en los que las
reglas genéricas se equivocarían, por ejemplo porque llevan progreso o datos de otros jugadores.

Para cada addon con datos, se usa lo primero que aplique:

1. **Adaptador escrito** para ese `addon`.
2. **AceDB detectado**: la variable de cuenta tiene `profileKeys` y `profiles`. Viaja el perfil que usa el
   personaje (`perfil-acedb`).
3. **Genérico de cuenta**: fichero de cuenta que no menciona a ningún personaje de la cuenta. Viaja entero, pero
   afecta a todos los personajes del destino: la app enseña qué sobrescribe y solo lo aplica si el usuario
   acepta.
4. **Genérico por personaje**: fichero por personaje sin adaptador. Se ofrece **desmarcado** y con el aviso
   «puede incluir progreso o datos de otros jugadores».
5. **Genérico de cuenta con personajes**: fichero de cuenta que menciona personajes y no es AceDB. Se ofrece
   entero, **desmarcado**, con el aviso «puede llevar progreso o datos de otros personajes, y afecta a todos los
   personajes de la cuenta del destino». Si el usuario lo marca, la app enseña qué sobrescribe y pide
   confirmación.

**Regla dura:** ningún adaptador escrito incluye datos de otros jugadores (nombres, GUID, conversaciones, listas
de enemigos o de compañeros de grupo). Las reglas genéricas 4 y 5 no pueden garantizarlo, y por eso van
desmarcadas y con aviso: decide el usuario.

## Cómo se escribe un adaptador

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

- `reglas`: una por variable (las de `## SavedVariables` o `## SavedVariablesPerCharacter` del `.toc`).
  **Una variable que no aparece en las reglas no viaja.** Es una lista blanca: si el addon añade mañana una
  variable de estado, no se filtra por accidente.
- Cada regla elige **una** forma de seleccionar:

| campo | qué viaja |
|---|---|
| `"entera": true` | la variable completa. **Exige `versionesAddon`** (ver abajo) |
| `"incluir": [rutas]` | solo esas claves |
| `"excluir": [rutas]` | todo menos esas claves. **Exige `versionesAddon`** (ver abajo) |
| `"perfilAcedb": true` | el perfil AceDB del personaje (solo en variables de cuenta) |

- **Regla para cualquier adaptador: lo que no se ha nombrado no viaja sin una versión medida.**
  - `incluir` y `perfilAcedb` solo dejan pasar lo que nombran (claves concretas, o el perfil). Una clave que el
    addon añada en una actualización no viaja hasta que alguien la añade al adaptador.
  - `entera` y `excluir` dejan pasar claves que nadie ha listado. Por eso el schema los rechaza si el adaptador no
    declara `versionesAddon`, y la app, con otra versión instalada, ofrece la entrada desmarcada y con aviso.
  - Se prefiere `incluir`: no envejece con cada actualización del addon.
- `soloPersonaje` (opcional, junto a `entera`, `incluir` o `excluir`): rutas de tablas indexadas por personaje.
  Solo viaja la entrada del personaje de origen y, al importar, se escribe con la clave del destino.
- `clavePersonaje`: cómo escribe el addon esa clave. `nombre-apellido` (`"Nombre Apellido"`, como AceDB en
  Forever) es el valor por defecto.
- `noExportar: true` en lugar de `reglas`: el addon solo guarda estado o datos ajenos, y nunca se ofrece.
- **Rutas**: claves separadas por `/`. Un segmento es un nombre literal, `*` (cualquier clave de ese nivel) o
  `$PJ` (la clave del personaje). Ejemplo: `Scales/*/PerCharacterOptions/$PJ`.

### Cómo se aplica en el destino

- Se **fusiona por clave**: en el fichero del destino solo se sustituyen las claves que viajan. El resto del
  fichero del destino no se toca.
- Las reglas de `ambito: "cuenta"` cambian un fichero que comparten todos los personajes del destino. Si solo
  llevan `soloPersonaje`, solo cambian la entrada del personaje destino y no hace falta confirmar. Si llevan
  más, la app enseña qué sobrescribe y pide confirmación.
- Si la versión instalada del addon no está en `versionesAddon`, la entrada se ofrece desmarcada y con aviso.
- Los ficheros se leen y se escriben como datos (Lua sin ejecutar), con cambios mínimos.

## Adaptadores escritos (excepciones medidas)

Addons en los que las reglas genéricas se equivocarían, medidos el 2026-11-17 en Forever 1.60.1. La lista
crece con cada addon que se mida (los populares, en la Fase 5) y con los que proponga cualquiera por PR. La clasificación de cada clave como preferencia o estado es **[I]**
salvo que se diga otra cosa: se lee el nombre y el contenido, no el código del addon.

### ForeverDungeonJournal

```json
{
  "id": "forever-dungeon-journal",
  "version": 1,
  "addon": "ForeverDungeonJournal",
  "medido": "ForeverDungeonJournalCharacterDB (por personaje) tiene 16 claves: journalMode, favouriteItems, trackerSlots, trackedDungeons y hiddenDungeons son preferencias; rpgTrash, rpgBosses, rpgKills, rpgMaps, rpgQuests, runTracker, runLog, flightPoints, flightPointsVersion, hearthSpot y arrowChains son progreso.",
  "reglas": [
    {
      "ambito": "personaje",
      "variable": "ForeverDungeonJournalCharacterDB",
      "incluir": ["journalMode", "favouriteItems", "trackerSlots", "trackedDungeons", "hiddenDungeons"]
    }
  ]
}
```

`journalMode` es la elección Gamer/RPG que el addon vuelve a pedir si falta [V].

### LFGForever

```json
{
  "id": "lfg-forever",
  "version": 1,
  "addon": "LFGForever",
  "medido": "LFGForeverCharDB (por personaje): inviteMessage y askToJoinMessage son textos del usuario; recentPartyMembers guarda nombre, GUID, clase y nivel de otros jugadores.",
  "reglas": [
    { "ambito": "personaje", "variable": "LFGForeverCharDB", "incluir": ["inviteMessage", "askToJoinMessage"] }
  ]
}
```

### WIM

```json
{
  "id": "wim",
  "version": 2,
  "addon": "WIM",
  "versionesAddon": ["3.19.7"],
  "medido": "WIM 3.19.7. Cuenta: WIM3_Data (42 claves de opciones: ventanas, colores, sonidos, reglas), WIM3_Filters y WIM3_ChatFilters son configuración; WIM3_History, WIM3_HistoryArchive y WIM3_Cache guardan conversaciones y otros jugadores. Por personaje: WIM3_DebugLog y WIM3_Snapshots.",
  "reglas": [
    { "ambito": "cuenta", "variable": "WIM3_Data", "entera": true },
    { "ambito": "cuenta", "variable": "WIM3_Filters", "entera": true },
    { "ambito": "cuenta", "variable": "WIM3_ChatFilters", "entera": true }
  ]
}
```

### Auctionator

```json
{
  "id": "auctionator",
  "version": 1,
  "addon": "Auctionator",
  "versionesAddon": ["347"],
  "medido": "Cuenta: AUCTIONATOR_CONFIG (68 opciones), AUCTIONATOR_SHOPPING_LISTS y AUCTIONATOR_SELLING_GROUPS son configuración; AUCTIONATOR_PRICE_DATABASE (precios por reino), AUCTIONATOR_POSTING_HISTORY, AUCTIONATOR_VENDOR_PRICE_CACHE, AUCTIONATOR_RECENT_SEARCHES y AUCTIONATOR_SAVEDVARS (marcas de escaneo) son estado. Por personaje: AUCTIONATOR_CHARACTER_CONFIG.",
  "reglas": [
    { "ambito": "cuenta", "variable": "AUCTIONATOR_CONFIG", "entera": true },
    { "ambito": "cuenta", "variable": "AUCTIONATOR_SHOPPING_LISTS", "entera": true },
    { "ambito": "cuenta", "variable": "AUCTIONATOR_SELLING_GROUPS", "entera": true },
    { "ambito": "personaje", "variable": "AUCTIONATOR_CHARACTER_CONFIG", "entera": true }
  ]
}
```

### Pawn

```json
{
  "id": "pawn",
  "version": 2,
  "addon": "Pawn",
  "medido": "Pawn 2.14.3. Cuenta: PawnCommon tiene 21 claves; 19 son opciones (Scales incluida) y LastVersion y ShownGettingStarted son estado. Dentro de cada escala, PerCharacterOptions indexado por \"Nombre Apellido\". Por personaje: PawnOptions con AutoSelectScales (preferencia) y LastVersion, LastPlayerFullName, ItemLevels y LastKeybindingsSet (estado).",
  "reglas": [
    {
      "ambito": "cuenta",
      "variable": "PawnCommon",
      "incluir": [
        "AlignNumbersRight", "ButtonPosition", "ColorTooltipBorder", "Debug", "Digits", "IgnoreGemsWhileLeveling",
        "Scales", "ShowBagUpgradeAdvisor", "ShowEnchanted", "ShowItemID", "ShowItemLevelUpgrades",
        "ShowLootUpgradeAdvisor", "ShowQuestUpgradeAdvisor", "ShowRelicUpgrades", "ShowSocketingAdvisor",
        "ShowSpecIcons", "ShowTooltipIcons", "ShowUpgradesOnTooltips", "ShowValuesForUpgradesOnly"
      ],
      "soloPersonaje": ["Scales/*/PerCharacterOptions/$PJ"]
    },
    { "ambito": "personaje", "variable": "PawnOptions", "incluir": ["AutoSelectScales"] }
  ]
}
```

En `PawnCommon`, `soloPersonaje` hace que de cada `PerCharacterOptions` viaje solo la entrada del personaje de
origen; las de los demás personajes de la cuenta se quedan fuera.

### EllesmereUI

```json
{
  "id": "ellesmereui",
  "version": 2,
  "addon": "EllesmereUI",
  "medido": "EllesmereUI 9.8.8. Cuenta: EllesmereUIDB, 58 claves. Tiene perfiles propios (profiles, activeProfile, profileOrder, specProfiles) que no son AceDB. 27 claves son configuración. Estado (31): chars (oro, clase, raza y nivel de cada personaje), goldLedger, _migrations, lastLoginVersion, patchDotPending, dismissedConflicts, _tbbLinkOwner y las marcas de 'ya visto' (_capturedOnce_*, *Seeded, *Seen, firstInstallPopupShown, kickTrackerIntroShown, charSheetCardStyleAdopted).",
  "reglas": [
    {
      "ambito": "cuenta",
      "variable": "EllesmereUIDB",
      "incluir": [
        "activeProfile", "autoLogging", "bagsUserChosen", "clickCast", "customColors", "fonts", "lastNonSpecProfile",
        "panelScale", "ppUIScale", "ppUIScaleAuto", "profileKeybinds", "profileOrder", "profiles", "reskinBagBar",
        "reskinFlagCarriers", "reskinMicroMenu", "restoreOnUninstall", "specProfiles", "spellAssignments",
        "threatMeter", "unlockAnchors", "unlockGridMode", "unlockHeightMatch", "unlockSnapEnabled", "unlockWidthMatch",
        "windowLookPerProfile", "windowSkinStyleSlots"
      ]
    }
  ]
}
```

Afecta a toda la cuenta: pide confirmación.

### TalentsForeverBook

```json
{
  "id": "talents-forever-book",
  "version": 1,
  "addon": "TalentsForeverBook",
  "medido": "Cuenta: TalentsForeverBookDB con settings y builds (configuración) y prices, chars, sent, diag, version y addon (estado). chars usa la clave 'Nombre-Reino interno' sin apellido: no distingue personajes con el mismo nombre, así que los planes por personaje no viajan.",
  "reglas": [
    { "ambito": "cuenta", "variable": "TalentsForeverBookDB", "incluir": ["settings", "builds"] }
  ]
}
```

### Spy

```json
{
  "id": "spy",
  "version": 1,
  "addon": "Spy",
  "medido": "Cuenta: SpyDB es AceDB. Por personaje: SpyPerCharDB con KOSData, PlayerData, IgnoreData y Encounters, todo datos de otros jugadores.",
  "reglas": [
    { "ambito": "cuenta", "variable": "SpyDB", "perfilAcedb": true }
  ]
}
```

Sin este adaptador, la regla AceDB cubriría `SpyDB`, pero el genérico por personaje ofrecería `SpyPerCharDB`.
El adaptador lo cierra: como `SpyPerCharDB` no está en las reglas, no viaja.

### Addons que no se exportan

```json
{
  "id": "skillup-forever",
  "version": 1,
  "addon": "SkillUpForever",
  "medido": "Recetas aprendidas y vistas por personaje, con clave 'Nombre-Reino' sin apellido: estado.",
  "noExportar": true
}
```

```json
{
  "id": "layerswap",
  "version": 1,
  "addon": "LayerSwap",
  "medido": "Datos de sesión (session, previous, hostLockWho) con nombres de personajes: estado.",
  "noExportar": true
}
```

```json
{
  "id": "buggrabber",
  "version": 1,
  "addon": "!BugGrabber",
  "medido": "Registro de errores de Lua capturados: estado.",
  "noExportar": true
}
```

## Adaptadores internos

El **Modo Edición** de Blizzard no es un `SavedVariables`: son `edit-mode-cache-account.txt` (los diseños) y
`edit-mode-cache-character.txt` (qué diseño usa cada especialización). Lo resuelve un adaptador interno de la
app, `modo-edicion`, porque no se puede describir con rutas de tabla. Su formato está medido solo en parte [H].

## Pendiente

- Adaptadores de addons populares no medidos aquí (WeakAuras, Plater, Details!, Baganator, Questie, Bartender4,
  TradeSkillMaster…).
- Comprobar en el juego cada adaptador escrito, como se hizo con AceDB.
