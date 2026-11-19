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

Exportar = copiar el perfil que usa el personaje (`profiles["Default"]`) y, si el addon tiene módulos
(`namespaces`, medido en Prat, Dominos y Bazooka), el perfil de ese mismo nombre de cada módulo. Al lado de los
`profiles` de un módulo puede haber `char` o `realm` con estado y nombres de otros jugadores: eso no viaja. Importar = añadirlo al fichero del
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

Se decide **por pieza**: cada variable de cuenta y cada fichero por personaje. Un mismo addon puede tener piezas
que caen en reglas distintas. Solo se miran los addons **instalados**; las variables salen del fichero, no del `.toc`.

1. **Adaptador escrito** para ese `addon`.
2. **AceDB detectado**: la variable de cuenta tiene `profileKeys` y `profiles`. Viaja el perfil que usa el
   personaje (`perfil-acedb`).
3. **Genérico de cuenta**: fichero de cuenta que no menciona a ningún personaje de la cuenta. Se ofrece entero,
   **desmarcado**, con el aviso base. En el destino afecta a todos los personajes: la app enseña qué sobrescribe y
   solo lo aplica si el usuario acepta.
4. **Genérico por personaje**: fichero por personaje sin adaptador. Se ofrece **desmarcado** y con el aviso base.
5. **Genérico de cuenta con personajes**: fichero de cuenta que menciona personajes y no es AceDB. Se ofrece
   entero, **desmarcado**, con el aviso base y «menciona a personajes de esta cuenta». Como la 3, en el destino
   afecta a todos los personajes y pide confirmación.

Aviso base de las reglas 3, 4 y 5: «puede llevar progreso, estado o datos de otros jugadores».

**Regla dura:** ningún adaptador escrito incluye datos de otros jugadores (nombres, GUID, conversaciones, listas
de enemigos o de compañeros de grupo). Las reglas genéricas 3, 4 y 5 no pueden garantizarlo, y por eso van
desmarcadas y con aviso: decide el usuario.

Por qué la 3 tampoco: un nombre suelto de otro jugador es una cadena como cualquier otra, y ninguna comprobación
sobre el texto puede asegurar que no esté. Medido en una instalación [V]: de 31 ficheros de cuenta en los que no
aparecía ningún personaje de la cuenta con su formato habitual, 3 sí guardaban personajes (un GUID de jugador y
claves `Nombre-Reino` sin apellido).

**Pistas.** Al exportar, la app busca en los ficheros de las reglas genéricas lo que sí se puede reconocer: GUID de
jugador (`Player-<servidor>-<8 cifras hexadecimales>`) y claves con forma `Nombre-Reino` o `Nombre - Reino` (nombre de
una palabra con mayúscula y de 2 a 12 letras). La forma `Nombre Apellido` no es pista: es la de muchos textos que no son
personajes. Si encuentra alguna, el aviso lo dice («parece contener otros jugadores»).
Las pistas **nunca marcan una entrada**: que no haya pistas no demuestra nada.

Para no obligar a ir addon a addon, la selección tiene una acción «marcar todos» por cada tipo de regla genérica.
No marca las piezas con pistas: esas se marcan una a una, viendo su aviso.

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
  - Lo aplica la regla del catálogo de la app, no lo que diga la pieza: una clave que la regla no nombra no llega
    aunque venga en el paquete.
  - Lo que la regla deja pasar pero la pieza no trae se queda como está en el destino: nada se borra. `entera` sustituye
    la variable entera.
  - En `soloPersonaje`, la entrada del marcador se escribe con la clave del personaje destino; las de los demás
    personajes del destino no cambian.
- Las reglas de `ambito: "cuenta"` cambian un fichero que comparten todos los personajes del destino. Si solo
  llevan `soloPersonaje`, solo cambian la entrada del personaje destino y no hace falta confirmar. Si llevan
  más, la app enseña qué sobrescribe y pide confirmación.
- Si la versión instalada del addon no está en `versionesAddon` y la regla usa `entera` o `excluir`, la vista previa
  lo avisa.
- Los ficheros se leen y se escriben como datos (Lua sin ejecutar), con cambios mínimos.

## Adaptadores escritos (excepciones medidas)

Cada adaptador es un fichero JSON en [`src-tauri/adaptadores/`](../src-tauri/adaptadores/), que va dentro del
ejecutable: esa es la única copia. Aquí se explica cada uno.

Addons en los que las reglas genéricas se equivocarían, medidos el 2026-11-17 en Forever 1.60.1. La lista
crece con cada addon que se mida (los populares, en la Fase 5) y con los que proponga cualquiera por PR. La clasificación de cada clave como preferencia o estado es **[I]**
salvo que se diga otra cosa: se lee el nombre y el contenido, no el código del addon.

### ForeverDungeonJournal

Fichero: [`forever-dungeon-journal.json`](../src-tauri/adaptadores/forever-dungeon-journal.json).

`journalMode` es la elección Gamer/RPG que el addon vuelve a pedir si falta [V].

### LFGForever

Fichero: [`lfg-forever.json`](../src-tauri/adaptadores/lfg-forever.json).

Viajan los dos mensajes del usuario (`inviteMessage`, `askToJoinMessage`); `recentPartyMembers` guarda otros jugadores y no
viaja.

### WIM

Fichero: [`wim.json`](../src-tauri/adaptadores/wim.json).

Viajan las opciones y los filtros (`WIM3_Data`, `WIM3_Filters`, `WIM3_ChatFilters`), enteros, solo con la versión
medida. Las conversaciones y la caché de jugadores no viajan.

### Auctionator

Fichero: [`auctionator.json`](../src-tauri/adaptadores/auctionator.json).

Viajan la configuración, las listas de la compra y los grupos de venta, enteros, solo con la versión medida. Precios,
historial y búsquedas recientes no viajan. Medido el 2026-11-18: el nombre de una lista lo pone el usuario (o un addon,
como `SkillUp: <personaje>: Cocina`) y viaja tal cual, como el texto de una macro.

### Pawn

Fichero: [`pawn.json`](../src-tauri/adaptadores/pawn.json).

En `PawnCommon`, `soloPersonaje` hace que de cada `PerCharacterOptions` viaje solo la entrada del personaje de
origen; las de los demás personajes de la cuenta se quedan fuera.

### EllesmereUI

Fichero: [`ellesmereui.json`](../src-tauri/adaptadores/ellesmereui.json).

Afecta a toda la cuenta: pide confirmación. Versión 3 (2026-11-18): `restoreOnUninstall` sale de la lista; es una copia
de toda la base del addon, con `chars` y GUID de personajes, para restaurarla al desinstalar.

### TalentsForeverBook

Fichero: [`talents-forever-book.json`](../src-tauri/adaptadores/talents-forever-book.json).

Viajan `settings` y `builds`. Los planes por personaje no, porque su clave no distingue personajes con el mismo nombre.

### Spy

Fichero: [`spy.json`](../src-tauri/adaptadores/spy.json).

Sin este adaptador, la regla AceDB cubriría `SpyDB`, pero el genérico por personaje ofrecería `SpyPerCharDB`.
El adaptador lo cierra: como `SpyPerCharDB` no está en las reglas, no viaja.

### Addons que no se exportan

SkillUpForever (recetas por personaje), LayerSwap (datos de sesión) y !BugGrabber (errores capturados) solo guardan
estado: nunca se ofrecen.

Fichero: [`skillup-forever.json`](../src-tauri/adaptadores/skillup-forever.json).

Fichero: [`layerswap.json`](../src-tauri/adaptadores/layerswap.json).

Fichero: [`buggrabber.json`](../src-tauri/adaptadores/buggrabber.json).

## Adaptadores internos

El **Modo Edición** de Blizzard no es un `SavedVariables`: son `edit-mode-cache-account.txt` (los diseños) y
`edit-mode-cache-character.txt` (qué diseño usa cada especialización). Lo resuelve un adaptador interno de la
app, `modo-edicion` (versión 1), porque no se puede describir con rutas de tabla. Exporta solo el diseño que usa el
personaje y, al importar, lo añade a los diseños de la cuenta y apunta el personaje a él. Formato medido en
[`compatibilidad-wow-forever.md`](compatibilidad-wow-forever.md#modo-edición-v).

## Pendiente

- Adaptadores de addons populares no medidos aquí (WeakAuras, Plater, Details!, Baganator, Questie, Bartender4,
  TradeSkillMaster…).
- Comprobar en el juego cada adaptador escrito, como se hizo con AceDB.
