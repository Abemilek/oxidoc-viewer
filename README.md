# Visor DOCX en Rust (Android + escritorio)

Visor de documentos **escrito 100 % en Rust**: sin Java ni Kotlin. La app es una
`NativeActivity` de Android que carga una librería Rust (`libvisor_docx.so`) y dibuja
todo con [egui](https://github.com/emilk/egui) (eframe 0.32, backend OpenGL ES).

Formatos:

| Formato | Soporte |
|---|---|
| `.docx` `.docm` `.dotx` `.dotm` | Párrafos, negrita/cursiva/subrayado/tachado, color, tamaño, resaltado, super/subíndice, encabezados, listas con viñetas y numeración, tablas (también anidadas), imágenes (PNG/JPEG/GIF/BMP), alineación, hipervínculos (solo estilo), saltos de página |
| `.odt` `.ott` | Lo mismo en versión básica (estilos automáticos, listas, tablas, imágenes) |
| `.txt` `.md` | Texto plano; en `.md` solo se interpretan los encabezados `#` |
| `.doc` (binario viejo) y `.rtf` | **No soportados** → el visor muestra un aviso para convertirlos a `.docx` |

El parser de DOCX/ODT es propio (tokenizador XML de ~200 líneas + `zip`), así que no depende de
crates de Word con API inestable. Dependencias: `eframe/egui`, `zip` (solo deflate), `image`
(png/jpeg/gif/bmp), `android-activity` y `jni` (estos dos solo en Android, para la `NativeActivity`
y el permiso de almacenamiento).

## Estado honesto

* El código **compila** con Rust stable en escritorio y para `aarch64-linux-android`
  (`cargo check`), y el `.docx` de `samples/` se parsea correctamente en la prueba de humo.
* **No está probado en un dispositivo Android real**: el empaquetado del APK, los permisos y el
  renderizado OpenGL ES pueden necesitar ajustes en el primer arranque. Si algo falla, abre un
  *issue* con el error de `cargo`/`adb logcat` y se corrige.


## 1) Probar primero en la laptop (rápido)

```bash
sudo apt install build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev \
                 libegl1-mesa-dev libgl1-mesa-dev
sudo apt install rustup && rustup default stable      # o https://rustup.rs
cargo run --release -- samples/ejemplo.docx
```

Es la misma interfaz que en Android (explorador + visor). Si algo falla aquí, fallará en el móvil.

## 2) Compilar el APK en Debian 13

Herramientas necesarias (Rust para Android, JDK, SDK/NDK de Android, cargo-apk):

```bash
sudo apt install default-jdk-headless unzip adb
# Rust + target Android (rust-toolchain.toml añade aarch64-linux-android solo):
rustup default stable
rustup target add aarch64-linux-android
```

Android SDK y NDK: Google no los distribuye por apt. En Debian existen paquetes instaladores
en `contrib` (p. ej. `google-android-ndk-*-installer`, `google-android-platform-34-installer`,
`google-android-build-tools-34.0.0-installer`; busca con `apt search google-android`). Si no
los tienes, descarga las *command line tools* de developer.android.com y ejecuta:

```bash
sdkmanager "platforms;android-34" "build-tools;34.0.0" "platform-tools" "ndk;27.2.12479018"
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/27.2.12479018
```

Compilar:

```bash
./build-android.sh            # APK de depuración firmado con la clave debug (suficiente para probar)
adb install -r $(find target -name '*.apk' | head -1)
# o directamente: cargo apk run --lib
```

Versión release (necesita tu propia firma):

```bash
keytool -genkey -v -keystore ~/visor.keystore -alias visor -keyalg RSA -keysize 2048 -validity 10000
export CARGO_APK_RELEASE_KEYSTORE=$HOME/visor.keystore
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD='tu-clave'
./build-android.sh release
```

> `cargo-apk` fue marcado como deprecado en favor de `xbuild` hace tiempo, pero sigue
> funcionando con NativeActivity y se ha vuelto a usar en proyectos de 2026. Si `cargo install
> cargo-apk` falla, la alternativa es `xbuild` (`x build --platform android --arch arm64`).

## 3) Usar la app

1. Abre **Visor DOCX**. Si falta el permiso, pulsa **Conceder acceso**: se abre la pantalla de
   Android «Acceso a todos los archivos»; actívalo y vuelve a la app.
2. Navega por *Descargas*, *Documentos*, etc. y toca un archivo.
3. Zoom con los botones **− / +** o pellizcando. Scroll con el dedo.

También puedes copiar archivos con `adb push doc.docx /sdcard/Download/`.

## Limitaciones conocidas

* Sin edición, búsqueda ni impresión; solo visualización.
* No se leen encabezados/pies de página, notas al pie, cuadros de texto ni gráficos/ecuaciones complejas.
  Las imágenes flotantes se muestran en línea.
* Tablas: columnas de igual ancho; celdas combinadas se tratan como una celda.
* Justificado se muestra alineado a la izquierda. Interlineado y espaciado fijos.
* Fuentes: egui incluye una fuente latina; la negrita real depende de que exista una fuente
  `*-Bold.ttf` en el sistema (Android: `/system/fonts`). Sin ella se usa mayor contraste.
  Alfabetos CJK/árabe/etc. no se renderizan.
* No se abre desde «Abrir con…» de otras apps (requeriría leer `content://` vía JNI).
* El permiso `MANAGE_EXTERNAL_STORAGE` sirve para instalar por APK; Google Play no lo acepta
  para un visor como este (habría que usar el selector de documentos de Android, que requiere JNI
  adicional o una capa Java).
* Si ves pantalla negra en algún dispositivo, cambia en `Cargo.toml` la feature `glow` por `wgpu`
  (y en `src/lib.rs` añade `renderer: eframe::Renderer::Wgpu` a `NativeOptions`).

## Estructura

```
src/lib.rs      punto de entrada (android_main / run_desktop)
src/xml.rs      tokenizador XML mínimo
src/docx.rs     parser DOCX (estilos, numeración, tablas, imágenes)
src/odt.rs      parser ODT
src/text.rs     texto plano / markdown
src/load.rs     detección de formato
src/ui.rs       explorador + visor egui (scroll virtualizado)
src/platform.rs JNI: permiso de almacenamiento en Android
```
