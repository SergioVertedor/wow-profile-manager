# Compatibilidad con WoW Forever: qué se puede exportar e importar

Resultado de la Fase 1 (investigación). Medido en noviembre de 2026 sobre una instalación real de WoW Forever,
cliente `wowf` **1.60.1.70938** (Interface 16001), copiando la configuración de un personaje «origen» a otro
«destino» de la misma cuenta y comprobándola dentro del juego.

Marcas:
- **[V]** verificado: medido en los ficheros o probado en el juego.
- **[I]** inferencia a partir de lo medido.
- **[H]** hipótesis, todavía sin probar.

Ninguna documentación oficial de Blizzard describe estos ficheros: lo que hay aquí sale de medirlos.

## Dónde guarda Forever la configuración [V]

- Instalación: carpeta `_forever_`, producto `wowf` en `.build.info`. Retail (`_retail_`, producto `wow`) puede
  convivir en la misma instalación, con su propio `WTF/` e `Interface/`.
- Personaje: `WTF/Account/<cuenta>/<n>/Nombre-Apellido/`. Los personajes de Forever tienen nombre y apellido;
  en la carpeta van unidos por un guion.
- Cuenta: `WTF/Account/<cuenta>/` y `WTF/Account/<cuenta>/SavedVariables/`.
- Global: `WTF/Config.wtf`, con gráficos y ajustes de hardware.
- **Qué hay en cada botón de las barras de acción no está en ningún fichero de `WTF`**: lo guarda el servidor.

## `cache.md5`: sincronización con el servidor [V]

- 240 bytes: **10 ranuras de 24 bytes**. Cada ranura es el **MD5 de los bytes exactos** de un fichero (16 bytes)
  más una marca de tiempo Unix `uint64` little-endian (8 bytes). Ranura vacía = MD5 a ceros.
- La ranura → fichero es fija:

| ranura | cuenta | personaje |
|---|---|---|
| 0 | `config-cache.wtf` | `config-cache.wtf` |
| 1 | `bindings-cache.wtf` | `bindings-cache.wtf` |
| 2 | `macros-cache.txt` | `macros-cache.txt` |
| 3 | `tts-cache-account.txt` | `layout-local.txt` |
| 4 | `flagged-cache-account.txt` | `chat-cache.txt` |
| 5 | `edit-mode-cache-account.txt` | `tts-cache-character.txt` |
| 6 | vacía | `flagged-cache-character.txt` |
| 7 | `character-list-order.txt` | `click-bindings-cache.txt` |
| 8 | — | `edit-mode-cache-character.txt` |
| 9 | — | `cooldownmanager.txt` |

- `AddOns.txt`, `CUFProfiles.txt` y `SavedVariables/` no tienen ranura: no se sincronizan.
- **Regla de importación medida**: con WoW cerrado, se copia el fichero al personaje destino **sin tocar su
  `cache.md5`**. Al entrar, el cliente adopta el fichero local y actualiza la ranura. Probado con la ranura
  vacía, con la ranura ocupada por otro contenido y en las ranuras 0, 1, 2, 4 y 8.
- En 1.60.1 **gana el fichero local** aunque la ranura tenga otro MD5. Otras herramientas, medidas en la beta,
  describen lo contrario.

## Configuración de addons [V]

- Los addons guardan su configuración en `SavedVariables/*.lua`, de cuenta o de personaje. Son tablas Lua: se
  leen y reescriben sin ejecutarlas.
- **AceDB en Forever** usa como clave de personaje `"Nombre Apellido"`, sin reino. Llevar un perfil a otro
  personaje es añadir el perfil a `profiles` y apuntar `profileKeys["<destino>"]` a él.
- La mayoría de los addons medidos usan también `"Nombre Apellido"`. Algunos tienen formato propio
  (`"Nombre Apellido-Reino"`, `"Nombre Apellido - Reino"`, o el nombre como valor) y necesitan un adaptador.
- Algunos addons mezclan preferencias y progreso en la misma tabla: copiar el fichero entero arrastra el
  progreso.
- El `.toc` de un addon sirve de pista, no de verdad: hay `SavedVariables` de addons ya desinstalados y `.toc`
  que declaran ficheros que no existen.
- Hay `SavedVariables` que no son UTF-8 válido (datos binarios dentro de cadenas Lua). Se tratan como bytes.

## Matriz de compatibilidad

Ámbito: P = personaje, C = cuenta, G = global.

| categoría | fichero(s) | ámbito | ranura | cómo se importa | estado |
|---|---|---|---|---|---|
| Macros de personaje | `macros-cache.txt` | P | 2 | copiar, `cache.md5` intacto | **[V]** |
| Macros generales | `macros-cache.txt` | C | 2 | afecta a toda la cuenta: aviso y confirmación | [H] |
| Teclas | `bindings-cache.wtf` | P o C | 1 | copiar al personaje destino | **[V]** |
| Teclas de clic | `click-bindings-cache.txt` | P | 7 | copiar | [H] |
| Opciones de personaje | `config-cache.wtf` | P | 0 | copiar quitando las claves de estado | **[V]** |
| Opciones de cuenta | `config-cache.wtf` | C | 0 | afecta a toda la cuenta: aviso y confirmación | [H] |
| Chat | `chat-cache.txt` | P | 4 | copiar | **[V]** |
| Disposición de ventanas | `layout-local.txt` | P | 3 | copiar | [H] |
| Modo Edición: diseño que usa | `edit-mode-cache-character.txt` | P | 8 | copiar | **[V]** misma cuenta |
| Modo Edición: diseños | `edit-mode-cache-account.txt` | C | 5 | añadir el diseño y reescribir el índice | [H] entre cuentas |
| Addons activos | `AddOns.txt` | P | — | copiar solo los addons instalados | **[V]** |
| Configuración AceDB | `SavedVariables/*.lua` | C | — | solo perfiles (`profiles` y `profileKeys`) | **[V]** |
| Configuración por personaje | `SavedVariables/*.lua` | P | — | copiar con la clave del destino | **[V]** |
| Addons sin AceDB | propio de cada addon | C y P | — | adaptador por addon | [H] |
| Marcos de banda | `CUFProfiles.txt` | P | — | copiar | [H] |
| Gráficos | `Config.wtf` | G | — | no se exporta: depende del hardware | [I] |
| Barras de acción | ninguno | servidor | — | no se puede por ficheros | [V] |

Macros de un personaje de Retail copiadas a uno de Forever funcionaron [V]. La configuración de addons entre
versiones necesita traducir claves (`"Nombre - Reino"` en Retail, `"Nombre Apellido"` en Forever).

## Criterios de importación

- Todo se escribe **con WoW cerrado**, con backup antes y posibilidad de deshacer.
- De los addons AceDB se importan **solo los perfiles**, nunca la sección `char`, que guarda estado del
  personaje.
- Las **claves de estado** dentro de ficheros de opciones (por ejemplo `hardTrackedQuests`, las misiones
  seguidas) se quitan al exportar.
- Lo que el personaje usa a nivel de cuenta se aplica **solo al personaje destino** cuando se puede (teclas,
  perfiles AceDB, diseño del Modo Edición). Cuando no se puede (macros generales, opciones de cuenta), la app
  muestra qué se va a sobrescribir y solo lo aplica si el usuario acepta.
- Copiar entre clases distintas es decisión del usuario: la app como mucho avisa. La clase no se puede leer de
  forma fiable de los ficheros del cliente.

## Pendiente de probar

- Importar en **otra máquina** y en **otra cuenta**: todas las pruebas se hicieron entre dos personajes de la
  misma cuenta.
- Los diseños del Modo Edición entre cuentas.
- `layout-local.txt`, `click-bindings-cache.txt`, `CUFProfiles.txt` y `cooldownmanager.txt` por separado.
- Adaptadores para los addons sin AceDB y para addons populares no medidos (WeakAuras, Plater, Details!,
  Baganator, Questie, Bartender4, TradeSkillMaster…).
- Si un nombre o apellido con guion hace ambigua la carpeta `Nombre-Apellido`.
