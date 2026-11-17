# WoW Profile Manager

Gestor de perfiles de configuración para **WoW Forever**, en Windows. Sirve para
exportar la configuración de un personaje (addons, su configuración, macros,
teclas y disposición de las barras) a un único fichero `.wowprofile`. Lo que hay
dentro de cada botón no viaja: lo guarda el servidor del juego, y cada jugador
coloca sus habilidades. Otro jugador, en otro
ordenador, elige qué importar y a qué personaje suyo aplicarlo. Antes de
escribir, la app hace backup y muestra la vista previa de los cambios; después
permite deshacer.

> **Estado: Fase 2 (diseño técnico).** Aún no hay código de la aplicación.
> Stack decidido: Tauri 2 + Rust, con `full_moon` para leer y escribir las
> `SavedVariables`. Lo medido en la Fase 1 está en
> [`docs/compatibilidad-wow-forever.md`](docs/compatibilidad-wow-forever.md), y el
> formato de los paquetes en [`docs/formato-wowprofile.md`](docs/formato-wowprofile.md) y el
> modelo de datos en [`docs/modelo-de-datos.md`](docs/modelo-de-datos.md). Los adaptadores de
> addons, en [`docs/adaptadores.md`](docs/adaptadores.md), y los flujos de exportación e
> importación, con los backups, en [`docs/flujos.md`](docs/flujos.md). La arquitectura, en
> [`docs/arquitectura.md`](docs/arquitectura.md).

## Desarrollo

```powershell
git clone https://github.com/SergioVertedor/wow-profile-manager.git
cd wow-profile-manager
git switch develop
```

## Flujo

1. Issue en GitHub.
2. Rama `feature/<issue>-<slug>` desde `develop`.
3. Implementar, con tests.
4. PR a `develop`.
5. Releases: `release/x.y.z` → `main` + tag.

## Cómo se hace

Este proyecto se desarrolla con ayuda de herramientas de IA. Las decisiones y las pruebas en el juego las hace
una persona, y todo lo que dice el proyecto sobre WoW Forever está medido en una
instalación real, no supuesto.

## Apoyar el proyecto

Si te resulta útil, puedes invitarme a un café en [Ko-fi](https://ko-fi.com/tonotabidev).

## Licencia

[GNU GPL v3](LICENSE), con un término adicional de atribución (sección 7(b)): las versiones modificadas que se
distribuyan deben mostrar en su interfaz el crédito al proyecto original. Detalles en [`NOTICE.md`](NOTICE.md).
