# Formato `.wowprofile` 1.0

Un `.wowprofile` lleva la configuración de un personaje de WoW a otro, normalmente de otro jugador en otro
ordenador. Este documento es el contrato entre exportar e importar. El manifiesto se valida con
[`wowprofile.schema.json`](wowprofile.schema.json) (JSON Schema 2020-12). Lo medido que lo justifica está en
[`compatibilidad-wow-forever.md`](compatibilidad-wow-forever.md).

## Principios

1. **Piezas, no ficheros enteros.** Los `SavedVariables` de cuenta guardan datos de todos los personajes de la
   cuenta. Se exporta solo lo que es del personaje: el perfil que usa, sus macros, sus teclas.
2. **Configuración, no estado.** Fuera el progreso, las misiones seguidas y las tablas `char` de AceDB.
3. **Nada privado.** Sin número de cuenta, sin reino, sin el nombre del personaje de origen y sin rutas de
   disco. El único nombre es el que el usuario pone al perfil.

Los tres primeros los garantizan las piezas de AceDB y de adaptador. Una pieza de una regla genérica de
[`adaptadores.md`](adaptadores.md#orden-de-decisión) viaja como fichero entero y puede romperlos (estado, nombres
del origen o de otros jugadores). Por eso sale desmarcada y con aviso, y solo viaja si el usuario la marca.
4. **El paquete no decide dónde se escribe.** Lleva rutas lógicas. La app que importa las traduce al personaje
   que elige el usuario.
5. **Lua es dato.** Ninguna pieza se ejecuta: se lee y se reescribe como texto.

## Contenedor

- Un ZIP (método `deflate` o `stored`) con extensión `.wowprofile`. El nombre del fichero es libre.
- Contenido:

```
manifest.json        ← UTF-8, sin BOM
datos/<id>           ← una pieza por entrada del manifiesto, bytes tal cual
```

- Nada más. Cualquier otra entrada del ZIP, carpeta incluida, hace que el paquete se rechace.

## Manifiesto

| campo | obligatorio | qué es |
|---|---|---|
| `formato.nombre` | sí | siempre `"wowprofile"` |
| `formato.version` | sí | `"MAYOR.MENOR"`; este documento es la `1.0` |
| `perfil.nombre` | sí | nombre descriptivo que pone el usuario (1–80 caracteres) |
| `perfil.descripcion` | no | texto libre (hasta 500) |
| `perfil.creado` | sí | fecha de exportación, RFC 3339 en UTC (`…Z`) |
| `origen.producto` | sí | `wowf` (Forever) o `wow` (Retail), de `.build.info` |
| `origen.build` | sí | versión del cliente, p. ej. `1.60.1.70938` |
| `origen.interfaz` | sí | número de interfaz, p. ej. `16001` |
| `origen.clase` | no | clase del personaje (`DRUID`…), solo si se pudo saber; sirve para avisar, nunca para bloquear. La app no la escribe todavía: no se lee de forma fiable de los ficheros |
| `addons[]` | sí | addons que el paquete menciona: los de sus piezas `config-addon` y los instalados que su `AddOns.txt` deja activados (puede estar vacía) |
| `entradas[]` | sí | una por pieza en `datos/` |

### `addons[]`

Solo la descripción del addon. **El código del addon nunca viaja**: quien importa ve qué le falta y dónde
conseguirlo.

| campo | obligatorio | de dónde sale |
|---|---|---|
| `id` | sí | nombre de la carpeta en `Interface/AddOns` |
| `titulo` | sí | `## Title` del `.toc` |
| `version` | no | `## Version` (no todos la declaran) |
| `enlace` | no | `## X-Website` o similar, solo `https://` |
| `dependencias` | no | `## Dependencies` / `## RequiredDeps`, como `id` |

### `entradas[]`

| campo | obligatorio | qué es |
|---|---|---|
| `id` | sí | `^[a-z0-9][a-z0-9-]{0,63}$`; la pieza está en `datos/<id>` |
| `categoria` | sí | lo que ve el usuario al elegir qué importa (tabla de abajo) |
| `tipo` | sí | cómo se extrajo la pieza y cómo se aplica (tabla de abajo) |
| `ambito` | sí | dónde estaba en el origen: `personaje` o `cuenta` |
| `destino` | sí | ruta lógica donde se aplica por defecto: `personaje/<fichero>` o `cuenta/[SavedVariables/]<fichero>` |
| `addon` | según tipo | `id` del addon al que pertenece |
| `sha256` | sí | SHA-256 de la pieza, en hexadecimal minúsculas |
| `bytes` | sí | tamaño exacto de la pieza |
| `marcadores` | no | `PERSONAJE` y/o `REINO` si la pieza los contiene |

`ambito` y `destino` pueden no coincidir. Ejemplo: un personaje sin teclas propias usa las de la cuenta
(`ambito: cuenta`), y se importan como teclas del personaje destino (`destino: personaje/bindings-cache.wtf`)
para no tocar al resto de personajes de esa cuenta.

### Categorías

Una versión menor puede añadir más. El contenido de los botones de las barras de acción no será una: no está
en ningún fichero, lo guarda el servidor.

| categoría | tipo | `destino` |
|---|---|---|
| `macros` | `fichero` | `personaje/macros-cache.txt` o `cuenta/macros-cache.txt` (macros generales) |
| `teclas` | `fichero` | `personaje/bindings-cache.wtf` (también si en el origen eran de cuenta) |
| `teclas-clic` | `fichero` | `personaje/click-bindings-cache.txt` |
| `opciones` | `fichero-filtrado` | `personaje/config-cache.wtf` o `cuenta/config-cache.wtf` |
| `chat` | `fichero` | `personaje/chat-cache.txt` |
| `ventanas` | `fichero` | `personaje/layout-local.txt` |
| `modo-edicion` | dos entradas: `fichero` (qué diseño usa) y `adaptador` `modo-edicion` (el diseño) | `personaje/edit-mode-cache-character.txt` y `cuenta/edit-mode-cache-account.txt` |
| `addons-activos` | `fichero-filtrado` | `personaje/AddOns.txt` |
| `config-addon` | `perfil-acedb`, `fichero` o `adaptador` | `cuenta/SavedVariables/<addon>.lua` o `personaje/SavedVariables/<addon>.lua`, con `<addon>` = la carpeta del addon (el juego guarda así sus variables) |
| `marcos-banda` | `fichero` | `personaje/CUFProfiles.txt` |

Reglas de las categorías de fichero:

- **Teclas.** Si el personaje tiene `bindings-cache.wtf`, viaja ese (`ambito: personaje`). Si no, usa las de la cuenta
  [H] y viaja el de la cuenta (`ambito: cuenta`), siempre hacia `personaje/bindings-cache.wtf`.
- **Opciones** (`config-cache.wtf`, del personaje y de la cuenta). Se quitan las líneas `SET <clave>` de estado y la de
  seguridad; las demás viajan, porque son opciones del cliente. Sin distinguir mayúsculas, como los CVar:
  - exactas: `AllowDangerousScripts` (quitaría al que importa el aviso al ejecutar scripts), `lastSelectedClubId`,
    `EJLootClass`, `flaggedTutorials`, `closedInfoFrames`, `closedInfoFramesAccountWide`, `numReputationHeaders`,
    `numCurrencyCategories`;
  - que empiezan por `tracked`, `hardTracked`, `cameraSaved`, `perksActivities` o `petJournal`;
  - que contienen `collapsed`.
- **Addons activos** (`AddOns.txt`). Se quitan las líneas de addons que no están instalados en el origen. Las de
  `Blizzard_*` siempre viajan: son módulos del cliente, no están en `Interface/AddOns` y se pueden desactivar.
- **Chat** viaja entero: solo lleva ventanas, colores, tipos de mensaje y canales [V].
- **Al importar**, las opciones y los addons activos se fusionan línea a línea con lo que ya tiene el destino, y la lista
  de claves de opciones que no viajan se vuelve a aplicar: un paquete hecho a mano podría traerlas. El resto de
  categorías de fichero sustituyen el fichero entero ([`flujos.md`](flujos.md#cómo-se-escribe-cada-categoría)).

No viajan en la `1.0`: `flagged-cache-*` (tutoriales vistos), `character-list-order.txt` (orden de la pantalla de
selección), `tts-cache-*` y `cooldownmanager.txt` (sin medir), `cache.md5` y las copias `.old` y `.bak` del juego.

### Tipos de entrada

| tipo | qué lleva la pieza | campos extra | ejemplo |
|---|---|---|---|
| `fichero` | el fichero entero | — | `macros-cache.txt` del personaje |
| `fichero-filtrado` | el fichero sin las claves de estado o las líneas que no deben viajar | `claves_quitadas` | `config-cache.wtf` sin `hardTrackedQuests`; `AddOns.txt` sin los addons que no estaban instalados |
| `perfil-acedb` | solo la tabla del perfil, en texto Lua | `addon`, `variable`, `perfil` | el perfil `Default` de `DominosDB` |
| `adaptador` | lo que defina el adaptador de ese addon | `adaptador.id`, `adaptador.version`, `addon` si es de un addon | preferencias de un addon que mezcla progreso |

Cómo se aplica cada tipo al importar:
- `fichero` y `fichero-filtrado`: se escribe en el `destino`, con WoW cerrado, sin tocar el `cache.md5` del
  destino.
- `perfil-acedb`: se añade a `<variable>.profiles["<perfil>"]` del fichero de cuenta del destino, y se apunta
  **solo** el `profileKeys` del personaje destino a ese perfil. Si ya existe un perfil con ese nombre, la app
  pregunta (renombrar o sobrescribir).
- `adaptador`: lo aplica el adaptador con ese `id`. Si la app no lo conoce, o conoce una versión anterior, la
  entrada se ofrece desactivada y con aviso.

Las entradas de `cuenta` que no se pueden aplicar solo al personaje destino (macros generales, opciones de
cuenta) se muestran con lo que van a sobrescribir y solo se aplican si el usuario acepta.

## Marcadores

Donde una pieza lleva el nombre del personaje de origen, se sustituye al exportar por `@@WOWPROFILE:PERSONAJE@@`.
Al importar se cambia por el nombre del destino, en el formato que use ese dato (`Nombre Apellido` en Forever).
Para los addons que guardan el reino se usa `@@WOWPROFILE:REINO@@`.

- La sustitución la hace la regla de AceDB o el adaptador, que saben qué texto es el nombre. No es un buscar y
  reemplazar sobre el fichero.
- Si el origen ya contiene `@@WOWPROFILE:`, esa pieza no se exporta.
- Las macros no se reescriben: si una macro nombra a un personaje, eso es cosa del usuario.

## Leer un paquete de forma segura

Requisitos de la implementación, en este orden, **antes** de escribir nada:

1. Rechazar el ZIP si tiene más de 257 entradas (`manifest.json` + 256 piezas).
2. Leer `manifest.json` (como mucho 1 MiB) y validarlo contra el schema de su versión mayor.
   - Versión mayor desconocida: rechazar y pedir que se actualice la app.
   - Misma mayor y menor más nueva: ignorar los campos y valores desconocidos; las entradas con una categoría
     o un tipo desconocidos se ofrecen desactivadas.
3. Los `id` de las entradas son únicos. El conjunto de entradas del ZIP tiene que ser exactamente
   `manifest.json` más `datos/<id>` de cada entrada. Nunca se usa un nombre de entrada del ZIP como ruta.
4. Descomprimir cada pieza a memoria parando en `bytes` + 1. Si pasa de `bytes`, o el total pasa de 128 MiB,
   rechazar (ZIP bomba).
5. Comprobar `bytes` y `sha256` de cada pieza. Si alguna falla, se rechaza el paquete entero.
6. Traducir cada `destino` a una ruta real **dentro** de la carpeta del personaje o de la cuenta elegidos, y
   comprobar que la ruta resultante sigue dentro de esa carpeta.
   - Una entrada de categoría de fichero solo puede ir al `destino` de su categoría en la tabla de
     [Categorías](#categorías) (por ejemplo, `macros` solo a `personaje/macros-cache.txt` o `cuenta/macros-cache.txt`).
     Si no, no se importa, aunque cumpla el patrón del schema: un paquete no puede mandar un fichero a un sitio que su
     categoría no explica.
   - La ruta real sale de esa tabla, no del texto del `destino`.

Después vienen la vista previa, el backup y la escritura, con WoW cerrado.

**Integridad, no autoría.** Los hashes detectan un paquete dañado o modificado a medias, pero no dicen quién lo
hizo. El formato no lleva firma: los paquetes se pasan entre jugadores y no hay claves que verificar. La
protección está en tratar todo como dato, en las rutas lógicas, en la vista previa y en el backup.

## Versiones

- `MAYOR.MENOR`. Este documento define la `1.0`.
- **Menor**: añadir categorías, tipos o campos opcionales. Una app que lee `1.0` puede abrir un `1.3`.
- **Mayor**: cambiar el significado de un campo, quitar uno obligatorio o cambiar el contenedor.
- Los límites (256 entradas, 32 MiB por pieza, 128 MiB en total) se pueden subir en una versión menor.

## Ejemplo

Un personaje exporta macros, teclas (las de su cuenta), opciones, addons activos y el perfil de Dominos:

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

Los hashes y tamaños del ejemplo son ilustrativos.
