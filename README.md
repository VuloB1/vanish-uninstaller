# Vanish Uninstaller

Desinstalador de programas para Windows: rápido, sin publicidad y sin instalar nada extra. Después de desinstalar, busca lo que el programa dejó atrás y te deja decidir qué borrar.

Hecho con Rust + Tauri 2 y Svelte 5.

## Qué hace

- **Programas**: lista los programas del registro (64 y 32 bits) y las apps de la Microsoft Store, con iconos, categorías, búsqueda y orden.
- **Restos**: tras desinstalar (o con eliminación forzada) busca carpetas, claves de registro, servicios, inicio automático y tareas programadas del programa. Cada resto lleva un nivel de confianza y solo se marcan los de confianza alta.
- **Desinstalación en lote** de varios programas seguidos.
- **Limpiar caché**: temporales, miniaturas, navegadores, shaders de GPU, cachés de desarrollo, volcados y Windows Update.
- **Registro**: entradas huérfanas (desinstalación, rutas de aplicación, DLL compartidas, inicio roto, MuiCache, COM y reglas de firewall).
- Modo claro y oscuro, menú contextual propio y panel inferior con los detalles del programa.

## Seguridad

- Solo se borra lo que salió del último análisis, por identificador; la interfaz nunca envía rutas.
- Los archivos no se eliminan: pasan a una cuarentena (`%LocalAppData%\Vanish\quarantine`) que se purga a los 7 días.
- Antes de borrar una clave o valor del registro se exporta una copia `.reg`.
- Hay una lista de rutas y nombres protegidos (Windows, Microsoft Shared, etc.).
- La limpieza de caché sí borra de forma definitiva, pero solo de las categorías incluidas.

## Requisitos

- Windows 10/11 con WebView2 (viene con Windows 11).
- Pide permisos de administrador al abrirse; sin ellos no puede tocar `HKLM` ni carpetas protegidas.

## Descarga

En [Releases](../../releases): instalador (`setup.exe`) o versión portable.
Los ejecutables no están firmados, así que SmartScreen puede avisar: *Más información → Ejecutar de todas formas*.

## Compilar

Necesitas Node.js, Rust y los requisitos de [Tauri en Windows](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev      # desarrollo
npm run tauri build    # exe + instalador NSIS en src-tauri/target/release
```

Pruebas del backend: `cargo test` dentro de `src-tauri`.

## Licencia

[MIT](LICENSE)

## Limitaciones conocidas

- Las tareas programadas solo se leen con permisos de administrador.
- No hay botón para restaurar desde la cuarentena ni desde los `.reg`; hay que hacerlo a mano.
- La versión portable también guarda datos en `%LocalAppData%\Vanish`.
- La eliminación de apps de la Tienda no está probada con muchas apps reales.
