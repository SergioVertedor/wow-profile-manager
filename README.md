# WoW Profile Manager

Gestor de perfiles de configuración para **WoW Forever**, en Windows. Sirve para
exportar la configuración de un personaje (addons, su configuración, macros,
teclas y barras) a un único fichero `.wowprofile`. Otro jugador, en otro
ordenador, elige qué importar y a qué personaje suyo aplicarlo. Antes de
escribir, la app hace backup y muestra la vista previa de los cambios; después
permite deshacer.

> **Estado: Fase 1 (investigación).** Aún no hay código de la aplicación. El
> stack (Tauri 2 + Rust + React como primera candidata) **no está decidido**.

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
